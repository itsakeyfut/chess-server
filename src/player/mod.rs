pub mod player;
pub mod session;

pub use player::*;
pub use session::*;

use crate::utils::{ChessResult, ChessServerError};
use std::collections::HashMap;

#[derive(Debug)]
pub struct PlayerManager {
    players: HashMap<String, Player>,
    session_manager: SessionManager,
    name_to_id: HashMap<String, String>,
}

impl PlayerManager {
    pub fn new(session_timeout_secs: u64) -> Self {
        Self {
            players: HashMap::new(),
            session_manager: SessionManager::new(session_timeout_secs),
            name_to_id: HashMap::new(),
        }
    }

    pub fn register_player(&mut self, name: String) -> ChessResult<String> {
        let sanitized_name = crate::utils::sanitize_player_name(&name);
        if self.name_to_id.contains_key(&sanitized_name) {
            return Err(ChessServerError::PlayerAlreadyInGame {
                player_id: sanitized_name,
            });
        }

        let player = Player::new(sanitized_name.clone())?;
        let player_id = player.id.clone();

        self.players.insert(player_id.clone(), player);
        self.name_to_id.insert(sanitized_name, player_id.clone());

        Ok(player_id)
    }

    pub fn get_player(&self, player_id: &str) -> Option<&Player> {
        self.players.get(player_id)
    }

    pub fn get_player_mut(&mut self, player_id: &str) -> Option<&mut Player> {
        self.players.get_mut(player_id)
    }

    pub fn get_player_by_name(&self, name: &str) -> Option<&Player> {
        let sanitized_name = crate::utils::sanitize_player_name(name);
        self.name_to_id.get(&sanitized_name)
            .and_then(|id| self.players.get(id))
    }

    pub fn get_player_id_by_name(&self, name: &str) -> Option<String> {
        let sanitized_name = crate::utils::sanitize_player_name(name);
        self.name_to_id.get(&sanitized_name).cloned()
    }

    pub fn remove_player(&mut self, player_id: &str) -> Option<Player> {
        if let Some(player) = self.players.remove(player_id) {
            self.name_to_id.remove(&player.name);

            if let Some(session) = self.session_manager.get_session_by_player(player_id) {
                self.session_manager.remove_session(&session.id);
            }

            Some(player)
        } else {
            None
        }
    }

    pub fn search_players(&self, criteria: &PlayerSearchCriteria) -> Vec<&Player> {
        self.players.values()
            .filter(|player| criteria.matches(player))
            .collect()
    }

    pub fn get_online_players(&self) -> Vec<&Player> {
        self.players.values()
            .filter(|player| player.is_online())
            .collect()
    }

    pub fn get_available_players(&self) -> Vec<&Player> {
        let criteria = PlayerSearchCriteria::online_available();
        self.search_players(&criteria)
    }

    pub fn add_player_to_game(&mut self, player_id: &str, game_id: &str) -> ChessResult<()> {
        let player = self.players.get_mut(player_id)
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

        player.add_game(game_id.to_string())
    }

    pub fn remove_player_from_game(&mut self, player_id: &str, game_id: &str) -> ChessResult<()> {
        let player = self.players.get_mut(player_id)
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

        player.remove_game(game_id);
        Ok(())
    }

    pub fn update_player_stats(&mut self, player_id: &str, won: bool, lost: bool, drawn: bool, moves: u32, duration_secs: u64) -> ChessResult<()> {
        let player = self.players.get_mut(player_id)
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

        player.stats.update_after_game(won, lost, drawn, moves, duration_secs);
        Ok(())
    }

    pub fn update_player_rating(&mut self, player_id: &str, new_rating: u32) -> ChessResult<()> {
        let player = self.players.get_mut(player_id)
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

        player.stats.update_rating(new_rating);
        Ok(())
    }

    pub fn update_ratings_after_game(&mut self, player1_id: &str, player2_id: &str, result: GameResult) -> ChessResult<()> {
        let (player1_rating, player2_rating) = {
            let player1 = self.get_player(player1_id)
                .ok_or_else(|| ChessServerError::PlayerNotFound {
                    player_id: player1_id.to_string(),
                })?;
            let player2 = self.get_player(player2_id)
                .ok_or_else(|| ChessServerError::PlayerNotFound {
                    player_id: player2_id.to_string(),
                })?;
            (player1.stats.rating, player2.stats.rating)
        };

        let (change1, change2) = EloCalculator::calculate_rating_change(
            player1_rating,
            player2_rating,
            result
        );

        let new_rating1 = ((player1_rating as i32) + change1).max(100) as u32;
        let new_rating2 = ((player2_rating as i32) + change2).max(100) as u32;

        self.update_player_rating(player1_id, new_rating1)?;
        self.update_player_rating(player2_id, new_rating2)?;

        Ok(())
    }

    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }

    pub fn create_player_session(&mut self, player_id: &str, addr: std::net::SocketAddr, user_agent: Option<String>) -> ChessResult<String> {
        if !self.players.contains_key(player_id) {
            return Err(ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            });
        }

        self.session_manager.create_session(player_id.to_string(), addr, user_agent)
    }

    pub fn update_player_online_status(&mut self, player_id: &str, status: PlayerStatus) -> ChessResult<()> {
        let player = self.players.get_mut(player_id)
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

        player.set_status(status);
        Ok(())
    }

    pub fn get_idle_players(&self, idle_threshold_secs: u64) -> Vec<&Player> {
        self.players.values()
            .filter(|player| player.is_idle(idle_threshold_secs))
            .collect()
    }

    pub fn cleanup_expired_sessions(&mut self) -> usize {
        self.session_manager.cleanup_expired_sessions()
    }

    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }

    pub fn get_online_player_count(&self) -> usize {
        self.players.values()
            .filter(|player| player.is_online())
            .count()
    }

    pub fn get_in_game_player_count(&self) -> usize {
        self.players.values()
            .filter(|player| player.status == PlayerStatus::InGame)
            .count()
    }

    pub fn get_rating_distribution(&self) -> HashMap<String, usize> {
        let mut distribution = HashMap::new();
        
        for player in self.players.values() {
            let rating_range = match player.stats.rating {
                0..=999 => "Beginner (0-999)",
                1000..=1199 => "Novice (1000-1199)",
                1200..=1399 => "Intermediate (1200-1399)",
                1400..=1599 => "Advanced (1400-1599)",
                1600..=1799 => "Expert (1600-1799)",
                1800..=1999 => "Master (1800-1999)",
                2000..=2199 => "Grandmaster (2000-2199)",
                _ => "Super Grandmaster (2200+)",
            };
            
            *distribution.entry(rating_range.to_string()).or_insert(0) += 1;
        }
        
        distribution
    }

    pub fn find_matchmaking_opponent(&self, player_id: &str, rating_tolerance: u32) -> Option<&Player> {
        let player = self.get_player(player_id)?;
        let target_rating = player.stats.rating;

        let criteria = PlayerSearchCriteria {
            min_rating: Some(target_rating.saturating_sub(rating_tolerance)),
            max_rating: Some(target_rating + rating_tolerance),
            available_for_game: Some(true),
            online_only: true,
            ..Default::default()
        };

        self.search_players(&criteria)
            .into_iter()
            .filter(|p| p.id != player_id)
            .min_by_key(|p| {
                ((p.stats.rating as i32) - (target_rating as i32)).abs()
            })
    }

    pub fn get_player_details(&self, player_id: &str) -> Option<PlayerDetails> {
        let player = self.get_player(player_id)?;
        let session = self.session_manager.get_session_by_player(player_id);
        
        Some(PlayerDetails {
            player: player.clone(),
            session_info: session.map(|s| SessionInfo {
                session_id: s.id.clone(),
                ip_address: s.ip_address.clone(),
                connected_at: s.created_at,
                last_activity: s.last_activity,
                is_authenticated: s.is_authenticated,
                permissions: s.permissions.clone(),
            }),
            current_games: player.current_games.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlayerDetails {
    pub player: Player,
    pub session_info: Option<SessionInfo>,
    pub current_games: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub ip_address: String,
    pub connected_at: u64,
    pub last_activity: u64,
    pub is_authenticated: bool,
    pub permissions: SessionPermissions,
}

impl Default for PlayerManager {
    fn default() -> Self {
        Self::new(3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_test_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
    }

    #[test]
    fn test_player_registration() {
        let mut manager = PlayerManager::new(3600);

        let player_id = manager.register_player("TestPlayer".to_string()).unwrap();
        assert!(manager.get_player(&player_id).is_some());
        assert!(manager.get_player_by_name("TestPlayer").is_some());

        // should be failed if trying to register same name player
        assert!(manager.register_player("TestPlayer".to_string()).is_err());
    }

    #[test]
    fn test_player_search() {
        let mut manager = PlayerManager::new(3600);
        
        let player1_id = manager.register_player("Alice".to_string()).unwrap();
        let _ = manager.register_player("Bob".to_string()).unwrap();

        manager.update_player_rating(&player1_id, 1500).unwrap();
        
        let criteria = PlayerSearchCriteria::by_rating_range(1400, 1600);
        let results = manager.search_players(&criteria);
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");
    }

    #[test]
    fn test_game_management() {
        let mut manager = PlayerManager::new(3600);
        
        let player_id = manager.register_player("TestPlayer".to_string()).unwrap();

        manager.add_player_to_game(&player_id, "game1").unwrap();
        let player = manager.get_player(&player_id).unwrap();
        assert!(player.is_in_game("game1"));
        assert_eq!(player.status, PlayerStatus::InGame);

        manager.remove_player_from_game(&player_id, "game1").unwrap();
        let player = manager.get_player(&player_id).unwrap();
        assert!(!player.is_in_game("game1"));
        assert_eq!(player.status, PlayerStatus::Online);
    }

    #[test]
    fn test_rating_update() {
        let mut manager = PlayerManager::new(3600);

        let player1_id = manager.register_player("Player1".to_string()).unwrap();
        let player2_id = manager.register_player("Player2".to_string()).unwrap();

        assert_eq!(manager.get_player(&player1_id).unwrap().stats.rating, 1200);
        assert_eq!(manager.get_player(&player2_id).unwrap().stats.rating, 1200);

        manager.update_ratings_after_game(&player1_id, &player2_id, GameResult::PlayerWin).unwrap();
        
        let player1 = manager.get_player(&player1_id).unwrap();
        let player2 = manager.get_player(&player2_id).unwrap();

        assert!(player1.stats.rating > 1200);
        assert!(player2.stats.rating < 1200);
    }

    #[test]
    fn test_matchmaking() {
        let mut manager = PlayerManager::new(3600);
        
        let player1_id = manager.register_player("Player1".to_string()).unwrap();
        let player2_id = manager.register_player("Player2".to_string()).unwrap();
        let player3_id = manager.register_player("Player3".to_string()).unwrap();

        manager.update_player_rating(&player1_id, 1200).unwrap();
        manager.update_player_rating(&player2_id, 1250).unwrap();
        manager.update_player_rating(&player3_id, 1500).unwrap();
        
        let opponent = manager.find_matchmaking_opponent(&player1_id, 100);
        assert!(opponent.is_some());
        assert_eq!(opponent.unwrap().name, "Player2");
    }

    #[test]
    fn test_session_integration() {
        let mut manager = PlayerManager::new(3600);
        let addr = create_test_addr();

        let player_id = manager.register_player("TestPlayer".to_string()).unwrap();
        let session_id = manager.create_player_session(&player_id, addr, None).unwrap();

        assert!(manager.session_manager().get_session(&session_id).is_some());
        assert!(manager.session_manager().get_session_by_player(&player_id).is_some());

        let details = manager.get_player_details(&player_id).unwrap();
        assert!(details.session_info.is_some());
    }

    #[test]
    fn test_statistics() {
        let mut manager = PlayerManager::new(3600);

        for i in 0..10 {
            let player_id = manager.register_player(format!("Player{}", i)).unwrap();
            manager.update_player_rating(&player_id, 1000 + (i as u32 * 100)).unwrap();
        }

        assert_eq!(manager.get_player_count(), 10);

        let distribution = manager.get_rating_distribution();
        assert!(distribution.contains_key("Novice (1000-1199)"));
        assert!(distribution.contains_key("Intermediate (1200-1399)"));
    }
}