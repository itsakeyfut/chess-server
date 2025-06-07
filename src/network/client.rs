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
                                let session_ref = {
                                    let session_guard = session.read().await;
                                    session_guard.as_ref().cloned()
                                };

                                // Pass a process to Meesage Handler
                                let client_info = {
                                    let info_guard = info.read().await;
                                    info_guard.clone()
                                };

                                let response = handler.handle_message(message, client_info, session_ref).await;

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
    session_clients: Arc<RwLock<HashMap<String, String>>>, // session_id -> client_id
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            player_clients: Arc::new(RwLock::new(HashMap::new())),
            session_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_client(&self, client: Arc<Client>) {
        let client_id = {
            let info = client.get_info().await;
            info.id.clone()
        };

        let mut clients_guard = self.clients.write().await;
        clients_guard.insert(client_id, client);
    }

    pub async fn remove_client(&self, client_id: &str) -> Option<Arc<Client>> {
        let mut clients_guard = self.clients.write().await;
        let client = clients_guard.remove(client_id)?;

        let client_info = client.get_info().await;

        if let Some(ref player_id) = client_info.player_id {
            let mut player_clients_guard = self.player_clients.write().await;
            player_clients_guard.remove(player_id);
        }

        if let Some(ref session_id) = client_info.session_id {
            let mut session_clients_guard = self.session_clients.write().await;
            session_clients_guard.remove(session_id);
        }

        Some(client)
    }

    pub async fn get_client(&self, client_id: &str) -> Option<Arc<Client>> {
        let client_guard = self.clients.read().await;
        client_guard.get(client_id).cloned()
    }

    pub async fn get_client_by_player(&self, player_id: &str) -> Option<Arc<Client>> {
        let player_clients_guard = self.player_clients.read().await;
        let client_id = player_clients_guard.get(player_id)?;

        let clients_guard = self.clients.read().await;
        clients_guard.get(client_id).cloned()
    }

    pub async fn get_client_by_session(&self, session_id: &str) -> Option<Arc<Client>> {
        let session_clients_guard = self.session_clients.read().await;
        let client_id = session_clients_guard.get(session_id)?;

        let clients_guard = self.clients.read().await;
        clients_guard.get(client_id).cloned()
    }

    pub async fn associate_player(&self, client_id: &str, player_id: String) -> ChessResult<()> {
        // Check if clients exist
        {
            let clients_guard = self.clients.read().await;
            if !clients_guard.contains_key(client_id) {
                return Err(ChessServerError::PlayerNotFound {
                    player_id: client_id.to_string(),
                });
            }
        }

        let mut player_clients_guard = self.player_clients.write().await;
        player_clients_guard.insert(player_id, client_id.to_string());

        Ok(())
    }

    pub async fn associate_session(&self, client_id: &str, session_id: String) -> ChessResult<()> {
        // Check if clients exist
        {
            let clients_guard = self.clients.read().await;
            if !clients_guard.contains_key(client_id) {
                return Err(ChessServerError::PlayerNotFound {
                    player_id: client_id.to_string(),
                });
            }
        }

        let mut session_clients_guard = self.session_clients.write().await;
        session_clients_guard.insert(session_id, client_id.to_string());

        Ok(())
    }

    pub async fn broadcast_message(&self, message: Message) -> usize {
        let clients_guard = self.clients.read().await;
        let mut sent_count = 0;

        for client in clients_guard.values() {
            if client.send_message(message.clone()).await.is_ok() {
                sent_count += 1;
            }
        }

        sent_count
    }

    pub async fn broadcast_to_authenticated(&self, message: Message) -> usize {
        let clients_guard = self.clients.read().await;
        let mut sent_count = 0;

        for client in clients_guard.values() {
            if client.is_authenticated().await {
                if client.send_message(message.clone()).await.is_ok() {
                    sent_count += 1;
                }
            }
        }

        sent_count
    }

    pub async fn send_to_player(&self, player_id: &str, message: Message) -> ChessResult<()> {
        let client = self.get_client_by_player(player_id).await
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

        client.send_message(message).await
    }

    pub async fn send_to_players(&self, player_ids: &[String], message: Message) -> usize {
        let mut sent_count = 0;

        for player_id in player_ids {
            if let Some(client) = self.get_client_by_player(player_id).await {
                if client.send_message(message.clone()).await.is_ok() {
                    sent_count += 1;
                }
            } 
        }

        sent_count
    }

    pub async fn disconnect_client(&self, client_id: &str) -> ChessResult<()> {
        let client = self.get_client(client_id).await
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: client_id.to_string(),
            })?;

        client.disconnect().await;
        Ok(())
    }

    pub async fn disconnect_player(&self, player_id: &str) -> ChessResult<()> {
        let client = self.get_client_by_player(player_id).await
            .ok_or_else(|| ChessServerError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

        client.disconnect().await;
        Ok(())
    }

    pub async fn get_connected_clients(&self) -> Vec<Arc<Client>> {
        let clients_guard = self.clients.read().await;
        let mut connected = Vec::new();

        for client in clients_guard.values() {
            if client.is_connected().await {
                connected.push(client.clone());
            }
        }

        connected
    }

    pub async fn get_authenticated_clients(&self) -> Vec<Arc<Client>> {
        let clients_guard = self.clients.read().await;
        let mut authenticated = Vec::new();

        for client in clients_guard.values() {
            if client.is_authenticated().await {
                authenticated.push(client.clone());
            }
        }

        authenticated
    }

    pub async fn cleanup_disconnected_clinets(&self) -> usize {
        let mut disconnected_ids = Vec::new();

        {
            let clients_guard = self.clients.read().await;
            for (client_id, client) in clients_guard.iter() {
                if !client.is_connected().await {
                    disconnected_ids.push(client_id.clone());
                }
            }
        }

        let cnt = disconnected_ids.len();
        for client_id in disconnected_ids {
            self.remove_client(&client_id).await;
        }

        cnt
    }

    pub async fn get_client_count(&self) -> usize {
        let clients_guard = self.clients.read().await;
        clients_guard.len()
    }

    pub async fn get_authenticated_client_count(&self) -> usize {
        let clients_guard = self.clients.read().await;
        let mut cnt = 0;

        for client in clients_guard.values() {
            if client.is_authenticated().await {
                cnt += 1;
            }
        }

        cnt
    }

    pub async fn get_client_statistics(&self) -> ClientStatistics {
        let clients_guard = self.clients.read().await;
        let mut stats = ClientStatistics::default();

        stats.total_clients = clients_guard.len();

        for client in clients_guard.values() {
            let info = client.get_info().await;

            stats.total_bytes_sent += info.bytes_sent;
            stats.total_bytes_received += info.bytes_received;
            stats.total_messages_sent += info.messages_sent as u64;
            stats.total_messages_received += info.messages_received as u64;

            match info.state {
                ClientState::Connected => stats.connected_clients += 1,
                ClientState::Authenticated => stats.authenticated_clients += 1,
                ClientState::InGame => stats.in_game_clients += 1,
                ClientState::Disconnected => stats.disconnected_clients += 1,
                _ => {}
            }

            let session_duration = current_timestamp() - info.connected_at;
            stats.total_session_duration += session_duration;
        }

        if stats.total_clients > 0 {
            stats.average_session_duration = stats.total_session_duration / stats.total_clients as u64;
        }

        stats
    }

    pub async fn get_clients_by_state(&self, state: ClientState) -> Vec<Arc<Client>> {
        let clients_guard = self.clients.read().await;
        let mut matching_clients = Vec::new();

        for client in clients_guard.values() {
            let info = client.get_info().await;
            if info.state == state {
                matching_clients.push(client.clone());
            }
        }

        matching_clients
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClientStatistics {
    pub total_clients: usize,
    pub connected_clients: usize,
    pub authenticated_clients: usize,
    pub in_game_clients: usize,
    pub disconnected_clients: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_session_duration: u64,
    pub average_session_duration: u64,
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::net::{TcpListener, TcpStream};

    struct TestMessageHandler;

    #[async_trait::async_trait]
    impl MessageHandler for TestMessageHandler {
        async fn handle_message(
            &self,
            message: Message,
            _client_info: ClientInfo,
            _session: Option<Session>,
        ) -> Option<Message> {
            // エコーハンドラー
            match message.message_type {
                MessageType::Ping => Some(Message::new(MessageType::Pong)),
                _ => None,
            }
        }
    }

    async fn create_test_connection() -> (TcpStream, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        let connect_task = TcpStream::connect(addr);
        let accept_task = listener.accept();
        
        let (client_stream, (server_stream, client_addr)) = 
            tokio::join!(connect_task, accept_task);
        
        (server_stream.unwrap(), client_addr)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let (stream, addr) = create_test_connection().await;
        let handler = Arc::new(TestMessageHandler);
        
        let client = Client::new(stream, addr, handler).await.unwrap();
        let info = client.get_info().await;
        
        assert_eq!(info.address, addr);
        assert_eq!(info.state, ClientState::Connected);
        assert!(client.is_connected().await);
    }

    #[tokio::test]
    async fn test_client_manager() {
        let manager = ClientManager::new();
        let (stream, addr) = create_test_connection().await;
        let handler = Arc::new(TestMessageHandler);
        
        let client = Arc::new(Client::new(stream, addr, handler).await.unwrap());
        let client_id = client.get_info().await.id.clone();
        
        manager.add_client(client.clone()).await;
        
        assert_eq!(manager.get_client_count().await, 1);
        assert!(manager.get_client(&client_id).await.is_some());
        
        manager.remove_client(&client_id).await;
        assert_eq!(manager.get_client_count().await, 0);
    }

    #[tokio::test]
    async fn test_player_association() {
        let manager = ClientManager::new();
        let (stream, addr) = create_test_connection().await;
        let handler = Arc::new(TestMessageHandler);
        
        let client = Arc::new(Client::new(stream, addr, handler).await.unwrap());
        let client_id = client.get_info().await.id.clone();
        
        manager.add_client(client.clone()).await;
        manager.associate_player(&client_id, "player123".to_string()).await.unwrap();
        
        let found_client = manager.get_client_by_player("player123").await;
        assert!(found_client.is_some());
        assert_eq!(found_client.unwrap().get_info().await.id, client_id);
    }
}