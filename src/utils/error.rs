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