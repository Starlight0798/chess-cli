use crate::{
    engine::protocol::EngineProtocol,
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
    /// 当前思考任务通道
    pub think_task: Option<UnboundedSender<()>>,
}

impl GameManager {
    /// 创建新游戏管理器
    pub fn new(engine: Box<dyn EngineProtocol>) -> Self {
        Self {
            state: GameState::new(),
            engine,
            think_task: None,
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
            self.engine_move().await?;
        }
        
        Ok(())
    }
    
    /// 玩家走子
    pub async fn player_move(&mut self, move_str: &str) -> Result<()> {
        // 应用玩家走法
        self.state.apply_move(move_str)?;
        
        // 更新引擎位置
        self.engine.set_position(&self.state.to_fen()).await?;
        
        Ok(())
    }
    
    /// 引擎思考并走子
    pub async fn engine_move(&mut self) -> Result<()> {
        // 创建中断通道
        let (tx, mut rx) = unbounded_channel();
        self.think_task = Some(tx);

        // 直接异步等待引擎走子或中断
        const MAX_THINK_TIME: usize = 5000;
        let result: Result<Result<String>> = select! {
            best_move = self.engine.go(Some(MAX_THINK_TIME)) => Ok(best_move),
            _ = rx.recv() => Ok(Err(anyhow!("思考被中断"))),
        };

        // 处理引擎走子
        match result {
            Ok(Ok(best_move)) => {
                self.state.apply_move(&best_move)?;
                self.engine.set_position(&self.state.to_fen()).await?;
            }
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(e),
        }

        self.think_task = None;
        Ok(())
    }
    
    /// 退出游戏
    pub async fn quit(&mut self) -> Result<()> {
        if let Some(tx) = self.think_task.take() {
            let _ = tx.send(());
        }
        self.engine.quit().await?;
        Ok(())
    }
}
