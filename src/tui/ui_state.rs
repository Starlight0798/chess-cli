use crate::engine::protocol::{EngineOption, EngineThinkingInfo};
use crate::game::Position;
use crate::tui::config::GameConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    Game,
    Settings,
    Help,
}

pub struct UiState {
    pub cursor: Position,
    pub selected: Option<Position>,
    pub view: View,
    pub messages: Vec<String>,
    pub legal_moves: Vec<Position>,
    pub engine_info: Option<EngineThinkingInfo>,
    pub ponder_move: Option<String>,
    pub engine_options: Vec<EngineOption>,
    pub game_config: GameConfig,
    pub menu_index: usize,
    pub engine_list: Vec<String>,
    pub engine_pv_chinese: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            cursor: Position { row: 0, col: 0 },
            selected: None,
            view: View::Home,
            messages: Vec::new(),
            legal_moves: Vec::new(),
            engine_info: None,
            ponder_move: None,
            engine_options: Vec::new(),
            game_config: GameConfig::default(),
            menu_index: 0,
            engine_list: Vec::new(),
            engine_pv_chinese: None,
        }
    }
}
