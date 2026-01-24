use crate::game::fen::FenProcessor;
use crate::game::notation::move_to_chinese;
use crate::game::rules::check_piece_rules;
use crate::game::types::{Board, Piece, PieceKind, PlayerColor, Position, format_move, parse_move};
use crate::utils::*;

#[derive(Clone)]
pub struct GameState {
    pub board: Board,
    pub current_player: PlayerColor,
    pub history: Vec<String>,
    pub move_history: Vec<(Position, Position, Option<Piece>)>,
    pub redo_history: Vec<String>,
    pub flipped: bool,
}

impl GameState {
    pub fn new() -> Self {
        FenProcessor::parse_fen("rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w")
            .expect("initial FEN is invalid - this is a bug")
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn apply_move(&mut self, move_str: &str) -> Result<()> {
        self.redo_history.clear();
        self.apply_move_internal(move_str)
    }

    fn apply_move_internal(&mut self, move_str: &str) -> Result<()> {
        let (from, to) = parse_move(move_str)?;
        self.is_valid_move(from, to)?;

        let chinese_move: String = self.move_to_chinese(move_str)?;
        log_info!(self.current_player, move_str, chinese_move, from, to);
        self.history.push(chinese_move);

        let captured = self.board[to.row][to.col];
        self.move_history.push((from, to, captured));

        self.board[to.row][to.col] = self.board[from.row][from.col];
        self.board[from.row][from.col] = None;

        self.current_player = self.current_player.opponent();

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_king_pos(&self, color: PlayerColor) -> Option<Position> {
        Self::get_king_pos_on_board(&self.board, color)
    }

    pub fn is_check(&self, color: PlayerColor) -> bool {
        Self::is_check_on_board(&self.board, color)
    }

    #[allow(dead_code)]
    pub fn is_attacked(&self, pos: Position, attacker_color: PlayerColor) -> bool {
        Self::is_attacked_on_board(&self.board, pos, attacker_color)
    }

    pub fn get_legal_moves(&self) -> Vec<String> {
        let mut moves = Vec::new();
        let color = self.current_player;

        for r1 in 0..10 {
            for c1 in 0..9 {
                if let Some(p) = self.board[r1][c1]
                    && p.color == color
                {
                    let from = Position { row: r1, col: c1 };
                    let piece_moves = self.get_piece_legal_moves(from);

                    for to in piece_moves {
                        moves.push(format_move(from, to));
                    }
                }
            }
        }
        moves
    }

    pub fn get_piece_legal_moves(&self, from: Position) -> Vec<Position> {
        let mut moves = Vec::new();
        let piece = if let Some(p) = self.board[from.row][from.col] {
            if p.color != self.current_player {
                return moves;
            }
            p
        } else {
            return moves;
        };

        let mut try_add_move = |to: Position| {
            if check_piece_rules(&self.board, from, to).is_ok() {
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
                    if (0..10).contains(&nr) && (0..9).contains(&nc) {
                        try_add_move(Position {
                            row: nr as usize,
                            col: nc as usize,
                        });
                    }
                }
            }
            PieceKind::Advisor => {
                let offsets = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
                for (dr, dc) in offsets {
                    let nr = from.row as isize + dr;
                    let nc = from.col as isize + dc;
                    if (0..10).contains(&nr) && (0..9).contains(&nc) {
                        try_add_move(Position {
                            row: nr as usize,
                            col: nc as usize,
                        });
                    }
                }
            }
            PieceKind::Elephant => {
                let offsets = [(2, 2), (2, -2), (-2, 2), (-2, -2)];
                for (dr, dc) in offsets {
                    let nr = from.row as isize + dr;
                    let nc = from.col as isize + dc;
                    if (0..10).contains(&nr) && (0..9).contains(&nc) {
                        try_add_move(Position {
                            row: nr as usize,
                            col: nc as usize,
                        });
                    }
                }
            }
            PieceKind::Horse => {
                let offsets = [
                    (2, 1),
                    (2, -1),
                    (-2, 1),
                    (-2, -1),
                    (1, 2),
                    (1, -2),
                    (-1, 2),
                    (-1, -2),
                ];
                for (dr, dc) in offsets {
                    let nr = from.row as isize + dr;
                    let nc = from.col as isize + dc;
                    if (0..10).contains(&nr) && (0..9).contains(&nc) {
                        try_add_move(Position {
                            row: nr as usize,
                            col: nc as usize,
                        });
                    }
                }
            }
            PieceKind::Rook => {
                for c in 0..9 {
                    if c != from.col {
                        try_add_move(Position {
                            row: from.row,
                            col: c,
                        });
                    }
                }
                for r in 0..10 {
                    if r != from.row {
                        try_add_move(Position {
                            row: r,
                            col: from.col,
                        });
                    }
                }
            }
            PieceKind::Cannon => {
                for c in 0..9 {
                    if c != from.col {
                        try_add_move(Position {
                            row: from.row,
                            col: c,
                        });
                    }
                }
                for r in 0..10 {
                    if r != from.row {
                        try_add_move(Position {
                            row: r,
                            col: from.col,
                        });
                    }
                }
            }
            PieceKind::Pawn => {
                let forward = if piece.color == PlayerColor::Red {
                    1
                } else {
                    -1
                };
                let nr = from.row as isize + forward;
                if (0..10).contains(&nr) {
                    try_add_move(Position {
                        row: nr as usize,
                        col: from.col,
                    });
                }
                let is_crossed = match piece.color {
                    PlayerColor::Red => from.row > 4,
                    PlayerColor::Black => from.row < 5,
                };
                if is_crossed {
                    if from.col > 0 {
                        try_add_move(Position {
                            row: from.row,
                            col: from.col - 1,
                        });
                    }
                    if from.col < 8 {
                        try_add_move(Position {
                            row: from.row,
                            col: from.col + 1,
                        });
                    }
                }
            }
        }

        moves
    }

    pub fn get_last_move(&self) -> Option<(Position, Position)> {
        self.move_history.last().map(|(from, to, _)| (*from, *to))
    }

    pub fn check_winner(&self) -> Option<PlayerColor> {
        let legal_moves = self.get_legal_moves();
        if legal_moves.is_empty() {
            return Some(self.current_player.opponent());
        }

        None
    }

    pub fn undo_move(&mut self) -> Result<()> {
        if let Some((from, to, captured)) = self.move_history.pop() {
            self.board[from.row][from.col] = self.board[to.row][to.col];
            self.board[to.row][to.col] = captured;

            self.history.pop();
            self.current_player = self.current_player.opponent();

            self.redo_history.push(format_move(from, to));

            Ok(())
        } else {
            Err(anyhow!("无棋可悔"))
        }
    }

    pub fn redo_move(&mut self) -> Result<()> {
        if let Some(move_str) = self.redo_history.pop() {
            if let Err(e) = self.apply_move_internal(&move_str) {
                self.redo_history.push(move_str);
                return Err(e);
            }
            Ok(())
        } else {
            Err(anyhow!("无棋可重做"))
        }
    }

    pub fn is_valid_move(&self, from: Position, to: Position) -> Result<()> {
        let piece = self.board[from.row][from.col].ok_or_else(|| anyhow!("起始位置没有棋子"))?;
        if piece.color != self.current_player {
            return Err(anyhow!("不能移动对方的棋子"));
        }

        check_piece_rules(&self.board, from, to)?;

        let mut temp_board = self.board;
        temp_board[to.row][to.col] = temp_board[from.row][from.col];
        temp_board[from.row][from.col] = None;

        if Self::is_check_on_board(&temp_board, self.current_player) {
            return Err(anyhow!("不能送将"));
        }

        Ok(())
    }

    pub fn is_check_on_board(board: &Board, color: PlayerColor) -> bool {
        let king_pos = match Self::get_king_pos_on_board(board, color) {
            Some(p) => p,
            None => return false,
        };

        if Self::is_attacked_on_board(board, king_pos, color.opponent()) {
            return true;
        }

        if let Some(opp_king) = Self::get_king_pos_on_board(board, color.opponent())
            && king_pos.col == opp_king.col
        {
            let min_row = king_pos.row.min(opp_king.row);
            let max_row = king_pos.row.max(opp_king.row);
            let mut has_obstacle = false;
            #[allow(clippy::needless_range_loop)]
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

        false
    }

    pub fn is_attacked_on_board(board: &Board, pos: Position, attacker_color: PlayerColor) -> bool {
        #[allow(clippy::needless_range_loop)]
        for row in 0..10 {
            #[allow(clippy::needless_range_loop)]
            for col in 0..9 {
                if let Some(piece) = board[row][col]
                    && piece.color == attacker_color
                {
                    let from = Position { row, col };
                    if check_piece_rules(board, from, pos).is_ok() {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn get_king_pos_on_board(board: &Board, color: PlayerColor) -> Option<Position> {
        #[allow(clippy::needless_range_loop)]
        for row in 0..10 {
            #[allow(clippy::needless_range_loop)]
            for col in 0..9 {
                if let Some(piece) = board[row][col]
                    && piece.kind == PieceKind::General
                    && piece.color == color
                {
                    return Some(Position { row, col });
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn check_piece_rules(&self, from: Position, to: Position) -> Result<()> {
        check_piece_rules(&self.board, from, to)
    }

    pub fn to_fen(&self) -> String {
        FenProcessor::generate_fen(self)
    }

    pub fn move_to_chinese(&self, move_str: &str) -> Result<String> {
        let (from, to) = parse_move(move_str)?;
        move_to_chinese(&self.board, self.current_player, from, to)
    }

    pub fn pv_to_chinese(&self, pv: &[String]) -> Vec<String> {
        let mut state = self.clone();
        let mut zh_moves = Vec::new();

        for move_str in pv {
            match state.move_to_chinese(move_str) {
                Ok(zh) => {
                    zh_moves.push(zh);
                    if state.apply_move(move_str).is_err() {
                        let current_idx = zh_moves.len();
                        if current_idx < pv.len() {
                            zh_moves.extend(pv[current_idx..].iter().cloned());
                        }
                        break;
                    }
                }
                Err(_) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::fen::FenProcessor;

    #[test]
    fn test_flying_general() {
        let mut state = FenProcessor::parse_fen("4k4/9/9/9/9/9/9/9/9/4K4 w").unwrap();

        assert!(state.is_check(PlayerColor::Red));

        assert!(state.apply_move("e0d0").is_ok());
        assert!(!state.is_check(PlayerColor::Red));
    }

    #[test]
    fn test_cant_move_into_check() {
        let mut state = FenProcessor::parse_fen("4r4/9/9/9/9/9/9/9/9/4K4 w").unwrap();

        assert!(state.is_check(PlayerColor::Red));

        assert!(state.apply_move("e0e1").is_err());

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
