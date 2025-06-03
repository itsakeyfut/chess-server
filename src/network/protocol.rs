use serde::{Deserialize, Serialize};

use crate::game::{Color, GameInfo, GameResult, Move, Position};
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
