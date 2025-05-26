use serde::{Deserialize, Serialize};

use super::piece::{Color, Move, Piece, PieceType, Position};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    squares: [[Option<Piece>; 8]; 8],
    to_move: Color,
    castling_rights: CastlingRights,
    en_passant_target: Option<Position>,
    halfmove_clock: u32,
    fullmove_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

impl Default for CastlingRights {
    fn default() -> Self {
        Self {
            white_kingside: true,
            white_queenside: true,
            black_kingside: true,
            black_queenside: true,
        }
    }
}

impl Board {
    pub fn new() -> Self {
        let mut board = Self {
            squares: [[None; 8]; 8],
            to_move: Color::White,
            castling_rights: CastlingRights::default(),
            en_passant_target: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        };
        board.setup_starting_position();
        board
    }

    pub fn empty() -> Self {
        Self {
            squares: [[None; 8]; 8],
            to_move: Color::White,
            castling_rights: CastlingRights {
                white_kingside: false,
                white_queenside: false,
                black_kingside: false,
                black_queenside: false,
            },
            en_passant_target: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        }
    }

    fn setup_starting_position(&mut self) {
        self.place_piece(Position::new(0, 0).unwrap(), Piece::new(PieceType::Rook, Color::White));
        self.place_piece(Position::new(1, 0).unwrap(), Piece::new(PieceType::Knight, Color::White));
        self.place_piece(Position::new(2, 0).unwrap(), Piece::new(PieceType::Bishop, Color::White));
        self.place_piece(Position::new(3, 0).unwrap(), Piece::new(PieceType::Queen, Color::White));
        self.place_piece(Position::new(4, 0).unwrap(), Piece::new(PieceType::King, Color::White));
        self.place_piece(Position::new(5, 0).unwrap(), Piece::new(PieceType::Bishop, Color::White));
        self.place_piece(Position::new(6, 0).unwrap(), Piece::new(PieceType::Knight, Color::White));
        self.place_piece(Position::new(7, 0).unwrap(), Piece::new(PieceType::Rook, Color::White));
        
        for file in 0..8 {
            self.place_piece(Position::new(file, 1).unwrap(), Piece::new(PieceType::Pawn, Color::White));
        }

        self.place_piece(Position::new(0, 7).unwrap(), Piece::new(PieceType::Rook, Color::Black));
        self.place_piece(Position::new(1, 7).unwrap(), Piece::new(PieceType::Knight, Color::Black));
        self.place_piece(Position::new(2, 7).unwrap(), Piece::new(PieceType::Bishop, Color::Black));
        self.place_piece(Position::new(3, 7).unwrap(), Piece::new(PieceType::Queen, Color::Black));
        self.place_piece(Position::new(4, 7).unwrap(), Piece::new(PieceType::King, Color::Black));
        self.place_piece(Position::new(5, 7).unwrap(), Piece::new(PieceType::Bishop, Color::Black));
        self.place_piece(Position::new(6, 7).unwrap(), Piece::new(PieceType::Knight, Color::Black));
        self.place_piece(Position::new(7, 7).unwrap(), Piece::new(PieceType::Rook, Color::Black));
        
        for file in 0..8 {
            self.place_piece(Position::new(file, 6).unwrap(), Piece::new(PieceType::Pawn, Color::Black));
        }
    }

    pub fn get_piece(&self, pos: Position) -> Option<Piece> {
        match pos.is_valid() {
            true => self.squares[pos.rank as usize][pos.file as usize],
            false => None,
        }
    }

    pub fn place_piece(&mut self, pos: Position, piece: Piece) {
        if pos.is_valid() {
            self.squares[pos.rank as usize][pos.file as usize] = Some(piece);
        }
    }

    pub fn remove_piece(&mut self, pos: Position) -> Option<Piece> {
        match pos.is_valid() {
            true => {
                let piece = self.squares[pos.rank as usize][pos.file as usize];
                self.squares[pos.rank as usize][pos.file as usize] = None;
                piece
            }
            false => None,
        }
    }

    pub fn is_empty(&self, pos: Position) -> bool {
        self.get_piece(pos).is_none()
    }

    pub fn is_occupied_by(&self, pos: Position, color: Color) -> bool {
        match self.get_piece(pos) {
            Some(piece) => piece.color == color,
            None => false,
        }
    }

    pub fn get_to_move(&self) -> Color {
        self.to_move
    }

    pub fn set_to_move(&mut self, color: Color) {
        self.to_move = color;
    }

    pub fn get_castling_rights(&self) -> &CastlingRights {
        &self.castling_rights
    }

    pub fn get_en_passant_target(&self) -> Option<Position> {
        self.en_passant_target
    }

    pub fn get_halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }

    pub fn get_fullmove_number(&self) -> u32 {
        self.fullmove_number
    }

    pub fn find_king(&self, color: Color) -> Option<Position> {
        for rank in 0..8 {
            for file in 0..8 {
                let pos = Position::new(file, rank).unwrap();
                if let Some(piece) = self.get_piece(pos) {
                    if piece.piece_type == PieceType::King && piece.color == color {
                        return Some(pos);
                    }
                }
            }
        }
        None
    }

    pub fn is_path_clear(&self, from: Position, to: Position) -> bool {
        let file_diff = to.file as i8 - from.file as i8;
        let rank_diff = to.rank as i8 - from.rank as i8;

        // Directly adjacent squares are always clear
        if file_diff.abs() <= 1 && rank_diff.abs() <= 1 {
            return true;
        }

        let file_step = file_diff.signum();
        let rank_step = rank_diff.signum();

        let mut curr_file = from.file as i8 + file_step;
        let mut curr_rank = from.rank as i8 + rank_step;

        while curr_file != to.file as i8 || curr_rank != to.rank as i8 {
            let pos = Position::new(curr_file as u8, curr_rank as u8);
            if let Some(pos) = pos {
                if !self.is_empty(pos) {
                    return false; 
                }
            }
            curr_file += file_step;
            curr_rank += rank_step;
        }
        true
    }

    pub fn make_move(&mut self, chess_move: &Move) -> Result<(), String> {
        let piece = self.get_piece(chess_move.from)
            .ok_or("No piece at source position")?;

        if piece.color != self.to_move {
            return Err("Not your turn".to_string());
        }

        let mut moved_piece = piece;
        moved_piece.mark_moved();

        if chess_move.is_castle {
            self.execute_castle(chess_move)?;
        } else if chess_move.is_en_passant {
            self.execute_en_passant(chess_move)?;
        } else {
            self.remove_piece(chess_move.from);

            if let Some(promotion_type) = chess_move.promotion {
                moved_piece.piece_type = promotion_type;
            }

            self.place_piece(chess_move.to, moved_piece);
        }

        self.update_en_passant_target(chess_move, &piece);
        self.update_castling_rights(chess_move, &piece);

        match piece.piece_type == PieceType::Pawn || !self.is_empty(chess_move.to) {
            true => self.halfmove_clock = 0,
            false => self.halfmove_clock += 1,
        };

        if self.to_move == Color::White {
            self.fullmove_number += 1;
        }

        self.to_move = self.to_move.opposite();

        Ok(())
    }

    fn execute_castle(&mut self, chess_move: &Move) -> Result<(), String> {
        let king = self.remove_piece(chess_move.from)
            .ok_or("No king at source position")?;

        self.place_piece(chess_move.to, king);

        let (rook_from, rook_to) = if chess_move.to.file > chess_move.from.file {
            // Kingside castle
            (Position::new(7, chess_move.from.rank).unwrap(),
            Position::new(5, chess_move.from.rank).unwrap())
        } else {
            // Queenside castle
            (Position::new(0, chess_move.from.rank).unwrap(),
            Position::new(3, chess_move.from.rank).unwrap())
        };

        let mut rook = self.remove_piece(rook_from)
            .ok_or("No rook for castling")?;
        rook.mark_moved();
        self.place_piece(rook_to, rook);

        Ok(())
    }

    fn execute_en_passant(&mut self, chess_move: &Move) -> Result<(), String> {
        let pawn = self.remove_piece(chess_move.from)
            .ok_or("No pawn for en passant")?;

        self.place_piece(chess_move.to, pawn);

        let captured_pawn_pos = Position::new(chess_move.to.file, chess_move.from.rank).unwrap();
        self.remove_piece(captured_pawn_pos);

        Ok(())
    }

    fn update_en_passant_target(&mut self, chess_move: &Move, piece: &Piece) {
        self.en_passant_target = None;

        if piece.piece_type == PieceType::Pawn {
            let rank_diff = (chess_move.to.rank as i8 - chess_move.from.rank as i8).abs();
            if rank_diff == 2 {
                let target_rank = (chess_move.from.rank + chess_move.to.rank) / 2;
                self.en_passant_target = Position::new(chess_move.from.file, target_rank);
            }
        }
    }

    fn update_castling_rights(&mut self, chess_move: &Move, piece: &Piece) {
        match piece.piece_type {
            PieceType::King => {
                match piece.color {
                    Color::White => {
                        self.castling_rights.white_kingside = false;
                        self.castling_rights.white_queenside = false;
                    }
                    Color::Black => {
                        self.castling_rights.black_kingside = false;
                        self.castling_rights.black_queenside = false;
                    }
                }
            }
            PieceType::Rook => {
                match (piece.color, chess_move.from.file, chess_move.from.rank) {
                    (Color::White, 0, 0) => self.castling_rights.white_queenside = false,
                    (Color::White, 7, 0) => self.castling_rights.white_kingside = false,
                    (Color::Black, 0, 7) => self.castling_rights.black_queenside = false,
                    (Color::Black, 7, 7) => self.castling_rights.black_kingside = false,
                    _ => {}
                }
            }
            _ => {}
        }

        match (chess_move.to.file, chess_move.to.rank) {
            (0, 0) => self.castling_rights.white_queenside = false,
            (7, 0) => self.castling_rights.white_kingside = false,
            (0, 7) => self.castling_rights.black_queenside = false,
            (7, 7) => self.castling_rights.black_kingside = false,
            _ => {}
        }
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        for rank in (0..8).rev() {
            let mut empty_count = 0;

            for file in 0..8 {
                let pos = Position::new(file, rank).unwrap();
                match self.get_piece(pos) {
                    Some(piece) => {
                        if empty_count > 0 {
                            fen.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        fen.push(piece.to_fen_char());
                    }
                    None => empty_count += 1,
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        fen.push(' ');
        fen.push(match self.to_move {
            Color::White => 'w',
            Color::Black => 'b',
        });

        fen.push(' ');
        let mut castling = String::new();
        if self.castling_rights.white_kingside { castling.push('K'); }
        if self.castling_rights.white_queenside { castling.push('Q'); }
        if self.castling_rights.black_kingside { castling.push('k'); }
        if self.castling_rights.black_queenside { castling.push('q'); }
        if castling.is_empty() { castling.push('-'); }
        fen.push_str(&castling);

        fen.push(' ');
        match self.en_passant_target {
            Some(pos) => fen.push_str(&pos.to_algebraic()),
            None => fen.push('-'),
        }

        fen.push_str(&format!(" {} {}", self.halfmove_clock, self.fullmove_number));

        fen
    }

    pub fn display(&self) -> String {
        let mut display = String::new();

        for rank in (0..8).rev() {
            display.push_str(&format!("{} ", rank + 1));
            for file in 0..8 {
                let pos = Position::new(file, rank).unwrap();
                match self.get_piece(pos) {
                    Some(piece) => display.push(piece.to_fen_char()),
                    None => display.push('.'),
                }
                display.push(' ');
            }
            display.push('\n');
        }
        display.push_str("  a b c d e f g h\n");
        
        display
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starting_position() {
        let board = Board::new();
        
        let king_pos = Position::from_algebraic("e1").unwrap();
        let king = board.get_piece(king_pos).unwrap();
        assert_eq!(king.piece_type, PieceType::King);
        assert_eq!(king.color, Color::White);
        
        let pawn_pos = Position::from_algebraic("e2").unwrap();
        let pawn = board.get_piece(pawn_pos).unwrap();
        assert_eq!(pawn.piece_type, PieceType::Pawn);
        assert_eq!(pawn.color, Color::White);
    }

    #[test]
    fn test_fen_generation() {
        let board = Board::new();
        let fen = board.to_fen();
        assert!(fen.starts_with("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -"));
    }

    #[test]
    fn test_path_clear() {
        let board = Board::new();

        let rook_pos = Position::from_algebraic("a1").unwrap();
        let target_pos = Position::from_algebraic("a3").unwrap();
        assert!(!board.is_path_clear(rook_pos, target_pos));

        let empty_board = Board::empty();
        assert!(empty_board.is_path_clear(rook_pos, target_pos));
    }
}