use crate::game::fen::FenProcessor;
use crate::utils::*;

/// 玩家颜色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerColor {
    /// 红方
    Red,
    /// 黑方
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::fen::FenProcessor;

    #[test]
    fn test_flying_general() {
        // 红帅在 e0，黑将 e9。中间无子。轮到红方。
        // FEN: 4k4/9/9/9/9/9/9/9/9/4K4 w
        let mut state = FenProcessor::parse_fen("4k4/9/9/9/9/9/9/9/9/4K4 w").unwrap();

        // 此时已经被将军（飞将）
        assert!(state.is_check(PlayerColor::Red));

        // 尝试平帅：e0 -> d0 (4,0 -> 3,0)
        // 这步是合法的，因为移开后就不被将军了
        assert!(state.apply_move("e0d0").is_ok());
        assert!(!state.is_check(PlayerColor::Red));
    }

    #[test]
    fn test_cant_move_into_check() {
        // 红帅 e0，黑车 e9。
        // FEN: 4r4/9/9/9/9/9/9/9/9/4K4 w
        let mut state = FenProcessor::parse_fen("4r4/9/9/9/9/9/9/9/9/4K4 w").unwrap();

        // 此时被将军
        assert!(state.is_check(PlayerColor::Red));

        // 尝试进帅：e0 -> e1
        // e1 依然在黑车 e9 的攻击范围内，所以这步非法（不能送将）
        assert!(state.apply_move("e0e1").is_err());

        // 尝试平帅：e0 -> d0
        // 这步合法
        assert!(state.apply_move("e0d0").is_ok());
    }

    #[test]
    fn test_undo() {
        let mut state = GameState::new();
        let fen_start = state.to_fen();

        state.apply_move("h2e2").unwrap();
        assert_ne!(state.to_fen(), fen_start);

        state.undo_move().unwrap();
        assert_eq!(state.to_fen(), fen_start);
    }
}

/// 棋子种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    General,  // 将/帅
    Advisor,  // 士/仕
    Elephant, // 象/相
    Horse,    // 马
    Rook,     // 车
    Cannon,   // 炮
    Pawn,     // 兵/卒
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
    /// 走法历史
    pub history: Vec<String>,
    /// 详细的走法历史，用于悔棋
    pub move_history: Vec<(Position, Position, Option<Piece>)>,
    /// 重做历史
    pub redo_history: Vec<String>,
    /// 棋盘是否翻转显示
    pub flipped: bool,
}

impl GameState {
    /// 创建初始游戏状态
    pub fn new() -> Self {
        FenProcessor::parse_fen("rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w")
            .unwrap()
    }

    /// 重置为初始状态
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// 应用一个走法（外部调用，会清空重做历史）
    pub fn apply_move(&mut self, move_str: &str) -> Result<()> {
        self.redo_history.clear();
        self.apply_move_internal(move_str)
    }

    /// 内部应用走法
    fn apply_move_internal(&mut self, move_str: &str) -> Result<()> {
        // 将走法字符串转换为坐标
        let (from, to) = Self::parse_move(move_str)?;

        // 合法性检查
        self.is_valid_move(from, to)?;

        // 记录走法
        let chinese_move: String = self.move_to_chinese(move_str)?;
        log_info!(self.current_player, move_str, chinese_move, from, to);
        self.history.push(chinese_move);

        // 记录详细历史
        let captured = self.board[to.row][to.col];
        self.move_history.push((from, to, captured));

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

        Ok((
            Position {
                col: from_x,
                row: from_y,
            },
            Position {
                col: to_x,
                row: to_y,
            },
        ))
    }

    /// 获取某方将帅的位置
    pub fn get_king_pos(&self, color: PlayerColor) -> Option<Position> {
        for row in 0..10 {
            for col in 0..9 {
                if let Some(piece) = self.board[row][col] {
                    if piece.kind == PieceKind::General && piece.color == color {
                        return Some(Position { row, col });
                    }
                }
            }
        }
        None
    }

    /// 检查某方是否被将军
    pub fn is_check(&self, color: PlayerColor) -> bool {
        let king_pos = match self.get_king_pos(color) {
            Some(p) => p,
            None => return false,
        };

        // 1. 常规攻击
        if self.is_attacked(king_pos, color.opponent()) {
            return true;
        }

        // 2. 飞将（对脸杀）
        if let Some(opp_king) = self.get_king_pos(color.opponent()) {
            if king_pos.col == opp_king.col {
                let min_row = king_pos.row.min(opp_king.row);
                let max_row = king_pos.row.max(opp_king.row);
                let mut has_obstacle = false;
                for r in (min_row + 1)..max_row {
                    if self.board[r][king_pos.col].is_some() {
                        has_obstacle = true;
                        break;
                    }
                }
                if !has_obstacle {
                    return true;
                }
            }
        }

        false
    }

    /// 检查位置是否被某方攻击
    pub fn is_attacked(&self, pos: Position, attacker_color: PlayerColor) -> bool {
        for row in 0..10 {
            for col in 0..9 {
                if let Some(piece) = self.board[row][col] {
                    if piece.color == attacker_color {
                        let from = Position { row, col };
                        if self.check_piece_rules(from, pos).is_ok() {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// 检查当前玩家是否有合法走法（优化版）
    pub fn get_legal_moves(&self) -> Vec<String> {
        let mut moves = Vec::new();
        let color = self.current_player;

        for r1 in 0..10 {
            for c1 in 0..9 {
                if let Some(p) = self.board[r1][c1] {
                    if p.color == color {
                        let from = Position { row: r1, col: c1 };
                        // 使用棋子专属的生成逻辑，而不是遍历全图
                        let piece_moves = self.get_piece_legal_moves(from);
                        
                        let col_chars = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
                        for to in piece_moves {
                             let move_str = format!(
                                "{}{}{}{}",
                                col_chars[from.col], from.row, col_chars[to.col], to.row
                            );
                            moves.push(move_str);
                        }
                    }
                }
            }
        }
        moves
    }

    /// 获取指定位置棋子的所有合法目标位置（优化版）
    pub fn get_piece_legal_moves(&self, from: Position) -> Vec<Position> {
        let mut moves = Vec::new();
        // 只有当前玩家的棋子才能移动
        let piece = if let Some(p) = self.board[from.row][from.col] {
            if p.color != self.current_player {
                return moves;
            }
            p
        } else {
            return moves;
        };

        // 辅助闭包：检查并添加走法
        let mut try_add_move = |to: Position| {
             // 检查基础规则
             if self.check_piece_rules(from, to).is_ok() {
                 // 检查送将 (复用 is_valid_move 的逻辑，但这里我们知道 check_piece_rules 已经通过)
                 // is_valid_move 会再次检查 check_piece_rules，为了复用代码简单点，直接调用 is_valid_move
                 // 这里的性能损耗在于 check_piece_rules 被调用了两次（一次这里，一次 is_valid_move）
                 // 优化：直接调用底层检查
                 let mut temp_board = self.board;
                 temp_board[to.row][to.col] = temp_board[from.row][from.col];
                 temp_board[from.row][from.col] = None;
                 
                 if !Self::is_check_on_board(&temp_board, self.current_player) {
                     moves.push(to);
                 }
             }
        };

        match piece.kind {
            PieceKind::General => {
                 let offsets = [(0, 1), (0, -1), (1, 0), (-1, 0)];
                 for (dr, dc) in offsets {
                     let nr = from.row as isize + dr;
                     let nc = from.col as isize + dc;
                     if nr >= 0 && nr < 10 && nc >= 0 && nc < 9 {
                         try_add_move(Position { row: nr as usize, col: nc as usize });
                     }
                 }
            }
            PieceKind::Advisor => {
                 let offsets = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
                 for (dr, dc) in offsets {
                     let nr = from.row as isize + dr;
                     let nc = from.col as isize + dc;
                     if nr >= 0 && nr < 10 && nc >= 0 && nc < 9 {
                         try_add_move(Position { row: nr as usize, col: nc as usize });
                     }
                 }
            }
            PieceKind::Elephant => {
                 let offsets = [(2, 2), (2, -2), (-2, 2), (-2, -2)];
                 for (dr, dc) in offsets {
                     let nr = from.row as isize + dr;
                     let nc = from.col as isize + dc;
                     if nr >= 0 && nr < 10 && nc >= 0 && nc < 9 {
                         try_add_move(Position { row: nr as usize, col: nc as usize });
                     }
                 }
            }
            PieceKind::Horse => {
                 let offsets = [
                     (2, 1), (2, -1), (-2, 1), (-2, -1),
                     (1, 2), (1, -2), (-1, 2), (-1, -2)
                 ];
                 for (dr, dc) in offsets {
                     let nr = from.row as isize + dr;
                     let nc = from.col as isize + dc;
                     if nr >= 0 && nr < 10 && nc >= 0 && nc < 9 {
                         try_add_move(Position { row: nr as usize, col: nc as usize });
                     }
                 }
            }
            PieceKind::Rook => {
                 // 横向
                 for c in 0..9 {
                     if c != from.col { try_add_move(Position { row: from.row, col: c }); }
                 }
                 // 纵向
                 for r in 0..10 {
                     if r != from.row { try_add_move(Position { row: r, col: from.col }); }
                 }
                 // 注意：这里仍然是遍历整行整列，但比全图好。
                 // 更好的优化是碰到阻挡就停止，但复用 check_piece_rules 比较简单。
                 // check_piece_rules 已经处理了阻挡逻辑。
                 // 为了极致性能，应该重写这里的逻辑来做射线检测。
            }
            PieceKind::Cannon => {
                 // 同车
                 for c in 0..9 {
                     if c != from.col { try_add_move(Position { row: from.row, col: c }); }
                 }
                 for r in 0..10 {
                     if r != from.row { try_add_move(Position { row: r, col: from.col }); }
                 }
            }
            PieceKind::Pawn => {
                 let forward = if piece.color == PlayerColor::Red { 1 } else { -1 };
                 // 前进
                 let nr = from.row as isize + forward;
                 if nr >= 0 && nr < 10 {
                      try_add_move(Position { row: nr as usize, col: from.col });
                 }
                 // 过河后横走
                 let is_crossed = match piece.color {
                     PlayerColor::Red => from.row > 4,
                     PlayerColor::Black => from.row < 5,
                 };
                 if is_crossed {
                     if from.col > 0 { try_add_move(Position { row: from.row, col: from.col - 1 }); }
                     if from.col < 8 { try_add_move(Position { row: from.row, col: from.col + 1 }); }
                 }
            }
        }
        
        moves
    }

    /// 获取最后一步走法
    pub fn get_last_move(&self) -> Option<(Position, Position)> {
        self.move_history.last().map(|(from, to, _)| (*from, *to))
    }

    /// 检查胜负
    /// 返回 None 表示未分胜负，Some(color) 表示 color 获胜
    pub fn check_winner(&self) -> Option<PlayerColor> {
        // 如果当前玩家被将军且无路可走 -> 对方胜
        // 如果当前玩家未被将军但无路可走 -> 困毙 -> 对方胜

        let legal_moves = self.get_legal_moves();
        if legal_moves.is_empty() {
            return Some(self.current_player.opponent());
        }

        None
    }

    /// 悔棋
    pub fn undo_move(&mut self) -> Result<()> {
        if let Some((from, to, captured)) = self.move_history.pop() {
            // 还原棋子
            self.board[from.row][from.col] = self.board[to.row][to.col];
            self.board[to.row][to.col] = captured;

            // 还原历史记录
            self.history.pop();

            // 还原玩家
            self.current_player = self.current_player.opponent();

            // 记录到重做历史
            let col_chars = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
            let move_str = format!(
                "{}{}{}{}",
                col_chars[from.col], from.row, col_chars[to.col], to.row
            );
            self.redo_history.push(move_str);

            Ok(())
        } else {
            Err(anyhow!("无棋可悔"))
        }
    }

    /// 重做
    pub fn redo_move(&mut self) -> Result<()> {
        if let Some(move_str) = self.redo_history.pop() {
            if let Err(e) = self.apply_move_internal(&move_str) {
                // 如果失败（不应该发生），放回去
                self.redo_history.push(move_str);
                return Err(e);
            }
            Ok(())
        } else {
            Err(anyhow!("无棋可重做"))
        }
    }

    /// 综合合法性验证（包含规则、送将检查）
    pub fn is_valid_move(&self, from: Position, to: Position) -> Result<()> {
        // 1. 检查是否轮到该玩家
        let piece = self.board[from.row][from.col].ok_or_else(|| anyhow!("起始位置没有棋子"))?;
        if piece.color != self.current_player {
            return Err(anyhow!("不能移动对方的棋子"));
        }

        // 2. 检查基本移动规则
        self.check_piece_rules(from, to)?;

        // 3. 模拟移动，检查是否送将（移动后己方被将军）
        // 优化：不再使用 clone()，而是操作临时棋盘
        let mut temp_board = self.board;
        temp_board[to.row][to.col] = temp_board[from.row][from.col];
        temp_board[from.row][from.col] = None;

        if Self::is_check_on_board(&temp_board, self.current_player) {
            return Err(anyhow!("不能送将"));
        }

        Ok(())
    }

    /// 检查某方是否被将军（在指定棋盘上）
    pub fn is_check_on_board(board: &[[Option<Piece>; 9]; 10], color: PlayerColor) -> bool {
        let king_pos = match Self::get_king_pos_on_board(board, color) {
            Some(p) => p,
            None => return false,
        };

        // 1. 常规攻击
        if Self::is_attacked_on_board(board, king_pos, color.opponent()) {
            return true;
        }

        // 2. 飞将（对脸杀）
        if let Some(opp_king) = Self::get_king_pos_on_board(board, color.opponent()) {
            if king_pos.col == opp_king.col {
                let min_row = king_pos.row.min(opp_king.row);
                let max_row = king_pos.row.max(opp_king.row);
                let mut has_obstacle = false;
                for r in (min_row + 1)..max_row {
                    if board[r][king_pos.col].is_some() {
                        has_obstacle = true;
                        break;
                    }
                }
                if !has_obstacle {
                    return true;
                }
            }
        }

        false
    }

    /// 检查位置是否被某方攻击（在指定棋盘上）
    pub fn is_attacked_on_board(board: &[[Option<Piece>; 9]; 10], pos: Position, attacker_color: PlayerColor) -> bool {
        for row in 0..10 {
            for col in 0..9 {
                if let Some(piece) = board[row][col] {
                    if piece.color == attacker_color {
                        let from = Position { row, col };
                        if Self::check_piece_rules_on_board(board, from, pos).is_ok() {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// 获取某方将帅的位置（在指定棋盘上）
    pub fn get_king_pos_on_board(board: &[[Option<Piece>; 9]; 10], color: PlayerColor) -> Option<Position> {
        for row in 0..10 {
            for col in 0..9 {
                if let Some(piece) = board[row][col] {
                    if piece.kind == PieceKind::General && piece.color == color {
                        return Some(Position { row, col });
                    }
                }
            }
        }
        None
    }

    /// 走法合法性验证（基础规则，在指定棋盘上）
    pub fn check_piece_rules_on_board(board: &[[Option<Piece>; 9]; 10], from: Position, to: Position) -> Result<()> {
        // 检查起始位置是否有棋子
        let piece: Piece =
            board[from.row][from.col].ok_or_else(|| anyhow!("起始位置没有棋子"))?;

        // 检查目标位置是否有己方棋子
        if let Some(target_piece) = board[to.row][to.col] {
            if target_piece.color == piece.color {
                return Err(anyhow!("目标位置已有己方棋子"));
            }
        }

        // 根据棋子种类检查
        match piece.kind {
            // 将/帅
            PieceKind::General => {
                // 将帅只能在九宫内移动
                match piece.color {
                    PlayerColor::Red => {
                        if to.row > 2 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("帅只能在九宫内移动"));
                        }
                    }
                    PlayerColor::Black => {
                        if to.row < 7 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("将只能在九宫内移动"));
                        }
                    }
                }
                // 将帅只能横向或纵向移动一步
                if (from.row != to.row && from.col != to.col)
                    || (from.row == to.row && (from.col as isize - to.col as isize).abs() > 1)
                    || (from.col == to.col && (from.row as isize - to.row as isize).abs() > 1)
                {
                    return Err(anyhow!("将帅只能横向或纵向移动一步"));
                }
            }
            // 士/仕
            PieceKind::Advisor => {
                // 士/仕只能在九宫内移动
                match piece.color {
                    PlayerColor::Red => {
                        if to.row > 2 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("仕只能在九宫内移动"));
                        }
                    }
                    PlayerColor::Black => {
                        if to.row < 7 || to.col < 3 || to.col > 5 {
                            return Err(anyhow!("士只能在九宫内移动"));
                        }
                    }
                }
                // 士/仕只能斜向移动一步
                if (from.row as isize - to.row as isize).abs() != 1
                    || (from.col as isize - to.col as isize).abs() != 1
                {
                    return Err(anyhow!("士/仕只能斜向移动一步"));
                }
            }
            // 象/相
            PieceKind::Elephant => {
                // 象/相不能过河
                match piece.color {
                    PlayerColor::Red => {
                        if to.row > 4 {
                            return Err(anyhow!("相不能过河"));
                        }
                    }
                    PlayerColor::Black => {
                        if to.row < 5 {
                            return Err(anyhow!("象不能过河"));
                        }
                    }
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
                if board[mid_row][mid_col].is_some() {
                    return Err(anyhow!("象/相的路径被挡"));
                }
            }
            // 马
            PieceKind::Horse => {
                // 马只能走日字形
                if !((from.row as isize - to.row as isize).abs() == 2
                    && (from.col as isize - to.col as isize).abs() == 1
                    || (from.row as isize - to.row as isize).abs() == 1
                        && (from.col as isize - to.col as isize).abs() == 2)
                {
                    return Err(anyhow!("马必须走日字"));
                }
                // 检查马腿是否被挡
                let row_diff: usize = (from.row as isize - to.row as isize).unsigned_abs();
                let col_diff: usize = (from.col as isize - to.col as isize).unsigned_abs();
                let leg_row: usize = if row_diff == 2 {
                    (to.row + from.row) / 2
                } else {
                    from.row
                };
                let leg_col: usize = if col_diff == 2 {
                    (to.col + from.col) / 2
                } else {
                    from.col
                };
                if board[leg_row][leg_col].is_some() {
                    return Err(anyhow!("马腿被挡"));
                }
            }
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
                        if board[from.row][col].is_some() {
                            return Err(anyhow!("车的路径被挡"));
                        }
                    }
                } else {
                    // 纵向移动
                    let start_row: usize = from.row.min(to.row);
                    let end_row: usize = from.row.max(to.row);
                    for row in (start_row + 1)..end_row {
                        if board[row][from.col].is_some() {
                            return Err(anyhow!("车的路径被挡"));
                        }
                    }
                };
            }
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
                        if board[from.row][col].is_some() {
                            obstacle_count += 1;
                        }
                    }
                } else {
                    // 纵向移动
                    let start_row: usize = from.row.min(to.row);
                    let end_row: usize = from.row.max(to.row);
                    for row in (start_row + 1)..end_row {
                        if board[row][from.col].is_some() {
                            obstacle_count += 1;
                        }
                    }
                }

                // 吃子规则：中间必须有一个子（炮架）
                // 移动规则：中间不能有子
                if let Some(_) = board[to.row][to.col] {
                    // 吃子
                    if obstacle_count != 1 {
                        return Err(anyhow!("炮吃子必须隔一个棋子"));
                    }
                } else {
                    // 移动
                    if obstacle_count != 0 {
                        return Err(anyhow!("炮移动路径不能有阻挡"));
                    }
                }
            }
            // 兵/卒
            PieceKind::Pawn => {
                match piece.color {
                    PlayerColor::Red => {
                        // 兵只能向前移动
                        if to.row < from.row {
                            return Err(anyhow!("兵不能后退"));
                        }
                        // 过河前只能向前
                        if from.row <= 4 && from.row == to.row {
                             return Err(anyhow!("兵过河前只能向前"));
                        }
                        // 只能移动一步
                        if (to.row as isize - from.row as isize) + (to.col as isize - from.col as isize).abs() != 1 {
                            return Err(anyhow!("兵每次只能移动一步"));
                        }
                    }
                    PlayerColor::Black => {
                        // 卒只能向前移动（黑方视角前是行号减小）
                        if to.row > from.row {
                            return Err(anyhow!("卒不能后退"));
                        }
                         // 过河前只能向前
                        if from.row >= 5 && from.row == to.row {
                             return Err(anyhow!("卒过河前只能向前"));
                        }
                        // 只能移动一步
                        if (from.row as isize - to.row as isize) + (to.col as isize - from.col as isize).abs() != 1 {
                            return Err(anyhow!("卒每次只能移动一步"));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 走法合法性验证（基础规则）
    pub fn check_piece_rules(&self, from: Position, to: Position) -> Result<()> {
        Self::check_piece_rules_on_board(&self.board, from, to)
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
        let piece: Piece =
            self.board[from.row][from.col].ok_or_else(|| anyhow!("起始位置没有棋子"))?;

        // 获取棋子中文名称
        let piece_name: &'static str = piece.get_chinese_name();

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
        let part1: String = if same_piece_idxs.len() == 1 {
            let from_col_name: &str = match self.current_player {
                PlayerColor::Red => ZH_LIST[from.col],
                PlayerColor::Black => DIG_LIST[from.col],
            };
            format!("{}{}", piece_name, from_col_name)
        }
        // 前/后
        else {
            let idx: usize = same_piece_idxs.iter().position(|&r| r == from.row).unwrap();
            let pos_type: &str = match self.current_player {
                PlayerColor::Red => {
                    if idx == same_piece_idxs.len() - 1 {
                        "前"
                    } else {
                        "后"
                    }
                }
                PlayerColor::Black => {
                    if idx == 0 {
                        "前"
                    } else {
                        "后"
                    }
                }
            };
            format!("{}{}", pos_type, piece_name)
        };

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
                PlayerColor::Red => {
                    if from.row < to.row {
                        "进"
                    } else {
                        "退"
                    }
                }
                PlayerColor::Black => {
                    if from.row > to.row {
                        "进"
                    } else {
                        "退"
                    }
                }
            };
            // 按进退步数
            if from.col == to.col {
                let diff: usize = (from.row as isize - to.row as isize).unsigned_abs();
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
    /// 如果某步转换失败或非法，后续步骤将保留原始字符串
    pub fn pv_to_chinese(&self, pv: &[String]) -> Vec<String> {
        let mut state = self.clone();
        let mut zh_moves = Vec::new();
        
        for move_str in pv {
            match state.move_to_chinese(move_str) {
                Ok(zh) => {
                    zh_moves.push(zh);
                    // 尝试应用走法，如果失败则后续无法继续转换
                    if state.apply_move(move_str).is_err() {
                        // 当前走法虽然转换中文成功，但应用失败（可能是非法走法）
                        // 这种情况比较少见，因为move_to_chinese已经做了一些检查
                        // 但apply_move有更严格的规则（如送将）
                        // 我们继续后续的原始字符串
                        let current_idx = zh_moves.len();
                        if current_idx < pv.len() {
                            zh_moves.extend(pv[current_idx..].iter().cloned());
                        }
                        break;
                    }
                }
                Err(_) => {
                    // 转换失败，添加原始字符串并结束转换
                    let current_idx = zh_moves.len();
                    zh_moves.extend(pv[current_idx..].iter().cloned());
                    break;
                }
            }
        }
        zh_moves
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
