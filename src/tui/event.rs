use crate::engine::protocol::EngineEvent;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::sync::mpsc;

/// 终端事件
#[derive(Clone, Debug)]
pub enum Event {
    /// 按键事件
    Key(KeyEvent),
    /// 定时器滴答
    Tick,
    /// 引擎事件
    Engine(EngineEvent),
    /// 错误
    Error,
}

/// 事件处理器
pub struct EventHandler {
    pub sender: mpsc::UnboundedSender<Event>, // Made public to allow App to clone
    rx: mpsc::UnboundedReceiver<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// 创建新的事件处理器
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        let task = tokio::spawn(async move {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).expect("failed to poll new events") {
                    match event::read().expect("failed to read events") {
                        CEvent::Key(key) => {
                            if let Err(_) = tx.send(Event::Key(key)) {
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if let Err(_) = tx.send(Event::Tick) {
                        return;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self { rx, sender: _tx, _task: task }
    }

    /// 获取下一个事件
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
