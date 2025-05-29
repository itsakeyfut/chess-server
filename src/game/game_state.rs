use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Board, Color, Move, MoveValidator, PieceType, Position};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameResult {
    Ongoing,
    Checkmate(Color),
    Stalemate,
    Draw(DrawReason),
    Resignation(Color),
    Timeout(Color),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DrawReason {
    FiftyMoveRule,
    ThreefoldRepetition,
    InsufficientMaterial,
    Agreement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub id: String,
    pub board: Board,
    pub white_player: Option<String>,
    pub black_player: Option<String>,
    pub result: GameResult,
    pub move_history: Vec<Move>,
    pub position_history: Vec<String>, // FEN
    pub created_at: u64,
    pub last_move_at: u64,
}

impl GameState {
    pub fn new() -> Self {
        let board = Board::new();
        let fen = board.to_fen();

        Self {
            id: Uuid::new_v4().to_string(),
            board,
            white_player: None,
            black_player: None,
            result: GameResult::Ongoing,
            move_history: Vec::new(),
            position_history: vec![fen],
            created_at: Self::current_timestamp(),
            last_move_at: Self::current_timestamp(),
        }
    }

    pub fn from_fen(fen: &str) -> Result<Self, String> {
        // FEN Analyzer is required.
        let mut game = Self::new();
        game.position_history = vec![fen.to_string()];
        Ok(game)
    }

    pub fn add_player(&mut self, player_id: String, color: Option<Color>) -> Result<Color, String> {
        match color {
            Some(Color::White) => {
                if self.white_player.is_some() {
                    return Err("White player already assigned".to_string());
                }
                self.white_player = Some(player_id);
                Ok(Color::White)
            }
            Some(Color::Black) => {
                if self.black_player.is_some() {
                    return Err("Black player already assigned".to_string());
                }
                self.black_player = Some(player_id);
                Ok(Color::Black)
            }
            None => {
                if self.white_player.is_none() {
                    self.white_player = Some(player_id);
                    Ok(Color::White)
                } else if self.black_player.is_none() {
                    self.black_player = Some(player_id);
                    Ok(Color::Black)
                } else {
                    Err("Game is full".to_string())
                }
            }
        }
    }

    pub fn remove_player(&mut self, player_id: &str) {
        if let Some(ref white_id) = self.white_player {
            if white_id == player_id {
                self.white_player = None;
            }
        }
        if let Some(ref black_id) = self.black_player {
            if black_id == player_id {
                self.black_player = None;
            }
        }
    }

    pub fn is_player_in_game(&self, player_id: &str) -> bool {
        self.white_player.as_ref().map_or(false, |id| id == player_id) ||
        self.black_player.as_ref().map_or(false, |id| id == player_id)
    }

    pub fn get_player_color(&self, player_id: &str) -> Option<Color> {
        if let Some(ref white_id) = self.white_player {
            if white_id == player_id {
                return Some(Color::White);
            }
        }
        if let Some(ref black_id) = self.black_player {
            if black_id == player_id {
                return Some(Color::Black);
            }
        }
        None
    }

    pub fn is_ready_to_start(&self) -> bool {
        self.white_player.is_some() && self.black_player.is_some()
    }

    pub fn make_move(&mut self, player_id: &str, chess_move: Move) -> Result<(), String> {
        if self.result != GameResult::Ongoing {
            return Err("Game is already finished".to_string());
        }

        let player_color = self.get_player_color(player_id)
            .ok_or("Player not in this game")?;

        if player_color != self.board.get_to_move() {
            return Err("Not your turn".to_string());
        }

        if !MoveValidator::is_valid_move(&self.board, &chess_move) {
            return Err("Invalid move".to_string());
        }

        self.board.make_move(&chess_move);
        self.move_history.push(chess_move);
        self.position_history.push(self.board.to_fen());
        self.last_move_at = Self::current_timestamp();

        self.chess_game_end();

        Ok(())
    }

    fn check_game_end(&mut self) {
        if MoveValidator::is_checkmate(&self.board) {
            let winner = self.board.get_to_move().opposite();
            self.result = GameResult::Checkmate(winner);
            return;
        }

        if MoveValidator::is_stalemate(&self.board) {
            self.result = GameResult::Stalemate;
            return;
        }

        if MoveValidator::is_draw_by_fifty_move_rule(&self.board) {
            self.result = GameResult::Draw(DrawReason::FiftyMoveRule);
            return;
        }

        if self.is_threefold_repetition() {
            self.result = GameResult::Draw(DrawReason::ThreefoldRepetition);
            return;
        }

        if self.is_insufficient_material() {
            self.result = GameResult::Draw(DrawReason::InsufficientMaterial);
            return;
        }
    }

    fn is_threefold_repetition(&self) -> bool {
        let curr_pos = self.position_history.last().unwrap();
        let mut cnt = 0;

        for pos in &self.position_history {
            let pos_parts: Vec<&str> = pos.split(' ').collect();
            let curr_parts: Vec<&str> = curr_pos.split(' ').collect();

            if pos_parts.len() >= 4 && curr_parts.len() >= 4 {
                if pos_parts[0..4] == curr_parts[0..4] {
                    cnt += 1;
                }
            }
        }

        cnt >= 3
    }

    fn is_insufficient_material(&self) -> bool {
        let mut white_pieces = Vec::new();
        let mut black_pieces = Vec::new();

        for rank in 0..8 {
            for file in 0..8 {
                if let Some(pos) = Position::new(file, rank) {
                    if let Some(piece) = self.board.get_piece(pos) {
                        match piece.color {
                            Color::White => white_pieces.push(piece.piece_type),
                            Color::Black => black_pieces.push(piece.piece_type),
                        }
                    }
                }
            }
        }

        Self::is_insufficient_material_for_color(&white_pieces) &&
        Self::is_insufficient_material_for_color(&black_pieces)
    }

    fn is_insufficient_material_for_color(pieces: &[PieceType]) -> bool {
        let mut bishops = 0;
        let mut knights = 0;
        let mut has_major_pieces = false;

        for &piece_type in pieces {
            match piece_type {
                PieceType::King => {},
                PieceType::Bishop => bishops += 1,
                PieceType::Knight => knights += 1,
                PieceType::Pawn | PieceType::Rook | PieceType::Queen => {
                    has_major_pieces = true;
                }
            }
        }

        if has_major_pieces {
            return false;
        }

        if bishops == 0 && knights == 0 {
            return true;
        }

        if (bishops == 1 && knights == 0) || (bishops == 0 && knights == 1) {
            return true;
        }

        false
    }
}