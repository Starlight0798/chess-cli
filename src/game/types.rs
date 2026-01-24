use crate::utils::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerColor {
    Red,
    Black,
}

impl PlayerColor {
    pub fn opponent(&self) -> Self {
        match self {
            PlayerColor::Red => PlayerColor::Black,
            PlayerColor::Black => PlayerColor::Red,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    General,
    Advisor,
    Elephant,
    Horse,
    Rook,
    Cannon,
    Pawn,
}

#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub color: PlayerColor,
    pub kind: PieceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

pub type Board = [[Option<Piece>; 9]; 10];

pub fn parse_move(move_str: &str) -> Result<(Position, Position)> {
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

pub const COL_CHARS: [char; 9] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];

pub fn format_move(from: Position, to: Position) -> String {
    format!(
        "{}{}{}{}",
        COL_CHARS[from.col], from.row, COL_CHARS[to.col], to.row
    )
}
