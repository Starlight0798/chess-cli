use crate::utils::*;
use crate::engine::protocol::*;

/// 引擎配置
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// 引擎名称
    pub name: EngineType,
    
    /// 通信协议 (如 uci)
    pub protocol: EngineProtocolType,
    
    /// 引擎可执行文件路径
    pub path: String,
}

/// 引擎管理器
pub struct EngineManager {
    /// 所有引擎配置
    engines: HashMap<EngineType, EngineConfig>,
    
    /// 默认引擎名称
    default_engine: EngineType,
}

impl EngineManager {
    /// 创建新的引擎管理器
    pub fn new() -> Result<Self> {
        // 查找配置文件
        let config_path: PathBuf = Self::find_config()?;
        
        // 读取配置文件内容
        let config_content: String = read_to_string(&config_path)
            .with_context(|| format!("读取配置文件失败: {}", config_path.display()))?;
        
        // 解析 TOML 配置
        let config: toml::Value = toml::from_str(&config_content)
            .with_context(|| format!("配置文件格式无效: {}", config_path.display()))?;

        log_info!(config);
        
        // 提取默认引擎
        let default_engine_str: &str = config.get("default")
            .ok_or_else(|| anyhow!("配置文件缺少 'default' 部分"))?
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("默认配置缺少 'name' 字段"))?;
        let default_engine: EngineType = EngineType::from_str(default_engine_str)?;

        log_info!(default_engine);
        
        // 创建引擎映射
        let mut engines: HashMap<EngineType, EngineConfig> = HashMap::new();
        for item in config.as_table().unwrap().values() {
            let name = item.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("引擎配置缺少 'name' 字段"))?;
            engines.insert(EngineType::from_str(name)?, EngineConfig::try_from(item.clone())?);
        }

        log_info!(engines);

        Ok(Self {
            engines,
            default_engine,
        })
    }
    
    /// 查找配置文件
    fn find_config() -> Result<PathBuf> {
        // 1. 当前目录
        let current_dir: PathBuf = Path::new(".").join("engines.toml");
        if current_dir.exists() {
            return Ok(current_dir);
        }
        
        // 2. 可执行文件所在目录
        if let Ok(exe_path) = current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let exe_config: PathBuf = exe_dir.join("engines.toml");
                if exe_config.exists() {
                    return Ok(exe_config);
                }
            }
        }
        
        // 3. 用户配置目录
        if let Some(mut config_dir) = dirs::config_dir() {
            config_dir.push("chess-cli");
            config_dir.push("engines.toml");
            if config_dir.exists() {
                return Ok(config_dir);
            }
        }
        
        // 4. 系统配置目录
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
        self.engines.keys()
            .map(|k| k.to_string())
            .collect()
    }

    /// 获取指定引擎配置
    pub fn get_config(&self, engine_type: &EngineType) -> Result<&EngineConfig> {
        self.engines.get(engine_type)
            .ok_or_else(|| anyhow!("未找到引擎 '{:?}' 的配置", engine_type))
    }
    
    /// 创建引擎协议实例
    pub fn create_engine_instance(&self, engine_type: &EngineType) -> Result<Box<dyn EngineProtocol>> {
        let config: &EngineConfig = self.get_config(engine_type)?;

        // 根据协议类型创建引擎
        match config.protocol {
            EngineProtocolType::Uci => {
                // 解析路径中的环境变量
                let engine_path: String = Self::resolve_path(&config.path)?;
                
                // 创建引擎
                let engine: Box<dyn EngineProtocol> = match config.name {
                    EngineType::Pikafish => {
                        Box::new(UciEngine::new(&engine_path)?)
                    },
                };
                Ok(engine)
            }
        }
    }
    
    /// 解析路径中的环境变量
    fn resolve_path(path: &str) -> Result<String> {
        if path.starts_with('$') {
            // 处理环境变量
            let parts: Vec<&str> = path.split('/').collect();
            if let Some(var_name) = parts[0].strip_prefix('$') {
                let var_value: String = var(var_name)
                    .with_context(|| format!("环境变量 {} 未设置", var_name))?;
                
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
        let table: &toml::map::Map<String, toml::Value> = value.as_table()
            .ok_or_else(|| anyhow!("引擎配置应为表结构"))?;
        
        Ok(EngineConfig {
            name: EngineType::from_str(table.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("引擎配置缺少 'name' 字段"))?)?,
            protocol: EngineProtocolType::from_str(table.get("protocol")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("引擎配置缺少 'protocol' 字段"))?)?,
            path: table.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("引擎配置缺少 'path' 字段"))?
                .to_string(),
        })
    }
}