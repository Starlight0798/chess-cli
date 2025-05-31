use crate::{
    cli::{display, input}, engine::{EngineManager, EngineProtocol, EngineType}, game::{GameManager, GameState, PlayerColor}, log_error, utils::*
};


/// 用户命令
#[derive(Debug)]
pub enum Command {
    NewGame { engine_type: EngineType, player_color: PlayerColor },
    MakeMove(String),
    StopEngine,
    ShowBoard,
    Quit,
    Error(String),
}

/// 运行交互式主循环
pub async fn run_interactive_loop() -> Result<()> {
    // 初始化显示
    display::show_welcome()?;
    
    // 创建命令通道
    let (tx, mut rx) = unbounded_channel::<Command>();
    
    // 启动输入监听
    spawn(input::listen_for_commands(tx));
    
    // 创建引擎管理器
    let engine_manager: EngineManager = EngineManager::new()
        .map_err(|e| anyhow!("引擎初始化失败: {}", e))?;
    
    // 初始化游戏管理器
    let mut game_manager: Option<GameManager> = None;
    
    // 主事件循环
    while let Some(cmd) = rx.recv().await {
        // 先清空消息区域
        display::clear_message_area()?;
        
        match cmd {
            Command::NewGame { engine_type, player_color } => {
                match handle_new_game(&engine_manager, engine_type, player_color).await {
                    Ok(game) => {
                        game_manager = Some(game);
                        display::render_board(&game_manager.as_ref().unwrap().state)?;
                    }
                    Err(e) => display::show_error(&e.to_string())?,
                }
            }
            Command::MakeMove(move_str) => {
                if let Some(game) = &mut game_manager {
                    match game.player_move(&move_str).await {
                        Ok(_) => {
                            display::render_board(&game.state)?;
                        }
                        Err(e) => display::show_error(&e.to_string())?,
                    }
                } else {
                    display::show_error("请先使用 'new' 命令开始游戏")?;
                }
            }
            Command::StopEngine => {
                if let Some(game) = &mut game_manager {
                    match game.stop_and_get_bestmove().await {
                        Ok(best_move) => {
                            display::show_best_move(&best_move)?;
                        }
                        Err(e) => display::show_error(&e.to_string())?,
                    }
                } else {
                    display::show_error("没有正在进行的游戏")?;
                }
            }
            Command::ShowBoard => {
                if let Some(game) = &game_manager {
                    display::render_board(&game.state)?;
                } else {
                    display::show_error("没有游戏状态可显示")?;
                }
            }
            Command::Quit => break,
            Command::Error(msg) => display::show_error(&msg)?,
        }
    }
    
    // 清理并退出
    if let Some(mut game) = game_manager {
        let _ = game.quit().await;
    }
    
    display::cleanup_terminal()?;
    Ok(())
}

/// 处理新游戏命令
async fn handle_new_game(
    engine_manager: &EngineManager,
    engine_type: EngineType,
    player_color: PlayerColor,
) -> Result<GameManager> {
    // 创建引擎实例
    let mut engine: Box<dyn EngineProtocol + 'static> = engine_manager.create_engine_instance(&engine_type)?;
    
    // 初始化引擎
    engine.init().await?;
    
    // 创建游戏管理器
    let mut game: GameManager = GameManager::new(engine);
    
    // 开始新游戏
    game.start_new_game(player_color).await?;
    
    Ok(game)
}

/// 主循环
pub async fn run() -> Result<()> {
    match run_interactive_loop().await {
        Err(e) => {
            #[cfg(debug_assertions)]
            { 
                log_error!(e);
                Err(e) 
            }

            #[cfg(not(debug_assertions))]
            {
                let _ = display::show_error("内部错误");
                Ok(())
            }
        },
        _ => Ok(()),
    }
}