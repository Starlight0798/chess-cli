use crate::engine::actor::{EngineActor, EngineCommand};
use crate::engine::protocol::{EngineEvent, GoParams};
use crate::engine::{EngineManager, EngineType};
use crate::game::GameState;
use crate::tui::event::{Event, EventHandler};
use crate::tui::ui;
use crate::tui::ui_state::UiState;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct App {
    pub game_state: Option<GameState>,
    pub engine_actor: Option<EngineActor>,
    pub engine_manager: EngineManager,
    pub ui_state: UiState,
    pub should_quit: bool,
    pub event_sender: Option<mpsc::UnboundedSender<Event>>,
    pub active_engine_name: Option<String>,
    pub should_ignore_next_best_move: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let (engine_manager, config_msg) = EngineManager::new()?;
        let mut ui_state = UiState::default();
        ui_state.messages.push(config_msg);
        ui_state.engine_list = engine_manager.list_engines();

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
            should_ignore_next_best_move: false,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        terminal.clear()?;

        let mut events = EventHandler::new(Duration::from_millis(250));
        self.event_sender = Some(events.sender.clone());

        while !self.should_quit {
            terminal.draw(|f| ui::draw(f, self))?;

            match events.next().await {
                Some(Event::Key(key)) => self.handle_key(key).await?,
                Some(Event::Tick) => {}
                Some(Event::Engine(event)) => self.handle_engine_event(event).await?,
                _ => {}
            }
        }

        ratatui::restore();
        Ok(())
    }

    pub async fn initialize_engine_actor(&mut self) {
        let engine_name = &self.ui_state.game_config.engine_name;
        let mut engine_changed = false;

        if let Some(active_name) = &self.active_engine_name {
            if active_name != engine_name {
                self.engine_actor = None;
                self.active_engine_name = None;
                engine_changed = true;
            }
        } else {
            engine_changed = true;
        }

        if !engine_changed && self.engine_actor.is_some() {
            self.apply_engine_options().await;
            return;
        }

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
                self.ui_state
                    .messages
                    .push(format!("引擎 {} 初始化成功", engine_name));

                self.apply_engine_options().await;
            }
            Err(e) => {
                self.ui_state.messages.push(format!("引擎启动失败: {}", e));
            }
        }
    }

    async fn apply_engine_options(&self) {
        if let Some(actor) = &self.engine_actor {
            let _ = actor
                .send(EngineCommand::SetOption {
                    name: "MultiPV".to_string(),
                    value: Some(self.ui_state.game_config.multipv.to_string()),
                })
                .await;

            if self.ui_state.game_config.difficulty_level < 20 {
                let elo = 1350 + self.ui_state.game_config.difficulty_level * 75;
                let _ = actor
                    .send(EngineCommand::SetOption {
                        name: "UCI_LimitStrength".to_string(),
                        value: Some("true".to_string()),
                    })
                    .await;
                let _ = actor
                    .send(EngineCommand::SetOption {
                        name: "UCI_Elo".to_string(),
                        value: Some(elo.to_string()),
                    })
                    .await;
            } else {
                let _ = actor
                    .send(EngineCommand::SetOption {
                        name: "UCI_LimitStrength".to_string(),
                        value: Some("false".to_string()),
                    })
                    .await;
            }
        }
    }

    async fn handle_engine_event(&mut self, event: EngineEvent) -> Result<()> {
        match event {
            EngineEvent::Thinking(info) => {
                let pv_chinese = if let Some(state) = &self.game_state {
                    let mut temp_state = state.clone();
                    if let Some(ponder_move) = &self.ui_state.ponder_move {
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
                if self.should_ignore_next_best_move {
                    self.should_ignore_next_best_move = false;
                    self.ui_state.engine_info = None;
                    self.ui_state.engine_pv_chinese = None;
                    return Ok(());
                }

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
                    if state.apply_move(&best_move).is_ok() {
                        if let Some(ponder) = ponder_move {
                            self.ui_state.ponder_move = Some(ponder.clone());

                            let ponder_display =
                                state.move_to_chinese(&ponder).unwrap_or(ponder.clone());

                            self.ui_state
                                .messages
                                .push(format!("引擎后台思考: {}", ponder_display));

                            let fen = state.to_fen();

                            if let Some(actor) = &self.engine_actor {
                                actor
                                    .send(EngineCommand::SetPosition {
                                        fen,
                                        moves: Some(vec![ponder.clone()]),
                                    })
                                    .await?;

                                let params = GoParams {
                                    ponder: true,
                                    wtime: Some(300000),
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
}
