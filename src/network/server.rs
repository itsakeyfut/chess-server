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
}