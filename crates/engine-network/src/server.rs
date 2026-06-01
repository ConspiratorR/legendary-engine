//! Game server implementation.

use crate::message::NetworkMessage;
use crate::routing::MessageRouter;
use crate::session::{SessionConfig, SessionManager};
use crate::socket::{SocketError, TcpListener};

/// Errors specific to the game server.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("socket error: {0}")]
    Socket(#[from] SocketError),
    #[error("server is not running")]
    NotRunning,
    #[error("client {0} not found")]
    ClientNotFound(u64),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration for the game server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to.
    pub bind_address: String,
    /// Port to listen on.
    pub port: u16,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// Session management configuration.
    pub session: SessionConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 7777,
            max_connections: 32,
            session: SessionConfig::default(),
        }
    }
}

/// The game server manages client connections, sessions, and message routing.
pub struct GameServer {
    listener: Option<TcpListener>,
    sessions: SessionManager,
    router: MessageRouter,
    config: ServerConfig,
    running: bool,
    incoming: Vec<(u64, NetworkMessage)>,
}

impl GameServer {
    /// Create a new game server with the given configuration.
    pub fn new(config: ServerConfig) -> Self {
        Self {
            listener: None,
            sessions: SessionManager::new(config.session.clone()),
            router: MessageRouter::new(),
            config,
            running: false,
            incoming: Vec::new(),
        }
    }

    /// Start the server, binding to the configured address.
    pub fn start(&mut self) -> Result<(), ServerError> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let listener = TcpListener::bind(&addr)?;
        listener.set_nonblocking(true);
        self.listener = Some(listener);
        self.running = true;
        Ok(())
    }

    /// Stop the server.
    pub fn stop(&mut self) {
        self.running = false;
        self.listener = None;
        self.sessions = SessionManager::new(self.config.session.clone());
        self.router.clear();
        self.incoming.clear();
    }

    /// Check if the server is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Accept new client connections (non-blocking).
    pub fn accept_connections(&mut self) -> Vec<u64> {
        let mut new_clients = Vec::new();
        let listener = match &self.listener {
            Some(l) => l,
            None => return new_clients,
        };

        if self.sessions.client_count() >= self.config.max_connections {
            return new_clients;
        }

        while let Ok((conn, _addr)) = listener.accept() {
            conn.set_nonblocking(true);
            let client_id = self.sessions.add_client(conn);
            new_clients.push(client_id);

            // Send handshake to the new client
            let handshake = NetworkMessage::Handshake {
                client_id: Some(client_id),
                version: "1.0".to_string(),
            };
            self.router.unicast(0, client_id, handshake);

            if self.sessions.client_count() >= self.config.max_connections {
                break;
            }
        }

        new_clients
    }

    /// Read messages from all connected clients (non-blocking).
    pub fn receive_messages(&mut self) -> Vec<(u64, NetworkMessage)> {
        let mut messages = Vec::new();
        let client_ids: Vec<u64> = self.sessions.client_ids();

        for client_id in client_ids {
            if let Some(session) = self.sessions.get_mut(client_id) {
                let mut buf = vec![0u8; 4096];
                match session.connection.receive(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        if let Some(msg) = NetworkMessage::deserialize(&buf) {
                            // Handle heartbeat internally
                            if matches!(msg, NetworkMessage::Heartbeat) {
                                session.heartbeat();
                            } else {
                                messages.push((client_id, msg));
                            }
                        }
                    }
                    Err(SocketError::ConnectionClosed) => {
                        session.state = crate::connection::ConnectionState::Disconnected;
                    }
                    Err(_) => {}
                }
            }
        }

        self.incoming.extend(messages.clone());
        messages
    }

    /// Send all queued outgoing messages to clients.
    pub fn send_messages(&mut self) {
        self.router.route_pending(&self.sessions);
        let client_ids: Vec<u64> = self.sessions.client_ids();

        for client_id in client_ids {
            let messages = self.router.drain_outgoing(client_id);
            for msg in messages {
                let data = msg.serialize();
                if let Some(session) = self.sessions.get_mut(client_id) {
                    let _ = session.connection.send(&data);
                }
            }
        }
    }

    /// Check for timed-out clients and disconnect them.
    pub fn check_timeouts(&mut self) -> Vec<u64> {
        let timed_out = self.sessions.check_timeouts();
        for &client_id in &timed_out {
            self.sessions.remove_client(client_id);
        }
        timed_out
    }

    /// Send a heartbeat to all clients (server-side keepalive).
    pub fn send_heartbeats(&mut self) {
        let client_ids = self.sessions.client_ids();
        for client_id in client_ids {
            self.router.unicast(0, client_id, NetworkMessage::Heartbeat);
        }
    }

    /// Broadcast a message to all connected clients.
    pub fn broadcast(&mut self, message: NetworkMessage) {
        self.router.broadcast(0, message);
    }

    /// Send a message to a specific client.
    pub fn send_to(&mut self, client_id: u64, message: NetworkMessage) {
        self.router.unicast(0, client_id, message);
    }

    /// Send a message to all clients in a group.
    pub fn send_to_group(&mut self, group: &str, message: NetworkMessage) {
        self.router.group_send(0, group, message);
    }

    /// Broadcast a message to all clients except the sender.
    pub fn broadcast_except(&mut self, from: u64, message: NetworkMessage) {
        self.router.broadcast_except(from, message);
    }

    /// Add a client to a group.
    pub fn join_group(&mut self, client_id: u64, group: &str) -> Result<(), ServerError> {
        self.sessions
            .get_mut(client_id)
            .ok_or(ServerError::ClientNotFound(client_id))?
            .join_group(group);
        Ok(())
    }

    /// Remove a client from a group.
    pub fn leave_group(&mut self, client_id: u64, group: &str) -> Result<(), ServerError> {
        self.sessions
            .get_mut(client_id)
            .ok_or(ServerError::ClientNotFound(client_id))?
            .leave_group(group);
        Ok(())
    }

    /// Get the session manager.
    pub fn sessions(&self) -> &SessionManager {
        &self.sessions
    }

    /// Get the message router.
    pub fn router(&self) -> &MessageRouter {
        &self.router
    }

    /// Get the server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get the number of connected clients.
    pub fn client_count(&self) -> usize {
        self.sessions.client_count()
    }

    /// Drain all incoming messages.
    pub fn drain_incoming(&mut self) -> Vec<(u64, NetworkMessage)> {
        std::mem::take(&mut self.incoming)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 7777);
        assert_eq!(config.max_connections, 32);
    }

    #[test]
    fn test_game_server_new() {
        let server = GameServer::new(ServerConfig::default());
        assert!(!server.is_running());
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_game_server_start_stop() {
        let mut server = GameServer::new(ServerConfig {
            port: 0, // let OS pick a port
            ..Default::default()
        });
        assert!(server.start().is_ok());
        assert!(server.is_running());

        server.stop();
        assert!(!server.is_running());
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_server_broadcast() {
        let mut server = GameServer::new(ServerConfig::default());
        server.broadcast(NetworkMessage::Chat {
            sender: "server".to_string(),
            text: "hello".to_string(),
        });
        assert_eq!(server.router.pending_count(), 1);
    }

    #[test]
    fn test_server_send_to() {
        let mut server = GameServer::new(ServerConfig::default());
        server.send_to(
            1,
            NetworkMessage::Chat {
                sender: "server".to_string(),
                text: "private".to_string(),
            },
        );
        assert_eq!(server.router.pending_count(), 1);
    }

    #[test]
    fn test_server_send_to_group() {
        let mut server = GameServer::new(ServerConfig::default());
        server.send_to_group(
            "team1",
            NetworkMessage::Chat {
                sender: "server".to_string(),
                text: "team msg".to_string(),
            },
        );
        assert_eq!(server.router.pending_count(), 1);
    }

    #[test]
    fn test_server_broadcast_except() {
        let mut server = GameServer::new(ServerConfig::default());
        server.broadcast_except(
            1,
            NetworkMessage::Chat {
                sender: "player1".to_string(),
                text: "hello".to_string(),
            },
        );
        assert_eq!(server.router.pending_count(), 1);
    }
}
