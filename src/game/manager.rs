use crate::{
    engine::protocol::EngineProtocol,
    game::state::{GameState, PlayerColor},
};
use crate::utils::*;

/// 游戏管理器
pub struct GameManager {
    /// 游戏状态
    pub state: GameState,
    /// 引擎实例
    engine: Box<dyn EngineProtocol>,
    /// 当前思考任务通道
    think_task: Option<UnboundedSender<()>>,
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
    pub async fn start_new_game(&mut self, player_color: PlayerColor) -> Result<()> {
        // 重置游戏状态
        self.state.reset();
        
        // 设置初始位置
        self.engine.set_position(&self.state.to_fen()).await?;
        
        // 如果玩家选择黑方，引擎先走
        if player_color == PlayerColor::Black {
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
        
        // 引擎思考并走子
        self.engine_move().await?;
        
        Ok(())
    }
    
    /// 引擎思考并走子
    async fn engine_move(&mut self) -> Result<()> {
        // 创建中断通道
        let (tx, mut rx) = unbounded_channel();
        self.think_task = Some(tx);

        // 直接异步等待引擎走子或中断
        let result: Result<Result<String>> = select! {
            best_move = self.engine.go(Some(3000)) => Ok(best_move),
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
    
    /// 停止引擎思考并获取最佳着法
    pub async fn stop_and_get_bestmove(&mut self) -> Result<String> {
        if let Some(tx) = self.think_task.take() {
            // 中断思考
            let _ = tx.send(());
            
            // 获取当前最佳着法
            self.engine.stop().await?;
            self.engine.go(Some(100)).await  // ms思考
        } else {
            Err(anyhow!("引擎当前没有在思考"))
        }
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
