use crate::utils::*;

/// 初始化终端界面
pub fn init_terminal() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        Hide,
        Clear(ClearType::All)
    )?;
    Ok(())
}

/// 清理终端界面
pub fn cleanup_terminal() -> Result<()> {
    let mut stdout = stdout();
    execute!(
        stdout,
        Show,
        DisableMouseCapture,
        LeaveAlternateScreen,
        ResetColor
    )?;
    disable_raw_mode()?;
    Ok(())
}

/// 渲染棋盘到终端
pub fn render_board(fen: &str) -> Result<()> {
    // 简化的棋盘渲染，实际实现需要解析FEN字符串
    let mut stdout = stdout();
    
    // 清屏并移动到顶部
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    
    // 打印顶部坐标
    execute!(stdout, Print("  a b c d e f g h i\n"))?;
    
    // 渲染棋盘主体
    for row in 0..10 {
        // 打印行号
        execute!(stdout, Print(format!("{} ", 9 - row)))?;
        
        for col in 0..9 {
            // 设置背景色（棋盘格）
            let bg_color = if (row + col) % 2 == 0 {
                Color::Rgb { r: 238, g: 215, b: 170 } // 浅色
            } else {
                Color::Rgb { r: 184, g: 139, b: 74 }  // 深色
            };
            
            // 设置前景色（棋子颜色）
            let fg_color = if row < 5 { 
                Color::Red  // 红方
            } else { 
                Color::Blue // 黑方
            };
            
            // 渲染棋子（简化版，实际需要根据FEN解析）
            let piece_char = if row == 0 || row == 9 {
                match col {
                    0 | 8 => "车",
                    1 | 7 => "马",
                    2 | 6 => "相",
                    3 | 5 => "士",
                    4 => if row == 0 { "帅" } else { "将" },
                    _ => " ",
                }
            } else if row == 2 || row == 7 {
                if col == 1 || col == 7 { "炮" } else { " " }
            } else if row == 3 || row == 6 {
                if col % 2 == 0 { "兵" } else { " " }
            } else {
                " "
            };
            
            // 绘制棋盘格和棋子
            execute!(
                stdout,
                SetBackgroundColor(bg_color),
                SetForegroundColor(fg_color),
                Print(format!("{}", piece_char)),
                ResetColor,
                Print(" ")
            )?;
        }
        
        // 行号
        execute!(stdout, Print(format!(" {}\n", 9 - row)))?;
    }
    
    // 打印底部坐标
    execute!(stdout, Print("  a b c d e f g h i\n"))?;
    
    // 显示当前回合
    execute!(stdout, Print("\n当前回合: "))?;
    if fen.contains(" w ") {
        execute!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("红方"),
            ResetColor
        )?;
    } else {
        execute!(
            stdout,
            SetForegroundColor(Color::Blue),
            Print("黑方"),
            ResetColor
        )?;
    }
    
    stdout.flush()?;
    Ok(())
}

/// 打印欢迎信息
pub fn print_welcome() {
    println!("欢迎使用 Chess CLI - 中国象棋终端对弈系统");
    println!("输入 'new <引擎名> <red|black>' 开始新游戏");
    println!("输入 'move <着法>' 走棋 (例如: move h2e2)");
    println!("输入 'stop' 停止引擎思考");
    println!("输入 'board' 显示当前棋盘");
    println!("输入 'quit' 退出\n");
}

/// 清屏
pub fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

/// 打印错误信息
pub fn print_error(msg: &str) {
    println!("\x1B[1;31m错误: {}\x1B[0m", msg);
}

/// 打印引擎推荐着法
pub fn print_best_move(best_move: &str) {
    println!("\n引擎推荐着法: \x1B[1;32m{}\x1B[0m", best_move);
}
