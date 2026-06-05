//! Network module for multiplayer game support.
//!
//! This module provides networking capabilities including:
//! - Network messaging
//! - Client/server architecture
//! - Basic connection management
//! - Session management with heartbeat
//! - Message routing (broadcast, unicast, group)
//! - ECS system integration
//! - State snapshot interpolation and client-side prediction
//! - NAT traversal (STUN, UDP hole punching, P2P)
//! - Matchmaking (rooms, lobbies, player matching)
//! - Reconnection v2 (session resumption, snapshot recovery)

pub mod error;
pub use error::NetworkError;

pub mod authority;
pub mod client;
pub mod connection;
pub mod interpolation;
pub mod matchmaking;
pub mod message;
pub mod nat;
pub mod plugin;
pub mod reconnect;
pub mod routing;
pub mod server;
pub mod session;
pub mod snapshot;
pub mod socket;

pub use authority::{AuthoritativeServer, AuthorityConfig, ClientAuthority, PendingInput};
pub use client::{ClientConfig, ClientError, GameClient};
pub use connection::{Connection, ConnectionState};
pub use interpolation::{Interpolator, PredictionManager, SnapshotBuffer};
pub use matchmaking::{
    Lobby, LobbyManager, LobbyState, Matchmaker, MatchmakingError, QueuedPlayer, Room, RoomManager,
    RoomState,
};
pub use message::{EntityComponentData, NetworkMessage, RoomInfo};
pub use nat::{HolePuncher, NatError, P2pConnection, RendezvousServer, StunClient, StunResponse};
pub use plugin::{NetworkConfig, NetworkPlugin};
pub use reconnect::{ReconnectConfig, ReconnectManager, ReconnectToken, compute_reconnect_delta};
pub use routing::{MessageRouter, RoutedMessage, RoutingTarget};
pub use server::{GameServer, ServerConfig, ServerError};
pub use session::{ClientSession, SessionConfig, SessionManager};
pub use snapshot::{NetworkSync, SnapshotRegistry, WorldSnapshot};
pub use socket::{
    NetworkConfig as SocketConfig, NetworkPacket, PacketQueue, Protocol, SocketError,
    TcpConnection, TcpListener, UdpSocket,
};
