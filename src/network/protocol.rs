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
    pub game_state: GameStateShapshot,
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
    pub game_state: GameStateShapshot,
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
