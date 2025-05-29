use std::collections::HashMap;

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

        self.board.make_move(&chess_move)?;
        self.move_history.push(chess_move);
        self.position_history.push(self.board.to_fen());
        self.last_move_at = Self::current_timestamp();

        self.check_game_end();

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

    pub fn resign(&mut self, player_id: &str) -> Result<(), String> {
        if self.result != GameResult::Ongoing {
            return Err("Game is already finished".to_string());
        }

        let player_color = self.get_player_color(player_id)
            .ok_or("Player not in this game")?;

        self.result = GameResult::Resignation(player_color);
        self.last_move_at = Self::current_timestamp();
        Ok(())
    }

    pub fn offer_draw(&mut self, player_id: &str) -> Result<(), String> {
        if self.result != GameResult::Ongoing {
            return Err("Game is already finished".to_string());
        }

        if !self.is_player_in_game(player_id) {
            return Err("Player not in this game".to_string());
        }

        // TODO: Wait for opponent's agreement of draw
        self.result = GameResult::Draw(DrawReason::Agreement);
        self.last_move_at = Self::current_timestamp();
        Ok(())
    }

    pub fn timeout(&mut self, player_id: &str) -> Result<(), String> {
        if self.result != GameResult::Ongoing {
            return Err("Game is already finished".to_string());
        }

        let player_color = self.get_player_color(player_id)
            .ok_or("Player not in this game")?;

        self.result = GameResult::Timeout(player_color);
        self.last_move_at = Self::current_timestamp();
        Ok(())
    }

    pub fn get_legal_moves(&self) -> Vec<Move> {
        if self.result != GameResult::Ongoing {
            return Vec::new();
        }
        MoveValidator::generate_legal_moves(&self.board)
    }

    pub fn get_legal_moves_for_player(&self, player_id: &str) -> Vec<Move> {
        if let Some(player_color) = self.get_player_color(player_id) {
            if player_color == self.board.get_to_move() {
                return self.get_legal_moves();
            }
        }
        Vec::new()
    }

    pub fn is_in_check(&self) -> bool {
        MoveValidator::is_in_check(&self.board, self.board.get_to_move())
    }

    pub fn get_current_player(&self) -> Option<&String> {
        match self.board.get_to_move() {
            Color::White => self.white_player.as_ref(),
            Color::Black => self.black_player.as_ref(),
        }
    }

    pub fn get_opponent(&self, player_id: &str) -> Option<&String> {
        if let Some(ref white_id) = self.white_player {
            if white_id == player_id {
                return self.black_player.as_ref();
            }
        }
        if let Some(ref black_id) = self.black_player {
            if black_id == player_id {
                return self.white_player.as_ref();
            }
        }
        None
    }

    pub fn get_move_count(&self) -> usize {
        self.move_history.len()
    }

    pub fn get_last_move(&self) -> Option<&Move> {
        self.move_history.last()
    }

    pub fn to_pgn(&self) -> String {
        let mut pgn = String::new();

        // PGN headers
        pgn.push_str(&format!("[Event \"Chess game\"]\n"));
        pgn.push_str(&format!("[Site \"Chess Server\"]\n"));
        pgn.push_str(&format!("[Date \"{}\"]\n", Self::format_date(self.created_at)));
        pgn.push_str(&format!("[White \"{}\"]\n",
            self.white_player.as_deref().unwrap_or("Unknown")));
        pgn.push_str(&format!("[Black \"{}\"]\n",
            self.black_player.as_deref().unwrap_or("Unknown")));

        let result_str = match &self.result {
            GameResult::Checkmate(Color::White) => "1-0",
            GameResult::Checkmate(Color::Black) => "0-1",
            GameResult::Stalemate | GameResult::Draw(_) => "1/2-1/2",
            GameResult::Resignation(Color::White) => "0-1",
            GameResult::Resignation(Color::Black) => "1-0",
            GameResult::Timeout(Color::White) => "0-1",
            GameResult::Timeout(Color::Black) => "1-0",
            GameResult::Ongoing => "*",
        };
        pgn.push_str(&format!("[Result \"{}\"]\n", result_str));
        pgn.push('\n');

        for (i, chess_move) in self.move_history.iter().enumerate() {
            if i % 2 == 0 {
                pgn.push_str(&format!("{}.", (i / 2) + 1));
            }
            pgn.push_str(&format!(" {} ", chess_move.to_algebraic()));
        }

        pgn.push_str(&format!(" {}", result_str));
        pgn
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn format_date(timestamp: u64) -> String {
        // TODO: use chrono
        format!("{}", timestamp)
    }

    pub fn get_game_info(&self) -> GameInfo {
        GameInfo {
            id: self.id.clone(),
            white_player: self.white_player.clone(),
            black_player: self.black_player.clone(),
            to_move: self.board.get_to_move(),
            result: self.result.clone(),
            move_count: self.move_history.len(),
            is_in_check: self.is_in_check(),
            last_move: self.get_last_move().cloned(),
            created_at: self.created_at,
            last_move_at: self.last_move_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: String,
    pub white_player: Option<String>,
    pub black_player: Option<String>,
    pub to_move: Color,
    pub result: GameResult,
    pub move_count: usize,
    pub is_in_check: bool,
    pub last_move: Option<Move>,
    pub created_at: u64,
    pub last_move_at: u64,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GameManager {
    games: HashMap<String, GameState>,
    player_games: HashMap<String, Vec<String>>, // Player ID -> its list
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
            player_games: HashMap::new(),
        }
    }

    pub fn create_game(&mut self) -> String {
        let game = GameState::new();
        let game_id = game.id.clone();
        self.games.insert(game_id.clone(), game);
        game_id
    }

    pub fn join_game(&mut self, game_id: &str, player_id: String, color: Option<Color>) -> Result<Color, String> {
        let game = self.games.get_mut(game_id)
            .ok_or("Game not found")?;

        let assigned_color = game.add_player(player_id.clone(), color)?;

        self.player_games.entry(player_id)
            .or_insert_with(Vec::new)
            .push(game_id.to_string());

        Ok(assigned_color)
    }

    pub fn leave_game(&mut self, game_id: &str, player_id: &str) -> Result<(), String> {
        let game = self.games.get_mut(game_id)
            .ok_or("Game not found")?;

        game.remove_player(player_id);

        if let Some(player_games) = self.player_games.get_mut(player_id) {
            player_games.retain(|id| id != game_id);
        }

        Ok(())
    }

    pub fn make_move(&mut self, game_id: &str, player_id: &str, chess_move: Move) -> Result<(), String> {
        let game = self.games.get_mut(game_id)
            .ok_or("Game not found")?;

        game.make_move(player_id, chess_move)
    }

    pub fn get_game(&self, game_id: &str) -> Option<&GameState> {
        self.games.get(game_id)
    }

    pub fn get_game_mut(&mut self, game_id: &str) -> Option<&mut GameState> {
        self.games.get_mut(game_id)
    }

    pub fn get_player_games(&self, player_id: &str) -> Vec<&GameState> {
        if let Some(game_ids) = self.player_games.get(player_id) {
            game_ids.iter()
                .filter_map(|id| self.games.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }
}