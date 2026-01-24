use crate::engine::actor::EngineCommand;
use crate::game::{FenProcessor, GameState, PlayerColor, Position};
use crate::tui::App;
use crate::tui::config::StrategyType;
use crate::tui::ui_state::View;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
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
                self.handle_settings_change(key.code);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_settings_change(&mut self, code: KeyCode) {
        let is_left = code == KeyCode::Left;
        match self.ui_state.menu_index {
            0 => {
                self.ui_state.game_config.player_side =
                    self.ui_state.game_config.player_side.opponent();
            }
            1 => {
                if !self.ui_state.engine_list.is_empty() {
                    let current = &self.ui_state.game_config.engine_name;
                    let idx = self
                        .ui_state
                        .engine_list
                        .iter()
                        .position(|x| x == current)
                        .unwrap_or(0);
                    let next_idx = if is_left {
                        if idx == 0 {
                            self.ui_state.engine_list.len() - 1
                        } else {
                            idx - 1
                        }
                    } else {
                        (idx + 1) % self.ui_state.engine_list.len()
                    };
                    self.ui_state.game_config.engine_name =
                        self.ui_state.engine_list[next_idx].clone();
                }
            }
            2 => {
                if is_left {
                    if self.ui_state.game_config.multipv > 1 {
                        self.ui_state.game_config.multipv -= 1;
                    }
                } else if self.ui_state.game_config.multipv < 5 {
                    self.ui_state.game_config.multipv += 1;
                }
            }
            3 => {
                if is_left {
                    if self.ui_state.game_config.difficulty_level > 0 {
                        self.ui_state.game_config.difficulty_level -= 1;
                    }
                } else if self.ui_state.game_config.difficulty_level < 20 {
                    self.ui_state.game_config.difficulty_level += 1;
                }
            }
            4 => {
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
                let next_idx = if is_left {
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
                if is_left {
                    if self.ui_state.game_config.move_time > 100 {
                        self.ui_state.game_config.move_time -= 100;
                    }
                } else {
                    self.ui_state.game_config.move_time += 100;
                }
            }
            6 => {
                if is_left {
                    if self.ui_state.game_config.depth > 1 {
                        self.ui_state.game_config.depth -= 1;
                    }
                } else if self.ui_state.game_config.depth < 128 {
                    self.ui_state.game_config.depth += 1;
                }
            }
            7 => {
                if is_left {
                    if self.ui_state.game_config.game_time > 1 {
                        self.ui_state.game_config.game_time -= 1;
                    }
                } else {
                    self.ui_state.game_config.game_time += 1;
                }
            }
            8 => {
                if is_left {
                    if self.ui_state.game_config.game_inc > 0 {
                        self.ui_state.game_config.game_inc -= 1;
                    }
                } else {
                    self.ui_state.game_config.game_inc += 1;
                }
            }
            9 => {
                self.ui_state.view = View::Home;
            }
            _ => {}
        }
    }

    pub async fn start_new_game(&mut self) -> Result<()> {
        self.initialize_engine_actor().await;

        if let Some(actor) = &self.engine_actor {
            let _ = actor.send(EngineCommand::Stop).await;
            actor.send(EngineCommand::NewGame).await?;

            let mut state = GameState::new();
            state.flipped = self.ui_state.game_config.player_side == PlayerColor::Black;
            let fen = state.to_fen();

            actor
                .send(EngineCommand::SetPosition {
                    fen: fen.clone(),
                    moves: None,
                })
                .await?;

            self.game_state = Some(state);
            self.ui_state.view = View::Game;
            self.ui_state.cursor = if self.ui_state.game_config.player_side == PlayerColor::Black {
                Position { row: 9, col: 4 }
            } else {
                Position { row: 0, col: 4 }
            };

            self.ui_state.messages.push("新游戏开始".to_string());

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

    pub async fn continue_game(&mut self) -> Result<()> {
        self.initialize_engine_actor().await;

        if let Some(state) = &self.game_state {
            if let Some(actor) = &self.engine_actor {
                let fen = state.to_fen();
                actor
                    .send(EngineCommand::SetPosition { fen, moves: None })
                    .await?;
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
            KeyCode::Char('q') => self.ui_state.view = View::Home,
            KeyCode::Char('u') => self.handle_undo(),
            KeyCode::Char('r') => self.handle_redo(),
            KeyCode::Char('s') => self.save_game(),
            KeyCode::Char('l') => {
                if let Err(e) = self.load_game().await {
                    self.ui_state.messages.push(format!("加载失败: {}", e));
                }
            }
            KeyCode::Up => {
                let dy = if self.game_state.as_ref().is_some_and(|s| s.flipped) {
                    -1
                } else {
                    1
                };
                self.move_cursor(0, dy);
            }
            KeyCode::Down => {
                let dy = if self.game_state.as_ref().is_some_and(|s| s.flipped) {
                    1
                } else {
                    -1
                };
                self.move_cursor(0, dy);
            }
            KeyCode::Left => {
                let dx = if self.game_state.as_ref().is_some_and(|s| s.flipped) {
                    1
                } else {
                    -1
                };
                self.move_cursor(dx, 0);
            }
            KeyCode::Right => {
                let dx = if self.game_state.as_ref().is_some_and(|s| s.flipped) {
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

    fn handle_undo(&mut self) {
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

    fn handle_redo(&mut self) {
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

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let mut new_col = self.ui_state.cursor.col as i32 + dx;
        let mut new_row = self.ui_state.cursor.row as i32 + dy;

        new_col = new_col.clamp(0, 8);
        new_row = new_row.clamp(0, 9);

        self.ui_state.cursor = Position {
            col: new_col as usize,
            row: new_row as usize,
        };
    }

    async fn handle_selection(&mut self) -> Result<()> {
        if self.game_state.is_none() {
            return Ok(());
        }

        let cursor = self.ui_state.cursor;
        let selected = self.ui_state.selected;

        if let Some(selected_pos) = selected {
            if selected_pos == cursor {
                self.ui_state.selected = None;
                self.ui_state.legal_moves.clear();
                return Ok(());
            }

            let col_chars = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
            let move_str = format!(
                "{}{}{}{}",
                col_chars[selected_pos.col], selected_pos.row, col_chars[cursor.col], cursor.row
            );

            if let Some(state) = &mut self.game_state {
                let move_display = state.move_to_chinese(&move_str).unwrap_or(move_str.clone());
                if state.apply_move(&move_str).is_ok() {
                    self.ui_state.selected = None;
                    self.ui_state.legal_moves.clear();
                    self.ui_state
                        .messages
                        .push(format!("移动: {}", move_display));

                    self.trigger_engine_after_move(&move_str).await?;
                } else {
                    self.ui_state.messages.push("无效移动".to_string());
                }
            }
        } else if let Some(state) = &self.game_state
            && let Some(piece) = state.board[cursor.row][cursor.col]
        {
            if piece.color == state.current_player {
                self.ui_state.selected = Some(cursor);
                self.ui_state.legal_moves = state.get_piece_legal_moves(cursor);
            } else {
                self.ui_state.messages.push("不能选择对方棋子".to_string());
            }
        }

        Ok(())
    }

    async fn trigger_engine_after_move(&mut self, move_str: &str) -> Result<()> {
        if let Some(actor) = &self.engine_actor {
            let mut hit = false;
            if let Some(ponder) = &self.ui_state.ponder_move
                && *ponder == move_str
            {
                hit = true;
                actor.send(EngineCommand::PonderHit).await?;
                self.ui_state.messages.push("Ponder Hit!".to_string());
            }

            if !hit {
                if self.ui_state.ponder_move.is_some() {
                    actor.send(EngineCommand::Stop).await?;
                    self.should_ignore_next_best_move = true;
                }

                if let Some(state) = &self.game_state {
                    let fen = state.to_fen();
                    actor
                        .send(EngineCommand::SetPosition { fen, moves: None })
                        .await?;
                }

                let params = self.ui_state.game_config.get_go_params();
                actor.send(EngineCommand::Go(params)).await?;
                self.ui_state.messages.push("引擎思考中...".to_string());
            }

            self.ui_state.ponder_move = None;
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

    pub async fn load_game(&mut self) -> Result<()> {
        match std::fs::read_to_string("saved_game.fen") {
            Ok(fen) => match FenProcessor::parse_fen(&fen) {
                Ok(state) => {
                    self.game_state = Some(state);
                    if let Some(actor) = &self.engine_actor {
                        let _ = actor
                            .send(EngineCommand::SetPosition { fen, moves: None })
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
            },
            Err(e) => self
                .ui_state
                .messages
                .push(format!("加载失败 (文件读取错误): {}", e)),
        }
        Ok(())
    }
}
