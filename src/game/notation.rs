use crate::game::types::{Piece, PieceKind, PlayerColor, Position};
use crate::utils::*;

const ZH_LIST: [&str; 9] = ["九", "八", "七", "六", "五", "四", "三", "二", "一"];
const DIG_LIST: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

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

pub fn move_to_chinese(
    board: &[[Option<Piece>; 9]; 10],
    current_player: PlayerColor,
    from: Position,
    to: Position,
) -> Result<String> {
    let piece: Piece = board[from.row][from.col].ok_or_else(|| anyhow!("起始位置没有棋子"))?;

    let piece_name: &'static str = piece.get_chinese_name();

    let mut same_piece_idxs: Vec<usize> = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for row in 0..10 {
        if let Some(other_piece) = board[row][from.col]
            && other_piece.color == piece.color
            && other_piece.kind == piece.kind
        {
            same_piece_idxs.push(row);
        }
    }

    let part1: String = if same_piece_idxs.len() == 1 {
        let from_col_name: &str = match current_player {
            PlayerColor::Red => ZH_LIST[from.col],
            PlayerColor::Black => DIG_LIST[from.col],
        };
        format!("{}{}", piece_name, from_col_name)
    } else {
        let idx: usize = same_piece_idxs
            .iter()
            .position(|&r| r == from.row)
            .expect("from.row must be in same_piece_idxs - this is a bug");
        let pos_type: &str = match current_player {
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

    if from.row == to.row {
        move_type = "平";
        move_detail = match current_player {
            PlayerColor::Red => ZH_LIST[to.col],
            PlayerColor::Black => DIG_LIST[to.col],
        };
    } else {
        move_type = match current_player {
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

        if from.col == to.col {
            let diff: usize = (from.row as isize - to.row as isize).unsigned_abs();
            move_detail = match current_player {
                PlayerColor::Red => ZH_LIST[9 - diff],
                PlayerColor::Black => DIG_LIST[diff - 1],
            };
        } else {
            move_detail = match current_player {
                PlayerColor::Red => ZH_LIST[to.col],
                PlayerColor::Black => DIG_LIST[to.col],
            };
        }
    }

    Ok(format!("{}{}{}", part1, move_type, move_detail))
}
