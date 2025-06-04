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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug)]
pub struct Client {
    pub info: Arc<RwLock<ClientInfo>>,
    pub session: Arc<RwLock<Option<Session>>>,
    sender: mpsc::UnboundedSender<Message>,
    _receiver_handle: tokio::task::JoinHandle<()>,
    _sender_handle: tokio::task::JoinHandle<()>,
}

impl Client {
    pub async fn new(
        stream: TcpStream,
        address: SocketAddr,
        message_handler: Arc<dyn MessageHandler + Send + Sync>,
    ) -> ChessResult<Self> {
        let client_id = crate::utils::generate_id();

        let info = Arc::new(RwLock::new(ClientInfo {
            id: client_id.clone(),
            session_id: None,
            player_id: None,
            address,
            state: ClientState::Connecting,
            connected_at: current_timestamp(),
            last_activity: current_timestamp(),
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            user_agent: None,
            protocol_version: crate::network::protocol::PROTOCOL_VERSION.to_string(),
        }));

        let session = Arc::new(RwLock::new(None));
        let (tx, rx) = mpsc::unbounded_channel::<Message>();

        let (reader, writer) = stream.into_split();
        let reader = BufReader::new(reader);

        let receiver_handle = {
            let info_clone = Arc::clone(&info);
            let session_clone = Arc::clone(&session);
            let handler_clone = Arc::clone(&message_handler);
            let tx_clone = tx.clone();

            tokio::spawn(async move {
                Self::handle_incoming_messages(
                    reader,
                    info_clone,
                    session_clone,
                    handler_clone,
                    tx_clone,
                ).await;
            })
        };

        let sender_handle = {
            let info_clone = Arc::clone(&info);

            tokio::spawn(async move {
                Self::handle_outgoing_messages(writer, rx, info_clone).await;
            })
        };

        {
            let mut info_guard = info.write().await;
            info_guard.state = ClientState::Connected;
            info_guard.last_activity = current_timestamp();
        }

        Ok(Self {
            info,
            session,
            sender: tx,
            _receiver_handle: receiver_handle,
            _sender_handle: sender_handle,
        })
    }

    async fn handle_incoming_messages(
        mut reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
        info: Arc<RwLock<ClientInfo>>,
        session: Arc<RwLock<Option<Session>>>,
        handler: Arc<dyn MessageHandler + Send + Sync>,
        sender: mpsc::UnboundedSender<Message>,
    ) {
        let mut buffer = String::new();

        loop {
            buffer.clear();

            match timeout(Duration::from_secs(30), reader.read_line(&mut buffer)).await {
                Ok(Ok(0)) => {
                    // Connection closed
                    break;
                }
                Ok(Ok(bytes_read)) => {
                    // Received a message
                    {
                        let mut info_guard = info.write().await;
                        info_guard.bytes_received += bytes_read as u64;
                        info_guard.messages_received += 1;
                        info_guard.last_activity = current_timestamp();
                    }

                    // Parse a message
                    let line = buffer.trim();
                    if !line.is_empty() {
                        match Message::from_json(line) {
                            Ok(message) => {
                                // Fetch a session info
                                let session_guard = session.read().await;
                                let session_ref = session_guard.as_ref();
                                drop(session_guard);

                                // Pass a process to Meesage Handler
                                let client_info = {
                                    let info_guard = info.read().await;
                                    info_guard.clone()
                                };

                                let response = handler.handle_message(message, client_info, session_ref.cloned()).await;

                                if let Some(response_message) = response {
                                    if sender.send(response_message).is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                // Parse error
                                let error_msg = Message::error(e, None);
                                if sender.send(error_msg).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    // Read error or timeout
                    break;
                }
            }
        }

        // Update disconnection state
        {
            let mut info_guard = info.write().await;
            info_guard.state = ClientState::Disconnected;
        }
    }

    async fn handle_outgoing_messages(
        mut writer: tokio::net::tcp::OwnedWriteHalf,
        mut receiver: mpsc::UnboundedReceiver<Message>,
        info: Arc<RwLock<ClientInfo>>,
    ) {
        while let Some(message) = receiver.recv().await {
            match message.to_json() {
                Ok(json) => {
                    let line = format!("{}\n", json);
                    let bytes = line.as_bytes();

                    match writer.write_all(bytes).await {
                        Ok(()) => {
                            if let Err(_) = writer.flush().await {
                                break;
                            }

                            // Update sent statistics
                            {
                                let mut info_guard = info.write().await;
                                info_guard.bytes_sent += bytes.len() as u64;
                                info_guard.messages_sent += 1;
                                info_guard.last_activity = current_timestamp();
                            }
                        }
                        Err(_) => {
                            // Sending error
                            break;
                        }
                    }
                }
                Err(_) => {
                    // Serialization error
                    continue;
                }
            }
        }
    }

    pub async fn send_message(&self, message: Message) -> ChessResult<()> {
        self.sender.send(message)
            .map_err(|_| ChessServerError::ConnectionLost)?;
        Ok(())
    }

    pub async fn get_info(&self) -> ClientInfo {
        self.info.read().await.clone()
    }

    pub async fn set_session(&self, session: Session) {
        let mut session_guard = self.session.write().await;
        *session_guard = Some(session);

        // Update client info
        let mut info_guard = self.info.write().await;
        if let Some(ref session) = *session_guard {
            info_guard.session_id = Some(session.id.clone());
            info_guard.player_id = Some(session.player_id.clone());
            if session.is_authenticated {
                info_guard.state = ClientState::Authenticated;
            }
        }
    }

    pub async fn get_session(&self) -> Option<Session> {
        self.session.read().await.clone()
    }

    pub async fn set_state(&self, state: ClientState) {
        let mut info_guard = self.info.write().await;
        info_guard.state = state;
        info_guard.last_activity = current_timestamp();
    }

    pub async fn set_user_agent(&self, user_agent: String) {
        let mut info_guard = self.info.write().await;
        info_guard.user_agent = Some(user_agent);
    }

    pub async fn is_connected(&self) -> bool {
        let info_guard = self.info.read().await;
        !matches!(info_guard.state, ClientState::Disconnected)
    }

    pub async fn is_authenticated(&self) -> bool {
        let session_guard = self.session.read().await;
        session_guard.as_ref()
            .map(|s| s.is_authenticated)
            .unwrap_or(false)
    }

    pub async fn disconnect(&self) {
        self.set_state(ClientState::Disconnecting).await;
        // Actual TCP Connection clean up automatically
    }

    pub async fn get_player_id(&self) -> Option<String> {
        let session_guard = self.session.read().await;
        session_guard.as_ref().map(|s| s.player_id.clone())
    }
}

#[async_trait::async_trait]
pub trait MessageHandler {
    async fn handle_message(
        &self,
        message: Message,
        client_info: ClientInfo,
        session: Option<Session>,
    ) -> Option<Message>;
}

#[derive(Debug)]
pub struct ClientManager {
    clients: Arc<RwLock<HashMap<String, Arc<Client>>>>,
    player_clients: Arc<RwLock<HashMap<String, String>>>, // player_id -> client_id
    session_client: Arc<RwLock<HashMap<String, String>>>, // session_id -> client_id
}
