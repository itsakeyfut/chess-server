use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{timeout, Duration};

use super::protocol::{Message, MessageType};
use crate::player::Session;
use crate::utils::{current_timestamp, ChessResult, ChessServerError};

#[derive(Debug, Clone, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    Authenticated,
    InGame,
    Disconnecting,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub id: String,
    pub session_id: Option<String>,
    pub player_id: Option<String>,
    pub address: SocketAddr,
    pub state: ClientState,
    pub connected_at: u64,
    pub last_activity: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u32,
    pub messages_received: u32,
    pub user_agent: Option<String>,
    pub protocol_version: String,
}

pub struct Client {
    pub info: Arc<RwLock<ClientInfo>>,
    pub session: Arc<RwLock<Option<Session>>>,
    sender: mpsc::UnboundedSender<Message>,
    _receiver_handle: tokio::task::JoinHandle<()>,
    _sender_handle: tokio::task::JoinHandle<()>,
}
