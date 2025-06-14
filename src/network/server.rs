use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, Duration};

use crate::game::{GameManager, Move, Position};
use crate::network::client::{Client, ClientManager, ClientState, MessageHandler};
use crate::network::protocol::*;
use crate::player::{PlayerManager, Session};
use crate::utils::{current_timestamp, ChessResult, ChessServerError, ServerConfig};

pub struct ChessServer {
    config: ServerConfig,
    client_manager: Arc<ClientManager>,
    player_manager: Arc<RwLock<PlayerManager>>,
    game_manager: Arc<RwLock<GameManager>>,
    server_info: ServerInfo,
    is_running: Arc<RwLock<bool>>,
    statistics: Arc<RwLock<ServerStatistics>>,
}

#[derive(Debug, Clone, Default)]
pub struct ServerStatistics {
    pub start_time: u64,
    pub total_connections: u64,
    pub peak_concurrent_connections: usize,
    pub total_games_created: u64,
    pub total_moves_player: u64,
    pub total_messages_processed: u64,
    pub uptime_seconds: u64,
}

impl ChessServer {
    pub fn new(config: ServerConfig) -> Self {
        let server_info = ServerInfo {
            server_name: "Chess Server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            max_players: config.server.max_connections as u32,
            current_players: 0,
            features: vec![
                "multiplayer".to_string(),
                "spectator_mode".to_string(),
                "chat".to_string(),
                "rating_system".to_string(),
            ],
        };

        Self {
            config: config.clone(),
            client_manager: Arc::new(ClientManager::new()),
            player_manager: Arc::new(RwLock::new(PlayerManager::new(
                config.security.session_timeout_secs,
            ))),
            game_manager: Arc::new(RwLock::new(GameManager::new())),
            server_info,
            is_running: Arc::new(RwLock::new(false)),
            statistics: Arc::new(RwLock::new(ServerStatistics {
                start_time: current_timestamp(),
                ..Default::default()
            })),
        }
    }

    pub async fn start(&self) -> ChessResult<()> {
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| ChessServerError::IoError {
                details: format!("Failed to bind to {}: {}", addr, e),
            })?;

        println!("Chess server listening on {}", addr);

        // Set a state server running
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        self.start_cleanup_tasks().await;

        // Accept connections
        loop {
            {
                let is_running = self.is_running.read().await;
                if !*is_running {
                    break;
                }
            }

            match listener.accept().await {
                Ok((stream, addr)) => {
                    if self.client_manager.get_client_count().await >= self.config.server.max_connections {
                        let _ = stream.shutdown().await;
                        continue;
                    }

                    {
                        let mut stats = self.statistics.write().await;
                        stats.total_connections += 1;
                        let curr_cnt = self.client_manager.get_client_count().await;
                        if curr_cnt > stats.peak_concurrent_connections {
                            stats.peak_concurrent_connections = curr_cnt;
                        }
                    }

                    self.handle_new_client(stream, addr).await;
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&self) {
        println!("Stopping chess server...");

        {
            let mut is_running = self.is_running.write().await;
            *is_running = false;
        }

        let disconnect_msg = Message::notification(MessageType::Disconnect(DisconnectRequest {
            reason: Some("Server shutdown".to_string()),
        }));

        self.client_manager.broadcast_message(disconnect_msg).await;

        tokio::time::sleep(Duration::from_millis(1000)).await;
        self.client_manager.cleanup_disconnected_clients().await;

        println!("Chess server stopped");
    }

    async fn handle_new_client(&self, stream: TcpStream, addr: SocketAddr) {
        let handler = Arc::new(ServerMessageHandler {
            client_manager: Arc::clone(&self.client_manager),
            player_manager: Arc::clone(&self.player_manager),
            game_manager: Arc::clone(&self.game_manager),
            server_info: self.server_info.clone(),
            config: self.config.clone(),
            statistics: Arc::clone(&self.statistics),
        });

        match Client::new(stream, addr, handler).await {
            Ok(client) => {
                let client = Arc::new(client);
                self.client_manager.add_client(client).await;
                println!("New cleint connected from {}", addr);
            }
            Err(e) => {
                eprintln!("Failed to create client for {}: {}", addr, e);
            }
        }
    }

    async fn start_cleanup_tasks(&self) {
        {
            let client_manager = Arc::clone(&self.client_manager);
            let player_manager = Arc::clone(&self.player_manager);
            let is_running = Arc::clone(&self.is_running);

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(300)); // 5 mins

                loop {
                    interval.tick().await;

                    {
                        let is_running = is_running.read().await;
                        if!*is_running {
                            break;
                        }
                    }

                    let disconnected_cnt = client_manager.cleanup_disconnected_clients().await;
                    if disconnected_cnt > 0 {
                        println!("Cleaned up {} disconnected clients", disconnected_cnt);
                    }

                    let expired_cnt = {
                        let mut pm = player_manager.write().await;
                        pm.cleanup_expired_sessions()
                    };
                    if expired_cnt > 0 {
                        println!("Cleaned up {} expired sessions", expired_cnt);
                    }
                }
            });
        }

        {
            let statistics = Arc::clone(&self.statistics);
            let is_running = Arc::clone(&self.is_running);

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(60)); // 1 min

                loop {
                    interval.tick().await;

                    {
                        let is_running = is_running.read().await;
                        if !*is_running {
                            break;
                        }
                    }

                    {
                        let mut stats = statistics.write().await;
                        stats.uptime_seconds = current_timestamp() - stats.start_time;
                    }
                }
            });
        }
    }

    pub async fn get_statistics(&self) -> ServerStatistics {
        self.statistics.read().await.clone()
    }

    pub async fn get_server_info(&self) -> ServerInfo {
        let mut info = self.server_info.clone();
        info.current_players = self.client_manager.get_authenticated_client_count().await as u32;
        info
    }
}

struct ServerMessageHandler {
    client_manager: Arc<ClientManager>,
    player_manager: Arc<RwLock<PlayerManager>>,
    game_manager: Arc<RwLock<GameManager>>,
    server_info: ServerInfo,
    config: ServerConfig,
    statistics: Arc<RwLock<ServerStatistics>>,
}

#[async_trait::async_trait]
impl MessageHandler for ServerMessageHandler {
    async fn handle_message(
        &self,
        message: Message,
        client_info: crate::network::client::ClientInfo,
        session: Option<Session>,
    ) -> Option<Message> {
        // 統計を更新
        {
            let mut stats = self.statistics.write().await;
            stats.total_messages_processed += 1;
        }

        match message.message_type {
            MessageType::Connect(req) => self.handle_connect(req, &client_info, message.id).await,
            MessageType::Authenticate(req) => self.handle_authenticate(req, &client_info, message.id).await,
            MessageType::CreateGame(req) => self.handle_create_game(req, &client_info, session, message.id).await,
            MessageType::JoinGame(req) => self.handle_join_game(req, &client_info, session, message.id).await,
            MessageType::MakeMove(req) => self.handle_make_move(req, &client_info, session, message.id).await,
            MessageType::GetPlayerInfo(req) => self.handle_get_player_info(req, &client_info, session, message.id).await,
            MessageType::GetGameList(req) => self.handle_get_game_list(req, &client_info, message.id).await,
            MessageType::GetGameInfo(req) => self.handle_get_game_info(req, &client_info, message.id).await,
            MessageType::GetLegalMoves(req) => self.handle_get_legal_moves(req, &client_info, session, message.id).await,
            MessageType::GetOnlinePlayers(req) => self.handle_get_online_players(req, &client_info, message.id).await,
            MessageType::Resign(req) => self.handle_resign(req, &client_info, session, message.id).await,
            MessageType::OfferDraw(req) => self.handle_offer_draw(req, &client_info, session, message.id).await,
            MessageType::RespondToDraw(req) => self.handle_respond_to_draw(req, &client_info, session, message.id).await,
            MessageType::SendMessage(req) => self.handle_send_message(req, &client_info, session, message.id).await,
            MessageType::Ping => Some(Message::response(MessageType::Pong, message.id)),
            MessageType::Heartbeat => {
                // update client's last activity
                None
            }
            _ => {
                Some(Message::error(
                    ChessServerError::UnsupportedMessageType {
                        message_type: message.type_name().to_string(),
                    },
                    message.id,
                ))
            }
        }
    }
}

impl ServerMessageHandler {
    async fn handle_connect(
        &self,
        req: ConnectRequest,
        client_info: &crate::network::client::ClientInfo,
        request_id: Option<String>,
    ) -> Option<Message> {
        let mut player_manager = self.player_manager.write().await;

        // Create a guest or new player session
        let (session_id, player_id) = if let Some(player_name) = req.player_name {
            // New player
            let player_id = match player_manager.get_player_id_by_name(&player_name) {
                Some(existing_id) => existing_id,
                None => {
                    match player_manager.register_player(player_name) {
                        Ok(id) => id,
                        Err(e) => return Some(Message::error(e, request_id)),
                    }
                }
            };

            match player_manager.create_player_session(&player_id, client_info.address, req.user_agent.clone()) {
                Ok(session_id) => (session_id, player_id),
                Err(e) => return Some(Message::error(e, request_id)),
            }
        } else {
            // Guest
            match player_manager.session_manager_mut().create_guest_session(client_info.address, req.user_agent.clone()) {
                Ok(session_id) => {
                    let session = player_manager.session_manager().get_session(&session_id).unwrap();
                    (session_id.clone(), session.player_id.clone())
                },
                Err(e) => return Some(Message::error(e, request_id)),
            }
        };

        if let Err(_) = self.client_manager.associate_session(&client_info.id, session_id.clone()).await {
            return Some(Message::error(
                ChessServerError::InternalServerError {
                    details: "Failed to associate session".to_string(),
                },
                request_id,
            ));
        }

        if let Err(_) = self.client_manager.associate_player(&client_info.id, player_id.clone()).await {
            return Some(Message::error(
                ChessServerError::InternalServerError {
                    details: "Failed to associate player".to_string(),
                },
                request_id,
            ));
        }

        Some(Message::response(
            MessageType::ConnectResponse(ConnectResponse {
                session_id,
                player_id,
                server_info: self.server_info.clone(),
            }),
            request_id,
        ))
    }

    async fn handle_authenticate(
        &self,
        req: AuthenticateRequest,
        client_info: &crate::network::client::ClientInfo,
        request_id: Option<String>,
    ) -> Option<Message> {
        let mut player_manager = self.player_manager.write().await;

        let player_id = match player_manager.get_player_id_by_name(&req.player_name) {
            Some(id) => id,
            None => {
                match player_manager.register_player(req.player_name.clone()) {
                    Ok(id) => id,
                    Err(e) => return Some(Message::error(e, request_id)),
                }
            }
        };

        if let Some(session_id) = &client_info.session_id {
            if let Err(e) = player_manager.session_manager_mut().authenticate_session(session_id, player_id.clone()) {
                return Some(Message::error(e, request_id));
            }
        }

        let player = match player_manager.get_player(&player_id) {
            Some(p) => p,
            None => return Some(Message::error(
                ChessServerError::PlayerNotFound { player_id },
                request_id,
            )),
        };

        Some(Message::response(
            MessageType::AuthenticateResponse(AuthenticateResponse {
                player_id: player.id.clone(),
                player_info: player.get_display_info(),
                session_expires_at: current_timestamp() + self.config.security.session_timeout_secs,
            }),
            request_id
        ))
    }

    async fn handle_create_game(
        &self,
        req: CreateGameRequest,
        client_info: &crate::network::client::ClientInfo,
        session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        let session = match session {
            Some(s) if s.can_create_game() => s,
            _ => return Some(Message::error(
                ChessServerError::InsufficientPermissions,
                request_id,
            )),
        };

        let mut game_manager = self.game_manager.write().await;
        let mut player_manager = self.player_manager.write().await;

        let game_id = game_manager.create_game();

        let player_color = match game_manager.join_game(&game_id, session.player_id.clone(), req.color_preference) {
            Ok(color) => color,
            Err(e) => {
                game_manager.remove_game(&game_id);
                return Some(Message::error(e, request_id));
            }
        };

        if let Err(e) = player_manager.add_player_to_game(&session.player_id, &game_id) {
            game_manager.remove_game(&game_id);
            return Some(Message::error(e, request_id));
        }

        {
            let mut stats = self.statistics.write().await;
            stats.total_games_created += 1;
        }

        Some(Message::response(
            MessageType::CreateGameResponse(CreateGameResponse {
                game_id,
                player_color,
            }),
            request_id,
        ))
    }

    async fn handle_join_game(
        &self,
        req: JoinGameRequest,
        _client_info: &crate::network::client::ClientInfo,
        session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        let session = match session {
            Some(s) if s.can_join_game() => s,
            _ => return Some(Message::error(
                ChessServerError::InsufficientPermissions,
                request_id,
            )),
        };

        let mut game_manager = self.game_manager.write().await;
        let mut player_manager = self.player_manager.write().await;

        let player_color = match game_manager.join_game(&req.game_id, session.palyer_id.clone(), req.color_preference) {
            Ok(color) => color,
            Err(e) => return Some(Message::error(e, request_id)),
        };

        if let Err(e) = player_manager.add_player_to_game(&session.player_id, &req.game_id) {
            return Some(Message::error(e, request_id));
        }

        let game = match game_manager.get_game(&req.game_id) {
            Some(g) => g,
            None => return Some(Message::error(
                ChessServerError::GameNotFound { game_id: req.game_id },
                request_id,
            )),
        };

        let opponent_id = match player_color {
            crate::game::Color::White => &game.black_player,
            crate::game::Color::Black => &game.white_player,
        };

        let opponent_info = if let Some(ref opp_id) = opponent_id {
            player_manager.get_player(opp_id).map(|p| p.get_display_info())
        } else {
            None
        };

        let game_state = self.create_game_state_snapshot(game, &player_manager).await;

        Some(Message::response(
            MessageType::JoinGameResponse(JoinGameResponse {
                game_id: req.game_id,
                player_color,
                opponent_info,
                game_state,
            }),
            request_id,
        ))
    }

    async fn handle_make_move(
        &self,
        req: MakeMoveRequest,
        _client_info: &crate::network::client::ClientInfo,
        session: Option<Session>,
        request_id: Option<String>
    ) -> Option<Message> {
        let session = match session {
            Some(s) => s,
            None => return Some(Message::error(
                ChessServerError::AuthenticationFailed,
                request_id,
            )),
        };

        let mut game_manager = self.game_manager.write().await;
        let player_manager = self.player_manager.read().await;

        if let Err(e) = game_manager.make_move(&req.game_id, &session.player_id, req.chess_move.clone()) {
            return Some(Message::error(e, request_id));
        }

        {
            let mut stats = self.statistics.write().await;
            stats.total_moves_player += 1;
        }

        let game = match game_manager.get_game(&req.game_id) {
            Some(g) => g,
            None => return Some(Message::error(
                ChessServerError::GameNotFound { game_id: req.game_id.clone() },
                request_id,
            )),
        };

        let game_state = self.create_game_state_snapshot(game, &player_manager).await;
        let update_notification = Message::notification(MessageType::GameUpdate(GameUpdateNotification {
            game_id: req.game_id.clone(),
            game_state,
            last_move: Some(req.chess_move),
            player_to_move: game.board.get_to_move(),
            is_check: game.is_in_check(),
            game_result: if game.result == crate::game::GameResult::Ongoing { None } else { Some(game.result.clone()) },
        }));

        let player_ids = vec![
            game.white_player.clone(),
            game.black_player.clone(),
        ].into_iter().flatten().collect::<Vec<_>>();

        drop(player_manager);
        drop(game_manager);

        tokio::spawn({
            let client_manager = Arc::clone(&self.client_manager);
            let notification = update_notification.clone();
            async move {
                client_manager.send_to_players(&player_ids, notification).await;
            }
        });

        Some(Message::success("Move made successfully", request_id))
    }

    async fn handle_get_player_info(
        &self,
        req: GetPlayerInfoRequest,
        _client_info: &crate::network::client::ClientInfo,
        session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        let player_manager = self.player_manager.read().await;

        let target_player_id = req.player_id.unwrap_or_else(|| {
            session.as_ref().map(|s| s.player_id.clone()).unwrap_or_default()
        });

        let player = match player_manager.get_player(&target_player_id) {
            Some(p) => p,
            None => return Some(Message::error(
                ChessServerError::PlayerNotFound { player_id: target_player_id },
                request_id,
            )),
        };

        Some(Message::response(
            MessageType::GetPlayerInfoResponse(GetPlayerInfoResponse {
                player_info: player.get_display_info(),
                detailed_stats: Some(player.stats.clone()),
            }),
            request_id,
        ))
    }

    async fn handle_get_game_list(
        &self,
        req: GetGameListRequest,
        _client_info: &crate::network::client::ClientInfo,
        request_id: Option<String>,
    ) -> Option<Message> {
        let game_manager = self.game_manager.read().await;

        let games = game_manager.get_active_games();
        let mut game_infos = Vec::new();

        for game in games {
            let game_info = game.get_game_info();
            
            // Filter
            let mut matches = true;
            
            if let Some(ref status_filter) = req.filter.status {
                let game_status = match game_info.result {
                    crate::game::GameResult::Ongoing => {
                        if game_info.white_player.is_some() && game_info.black_player.is_some() {
                            GameStatus::Active
                        } else {
                            GameStatus::Waiting
                        }
                    }
                    _ => GameStatus::Finished,
                };

                if *status_filter != game_status {
                    matches = false;
                }
            }

            if matches {
                game_infos.push(game_info);
            }
        }

        // Pagination
        let offset = req.offset.unwrap_or(0) as usize;
        let limit = req.limit.unwrap_or(50) as usize;
        let total_count = game_infos.len() as u32;
        
        if offset < game_infos.len() {
            let end = std::cmp::min(offset + limit, game_infos.len());
            game_infos = game_infos[offset..end].to_vec();
        } else {
            game_infos.clear();
        }

        Some(Message::response(
            MessageType::GetGameListResponse(GetGameListResponse {
                games: game_infos,
                total_count,
            }),
            request_id,
        ))
    }


    async fn handle_get_game_info(
        &self,
        req: GetGameInfoRequest,
        _client_info: &crate::network::client::ClientInfo,
        request_id: Option<String>,
    ) -> Option<Message> {
        let game_manager = self.game_manager.read().await;

        let game = match game_manager.get_game(&req.game_id) {
            Some(g) => g,
            None => return Some(Message::error(
                ChessServerError::GameNotFound { game_id: req.game_id },
                request_id,
            )),
        };

        Some(Message::response(
            MessageType::GetGameInfoResponse(game.get_game_info()),
            request_id,
        ))
    }

    async fn handle_get_legal_moves(
        &self,
        req: GetLegalMovesRequest,
        _client_info: &crate::network::client::ClientInfo,
        session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        let session = match session {
            Some(s) => s,
            None => return Some(Message::error(
                ChessServerError::AuthenticationFailed,
                request_id,
            )),
        };

        let game_manager = self.game_manager.read().await;

        let game = match game_manager.get_game(&req.game_id) {
            Some(g) => g,
            None => return Some(Message::error(
                ChessServerError::GameNotFound { game_id: req.game_id },
                request_id,
            )),
        };

        let legal_moves = game.get_legal_moves_for_player(&session.player_id);
        let in_check = game.is_in_check();

        Some(Message::response(
            MessageType::GetLegalMovesResponse(GetLegalMovesResponse {
                legal_moves,
                in_check,
            }),
            request_id,
        ))
    }

    async fn handle_get_online_players(
        &self,
        req: GetOnlinePlayersRequest,
        _client_info: &crate::network::client::ClientInfo,
        request_id: Option<String>,
    ) -> Option<Message> {
        let player_manager = self.player_manager.read().await;
        let online_players = player_manager.get_online_players();

        let mut player_infos: Vec<_> = online_players.iter()
            .map(|p| p.get_display_info())
            .collect();

        // Pagination
        let offset = req.offset.unwrap_or(0) as usize;
        let limit = req.limit.unwrap_or(50) as usize;
        let total_count = player_infos.len() as u32;

        if offset < player_infos.len() {
            let end = std::cmp::min(offset + limit, player_infos.len());
            player_infos = player_infos[offset..end].to_vec();
        } else {
            player_infos.clear();
        }

        Some(Message::response(
            MessageType::GetOnlinePlayersResponse(GetOnlinePlayersResponse {
                players: player_infos,
                total_count,
            }),
            request_id,
        ))
    }

    async fn handle_resign(
        &self,
        req: ResignRequest,
        _client_info: &crate::network::client::ClientInfo,
        session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        let session = match session {
            Some(s) => s,
            None => return Some(Message::error(
                ChessServerError::AuthenticationFailed,
                request_id,
            )),
        };

        let mut game_manager = self.game_manager.write().await;

        let game = match game_manager.get_game_mut(&req.game_id) {
            Some(g) => g,
            None => return Some(Message::error(
                ChessServerError::GameNotFound { game_id: req.game_id },
                request_id,
            )),
        };

        if let Err(e) = game.resign(&session.player_id) {
            return Some(Message::error(e, request_id));
        }

        Some(Message::success("Resignation recorded", request_id))
    }

    async fn handle_offer_draw(
        &self,
        _req: OfferDrawRequest,
        _client_info: &crate::network::client::ClientInfo,
        _session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        Some(Message::success("draw offer sent", request_id))
    }

    async fn handle_respond_to_draw(
        &self,
        _req: OfferDrawRequest,
        _client_info: &crate::network::client::ClientInfo,
        _session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        Some(Message::success("Draw response recorded", request_id))
    }

    async fn handle_send_message(
        &self,
        req: ChatMessageRequest,
        _client_info: &crate::network::client::ClientInfo,
        session: Option<Session>,
        request_id: Option<String>,
    ) -> Option<Message> {
        let session = match session {
            Some(s) if s.can_chat() => s,
            _ => return Some(Message::error(
                ChessServerError::InsufficientPermissions,
                request_id,
            )),
        };

        let player_manager = self.player_manager.read().await;
        let sender = match player_manager.get_player(&session.player_id) {
            Some(p) => p.get_display_info(),
            None => return Some(Message::error(
                ChessServerError::PlayerNotFound { player_id: session.player_id },
                request_id,
            )),
        };

        let chat_notification = Message::notification(MessageType::ChatMessage(ChatMessageNotification {
            game_id: req.game_id.clone(),
            sender,
            message: req.message,
            message_type: req.message_type,
            timestamp: current_timestamp(),
        }));

        drop(player_manager);

        if let Some(game_id) = req.game_id {
            let game_manager = self.game_manager.read().await;
            if let Some(game) = game_manager.get_game(&game_id) {
                let player_ids = vec![
                    game.white_player.clone(),
                    game.black_player.clone(),
                ].into_iter().flatten().collect::<Vec<_>>();

                drop(game_manager);

                tokio::spawn({
                    let client_manager = Arc::clone(&self.client_manager);
                    let notification = chat_notification.clone();
                    async move {
                        client_manager.send_to_players(&player_ids, notification).await;
                    }
                });
            }
        } else {
            tokio::spawn({
                let client_manager = Arc::clone(&self.client_manager);
                let notification = chat_notification.clone();
                async move {
                    client_manager.broadcast_to_authenticated(notification).await;
                }
            });
        }

        Some(Message::success("Message sent", request_id))
    }
}