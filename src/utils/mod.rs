pub mod error;
pub mod config;

pub use error::*;
pub use config::*;

use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

pub fn generate_short_id() -> String {
    generate_id()[..8].to_string()
}

pub fn sanitize_player_name(name: &str) -> String {
    name.trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(20)
        .collect()
}

pub fn message_size_bytes(message: &str) -> usize {
    message.as_bytes().len()
}

pub fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let remaining_seconds = seconds % 60;
        if remaining_seconds == 0 {
            format!("{}m", minutes)
        } else {
            format!("{}m {}s", minutes, remaining_seconds)
        }
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        let remaining_minutes = (seconds % 3600) / 60;
        if remaining_minutes == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, remaining_minutes)
        }
    } else {
        let days = seconds / 86400;
        let remaining_hours = (seconds % 86400) / 3600;
        if remaining_hours == 0 {
            format!("{}d", days)
        } else {
            format!("{}d {}h", days, remaining_hours)
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refil: u64,
}

impl RateLimiter {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refil: current_timestamp(),
        }
    }

    pub fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = current_timestamp();
        let time_passed = now - self.last_refil;

        if time_passed > 0 {
            let new_tokens = time_passed as f64 * self.refill_rate;
            self.tokens = (self.tokens + new_tokens).min(self.capacity);
            self.last_refil = now;
        }
    }

    pub fn available_tokens(&mut self) -> f64 {
        self.refill();
        self.tokens
    }

    pub fn time_until_available(&mut self, tokens: f64) -> Option<u64> {
        self.refill();

        if self.tokens >= tokens {
            None
        } else {
            let needed_tokens = tokens - self.tokens;
            Some((needed_tokens / self.refill_rate).ceil() as u64)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Statistics {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_games: u64,
    pub active_games: u64,
    pub total_moves: u64,
    pub message_send: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
    pub server_start_time: u64,
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            server_start_time: current_timestamp(),
            ..Default::default()
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        current_timestamp() - self.server_start_time
    }

    pub fn uptime_formatted(&self) -> String {
        format_duration(self.uptime_seconds())
    }

    pub fn games_per_hour(&self) -> f64 {
        let uptime_hours = self.uptime_seconds() as f64 / 3600.0;
        if uptime_hours > 0.0 {
            self.total_games as f64 / uptime_hours
        } else {
            0.0
        }
    }

    pub fn average_game_duration(&self) -> Option<f64> {
        if self.total_games > 0 {
            // TODO: record game end time
            Some(self.uptime_seconds() as f64 / self.total_games as f64)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

pub struct Logger {
    level: LogLevel,
    file_path: Option<String>,
}

impl Logger {
    pub fn new(level: LogLevel, file_path: Option<String>) -> Self {
        Self { level, file_path }
    }

    pub fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message);
    }

    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    fn log(&self, level: LogLevel, message: &str) {
        if self.should_log(&level) {
            let timestamp = current_timestamp();
            let formatted = format!(
                "[{}] [{}] {}",
                timestamp,
                self.level_string(&level),
                message
            );

            println!("{}", formatted);

            // TODO: Output log file
            // if let Some(ref _file_path) = self.file_path {}
        }
    }

    fn should_log(&self, level: &LogLevel) -> bool {
        self.level_priority(level) >= self.level_priority(&self.level)
    }

    fn level_priority(&self, level: &LogLevel) -> u8 {
        match level {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        }
    }

    fn level_string(&self, level: &LogLevel) -> &'static str {
        match level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len < 3 {
        s.chars().take(max_len).collect()
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}

pub fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}

pub fn is_valid_port(port: u16) -> bool {
    port > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(3660), "1h 1m");
        assert_eq!(format_duration(86400), "1d");
        assert_eq!(format_duration(90000), "1d 1h");
    }

    #[test]
    fn test_sanitize_player_name() {
        assert_eq!(sanitize_player_name("  Alice  "), "Alice");
        assert_eq!(sanitize_player_name("Player@123!"), "Player123");
        assert_eq!(sanitize_player_name("Valid_Name-123"), "Valid_Name-123");
        
        let long_name = "a".repeat(30);
        assert_eq!(sanitize_player_name(&long_name).len(), 20);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(10.0, 1.0);

        assert!(limiter.try_consume(5.0));
        assert!(limiter.try_consume(5.0));
        assert!(!limiter.try_consume(1.0)); // No Token Remains

        assert!(limiter.time_until_available(1.0).is_some());
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 5), "he...");
        assert_eq!(truncate_string("hi", 1), "h");
    }

    #[test]
    fn test_generate_ids() {
        let id = generate_id();
        assert_eq!(id.len(), 32); // UUID without hyphens

        let short_id = generate_short_id();
        assert_eq!(short_id.len(), 8);
    }

    #[test]
    fn test_validation_functions() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("::1"));
        assert!(!is_valid_ip("invalid.ip"));
        
        assert!(is_valid_port(8080));
        assert!(!is_valid_port(0));
    }

    #[test]
    fn test_statistics() {
        let mut stats = Statistics::new();
        stats.total_games = 100;
        stats.total_moves = 5000;
        
        assert!(stats.uptime_seconds() >= 0);
        assert!(stats.games_per_hour() >= 0.0);
    }
}