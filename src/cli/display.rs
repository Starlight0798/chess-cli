use crate::{
    game::{FenProcessor, GameManager, GameState, Piece, PieceKind, PlayerColor, Position},
    engine::{EngineProtocol, EngineThinkingInfo, EngineGoResult},
    utils::*,
};

/// 棋盘显示尺寸
pub const BOARD_WIDTH: u16 = 9 * 4 + 1;  // 9列 * 4字符 + 边框
pub const BOARD_HEIGHT: u16 = 10 * 2 + 1; // 10行 * 2行高 + 边框
pub const INPUT_AREA_Y: u16 = BOARD_HEIGHT + 3; // 输入区域起始位置
pub const INFO_PANEL_WIDTH: u16 = 100;           // 右侧信息面板宽度
pub const INFO_START_COL: u16 = BOARD_WIDTH + 4; // 信息面板起始列

/// 棋盘坐标标签
pub const COL_LABELS: [char; 9] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
pub const ROW_LABELS: [char; 10] = ['9', '8', '7', '6', '5', '4', '3', '2', '1', '0'];

/// 棋子符号
pub const RED_PIECES: [char; 7] = ['帅', '仕', '相', '马', '车', '炮', '兵'];
pub const BLACK_PIECES: [char; 7] = ['将', '士', '象', '马', '车', '炮', '卒'];

/// 颜色主题
#[derive(Clone, Copy)]
pub struct Theme {
    red_piece: Color,
    black_piece: Color,
    board_fg: Color,
    board_bg: Color,
    highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            red_piece: Color::Red,
            black_piece: Color::DarkYellow,
            board_fg: Color::White,
            board_bg: Color::Reset,
            highlight: Color::Yellow,
        }
    }
}

/// 渲染整个棋盘界面
pub fn render_view(game_manager: Option<&GameManager>) -> Result<()> {
    // 清屏
    execute!(stdout(), Clear(ClearType::All))?;
    
    // 清空右侧信息区域
    for y in 0..=BOARD_HEIGHT {
        execute!(
            stdout(),
            MoveTo(INFO_START_COL, y),
            Clear(ClearType::UntilNewLine),
        )?;
    }
    
    // 如果有游戏状态，绘制棋盘和状态信息
    if let Some(game) = game_manager {
        // 绘制棋盘
        render_board(&game.state)?;
        
        // 绘制状态信息
        draw_status_bar(&game.state)?;
        
        // 绘制思考信息
        if let Some(info) = game.think_info.as_ref() {
            draw_think_info(&info)?;
        }
    }
    
    // 绘制命令提示
    execute!(
        stdout(),
        MoveTo(0, INPUT_AREA_Y + 2),
        Print("命令: help 查看帮助 | 输入命令后按回车执行"),
    )?;
    
    execute!(stdout(), Show)?;
    stdout().flush()?;
    Ok(())
}

/// 渲染棋盘画面
pub fn render_board(state: &GameState) -> Result<()> {
    let theme: Theme = Theme::default();
    
    // 设置棋盘背景色
    execute!(
        stdout(),
        SetBackgroundColor(theme.board_bg),
        SetForegroundColor(theme.board_fg)
    )?;
    
    // 绘制棋盘网格
    for y in 0..=BOARD_HEIGHT {
        for x in 0..=BOARD_WIDTH {
            // 确定网格位置
            let is_corner: bool = (x == 0 || x == BOARD_WIDTH) && (y == 0 || y == BOARD_HEIGHT);
            let is_vertical: bool = x % 4 == 0;
            let is_horizontal: bool = y % 2 == 0;
            
            if is_vertical && is_horizontal {
                execute!(stdout(), MoveTo(x, y), Print('+'))?;
            } else if is_vertical {
                execute!(stdout(), MoveTo(x, y), Print('|'))?;
            } else if is_horizontal {
                execute!(stdout(), MoveTo(x, y), Print('-'))?;
            }
        }
    }
    
    // 绘制楚河汉界
    let river_y: u16 = BOARD_HEIGHT / 2;
    execute!(
        stdout(),
        MoveTo(2, river_y),
        SetForegroundColor(Color::DarkYellow),
        Print(" 楚 河        汉 界 "),
        ResetColor
    )?;
    
    // 绘制九宫格
    for (start_row, start_col) in [(0, 3), (7, 3)] {
        let x: u16 = (start_col * 4) as u16;
        let y: u16 = (start_row * 2) as u16;
        
        for i in 0..3 {
            execute!(
                stdout(),
                MoveTo(x, y + i * 2),
                Print('/'),
                MoveTo(x + 8, y + i * 2),
                Print('\\'),
            )?;
        }
    }
    
    // 绘制棋子
    for row in 0..10 {
        for col in 0..9 {
            // 根据翻转状态调整行坐标和列坐标
            let (screen_row, screen_col) = if state.flipped {
                (row, 8 - col)
            } else {
                (9 - row, col)
            };
            
            // 计算屏幕坐标
            let x: u16 = (screen_col * 4 + 2) as u16;
            let y: u16 = (screen_row * 2 + 1) as u16;
            
            if let Some(piece) = state.board[row][col] {
                // 获取棋子字符
                let piece_char: usize = match piece.kind {
                    PieceKind::General => 0,
                    PieceKind::Advisor => 1,
                    PieceKind::Elephant => 2,
                    PieceKind::Horse => 3,
                    PieceKind::Rook => 4,
                    PieceKind::Cannon => 5,
                    PieceKind::Pawn => 6,
                };
                
                let char: char = match piece.color {
                    PlayerColor::Red => RED_PIECES[piece_char],
                    PlayerColor::Black => BLACK_PIECES[piece_char],
                };
                
                // 设置棋子颜色
                let color: Color = match piece.color {
                    PlayerColor::Red => theme.red_piece,
                    PlayerColor::Black => theme.black_piece,
                };
                
                execute!(
                    stdout(),
                    MoveTo(x, y),
                    SetForegroundColor(color),
                    Print(char),
                )?;
            } else {
                // 空位置
                execute!(
                    stdout(),
                    MoveTo(x, y),
                    SetForegroundColor(theme.board_fg),
                    Print('·')
                )?;
            }
        }
    }
    
    // 绘制坐标标签
    // 列标签 (a-i) - 根据翻转状态调整
    let col_labels: Vec<char> = if state.flipped {
        COL_LABELS.iter().rev().copied().collect::<Vec<_>>()
    } else {
        COL_LABELS.to_vec()
    };
    
    for (i, label) in col_labels.iter().enumerate() {
        let x = (i * 4 + 2) as u16;
        execute!(
            stdout(),
            MoveTo(x, BOARD_HEIGHT + 1),
            SetForegroundColor(theme.board_fg),
            Print(label)
        )?;
    }
    
    // 行标签 (9-0) - 根据翻转状态调整
    let row_labels: Vec<char> = if state.flipped {
        ROW_LABELS.iter().rev().copied().collect::<Vec<_>>()
    } else {
        ROW_LABELS.to_vec()
    };
    
    for (i, label) in row_labels.iter().enumerate() {
        let y = (i * 2 + 1) as u16;
        execute!(
            stdout(),
            MoveTo(BOARD_WIDTH + 1, y),
            SetForegroundColor(theme.board_fg),
            Print(label)
        )?;
    }
    
    Ok(())
}

/// 绘制状态栏
fn draw_status_bar(state: &GameState) -> Result<()> {
    let theme: Theme = Theme::default();
    
    // 当前玩家
    let player_text: StyledContent<String> = match state.current_player {
        PlayerColor::Red => "红方回合".to_string().red(),
        PlayerColor::Black => "黑方回合".to_string().dark_yellow(),
    };
    
    // 历史记录
    let history_text: String = if state.history.is_empty() {
        "无历史记录".to_string()
    } else {
        let last_move: &String = state.history.last().unwrap();
        if last_move.len() > INFO_PANEL_WIDTH as usize - 10 {
            format!("最后一步: {}...", &last_move[..INFO_PANEL_WIDTH as usize - 10])
        } else {
            format!("最后一步: {}", last_move)
        }
    };
    
    // 绘制状态信息
    execute!(
        stdout(),
        MoveTo(INFO_START_COL, 0),
        SetForegroundColor(theme.board_fg),
        Print(player_text),
        MoveTo(INFO_START_COL, 1),
        Print(history_text),
        ResetColor
    )?;
    
    Ok(())
}

/// 绘制思考信息
fn draw_think_info(info: &EngineThinkingInfo) -> Result<()> {
    let mut lines: Vec<String> = Vec::new();
    
    // 第一行：基本指标
    let mut line1: String = format!("深度: {}", info.depth);
    if let Some(score) = info.score {
        line1.push_str(&format!(" | 分数: {}", score));
    }
    if let Some(nps) = info.nps {
        line1.push_str(&format!(" | NPS: {}k", (nps as f64 / 1024.0_f64).round() as usize));
    }
    if let Some(time) = info.time {
        if time >= 1000 {
            line1.push_str(&format!(" | 时间: {}s", time as f64 / 1000.0_f64));
        } else {
            line1.push_str(&format!(" | 时间: {}ms", time));
        }
    }
    lines.push(line1);
    
    // 第二行：主要变例
    if let Some(pv) = &info.pv {
        lines.push(format!("主变: {}", pv.join(" ")));
    }
    
    // 设置颜色
    let color = if let Some(score) = info.score {
        if score >= 0 { Color::Blue } else { Color::Red }
    } else {
        Color::Reset
    };
    
    // 显示思考信息
    for (i, line) in lines.iter().enumerate() {
        execute!(
            stdout(),
            MoveTo(INFO_START_COL, 4 + i as u16),
            SetForegroundColor(color),
            Print(line),
            ResetColor
        )?;
    }
    
    stdout().flush()?;
    Ok(())
}

/// 清理终端
pub fn cleanup_terminal() -> Result<()> {
    execute!(
        stdout(),
        Show,
        DisableMouseCapture,
        LeaveAlternateScreen,
        ResetColor
    )?;
    disable_raw_mode()?;
    Ok(())
}

/// 清空消息区域
pub fn clear_message_area() -> Result<()> {
    // 清除错误消息区域
    for i in 0..3 {
        execute!(
            stdout(),
            MoveTo(0, BOARD_HEIGHT + i),
            Clear(ClearType::CurrentLine)
        )?;
    }
    
    // 清除右侧信息面板中部区域
    for y in 3..BOARD_HEIGHT - 2 {
        execute!(
            stdout(),
            MoveTo(INFO_START_COL, y),
            Clear(ClearType::CurrentLine)
        )?;
    }
    
    stdout().flush()?;
    Ok(())
}

/// 显示欢迎信息
pub fn show_welcome() -> Result<()> {
    let version: &'static str = env!("CARGO_PKG_VERSION");
    execute!(
        stdout(),
        MoveTo(INFO_START_COL, 0),
        SetForegroundColor(Color::Cyan),
        Print(format!("中国象棋终端对弈系统v{}", version)),
        MoveTo(INFO_START_COL, 1),
        SetForegroundColor(Color::Yellow),
        Print("输入 'help' 查看命令帮助"),
        ResetColor
    )?;
    
    stdout().flush()?;
    reset_input_prompt()
}

/// 显示普通消息
pub fn show_message(msg: &str) -> Result<()> {
    display_info_panel(msg, 3, Color::Reset, None)
}

/// 显示错误消息
pub fn show_error(msg: &str) -> Result<()> {
    display_info_panel(&format!("错误: {}", msg), BOARD_HEIGHT - 2, Color::Red, None)
}

/// 显示帮助信息
pub fn show_help() -> Result<()> {
    const HELP_TEXT: &str = "可用命令:
    new <引擎> <red|black> [FEN] - 开始新游戏
    move <走法> - 走子(如'h2e2')
    reverse|flip - 翻转棋盘显示
    board - 重新显示棋盘
    history - 显示走子历史
    set <参数> <值> - 设置引擎参数
    listengines - 列出所有可用引擎
    help - 显示帮助
    quit - 退出程序";
    
    display_info_panel(HELP_TEXT, 3, Color::Reset, Some("命令帮助:"))
}

/// 显示引擎列表
pub fn show_engines(engines: &[String]) -> Result<()> {
    let content: String = engines.iter()
        .enumerate()
        .map(|(i, e)| format!("{}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n");
    
    display_info_panel(&content, 3, Color::Reset, Some("可用引擎:"))
}

/// 显示历史记录
pub fn show_history(history: &[String]) -> Result<()> {
    if history.is_empty() {
        return show_message("没有走子历史");
    }

    let content: String = history.iter()
        .enumerate()
        .take(10)
        .map(|(i, m)| format!("{}. {}", i + 1, m))
        .collect::<Vec<_>>()
        .join("\n");
    
    display_info_panel(&content, 3, Color::Reset, Some("走子历史:"))
}

/// 显示设置成功消息
pub fn show_set_success(name: &str, value: Option<&str>) -> Result<()> {
    let msg: String = match value {
        Some(v) => format!("设置成功: {} = {}", name, v),
        None => format!("设置成功: {}", name),
    };
    show_message(&msg)
}

/// 重置输入提示符
pub fn reset_input_prompt() -> Result<()> {
    execute!(
        stdout(),
        MoveTo(0, INPUT_AREA_Y),
        Clear(ClearType::CurrentLine),
        Print("> "),
        Show
    )?;
    stdout().flush()?;
    Ok(())
}

/// 通用信息显示函数
fn display_info_panel(
    content: &str, 
    start_y: u16,
    color: Color,
    title: Option<&str>
) -> Result<()> {
    let mut lines: Vec<String> = wrap_text(content, (INFO_PANEL_WIDTH - 2) as usize);

    // 添加标题
    if let Some(title_text) = title {
        lines.insert(0, title_text.to_string());
    }

    // 显示内容
    for (i, line) in lines.iter().enumerate() {
        execute!(
            stdout(),
            MoveTo(INFO_START_COL, start_y + i as u16),
            SetForegroundColor(color),
            Print(line),
            ResetColor
        )?;
    }

    stdout().flush()?;
    reset_input_prompt()
}

/// 文本换行处理
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    
    for paragraph in text.split('\n') {
        let mut current_line = String::new();
        
        for word in paragraph.split_whitespace() {
            let potential_length = if current_line.is_empty() {
                word.len()
            } else {
                current_line.len() + 1 + word.len()
            };
            
            if potential_length > width {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
                
                if word.len() > width {
                    let mut remaining = word;
                    while !remaining.is_empty() {
                        let split_point = width.min(remaining.len());
                        let (part, rest) = remaining.split_at(split_point);
                        lines.push(part.to_string());
                        remaining = rest;
                    }
                    continue;
                }
            }
            
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        if paragraph.is_empty() {
            lines.push(String::new());
        }
    }
    
    lines
}
