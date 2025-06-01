use crate::{
    cli::interface::Command,
    game::state::PlayerColor,
    engine::EngineType,
    cli::display::*,
};
use crate::utils::*;

/// 监听用户输入
pub async fn listen_for_commands(tx: UnboundedSender<Command>) {
    let stdin: Stdin = stdin();
    let mut reader: Lines<BufReader<Stdin>> = BufReader::new(stdin).lines();
    let _ = reset_input_prompt();
    
    loop {
        execute!(
            stdout(),
            MoveTo(0, INPUT_AREA_Y),
            Clear(ClearType::CurrentLine),
            Print("> "),
            Show
        ).unwrap();
        stdout().flush().unwrap();

        match reader.next_line().await {
            Ok(Some(line)) => {
                let line: String = line.trim().to_string();
                if line.is_empty() {
                    let _ = reset_input_prompt();
                    continue;
                }

                execute!(
                    stdout(),
                    MoveTo(0, INPUT_AREA_Y + 1),
                    Clear(ClearType::CurrentLine),
                    Print(format!("输入: {}\n", line))
                ).unwrap();
                stdout().flush().unwrap();
                
                match parse_command(&line) {
                    Ok(cmd) => {
                        if tx.send(cmd).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Command::Error(e.to_string()));
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Command::Error(format!("输入错误: {}", e)));
            }
            _ => break,
        }
    }
}

/// 解析用户命令
fn parse_command(input: &str) -> Result<Command> {
    let mut parts: SplitWhitespace<'_> = input.split_whitespace();
    let cmd: &str = parts.next().ok_or_else(|| anyhow!("无效命令"))?;
    
    match cmd.to_lowercase().as_str() {
        "new" => {
            let engine_type: EngineType = EngineType::from_str(parts.next().ok_or_else(|| anyhow!("缺少引擎类型"))?)
                .map_err(|_| anyhow!("无效引擎类型"))?;
            let color: &str = parts.next().ok_or_else(|| anyhow!("缺少颜色参数"))?;
            
            let player_color: PlayerColor = match color.to_lowercase().as_str() {
                "红" | "red" => PlayerColor::Red,
                "黑" | "black" => PlayerColor::Black,
                _ => return Err(anyhow!("无效颜色，使用 '红' 或 '黑'")),
            };

            // 剩余的部分组合成FEN字符串
            let remaining_parts: Vec<&str> = parts.collect();
            let fen: Option<String> = if !remaining_parts.is_empty() {
                Some(remaining_parts.join(" "))
            } else {
                None
            };

            Ok(Command::NewGame { engine_type, player_color, fen })
        },
        "move" => {
            let move_str: String = parts.next().ok_or_else(|| anyhow!("缺少走法"))?.to_string();
            if move_str.len() != 4 {
                return Err(anyhow!("走法格式应为4字符，如 'h2e2'"));
            }
            Ok(Command::MakeMove(move_str))
        },
        "board" => Ok(Command::ShowBoard),
        "history" => Ok(Command::History),
        "set" => {
            let name: String = parts.next().ok_or_else(|| anyhow!("缺少选项名"))?.to_string();
            if let Some(value) = parts.next() {
                Ok(Command::SetOption { name, value: Some(value.to_string()) })
            } else {
                Ok(Command::SetOption { name, value: None })
            }
        }
        "listengines" => Ok(Command::ListEngines),
        "help" => Ok(Command::Help),
        "quit" | "exit" => Ok(Command::Quit),
        _ => Err(anyhow!("未知命令: {}", cmd)),
    }
}