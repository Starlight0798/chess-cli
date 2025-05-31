use crate::utils::*;

/// 引擎协议抽象
#[async_trait]
pub trait EngineProtocol: Send + Sync {
    /// 初始化引擎
    async fn init(&mut self) -> Result<()>;
    
    /// 设置棋局位置
    async fn set_position(&mut self, fen: &str) -> Result<()>;
    
    /// 开始思考
    async fn go(&mut self, think_time: Option<u64>) -> Result<String>;
    
    /// 停止思考
    async fn stop(&mut self) -> Result<()>;
    
    /// 退出引擎
    async fn quit(&mut self) -> Result<()>;
    
    /// 发送自定义命令
    async fn send_command(&mut self, command: &str) -> Result<()>;
}

/// UCI 协议引擎实现
pub struct UciEngine {
    process: Child,
    reader: BufReader<tokio::process::ChildStdout>,
}

impl UciEngine {
    /// 创建新的 UCI 引擎实例
    pub fn new(engine_path: &str, args: &[String]) -> Result<Self> {
        // 构建命令
        let mut cmd = Command::new(engine_path);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // 将 stderr 重定向到终端
            .kill_on_drop(true);

        // 启动进程
        let mut process = cmd
            .spawn()
            .with_context(|| format!("Failed to start engine: {}", engine_path))?;

        // 获取 stdout
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to capture engine stdout"))?;

        Ok(Self {
            process,
            reader: BufReader::new(stdout),
        })
    }

    /// 发送命令到引擎
    async fn send_command(&mut self, command: &str) -> Result<()> {
        let stdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to open engine stdin"))?;

        // 写入命令并添加换行符
        stdin
            .write_all(command.as_bytes())
            .await
            .context("Failed to write command to engine")?;
        stdin
            .write_all(b"\n")
            .await
            .context("Failed to write newline to engine")?;
        stdin
            .flush()
            .await
            .context("Failed to flush engine stdin")?;

        // 记录发送的命令（调试用）
        log::debug!("-> {}", command.trim());
        Ok(())
    }

    /// 读取引擎响应
    async fn read_response(&mut self) -> Result<String> {
        let mut response = String::new();
        self.reader
            .read_line(&mut response)
            .await
            .context("Failed to read from engine stdout")?;

        // 记录接收的响应（调试用）
        log::debug!("<- {}", response.trim());
        Ok(response)
    }
}

#[async_trait]
impl EngineProtocol for UciEngine {
    async fn init(&mut self) -> Result<()> {
        // 发送 uci 命令
        self.send_command("uci").await?;
        
        // 等待 uciok 响应
        let mut response = String::new();
        while !response.contains("uciok") {
            response = self.read_response().await?;
        }
        
        // 发送 isready 命令
        self.send_command("isready").await?;
        
        // 等待 readyok 响应
        let mut response = String::new();
        while !response.contains("readyok") {
            response = self.read_response().await?;
        }
        
        Ok(())
    }

    async fn set_position(&mut self, fen: &str) -> Result<()> {
        self.send_command(&format!("position fen {}", fen)).await
    }

    async fn go(&mut self, think_time: Option<u64>) -> Result<String> {
        // 构建 go 命令
        let command = match think_time {
            Some(time) => format!("go movetime {}", time),
            None => "go".to_string(),
        };
        
        self.send_command(&command).await?;
        
        // 读取响应直到找到 bestmove
        let mut best_move = None;
        while best_move.is_none() {
            let response = self.read_response().await?;
            
            if response.starts_with("bestmove") {
                let parts: Vec<&str> = response.split_whitespace().collect();
                if parts.len() > 1 {
                    best_move = Some(parts[1].to_string());
                }
            }
        }
        
        best_move.ok_or_else(|| anyhow!("Engine did not return bestmove"))
    }

    async fn stop(&mut self) -> Result<()> {
        self.send_command("stop").await
    }

    async fn quit(&mut self) -> Result<()> {
        self.send_command("quit").await?;
        
        // 等待引擎退出
        sleep(Duration::from_millis(100)).await;
        
        // 尝试终止进程（如果仍在运行）
        let _ = self.process.kill().await;
        
        Ok(())
    }

    async fn send_command(&mut self, command: &str) -> Result<()> {
        self.send_command(command).await
    }
}
