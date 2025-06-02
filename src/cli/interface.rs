use crate::{
    cli::{display, input}, 
    engine::{EngineManager, EngineProtocol, EngineType}, 
    game::{GameManager, GameState, PlayerColor}
};
use crate::utils::*;

/// 用户命令
#[derive(Debug)]
pub enum Command {
    NewGame { 
        engine_type: EngineType, 
        player_color: PlayerColor,
        fen: Option<String>
    },
    MakeMove(String),
    ShowBoard,
    History,
    SetOption { name: String, value: Option<String> },
    ListEngines,
    Reverse,
    Help,
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
        // 渲染视图
        display::render_view(game_manager.as_ref())?;
        
        match cmd {
            Command::NewGame { engine_type, player_color, fen } => {
                match handle_new_game(&engine_manager, engine_type, player_color, fen).await {
                    Ok(game) => {
                        game_manager = Some(game);
                        display::render_view(game_manager.as_ref())?;
                    }
                    Err(e) => display::show_error(&e.to_string())?,
                }
            },
            Command::MakeMove(move_str) => {
                if let Some(game) = &mut game_manager {
                    if let Err(e) = game.player_move(&move_str).await {
                        display::show_error(&e.to_string())?;
                        continue;
                    }
                }
                
                if game_manager.is_some() {
                    display::render_view(game_manager.as_ref())?;
                    
                    if let Some(game) = &mut game_manager {
                        if let Err(e) = game.engine_move().await {
                            display::show_error(&e.to_string())?;
                            continue;
                        }
                    }
                    display::render_view(game_manager.as_ref())?;
                } else {
                    display::show_error("请先使用 'new' 命令开始游戏")?;
                }
            },
            Command::ShowBoard => {
                if let Some(game) = &game_manager {
                    display::render_view(game_manager.as_ref())?;
                } else {
                    display::show_error("没有游戏状态可显示")?;
                }
            },
            Command::History => {
                if let Some(game) = &game_manager {
                    display::show_history(&game.state.history)?;
                } else {
                    display::show_error("没有游戏进行中")?;
                }
            },
            Command::SetOption { name, value } => { 
                if let Some(game) = &mut game_manager {
                    game.engine.set_option(&name, value.as_deref()).await?;
                    display::show_set_success(&name, value.as_deref())?;
                } else {
                    display::show_error("没有游戏进行中")?;
                }
            },
            Command::ListEngines => { 
                let engines: Vec<String> = engine_manager.list_engines();
                display::show_engines(&engines)?;
            },
            Command::Reverse => {
                if let Some(game) = &mut game_manager {
                    game.state.flipped = !game.state.flipped;
                    display::render_view(game_manager.as_ref())?;
                } else {
                    display::show_error("没有游戏进行中")?;
                }
            },
            Command::Help => {
                display::show_help()?;
            },
            Command::Quit => exit(0),
            Command::Error(msg) => display::show_error(&msg)?,
        }

        // 命令处理后，重置输入提示符和重绘棋盘
        if let Some(game) = &mut game_manager {
            display::render_board(&game.state)?;
        }
        display::reset_input_prompt()?;
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
    fen: Option<String>,
) -> Result<GameManager> {
    // 创建引擎实例
    let mut engine: Box<dyn EngineProtocol> = engine_manager.create_engine_instance(&engine_type).await?;
    
    // 初始化引擎
    engine.init().await?;
    
    // 创建游戏管理器
    let mut game: GameManager = GameManager::new(engine);
    
    // 开始新游戏
    game.start_new_game(player_color, fen).await?;
    
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