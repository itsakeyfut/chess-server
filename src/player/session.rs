use std::{collections::HashMap, net::SocketAddr};

use serde::{Deserialize, Serialize};

use crate::utils::{current_timestamp, generate_id, ChessResult, ChessServerError, RateLimiter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub player_id: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub is_authenticated: bool,
    pub permissions: SessionPermissions,
    pub rate_limiter: Option<RateLimiterState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPermissions {
    pub can_create_games: bool,
    pub can_join_games: bool,
    pub can_spectate: bool,
    pub can_chat: bool,
    pub is_admin: bool,
    pub is_moderator: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterState {
    pub tokens: f64,
    pub capacity: f64,
    pub refill_rate: f64,
    pub last_refill: u64,
}

impl Default for SessionPermissions {
    fn default() -> Self {
        Self {
            can_create_games: true,
            can_join_games: true,
            can_spectate: true,
            can_chat: true,
            is_admin: false,
            is_moderator: false,
        }
    }
}

impl SessionPermissions {
    pub fn guest() -> Self {
        Self {
            can_create_games: false,
            can_join_games: true,
            can_spectate: true,
            can_chat: false,
            is_admin: false,
            is_moderator: false,
        }
    }

    pub fn admin() -> Self {
        Self {
            can_create_games: true,
            can_join_games: true,
            can_spectate: true,
            can_chat: true,
            is_admin: true,
            is_moderator: true,
        }
    }

    pub fn moderator() -> Self {
        Self {
            can_create_games: true,
            can_join_games: true,
            can_spectate: true,
            can_chat: true,
            is_admin: false,
            is_moderator: true,
        }
    }

    pub fn banned() -> Self {
        Self {
            can_create_games: false,
            can_join_games: false,
            can_spectate: false,
            can_chat: false,
            is_admin: false,
            is_moderator: false,
        }
    }
}

impl Session {
    pub fn new(player_id: String, ip_address: String, user_agent: Option<String>) -> Self {
        Self {
            id: generate_id(),
            player_id,
            created_at: current_timestamp(),
            last_activity: current_timestamp(),
            ip_address,
            user_agent,
            is_authenticated: false,
            permissions: SessionPermissions::default(),
            rate_limiter: None,
        }
    }

    pub fn guest(ip_address: String, user_agent: Option<String>) -> Self {
        Self {
            id: generate_id(),
            player_id: format!("guest_{}", generate_id()[..8].to_string()),
            created_at: current_timestamp(),
            last_activity: current_timestamp(),
            ip_address,
            user_agent,
            is_authenticated: false,
            permissions: SessionPermissions::guest(),
            rate_limiter: None,
        }
    }

    pub fn authenticate(&mut self, player_id: String) {
        self.player_id = player_id;
        self.is_authenticated = true;
        self.permissions = SessionPermissions::default();
        self.update_activity();
    }

    pub fn update_activity(&mut self) {
        self.last_activity = current_timestamp();
    }

    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        current_timestamp() - self.last_activity > timeout_secs
    }

    pub fn duration_secs(&self) -> u64 {
        current_timestamp() - self.created_at
    }

    pub fn set_rate_limiter(&mut self, capacity: f64, refill_rate: f64) {
        self.rate_limiter = Some(RateLimiterState {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: current_timestamp(),
        });
    }

    pub fn can_perform_action(&mut self, cost: f64) -> bool {
        if let Some(ref mut limiter_state) = self.rate_limiter {
            let mut limiter = RateLimiter::new(
                limiter_state.capacity,
                limiter_state.refill_rate,
            );
            limiter.tokens = limiter_state.tokens;
            limiter.last_refill = limiter_state.last_refill;

            let can_consume = limiter.try_consume(cost);
            
            limiter_state.tokens = limiter.tokens;
            limiter_state.last_refill = limiter.last_refill;

            can_consume
        } else {
            true
        }
    }

    pub fn set_permissions(&mut self, permissions: SessionPermissions) {
        self.permissions = permissions;
        self.update_activity();
    }

    pub fn promote_to_moderator(&mut self) {
        self.permissions = SessionPermissions::moderator();
        self.update_activity();
    }

    pub fn promote_to_admin(&mut self) {
        self.permissions = SessionPermissions::admin();
        self.update_activity();
    }

    pub fn ban(&mut self) {
        self.permissions = SessionPermissions::banned();
        self.update_activity();
    }

    pub fn is_guest(&self) -> bool {
        !self.is_authenticated || self.player_id.starts_with("guest_")
    }

    pub fn can_create_game(&self) -> bool {
        self.permissions.can_create_games
    }

    pub fn can_join_game(&self) -> bool {
        self.permissions.can_join_games
    }

    pub fn can_spectate(&self) -> bool {
        self.permissions.can_chat
    }

    pub fn can_chat(&self) -> bool {
        self.permissions.can_chat
    }

    pub fn is_admin(&self) -> bool {
        self.permissions.is_admin
    }

    pub fn is_moderator(&self) -> bool {
        self.permissions.is_moderator || self.permissions.is_admin
    }

    pub fn has_elevated_permissions(&self) -> bool {
        self.is_moderator() || self.is_admin()
    }
}


#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    player_sessions: HashMap<String, String>, // player_id -> session_id
    ip_sessions: HashMap<String, Vec<String>>, // ip -> session_ids
    timeout_secs: u64,
}

impl SessionManager {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            player_sessions: HashMap::new(),
            ip_sessions: HashMap::new(),
            timeout_secs,
        }
    }

    pub fn create_session(
        &mut self,
        player_id: String,
        addr: SocketAddr,
        user_agent: Option<String>,
    ) -> ChessResult<String> {
        if let Some(existing_session_id) = self.player_sessions.get(&player_id) {
            if let Some(session) = self.sessions.get_mut(existing_session_id) {
                if !session.is_expired(self.timeout_secs) {
                    session.update_activity();
                    return Ok(session.id.clone())
                }
            }
        }

        let ip_str = addr.ip().to_string();

        let ip_session_cnt = self.ip_sessions
            .get(&ip_str)
            .map(|sessions| sessions.len())
            .unwrap_or(0);

        // 5 session per each IP
        if ip_session_cnt >= 5 {
            return Err(ChessServerError::TooManyGames {
                player_id: ip_str,
            });
        }

        let mut session = Session::new(player_id.clone(), ip_str.clone(), user_agent);

        // 60 actions within 1 min
        session.set_rate_limiter(60.0, 1.0);

        let session_id = session.id.clone();

        self.remove_player_session(&player_id);

        self.sessions.insert(session_id.clone(), session);
        self.player_sessions.insert(player_id, session_id.clone());

        self.ip_sessions
            .entry(ip_str)
            .or_insert_with(Vec::new)
            .push(session_id.clone());

        Ok(session_id)
    }

    pub fn create_guest_session(
        &mut self,
        addr: SocketAddr,
        user_agent: Option<String>,
    ) -> ChessResult<String> {
        let ip_str = addr.ip().to_string();

        let ip_session_cnt = self.ip_sessions
            .get(&ip_str)
            .map(|sessions| sessions.len())
            .unwrap_or(0);

        // 10 session per each IP
        if ip_session_cnt >= 10 {
            return Err(ChessServerError::ServerOverloaded);
        }

        let mut session = Session::guest(ip_str.clone(), user_agent);

        // 30 actions within 1 min
        session.set_rate_limiter(30.0, 0.5);

        let session_id = session.id.clone();
        let player_id = session.player_id.clone();

        self.sessions.insert(session_id.clone(), session);
        self.player_sessions.insert(player_id, session_id.clone());

        self.ip_sessions
            .entry(ip_str)
            .or_insert_with(Vec::new)
            .push(session_id.clone());

        Ok(session_id)
    }

    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    pub fn get_session_by_player(&self, player_id: &str) -> Option<&Session> {
        self.player_sessions
            .get(player_id)
            .and_then(|session_id| self.sessions.get(session_id))
    }

    pub fn get_session_by_player_mut(&mut self, player_id: &str) -> Option<&mut Session> {
        if let Some(session_id) = self.player_sessions.get(player_id).cloned() {
            self.sessions.get_mut(&session_id)
        } else {
            None
        }
    }

    pub fn authenticate_session(
        &mut self,
        session_id: &str,
        player_id: String,
    ) -> ChessResult<()> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: session_id.to_string(),
            })?;

        if session.is_authenticated {
            self.player_sessions.remove(&session.player_id);
        }

        session.authenticate(player_id.clone());
        self.player_sessions.insert(player_id, session_id.to_string());

        Ok(())
    }

    pub fn update_session_activity(&mut self, session_id: &str) -> ChessResult<()> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: session_id.to_string(),
            })?;

        session.update_activity();
        Ok(())
    }

    pub fn remove_session(&mut self, session_id: &str) -> Option<Session> {
        if let Some(session) = self.sessions.remove(session_id) {
            self.player_sessions.remove(&session.player_id);

            if let Some(ip_sessions) = self.ip_sessions.get_mut(&session.ip_address) {
                ip_sessions.retain(|id| id != session_id);
                if ip_sessions.is_empty() {
                    self.ip_sessions.remove(&session.ip_address);
                }
            }

            Some(session)
        } else {
            None
        }
    }

    fn remove_player_session(&mut self, player_id: &str) {
        if let Some(session_id) = self.player_sessions.remove(player_id) {
            self.remove_session(&session_id);
        }
    }

    pub fn cleanup_expired_sessions(&mut self) -> usize {
        let expired_session_ids: Vec<String> = self.sessions
            .iter()
            .filter(|(_, session)| session.is_expired(self.timeout_secs))
            .map(|(id, _)| id.clone())
            .collect();

        let cnt = expired_session_ids.len();
        for session_id in expired_session_ids {
            self.remove_session(&session_id);
        }

        cnt
    }

    pub fn get_active_session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn get_authenticated_session_count(&self) -> usize {
        self.sessions.values()
            .filter(|session| session.is_authenticated)
            .count()
    }

    pub fn get_guest_session_count(&self) -> usize {
        self.sessions.values()
            .filter(|session| session.is_guest())
            .count()
    }

    pub fn get_sessions_by_ip(&self, ip: &str) -> Vec<&Session> {
        if let Some(session_ids) = self.ip_sessions.get(ip) {
            session_ids.iter()
                .filter_map(|id| self.sessions.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn ban_ip(&mut self, ip: &str) {
        if let Some(session_ids) = self.ip_sessions.get(ip).cloned() {
            for session_id in session_ids {
                if let Some(session) = self.sessions.get_mut(&session_id) {
                    session.ban();
                }
            }
        }
    }

    pub fn get_session_statistics(&self) -> SessionStatistics {
        let mut stats = SessionStatistics::default();

        stats.total_sessions = self.sessions.len();
        stats.authenticated_sessions = self.get_authenticated_session_count();
        stats.guest_sessions = self.get_guest_session_count();
        stats.unique_ips = self.ip_sessions.len();

        for session in self.sessions.values() {
            stats.total_session_duration += session.duration_secs();

            if session.is_admin() {
                stats.admin_sessions += 1;
            }
            if session.is_moderator() {
                stats.moderator_sessions += 1;
            }
        }

        if stats.total_sessions > 0 {
            stats.average_session_duration = stats.total_session_duration / stats.total_sessions as u64;
        }

        stats
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionStatistics {
    pub total_sessions: usize,
    pub authenticated_sessions: usize,
    pub guest_sessions: usize,
    pub admin_sessions: usize,
    pub moderator_sessions: usize,
    pub unique_ips: usize,
    pub total_session_duration: u64,
    pub average_session_duration: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
    }

    #[test]
    fn test_session_creation() {
        let session = Session::new(
            "player123".to_string(),
            "127.0.0.1".to_string(),
            Some("TestClient/1.0".to_string())
        );
        
        assert_eq!(session.player_id, "player123");
        assert_eq!(session.ip_address, "127.0.0.1");
        assert!(!session.is_authenticated);
        assert!(session.permissions.can_create_games);
    }

    #[test]
    fn test_guest_session() {
        let session = Session::guest(
            "127.0.0.1".to_string(),
            Some("TestClient/1.0".to_string())
        );
        
        assert!(session.is_guest());
        assert!(!session.permissions.can_create_games);
        assert!(session.permissions.can_spectate);
    }

    #[test]
    fn test_session_authentication() {
        let mut session = Session::guest(
            "127.0.0.1".to_string(),
            None
        );
        
        assert!(session.is_guest());
        
        session.authenticate("authenticated_player".to_string());
        assert!(!session.is_guest());
        assert!(session.is_authenticated);
        assert_eq!(session.player_id, "authenticated_player");
    }

    #[test]
    fn test_session_manager() {
        let mut manager = SessionManager::new(3600);
        let addr = create_test_addr();
        
        let session_id = manager.create_session(
            "player1".to_string(),
            addr,
            Some("TestClient/1.0".to_string())
        ).unwrap();
        
        assert!(manager.get_session(&session_id).is_some());
        assert!(manager.get_session_by_player("player1").is_some());
        assert_eq!(manager.get_active_session_count(), 1);
    }

    #[test]
    fn test_rate_limiting() {
        let mut session = Session::new(
            "player1".to_string(),
            "127.0.0.1".to_string(),
            None
        );
        
        session.set_rate_limiter(5.0, 1.0); // 5 tokens, 1 per second
        
        // should be success til 5th times
        for _ in 0..5 {
            assert!(session.can_perform_action(1.0));
        }
        
        // should be failed in 6th times
        assert!(!session.can_perform_action(1.0));
    }

    #[test]
    fn test_session_expiration() {
        let mut session = Session::new(
            "player1".to_string(),
            "127.0.0.1".to_string(),
            None
        );
        
        assert!(!session.is_expired(3600));
        
        session.last_activity = current_timestamp() - 7200; // 2 hours ago
        assert!(session.is_expired(3600)); // 1 hour timeout
    }

    #[test]
    fn test_permissions() {
        let mut session = Session::new(
            "player1".to_string(),
            "127.0.0.1".to_string(),
            None
        );
        
        assert!(session.can_create_game());
        assert!(!session.is_admin());
        
        session.promote_to_admin();
        assert!(session.is_admin());
        assert!(session.is_moderator());
        
        session.ban();
        assert!(!session.can_create_game());
        assert!(!session.can_join_game());
    }

    #[test]
    fn test_ip_session_tracking() {
        let mut manager = SessionManager::new(3600);
        let addr = create_test_addr();
        
        for i in 0..3 {
            let session_id = manager.create_session(
                format!("player{}", i),
                addr,
                None
            ).unwrap();
            assert!(manager.get_session(&session_id).is_some());
        }
        
        let sessions = manager.get_sessions_by_ip("127.0.0.1");
        assert_eq!(sessions.len(), 3);
    }
}