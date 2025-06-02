use crate::utils::*;

/// 引擎协议抽象
#[async_trait]
pub trait EngineProtocol: Send + Sync {
    /// 初始化引擎
    async fn init(&mut self) -> Result<()>;
    
    /// 设置棋局位置
    async fn set_position(&mut self, fen: &str) -> Result<()>;
    
    /// 开始思考
    async fn go(&mut self, think_time: Option<usize>) -> Result<String>;
    
    /// 停止思考
    async fn stop(&mut self) -> Result<()>;

    /// 设置引擎选项
    async fn set_option(&mut self, name: &str, value: Option<&str>) -> Result<()>;
    
    /// 退出引擎
    async fn quit(&mut self) -> Result<()>;

    /// 获取最后的思考信息
    fn get_last_think_info(&self) -> Option<EngineThinkingInfo>;
}

/// 引擎思考信息
#[derive(Debug, Clone)]
pub struct EngineThinkingInfo {
    pub depth: usize,
    pub score: Option<isize>,
    pub nps: Option<usize>,
    pub time: Option<usize>,
    pub pv: Option<String>,
}

impl Default for EngineThinkingInfo {
    fn default() -> Self {
        Self {
            depth: 0,
            score: None,
            nps: None,
            time: None,
            pv: None,
        }
    }
}

impl FromStr for EngineThinkingInfo {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        if !s.starts_with("info") {
            return Err(anyhow!("无效的思考信息行: {}", s));
        }
        
        let mut depth: Option<usize> = None;
        let mut score: Option<isize> = None;
        let mut nps: Option<usize> = None;
        let mut time: Option<usize> = None;
        let mut pv: Option<String> = None;
        
        // 分割行并迭代
        let tokens: Vec<&str> = s.split_whitespace().collect();
        let mut i = 1; // 跳过 "info"
        
        while i < tokens.len() {
            match tokens[i] {
                "depth" if i + 1 < tokens.len() => {
                    depth = Some(tokens[i + 1].parse().context("解析深度失败")?);
                    i += 2;
                }
                "score" if i + 2 < tokens.len() && tokens[i + 1] == "cp" => {
                    score = Some(tokens[i + 2].parse().context("解析得分失败")?);
                    i += 3;
                }
                "nps" if i + 1 < tokens.len() => {
                    nps = Some(tokens[i + 1].parse().context("解析节点每秒失败")?);
                    i += 2;
                }
                "time" if i + 1 < tokens.len() => {
                    time = Some(tokens[i + 1].parse().context("解析时间失败")?);
                    i += 2;
                }
                "pv" if i + 1 < tokens.len() => {
                    // pv 后面的前四个着法
                    let pv_moves: Vec<&str> = tokens[i + 1..].iter().take(4).copied().collect();
                    pv = Some(pv_moves.join(" "));
                    break;
                }
                _ => {
                    i += 1;
                }
            }
        }
        
        // depth 是必须的
        depth
            .map(|d| Self {
                depth: d,
                score,
                nps,
                time,
                pv,
            })
            .ok_or_else(|| anyhow!("思考信息缺少深度"))
    }
}

/// 支持的引擎
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineType {
    Pikafish,
    // TODO: 支持其他引擎
}

impl FromStr for EngineType {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pikafish" => Ok(EngineType::Pikafish),
            _ => Err(anyhow!("不支持的引擎类型: {}", s)),
        }
    }
}

impl ToString for EngineType {
    fn to_string(&self) -> String {
        match self {
            EngineType::Pikafish => "pikafish".to_string(),
        }
    }
}

/// UCI 协议引擎实现
pub struct UciEngine {
    process: Child,
    reader: BufReader<ChildStdout>,
    last_think_info: Option<EngineThinkingInfo>,
}

impl UciEngine {
    /// 创建新的 UCI 引擎实例
    pub fn new(engine_path: &str) -> Result<Self> {
        // 构建命令
        let mut cmd: Command = Command::new(engine_path);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) 
            .kill_on_drop(true);

        // 启动进程
        let mut process: Child = cmd
            .spawn()
            .with_context(|| format!("启动引擎失败: {}", engine_path))?;

        // 获取 stdout
        let stdout: ChildStdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("获取引擎标准输出失败"))?;

        Ok(Self {
            process,
            reader: BufReader::new(stdout),
            last_think_info: None,
        })
    }

    /// 发送命令到引擎
    async fn send_command(&mut self, command: &str) -> Result<()> {
        let stdin: &mut ChildStdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("打开引擎标准输入失败"))?;

        // 写入命令并添加换行符
        stdin
            .write_all(command.as_bytes())
            .await
            .context("写入命令到引擎失败")?;
        stdin
            .write_all(b"\n")
            .await
            .context("写入换行符到引擎失败")?;
        stdin
            .flush()
            .await
            .context("刷新引擎标准输入失败")?;

        log_info!(command);

        Ok(())
    }

    /// 读取引擎响应
    async fn read_response(&mut self) -> Result<String> {
        let mut response: String = String::new();
        self.reader
            .read_line(&mut response)
            .await
            .context("读取引擎输出失败")?;

        log_info!(response);

        Ok(response)
    }
}

#[async_trait]
impl EngineProtocol for UciEngine {
    async fn init(&mut self) -> Result<()> {
        // 发送 uci 命令
        self.send_command("uci").await?;
        
        // 等待 uciok 响应
        let mut response: String = String::new();
        while !response.contains("uciok") {
            response = self.read_response().await?;
        }
        
        // 发送 isready 命令
        self.send_command("isready").await?;
        
        // 等待 readyok 响应
        let mut response: String = String::new();
        while !response.contains("readyok") {
            response = self.read_response().await?;
        }
        
        Ok(())
    }

    async fn set_position(&mut self, fen: &str) -> Result<()> {
        self.send_command(&format!("position fen {}", fen)).await
    }

    async fn go(&mut self, think_time: Option<usize>) -> Result<String> {
        // 构建 go 命令
        let command: String = match think_time {
            Some(time) => format!("go movetime {}", time),
            None => "go".to_string(),
        };
        
        self.send_command(&command).await?;
        
        // 读取响应直到找到 bestmove
        let mut best_move: Option<String> = None;
        while best_move.is_none() {
            let response: String = self.read_response().await?;
            
            if response.starts_with("bestmove") {
                let parts: Vec<&str> = response.split_whitespace().collect();
                if parts.len() > 1 {
                    best_move = Some(parts[1].to_string());
                }
            }
            // 解析并记录思考信息
            else if response.starts_with("info") {
                match EngineThinkingInfo::from_str(&response) {
                    Ok(info) => {
                        log_info!(info);
                        self.last_think_info = Some(info);
                    },
                    Err(e) => {
                        log_error!(format!("解析思考信息失败: {}", e))
                    },
                }
            }
        }
        
        best_move.ok_or_else(|| anyhow!("引擎未返回最佳着法"))
    }

    async fn stop(&mut self) -> Result<()> {
        self.send_command("stop").await
    }

    async fn set_option(&mut self, name: &str, value: Option<&str>) -> Result<()> {
        let command: String = match value {
            Some(v) => format!("setoption name {} value {}", name, v),
            None => format!("setoption name {}", name),
        };
        
        self.send_command(&command).await
    }

    async fn quit(&mut self) -> Result<()> {
        self.send_command("quit").await?;
        
        // 等待引擎退出
        sleep(Duration::from_millis(100)).await;
        
        // 尝试终止进程
        self.process.kill().await?;
        
        Ok(())
    }

    fn get_last_think_info(&self) -> Option<EngineThinkingInfo> {
        self.last_think_info.clone()
    }
}
