use crate::utils::*;
use async_trait::async_trait;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;

/// Go command parameters
#[derive(Debug, Clone, Default)]
pub struct GoParams {
    pub searchmoves: Option<Vec<String>>,
    pub ponder: bool,
    pub wtime: Option<usize>,
    pub btime: Option<usize>,
    pub winc: Option<usize>,
    pub binc: Option<usize>,
    pub movestogo: Option<usize>,
    pub depth: Option<usize>,
    pub nodes: Option<usize>,
    pub mate: Option<usize>,
    pub movetime: Option<usize>,
    pub infinite: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EngineOption {
    pub name: String,
    pub type_: String,
    pub default: Option<String>,
    pub min: Option<i32>,
    pub max: Option<i32>,
    pub vars: Option<Vec<String>>,
}

/// 引擎事件
#[derive(Debug, Clone)]
pub enum EngineEvent {
    Thinking(EngineThinkingInfo),
    BestMove {
        best_move: String,
        ponder_move: Option<String>,
    },
    Option(EngineOption),
    Ready,
    Error(String),
}

/// 引擎协议抽象
#[async_trait]
pub trait EngineProtocol: Send + Sync {
    /// 初始化引擎
    async fn init(&mut self) -> Result<()>;

    /// 设置棋局位置
    async fn set_position(&mut self, fen: &str, moves: Option<&[String]>) -> Result<()>;

    /// 开始思考
    async fn go(&mut self, params: GoParams) -> Result<()>;

    /// 停止思考
    async fn stop(&mut self) -> Result<()>;

    /// Ponder Hit (Opponent played the expected move)
    async fn ponderhit(&mut self) -> Result<()>;

    /// 准备就绪 (isready)
    async fn ready(&mut self) -> Result<()>;

    /// 设置引擎选项
    async fn set_option(&mut self, name: &str, value: Option<&str>) -> Result<()>;

    /// 通知引擎新游戏开始
    async fn new_game(&mut self) -> Result<()>;

    /// 退出引擎
    async fn quit(&mut self) -> Result<()>;

    /// 设置事件发送器
    async fn set_event_sender(&mut self, tx: UnboundedSender<EngineEvent>) -> Result<()>;
}

/// 引擎思考信息
#[derive(Debug, Clone, Default)]
pub struct EngineThinkingInfo {
    /// 搜索深度
    pub depth: usize,
    /// 局面评分
    pub score: Option<isize>,
    /// 每秒节点数 (Nodes Per Second)
    pub nps: Option<usize>,
    /// 搜索节点数
    pub nodes: Option<usize>,
    /// Hash 使用率 (千分比)
    pub hashfull: Option<usize>,
    /// MultiPV 索引
    #[allow(dead_code)]
    pub multipv: Option<usize>,
    /// 思考耗时 (毫秒)
    #[allow(dead_code)]
    pub time: Option<usize>,
    /// 主要变例 (Principal Variation)
    pub pv: Option<Vec<String>>,
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
        let mut nodes: Option<usize> = None;
        let mut hashfull: Option<usize> = None;
        let mut multipv: Option<usize> = None;
        let mut time: Option<usize> = None;
        let mut pv: Option<Vec<String>> = None;

        // 分割行并迭代
        let tokens: Vec<&str> = s.split_whitespace().collect();
        let mut i: usize = 1; // 跳过 "info"

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
                "nodes" if i + 1 < tokens.len() => {
                    nodes = Some(tokens[i + 1].parse().context("解析节点数失败")?);
                    i += 2;
                }
                "hashfull" if i + 1 < tokens.len() => {
                    hashfull = Some(tokens[i + 1].parse().context("解析Hash使用率失败")?);
                    i += 2;
                }
                "multipv" if i + 1 < tokens.len() => {
                    multipv = Some(tokens[i + 1].parse().context("解析MultiPV失败")?);
                    i += 2;
                }
                "time" if i + 1 < tokens.len() => {
                    time = Some(tokens[i + 1].parse().context("解析时间失败")?);
                    i += 2;
                }
                "pv" if i + 1 < tokens.len() => {
                    // pv 后面的前6个着法
                    pv = Some(
                        tokens[i + 1..]
                            .iter()
                            .take(6)
                            .map(|&s| s.to_string())
                            .collect(),
                    );
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
                nodes,
                hashfull,
                multipv,
                time,
                pv,
            })
            .ok_or_else(|| anyhow!("思考信息缺少深度"))
    }
}

/// 引擎思考结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EngineGoResult {
    /// 最佳着法
    pub best_move: String,
    /// 思考信息
    pub infos: Vec<EngineThinkingInfo>,
}

/// 支持的引擎
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EngineType {
    Pikafish,
    Other(String),
}

impl FromStr for EngineType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pikafish" => Ok(EngineType::Pikafish),
            _ => Ok(EngineType::Other(s.to_string())),
        }
    }
}

impl std::fmt::Display for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineType::Pikafish => write!(f, "pikafish"),
            EngineType::Other(s) => write!(f, "{}", s),
        }
    }
}

/// 基于 UCI 协议的引擎实现
pub struct UciEngine {
    process: Child,
    reader: Option<BufReader<ChildStdout>>,
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
            reader: Some(BufReader::new(stdout)),
        })
    }

    fn parse_best_move(line: &str) -> (String, Option<String>) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut best_move = String::new();
        let mut ponder_move = None;

        if parts.len() >= 2 {
            best_move = parts[1].to_string();
            if parts.len() >= 4 && parts[2] == "ponder" {
                ponder_move = Some(parts[3].to_string());
            }
        }

        (best_move, ponder_move)
    }

    fn parse_option(line: &str) -> Option<EngineOption> {
        // option name <name> type <type> [default <default>] [min <min> max <max>] [var <var>]
        if !line.starts_with("option name ") {
            return None;
        }

        let remaining = &line["option name ".len()..];
        let type_idx = remaining.find(" type ")?;
        
        let name = remaining[..type_idx].to_string();
        let remaining = &remaining[type_idx + " type ".len()..];
        
        let tokens: Vec<&str> = remaining.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }
        
        let type_ = tokens[0].to_string();
        let mut default = None;
        let mut min = None;
        let mut max = None;
        let mut vars = None;
        
        let mut i = 1;
        while i < tokens.len() {
            match tokens[i] {
                "default" if i + 1 < tokens.len() => {
                    // For string/combo, default might be multiple words? 
                    // Actually UCI says: "default <x>"
                    // If type is string, default can be empty or rest of line?
                    // Let's assume simple tokens for now, or handle specific types.
                    if type_ == "string" || type_ == "combo" {
                         // For combo/string, it might take the rest, but usually it's a single token for combo vars
                         // For string default value, it might be the rest of the line until another keyword?
                         // But 'min', 'max', 'var' are keywords.
                         // Let's just take the next token for now.
                         default = Some(tokens[i+1].to_string());
                         i += 2;
                    } else {
                         default = Some(tokens[i+1].to_string());
                         i += 2;
                    }
                }
                "min" if i + 1 < tokens.len() => {
                    min = tokens[i+1].parse().ok();
                    i += 2;
                }
                "max" if i + 1 < tokens.len() => {
                    max = tokens[i+1].parse().ok();
                    i += 2;
                }
                "var" if i + 1 < tokens.len() => {
                    let v = vars.get_or_insert(Vec::new());
                    v.push(tokens[i+1].to_string());
                    i += 2;
                }
                _ => i += 1,
            }
        }
        
        Some(EngineOption {
            name,
            type_,
            default,
            min,
            max,
            vars,
        })
    }

    /// 发送命令到引擎
    async fn send_command(&mut self, command: &str) -> Result<()> {
        let stdin: &mut ChildStdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("打开引擎标准输入失败"))?;

        // 拼接命令和换行符，一次性写入
        #[cfg(windows)]
        let line_ending = "\r\n";
        #[cfg(not(windows))]
        let line_ending = "\n";

        let full_command = format!("{}{}", command, line_ending);

        // 写入命令并添加换行符
        stdin
            .write_all(full_command.as_bytes())
            .await
            .context("写入命令到引擎失败")?;
        stdin.flush().await.context("刷新引擎标准输入失败")?;

        log_info!(command);

        Ok(())
    }

    /// 读取引擎响应 (同步等待一行)
    async fn read_line(&mut self) -> Result<String> {
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| anyhow!("Reader moved"))?;
        let mut response: String = String::new();
        reader
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

        // 如果 reader 存在，说明是同步模式，需要等待响应
        if self.reader.is_some() {
            // 等待 uciok 响应
            let mut response: String = String::new();
            while !response.contains("uciok") {
                response = self.read_line().await?;
            }
        }

        Ok(())
    }

    async fn ready(&mut self) -> Result<()> {
        // 发送 isready 命令
        self.send_command("isready").await?;

        // 如果 reader 存在，说明是同步模式，需要等待响应
        if self.reader.is_some() {
            // 等待 readyok 响应
            let mut response: String = String::new();
            while !response.contains("readyok") {
                response = self.read_line().await?;
            }
        }

        Ok(())
    }

    async fn set_position(&mut self, fen: &str, moves: Option<&[String]>) -> Result<()> {
        let mut command = format!("position fen {}", fen);
        if let Some(mvs) = moves {
            if !mvs.is_empty() {
                command.push_str(" moves");
                for mv in mvs {
                    command.push_str(" ");
                    command.push_str(mv);
                }
            }
        }
        self.send_command(&command).await
    }

    async fn go(&mut self, params: GoParams) -> Result<()> {
        let mut command = String::from("go");

        if params.ponder {
            command.push_str(" ponder");
        }
        if let Some(wtime) = params.wtime {
            command.push_str(&format!(" wtime {}", wtime));
        }
        if let Some(btime) = params.btime {
            command.push_str(&format!(" btime {}", btime));
        }
        if let Some(winc) = params.winc {
            command.push_str(&format!(" winc {}", winc));
        }
        if let Some(binc) = params.binc {
            command.push_str(&format!(" binc {}", binc));
        }
        if let Some(movestogo) = params.movestogo {
            command.push_str(&format!(" movestogo {}", movestogo));
        }
        if let Some(depth) = params.depth {
            command.push_str(&format!(" depth {}", depth));
        }
        if let Some(nodes) = params.nodes {
            command.push_str(&format!(" nodes {}", nodes));
        }
        if let Some(mate) = params.mate {
            command.push_str(&format!(" mate {}", mate));
        }
        if let Some(movetime) = params.movetime {
            command.push_str(&format!(" movetime {}", movetime));
        }
        if params.infinite {
            command.push_str(" infinite");
        }
        if let Some(searchmoves) = params.searchmoves {
            if !searchmoves.is_empty() {
                command.push_str(" searchmoves");
                for mv in searchmoves {
                    command.push_str(" ");
                    command.push_str(&mv);
                }
            }
        }

        self.send_command(&command).await
    }

    async fn stop(&mut self) -> Result<()> {
        self.send_command("stop").await
    }

    async fn ponderhit(&mut self) -> Result<()> {
        self.send_command("ponderhit").await
    }

    async fn set_option(&mut self, name: &str, value: Option<&str>) -> Result<()> {
        let command: String = match value {
            Some(v) => format!("setoption name {} value {}", name, v),
            None => format!("setoption name {}", name),
        };

        self.send_command(&command).await
    }

    async fn new_game(&mut self) -> Result<()> {
        self.send_command("ucinewgame").await?;
        self.ready().await
    }

    async fn quit(&mut self) -> Result<()> {
        self.send_command("quit").await?;

        // 等待引擎退出
        sleep(Duration::from_millis(100)).await;

        // 尝试终止进程
        self.process.kill().await?;

        Ok(())
    }

    async fn set_event_sender(&mut self, tx: UnboundedSender<EngineEvent>) -> Result<()> {
        let mut reader = self
            .reader
            .take()
            .ok_or_else(|| anyhow!("Reader already taken"))?;

        tokio::spawn(async move {
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                } // EOF

                let response = line.trim().to_string();
                log_info!(response);

                if response.starts_with("bestmove") {
                    let (best_move, ponder_move) = Self::parse_best_move(&response);
                    if !best_move.is_empty() {
                        let _ = tx.send(EngineEvent::BestMove {
                            best_move,
                            ponder_move,
                        });
                    }
                } else if response.starts_with("info") {
                    if let Ok(info) = EngineThinkingInfo::from_str(&response) {
                        let _ = tx.send(EngineEvent::Thinking(info));
                    }
                } else if response.starts_with("option name") {
                    if let Some(opt) = Self::parse_option(&response) {
                        let _ = tx.send(EngineEvent::Option(opt));
                    }
                } else if response.contains("readyok") {
                    let _ = tx.send(EngineEvent::Ready);
                }

                line.clear();
            }
        });

        Ok(())
    }
}
