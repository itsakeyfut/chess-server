use serde::{Deserialize, Serialize};

use crate::utils::{current_timestamp, generate_id, RateLimiter};

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
