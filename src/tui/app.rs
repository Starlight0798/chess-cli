use crate::engine::actor::{EngineActor, EngineCommand};
use crate::engine::protocol::EngineEvent;
use crate::engine::{EngineManager, EngineType};
use crate::game::{GameState, PlayerColor, Position};
use crate::tui::event::{Event, EventHandler};
use crate::tui::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::time::Duration;
use tokio::sync::mpsc;

/// UI 视图状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    Game,
    Help,
}

/// UI 状态
pub struct UiState {
    pub cursor: Position,
    pub selected: Option<Position>,
    pub view: View,
    pub messages: Vec<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            cursor: Position { row: 0, col: 0 },
            selected: None,
            view: View::Home,
            messages: Vec::new(),
        }
    }
}

/// 应用程序
pub struct App {
    pub game_state: Option<GameState>,
    pub engine_actor: Option<EngineActor>,
    pub engine_manager: EngineManager,
    pub ui_state: UiState,
    pub should_quit: bool,
    pub event_sender: Option<mpsc::UnboundedSender<Event>>,
}

impl App {
    /// 创建新应用程序
    pub fn new() -> Result<Self> {
        let engine_manager = EngineManager::new()?;
        Ok(Self {
            game_state: None,
            engine_actor: None,
            engine_manager,
            ui_state: UiState::default(),
            should_quit: false,
            event_sender: None,
        })
    }

    /// 运行应用程序
    pub async fn run(&mut self) -> Result<()> {
        // 初始化终端
        let mut terminal = ratatui::init();
        terminal.clear()?;

        // 创建事件处理器
        let mut events = EventHandler::new(Duration::from_millis(250));
        self.event_sender = Some(events.sender.clone());

        // 尝试初始化引擎
        self.initialize_engine_actor().await;

        // 主循环
        while !self.should_quit {
            // 渲染
            terminal.draw(|f| ui::draw(f, self))?;

            // 处理事件
            match events.next().await {
                Some(Event::Key(key)) => self.handle_key(key).await?,
                Some(Event::Tick) => {},
                Some(Event::Engine(event)) => self.handle_engine_event(event).await?,
                _ => {},
            }
        }

        // 恢复终端
        ratatui::restore();
        Ok(())
    }

    /// 初始化引擎 Actor
    async fn initialize_engine_actor(&mut self) {
        if self.engine_actor.is_some() {
            return;
        }

        let engines = self.engine_manager.list_engines();
        if engines.is_empty() {
            self.ui_state.messages.push("未找到引擎配置".to_string());
            return;
        }

        // 临时 hack: 假设是 Pikafish
        let engine_type = EngineType::Pikafish;
        
        match self.engine_manager.create_engine_instance(&engine_type).await {
            Ok(engine) => {
                let (actor, mut rx) = EngineActor::new(engine);
                
                if let Some(tx) = &self.event_sender {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        while let Some(event) = rx.recv().await {
                            if tx.send(Event::Engine(event)).is_err() {
                                break;
                            }
                        }
                    });
                }
                
                self.engine_actor = Some(actor);
                self.ui_state.messages.push("引擎初始化成功".to_string());
            }
            Err(e) => {
                self.ui_state.messages.push(format!("引擎启动失败: {}", e));
            }
        }
    }


    /// 处理引擎事件
    async fn handle_engine_event(&mut self, event: EngineEvent) -> Result<()> {
        match event {
            EngineEvent::Thinking(info) => {
                // 可以显示思考信息，例如 PV
                // self.ui_state.messages.push(format!("Thinking: depth {}", info.depth));
            }
            EngineEvent::BestMove(move_str) => {
                self.ui_state.messages.push(format!("引擎着法: {}", move_str));
                if let Some(state) = &mut self.game_state {
                    // 解析并应用着法
                    // GameState 需要支持 uci move string
                    if let Ok(_) = state.apply_move(&move_str) {
                         // 检查是否结束
                         // ...
                    } else {
                        self.ui_state.messages.push(format!("引擎返回非法着法: {}", move_str));
                    }
                }
            }
            EngineEvent::Ready => {
                self.ui_state.messages.push("引擎准备就绪".to_string());
            }
            EngineEvent::Error(err) => {
                self.ui_state.messages.push(format!("引擎错误: {}", err));
            }
        }
        Ok(())
    }

    /// 处理按键
    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.ui_state.view {
            View::Home => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('n') => self.start_new_game().await?,
                KeyCode::Char('h') => self.ui_state.view = View::Help,
                _ => {}
            },
            View::Game => self.handle_game_key(key).await?,
            View::Help => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.ui_state.view = View::Home,
                _ => {}
            },
        }
        Ok(())
    }

    async fn start_new_game(&mut self) -> Result<()> {
        // 如果引擎未初始化，尝试初始化
        if self.engine_actor.is_none() {
            self.initialize_engine_actor().await;
        }

        if let Some(actor) = &self.engine_actor {
            // 停止之前的思考
            let _ = actor.send(EngineCommand::Stop).await;

            // 初始化游戏状态
            let state = GameState::new();
            let fen = state.to_fen();
            
            // 设置局面
            actor.send(EngineCommand::SetPosition(fen)).await?;
            
            self.game_state = Some(state);
            self.ui_state.view = View::Game;
            self.ui_state.cursor = Position { row: 9, col: 4 }; // 初始光标在帅的位置 (假设执红在下)
            self.ui_state.messages.push("新游戏开始".to_string());
        } else {
            self.ui_state.messages.push("无法开始游戏：引擎未就绪".to_string());
        }
        
        Ok(())
    }

    async fn handle_game_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.ui_state.view = View::Home, // 返回主菜单而不是退出
            KeyCode::Up => self.move_cursor(0, 1),
            KeyCode::Down => self.move_cursor(0, -1),
            KeyCode::Left => self.move_cursor(-1, 0),
            KeyCode::Right => self.move_cursor(1, 0),
            KeyCode::Enter | KeyCode::Char(' ') => self.handle_selection().await?,
            _ => {}
        }
        Ok(())
    }

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let mut new_col = self.ui_state.cursor.col as i32 + dx;
        let mut new_row = self.ui_state.cursor.row as i32 + dy;
        
        // 限制范围
        new_col = new_col.clamp(0, 8);
        new_row = new_row.clamp(0, 9);
        
        self.ui_state.cursor = Position {
            col: new_col as usize,
            row: new_row as usize,
        };
    }

    async fn handle_selection(&mut self) -> Result<()> {
        // Clone state to avoid mutable borrow conflict if needed, 
        // but here we are mutating self, so we need to be careful.
        // We need to access self.game_state and self.ui_state.
        
        // Temporarily take state out? No, just access fields carefully.
        if self.game_state.is_none() {
            return Ok(());
        }

        let cursor = self.ui_state.cursor;
        let selected = self.ui_state.selected;
        
        if let Some(selected_pos) = selected {
            // 尝试移动
            let move_str = format_move(selected_pos, cursor);
            let mut move_success = false;
            let mut fen = String::new();
            
            if let Some(state) = &mut self.game_state {
                // 验证移动是否合法
                // GameState::apply_move checks validity
                if state.apply_move(&move_str).is_ok() {
                    move_success = true;
                    fen = state.to_fen();
                }
            }
            
            if move_success {
                self.ui_state.selected = None;
                self.ui_state.messages.push(format!("移动: {}", move_str));
                
                // 通知引擎
                if let Some(actor) = &self.engine_actor {
                    actor.send(EngineCommand::SetPosition(fen)).await?;
                    actor.send(EngineCommand::Go(Some(3000))).await?; // 思考 3 秒
                    self.ui_state.messages.push("引擎思考中...".to_string());
                }
            } else {
                 self.ui_state.messages.push("无效移动".to_string());
            }
        } else {
            // 尝试选中
             if let Some(state) = &self.game_state {
                if let Some(piece) = state.board[cursor.row][cursor.col] {
                    if piece.color == state.current_player {
                        self.ui_state.selected = Some(cursor);
                    } else {
                        self.ui_state.messages.push("不能选择对方棋子".to_string());
                    }
                }
             }
        }
        
        Ok(())
    }
}

fn format_move(from: Position, to: Position) -> String {
    let col_chars = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
    let row_chars = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    
    format!("{}{}{}{}", 
        col_chars[from.col], row_chars[from.row],
        col_chars[to.col], row_chars[to.row]
    )
}

fn parse_move(move_str: &str) -> Option<(Position, Position)> {
    if move_str.len() < 4 { return None; }
    let chars: Vec<char> = move_str.chars().collect();
    
    let from_col = match chars[0] {
        'a'..='i' => chars[0] as usize - 'a' as usize,
        _ => return None,
    };
    let from_row = match chars[1] {
        '0'..='9' => chars[1] as usize - '0' as usize,
        _ => return None,
    };
    
    let to_col = match chars[2] {
        'a'..='i' => chars[2] as usize - 'a' as usize,
        _ => return None,
    };
    let to_row = match chars[3] {
        '0'..='9' => chars[3] as usize - '0' as usize,
        _ => return None,
    };
    
    Some((
        Position { row: from_row, col: from_col },
        Position { row: to_row, col: to_col }
    ))
}
