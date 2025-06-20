use crate::utils::*;
use crate::game::{GameState, PlayerColor, Piece, PieceKind};

/// 处理FEN字符串的解析和生成
pub struct FenProcessor;

impl FenProcessor {
    /// 解析FEN字符串，创建游戏状态
    pub fn parse_fen(fen: &str) -> Result<GameState> {
        log_info!(fen);
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow!("FEN字符串至少包括两部分: 棋盘状态和当前玩家"));
        }
        
        let board_str: &str = parts[0];
        let mut board: [[Option<Piece>; 9]; 10] = [[None; 9]; 10];
        
        // 按行分割并反转顺序
        let mut rows: Vec<&str> = board_str.split('/').collect();
        rows.reverse();
        if rows.len() != 10 {
            return Err(anyhow!("FEN必须有10行"));
        }
        
        for (y, row) in rows.iter().enumerate() {
            let mut x: usize = 0;
            for c in row.chars() {
                // 数字表示空格子数量
                if let Some(digit) = c.to_digit(10) {
                    x += digit as usize;
                } 
                // 否则是棋子字符
                else {
                    let piece: Piece = Self::char_to_piece(c)?;
                    board[y][x] = Some(piece);
                    x += 1;
                }
            }
            
            if x != 9 {
                return Err(anyhow!("一行不足9个格子"));
            }
        }
        
        // 解析当前玩家
        let current_player: PlayerColor = match parts[1] {
            "w" => PlayerColor::Red,
            "b" => PlayerColor::Black,
            _ => return Err(anyhow!("当前玩家必须是 'w' 或 'b'")),
        };
        
        Ok(GameState {
            board,
            current_player,
            history: Vec::new(),
            flipped: false,
        })
    }
    
    /// 将字符转换为棋子
    fn char_to_piece(c: char) -> Result<Piece> {
        let (color, kind) = match c {
            'K' => (PlayerColor::Red, PieceKind::General),
            'A' => (PlayerColor::Red, PieceKind::Advisor),
            'B' => (PlayerColor::Red, PieceKind::Elephant),
            'N' => (PlayerColor::Red, PieceKind::Horse),
            'R' => (PlayerColor::Red, PieceKind::Rook),
            'C' => (PlayerColor::Red, PieceKind::Cannon),
            'P' => (PlayerColor::Red, PieceKind::Pawn),
            'k' => (PlayerColor::Black, PieceKind::General),
            'a' => (PlayerColor::Black, PieceKind::Advisor),
            'b' => (PlayerColor::Black, PieceKind::Elephant),
            'n' => (PlayerColor::Black, PieceKind::Horse),
            'r' => (PlayerColor::Black, PieceKind::Rook),
            'c' => (PlayerColor::Black, PieceKind::Cannon),
            'p' => (PlayerColor::Black, PieceKind::Pawn),
            _ => return Err(anyhow!("无效的棋子字符: {}", c)),
        };
        Ok(Piece { color, kind })
    }
    
    /// 从游戏状态生成FEN字符串
    pub fn generate_fen(state: &GameState) -> String {
        let mut fen: String = String::new();
        
        // 反转行顺序：从黑方顶部（第9行）到红方底部（第0行）
        for y in (0..10).rev() {
            let mut empty: usize = 0;
            for piece in &state.board[y] {
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
            
            // 行之间用斜杠分隔（除了最后一行）
            if y > 0 {
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
            (PlayerColor::Red, PieceKind::Elephant) => 'B',
            (PlayerColor::Red, PieceKind::Horse) => 'N',
            (PlayerColor::Red, PieceKind::Rook) => 'R',
            (PlayerColor::Red, PieceKind::Cannon) => 'C',
            (PlayerColor::Red, PieceKind::Pawn) => 'P',
            (PlayerColor::Black, PieceKind::General) => 'k',
            (PlayerColor::Black, PieceKind::Advisor) => 'a',
            (PlayerColor::Black, PieceKind::Elephant) => 'b',
            (PlayerColor::Black, PieceKind::Horse) => 'n',
            (PlayerColor::Black, PieceKind::Rook) => 'r',
            (PlayerColor::Black, PieceKind::Cannon) => 'c',
            (PlayerColor::Black, PieceKind::Pawn) => 'p',
        }
    }
}
