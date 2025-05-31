use serde::{Deserialize, Serialize};

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