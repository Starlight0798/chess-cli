use crate::game::types::{Board, Piece, PieceKind, PlayerColor, Position};
use crate::utils::*;

pub fn check_piece_rules(board: &Board, from: Position, to: Position) -> Result<()> {
    let piece: Piece = board[from.row][from.col].ok_or_else(|| anyhow!("起始位置没有棋子"))?;

    if let Some(target_piece) = board[to.row][to.col]
        && target_piece.color == piece.color
    {
        return Err(anyhow!("目标位置已有己方棋子"));
    }

    match piece.kind {
        PieceKind::General => check_general_rules(piece.color, from, to)?,
        PieceKind::Advisor => check_advisor_rules(piece.color, from, to)?,
        PieceKind::Elephant => check_elephant_rules(board, piece.color, from, to)?,
        PieceKind::Horse => check_horse_rules(board, from, to)?,
        PieceKind::Rook => check_rook_rules(board, from, to)?,
        PieceKind::Cannon => check_cannon_rules(board, from, to)?,
        PieceKind::Pawn => check_pawn_rules(piece.color, from, to)?,
    }

    Ok(())
}

fn check_general_rules(color: PlayerColor, from: Position, to: Position) -> Result<()> {
    match color {
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

    if (from.row != to.row && from.col != to.col)
        || (from.row == to.row && (from.col as isize - to.col as isize).abs() > 1)
        || (from.col == to.col && (from.row as isize - to.row as isize).abs() > 1)
    {
        return Err(anyhow!("将帅只能横向或纵向移动一步"));
    }

    Ok(())
}

fn check_advisor_rules(color: PlayerColor, from: Position, to: Position) -> Result<()> {
    match color {
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

    if (from.row as isize - to.row as isize).abs() != 1
        || (from.col as isize - to.col as isize).abs() != 1
    {
        return Err(anyhow!("士/仕只能斜向移动一步"));
    }

    Ok(())
}

fn check_elephant_rules(
    board: &Board,
    color: PlayerColor,
    from: Position,
    to: Position,
) -> Result<()> {
    match color {
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

    if (from.row as isize - to.row as isize).abs() != 2
        || (from.col as isize - to.col as isize).abs() != 2
    {
        return Err(anyhow!("象/相只能斜向移动两步"));
    }

    let mid_row: usize = (from.row + to.row) / 2;
    let mid_col: usize = (from.col + to.col) / 2;
    if board[mid_row][mid_col].is_some() {
        return Err(anyhow!("象/相的路径被挡"));
    }

    Ok(())
}

fn check_horse_rules(board: &Board, from: Position, to: Position) -> Result<()> {
    if !((from.row as isize - to.row as isize).abs() == 2
        && (from.col as isize - to.col as isize).abs() == 1
        || (from.row as isize - to.row as isize).abs() == 1
            && (from.col as isize - to.col as isize).abs() == 2)
    {
        return Err(anyhow!("马必须走日字"));
    }

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

    Ok(())
}

fn check_rook_rules(board: &Board, from: Position, to: Position) -> Result<()> {
    if from.row != to.row && from.col != to.col {
        return Err(anyhow!("车只能横向或纵向移动"));
    }

    if from.row == to.row {
        let start_col: usize = from.col.min(to.col);
        let end_col: usize = from.col.max(to.col);
        #[allow(clippy::needless_range_loop)]
        for col in (start_col + 1)..end_col {
            if board[from.row][col].is_some() {
                return Err(anyhow!("车的路径被挡"));
            }
        }
    } else {
        let start_row: usize = from.row.min(to.row);
        let end_row: usize = from.row.max(to.row);
        #[allow(clippy::needless_range_loop)]
        for row in (start_row + 1)..end_row {
            if board[row][from.col].is_some() {
                return Err(anyhow!("车的路径被挡"));
            }
        }
    };

    Ok(())
}

fn check_cannon_rules(board: &Board, from: Position, to: Position) -> Result<()> {
    if from.row != to.row && from.col != to.col {
        return Err(anyhow!("炮只能横向或纵向移动"));
    }

    let mut obstacle_count: usize = 0;
    if from.row == to.row {
        let start_col: usize = from.col.min(to.col);
        let end_col: usize = from.col.max(to.col);
        #[allow(clippy::needless_range_loop)]
        for col in (start_col + 1)..end_col {
            if board[from.row][col].is_some() {
                obstacle_count += 1;
            }
        }
    } else {
        let start_row: usize = from.row.min(to.row);
        let end_row: usize = from.row.max(to.row);
        #[allow(clippy::needless_range_loop)]
        for row in (start_row + 1)..end_row {
            if board[row][from.col].is_some() {
                obstacle_count += 1;
            }
        }
    }

    if board[to.row][to.col].is_some() {
        if obstacle_count != 1 {
            return Err(anyhow!("炮吃子必须隔一个棋子"));
        }
    } else if obstacle_count != 0 {
        return Err(anyhow!("炮移动路径不能有阻挡"));
    }

    Ok(())
}

fn check_pawn_rules(color: PlayerColor, from: Position, to: Position) -> Result<()> {
    match color {
        PlayerColor::Red => {
            if to.row < from.row {
                return Err(anyhow!("兵不能后退"));
            }
            if from.row <= 4 && from.row == to.row {
                return Err(anyhow!("兵过河前只能向前"));
            }
            if (to.row as isize - from.row as isize) + (to.col as isize - from.col as isize).abs()
                != 1
            {
                return Err(anyhow!("兵每次只能移动一步"));
            }
        }
        PlayerColor::Black => {
            if to.row > from.row {
                return Err(anyhow!("卒不能后退"));
            }
            if from.row >= 5 && from.row == to.row {
                return Err(anyhow!("卒过河前只能向前"));
            }
            if (from.row as isize - to.row as isize) + (to.col as isize - from.col as isize).abs()
                != 1
            {
                return Err(anyhow!("卒每次只能移动一步"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::fen::FenProcessor;

    fn make_board(fen: &str) -> Board {
        FenProcessor::parse_fen(fen).unwrap().board
    }

    fn pos(col: usize, row: usize) -> Position {
        Position { row, col }
    }

    #[test]
    fn general_valid_moves() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/4K4 w");
        assert!(check_piece_rules(&board, pos(4, 0), pos(4, 1)).is_ok());
        assert!(check_piece_rules(&board, pos(4, 0), pos(5, 0)).is_ok());
        assert!(check_piece_rules(&board, pos(4, 0), pos(3, 0)).is_ok());
    }

    #[test]
    fn general_cannot_leave_palace() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/4K4 w");
        assert!(check_piece_rules(&board, pos(4, 0), pos(4, 3)).is_err());
        assert!(check_piece_rules(&board, pos(4, 0), pos(2, 0)).is_err());
        assert!(check_piece_rules(&board, pos(4, 0), pos(6, 0)).is_err());
    }

    #[test]
    fn general_cannot_move_diagonally() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/4K4 w");
        assert!(check_piece_rules(&board, pos(4, 0), pos(5, 1)).is_err());
    }

    #[test]
    fn advisor_valid_diagonal() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/3AK4 w");
        assert!(check_piece_rules(&board, pos(3, 0), pos(4, 1)).is_ok());
    }

    #[test]
    fn advisor_cannot_move_straight() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/3AK4 w");
        assert!(check_piece_rules(&board, pos(3, 0), pos(3, 1)).is_err());
    }

    #[test]
    fn elephant_valid_moves() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/2B1K4 w");
        assert!(check_piece_rules(&board, pos(2, 0), pos(0, 2)).is_ok());
        assert!(check_piece_rules(&board, pos(2, 0), pos(4, 2)).is_ok());
    }

    #[test]
    fn elephant_blocked_by_eye() {
        let board = make_board("4k4/9/9/9/9/9/9/9/1P7/2B1K4 w");
        assert!(check_piece_rules(&board, pos(2, 0), pos(0, 2)).is_err());
    }

    #[test]
    fn elephant_cannot_cross_river() {
        let board = make_board("4k4/9/9/9/9/9/2B6/9/9/4K4 w");
        assert!(check_piece_rules(&board, pos(2, 4), pos(0, 6)).is_err());
    }

    #[test]
    fn horse_valid_moves() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/1N2K4 w");
        assert!(check_piece_rules(&board, pos(1, 0), pos(0, 2)).is_ok());
        assert!(check_piece_rules(&board, pos(1, 0), pos(2, 2)).is_ok());
        assert!(check_piece_rules(&board, pos(1, 0), pos(3, 1)).is_ok());
    }

    #[test]
    fn horse_blocked_by_leg() {
        let board = make_board("4k4/9/9/9/9/9/9/9/1P7/1N2K4 w");
        assert!(check_piece_rules(&board, pos(1, 0), pos(0, 2)).is_err());
        assert!(check_piece_rules(&board, pos(1, 0), pos(2, 2)).is_err());
    }

    #[test]
    fn rook_valid_moves() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/R3K4 w");
        assert!(check_piece_rules(&board, pos(0, 0), pos(0, 5)).is_ok());
        assert!(check_piece_rules(&board, pos(0, 0), pos(3, 0)).is_ok());
    }

    #[test]
    fn rook_blocked_path() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/RP2K4 w");
        assert!(check_piece_rules(&board, pos(0, 0), pos(3, 0)).is_err());
    }

    #[test]
    fn cannon_valid_move() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/C3K4 w");
        assert!(check_piece_rules(&board, pos(0, 0), pos(0, 5)).is_ok());
        assert!(check_piece_rules(&board, pos(0, 0), pos(3, 0)).is_ok());
    }

    #[test]
    fn cannon_capture_over_one() {
        let board = make_board("r8/9/9/9/9/9/9/9/P8/C3K4 w");
        assert!(check_piece_rules(&board, pos(0, 0), pos(0, 9)).is_ok());
    }

    #[test]
    fn cannon_cannot_capture_without_mount() {
        let board = make_board("4k4/9/9/9/9/9/9/9/r8/C3K4 w");
        assert!(check_piece_rules(&board, pos(0, 0), pos(0, 1)).is_err());
    }

    #[test]
    fn pawn_forward_before_crossing() {
        let board = make_board("4k4/9/9/9/9/9/P8/9/9/4K4 w");
        assert!(check_piece_rules(&board, pos(0, 3), pos(0, 4)).is_ok());
        assert!(check_piece_rules(&board, pos(0, 3), pos(1, 3)).is_err());
    }

    #[test]
    fn pawn_sideways_after_crossing() {
        let board = make_board("4k4/9/9/9/P8/9/9/9/9/4K4 w");
        assert!(check_piece_rules(&board, pos(0, 5), pos(0, 6)).is_ok());
        assert!(check_piece_rules(&board, pos(0, 5), pos(1, 5)).is_ok());
    }

    #[test]
    fn pawn_cannot_retreat() {
        let board = make_board("4k4/9/9/9/P8/9/9/9/9/4K4 w");
        assert!(check_piece_rules(&board, pos(0, 5), pos(0, 4)).is_err());
    }

    #[test]
    fn cannot_capture_own_piece() {
        let board = make_board("4k4/9/9/9/9/9/9/9/9/RR2K4 w");
        assert!(check_piece_rules(&board, pos(0, 0), pos(1, 0)).is_err());
    }
}
