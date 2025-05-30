use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChessServerError {
    // Game
    #[error("Game not found: {game_id}")]
    GameNotFound { game_id: String },

    #[error("Invalid move: {reason}")]
    InvalidMove { reason: String },

    #[error("Game is already finished")]
    GmaeFinished,

    #[error("Not your turn")]
    NotYourTurn,

    #[error("Game is full")]
    GameFull,

    // Player
    #[error("Player not found: {player_id}")]
    PlayerNotFound { player_id: String },

    #[error("Player already in game: {player_id}")]
    PlayerAlreadyInGame { player_id: String },

    #[error("Player not in this game: {player_id}")]
    PlayerNotInGame { player_id: String },

    #[error("Invalid player name: {name}")]
    InvalidPlayerName { name: String },

    #[error("Player authentication failed")]
    AuthenticationFailed,

    // Network
    #[error("Connection lost")]
    ConnectionLost,

    #[error("invalid message format: {details}")]
    InvalidMessage { details: String },

    #[error("Message too large: {size} bytes")]
    MessageTooLarge { size: usize },

    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Server overloaded")]
    ServerOverloaded,

    // Protocol
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersionMismatch { expected: String, actual: String },

    #[error("Unsupported message type: {message_type}")]
    UnsupportedMessageType { message_type: String },

    #[error("Missing required field: {field}")]
    MissingRequiredField { field: String },

    // System
    #[error("Configuration error: {details}")]
    ConfigurationError { details: String },

    #[error("Database error: {details}")]
    DatabaseError { details: String },

    #[error("IO error: {details}")]
    IoError { details: String },

    #[error("Serialization error: {details}")]
    SerializationError { details: String },

    #[error("Internal server error: {details}")]
    InternalServerError { details: String },

    // Validation
    #[error("Invalid position: {position}")]
    InvalidPosition { position: String },

    #[error("Invalid FEN string: {fen}")]
    InvalidFen { fen: String },

    #[error("Invalid PGN format: {details}")]
    InvalidPgn { details: String },

    // Rate Limit
    #[error("Rate limit exceeded for player: {player_id}")]
    RateLimitExceeded { player_id: String },

    #[error("Too many games for player: {player_id}")]
    TooManyGames { player_id: String },

    // Auth
    #[error("Insufficient permission")]
    InsufficientPermissions,

    #[error("Action not allowed in current game state")]
    ActionNotAllowed,
}

impl ChessServerError {
    pub fn error_code(&self) -> &'static str {
        match self {
            // Game
            ChessServerError::GameNotFound { .. } => "1001",
            ChessServerError::InvalidMove { .. } => "1002",
            ChessServerError::GmaeFinished => "1003",
            ChessServerError::NotYourTurn => "1004",
            ChessServerError::GameFull => "1005",

            // Player
            ChessServerError::PlayerNotFound { .. } => "2001",
            ChessServerError::PlayerAlreadyInGame { .. } => "2002",
            ChessServerError::InvalidPlayerName { .. } => "2004",
            ChessServerError::AuthenticationFailed => "2005",

            // Network
            ChessServerError::ConnectionLost => "3001",
            ChessServerError::InvalidMessage { .. } => "3002",
            ChessServerError::MessageTooLarge { .. } => "3003",
            ChessServerError::ConnectionTimeout => "3004",
            ChessServerError::ServerOverloaded => "3005",

            // Protocol
            ChessServerError::ProtocolVersionMismatch { .. } => "4001",
            ChessServerError::UnsupportedMessageType { .. } => "4002",
            ChessServerError::MissingRequiredField { .. } => "4003",

            // System
            ChessServerError::ConfigurationError { .. } => "5001",
            ChessServerError::DatabaseError { .. } => "5002",
            ChessServerError::IoError { .. } => "5003",
            ChessServerError::SerializationError { .. } => "5004",
            ChessServerError::InternalServerError { .. } => "5005",

            // Validation
            ChessServerError::InvalidPosition { .. } => "6001",
            ChessServerError::InvalidFen { .. } => "6002",
            ChessServerError::InvalidPgn { .. } => "6003",

            // Rate Limit
            ChessServerError::RateLimitExceeded { .. } => "7001",
            ChessServerError::TooManyGames { .. } => "7002",

            // Authentication
            ChessServerError::InsufficientPermissions => "8001",
            ChessServerError::ActionNotAllowed => "8002",
        }
    }

    pub fn is_client_error(&self) -> bool {
        matches!(self.error_code().chars().next(), Some('1'..='4') | Some('6'..='8'))
    }

    pub fn is_server_error(&self) -> bool {
        matches!(self.error_code().chars().next(), Some('5'))
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self,
            ChessServerError::ConnectionTimeout |
            ChessServerError::ServerOverloaded |
            ChessServerError::ConnectionLost |
            ChessServerError::IoError { .. }
        )
    }
}

impl From<std::io::Error> for ChessServerError {
    fn from(error: std::io::Error) -> Self {
        ChessServerError::IoError {
            details: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for ChessServerError {
    fn from(error: serde_json::Error) -> Self {
        ChessServerError::SerializationError {
            details: error.to_string(),
        }
    }
}

pub type ChessResult<T> = Result<T, ChessServerError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: u64,
}

impl ErrorResponse {
    pub fn from_error(error: &ChessServerError) -> Self {
        Self {
            error_code: error.error_code().to_string(),
            message: error.to_string(),
            details: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}