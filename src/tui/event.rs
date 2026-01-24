use crate::engine::protocol::EngineEvent;
use crossterm::event::{self, Event as CEvent, KeyEvent, KeyEventKind};
use std::time::{Duration, Instant};
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
    #[allow(dead_code)]
    Error,
}

/// 事件处理器
pub struct EventHandler {
    /// 事件发送器，允许 App 克隆使用
    pub sender: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
    /// 后台轮询任务句柄
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

                match event::poll(timeout) {
                    Ok(true) => match event::read() {
                        Ok(CEvent::Key(key)) => {
                            if (key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat)
                                && tx.send(Event::Key(key)).is_err()
                            {
                                return;
                            }
                        }
                        Ok(_) => {} // Ignore non-key events (mouse, resize, etc.)
                        Err(_) => {
                            // Terminal read error - send error event and continue
                            let _ = tx.send(Event::Error);
                        }
                    },
                    Ok(false) => {} // No event available, continue
                    Err(_) => {
                        // Terminal poll error - send error event and continue
                        let _ = tx.send(Event::Error);
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if tx.send(Event::Tick).is_err() {
                        return;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self {
            rx,
            sender: _tx,
            _task: task,
        }
    }

    /// 获取下一个事件
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
