//! Game client implementation.

use crate::connection::ConnectionState;
use crate::message::NetworkMessage;
use crate::socket::{SocketError, TcpConnection};
use std::collections::VecDeque;
use std::time::Instant;

/// Errors specific to the game client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("socket error: {0}")]
    Socket(#[from] SocketError),
    #[error("not connected")]
    NotConnected,
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("connection timeout")]
    ConnectionTimeout,
}

/// Configuration for the game client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server address to connect to.
    pub server_address: String,
    /// Server port.
    pub port: u16,
    /// Connection timeout in milliseconds.
    pub connect_timeout_ms: u64,
    /// Reconnect attempts (0 = no reconnect).
    pub max_reconnect_attempts: u32,
    /// Delay between reconnect attempts in milliseconds.
    pub reconnect_delay_ms: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1".to_string(),
            port: 7777,
            connect_timeout_ms: 5000,
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
        }
    }
}

/// Client state for reconnection tracking.
#[derive(Debug)]
struct ReconnectState {
    attempts: u32,
    max_attempts: u32,
    delay_ms: u64,
    last_attempt: Instant,
}

/// The game client connects to a server and exchanges messages.
pub struct GameClient {
    connection: Option<TcpConnection>,
    client_id: Option<u64>,
    state: ConnectionState,
    config: ClientConfig,
    incoming: VecDeque<NetworkMessage>,
    outgoing: VecDeque<NetworkMessage>,
    reconnect: Option<ReconnectState>,
    last_heartbeat: Instant,
}

impl GameClient {
    /// Create a new game client with the given configuration.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            connection: None,
            client_id: None,
            state: ConnectionState::Disconnected,
            config,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            reconnect: None,
            last_heartbeat: Instant::now(),
        }
    }

    /// Connect to the server.
    pub fn connect(&mut self) -> Result<(), ClientError> {
        let addr = format!("{}:{}", self.config.server_address, self.config.port);
        self.state = ConnectionState::Connecting;

        match TcpConnection::connect(&addr) {
            Ok(conn) => {
                conn.set_nonblocking(true);
                self.connection = Some(conn);
                self.state = ConnectionState::Connected;
                self.reconnect = None;
                Ok(())
            }
            Err(e) => {
                self.state = ConnectionState::Disconnected;
                if self.config.max_reconnect_attempts > 0 {
                    self.reconnect = Some(ReconnectState {
                        attempts: 0,
                        max_attempts: self.config.max_reconnect_attempts,
                        delay_ms: self.config.reconnect_delay_ms,
                        last_attempt: Instant::now(),
                    });
                }
                Err(ClientError::Socket(e))
            }
        }
    }

    /// Disconnect from the server.
    pub fn disconnect(&mut self) {
        if let Some(ref conn) = self.connection {
            let msg = NetworkMessage::Disconnect {
                reason: "client disconnect".to_string(),
            };
            let _ = conn.send(&msg.serialize());
        }
        self.connection = None;
        self.client_id = None;
        self.state = ConnectionState::Disconnected;
        self.reconnect = None;
        self.incoming.clear();
        self.outgoing.clear();
    }

    /// Attempt to reconnect to the server.
    pub fn try_reconnect(&mut self) -> Result<bool, ClientError> {
        let reconnect = match self.reconnect.as_mut() {
            Some(r) => r,
            None => return Ok(false),
        };

        if reconnect.attempts >= reconnect.max_attempts {
            self.reconnect = None;
            return Ok(false);
        }

        if reconnect.last_attempt.elapsed().as_millis() < reconnect.delay_ms as u128 {
            return Ok(false);
        }

        reconnect.attempts += 1;
        reconnect.last_attempt = Instant::now();
        self.state = ConnectionState::Reconnecting;

        match self.connect() {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Queue a message to send to the server.
    pub fn send(&mut self, message: NetworkMessage) {
        self.outgoing.push_back(message);
    }

    /// Receive messages from the server (non-blocking).
    pub fn receive(&mut self) -> Vec<NetworkMessage> {
        let conn = match &self.connection {
            Some(c) => c,
            None => return Vec::new(),
        };

        let mut messages = Vec::new();
        let mut buf = vec![0u8; 4096];

        loop {
            match conn.receive(&mut buf) {
                Ok(n) => {
                    if let Some(msg) = NetworkMessage::deserialize(&buf[..n]) {
                        match &msg {
                            NetworkMessage::Handshake { client_id, .. } => {
                                self.client_id = *client_id;
                            }
                            NetworkMessage::Heartbeat => {
                                self.last_heartbeat = Instant::now();
                                continue;
                            }
                            _ => {}
                        }
                        messages.push(msg);
                    }
                }
                Err(SocketError::ConnectionClosed) => {
                    self.state = ConnectionState::Disconnected;
                    if self.config.max_reconnect_attempts > 0 {
                        self.reconnect = Some(ReconnectState {
                            attempts: 0,
                            max_attempts: self.config.max_reconnect_attempts,
                            delay_ms: self.config.reconnect_delay_ms,
                            last_attempt: Instant::now(),
                        });
                    }
                    break;
                }
                Err(_) => break,
            }
        }

        self.incoming.extend(messages.clone());
        messages
    }

    /// Send all queued outgoing messages.
    pub fn flush_outgoing(&mut self) {
        let conn = match &self.connection {
            Some(c) => c,
            None => return,
        };

        while let Some(msg) = self.outgoing.pop_front() {
            let data = msg.serialize();
            let _ = conn.send(&data);
        }
    }

    /// Send a heartbeat to the server.
    pub fn send_heartbeat(&mut self) {
        self.outgoing.push_back(NetworkMessage::Heartbeat);
    }

    /// Check if the client is connected.
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Get the client ID assigned by the server.
    pub fn client_id(&self) -> Option<u64> {
        self.client_id
    }

    /// Get the server address.
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.config.server_address, self.config.port)
    }

    /// Drain all incoming messages.
    pub fn drain_incoming(&mut self) -> Vec<NetworkMessage> {
        self.incoming.drain(..).collect()
    }

    /// Get the number of queued outgoing messages.
    pub fn outgoing_count(&self) -> usize {
        self.outgoing.len()
    }

    /// Get the client configuration.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Get the time since the last heartbeat.
    pub fn time_since_last_heartbeat(&self) -> std::time::Duration {
        self.last_heartbeat.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.server_address, "127.0.0.1");
        assert_eq!(config.port, 7777);
        assert_eq!(config.connect_timeout_ms, 5000);
        assert_eq!(config.max_reconnect_attempts, 3);
        assert_eq!(config.reconnect_delay_ms, 1000);
    }

    #[test]
    fn test_game_client_new() {
        let client = GameClient::new(ClientConfig::default());
        assert!(!client.is_connected());
        assert_eq!(client.state(), ConnectionState::Disconnected);
        assert!(client.client_id().is_none());
        assert_eq!(client.outgoing_count(), 0);
    }

    #[test]
    fn test_game_client_send_queue() {
        let mut client = GameClient::new(ClientConfig::default());
        client.send(NetworkMessage::Chat {
            sender: "test".to_string(),
            text: "hello".to_string(),
        });
        assert_eq!(client.outgoing_count(), 1);
    }

    #[test]
    fn test_game_client_disconnect() {
        let mut client = GameClient::new(ClientConfig::default());
        client.send(NetworkMessage::Chat {
            sender: "test".to_string(),
            text: "hello".to_string(),
        });
        client.disconnect();
        assert_eq!(client.outgoing_count(), 0);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_game_client_server_address() {
        let client = GameClient::new(ClientConfig::default());
        assert_eq!(client.server_address(), "127.0.0.1:7777");
    }

    #[test]
    fn test_game_client_drain_incoming() {
        let mut client = GameClient::new(ClientConfig::default());
        let messages = client.drain_incoming();
        assert!(messages.is_empty());
    }
}
