use crate::game::{GameState, Piece, PieceKind, PlayerColor, Position};
use crate::tui::app::{App, View, StrategyType};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.ui_state.view {
        View::Home => draw_home(f, app),
        View::Game => draw_game(f, app),
        View::Settings => draw_settings(f, app),
        View::Help => draw_help(f),
    }
}

fn draw_home(f: &mut Frame, app: &App) {
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
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let continue_option = if app.game_state.is_some() {
        "按 'c' 继续游戏\n"
    } else {
        ""
    };

    let menu = Paragraph::new(
        format!("欢迎使用 Chess CLI\n\n\
        {}\
        按 'n' 开始新游戏\n\
        按 's' 进入设置 (选择红黑/引擎/难度)\n\
        按 'h' 查看帮助\n\
        按 'q' 退出", continue_option)
    )
    .block(Block::default().borders(Borders::ALL).title("菜单"));
    f.render_widget(menu, chunks[1]);
}

fn draw_settings(f: &mut Frame, app: &App) {
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

    let title = Paragraph::new("设置")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Menu Items
    let player_side_str = match app.ui_state.game_config.player_side {
        PlayerColor::Red => "红方 (Red)",
        PlayerColor::Black => "黑方 (Black)",
    };

    let strategy_desc = format!("{}", app.ui_state.game_config.strategy);
    let is_move_time = app.ui_state.game_config.strategy == StrategyType::MoveTime;
    let is_depth = app.ui_state.game_config.strategy == StrategyType::Depth;
    let is_game_time = app.ui_state.game_config.strategy == StrategyType::GameTime;

    let items = vec![
        format!("玩家执色: < {} >", player_side_str),
        format!("引擎选择: < {} >", app.ui_state.game_config.engine_name),
        format!("MultiPV (多路分析): < {} >", app.ui_state.game_config.multipv),
        format!("难度等级 (0-20): < {} >", app.ui_state.game_config.difficulty_level),
        format!("思考策略: < {} >", strategy_desc),
        format!("步时 (ms): < {} >{}", app.ui_state.game_config.move_time, if !is_move_time { " (未启用)" } else { "" }),
        format!("搜索深度: < {} >{}", app.ui_state.game_config.depth, if !is_depth { " (未启用)" } else { "" }),
        format!("局时 (分): < {} >{}", app.ui_state.game_config.game_time, if !is_game_time { " (未启用)" } else { "" }),
        format!("加秒 (秒): < {} >{}", app.ui_state.game_config.game_inc, if !is_game_time { " (未启用)" } else { "" }),
        "返回主菜单".to_string(),
    ];

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut style = Style::default();
            
            // Determine if item is enabled based on strategy
            let enabled = match i {
                5 => is_move_time,
                6 => is_depth,
                7 | 8 => is_game_time,
                _ => true,
            };

            if !enabled {
                style = style.fg(Color::DarkGray);
            }

            if i == app.ui_state.menu_index {
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }

            ListItem::new(format!("{} {}", if i == app.ui_state.menu_index { ">>" } else { "  " }, item))
                .style(style)
        })
        .collect();

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title("选项 (上下移动，左右/回车修改)"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    
    f.render_widget(list, chunks[1]);
}

fn draw_help(f: &mut Frame) {
    let info = Paragraph::new(
        "帮助信息:\n\n\
        方向键: 移动光标\n\
        空格/回车: 选择/移动棋子\n\
        u: 悔棋\n\
        r: 重做\n\
        s: 保存游戏\n\
        l: 加载游戏\n\
        q/Esc: 返回主菜单 (可继续游戏)\n\n\
        更多引擎配置，请编辑 engines.toml",
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

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    // 绘制棋盘
    if let Some(state) = &app.game_state {
        let board_widget = BoardWidget {
            state,
            cursor: app.ui_state.cursor,
            selected: app.ui_state.selected,
            last_move: state.get_last_move(),
            legal_moves: &app.ui_state.legal_moves,
        };
        f.render_widget(board_widget, left_chunks[0]);
    }

    // 绘制状态栏
    let status_text = if let Some(state) = &app.game_state {
        let check_alert = if state.is_check(state.current_player) {
            " [将军!]"
        } else {
            ""
        };
        let winner_alert = if let Some(winner) = state.check_winner() {
            format!(" [游戏结束! {:?} 获胜]", winner)
        } else {
            "".to_string()
        };

        format!(
            "当前回合: {:?} | 状态: {}{}{}",
            state.current_player,
            if app.ui_state.selected.is_some() {
                "请选择目标位置"
            } else {
                "请选择棋子"
            },
            check_alert,
            winner_alert
        )
    } else {
        "无游戏".to_string()
    };

    let status =
        Paragraph::new(status_text).block(Block::default().borders(Borders::ALL).title("状态"));
    f.render_widget(status, left_chunks[1]);

    // 绘制历史记录
    let history_items: Vec<ListItem> = if let Some(state) = &app.game_state {
        state
            .history
            .iter()
            .enumerate()
            .rev() // 反转顺序，最新走法显示在最上方
            .map(|(i, m)| ListItem::new(format!("{}. {}", i + 1, m)))
            .collect()
    } else {
        Vec::new()
    };

    let history_list = List::new(history_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("走法历史 (u: 悔棋, r: 重做)"),
    );
    f.render_widget(history_list, right_chunks[0]);

    // 绘制引擎信息
    let engine_text = if let Some(info) = &app.ui_state.engine_info {
        let score_str = if let Some(score) = info.score {
            format!("{}", score)
        } else {
            "-".to_string()
        };

        let pv_str = if let Some(pv) = &info.pv {
            if let Some(state) = &app.game_state {
                state.pv_to_chinese(pv).join(" ")
            } else {
                pv.join(" ")
            }
        } else {
            "-".to_string()
        };

        let nps_str = if let Some(nps) = info.nps {
            format!("{}k", nps / 1000)
        } else {
            "0".to_string()
        };

        format!(
            "深度: {}\n分数: {}\nNPS: {}\n节点: {}\nHash: {}‰\nPV: {}",
            info.depth,
            score_str,
            nps_str,
            info.nodes.unwrap_or(0),
            info.hashfull.unwrap_or(0),
            pv_str
        )
    } else {
        "等待引擎...".to_string()
    };

    let engine_info = Paragraph::new(engine_text)
        .block(Block::default().borders(Borders::ALL).title("引擎分析"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(engine_info, right_chunks[1]);

    // 绘制信息面板
    let messages: Vec<ListItem> = app
        .ui_state
        .messages
        .iter()
        .rev()
        .take(20)
        .map(|m| ListItem::new(Line::from(Span::raw(m))))
        .collect();

    let messages_list =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("系统消息"));
    f.render_widget(messages_list, right_chunks[2]);
}

struct BoardWidget<'a> {
    state: &'a GameState,
    cursor: Position,
    selected: Option<Position>,
    last_move: Option<(Position, Position)>,
    legal_moves: &'a [Position],
}

impl<'a> Widget for BoardWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme_board = Style::default().fg(Color::DarkGray);
        let theme_red = Style::default().fg(Color::Red);
        let theme_black = Style::default().fg(Color::Cyan);
        let theme_cursor = Style::default().bg(Color::Yellow).fg(Color::Black);
        let theme_selected = Style::default().bg(Color::Green).fg(Color::Black);
        let theme_last_move = Style::default().bg(Color::Blue).fg(Color::White);
        let theme_legal_hint = Style::default().fg(Color::Green);
        let theme_capture_hint = Style::default().bg(Color::LightRed);

        let start_x = area.x + 2;
        let start_y = area.y + 1;

        // 绘制棋盘网格
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

                // 绘制网格点字符
                let grid_char = get_grid_char(logic_row, logic_col);
                buf.set_string(screen_x, screen_y, grid_char, theme_board);

                // 绘制横线
                if col < 8 {
                    buf.set_string(screen_x + 1, screen_y, "───", theme_board);
                }

                // 绘制竖线 (跳过楚河汉界)
                if row < 9 {
                    // 逻辑行 4 和 5 之间是河界
                    // 如果不翻转：屏幕行 row对应逻辑行 9-row。
                    // 屏幕行 4 (逻辑行5) 下方是河界。
                    // 屏幕行 5 (逻辑行4) 上方是河界。
                    // 竖线是向下画的，所以在逻辑行 5 (屏幕行4) 的时候不画竖线？
                    // 让我们看 logic_row。如果 logic_row 是 5 (上方河岸)，它下面的竖线不画？
                    // 竖线连接 logic_row 和 logic_row - 1。
                    // 所以 logic_row = 5 时，不画连接到 4 的竖线。

                    let _should_draw_vertical = logic_row != 0 && logic_row != 5;
                    // 等等，循环是 0..9，row 是屏幕行。
                    // 竖线画在 screen_y + 1。连接的是当前行和下一行。
                    // 当前行对应 logic_row。下一行对应 logic_row - 1 (如果不翻转)。
                    // 楚河汉界在 logic_row 4 和 5 之间。
                    // 所以如果当前是 logic_row 5，连接 4，不画。
                    // 如果翻转，当前是 logic_row，下一行是 logic_row + 1。
                    // 楚河汉界在 4 和 5 之间。
                    // 如果当前是 logic_row 4，连接 5，不画。

                    let is_river_crossing = if self.state.flipped {
                        logic_row == 4
                    } else {
                        logic_row == 5
                    };

                    if !is_river_crossing {
                        buf.set_string(screen_x, screen_y + 1, "│", theme_board);
                    }
                }

                let pos = Position {
                    row: logic_row,
                    col: logic_col,
                };
                let is_cursor = self.cursor == pos;
                let is_selected = self.selected == Some(pos);
                let is_last_move_from = self.last_move.is_some_and(|(f, _)| f == pos);
                let is_last_move_to = self.last_move.is_some_and(|(_, t)| t == pos);
                let is_legal_move = self.legal_moves.contains(&pos);

                // 优先级：Cursor > Selected > LastMove > Piece/Empty
                let mut style = Style::default();
                let mut symbol = String::new();
                let mut has_piece = false;

                if let Some(piece) = self.state.board[logic_row][logic_col] {
                    symbol = format!(" {} ", get_piece_char(piece));
                    style = match piece.color {
                        PlayerColor::Red => theme_red,
                        PlayerColor::Black => theme_black,
                    };
                    has_piece = true;
                }

                // 背景色处理
                if is_cursor {
                    style = style.patch(theme_cursor);
                    if !has_piece {
                        if is_legal_move {
                            symbol = " [·] ".to_string();
                        } else {
                            symbol = " [ ] ".to_string();
                        }
                    }
                } else if is_selected {
                    style = style.patch(theme_selected);
                    if !has_piece {
                        symbol = " ( ) ".to_string();
                    }
                } else if is_last_move_from || is_last_move_to {
                    style = style.patch(theme_last_move);
                } else if has_piece && is_legal_move {
                    style = style.patch(theme_capture_hint);
                }

                // 如果没有棋子，显示网格或者提示点
                if !has_piece {
                    if is_legal_move && !is_cursor {
                        symbol = " · ".to_string();
                        style = theme_legal_hint;
                        // 如果是 last move 且是空的（from），还是会被覆盖成 blue bg
                        if is_last_move_from || is_last_move_to {
                            style = style.patch(theme_last_move);
                        }
                    } else if !is_cursor && !is_selected {
                        // 恢复网格字符显示 (其实上面已经画了，这里不需要覆盖，除非要清除)
                        // 但是因为 set_string 会覆盖，所以如果这里不写，就会保留上面的网格。
                        // 问题是：棋子是 " X " (3 chars)，网格是 "┼" (1 char)。
                        // 我们需要在中心位置画。
                        // 我们的 grid 画在 screen_x。piece 也是画在 screen_x。
                        // piece 覆盖 grid。
                        // 如果没有 piece，grid 已经在那里了。
                        // 但是 cursor/selected 需要显示框框。
                        continue;
                    }
                }

                if !symbol.is_empty() {
                    buf.set_string(screen_x, screen_y, symbol, style);
                }
            }
        }

        let river_y = start_y + 4 * 2 + 1;
        buf.set_string(
            start_x + 4,
            river_y,
            "楚 河        汉 界",
            Style::default().fg(Color::Yellow),
        );
    }
}

fn get_grid_char(row: usize, col: usize) -> &'static str {
    match (row, col) {
        (9, 0) => "┌",
        (9, 8) => "┐",
        (9, _) => "┬",
        (0, 0) => "└",
        (0, 8) => "┘",
        (0, _) => "┴",
        (4, 0) => "├",
        (4, 8) => "┤",
        (4, _) => "┬", // Lower river bank (top of bottom half)
        (5, 0) => "├",
        (5, 8) => "┤",
        (5, _) => "┴", // Upper river bank (bottom of top half)
        (_, 0) => "├",
        (_, 8) => "┤",
        (_, _) => "┼",
    }
}

fn get_piece_char(piece: Piece) -> char {
    match piece.kind {
        PieceKind::General => {
            if piece.color == PlayerColor::Red {
                '帅'
            } else {
                '将'
            }
        }
        PieceKind::Advisor => {
            if piece.color == PlayerColor::Red {
                '仕'
            } else {
                '士'
            }
        }
        PieceKind::Elephant => {
            if piece.color == PlayerColor::Red {
                '相'
            } else {
                '象'
            }
        }
        PieceKind::Horse => '马',
        PieceKind::Rook => '车',
        PieceKind::Cannon => '炮',
        PieceKind::Pawn => {
            if piece.color == PlayerColor::Red {
                '兵'
            } else {
                '卒'
            }
        }
    }
}
