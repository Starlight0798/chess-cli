use crate::utils::*;
use crate::game::FenProcessor;

/// 玩家颜色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerColor {
    Red,
    Black,
}

impl PlayerColor {
    /// 获取对手颜色
    pub fn opponent(&self) -> Self {
        match self {
            PlayerColor::Red => PlayerColor::Black,
            PlayerColor::Black => PlayerColor::Red,
        }
    }
}

/// 棋子种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    General,   // 将/帅
    Advisor,   // 士/仕
    Elephant,  // 象/相
    Horse,     // 马
    Rook,      // 车
    Cannon,    // 炮
    Pawn,      // 兵/卒
}

/// 棋子
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub color: PlayerColor,
    pub kind: PieceKind,
}

/// 坐标
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

/// 游戏状态
#[derive(Clone)]
pub struct GameState {
    /// 棋盘，10行9列，行0-9，列0-8
    pub board: [[Option<Piece>; 9]; 10],
    /// 当前轮到哪个玩家
    pub current_player: PlayerColor,
    /// 走子历史
    pub history: Vec<String>,
    /// 棋盘是否翻转显示
    pub flipped: bool,
}

impl GameState {
    /// 创建初始游戏状态
    pub fn new() -> Self {
        FenProcessor::parse_fen("rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w").unwrap()
    }

    /// 重置为初始状态
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// 应用一个走法
    /// 走法字符串格式：起始位置+目标位置，例如 "h2e2"
    /// 起始位置：列从a到i，行从0到9（0在底部，9在顶部）
    pub fn apply_move(&mut self, move_str: &str) -> Result<()> {
        // 将走法字符串转换为坐标
        let (from, to) = Self::parse_move(move_str)?;
        
        // 合法性检查
        self.is_valid_move(from, to)?;

        // 记录走法
        let chinese_move: String = self.move_to_chinese(&move_str)?;
        log_info!(self.current_player, move_str, chinese_move, from, to);
        self.history.push(chinese_move);
        
        // 执行移动：将棋子移动到目标位置，起始位置置空
        self.board[to.row][to.col] = self.board[from.row][from.col];
        self.board[from.row][from.col] = None;
        
        // 切换玩家
        self.current_player = self.current_player.opponent();
        
        Ok(())
    }
    
    /// 将走法字符串解析为两个坐标：((from_x, from_y), (to_x, to_y))
    /// 坐标系统：x是列（0-8对应a-i），y是行（0-9，0是底部，9是顶部）
    /// 例如："h2e2" -> ((7,2), (4,2))
    fn parse_move(move_str: &str) -> Result<(Position, Position)> {
        if move_str.len() != 4 {
            return Err(anyhow!("走法格式错误，应为4个字符"));
        }
        
        let chars: Vec<char> = move_str.chars().collect();
        let from_x: usize = match chars[0] {
            'a'..='i' => chars[0] as usize - 'a' as usize,
            _ => return Err(anyhow!("起始列无效")),
        };
        let from_y: usize = match chars[1] {
            '0'..='9' => chars[1] as usize - '0' as usize,
            _ => return Err(anyhow!("起始行无效")),
        };
        let to_x: usize = match chars[2] {
            'a'..='i' => chars[2] as usize - 'a' as usize,
            _ => return Err(anyhow!("目标列无效")),
        };
        let to_y: usize = match chars[3] {
            '0'..='9' => chars[3] as usize - '0' as usize,
            _ => return Err(anyhow!("目标行无效")),
        };
        
        // 检查坐标是否在棋盘内
        if from_x > 8 || to_x > 8 {
            return Err(anyhow!("列超出范围"));
        }
        if from_y > 9 || to_y > 9 {
            return Err(anyhow!("行超出范围"));
        }
        
        Ok((Position { col: from_x, row: from_y }, Position { col: to_x, row: to_y }))
    }

    /// 走法合法性验证
    pub fn is_valid_move(&self, from: Position, to: Position) -> Result<()> {
        // 检查起始位置是否有棋子
        let piece: Piece = self.board[from.row][from.col]
            .ok_or_else(|| anyhow!("起始位置没有棋子"))?;
        
        // 检查棋子颜色是否与当前玩家一致   
        if piece.color != self.current_player {
            return Err(anyhow!("不能移动对方的棋子"));
        }

        // 检查目标位置是否有己方棋子
        if let Some(target_piece) = self.board[to.row][to.col] {
            if target_piece.color == self.current_player {
                return Err(anyhow!("目标位置已有己方棋子"));
            }
        }

        // 根据棋子种类检查
        match piece.kind {
            // 将/帅
            PieceKind::General => {
                // 将帅只能在九宫内移动
                match self.current_player {
                    PlayerColor::Red => {
                        if to.row > 2 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("帅只能在九宫内移动"));
                        }
                    },
                    PlayerColor::Black => {
                        if to.row < 7 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("将只能在九宫内移动"));
                        }
                    },
                }
                // 将帅只能横向或纵向移动一步
                if (from.row != to.row && from.col != to.col)
                    || (from.row == to.row && (from.col as isize - to.col as isize).abs() > 1)
                    || (from.col == to.col && (from.row as isize - to.row as isize).abs() > 1)
                {
                    return Err(anyhow!("将帅只能横向或纵向移动一步"));
                }
            },
            // 士/仕
            PieceKind::Advisor => {
                // 士/仕只能在九宫内移动
                match self.current_player {
                    PlayerColor::Red => {
                        if to.row > 2 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("仕只能在九宫内移动"));
                        }
                    },
                    PlayerColor::Black => {
                        if to.row < 7 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("士只能在九宫内移动"));
                        }
                    },
                }
                // 士/仕只能斜向移动一步
                if (from.row as isize - to.row as isize).abs() != 1
                    || (from.col as isize - to.col as isize).abs() != 1
                {
                    return Err(anyhow!("士/仕只能斜向移动一步"));
                }
            },
            // 象/相
            PieceKind::Elephant => {
                // 象/相不能过河
                match self.current_player {
                    PlayerColor::Red => {
                        if to.row > 4 {
                            return Err(anyhow!("相不能过河"));
                        }
                    },
                    PlayerColor::Black => {
                        if to.row < 5 {
                            return Err(anyhow!("象不能过河"));
                        }
                    },
                }
                // 象/相只能斜向移动两步
                if (from.row as isize - to.row as isize).abs() != 2
                    || (from.col as isize - to.col as isize).abs() != 2
                {
                    return Err(anyhow!("象/相只能斜向移动两步"));
                }
                // 检查象/相是否被挡
                let mid_row: usize = (from.row + to.row) / 2;
                let mid_col: usize = (from.col + to.col) / 2;
                if self.board[mid_row][mid_col].is_some() {
                    return Err(anyhow!("象/相的路径被挡"));
                }
            },
            // 马
            PieceKind::Horse => {
                // 马只能走日字形
                if !((from.row as isize - to.row as isize).abs() == 2 && (from.col as isize - to.col as isize).abs() == 1)
                    && !((from.row as isize - to.row as isize).abs() == 1 && (from.col as isize - to.col as isize).abs() == 2)
                {
                    return Err(anyhow!("马只能走日字形"));
                }
                // 检查马腿是否被挡
                let row_diff: usize = (from.row as isize - to.row as isize).abs() as usize;
                let col_diff: usize = (from.col as isize - to.col as isize).abs() as usize;
                let leg_row: usize = if row_diff == 2 { (to.row + from.row) / 2 } else { from.row };
                let leg_col: usize = if col_diff == 2 { (to.col + from.col) / 2 } else { from.col };
                if self.board[leg_row][leg_col].is_some() {
                    return Err(anyhow!("马腿被挡"));
                }
            },
            // 车
            PieceKind::Rook => {
                // 车可以横向或者纵向移动
                if from.row != to.row && from.col != to.col {
                    return Err(anyhow!("车只能横向或纵向移动"));
                }
                // 检查中间路径是否被挡
                if from.row == to.row {
                    // 横向移动
                    let start_col: usize = from.col.min(to.col);
                    let end_col: usize = from.col.max(to.col);
                    for col in (start_col + 1)..end_col {
                        if self.board[from.row][col].is_some() {
                            return Err(anyhow!("车的路径被挡"));
                        }
                    }
                } 
                else {
                    // 纵向移动
                    let start_row: usize = from.row.min(to.row);
                    let end_row: usize = from.row.max(to.row);
                    for row in (start_row + 1)..end_row {
                        if self.board[row][from.col].is_some() {
                            return Err(anyhow!("车的路径被挡"));
                        }
                    }
                };
            },
            // 炮
            PieceKind::Cannon => {
                // 炮可以横向或者纵向移动
                if from.row != to.row && from.col != to.col {
                    return Err(anyhow!("炮只能横向或纵向移动"));
                }
                
                // 检查中间路径的棋子数量
                let mut obstacle_count: usize = 0;
                if from.row == to.row {
                    // 横向移动
                    let start_col: usize = from.col.min(to.col);
                    let end_col: usize = from.col.max(to.col);
                    for col in (start_col + 1)..end_col {
                        if self.board[from.row][col].is_some() {
                            obstacle_count += 1;
                        }
                    }
                } else {
                    // 纵向移动
                    let start_row: usize = from.row.min(to.row);
                    let end_row: usize = from.row.max(to.row);
                    for row in (start_row + 1)..end_row {
                        if self.board[row][from.col].is_some() {
                            obstacle_count += 1;
                        }
                    }
                }
                
                // 如果炮是移动，不能有棋子挡路
                // 如果炮是吃子，检查炮架有且仅有一个子
                if self.board[to.row][to.col].is_some() {
                    if obstacle_count == 0 {
                        return Err(anyhow!("缺少炮架"));
                    }
                    else if obstacle_count > 1 {
                        return Err(anyhow!("炮架过多"));
                    }
                } 
                else {
                    if obstacle_count > 0 {
                        return Err(anyhow!("炮的路径被挡"));
                    }
                }
            },
            // 兵/卒
            PieceKind::Pawn => {
                match self.current_player {
                    PlayerColor::Red => {
                        // 兵过河前只能前进
                        if from.row < 5 {
                            if !(to.row == from.row + 1 && to.col == from.col) {
                                return Err(anyhow!("兵过河前只能前进一格"));
                            }
                        }
                        // 兵过河后可以前进或横向移动
                        else {
                            if !(to.row == from.row + 1 && to.col == from.col) &&
                               !(to.row == from.row && (to.col as isize - from.col as isize).abs() == 1) {
                                return Err(anyhow!("兵过河后只能前进或横向移动"));
                            }
                        }
                    },
                    PlayerColor::Black => {
                        // 卒过河前只能前进
                        if from.row > 4  {
                            if !(to.row == from.row - 1 && to.col == from.col) {
                                return Err(anyhow!("卒过河前只能前进一格"));
                            }
                        }
                        // 卒过河后可以前进或横向移动
                        else {
                            if !(to.row == from.row - 1 && to.col == from.col) &&
                               !(to.row == from.row && (to.col as isize - from.col as isize).abs() == 1) {
                                return Err(anyhow!("卒过河后只能前进或横向移动"));
                            }
                        }
                    },
                }
            },
        }

        Ok(())
    }
    
    /// 生成当前局面的FEN字符串
    pub fn to_fen(&self) -> String {
        FenProcessor::generate_fen(self)
    }

    /// 将走法转换为中文表示
    /// 例如: "e2h2" -> "炮二平五"
    pub fn move_to_chinese(&self, move_str: &str) -> Result<String> {
        let (from, to) = Self::parse_move(move_str)?;
        
        // 获取起始位置的棋子
        let piece: Piece = self.board[from.row][from.col]
            .ok_or_else(|| anyhow!("起始位置没有棋子"))?;
        
        // 获取棋子中文名称
        let piece_name: &'static str = piece.get_chinese_name();

        let part1: String;
        // 中文和数字列名
        const ZH_LIST: [&str; 9] = ["九", "八", "七", "六", "五", "四", "三", "二", "一"];
        const DIG_LIST: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

        // 检查该列该类棋子的位置
        let mut same_piece_idxs: Vec<usize> = Vec::new();
        for row in 0..10 {
            if let Some(other_piece) = self.board[row][from.col] {
                if other_piece.color == piece.color && other_piece.kind == piece.kind {
                    same_piece_idxs.push(row);
                }
            }
        }
        
        // 唯一
        if same_piece_idxs.len() == 1 {
            let from_col_name: &str = match self.current_player {
                PlayerColor::Red => ZH_LIST[from.col],
                PlayerColor::Black => DIG_LIST[from.col],
            };
            part1 = format!("{}{}", piece_name, from_col_name);
        }
        // 前/后
        else {
            let pos_type: &str;
            let idx: usize = same_piece_idxs.iter().position(|&r| r == from.row).unwrap();
            match self.current_player {
                PlayerColor::Red => {
                    pos_type = if idx == same_piece_idxs.len() - 1 { "前" } else { "后" };
                }
                PlayerColor::Black => {
                    pos_type = if idx == 0 { "前" } else { "后" };
                }
            };
            part1 = format!("{}{}", pos_type, piece_name);
        }

        let move_type: &str;
        let move_detail: &str;

        // 平
        if from.row == to.row {
            move_type = "平";
            move_detail = match self.current_player {
                PlayerColor::Red => ZH_LIST[to.col],
                PlayerColor::Black => DIG_LIST[to.col],
            };
        }
        // 进 退
        else {
            move_type = match self.current_player {
                PlayerColor::Red => if from.row < to.row { "进" } else { "退" },
                PlayerColor::Black => if from.row > to.row { "进" } else { "退" },
            };
            // 按进退步数
            if from.col == to.col {
                let diff: usize = (from.row as isize - to.row as isize).abs() as usize;
                move_detail = match self.current_player {
                    PlayerColor::Red => ZH_LIST[9 - diff],
                    PlayerColor::Black => DIG_LIST[diff - 1],
                };
            }
            // 按列名
            else {
                move_detail = match self.current_player {
                    PlayerColor::Red => ZH_LIST[to.col],
                    PlayerColor::Black => DIG_LIST[to.col],
                };
            }
        }
        let part2: String = format!("{}{}", move_type, move_detail);

        Ok(format!("{}{}", part1, part2))
    }

    /// 模拟连续走法转换为中文表示
    pub fn pv_to_chinese(&self, pv: &Vec<String>) -> Result<Vec<String>> {
        let mut state: GameState = self.clone();
        let mut zh_moves: Vec<String> = Vec::new();
        for move_str in pv {
            let move_zh: String = state.move_to_chinese(&move_str)?;
            state.apply_move(move_str)?;
            zh_moves.push(move_zh);
        }
        Ok(zh_moves)
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl Piece {
    pub fn get_chinese_name(&self) -> &'static str {
        match (self.color, self.kind) {
            (PlayerColor::Red, PieceKind::General) => "帅",
            (PlayerColor::Red, PieceKind::Advisor) => "仕",
            (PlayerColor::Red, PieceKind::Elephant) => "相",
            (PlayerColor::Red, PieceKind::Horse) => "马",
            (PlayerColor::Red, PieceKind::Rook) => "车",
            (PlayerColor::Red, PieceKind::Cannon) => "炮",
            (PlayerColor::Red, PieceKind::Pawn) => "兵",
            (PlayerColor::Black, PieceKind::General) => "将",
            (PlayerColor::Black, PieceKind::Advisor) => "士",
            (PlayerColor::Black, PieceKind::Elephant) => "象",
            (PlayerColor::Black, PieceKind::Horse) => "马",
            (PlayerColor::Black, PieceKind::Rook) => "车",
            (PlayerColor::Black, PieceKind::Cannon) => "炮",
            (PlayerColor::Black, PieceKind::Pawn) => "卒",
        }
    }
}
