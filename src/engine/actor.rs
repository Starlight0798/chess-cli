use crate::engine::protocol::{EngineEvent, EngineProtocol, GoParams};
use anyhow::Result;
use tokio::sync::mpsc;

/// 引擎控制命令
#[derive(Debug)]
pub enum EngineCommand {
    #[allow(dead_code)]
    Init,
    /// 新游戏
    NewGame,
    SetPosition {
        fen: String,
        moves: Option<Vec<String>>,
    },
    /// 思考命令
    Go(GoParams),
    Stop,
    PonderHit,
    #[allow(dead_code)]
    SetOption {
        name: String,
        value: Option<String>,
    },
    #[allow(dead_code)]
    Quit,
}

/// 引擎 Actor，负责处理引擎通信
pub struct EngineActor {
    tx: mpsc::Sender<EngineCommand>,
}

impl EngineActor {
    /// 创建新的引擎 Actor
    pub fn new(
        mut engine: Box<dyn EngineProtocol>,
    ) -> (Self, mpsc::UnboundedReceiver<EngineEvent>) {
        let (tx_cmd, mut rx_cmd) = mpsc::channel(32);
        let (tx_event, rx_event) = mpsc::unbounded_channel();

        // 将事件发送器传递给引擎
        let tx_event_clone = tx_event.clone();

        tokio::spawn(async move {
            // 我们先设置事件发送器，以便捕获任何早期事件。
            // 虽然 UciEngine::init 会处理握手，但我们希望确保通信通道早已建立。
            if let Err(e) = engine.set_event_sender(tx_event_clone).await {
                let _ = tx_event.send(EngineEvent::Error(format!(
                    "Failed to set event sender: {}",
                    e
                )));
                return;
            }

            while let Some(cmd) = rx_cmd.recv().await {
                match cmd {
                    EngineCommand::Init => {
                        if let Err(e) = engine.init().await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        } else {
                            // UciEngine::init 不再包含 readyok，所以我们只发送 Ready 事件如果手动 ready?
                            // 其实 Init 在 Actor 模式下很少用，如果用，通常也需要 ready。
                            // 这里我们手动调用 ready 确保兼容性。
                            if let Err(e) = engine.ready().await {
                                let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                            }
                            // ready 会等待 readyok，但 Actor 模式下 ready() 发送 isready 后，reader 已经被接管，所以 ready() 不会阻塞等待。
                            // 它只会发送 isready。然后 reader loop 会收到 readyok 并发送 EngineEvent::Ready。
                            // 所以这里不需要手动发送 EngineEvent::Ready。
                        }
                    }
                    EngineCommand::NewGame => {
                        if let Err(e) = engine.new_game().await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        } else {
                            // new_game calls ready(), which sends isready.
                            // The response readyok will be handled by the reader loop and emit EngineEvent::Ready.
                            // So we don't need to manually send Ready here anymore?
                            // Wait, new_game in UciEngine calls ready().
                            // In Async mode (reader taken), ready() just sends isready and returns.
                            // So we rely on the loop to catch readyok.
                        }
                    }
                    EngineCommand::SetPosition { fen, moves } => {
                        if let Err(e) = engine.set_position(&fen, moves.as_deref()).await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                    EngineCommand::Go(params) => {
                        if let Err(e) = engine.go(params).await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                    EngineCommand::Stop => {
                        let _ = engine.stop().await;
                    }
                    EngineCommand::PonderHit => {
                        if let Err(e) = engine.ponderhit().await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                    EngineCommand::SetOption { name, value } => {
                        if let Err(e) = engine.set_option(&name, value.as_deref()).await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                    EngineCommand::Quit => {
                        let _ = engine.quit().await;
                        break;
                    }
                }
            }
        });

        (Self { tx: tx_cmd }, rx_event)
    }

    /// 发送命令到引擎
    pub async fn send(&self, cmd: EngineCommand) -> Result<()> {
        self.tx
            .send(cmd)
            .await
            .map_err(|_| anyhow::anyhow!("引擎已停止"))
    }
}
