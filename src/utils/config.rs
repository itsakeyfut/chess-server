use crate::utils::error::{ChessResult, ChessServerError};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: NetworkConfig,
    pub game: GameConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub connection_timeout_secs: u64,
    pub max_message_size: usize,
    pub heartbeat_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub max_games_per_player: usize,
    pub game_timeout_secs: u64,
    pub move_timeout_secs: u64,
    pub cleanup_interval_secs: u64,
    pub max_concurrent_games: usize,
    pub allow_spectators: bool,
    pub auto_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub require_authentication: bool,
    pub rate_limit_moves_per_minute: u32,
    pub rate_limit_connections_per_ip: u32,
    pub max_player_name_length: usize,
    pub allowed_chars_in_name: String,
    pub session_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
    pub log_games: bool,
    pub log_connections: bool,
    pub log_errors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
    pub enable_migrations: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: NetworkConfig::default(),
            game: GameConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            database: None,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            connection_timeout_secs: 30,
            max_message_size: 1024 * 1024, // 1MB
            heartbeat_interval_secs: 30,
        }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            max_games_per_player: 5,
            game_timeout_secs: 3600, // 1 hour
            move_timeout_secs: 300,  // 5 min
            cleanup_interval_secs: 300, // 5 min
            max_concurrent_games: 10000,
            allow_spectators: true,
            auto_match: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_authentication: false,
            rate_limit_moves_per_minute: 60,
            rate_limit_connections_per_ip: 10,
            max_player_name_length: 20,
            allowed_chars_in_name: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-".to_string(),
            session_timeout_secs: 86400, // 24 hours
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
            log_games: true,
            log_connections: true,
            log_errors: true,
        }
    }
}

impl ServerConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> ChessResult<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| ChessServerError::ConfigurationError {
                details: format!("Failed to read config file: {}", e),
            })?;

        let config: ServerConfig = toml::from_str(&content)
            .or_else(|_| serde_json::from_str(&content))
            .map_err(|e| ChessServerError::ConfigurationError {
                details: format!("Failed to parse config file: {}", e),
            })?;

        config.validate()?;
        Ok(config)
    }

    pub fn merge_from_env(mut self) -> Self {
        // Server
        if let Ok(host) = env::var("CHESS_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = env::var("CHESS_SERVER_PORT") {
            if let Ok(port) = port.parse::<u16>() {
                self.server.port = port;
            }
        }
        if let Ok(max_conn) = env::var("CHESS_SERVER_MAX_CONNECTIONS") {
            if let Ok(max_conn) = max_conn.parse::<usize>() {
                self.server.max_connections = max_conn;
            }
        }

        // Game
        if let Ok(max_games) = env::var("CHESS_MAX_GAMES_PER_PLAYER") {
            if let Ok(max_games) = max_games.parse::<usize>() {
                self.game.max_games_per_player = max_games;
            }
        }
        if let Ok(timeout) = env::var("CHESS_GAME_TIMEOUT_SECS") {
            if let Ok(timeout) = timeout.parse::<u64>() {
                self.game.game_timeout_secs = timeout;
            }
        }

        // Security
        if let Ok(require_auth) = env::var("CHESS_REQUIRE_AUTH") {
            self.security.require_authentication = require_auth.to_lowercase() == "true";
        }

        // Logging
        if let Ok(level) = env::var("CHESS_LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(log_file) = env::var("CHESS_LOG_FILE") {
            self.logging.file_path = Some(log_file);
        }

        // Database
        if let Ok(db_url) = env::var("CHESS_DATABASE_URL") {
            let db_config = DatabaseConfig {
                url: db_url,
                max_connections: env::var("CHESS_DB_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
                connection_timeout_secs: env::var("CHESS_DB_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30),
                enable_migrations: env::var("CHESS_DB_ENABLE_MIGRATIONS")
                    .map(|s| s.to_lowercase() == "true")
                    .unwrap_or(true),
            };
            self.database = Some(db_config);
        }

        self
    }

    pub fn validate(&self) -> ChessResult<()> {
        if self.server.port == 0 {
            return Err(ChessServerError::ConfigurationError {
                details: "Server port cannot be 0".to_string(),
            });
        }

        if self.server.max_connections == 0 {
            return Err(ChessServerError::ConfigurationError {
                details: "Max connections must be greater than 0".to_string(),
            });
        }

        if self.security.max_player_name_length == 0 {
            return Err(ChessServerError::ConfigurationError {
                details: "Max player name length must be greater than 0".to_string(),
            });
        }

        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.logging.level.as_str()) {
            return Err(ChessServerError::ConfigurationError {
                details: format!(
                    "Invalid log level '{}'. Must be one of: {}",
                    self.logging.level,
                    valid_log_levels.join(", ")
                ),
            });
        }

        if let Some(ref db_config) = self.database {
            if db_config.url.is_empty() {
                return Err(ChessServerError::ConfigurationError {
                    details: "Database URL cannot be empty".to_string(),
                });
            }
        }

        Ok(())
    }

    pub fn to_string_pretty(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_else(|_| format!("{:#?}", self))
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> ChessResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ChessServerError::ConfigurationError {
                details: format!("Failed to serialize config: {}", e),
            })?;

        fs::write(path, content)
            .map_err(|e| ChessServerError::ConfigurationError {
                details: format!("Failed to write config file: {}", e),
            })?;

        Ok(())
    }

    pub fn development() -> Self {
        let mut config = Self::default();
        config.server.host = "0.0.0.0".to_string();
        config.server.port = 8080;
        config.logging.level = "debug".to_string();
        config.security.require_authentication = false;
        config.game.auto_match = true;
        config
    }

    pub fn production() -> Self {
        let mut config = Self::default();
        config.server.host = "0.0.0.0".to_string();
        config.server.port = 80;
        config.logging.level = "warn".to_string();
        config.logging.file_path = Some("/var/log/chess-server.log".to_string());
        config.security.require_authentication = true;
        config.security.rate_limit_moves_per_minute = 30;
        config.game.game_timeout_secs = 7200; // 2時間
        config
    }

    pub fn test() -> Self {
        let mut config = Self::default();
        config.server.host = "127.0.0.1".to_string();
        config.server.port = 0; // ランダムポート
        config.logging.level = "error".to_string();
        config.security.require_authentication = false;
        config.game.max_games_per_player = 1;
        config.game.game_timeout_secs = 60;
        config
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn is_valid_player_name(&self, name: &str) -> bool {
        if name.is_empty() || name.len() > self.security.max_player_name_length {
            return false;
        }

        name.chars().all(|c| self.security.allowed_chars_in_name.contains(c))
    }
}

pub fn load_config() -> ChessResult<ServerConfig> {
    let config_paths = [
        "chess-server.toml",
        "config/chess-server.toml",
        "/etc/chess-server/config.toml",
        "chess-server.json", 
        "config/chess-server.json",
    ];

    let mut config = ServerConfig::default();

    for path in &config_paths {
        if Path::new(path).exists() {
            config = ServerConfig::from_file(path)?;
            break;
        }
    }

    config = config.merge_from_env();

    config.validate()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = ServerConfig::default();
        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_player_name_validation() {
        let config = ServerConfig::default();
        assert!(config.is_valid_player_name("Alice"));
        assert!(config.is_valid_player_name("Player_123"));
        assert!(!config.is_valid_player_name(""));
        assert!(!config.is_valid_player_name("Player@Invalid"));
        
        // 長すぎる名前
        let long_name = "a".repeat(config.security.max_player_name_length + 1);
        assert!(!config.is_valid_player_name(&long_name));
    }

    #[test]
    fn test_config_file_operations() {
        let config = ServerConfig::development();
        
        // ファイルに保存
        let temp_file = NamedTempFile::new().unwrap();
        config.save_to_file(temp_file.path()).unwrap();
        
        // ファイルから読み込み
        let loaded_config = ServerConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(config.server.host, loaded_config.server.host);
        assert_eq!(config.server.port, loaded_config.server.port);
    }

    #[test]
    fn test_preset_configs() {
        let dev_config = ServerConfig::development();
        assert_eq!(dev_config.logging.level, "debug");
        assert!(!dev_config.security.require_authentication);
        
        let prod_config = ServerConfig::production();
        assert_eq!(prod_config.logging.level, "warn");
        assert!(prod_config.security.require_authentication);
        
        let test_config = ServerConfig::test();
        assert_eq!(test_config.server.port, 0);
        assert_eq!(test_config.game.max_games_per_player, 1);
    }
}