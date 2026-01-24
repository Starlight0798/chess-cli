use crate::engine::{EngineProtocol, EngineType, UciEngine};
use crate::utils::*;

/// 引擎配置
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// 引擎可执行文件路径
    pub path: String,
    /// 引擎默认选项
    pub options: HashMap<String, Option<String>>,
}

/// 引擎管理器
pub struct EngineManager {
    /// 引擎配置
    pub engines: HashMap<EngineType, EngineConfig>,
}

impl EngineManager {
    /// 创建新的引擎管理器
    /// 返回管理器实例和加载信息消息
    pub fn new() -> Result<(Self, String)> {
        // 查找配置文件
        let (config_path, is_created) = match Self::find_config() {
            Ok(path) => (path, false),
            Err(_) => {
                // 如果找不到，尝试创建默认配置
                let path = Self::create_default_config()?;
                (path, true)
            }
        };

        // 读取配置文件内容
        let config_content: String = read_to_string(&config_path)
            .with_context(|| format!("读取配置文件失败: {}", config_path.display()))?;

        log_info!(config_content);

        // 解析 TOML 配置
        let config: toml::Value = toml::from_str(&config_content)
            .with_context(|| format!("配置文件格式无效: {}", config_path.display()))?;

        log_info!(config);

        // 创建引擎映射
        let mut engines: HashMap<EngineType, EngineConfig> = HashMap::new();
        if let Some(table) = config.as_table() {
            for (key, value) in table {
                if let Ok(engine_type) = EngineType::from_str(key)
                    && let Ok(engine_config) = EngineConfig::try_from(value.clone())
                {
                    engines.insert(engine_type, engine_config);
                }
            }
        }

        log_info!(engines);

        let msg = if is_created {
            format!(
                "未找到配置文件，已在 {} 创建默认配置",
                config_path.display()
            )
        } else {
            format!("已加载配置文件: {}", config_path.display())
        };

        Ok((Self { engines }, msg))
    }

    /// 创建默认配置文件
    fn create_default_config() -> Result<PathBuf> {
        // 优先在当前目录创建
        let current_dir_config = Path::new("engines.toml");

        let default_content = r#"# Chess CLI 引擎配置文件

# 示例: 皮卡鱼 (Pikafish)
[pikafish]
# 引擎可执行文件路径 (支持绝对路径或相对路径)
# Windows 示例:
path = "./pikafish.exe" 
# Linux/Mac 示例:
# path = "./pikafish"

[pikafish.options]
# 这里可以设置 UCI 选项
# Threads = "1"
# Hash = "16"
"#;

        // 尝试写入当前目录
        if std::fs::write(current_dir_config, default_content).is_ok() {
            return Ok(current_dir_config.to_path_buf());
        }

        // 如果当前目录不可写，尝试用户配置目录
        if let Some(mut config_dir) = dirs::config_dir() {
            config_dir.push("chess-cli");
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir)?;
            }
            config_dir.push("engines.toml");
            std::fs::write(&config_dir, default_content)?;
            return Ok(config_dir);
        }

        Err(anyhow!("无法创建默认配置文件，请检查权限"))
    }

    /// 查找配置文件
    fn find_config() -> Result<PathBuf> {
        // 1. 检查当前工作目录
        let current_dir: PathBuf = Path::new(".").join("engines.toml");
        if current_dir.exists() {
            return Ok(current_dir);
        }

        // 2. 检查可执行文件所在目录
        if let Ok(exe_path) = current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            let exe_config: PathBuf = exe_dir.join("engines.toml");
            if exe_config.exists() {
                return Ok(exe_config);
            }

            // 2.1 开发环境回退：尝试在父目录查找 (例如 target/release/chess-cli -> ../../engines.toml)
            let mut parent = exe_dir;
            for _ in 0..3 {
                if let Some(p) = parent.parent() {
                    let config = p.join("engines.toml");
                    if config.exists() {
                        return Ok(config);
                    }
                    parent = p;
                }
            }
        }

        // 3. 检查用户配置目录
        if let Some(mut config_dir) = dirs::config_dir() {
            config_dir.push("chess-cli");
            config_dir.push("engines.toml");
            if config_dir.exists() {
                return Ok(config_dir);
            }
        }

        // 4. 检查系统全局配置目录
        #[cfg(target_os = "linux")]
        {
            let system_config: &Path = Path::new("/etc/chess-cli/engines.toml");
            if system_config.exists() {
                return Ok(system_config.to_path_buf());
            }
        }

        #[cfg(target_os = "windows")]
        {
            let system_config: &Path = Path::new("C:\\ProgramData\\chess-cli\\engines.toml");
            if system_config.exists() {
                return Ok(system_config.to_path_buf());
            }
        }

        Err(anyhow!("未能在任何标准位置找到 engines.toml 配置文件"))
    }

    /// 获取所有可用引擎名称
    pub fn list_engines(&self) -> Vec<String> {
        self.engines.keys().map(|k| k.to_string()).collect()
    }

    /// 获取指定引擎配置
    pub fn get_config(&self, engine_type: &EngineType) -> Result<&EngineConfig> {
        self.engines
            .get(engine_type)
            .ok_or_else(|| anyhow!("未找到引擎 '{:?}' 的配置", engine_type))
    }

    /// 创建引擎协议实例
    pub async fn create_engine_instance(
        &self,
        engine_type: &EngineType,
    ) -> Result<Box<dyn EngineProtocol>> {
        let config: &EngineConfig = self.get_config(engine_type)?;
        // 解析路径中的环境变量
        let engine_path: String = Self::resolve_path(&config.path)?;
        // 创建引擎实例
        let mut engine: Box<dyn EngineProtocol> = match engine_type {
            EngineType::Pikafish | EngineType::Other(_) => Box::new(UciEngine::new(&engine_path)?),
        };

        // 初始化引擎
        engine.init().await?;

        // 应用默认选项
        for (name, value) in &config.options {
            engine.set_option(name, value.as_deref()).await?;
        }

        // 准备就绪
        engine.ready().await?;

        Ok(engine)
    }

    /// 解析路径中的环境变量
    fn resolve_path(path: &str) -> Result<String> {
        if path.starts_with('$') {
            // 处理环境变量
            let parts: Vec<&str> = path.split('/').collect();
            if let Some(var_name) = parts[0].strip_prefix('$') {
                let var_value: String =
                    var(var_name).with_context(|| format!("环境变量 {} 未设置", var_name))?;

                let resolved_path: String = Path::new(&var_value)
                    .join(parts[1..].join("/"))
                    .to_string_lossy()
                    .to_string();

                return Ok(resolved_path);
            }
        }

        Ok(path.to_string())
    }
}

// 实现 TOML 值到 EngineConfig 的转换
impl TryFrom<toml::Value> for EngineConfig {
    type Error = anyhow::Error;

    fn try_from(value: toml::Value) -> Result<Self> {
        let table: &toml::map::Map<String, toml::Value> = value
            .as_table()
            .ok_or_else(|| anyhow!("引擎配置应为表结构"))?;

        let path: String = table
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("引擎配置缺少 'path' 字段"))?
            .to_string();

        // 解析选项
        let mut options: HashMap<String, Option<String>> = HashMap::new();
        if let Some(options_table) = table.get("options").and_then(|v| v.as_table()) {
            for (key, value) in options_table {
                // 值为空字符串表示无值选项
                if let Some(val_str) = value.as_str() {
                    if val_str.is_empty() {
                        options.insert(key.clone(), None);
                    } else {
                        options.insert(key.clone(), Some(val_str.to_string()));
                    }
                }
            }
        }

        Ok(EngineConfig { path, options })
    }
}
