use crate::{
    engine::protocol::{EngineThinkingInfo, EngineProtocol, EngineGoResult},
    game::state::{GameState, PlayerColor},
    game::fen::FenProcessor,
};
use crate::utils::*;

/// 游戏管理器
pub struct GameManager {
    /// 游戏状态
    pub state: GameState,
    /// 引擎实例
    pub engine: Box<dyn EngineProtocol>,
    /// 思考信息
    pub think_info: Option<EngineThinkingInfo>,
}

impl GameManager {
    /// 创建新游戏管理器
    pub fn new(engine: Box<dyn EngineProtocol>) -> Self {
        Self {
            state: GameState::new(),
            engine,
            think_info: None,
        }
    }

    /// 开始新游戏
    pub async fn start_new_game(&mut self, player_color: PlayerColor, fen: Option<String>) -> Result<()> {
        // 重置游戏状态
        self.state = if let Some(fen_str) = fen {
            FenProcessor::parse_fen(&fen_str)?
        } else {
            GameState::new()
        };
        
        // 重置引擎状态
        self.engine.set_option("Clear Hash", None).await?;
        
        // 设置初始位置
        self.engine.set_position(&self.state.to_fen()).await?;
        
        // 如果目前局面引擎先走
        if player_color.opponent() == self.state.current_player {
            self.state.flipped = true;
            self.engine_move().await?;
        }
        
        Ok(())
    }
    
    /// 玩家走子
    pub async fn player_move(&mut self, move_str: &str) -> Result<()> {
        self.state.apply_move(move_str)?;
        self.engine.set_position(&self.state.to_fen()).await?;
        Ok(())
    }
    
    /// 引擎思考并走子
    pub async fn engine_move(&mut self) -> Result<()> {
        // 等待引擎走子
        const MAX_THINK_TIME: usize = 5000;
        let result: EngineGoResult = self.engine.go(Some(MAX_THINK_TIME)).await?;

        // 处理引擎走子和记录思考信息
        if !result.infos.is_empty() {
            let mut info: EngineThinkingInfo = result.infos.into_iter().last().unwrap();
            if let Some(pv) = &info.pv {
                info.pv = Some(self.state.pv_to_chinese(pv)?);
            }
            self.think_info = Some(info);
        }
        
        self.state.apply_move(&result.best_move)?;
        self.engine.set_position(&self.state.to_fen()).await?;
        
        Ok(())
    }
    
    /// 退出游戏
    pub async fn quit(&mut self) -> Result<()> {
        self.engine.quit().await?;
        Ok(())
    }
}
