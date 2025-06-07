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