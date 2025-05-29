use super::{Board, Color, Move, Piece, PieceType, Position};

pub struct MoveValidator;

impl MoveValidator {
    pub fn is_valid_move(board: &Board, chess_move: &Move) -> bool {
        // Check if the move is within the bounds of the board
        if !chess_move.from.is_valid() || !chess_move.to.is_valid() {
            return false;
        }

        if chess_move.from == chess_move.to {
            return false;
        }

        let piece = match board.get_piece(chess_move.from) {
            Some(p) => p,
            None => return false,
        };

        // Check if the piece belongs to the player making the move
        if piece.color != board.get_to_move() {
            return false;
        }

        // Check if the destination square is occupied by a piece of the same color
        if board.is_occupied_by(chess_move.to, piece.color) {
            return false;
        }

        // Check if the move is valid for the piece type
        if !Self::is_piece_move_valid(board, chess_move, &piece) {
            return false;
        }

        // Check if the move puts the king in check
        if Self::would_be_in_check_after_move(board, chess_move) {
            return false;
        }

        true
    }

    fn is_piece_move_valid(board: &Board, chess_move: &Move, piece: &Piece) -> bool {
        match piece.piece_type {
            PieceType::Pawn => Self::is_valid_pawn_move(board, chess_move, piece),
            PieceType::Rook => Self::is_valid_rook_move(board, chess_move),
            PieceType::Knight => Self::is_valid_knight_move(chess_move),
            PieceType::Bishop => Self::is_valid_bishop_move(board, chess_move),
            PieceType::Queen => Self::is_valid_queen_move(board, chess_move),
            PieceType::King => Self::is_valid_king_move(board, chess_move, piece),
        }
    }

    fn is_valid_pawn_move(board: &Board, chess_move: &Move, piece: &Piece) -> bool {
        let from = chess_move.from;
        let to = chess_move.to;
        let direction = match piece.color {
            Color::White => 1,
            Color::Black => -1,
        };

        let file_diff = to.file as i8 - from.file as i8;
        let rank_diff = to.rank as i8 - from.rank as i8;

        if chess_move.is_en_passant {
            return Self::is_valid_en_passant(board, chess_move, piece);
        }

        // Normal move
        if file_diff == 0 {
            if rank_diff == direction && board.is_empty(to) {
                return true;
            }

            // Double move
            if rank_diff == 2 * direction && !piece.has_moved && board.is_empty(to) {
                return true;
            }
        }
        // Capture move
        else if file_diff.abs() == 1 && rank_diff == direction {
            if board.is_occupied_by(to, piece.color.opposite()) {
                return true;
            }
        }

        false
    }

    fn is_valid_en_passant(board: &Board, chess_move: &Move, piece: &Piece) -> bool {
        if let Some(target) = board.get_en_passant_target() {
            if chess_move.to == target {
                let captured_pawn_pos = Position::new(target.file, chess_move.from.rank).unwrap();
                if let Some(captured_piece) = board.get_piece(captured_pawn_pos) {
                    return captured_piece.piece_type == PieceType::Pawn
                        && captured_piece.color != piece.color;
                }
            }
        }
        false
    }

    fn is_valid_rook_move(board: &Board, chess_move: &Move) -> bool {
        let from = chess_move.from;
        let to = chess_move.to;

        // Rook can move horizontally or vertically
        if from.file != to.file && from.rank != to.rank {
            return false;
        }

        board.is_path_clear(from, to)
    }

    fn is_valid_knight_move(chess_move: &Move) -> bool {
        let from = chess_move.from;
        let to = chess_move.to;

        let file_diff = (to.file as i8 - from.file as i8).abs();
        let rank_diff = (to.rank as i8 - from.rank as i8).abs();

        // Knight moves in an L-shape: two squares in one direction and one square in the other
        (file_diff == 2 && rank_diff == 1) || (file_diff == 1 && rank_diff == 2)
    }

    fn is_valid_bishop_move(board: &Board, chess_move: &Move) -> bool {
        let from = chess_move.from;
        let to = chess_move.to;

        let file_diff = (to.file as i8 - from.file as i8).abs();
        let rank_diff = (to.rank as i8 - from.rank as i8).abs();

        // Bishop can move diagonally
        if file_diff != rank_diff {
            return false;
        }

        board.is_path_clear(from, to)
    }

    fn is_valid_queen_move(board: &Board, chess_move: &Move) -> bool {
        // Queen can move like both a rook and a bishop
        Self::is_valid_rook_move(board, chess_move) || Self::is_valid_bishop_move(board, chess_move)
    }

    fn is_valid_king_move(board: &Board, chess_move: &Move, piece: &Piece) -> bool {
        let from = chess_move.from;
        let to = chess_move.to;

        if chess_move.is_castle {
            return Self::is_valid_castle(board, chess_move, piece);
        }

        let file_diff = (to.file as i8 - from.file as i8).abs();
        let rank_diff = (to.rank as i8 - from.rank as i8).abs();

        // King can move one square in any direction
        file_diff <= 1 && rank_diff <= 1
    }

    fn is_valid_castle(board: &Board, chess_move: &Move, piece: &Piece) -> bool {
        // Check if the king is moving
        if piece.has_moved {
            return false;
        }

        // Check if the king is not be in check
        if Self::is_in_check(board, piece.color) {
            return false;
        }

        let is_kingside = chess_move.to.file > chess_move.from.file;
        let castling_rights = board.get_castling_rights();

        let has_right = match (piece.color, is_kingside) {
            (Color::White, true) => castling_rights.white_kingside,
            (Color::White, false) => castling_rights.white_queenside,
            (Color::Black, true) => castling_rights.black_kingside,
            (Color::Black, false) => castling_rights.black_queenside,
        };

        if !has_right {
            return false;
        }

        let rook_file = if is_kingside { 7 } else { 0 };
        let rook_pos = Position::new(rook_file, chess_move.from.rank).unwrap();

        if let Some(rook) = board.get_piece(rook_pos) {
            if rook.piece_type != PieceType::Rook || rook.color != piece.color || rook.has_moved {
                return false;
            }
        } else {
            return false;
        }

        // Check if the squares between the king and rook are empty
        if !board.is_path_clear(chess_move.from, rook_pos) {
            return false;
        }

        // Check if the squares the king moves through are not attacked
        let king_path = if is_kingside {
            vec![
                Position::new(chess_move.from.file + 1, chess_move.from.rank).unwrap(),
                Position::new(chess_move.from.file + 2, chess_move.from.rank).unwrap(),
            ]
        } else {
            vec![
                Position::new(chess_move.from.file - 1, chess_move.from.rank).unwrap(),
                Position::new(chess_move.from.file - 2, chess_move.from.rank).unwrap(),
            ]
        };

        for pos in king_path {
            if Self::is_square_attacked(board, pos, piece.color.opposite()) {
                return false;
            }
        }

        true
    }

    pub fn is_in_check(board: &Board, color: Color) -> bool {
        if let Some(king_pos) = board.find_king(color) {
            Self::is_square_attacked(board, king_pos, color.opposite())
        } else {
            false
        }
    }

    pub fn is_square_attacked(board: &Board, pos: Position, by_color: Color) -> bool {
        for rank in 0..8 {
            for file in 0..8 {
                let attacker_pos = Position::new(file, rank).unwrap();
                if let Some(piece) = board.get_piece(attacker_pos) {
                    if piece.color == by_color {
                        let attack_move = Move::new(attacker_pos, pos);
                        if Self::can_piece_attack(board, &attack_move, &piece) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn can_piece_attack(board: &Board, chess_move: &Move, piece: &Piece) -> bool {
        match piece.piece_type {
            PieceType::Pawn => Self::can_pawn_attack(chess_move, piece),
            PieceType::Rook => Self::is_valid_rook_move(board, chess_move),
            PieceType::Knight => Self::is_valid_knight_move(chess_move),
            PieceType::Bishop => Self::is_valid_bishop_move(board, chess_move),
            PieceType::Queen => Self::is_valid_queen_move(board, chess_move),
            PieceType::King => {
                let file_diff = (chess_move.to.file as i8 - chess_move.from.file as i8).abs();
                let rank_diff = (chess_move.to.rank as i8 - chess_move.from.rank as i8).abs();
                file_diff <= 1 && rank_diff <= 1
            }
        }
    }

    fn can_pawn_attack(chess_move: &Move, piece: &Piece) -> bool {
        let from = chess_move.from;
        let to = chess_move.to;
        let direction = match piece.color {
            Color::White => 1,
            Color::Black => 01,
        };

        let file_diff = (to.file as i8 - from.file as i8).abs();
        let rank_diff = to.rank as i8 - from.rank as i8;

        // Pawn can attack diagonally
        file_diff == 1 && rank_diff == direction
    }

    fn would_be_in_check_after_move(board: &Board, chess_move: &Move) -> bool {
        let mut test_board = board.clone();
        if test_board.make_move(chess_move).is_ok() {
            Self::is_in_check(&test_board, board.get_to_move())
        } else {
            true
        }
    }

    pub fn generate_legal_moves(board: &Board) -> Vec<Move> {
        let mut moves = Vec::new();
        let current_color = board.get_to_move();

        for rank in 0..8 {
            for file in 0..8 {
                let from = Position::new(file, rank).unwrap();
                if let Some(piece) = board.get_piece(from) {
                    if piece.color == current_color {
                        let piece_moves = Self::generate_piece_moves(board, from, &piece);
                        for chess_move in piece_moves {
                            if Self::is_valid_move(board, &chess_move) {
                                moves.push(chess_move);
                            }
                        }
                    }
                }
            }
        }

        moves
    }

    fn generate_piece_moves(board: &Board, from: Position, piece: &Piece) {
        match piece.piece_type {
            PieceType::Pawn => Self::generate_pawn_moves(board, from, piece),
            PieceType::Rook => Self::generate_rook_moves(board, from),
            PieceType::Knight => Self::generate_knight_moves(from),
            PieceType::Bishop => Self::generate_bishop_moves(board, from),
            PieceType::Queen => Self::generate_queen_moves(board, from),
            PieceType::King => Self::generate_king_moves(board, from, piece),
        }
    }

    fn generate_pawn_moves(board: &Board, from: Position, piece: &Piece) -> Vec<Move> {
        let mut moves = Vec::new();
        let direction = match piece.color {
            Color::White => 1,
            Color::Black => -1,
        };

        // Normal move
        if let Some(to) = Position::new(from.file, (from.rank as i8 + direction) as u8) {
            if board.is_empty(to) {
                if (piece.color == Color::White && to.rank == 7) ||
                    (piece.color == Color::Black && to.rank == 0) {
                    moves.push(Move::with_promotion(from, to, PieceType::Queen));
                    moves.push(Move::with_promotion(from, to, PieceType::Rook));
                    moves.push(Move::with_promotion(from, to, PieceType::Bishop));
                    moves.push(Move::with_promotion(from, to, PieceType::Knight));
                } else {
                    moves.push(Move::new(from, to));
                }

                if !piece.has_moved {
                    if let Some(to2) = Position::new(from.file, (from.rank as i8 + 2 * direction) as u8) {
                        if board.is_empty(to2) {
                            moves.push(Move::new(from, to2));
                        }
                    }
                }
            }
        }

        // Attack
        for file_offset in [-1, 1] {
            if let Some(to) = Position::new((from.file as i8 + file_offset) as u8, (from.rank as i8 + direction) as u8) {
                if board.is_occupied_by(to, piece.color.opposite()) {
                    if (piece.color == Color::White && to.rank == 7) ||
                        (piece.color == Color::Black && to.rank == 0) {
                        moves.push(Move::with_promotion(from, to, PieceType::Queen));
                        moves.push(Move::with_promotion(from, to, PieceType::Rook));
                        moves.push(Move::with_promotion(from, to, PieceType::Bishop));
                        moves.push(Move::with_promotion(from, to, PieceType::Knight));
                    } else {
                        moves.push(Move::new(from, to));
                    }
                }

                if let Some(en_passant_target) = board.get_en_passant_target() {
                    if to == en_passant_target {
                        moves.push(Move::en_passant(from, to));
                    }
                }
            }
        }

        moves
    }

    fn generate_rook_moves(board: &Board, from: Position) -> Vec<Move> {
        let mut moves = Vec::new();
        let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];

        for (file_dir, rank_dir) in directions {
            for distance in 1..8 {
                let new_file = from.file as i8 + file_dir * distance;
                let new_rank = from.rank as i8 + rank_dir * distance;

                if let Some(to) = Position::new(new_file as u8, new_rank as u8) {
                    if board.is_empty(to) {
                        moves.push(Move::new(from, to));
                    } else {
                        if board.is_occupied_by(to, board.get_piece(from).unwrap().color.opposite()) {
                            moves.push(Move::new(from, to));
                        }
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        moves
    }

    fn generate_knight_moves(from: Position) -> Vec<Move> {
        let mut moves = Vec::new();
        let knight_moves = [
            (2, 1), (2, -1), (-2, 1), (-2, -1),
            (1, 2), (1, -2), (-1, 2), (-1, -2),
        ];

        for (file_offset, rank_offset) in knight_moves {
            let new_file = from.file as i8 + file_offset;
            let new_rank = from.rank as i8 + rank_offset;

            if let Some(to) = Position::new(new_file as u8, new_rank as u8) {
                moves.push(Move::new(from, to));
            }
        }

        moves
    }
}