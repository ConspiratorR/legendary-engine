//! Connection management for networking.
use crate::message::NetworkMessage;
use std::collections::VecDeque;

/// State of a network connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Disconnected from remote.
    Disconnected,
    /// Trying to connect.
    Connecting,
    /// Connection established.
    Connected,
    /// Reconnecting after a disconnect.
    Reconnecting,
}

/// A network connection.
#[derive(Debug, Clone)]
pub struct Connection {
    /// Unique ID for this connection.
    pub id: u64,
    /// Current state of the connection.
    pub state: ConnectionState,
    /// Round trip time in milliseconds.
    pub rtt: f32,
    /// Packet loss percentage.
    pub packet_loss: f32,
    /// Incoming messages waiting to be processed.
    pub incoming: VecDeque<NetworkMessage>,
    /// Outgoing messages waiting to be sent.
    pub outgoing: VecDeque<NetworkMessage>,
}

impl Connection {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            state: ConnectionState::Disconnected,
            rtt: 0.0,
            packet_loss: 0.0,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
        }
    }

    pub fn send(&mut self, message: NetworkMessage) {
        self.outgoing.push_back(message);
    }

    pub fn receive(&mut self, message: NetworkMessage) {
        self.incoming.push_back(message);
    }

    pub fn take_incoming(&mut self) -> VecDeque<NetworkMessage> {
        std::mem::take(&mut self.incoming)
    }

    pub fn take_outgoing(&mut self) -> VecDeque<NetworkMessage> {
        std::mem::take(&mut self.outgoing)
    }

    pub fn connect(&mut self) {
        self.state = ConnectionState::Connecting;
    }

    pub fn disconnect(&mut self, reason: &str) {
        self.state = ConnectionState::Disconnected;
        self.send(NetworkMessage::Disconnect {
            reason: reason.to_string(),
        });
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new(0)
    }
}
