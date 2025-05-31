use crate::game::state::{GameState, PlayerColor, Piece, PieceKind, Position};
use crate::utils::*;

/// 棋盘显示尺寸
pub const BOARD_WIDTH: u16 = 9 * 4 + 1;  // 9列 * 4字符 + 边框
pub const BOARD_HEIGHT: u16 = 10 * 2 + 1; // 10行 * 2行高 + 边框
pub const INPUT_AREA_Y: u16 = BOARD_HEIGHT + 3; // 输入区域起始位置

/// 棋盘坐标标签
pub const COL_LABELS: [char; 9] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
pub const ROW_LABELS: [char; 10] = ['9', '8', '7', '6', '5', '4', '3', '2', '1', '0'];

/// 棋子符号
pub const RED_PIECES: [char; 7] = ['将', '士', '相', '马', '车', '炮', '兵'];
pub const BLACK_PIECES: [char; 7] = ['帅', '仕', '象', '傌', '俥', '砲', '卒'];

/// 颜色主题
pub struct Theme {
    red_piece: Color,
    black_piece: Color,
    board_bg: Color,
    board_fg: Color,
    highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            red_piece: Color::Red,
            black_piece: Color::DarkYellow,  
            board_bg: Color::Reset,         
            board_fg: Color::White,          
            highlight: Color::Yellow,
        }
    }
}

/// 渲染整个棋盘界面
pub fn render_board(state: &GameState) -> Result<()> {
    // 仅清除棋盘区域
    for y in 0..=BOARD_HEIGHT {
        execute!(
            stdout(),
            MoveTo(0, y),
            Clear(ClearType::CurrentLine)
        )?;
    }
    
    // 清屏
    execute!(stdout(), Clear(ClearType::All))?;
    
    // 绘制棋盘边框
    draw_board_frame()?;
    
    // 绘制棋盘内容
    for row in 0..10 {
        for col in 0..9 {
            let position: Position = Position { row, col };
            draw_piece(state, position)?;
        }
    }
    
    // 绘制坐标标签
    draw_coordinate_labels()?;
    
    // 绘制状态信息
    draw_status_bar(state)?;
    
    // 绘制命令提示
    draw_command_prompt()?;
    
    execute!(stdout(), Show)?;
    stdout().flush()?;
    Ok(())
}

/// 绘制棋盘边框
fn draw_board_frame() -> Result<()> {
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
    draw_river_label()?;
    
    // 绘制九宫格
    draw_palace_lines()?;
    
    Ok(())
}

/// 绘制楚河汉界标签
fn draw_river_label() -> Result<()> {
    let river_y: u16 = BOARD_HEIGHT / 2;
    let river_text: &'static str = " 楚 河        汉 界 ";
    
    execute!(
        stdout(),
        MoveTo(2, river_y),
        SetForegroundColor(Color::DarkYellow),
        Print(river_text)
    )?;
    
    Ok(())
}

/// 绘制九宫格线
fn draw_palace_lines() -> Result<()> {
    // 红方九宫
    draw_palace(0, 3)?;
    
    // 黑方九宫
    draw_palace(7, 3)?;
    
    Ok(())
}

/// 绘制单个九宫格
fn draw_palace(start_row: usize, start_col: usize) -> Result<()> {
    let theme: Theme = Theme::default();
    
    // 左上角坐标
    let x: u16 = (start_col * 4) as u16;
    let y: u16 = (start_row * 2) as u16;
    
    // 绘制斜线
    for i in 0..3 {
        execute!(
            stdout(),
            MoveTo(x, y + i * 2),
            Print('/'),
            MoveTo(x + 8, y + i * 2),
            Print('\\'),
        )?;
    }
    
    Ok(())
}

/// 绘制棋子
fn draw_piece(state: &GameState, position: Position) -> Result<()> {
    let theme: Theme = Theme::default();
    // 反转行坐标：存储的行0（红方底部）在屏幕上显示在底部
    let screen_row: usize = 9 - position.row;
    let (x, y) = board_to_screen(Position {
        row: screen_row,
        col: position.col,
    });
    
    if let Some(piece) = state.board[position.row][position.col] {
        // 设置棋子颜色
        let color: Color = match piece.color {
            PlayerColor::Red => theme.red_piece,
            PlayerColor::Black => theme.black_piece,
        };
        
        execute!(
            stdout(),
            MoveTo(x, y),
            SetForegroundColor(color),
            Print(piece_char(piece)),
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

    Ok(())
}

/// 获取棋子字符
fn piece_char(piece: Piece) -> char {
    let index: usize = match piece.kind {
        PieceKind::General => 0,
        PieceKind::Advisor => 1,
        PieceKind::Elephant => 2,
        PieceKind::Horse => 3,
        PieceKind::Rook => 4,
        PieceKind::Cannon => 5,
        PieceKind::Pawn => 6,
    };
    
    match piece.color {
        PlayerColor::Red => RED_PIECES[index],
        PlayerColor::Black => BLACK_PIECES[index],
    }
}

/// 绘制坐标标签
fn draw_coordinate_labels() -> Result<()> {
    let theme: Theme = Theme::default();
    
    // 列标签 (a-i)
    for (i, label) in COL_LABELS.iter().enumerate() {
        let x: u16 = (i * 4 + 2) as u16;
        execute!(
            stdout(),
            MoveTo(x, BOARD_HEIGHT + 1),
            SetForegroundColor(theme.board_fg),
            Print(label)
        )?;
    }
    
    // 行标签 (9-0)
    for (i, label) in ROW_LABELS.iter().enumerate() {
        let y: u16 = (i * 2 + 1) as u16;
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
    let status_y: u16 = INPUT_AREA_Y;
    
    // 当前玩家
    let player_text: StyledContent<&str> = match state.current_player {
        PlayerColor::Red => "红方回合".red(),
        PlayerColor::Black => "黑方回合".dark_yellow(),  // 使用深黄色
    };
    
    // 历史记录
    let history_text: String = if state.history.is_empty() {
        "无历史记录".to_string()
    } else {
        format!("最后一步: {}", state.history.last().unwrap())
    };
    
    // 绘制状态信息
    execute!(
        stdout(),
        MoveTo(0, status_y),
        SetForegroundColor(theme.board_fg),
        Print(player_text),
        MoveTo(20, status_y),
        Print(history_text)
    )?;
    
    Ok(())
}

/// 绘制命令提示
fn draw_command_prompt() -> Result<()> {
    let prompt_y: u16 = INPUT_AREA_Y + 2;
    
    execute!(
        stdout(),
        MoveTo(0, prompt_y),
        Print("命令: new <引擎> <红|黑> | move <走法> | stop | board | quit\n")
    )?;
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

/// 棋盘坐标转换为屏幕坐标
fn board_to_screen(pos: Position) -> (u16, u16) {
    let x: u16 = (pos.col * 4 + 2) as u16;
    let y: u16 = (pos.row * 2 + 1) as u16;
    (x, y)
}

/// 显示最佳着法
pub fn show_best_move(best_move: &str) -> Result<()> {
    execute!(
        stdout(),
        MoveTo(0, INPUT_AREA_Y + 1),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(Color::Green),
        Print(format!("引擎推荐: {}\n", best_move)),
        ResetColor
    )?;
    stdout().flush()?;
    Ok(())
}

/// 显示错误消息
pub fn show_error(msg: &str) -> Result<()> {
    execute!(
        stdout(),
        MoveTo(0, INPUT_AREA_Y + 1),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(Color::Red),
        Print(format!("错误: {}\n", msg)),
        ResetColor
    )?;
    stdout().flush()?;
    Ok(())
}

/// 清空消息区域
pub fn clear_message_area() -> Result<()> {
    for i in 0..3 {
        execute!(
            stdout(),
            MoveTo(0, INPUT_AREA_Y + i),
            Clear(ClearType::CurrentLine)
        )?;
    }
    stdout().flush()?;
    Ok(())
}

/// 显示欢迎信息
pub fn show_welcome() -> Result<()> {
    execute!(
        stdout(),
        Clear(ClearType::All),
        MoveTo(0, 0),
        SetForegroundColor(Color::Cyan),
        Print("中国象棋终端对弈系统"),
        MoveTo(0, 1),
        SetForegroundColor(Color::Yellow),
        Print("使用命令控制游戏: new, move, stop, board, quit"),
        MoveTo(0, 2),
        Print("输入命令后按回车执行，棋盘下方会显示输入和结果"),
        ResetColor
    )?;
    stdout().flush()?;
    Ok(())
}