use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PieceType {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
    pub has_moved: bool,
}

impl Piece {
    pub fn new(piece_type: PieceType, color: Color) -> Self {
        Self {
            piece_type,
            color,
            has_moved: false,
        }
    }

    pub fn mark_moved(&mut self) {
        self.has_moved = true;
    }

    pub fn get_value(&self) -> u32 {
        match self.piece_type {
            PieceType::Pawn => 1,
            PieceType::Rook => 5,
            PieceType::Knight => 3,
            PieceType::Bishop => 3,
            PieceType::Queen => 9,
            PieceType::King => 0,
        }
    }

    pub fn to_fen_char(&self) -> char {
        let base_char = match self.piece_type {
            PieceType::Pawn => 'p',
            PieceType::Rook => 'r',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
        };

        match self.color {
            Color::White => base_char.to_ascii_uppercase(),
            Color::Black => base_char,
        }
    }

    pub fn from_fen_char(c: char) -> Option<Self> {
        let color = match c.is_uppercase() {
            true => Color::White,
            false => Color::Black,
        };

        let piece_type = match c.to_ascii_lowercase() {
            'p' => PieceType::Pawn,
            'r' => PieceType::Rook,
            'n' => PieceType::Knight,
            'b' => PieceType::Bishop,
            'q' => PieceType::Queen,
            'k' => PieceType::King,
            _ => return None,
        };

        Some(Self::new(piece_type, color))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub file: u8, // 0-7 (a-h)
    pub rank: u8, // 0-7 (1-8)
}

impl Position {
    pub fn new(file: u8, rank: u8) -> Option<Self> {
        match (file < 8, rank < 8) {
            (true, true) => Some(Self { file, rank }),
            _ => None,
        }
    }

    pub fn from_algebraic(notation: &str) -> Option<Self> {
        let chars: Vec<char> = notation.chars().collect();
        if chars.len() != 2 {
            return None;
        }

        let file = match chars[0] {
            'a'..='h' => chars[0] as u8 - b'a',
            _ => return None,
        };

        let rank = match chars[1] {
            '1'..='8' => chars[1] as u8 - b'1',
            _ => return None,
        };

        Some(Self { file, rank })
    }

    pub fn to_algebraic(&self) -> String {
        format!(
            "{}{}",
            (b'a' + self.file) as char,
            (b'1' + self.rank) as char
        )
    }

    pub fn is_valid(&self) -> bool {
        self.file < 8 && self.rank < 8
    }

    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = (self.file as i8 - other.file as i8).abs() as f64;
        let dy = (self.rank as i8 - other.rank as i8).abs() as f64;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    pub from: Position,
    pub to: Position,
    pub promotion: Option<PieceType>,
    pub is_castle: bool,
    pub is_en_passant: bool,
}

impl Move {
    pub fn new(from: Position, to: Position) -> Self {
        Self {
            from,
            to,
            promotion: None,
            is_castle: false,
            is_en_passant: false,
        }
    }

    pub fn with_promotion(from: Position, to: Position, promotion: PieceType) -> Self {
        Self {
            from,
            to,
            promotion: Some(promotion),
            is_castle: false,
            is_en_passant: false,
        }
    }

    pub fn castle(from: Position, to: Position) -> Self {
        Self {
            from,
            to,
            promotion: None,
            is_castle: true,
            is_en_passant: false,
        }
    }

    pub fn en_passant(from: Position, to: Position) -> Self {
        Self {
            from,
            to,
            promotion: None,
            is_castle: false,
            is_en_passant: true,
        }
    }

    pub fn to_algebraic(&self) -> String {
        let mut result = format!("{}{}", self.from.to_algebraic(), self.to.to_algebraic());

        if let Some(promotion) = self.promotion {
            let promotion_char = match promotion {
                PieceType::Queen => 'q',
                PieceType::Rook => 'r',
                PieceType::Bishop => 'b',
                PieceType::Knight => 'n',
                _ => 'q',
            };
            result.push(promotion_char);
        }

        result
    }

    pub fn from_algebraic(notation: &str) -> Option<Self> {
        if notation.len() < 4 {
            return None;
        }

        let from = Position::from_algebraic(&notation[0..2])?;
        let to = Position::from_algebraic(&notation[2..4])?;

        let mut chess_move = Self::new(from, to);

        if notation.len() == 5 {
            let promotion_char = notation.chars().nth(4)?;
            chess_move.promotion = match promotion_char {
                'q' => Some(PieceType::Queen),
                'r' => Some(PieceType::Rook),
                'b' => Some(PieceType::Bishop),
                'n' => Some(PieceType::Knight),
                _ => None,
            };
        }

        Some(chess_move)
    }
}