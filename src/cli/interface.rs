use crate::{
    cli::{display, input},
    engine::{manager::EngineManager, protocol::EngineProtocol},
    game::state::GameState,
    utils::Result,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use crate::utils::*;

/// 玩家颜色
#[derive(Debug, Clone, Copy)]
pub enum PlayerColor {
    Red,
    Black,
}

/// 命令枚举
#[derive(Debug)]
pub enum Command {
    NewGame {
        engine_name: String,
        player_color: PlayerColor,
    },
    MakeMove(String),
    StopEngine,
    ShowBoard,
    Quit,
}

/// 游戏管理器
pub struct GameManager {
    pub state: GameState,
    engine: Box<dyn EngineProtocol>,
    current_thinking: Option<UnboundedSender<()>>,
}

impl GameManager {
    pub fn new(engine: Box<dyn EngineProtocol>) -> Self {
        Self {
            state: GameState::new(),
            engine,
            current_thinking: None,
        }
    }
    
    pub async fn start_new_game(&mut self, player_color: PlayerColor) -> Result<()> {
        self.state = GameState::new();
        self.engine.init().await?;
        self.engine.set_position(&self.state.to_fen()).await?;
        
        if player_color == PlayerColor::Black {
            self.engine_move().await?;
        }
        
        Ok(())
    }
    
    pub async fn player_move(&mut self, move_str: &str) -> Result<()> {
        self.state.apply_move(move_str)?;
        self.engine.set_position(&self.state.to_fen()).await?;
        self.engine_move().await?;
        Ok(())
    }
    
    async fn engine_move(&mut self) -> Result<()> {
        let (tx, rx) = unbounded_channel();
        self.current_thinking = Some(tx);
        
        let mut engine = self.engine.as_mut();
        let think_handle = tokio::spawn(async move {
            tokio::select! {
                result = engine.go(None) => result,
                _ = rx.recv() => Err(anyhow!("思考被中断")),
            }
        });
        
        let best_move = think_handle.await??;
        self.state.apply_move(&best_move)?;
        self.engine.set_position(&self.state.to_fen()).await?;
        self.current_thinking = None;
        
        Ok(())
    }
    
    pub async fn stop_and_get_bestmove(&mut self) -> Result<String> {
        if let Some(tx) = self.current_thinking.take() {
            let _ = tx.send(());
            self.engine.stop().await?;
            
            // 获取当前最佳着法
            self.engine.go(Some(100)).await // 短时间获取最佳着法
        } else {
            Err(anyhow!("引擎未在思考中"))
        }
    }
}

/// 运行交互循环
pub async fn run_interactive_loop() -> Result<()> {
    // 初始化终端
    display::init_terminal()?;
    
    let (tx, rx) = unbounded_channel();
    let input_handle = tokio::spawn(async move {
        if let Err(e) = input::listen_for_commands(tx).await {
            display::print_error(&format!("监听命令出错: {}", e));
        }
    });

    let mut current_game: Option<GameManager> = None;
    let engine_manager = EngineManager::new()?;

    display::clear_screen();
    display::print_welcome();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::NewGame {
                engine_name,
                player_color,
            } => {
                match engine_manager.create_protocol(&engine_name) {
                    Ok(protocol) => {
                        let mut game = GameManager::new(protocol);
                        if let Err(e) = game.start_new_game(player_color).await {
                            display::print_error(&format!("开始新游戏失败: {}", e));
                        } else {
                            if let Err(e) = display::render_board(&game.state.to_fen()) {
                                display::print_error(&format!("渲染棋盘失败: {}", e));
                            }
                            current_game = Some(game);
                        }
                    }
                    Err(e) => display::print_error(&format!("创建引擎失败: {}", e)),
                }
            }
            Command::MakeMove(move_str) => {
                if let Some(game) = &mut current_game {
                    if let Err(e) = game.player_move(&move_str).await {
                        display::print_error(&format!("走棋失败: {}", e));
                    } else {
                        if let Err(e) = display::render_board(&game.state.to_fen()) {
                            display::print_error(&format!("渲染棋盘失败: {}", e));
                        }
                    }
                } else {
                    display::print_error("没有进行中的游戏");
                }
            }
            Command::StopEngine => {
                if let Some(game) = &mut current_game {
                    match game.stop_and_get_bestmove().await {
                        Ok(best_move) => display::print_best_move(&best_move),
                        Err(e) => display::print_error(&format!("停止引擎失败: {}", e)),
                    }
                } else {
                    display::print_error("没有进行中的游戏");
                }
            }
            Command::ShowBoard => {
                if let Some(game) = &current_game {
                    if let Err(e) = display::render_board(&game.state.to_fen()) {
                        display::print_error(&format!("渲染棋盘失败: {}", e));
                    }
                } else {
                    display::print_error("没有进行中的游戏");
                }
            }
            Command::Quit => {
                // 退出前清理
                if let Some(mut game) = current_game {
                    let _ = game.engine.quit().await;
                }
                break;
            }
        }
    }

    // 取消输入监听
    input_handle.abort();
    
    // 清理终端
    display::cleanup_terminal()?;
    Ok(())
}
