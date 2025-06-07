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