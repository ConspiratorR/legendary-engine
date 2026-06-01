//! Network module for multiplayer game support.
//!
//! This module provides networking capabilities including:
//! - Network messaging
//! - Client/server architecture
//! - Basic connection management
//! - Session management with heartbeat
//! - Message routing (broadcast, unicast, group)
//! - ECS system integration

pub mod client;
pub mod connection;
pub mod message;
pub mod plugin;
pub mod routing;
pub mod server;
pub mod session;
pub mod socket;

pub use client::{ClientConfig, ClientError, GameClient};
pub use connection::{Connection, ConnectionState};
pub use message::NetworkMessage;
pub use plugin::{NetworkConfig, NetworkPlugin};
pub use routing::{MessageRouter, RoutedMessage, RoutingTarget};
pub use server::{GameServer, ServerConfig, ServerError};
pub use session::{ClientSession, SessionConfig, SessionManager};
pub use socket::{
    NetworkConfig as SocketConfig, NetworkPacket, PacketQueue, Protocol, SocketError,
    TcpConnection, TcpListener, UdpSocket,
};
