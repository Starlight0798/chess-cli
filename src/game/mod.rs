//! 游戏核心模块，包括状态管理和FEN处理

pub mod fen;
pub mod state;

// 公开导出
pub use fen::FenProcessor;
pub use state::{GameState, PlayerColor, Piece, PieceKind};
