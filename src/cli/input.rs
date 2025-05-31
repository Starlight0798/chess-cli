use crate::{
    cli::interface::{Command, PlayerColor},
    utils::Result,
};
use crate::utils::*;

/// 异步监听用户输入并发送命令
pub async fn listen_for_commands(tx: UnboundedSender<Command>) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let cmd = parse_command(&line)?;
        tx.send(cmd)?;
    }

    Ok(())
}

/// 解析用户输入的命令
fn parse_command(input: &str) -> Result<Command> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow!("无效命令"));
    }

    match parts[0] {
        "new" => {
            if parts.len() < 3 {
                return Err(anyhow!("用法: new <引擎名> <red|black>"));
            }
            let color = match parts[2] {
                "red" => PlayerColor::Red,
                "black" => PlayerColor::Black,
                _ => return Err(anyhow!("无效颜色，使用 red 或 black")),
            };
            Ok(Command::NewGame {
                engine_name: parts[1].to_string(),
                player_color: color,
            })
        }
        "move" => {
            if parts.len() < 2 {
                return Err(anyhow!("用法: move <着法>"));
            }
            Ok(Command::MakeMove(parts[1].to_string()))
        }
        "stop" => Ok(Command::StopEngine),
        "board" => Ok(Command::ShowBoard),
        "quit" => Ok(Command::Quit),
        _ => Err(anyhow!("未知命令")),
    }
}
