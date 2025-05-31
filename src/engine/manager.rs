use crate::utils::*;
use crate::engine::protocol::{EngineProtocol, UciEngine};

/// 引擎配置
#[derive(Debug, Deserialize, Clone)]
pub struct EngineConfig {
    /// 引擎名称
    pub name: String,
    
    /// 通信协议 (如 uci)
    pub protocol: String,
    
    /// 引擎可执行文件路径
    pub path: String,
    
    /// 权重文件路径 (可选)
    pub weights: Option<String>,
    
    /// 启动参数
    #[serde(default)]
    pub args: Vec<String>,
}

/// 引擎管理器
pub struct EngineManager {
    /// 所有引擎配置
    engines: HashMap<String, EngineConfig>,
    
    /// 默认引擎名称
    default_engine: String,
}

impl EngineManager {
    /// 创建新的引擎管理器
    pub fn new() -> Result<Self> {
        // 查找配置文件
        let config_path = Self::find_config()?;
        
        // 读取配置文件内容
        let config_content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
        
        // 解析 TOML 配置
        let config: toml::Value = toml::from_str(&config_content)
            .with_context(|| format!("Invalid config format: {}", config_path.display()))?;
        
        // 提取默认配置
        let default_config = config.get("default")
            .ok_or_else(|| anyhow!("Missing 'default' section in config"))?;
        
        // 创建引擎映射
        let mut engines = HashMap::new();
        
        // 添加默认引擎
        let default_engine = "default".to_string();
        engines.insert(
            default_engine.clone(),
            default_config.clone().try_into()?,
        );
        
        // 添加其他引擎
        for (key, value) in config.as_table().unwrap().iter() {
            if key != "default" {
                let engine_config: EngineConfig = value.clone().try_into()?;
                engines.insert(key.clone(), engine_config);
            }
        }
        
        Ok(Self {
            engines,
            default_engine,
        })
    }
    
    /// 查找配置文件
    fn find_config() -> Result<PathBuf> {
        // 1. 当前目录
        let current_dir = Path::new(".").join("engines.toml");
        if current_dir.exists() {
            return Ok(current_dir);
        }
        
        // 2. 可执行文件所在目录
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let exe_config = exe_dir.join("engines.toml");
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
            let system_config = Path::new("/etc/chess-cli/engines.toml");
            if system_config.exists() {
                return Ok(system_config.to_path_buf());
            }
        }
        
        Err(anyhow!("Could not find engines.toml in any standard location"))
    }
    
    /// 获取所有可用引擎名称
    pub fn list_engines(&self) -> Vec<&str> {
        self.engines.keys().map(|k| k.as_str()).collect()
    }
    
    /// 获取指定引擎配置
    pub fn get_config(&self, engine_name: &str) -> Result<&EngineConfig> {
        self.engines.get(engine_name)
            .ok_or_else(|| anyhow!("Engine '{}' not found", engine_name))
    }
    
    /// 创建引擎协议实例
    pub fn create_protocol(&self, engine_name: &str) -> Result<Box<dyn EngineProtocol>> {
        let config = self.get_config(engine_name)?;
        
        // 根据协议类型创建引擎
        match config.protocol.to_lowercase().as_str() {
            "uci" => {
                // 解析路径中的环境变量
                let engine_path = Self::resolve_path(&config.path)?;
                
                // 创建 UCI 引擎
                let engine = UciEngine::new(&engine_path, &config.args)?;
                Ok(Box::new(engine))
            }
            other => Err(anyhow!("Unsupported protocol: {}", other)),
        }
    }
    
    /// 解析路径中的环境变量
    fn resolve_path(path: &str) -> Result<String> {
        if path.starts_with('$') {
            // 处理环境变量
            let parts: Vec<&str> = path.split('/').collect();
            if let Some(var_name) = parts[0].strip_prefix('$') {
                let var_value = std::env::var(var_name)
                    .with_context(|| format!("Environment variable {} not set", var_name))?;
                
                let resolved_path = Path::new(&var_value)
                    .join(parts[1..].join("/"))
                    .to_string_lossy()
                    .to_string();
                
                return Ok(resolved_path);
            }
        }
        
        // 直接返回路径
        Ok(path.to_string())
    }
}

// 实现 TOML 值到 EngineConfig 的转换
impl TryFrom<toml::Value> for EngineConfig {
    type Error = anyhow::Error;
    
    fn try_from(value: toml::Value) -> Result<Self> {
        let table = value.as_table()
            .ok_or_else(|| anyhow!("Expected table for engine config"))?;
        
        Ok(EngineConfig {
            name: table.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'name' in engine config"))?
                .to_string(),
            protocol: table.get("protocol")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'protocol' in engine config"))?
                .to_string(),
            path: table.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'path' in engine config"))?
                .to_string(),
            weights: table.get("weights")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            args: table.get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }
}
