use crate::engine::protocol::GoParams;
use crate::game::PlayerColor;

#[derive(Debug, Clone, PartialEq)]
pub enum StrategyType {
    MoveTime,
    Depth,
    GameTime,
    Infinite,
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyType::MoveTime => write!(f, "固定步时"),
            StrategyType::Depth => write!(f, "固定深度"),
            StrategyType::GameTime => write!(f, "局时模式"),
            StrategyType::Infinite => write!(f, "无限分析"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub player_side: PlayerColor,
    pub engine_name: String,
    pub multipv: u32,
    pub difficulty_level: u32,

    pub strategy: StrategyType,
    pub move_time: u64,
    pub depth: u32,
    pub game_time: u64,
    pub game_inc: u64,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            player_side: PlayerColor::Red,
            engine_name: "pikafish".to_string(),
            multipv: 1,
            difficulty_level: 20,
            strategy: StrategyType::MoveTime,
            move_time: 3000,
            depth: 20,
            game_time: 10,
            game_inc: 5,
        }
    }
}

impl GameConfig {
    pub fn get_go_params(&self) -> GoParams {
        let mut params = GoParams::default();
        match self.strategy {
            StrategyType::MoveTime => params.movetime = Some(self.move_time as usize),
            StrategyType::Depth => params.depth = Some(self.depth as usize),
            StrategyType::GameTime => {
                let time_ms = self.game_time * 60 * 1000;
                let inc_ms = self.game_inc * 1000;
                params.wtime = Some(time_ms as usize);
                params.btime = Some(time_ms as usize);
                params.winc = Some(inc_ms as usize);
                params.binc = Some(inc_ms as usize);
            }
            StrategyType::Infinite => params.infinite = true,
        }
        params
    }
}
