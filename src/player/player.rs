use crate::game::Color;
use crate::utils::{current_timestamp, generate_id, ChessResult, ChessServerError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerStatus {
    Online,
    Away,
    InGame,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
    pub total_moves: u32,
    pub average_move_time_secs: f64,
    pub longest_game_moves: u32,
    pub shortest_game_moves: u32,
    pub rating: u32,
    pub peak_rating: u32,
    pub rating_games: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            games_played: 0,
            games_won: 0,
            games_lost: 0,
            games_drawn: 0,
            total_moves: 0,
            average_move_time_secs: 0.0,
            longest_game_moves: 0,
            shortest_game_moves: u32::MAX,
            rating: 1200,
            peak_rating: 1200,
            rating_games: 0,
        }
    }
}

impl PlayerStats {
    pub fn win_rate(&self) -> f64 {
        if self.games_played == 0 {
            0.0
        } else {
            self.games_won as f64 / self.games_played as f64
        }
    }

    pub fn draw_rate(&self) -> f64 {
        if self.games_played == 0 {
            0.0
        } else {
            self.games_drawn as f64 / self.games_played as f64
        }
    }

    pub fn loss_rate(&self) -> f64 {
        if self.games_played == 0 {
            0.0
        } else {
            self.games_lost as f64 / self.games_played as f64
        }
    }

    pub fn update_after_game(&mut self, won: bool, lost: bool, drawn: bool, moves: u32, duration_secs: u64) {
        self.games_played += 1;
        if won {
            self.games_won += 1;
        } else if lost {
            self.games_lost += 1;
        } else if drawn {
            self.games_drawn += 1;
        }

        self.total_moves += moves;

        // Average moves
        if moves > 0 {
            let game_avg_move_time = duration_secs as f64 / moves as f64;
            let total_time = self.average_move_time_secs * (self.games_played - 1) as f64;
            self.average_move_time_secs = (total_time + game_avg_move_time) / self.games_played as f64;
        }

        if moves > self.longest_game_moves {
            self.longest_game_moves = moves;
        }
        if moves < self.shortest_game_moves {
            self.shortest_game_moves = moves;
        }
    }

    pub fn update_rating(&mut self, new_rating: u32) {
        self.rating = new_rating;
        if new_rating > self.peak_rating {
            self.peak_rating = new_rating;
        }
        self.rating_games += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub status: PlayerStatus,
    pub stats: PlayerStats,
    pub created_at: u64,
    pub last_seen: u64,
    pub last_game_at: Option<u64>,
    pub current_games: Vec<String>,
    pub preferences: PlayerPreferences,
    pub connection_info: Option<ConnectionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPreferences {
    pub auto_accept_draws: bool,
    pub show_coordinates: bool,
    pub piece_style: String,
    pub board_style: String,
    pub sound_enabled: bool,
    pub move_confirmation: bool,
    pub preferred_time_control: Option<TimeControl>,
    pub auto_promote_to_queen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeControl {
    pub initial_time_secs: u32,
    pub increment_secs: u32,
    pub name: String, 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub connected_at: u64,
    pub last_heartbeat: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u32,
    pub messages_received: u32,
}

impl Default for PlayerPreferences {
    fn default() -> Self {
        Self {
            auto_accept_draws: false,
            show_coordinates: true,
            piece_style: "classic".to_string(),
            board_style: "wood".to_string(),
            sound_enabled: true,
            move_confirmation: false,
            preferred_time_control: None,
            auto_promote_to_queen: true,
        }
    }
}