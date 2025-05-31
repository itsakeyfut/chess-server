use crate::utils::{current_timestamp, generate_id, ChessResult, ChessServerError};
use serde::{Deserialize, Serialize};

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

impl Player {
    pub fn new(name: String) -> ChessResult<Self> {
        if name.trim().is_empty() {
            return Err(ChessServerError::InvalidPlayerName { name });
        }

        let sanitized_name = crate::utils::sanitize_player_name(&name);
        if sanitized_name.is_empty() {
            return Err(ChessServerError::InvalidPlayerName { name });
        }

        Ok(Self {
            id: generate_id(),
            name: sanitized_name,
            status: PlayerStatus::Online,
            stats: PlayerStats::default(),
            created_at: current_timestamp(),
            last_seen: current_timestamp(),
            last_game_at: None,
            current_games: Vec::new(),
            preferences: PlayerPreferences::default(),
            connection_info: None,
        })
    }

    pub fn with_connection(name: String, ip_address: String, user_agent: Option<String>) -> ChessResult<Self> {
        let mut player = Self::new(name)?;
        player.set_connection_info(ip_address, user_agent);
        Ok(player)
    }

    pub fn set_connection_info(&mut self, ip_address: String, user_agent: Option<String>) {
        let now = current_timestamp();
        self.connection_info = Some(ConnectionInfo {
            ip_address,
            user_agent,
            connected_at: now,
            last_heartbeat: now,
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
        });
        self.last_seen = now;
    }

    pub fn update_heartbeat(&mut self) {
        self.last_seen = current_timestamp();
        if let Some(ref mut conn_info) = self.connection_info {
            conn_info.last_heartbeat = current_timestamp();
        }
    }

    pub fn add_sent_data(&mut self, bytes: u64) {
        if let Some(ref mut conn_info) = self.connection_info {
            conn_info.bytes_sent += bytes;
            conn_info.messages_sent += 1;
        }
    }

    pub fn add_received_data(&mut self, bytes: u64) {
        if let Some(ref mut conn_info) = self.connection_info {
            conn_info.bytes_received += bytes;
            conn_info.messages_received += 1;
        }
    }
    
    pub fn disconnect(&mut self) {
        self.status = PlayerStatus::Offline;
        self.connection_info = None;
        self.last_seen = current_timestamp();
    }

    pub fn set_status(&mut self, status: PlayerStatus) {
        self.status = status;
        self.last_seen = current_timestamp();
    }

    pub fn add_game(&mut self, game_id: String) -> ChessResult<()> {
        if self.current_games.len() >= 10 {
            return Err(ChessServerError::TooManyGames {
                player_id: self.id.clone(),
            });
        }

        if !self.current_games.contains(&game_id) {
            self.current_games.push(game_id);
            self.status = PlayerStatus::InGame;
        }
        Ok(())
    }

    pub fn remove_game(&mut self, game_id: &str) {
        self.current_games.retain(|id| id != game_id);
        self.last_game_at = Some(current_timestamp());
        
        if self.current_games.is_empty() && self.status == PlayerStatus::InGame {
            self.status = PlayerStatus::Online;
        }
    }

    pub fn is_in_game(&self, game_id: &str) -> bool {
        self.current_games.contains(&game_id.to_string())
    }

    pub fn is_available_for_game(&self) -> bool {
        matches!(self.status, PlayerStatus::Online | PlayerStatus::Away) &&
        self.current_games.len() < 5 // 5 users can play at the same time
    }

    pub fn is_online(&self) -> bool {
        !matches!(self.status, PlayerStatus::Offline)
    }

    pub fn time_since_last_seen(&self) -> u64 {
        current_timestamp() - self.last_seen
    }

    pub fn is_idle(&self, idle_threshold_secs: u64) -> bool {
        self.time_since_last_seen() > idle_threshold_secs
    }

    pub fn update_preferences(&mut self, preferences: PlayerPreferences) {
        self.preferences = preferences;
        self.last_seen = current_timestamp();
    }

    pub fn get_rating(&self) -> u32 {
        self.stats.rating
    }


    pub fn get_display_info(&self) -> PlayerDisplayInfo {
        PlayerDisplayInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            status: self.status.clone(),
            rating: self.stats.rating,
            games_played: self.stats.games_played,
            win_rate: self.stats.win_rate(),
            is_online: self.is_online(),
            current_game_count: self.current_games.len(),
        }
    }

    pub fn get_detailed_stats(&self) -> DetailedPlayerStats {
        DetailedPlayerStats {
            basic_stats: self.stats.clone(),
            total_play_time_estimate: self.stats.games_played as u64 * 1800, // 30 min
            account_age_days: (current_timestamp() - self.created_at) / 86400,
            last_active: self.last_seen,
            games_this_session: if self.last_game_at.is_some() { 1 } else { 0 },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDisplayInfo {
    pub id: String,
    pub name: String,
    pub status: PlayerStatus,
    pub rating: u32,
    pub games_played: u32,
    pub win_rate: f64,
    pub is_online: bool,
    pub current_game_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedPlayerStats {
    pub basic_stats: PlayerStats,
    pub total_play_time_estimate: u64,
    pub account_age_days: u64,
    pub last_active: u64,
    pub games_this_session: u32,
}

pub struct EloCalculator;

impl EloCalculator {
    const K_FACTOR: f64 = 32.0;

    pub fn calculate_rating_change(
        player_rating: u32,
        opponent_rating: u32,
        result: GameResult,
    ) -> (i32, i32) {
        let player_expected = Self::expected_score(player_rating as f64, opponent_rating as f64);
        let opponent_expected = Self::expected_score(opponent_rating as f64, player_rating as f64);

        let (player_score, opponent_score) = match result {
            GameResult::PlayerWin => (1.0, 1.0),
            GameResult::OpponentWin => (0.0, 1.0),
            GameResult::Draw => (0.5, 0.5),
        };

        let player_change = Self::K_FACTOR * (player_score - player_expected);
        let opponent_change = Self::K_FACTOR * (opponent_score - opponent_expected);

        (player_change.round() as i32, opponent_change.round() as i32)
    }

    fn expected_score(rating_a: f64, rating_b: f64) -> f64 {
        1.0 / (1.0 + 10.0_f64.powf((rating_a - rating_b) / 400.0))
    }
}

#[derive(Debug, Clone)]
pub enum GameResult {
    PlayerWin,
    OpponentWin,
    Draw,
}

#[derive(Debug, Clone)]
pub struct PlayerSearchCriteria {
    pub name_contains: Option<String>,
    pub min_rating: Option<u32>,
    pub max_rating: Option<u32>,
    pub status: Option<PlayerStatus>,
    pub available_for_game: Option<bool>,
    pub min_games_played: Option<u32>,
    pub online_only: bool,
}

impl Default for PlayerSearchCriteria {
    fn default() -> Self {
        Self {
            name_contains: None,
            min_rating: None,
            max_rating: None,
            status: None,
            available_for_game: None,
            min_games_played: None,
            online_only: false,
        }
    }
}

impl PlayerSearchCriteria {
    pub fn matches(&self, player: &Player) -> bool {
        if let Some(ref name_filter) = self.name_contains {
            if !player.name.to_lowercase().contains(&name_filter.to_lowercase()) {
                return false;
            }
        }

        if let Some(min_rating) = self.min_rating {
            if player.stats.rating < min_rating {
                return false;
            }
        }

        if let Some(max_rating) = self.max_rating {
            if player.stats.rating > max_rating {
                return false;
            }
        }

        if let Some(ref status_filter) = self.status {
            if player.status != *status_filter {
                return false;
            }
        }

        if let Some(available) = self.available_for_game {
            if player.is_available_for_game() != available {
                return false;
            }
        }

        if let Some(min_games) = self.min_games_played {
            if player.stats.games_played < min_games {
                return false;
            }
        }

        true
    }
}