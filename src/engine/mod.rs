//! 引擎模块，负责管理象棋引擎的配置和通信

pub mod manager;
pub mod protocol;

// 公开导出
pub use manager::EngineManager;
pub use protocol::{EngineProtocol, UciEngine};
