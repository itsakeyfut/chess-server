pub mod error;
pub mod config;

pub use error::*;
pub use config::*;

use std::time::{SystemTime, UNIX_EPOCH};

const BYTES_PER_UNIT: f64 = 1024.0;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE; // 3600
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_MINUTE;  // 86400


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

    while size >= BYTES_PER_UNIT && unit_idx < UNITS.len() - 1 {
        size /= BYTES_PER_UNIT;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

pub fn format_duration(seconds: u64) -> String {
    if seconds < SECONDS_PER_MINUTE {
        format!("{}s", seconds)
    } else if seconds < SECONDS_PER_HOUR {
        let minutes = seconds / SECONDS_PER_MINUTE;
        let remaining_seconds = seconds % SECONDS_PER_MINUTE;
        if remaining_seconds == 0 {
            format!("{}m", minutes)
        } else {
            format!("{}m {}s", minutes, remaining_seconds)
        }
    } else if seconds < SECONDS_PER_DAY {
        let hours = seconds / SECONDS_PER_HOUR;
        let remaining_minutes = (seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
        if remaining_minutes == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, remaining_minutes)
        }
    } else {
        let days = seconds / SECONDS_PER_DAY;
        let remaining_hours = (seconds % SECONDS_PER_DAY) / SECONDS_PER_HOUR;
        if remaining_hours == 0 {
            format!("{}d", days)
        } else {
            format!("{}d {}h", days, remaining_hours)
        }
    }
}

