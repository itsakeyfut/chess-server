use serde::{Deserialize, Serialize};

use crate::game::{Color, GameInfo, GameResult, Move};
use crate::player::{PlayerDisplayInfo, PlayerPreferences, PlayerStats};
use crate::utils::{ChessResult, ChessServerError, ErrorResponse};

pub const PROTOCOL_VERSION: &str = "1.0";
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub version: String,
    pub timestamp: u64,
    pub message_type: MessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageType {
    // Connection/Authentication
    Connect(ConnectRequest),
    ConnectResponse(ConnectResponse),
    Authenticate(AuthenticateRequest),
    AuthenticateResponse(AuthenticateResponse),
    Disconnect(DisconnectRequest),

    // Game Management
    CreateGame(CreateGameRequest),
    CreateGameResponse(CreateGameResponse),
    JoinGame(JoinGameRequest),
    JoinGameResponse(JoinGameResponse),
    LeaveGame(LeaveGameRequest),
    SpectateGame(SpectateGameRequest),

    // Game Play
    MakeMove(MakeMoveRequest),
    GameUpdate(GameUpdateNotification),
    MoveUpdate(MoveUpdateNotification),

    // Game Control
    OfferDraw(OfferDrawRequest),
    RespondToDraw(RespondToDrawRequest),
    Resign(ResignRequest),
    RequestUndo(RequestUndoRequest),
    RespondToUndo(RespondToUndoRequest),

    // Player Management
    GetPlayerInfo(GetPlayerInfoRequest),
    GetPlayerInfoResponse(GetPlayerInfoResponse),
    UpdatePreferences(UpdatePreferencesRequest),
    GetOnlinePlayers(GetOnlinePlayersRequest),
    GetOnlinePlayersResponse(GetOnlinePlayersResponse),

    // Game Info
    GetGameList(GetGameListRequest),
    GetGameListResponse(GetGameListResponse),
    GetGameInfo(GetGameInfoRequest),
    GetGameInfoResponse(GameInfo),
    GetLegalMoves(GetLegalMovesRequest),
    GetLegalMovesResponse(GetLegalMovesResponse),

    // Chat
    SendMessage(ChatMessageRequest),
    ChatMessage(ChatMessageNotification),

    // System
    Ping,
    Pong,
    Heartbeat,
    Error(ErrorResponse),
    Success(SuccessResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRequest {
    pub player_name: Option<String>,
    pub client_version: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectResponse {
    pub session_id: String,
    pub player_id: String,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateRequest {
    pub player_name: String,
    pub password: Option<String>,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateResponse {
    pub player_id: String,
    pub player_info: PlayerDisplayInfo,
    pub session_expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGameRequest {
    pub time_control: Option<TimeControl>,
    pub color_preference: Option<Color>,
    pub is_private: bool,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGameResponse {
    pub game_id: String,
    pub player_color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinGameRequest {
    pub game_id: String,
    pub password: Option<String>,
    pub color_preference: Option<Color>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinGameResponse {
    pub game_id: String,
    pub player_color: Color,
    pub opponent_info: Option<PlayerDisplayInfo>,
    pub game_state: GameStateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveGameRequest {
    pub game_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectateGameRequest {
    pub game_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeMoveRequest {
    pub game_id: String,
    pub chess_move: Move,
    pub move_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameUpdateNotification {
    pub game_id: String,
    pub game_state: GameStateSnapshot,
    pub last_move: Option<Move>,
    pub player_to_move: Color,
    pub is_check: bool,
    pub game_result: Option<GameResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveUpdateNotification {
    pub game_id: String,
    pub chess_move: Move,
    pub player: Color,
    pub move_number: u32,
    pub time_taken_ms: Option<u64>,
    pub resulting_position: String, // FEN
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferDrawRequest {
    pub game_id: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespondToDrawRequest {
    pub game_id: String,
    pub accept: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResignRequest {
    pub game_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestUndoRequest {
    pub game_id: String,
    pub moves_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespondToUndoRequest {
    pub game_id: String,
    pub accept: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPlayerInfoRequest {
    // your info if None
    pub player_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPlayerInfoResponse {
    pub player_info: PlayerDisplayInfo,
    pub detailed_stats: Option<PlayerStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub preferences: PlayerPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOnlinePlayersRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOnlinePlayersResponse {
    pub players: Vec<PlayerDisplayInfo>,
    pub total_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGameListRequest {
    pub filter: GameListFilter,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGameListResponse {
    pub games: Vec<GameInfo>,
    pub total_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGameInfoRequest {
    pub game_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLegalMovesRequest {
    pub game_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLegalMovesResponse {
    pub legal_moves: Vec<Move>,
    pub in_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageRequest {
    pub game_id: Option<String>, // Global chat if None
    pub message: String,
    pub message_type: ChatMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageNotification {
    pub game_id: Option<String>,
    pub sender: PlayerDisplayInfo,
    pub message: String,
    pub message_type: ChatMessageType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_name: String,
    pub version: String,
    pub max_players: u32,
    pub current_players: u32,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeControl {
    pub initial_time_secs: u32,
    pub increment_secs: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    pub board_fen: String,
    pub move_history: Vec<Move>,
    pub white_player: Option<PlayerDisplayInfo>,
    pub black_player: Option<PlayerDisplayInfo>,
    pub to_move: Color,
    pub move_count: u32,
    pub game_result: Option<GameResult>,
    pub time_control: Option<TimeControl>,
    pub white_time_remaining_ms: Option<u64>,
    pub black_time_remaining_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameListFilter {
    pub status: Option<GameStatus>,
    pub player_name: Option<String>,
    pub time_control: Option<String>,
    pub min_rating: Option<u32>,
    pub max_rating: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameStatus {
    Waiting,
    Active,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMessageType {
    Game,
    Global,
    System,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl Message {
    pub fn new(message_type: MessageType) -> Self {
        Self {
            id: None,
            version: PROTOCOL_VERSION.to_string(),
            timestamp: crate::utils::current_timestamp(),
            message_type,
        }
    }

    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    pub fn request(message_type: MessageType) -> Self {
        Self::new(message_type).with_id(crate::utils::generate_short_id())
    }

    pub fn response(message_type: MessageType, request_id: Option<String>) -> Self {
        let mut msg = Self::new(message_type);
        msg.id = request_id;
        msg
    }

    pub fn notification(message_type: MessageType) -> Self {
        Self::new(message_type)
    }

    pub fn error(error: ChessServerError, request_id: Option<String>) -> Self {
        Self::response(
            MessageType::Error(ErrorResponse::from_error(&error)),
            request_id,
        )
    }

    pub fn success(message: &str, request_id: Option<String>) -> Self {
        Self::response(
            MessageType::Success(SuccessResponse {
                message: message.to_string(),
                data: None,
            }),
            request_id,
        )
    }

    pub fn success_with_data(
        message: &str,
        data: serde_json::Value,
        request_id: Option<String>,
    ) -> Self {
        Self::response(
            MessageType::Success(SuccessResponse {
                message: message.to_string(),
                data: Some(data),
            }),
            request_id,
        )
    }

    pub fn to_json(&self) -> ChessResult<String> {
        serde_json::to_string(self).map_err(ChessServerError::from)
    }

    pub fn to_bytes(&self) -> ChessResult<Vec<u8>> {
        let json = self.to_json()?;
        Ok(json.into_bytes())
    }

    pub fn from_json(json: &str) -> ChessResult<Self> {
        if json.len() > MAX_MESSAGE_SIZE {
            return Err(ChessServerError::MessageTooLarge { size: json.len() });
        }

        let message: Message = serde_json::from_str(json)?;

        if message.version != PROTOCOL_VERSION {
            return Err(ChessServerError::ProtocolVersionMismatch {
                expected: PROTOCOL_VERSION.to_string(),
                actual: message.version,
            });
        }

        Ok(message)
    }

    pub fn from_bytes(bytes: &[u8]) -> ChessResult<Self> {
        let json =
            String::from_utf8(bytes.to_vec()).map_err(|_| ChessServerError::InvalidMessage {
                details: "Invalid UTF-8 encoding".to_string(),
            })?;

        Self::from_json(&json)
    }

    pub fn size(&self) -> usize {
        self.to_json().map(|json| json.len()).unwrap_or(0)
    }

    pub fn is_request(&self) -> bool {
        matches!(
            self.message_type,
            MessageType::Connect(_)
                | MessageType::Authenticate(_)
                | MessageType::CreateGame(_)
                | MessageType::JoinGame(_)
                | MessageType::MakeMove(_)
                | MessageType::GetPlayerInfo(_)
                | MessageType::GetGameList(_)
                | MessageType::GetGameInfo(_)
                | MessageType::GetLegalMoves(_)
                | MessageType::GetOnlinePlayers(_)
                | MessageType::SendMessage(_)
                | MessageType::OfferDraw(_)
                | MessageType::Resign(_)
        )
    }

    pub fn is_response(&self) -> bool {
        matches!(
            self.message_type,
            MessageType::ConnectResponse(_)
                | MessageType::AuthenticateResponse(_)
                | MessageType::CreateGameResponse(_)
                | MessageType::JoinGameResponse(_)
                | MessageType::GetPlayerInfoResponse(_)
                | MessageType::GetGameListResponse(_)
                | MessageType::GetGameInfoResponse(_)
                | MessageType::GetLegalMovesResponse(_)
                | MessageType::GetOnlinePlayersResponse(_)
                | MessageType::Success(_)
                | MessageType::Error(_)
        )
    }

    pub fn is_notification(&self) -> bool {
        matches!(
            self.message_type,
            MessageType::GameUpdate(_)
                | MessageType::MoveUpdate(_)
                | MessageType::ChatMessage(_)
                | MessageType::Heartbeat
        )
    }

    pub fn type_name(&self) -> &'static str {
        match &self.message_type {
            MessageType::Connect(_) => "Connect",
            MessageType::ConnectResponse(_) => "ConnectResponse",
            MessageType::Authenticate(_) => "Authenticate",
            MessageType::AuthenticateResponse(_) => "AuthenticateResponse",
            MessageType::Disconnect(_) => "Disconnect",
            MessageType::CreateGame(_) => "CreateGame",
            MessageType::CreateGameResponse(_) => "CreateGameResponse",
            MessageType::JoinGame(_) => "JoinGame",
            MessageType::JoinGameResponse(_) => "JoinGameResponse",
            MessageType::LeaveGame(_) => "LeaveGame",
            MessageType::SpectateGame(_) => "SpectateGame",
            MessageType::MakeMove(_) => "MakeMove",
            MessageType::GameUpdate(_) => "GameUpdate",
            MessageType::MoveUpdate(_) => "MoveUpdate",
            MessageType::OfferDraw(_) => "OfferDraw",
            MessageType::RespondToDraw(_) => "RespondToDraw",
            MessageType::Resign(_) => "Resign",
            MessageType::RequestUndo(_) => "RequestUndo",
            MessageType::RespondToUndo(_) => "RespondToUndo",
            MessageType::GetPlayerInfo(_) => "GetPlayerInfo",
            MessageType::GetPlayerInfoResponse(_) => "GetPlayerInfoResponse",
            MessageType::UpdatePreferences(_) => "UpdatePreferences",
            MessageType::GetOnlinePlayers(_) => "GetOnlinePlayers",
            MessageType::GetOnlinePlayersResponse(_) => "GetOnlinePlayersResponse",
            MessageType::GetGameList(_) => "GetGameList",
            MessageType::GetGameListResponse(_) => "GetGameListResponse",
            MessageType::GetGameInfo(_) => "GetGameInfo",
            MessageType::GetGameInfoResponse(_) => "GetGameInfoResponse",
            MessageType::GetLegalMoves(_) => "GetLegalMoves",
            MessageType::GetLegalMovesResponse(_) => "GetLegalMovesResponse",
            MessageType::SendMessage(_) => "SendMessage",
            MessageType::ChatMessage(_) => "ChatMessage",
            MessageType::Ping => "Ping",
            MessageType::Pong => "Pong",
            MessageType::Heartbeat => "Heartbeat",
            MessageType::Error(_) => "Error",
            MessageType::Success(_) => "Success",
        }
    }
}

impl Default for GameListFilter {
    fn default() -> Self {
        Self {
            status: None,
            player_name: None,
            time_control: None,
            min_rating: None,
            max_rating: None,
        }
    }
}

pub fn create_connect_request(
    player_name: Option<String>,
    client_version: Option<String>,
) -> Message {
    Message::request(MessageType::Connect(ConnectRequest {
        player_name,
        client_version,
        user_agent: Some("Chess Client".to_string()),
    }))
}

pub fn create_make_move_request(game_id: String, chess_move: Move) -> Message {
    Message::request(MessageType::MakeMove(MakeMoveRequest {
        game_id,
        chess_move,
        move_time_ms: None,
    }))
}

pub fn create_game_update_notification(
    game_id: String,
    game_state: GameStateSnapshot,
    last_move: Option<Move>,
    player_to_move: Color,
    is_check: bool,
    game_result: Option<GameResult>,
) -> Message {
    Message::notification(MessageType::GameUpdate(GameUpdateNotification {
        game_id,
        game_state,
        last_move,
        player_to_move,
        is_check,
        game_result,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Position;

    #[test]
    fn test_message_serialization() {
        let connect_msg = create_connect_request(
            Some("TestPlayer".to_string()),
            Some("TestClient/1.0".to_string()),
        );

        let json = connect_msg.to_json().unwrap();
        let deserialized = Message::from_json(&json).unwrap();

        assert_eq!(connect_msg.version, deserialized.version);
        assert_eq!(connect_msg.type_name(), deserialized.type_name());
    }

    #[test]
    fn test_move_message() {
        let chess_move = Move::new(
            Position::from_algebraic("e2").unwrap(),
            Position::from_algebraic("e4").unwrap(),
        );

        let move_msg = create_make_move_request("game123".to_string(), chess_move);

        let json = move_msg.to_json().unwrap();
        let deserialized = Message::from_json(&json).unwrap();

        match deserialized.message_type {
            MessageType::MakeMove(req) => {
                assert_eq!(req.game_id, "game123");
                assert_eq!(req.chess_move.from.to_algebraic(), "e2");
                assert_eq!(req.chess_move.to.to_algebraic(), "e4");
            }
            _ => panic!("Expected MakeMove message"),
        }
    }

    #[test]
    fn test_message_size_limit() {
        let large_string = "a".repeat(MAX_MESSAGE_SIZE + 1);
        let result = Message::from_json(&large_string);
        assert!(result.is_err());
    }

    #[test]
    fn test_protocol_version_mismatch() {
        let mut connect_msg = create_connect_request(
            Some("TestPlayer".to_string()),
            Some("TestClient/1.0".to_string()),
        );
        connect_msg.version = "2.0".to_string();

        let json = connect_msg.to_json().unwrap();
        let result = Message::from_json(&json);

        assert!(result.is_err());
        if let Err(ChessServerError::ProtocolVersionMismatch { .. }) = result {
            // Expected error
        } else {
            panic!("Expected ProtocolVersionMismatch error");
        }
    }

    #[test]
    fn test_message_types() {
        let ping_msg = Message::new(MessageType::Ping);
        assert!(!ping_msg.is_request());
        assert!(!ping_msg.is_response());
        assert!(!ping_msg.is_notification());

        let connect_msg = create_connect_request(None, None);
        assert!(connect_msg.is_request());
        assert!(!connect_msg.is_response());
        assert!(!connect_msg.is_notification());

        let success_msg = Message::success("OK", Some("123".to_string()));
        assert!(!success_msg.is_request());
        assert!(success_msg.is_response());
        assert!(!success_msg.is_notification());
    }

    #[test]
    fn test_error_message() {
        let error = ChessServerError::GameNotFound {
            game_id: "test123".to_string(),
        };
        let error_msg = Message::error(error, Some("req123".to_string()));

        match error_msg.message_type {
            MessageType::Error(err_resp) => {
                assert_eq!(err_resp.error_code, "1001");
                assert!(err_resp.message.contains("test123"));
            }
            _ => panic!("Expected Error message"),
        }
    }
}
