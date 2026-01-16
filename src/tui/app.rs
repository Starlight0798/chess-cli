use crate::engine::actor::{EngineActor, EngineCommand};
use crate::engine::protocol::{EngineEvent, EngineOption, EngineThinkingInfo, GoParams};
use crate::engine::{EngineManager, EngineType};
use crate::game::{GameState, PlayerColor, Position};
use crate::tui::event::{Event, EventHandler};
use crate::tui::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

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

/// 游戏配置
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub player_side: PlayerColor,
    pub engine_name: String,
    pub multipv: u32,
    pub difficulty_level: u32, // 0-20, mapped to UCI_Elo or LimitStrength
    
    // 策略相关配置
    pub strategy: StrategyType,
    pub move_time: u64,     // ms
    pub depth: u32,         // plies
    pub game_time: u64,     // minutes
    pub game_inc: u64,      // seconds
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            player_side: PlayerColor::Red,
            engine_name: "pikafish".to_string(),
            multipv: 1,
            difficulty_level: 20, // Max
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
            },
            StrategyType::Infinite => params.infinite = true,
        }
        params
    }
}

/// UI 视图状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    Game,
    Settings,
    Help,
}

/// UI 状态
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

/// 应用程序
pub struct App {
    pub game_state: Option<GameState>,
    pub engine_actor: Option<EngineActor>,
    pub engine_manager: EngineManager,
    pub ui_state: UiState,
    pub should_quit: bool,
    pub event_sender: Option<mpsc::UnboundedSender<Event>>,
    pub active_engine_name: Option<String>,
}

impl App {
    /// 创建新应用程序
    pub fn new() -> Result<Self> {
        let engine_manager = EngineManager::new()?;
        let mut ui_state = UiState::default();
        ui_state.engine_list = engine_manager.list_engines();
        
        // 如果有引擎，默认选择第一个
        if let Some(first) = ui_state.engine_list.first() {
            ui_state.game_config.engine_name = first.clone();
        }

        Ok(Self {
            game_state: None,
            engine_actor: None,
            engine_manager,
            ui_state,
            should_quit: false,
            event_sender: None,
            active_engine_name: None,
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

        // 主循环
        while !self.should_quit {
            // 渲染
            terminal.draw(|f| ui::draw(f, self))?;

            // 处理事件
            match events.next().await {
                Some(Event::Key(key)) => self.handle_key(key).await?,
                Some(Event::Tick) => {}
                Some(Event::Engine(event)) => self.handle_engine_event(event).await?,
                _ => {}
            }
        }

        // 恢复终端
        ratatui::restore();
        Ok(())
    }

    /// 初始化引擎 Actor
    async fn initialize_engine_actor(&mut self) {
        let engine_name = &self.ui_state.game_config.engine_name;
        let mut engine_changed = false;

        // 如果已有引擎且名称一致，不需要重新初始化
        if let Some(active_name) = &self.active_engine_name {
            if active_name != engine_name {
                // 引擎不同，先清空旧的
                self.engine_actor = None;
                self.active_engine_name = None;
                engine_changed = true;
            }
        } else {
            engine_changed = true;
        }

        // 如果引擎未变且已存在，仅更新选项
        if !engine_changed && self.engine_actor.is_some() {
            self.apply_engine_options().await;
            return;
        }
        
        // 创建当前配置的引擎
        let engine_type = if engine_name.to_lowercase() == "pikafish" {
             EngineType::Pikafish
        } else {
             EngineType::Other(engine_name.clone())
        };

        match self
            .engine_manager
            .create_engine_instance(&engine_type)
            .await
        {
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
                self.active_engine_name = Some(engine_name.clone());
                self.ui_state.messages.push(format!("引擎 {} 初始化成功", engine_name));
                
                // 应用配置选项
                self.apply_engine_options().await;
            }
            Err(e) => {
                self.ui_state.messages.push(format!("引擎启动失败: {}", e));
            }
        }
    }

    /// 应用引擎配置选项
    async fn apply_engine_options(&self) {
        if let Some(actor) = &self.engine_actor {
            // 设置 MultiPV
            let _ = actor.send(EngineCommand::SetOption {
                name: "MultiPV".to_string(),
                value: Some(self.ui_state.game_config.multipv.to_string()),
            }).await;

            // 设置难度 (示例：UCI_Elo)
            if self.ui_state.game_config.difficulty_level < 20 {
                let elo = 1350 + self.ui_state.game_config.difficulty_level * 75;
                let _ = actor.send(EngineCommand::SetOption {
                    name: "UCI_LimitStrength".to_string(),
                    value: Some("true".to_string()),
                }).await;
                let _ = actor.send(EngineCommand::SetOption {
                    name: "UCI_Elo".to_string(),
                    value: Some(elo.to_string()),
                }).await;
            } else {
                let _ = actor.send(EngineCommand::SetOption {
                    name: "UCI_LimitStrength".to_string(),
                    value: Some("false".to_string()),
                }).await;
            }
        }
    }

    /// 处理引擎事件
    async fn handle_engine_event(&mut self, event: EngineEvent) -> Result<()> {
        match event {
            EngineEvent::Thinking(info) => {
                // 预计算中文 PV
                let pv_chinese = if let Some(state) = &self.game_state {
                    let mut temp_state = state.clone();
                    // 如果在 Ponder，说明引擎是在思考 ponder_move 之后的局面
                    // 所以我们需要先模拟走出这一步，再解析 PV
                    if let Some(ponder_move) = &self.ui_state.ponder_move {
                        // 忽略错误，如果 ponder_move 非法，就尽力解析
                        let _ = temp_state.apply_move(ponder_move);
                    }
                    
                    if let Some(pv) = &info.pv {
                        Some(temp_state.pv_to_chinese(pv).join(" "))
                    } else {
                        None
                    }
                } else {
                    None
                };

                self.ui_state.engine_pv_chinese = pv_chinese;
                self.ui_state.engine_info = Some(info);
            }
            EngineEvent::BestMove {
                best_move,
                ponder_move,
            } => {
                // 清除旧的思考信息，避免在界面上显示过时的 PV
                self.ui_state.engine_info = None;
                self.ui_state.engine_pv_chinese = None;

                let best_move_display = if let Some(state) = &self.game_state {
                    state
                        .move_to_chinese(&best_move)
                        .unwrap_or(best_move.clone())
                } else {
                    best_move.clone()
                };

                self.ui_state
                    .messages
                    .push(format!("引擎着法: {}", best_move_display));
                if let Some(state) = &mut self.game_state {
                    // 解析并应用着法
                    if state.apply_move(&best_move).is_ok() {
                        // 如果有 ponder_move，开始后台思考
                        if let Some(ponder) = ponder_move {
                            self.ui_state.ponder_move = Some(ponder.clone());
                            
                            let ponder_display = state
                                .move_to_chinese(&ponder)
                                .unwrap_or(ponder.clone());
                            
                            self.ui_state
                                .messages
                                .push(format!("引擎后台思考: {}", ponder_display));

                            // 更新引擎内部状态（需要发送新的位置）
                            // 注意：apply_move 已经更新了 game_state
                            // 我们需要把 best_move 之后加上 ponder_move 发送给引擎作为思考位置
                            // 但 UCI 协议中，ponder 是在 bestmove 后立即发送 go ponder
                            // 此时引擎已经知道位置了吗？
                            // UCI 协议：
                            // GUI receives "bestmove e2e4 ponder e7e5"
                            // GUI records e2e4 played.
                            // GUI sends "position startpos moves ... e2e4 e7e5" (the position to ponder on)
                            // GUI sends "go ponder wtime ... btime ..."
                            
                            // 获取当前 FEN（已经是 best_move 后的状态）
                            let fen = state.to_fen();
                            
                            if let Some(actor) = &self.engine_actor {
                                // 发送 ponder 位置
                                actor
                                    .send(EngineCommand::SetPosition {
                                        fen,
                                        moves: Some(vec![ponder.clone()]),
                                    })
                                    .await?;
                                
                                // 发送 go ponder
                                let params = GoParams {
                                    ponder: true,
                                    wtime: Some(300000), // 给予足够的时间，实际上应该根据游戏时间设定
                                    btime: Some(300000),
                                    ..Default::default()
                                };
                                actor.send(EngineCommand::Go(params)).await?;
                            }
                        } else {
                            self.ui_state.ponder_move = None;
                        }
                    } else {
                        self.ui_state
                            .messages
                            .push(format!("引擎返回非法着法: {}", best_move));
                    }
                }
            }
            EngineEvent::Option(opt) => {
                self.ui_state.engine_options.push(opt);
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
                KeyCode::Char('c') => {
                    if self.game_state.is_some() {
                        self.continue_game().await?;
                    }
                }
                KeyCode::Char('s') => self.ui_state.view = View::Settings,
                KeyCode::Char('h') => self.ui_state.view = View::Help,
                _ => {}
            },
            View::Game => self.handle_game_key(key).await?,
            View::Settings => self.handle_settings_key(key).await?,
            View::Help => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.ui_state.view = View::Home,
                _ => {}
            },
        }
        Ok(())
    }

    async fn handle_settings_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.ui_state.view = View::Home,
            KeyCode::Up => {
                if self.ui_state.menu_index > 0 {
                    self.ui_state.menu_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.ui_state.menu_index < 9 {
                    self.ui_state.menu_index += 1;
                }
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                match self.ui_state.menu_index {
                    0 => {
                        // Toggle Player Side
                        self.ui_state.game_config.player_side = self.ui_state.game_config.player_side.opponent();
                    }
                    1 => {
                        // Cycle Engine
                        if !self.ui_state.engine_list.is_empty() {
                             let current = &self.ui_state.game_config.engine_name;
                             let idx = self.ui_state.engine_list.iter().position(|x| x == current).unwrap_or(0);
                             let next_idx = if key.code == KeyCode::Left {
                                 if idx == 0 { self.ui_state.engine_list.len() - 1 } else { idx - 1 }
                             } else {
                                 (idx + 1) % self.ui_state.engine_list.len()
                             };
                             self.ui_state.game_config.engine_name = self.ui_state.engine_list[next_idx].clone();
                        }
                    }
                    2 => {
                        // MultiPV (1-5)
                        match key.code {
                            KeyCode::Left => {
                                if self.ui_state.game_config.multipv > 1 {
                                    self.ui_state.game_config.multipv -= 1;
                                }
                            }
                            _ => {
                                if self.ui_state.game_config.multipv < 5 {
                                    self.ui_state.game_config.multipv += 1;
                                }
                            }
                        }
                    }
                    3 => {
                        // Difficulty (0-20)
                        match key.code {
                            KeyCode::Left => {
                                if self.ui_state.game_config.difficulty_level > 0 {
                                    self.ui_state.game_config.difficulty_level -= 1;
                                }
                            }
                            _ => {
                                if self.ui_state.game_config.difficulty_level < 20 {
                                    self.ui_state.game_config.difficulty_level += 1;
                                }
                            }
                        }
                    }
                    4 => {
                        // Strategy
                        let strategies = [
                            StrategyType::MoveTime,
                            StrategyType::Depth,
                            StrategyType::GameTime,
                            StrategyType::Infinite,
                        ];
                        let current_idx = strategies
                            .iter()
                            .position(|s| *s == self.ui_state.game_config.strategy)
                            .unwrap_or(0);
                        let next_idx = if key.code == KeyCode::Left {
                            if current_idx == 0 {
                                strategies.len() - 1
                            } else {
                                current_idx - 1
                            }
                        } else {
                            (current_idx + 1) % strategies.len()
                        };
                        self.ui_state.game_config.strategy = strategies[next_idx].clone();
                    }
                    5 => {
                        // Move Time (ms)
                        match key.code {
                            KeyCode::Left => {
                                if self.ui_state.game_config.move_time > 100 {
                                    self.ui_state.game_config.move_time -= 100;
                                }
                            }
                            _ => {
                                self.ui_state.game_config.move_time += 100;
                            }
                        }
                    }
                    6 => {
                        // Depth
                        match key.code {
                            KeyCode::Left => {
                                if self.ui_state.game_config.depth > 1 {
                                    self.ui_state.game_config.depth -= 1;
                                }
                            }
                            _ => {
                                if self.ui_state.game_config.depth < 128 {
                                    self.ui_state.game_config.depth += 1;
                                }
                            }
                        }
                    }
                    7 => {
                        // Game Time (min)
                        match key.code {
                            KeyCode::Left => {
                                if self.ui_state.game_config.game_time > 1 {
                                    self.ui_state.game_config.game_time -= 1;
                                }
                            }
                            _ => {
                                self.ui_state.game_config.game_time += 1;
                            }
                        }
                    }
                    8 => {
                        // Game Inc (s)
                        match key.code {
                            KeyCode::Left => {
                                if self.ui_state.game_config.game_inc > 0 {
                                    self.ui_state.game_config.game_inc -= 1;
                                }
                            }
                            _ => {
                                self.ui_state.game_config.game_inc += 1;
                            }
                        }
                    }
                    9 => {
                        self.ui_state.view = View::Home;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn start_new_game(&mut self) -> Result<()> {
        // 确保引擎初始化
        self.initialize_engine_actor().await;

        if let Some(actor) = &self.engine_actor {
            // 停止之前的思考
            let _ = actor.send(EngineCommand::Stop).await;

            // 发送新游戏命令
            actor.send(EngineCommand::NewGame).await?;

            // 初始化游戏状态
            let mut state = GameState::new();
            
            // 设置玩家视角
            state.flipped = self.ui_state.game_config.player_side == PlayerColor::Black;

            let fen = state.to_fen();

            // 设置局面
            actor
                .send(EngineCommand::SetPosition {
                    fen: fen.clone(),
                    moves: None,
                })
                .await?;

            self.game_state = Some(state);
            self.ui_state.view = View::Game;
            // 初始光标位置
            self.ui_state.cursor = if self.ui_state.game_config.player_side == PlayerColor::Black {
                 Position { row: 9, col: 4 } // 黑将位置
            } else {
                 Position { row: 0, col: 4 } // 红帅位置
            };
            
            self.ui_state.messages.push("新游戏开始".to_string());

            // 如果玩家执黑，引擎（红方）先行
            if self.ui_state.game_config.player_side == PlayerColor::Black {
                 let params = self.ui_state.game_config.get_go_params();
                 actor.send(EngineCommand::Go(params)).await?;
                 self.ui_state.messages.push("引擎思考中...".to_string());
            }

        } else {
            self.ui_state
                .messages
                .push("无法开始游戏：引擎未就绪".to_string());
        }

        Ok(())
    }

    async fn continue_game(&mut self) -> Result<()> {
        // 确保引擎初始化并应用最新配置
        self.initialize_engine_actor().await;

        if let Some(state) = &self.game_state {
             if let Some(actor) = &self.engine_actor {
                 // 同步当前局面到引擎
                 let fen = state.to_fen();
                 actor.send(EngineCommand::SetPosition {
                     fen,
                     moves: None,
                 }).await?;
             }
             
             self.ui_state.view = View::Game;
             self.ui_state.messages.push("继续游戏".to_string());
        }

        Ok(())
    }

    async fn handle_game_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                if self.ui_state.selected.is_some() {
                    self.ui_state.selected = None;
                    self.ui_state.legal_moves.clear();
                } else {
                    self.ui_state.view = View::Home;
                }
            }
            KeyCode::Char('q') => self.ui_state.view = View::Home, // 返回主菜单而不是退出
            KeyCode::Char('u') => {
                if let Some(state) = &mut self.game_state {
                    match state.undo_move() {
                        Ok(_) => {
                            self.ui_state.messages.push("悔棋成功".to_string());
                            self.ui_state.selected = None;
                            self.ui_state.legal_moves.clear();
                        }
                        Err(e) => self.ui_state.messages.push(format!("悔棋失败: {}", e)),
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(state) = &mut self.game_state {
                    match state.redo_move() {
                        Ok(_) => {
                            self.ui_state.messages.push("重做成功".to_string());
                            self.ui_state.selected = None;
                            self.ui_state.legal_moves.clear();
                        }
                        Err(e) => self.ui_state.messages.push(format!("重做失败: {}", e)),
                    }
                }
            }
            KeyCode::Char('s') => self.save_game(),
            KeyCode::Char('l') => {
                if let Err(e) = self.load_game().await {
                    self.ui_state.messages.push(format!("加载失败: {}", e));
                }
            }
            KeyCode::Up => {
                let dy = if self.game_state.as_ref().map_or(false, |s| s.flipped) {
                    -1
                } else {
                    1
                };
                self.move_cursor(0, dy);
            }
            KeyCode::Down => {
                let dy = if self.game_state.as_ref().map_or(false, |s| s.flipped) {
                    1
                } else {
                    -1
                };
                self.move_cursor(0, dy);
            }
            KeyCode::Left => {
                let dx = if self.game_state.as_ref().map_or(false, |s| s.flipped) {
                    1
                } else {
                    -1
                };
                self.move_cursor(dx, 0);
            }
            KeyCode::Right => {
                let dx = if self.game_state.as_ref().map_or(false, |s| s.flipped) {
                    -1
                } else {
                    1
                };
                self.move_cursor(dx, 0);
            }
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
            // Check if user clicked the same piece to deselect
            if selected_pos == cursor {
                self.ui_state.selected = None;
                self.ui_state.legal_moves.clear();
                return Ok(());
            }

            // 尝试移动
            let col_chars = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
            let move_str = format!(
                "{}{}{}{}",
                col_chars[selected_pos.col], selected_pos.row, col_chars[cursor.col], cursor.row
            );

            if let Some(state) = &mut self.game_state {
                let move_display = state.move_to_chinese(&move_str).unwrap_or(move_str.clone());
                // Try apply move
                if state.apply_move(&move_str).is_ok() {
                    self.ui_state.selected = None;
                    self.ui_state.legal_moves.clear();
                    self.ui_state.messages.push(format!("移动: {}", move_display));

                    if let Some(actor) = &self.engine_actor {
                        let mut hit = false;
                        // Check Ponder Hit
                        if let Some(ponder) = &self.ui_state.ponder_move {
                            if *ponder == move_str {
                                // Hit!
                                hit = true;
                                actor.send(EngineCommand::PonderHit).await?;
                                self.ui_state.messages.push("Ponder Hit!".to_string());
                            }
                        }

                        if !hit {
                            // Stop any thinking
                            actor.send(EngineCommand::Stop).await?;
                            
                            // Send new position
                            let fen = state.to_fen();
                            actor
                                .send(EngineCommand::SetPosition {
                                    fen,
                                    moves: None,
                                })
                                .await?;
                            
                            // Start thinking
                            let params = self.ui_state.game_config.get_go_params();
                            actor.send(EngineCommand::Go(params)).await?;
                            self.ui_state.messages.push("引擎思考中...".to_string());
                        }
                        
                        // Clear ponder move
                        self.ui_state.ponder_move = None;
                    }
                } else {
                    self.ui_state.messages.push("无效移动".to_string());
                }
            }
        } else {
            // 尝试选中
            if let Some(state) = &self.game_state {
                if let Some(piece) = state.board[cursor.row][cursor.col] {
                    if piece.color == state.current_player {
                        self.ui_state.selected = Some(cursor);
                        self.ui_state.legal_moves = state.get_piece_legal_moves(cursor);
                    } else {
                        self.ui_state.messages.push("不能选择对方棋子".to_string());
                    }
                }
            }
        }

        Ok(())
    }
    fn save_game(&mut self) {
        if let Some(state) = &self.game_state {
            let fen = state.to_fen();
            match std::fs::write("saved_game.fen", fen) {
                Ok(_) => self
                    .ui_state
                    .messages
                    .push("游戏已保存到 saved_game.fen".to_string()),
                Err(e) => self.ui_state.messages.push(format!("保存失败: {}", e)),
            }
        } else {
            self.ui_state.messages.push("没有进行中的游戏".to_string());
        }
    }

    async fn load_game(&mut self) -> Result<()> {
        match std::fs::read_to_string("saved_game.fen") {
            Ok(fen) => {
                match crate::game::FenProcessor::parse_fen(&fen) {
                    Ok(state) => {
                        self.game_state = Some(state);
                        // Update engine position
                        if let Some(actor) = &self.engine_actor {
                            let _ = actor
                                .send(EngineCommand::SetPosition {
                                    fen,
                                    moves: None,
                                })
                                .await;
                        }
                        self.ui_state.messages.push("游戏已加载".to_string());
                        self.ui_state.selected = None;
                        self.ui_state.legal_moves.clear();
                    }
                    Err(e) => self
                        .ui_state
                        .messages
                        .push(format!("加载失败 (FEN解析错误): {}", e)),
                }
            }
            Err(e) => self
                .ui_state
                .messages
                .push(format!("加载失败 (文件读取错误): {}", e)),
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn parse_move(move_str: &str) -> Option<(Position, Position)> {
    if move_str.len() < 4 {
        return None;
    }
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
        Position {
            row: from_row,
            col: from_col,
        },
        Position {
            row: to_row,
            col: to_col,
        },
    ))
}
