use crate::utils::*;
use super::state::{GameState, PlayerColor, Piece, PieceKind};

/// 处理FEN字符串的解析和生成
pub struct FenProcessor;

impl FenProcessor {
    /// 解析FEN字符串，创建游戏状态
    pub fn parse_fen(fen: &str) -> Result<GameState> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 1 {
            return Err(anyhow!("FEN字符串不完整"));
        }
        
        let board_str = parts[0];
        let mut board = [[None; 9]; 10];
        
        // 按行分割
        let rows: Vec<&str> = board_str.split('/').collect();
        if rows.len() != 10 {
            return Err(anyhow!("FEN必须有10行"));
        }
        
        for (y, row) in rows.iter().enumerate() {
            let mut x = 0;
            for c in row.chars() {
                if x >= 9 {
                    return Err(anyhow!("一行超过9个格子"));
                }
                
                if let Some(digit) = c.to_digit(10) {
                    // 跳过空位
                    x += digit as usize;
                } else {
                    let piece = Self::char_to_piece(c)?;
                    board[y][x] = Some(piece);
                    x += 1;
                }
            }
            
            if x != 9 {
                return Err(anyhow!("一行不足9个格子"));
            }
        }
        
        // 解析当前玩家（默认为红方）
        let current_player = if parts.len() >= 2 {
            match parts[1] {
                "w" => PlayerColor::Red,
                "b" => PlayerColor::Black,
                _ => PlayerColor::Red,
            }
        } else {
            PlayerColor::Red
        };
        
        Ok(GameState {
            board,
            current_player,
            history: Vec::new(),
        })
    }
    
    /// 将字符转换为棋子
    fn char_to_piece(c: char) -> Result<Piece> {
        let (color, kind) = match c {
            'K' => (PlayerColor::Red, PieceKind::General),
            'A' => (PlayerColor::Red, PieceKind::Advisor),
            'E' => (PlayerColor::Red, PieceKind::Elephant),
            'H' => (PlayerColor::Red, PieceKind::Horse),
            'R' => (PlayerColor::Red, PieceKind::Rook),
            'C' => (PlayerColor::Red, PieceKind::Cannon),
            'P' => (PlayerColor::Red, PieceKind::Pawn),
            'k' => (PlayerColor::Black, PieceKind::General),
            'a' => (PlayerColor::Black, PieceKind::Advisor),
            'e' => (PlayerColor::Black, PieceKind::Elephant),
            'h' => (PlayerColor::Black, PieceKind::Horse),
            'r' => (PlayerColor::Black, PieceKind::Rook),
            'c' => (PlayerColor::Black, PieceKind::Cannon),
            'p' => (PlayerColor::Black, PieceKind::Pawn),
            _ => return Err(anyhow!("无效的棋子字符: {}", c)),
        };
        Ok(Piece { color, kind })
    }
    
    /// 从游戏状态生成FEN字符串
    pub fn generate_fen(state: &GameState) -> String {
        let mut fen = String::new();
        
        for (y, row) in state.board.iter().enumerate() {
            let mut empty = 0;
            for piece in row {
                if let Some(p) = piece {
                    // 如果有空位，先输出空位数字
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                        empty = 0;
                    }
                    fen.push(Self::piece_to_char(*p));
                } else {
                    empty += 1;
                }
            }
            
            // 行末的空位
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            
            // 行之间用斜杠分隔
            if y < 9 {
                fen.push('/');
            }
        }
        
        // 添加当前玩家
        fen.push(' ');
        match state.current_player {
            PlayerColor::Red => fen.push('w'),
            PlayerColor::Black => fen.push('b'),
        }
        
        fen
    }
    
    /// 将棋子转换为字符
    fn piece_to_char(piece: Piece) -> char {
        match (piece.color, piece.kind) {
            (PlayerColor::Red, PieceKind::General) => 'K',
            (PlayerColor::Red, PieceKind::Advisor) => 'A',
            (PlayerColor::Red, PieceKind::Elephant) => 'E',
            (PlayerColor::Red, PieceKind::Horse) => 'H',
            (PlayerColor::Red, PieceKind::Rook) => 'R',
            (PlayerColor::Red, PieceKind::Cannon) => 'C',
            (PlayerColor::Red, PieceKind::Pawn) => 'P',
            (PlayerColor::Black, PieceKind::General) => 'k',
            (PlayerColor::Black, PieceKind::Advisor) => 'a',
            (PlayerColor::Black, PieceKind::Elephant) => 'e',
            (PlayerColor::Black, PieceKind::Horse) => 'h',
            (PlayerColor::Black, PieceKind::Rook) => 'r',
            (PlayerColor::Black, PieceKind::Cannon) => 'c',
            (PlayerColor::Black, PieceKind::Pawn) => 'p',
        }
    }
}
