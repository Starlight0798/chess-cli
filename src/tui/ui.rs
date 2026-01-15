use crate::tui::app::{App, UiState, View};
use crate::game::{GameState, Piece, PieceKind, PlayerColor, Position};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.ui_state.view {
        View::Home => draw_home(f),
        View::Game => draw_game(f, app),
        View::Help => draw_help(f),
    }
}

fn draw_home(f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new("Chess CLI - 中国象棋")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let menu = Paragraph::new(
        "欢迎使用 Chess CLI\n\n\
        按 'n' 开始新游戏 (Pikafish, 执红)\n\
        按 'h' 查看帮助\n\
        按 'q' 退出",
    )
    .block(Block::default().borders(Borders::ALL).title("菜单"));
    f.render_widget(menu, chunks[1]);
}

fn draw_help(f: &mut Frame) {
    let info = Paragraph::new(
        "帮助信息:\n\n\
        方向键: 移动光标\n\
        空格/回车: 选择/移动棋子\n\
        q: 返回主菜单\n\n\
        关于引擎配置，请编辑 engines.toml",
    )
    .block(Block::default().borders(Borders::ALL).title("帮助"));
    f.render_widget(info, f.area());
}

fn draw_game(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(f.area());

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(25), Constraint::Length(3)].as_ref())
        .split(chunks[0]);

    // 绘制棋盘
    if let Some(state) = &app.game_state {
        let board_widget = BoardWidget {
            state: state,
            cursor: app.ui_state.cursor,
            selected: app.ui_state.selected,
        };
        f.render_widget(board_widget, left_chunks[0]);
    }

    // 绘制状态栏
    let status_text = if let Some(state) = &app.game_state {
        format!(
            "当前回合: {:?} | 状态: {}",
            state.current_player,
            if app.ui_state.selected.is_some() { "请选择目标位置" } else { "请选择棋子" }
        )
    } else {
        "无游戏".to_string()
    };
    
    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("状态"));
    f.render_widget(status, left_chunks[1]);

    // 绘制信息面板
    let messages: Vec<ListItem> = app
        .ui_state
        .messages
        .iter()
        .rev()
        .take(20)
        .map(|m| ListItem::new(Line::from(Span::raw(m))))
        .collect();
    
    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("消息"));
    f.render_widget(messages_list, chunks[1]);
}

struct BoardWidget<'a> {
    state: &'a GameState,
    cursor: Position,
    selected: Option<Position>,
}

impl<'a> Widget for BoardWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme_board = Style::default().fg(Color::White);
        let theme_red = Style::default().fg(Color::Red);
        let theme_black = Style::default().fg(Color::Cyan); 
        let theme_cursor = Style::default().bg(Color::Yellow).fg(Color::Black);
        let theme_selected = Style::default().bg(Color::Green).fg(Color::Black);

        let start_x = area.x + 2;
        let start_y = area.y + 1;

        for row in 0..10 {
            for col in 0..9 {
                let (logic_row, logic_col) = if self.state.flipped {
                    (row, 8 - col)
                } else {
                    (9 - row, col)
                };

                let screen_x = start_x + (col as u16 * 4);
                let screen_y = start_y + (row as u16 * 2);

                if screen_x + 4 > area.right() || screen_y + 2 > area.bottom() {
                    continue; 
                }

                buf.set_string(screen_x, screen_y, "+---", theme_board);
                if row < 9 {
                    buf.set_string(screen_x, screen_y + 1, "|", theme_board);
                }

                let pos = Position { row: logic_row, col: logic_col };
                let is_cursor = self.cursor == pos;
                let is_selected = self.selected == Some(pos);
                
                let mut style = if is_cursor {
                    theme_cursor
                } else if is_selected {
                    theme_selected
                } else {
                    Style::default()
                };

                if let Some(piece) = self.state.board[logic_row][logic_col] {
                    let symbol = get_piece_char(piece);
                    let piece_color = match piece.color {
                        PlayerColor::Red => theme_red,
                        PlayerColor::Black => theme_black,
                    };
                    
                    if !is_cursor && !is_selected {
                        style = piece_color;
                    }
                    
                    buf.set_string(screen_x, screen_y, format!(" {} ", symbol), style);
                } else {
                    if is_cursor {
                        buf.set_string(screen_x, screen_y, "[ ]", style);
                    } else if is_selected {
                         buf.set_string(screen_x, screen_y, "( )", style);
                    }
                }
            }
        }
        
        let river_y = start_y + 4 * 2 + 1;
        buf.set_string(start_x + 4, river_y, "楚 河        汉 界", Style::default().fg(Color::Yellow));
    }
}

fn get_piece_char(piece: Piece) -> char {
    match piece.kind {
        PieceKind::General => if piece.color == PlayerColor::Red { '帅' } else { '将' },
        PieceKind::Advisor => if piece.color == PlayerColor::Red { '仕' } else { '士' },
        PieceKind::Elephant => if piece.color == PlayerColor::Red { '相' } else { '象' },
        PieceKind::Horse => '马',
        PieceKind::Rook => '车',
        PieceKind::Cannon => '炮',
        PieceKind::Pawn => if piece.color == PlayerColor::Red { '兵' } else { '卒' },
    }
}
