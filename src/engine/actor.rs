use crate::engine::protocol::{EngineProtocol, EngineEvent};
use anyhow::Result;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum EngineCommand {
    Init,
    SetPosition(String),
    Go(Option<usize>), // think time in ms
    Stop,
    Quit,
}

pub struct EngineActor {
    tx: mpsc::Sender<EngineCommand>,
}

impl EngineActor {
    pub fn new(mut engine: Box<dyn EngineProtocol>) -> (Self, mpsc::UnboundedReceiver<EngineEvent>) {
        let (tx_cmd, mut rx_cmd) = mpsc::channel(32);
        let (tx_event, rx_event) = mpsc::unbounded_channel();

        // Pass sender to engine
        let tx_event_clone = tx_event.clone();

        tokio::spawn(async move {
            // Need to init first or set event sender?
            // Usually we set event sender first so we catch any early events, 
            // but engine init might block if it waits for handshake.
            // Our UciEngine::init handles handshake internally.
            
            // So we set sender first.
            if let Err(e) = engine.set_event_sender(tx_event_clone).await {
                let _ = tx_event.send(EngineEvent::Error(format!("Failed to set event sender: {}", e)));
                return;
            }

            while let Some(cmd) = rx_cmd.recv().await {
                match cmd {
                    EngineCommand::Init => {
                        if let Err(e) = engine.init().await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        } else {
                            // Ready is sent by engine task via channel if "readyok" is seen?
                            // Or init() waits for readyok?
                            // UciEngine::init waits for readyok.
                            // But UciEngine::set_event_sender spawns a task that consumes stdout.
                            // If we call set_event_sender BEFORE init, init's read_line will fail (reader moved).
                            // If we call set_event_sender AFTER init, init works fine.
                            // BUT, we want to capture "readyok" in event stream?
                            // UciEngine::init swallows "readyok".
                            // So we should send Ready event manually after init succeeds.
                            let _ = tx_event.send(EngineEvent::Ready);
                        }
                    }
                    EngineCommand::SetPosition(fen) => {
                        if let Err(e) = engine.set_position(&fen).await {
                            let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                    EngineCommand::Go(time) => {
                        if let Err(e) = engine.go(time).await {
                             let _ = tx_event.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                    EngineCommand::Stop => {
                        let _ = engine.stop().await;
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

    pub async fn send(&self, cmd: EngineCommand) -> Result<()> {
        self.tx.send(cmd).await.map_err(|_| anyhow::anyhow!("引擎已停止"))
    }
}
