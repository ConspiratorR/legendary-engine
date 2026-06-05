//! # engine-network
//!
//! Networking layer for the RustEngine game engine, providing multiplayer
//! support through a client/server architecture with ECS integration.
//!
//! ## Features
//!
//! - **Client/Server** — [`GameServer`] and [`GameClient`] with TCP connections,
//!   heartbeat keepalive, and automatic timeout detection.
//! - **Message Routing** — [`MessageRouter`] supports unicast, broadcast,
//!   broadcast-except, and group messaging.
//! - **Session Management** — [`SessionManager`] tracks connected clients,
//!   groups, and heartbeat state.
//! - **Authoritative Server** — [`AuthoritativeServer`] runs the ECS simulation,
//!   generates snapshots, and broadcasts to clients. [`ClientAuthority`] applies
//!   received snapshots on the client side.
//! - **Snapshots & Interpolation** — [`WorldSnapshot`], [`SnapshotRegistry`],
//!   and [`Interpolator`] for smooth state replication with delta compression.
//! - **Client-Side Prediction** — [`PredictionManager`] stores predicted states
//!   and reconciles with server corrections.
//! - **NAT Traversal** — [`StunClient`] for address discovery, [`HolePuncher`]
//!   for UDP hole punching, and [`P2pConnection`] for peer-to-peer links.
//! - **Matchmaking** — [`RoomManager`], [`Matchmaker`], and [`LobbyManager`]
//!   for room creation, skill-based matching, and pre-game lobbies.
//! - **Reconnection** — [`ReconnectManager`] stores disconnected sessions and
//!   restores them with snapshot recovery.
//! - **ECS Integration** — [`NetworkPlugin`] registers ECS systems for
//!   automatic network processing each tick.
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_network::{
//!     GameServer, ServerConfig, GameClient, ClientConfig,
//!     NetworkMessage, ConnectionState,
//! };
//!
//! // --- Server side ---
//! let mut server = GameServer::new(ServerConfig {
//!     port: 0, // let OS pick a port
//!     ..Default::default()
//! });
//! server.start().expect("server start");
//!
//! server.broadcast(NetworkMessage::Chat {
//!     sender: "server".into(),
//!     text: "welcome!".into(),
//! });
//! assert_eq!(server.router().pending_count(), 1);
//!
//! // --- Client side ---
//! let mut client = GameClient::new(ClientConfig::default());
//! assert!(!client.is_connected());
//! assert_eq!(client.state(), ConnectionState::Disconnected);
//!
//! client.send(NetworkMessage::Chat {
//!     sender: "player1".into(),
//!     text: "hello".into(),
//! });
//! assert_eq!(client.outgoing_count(), 1);
//!
//! // --- Message serialization roundtrip ---
//! let msg = NetworkMessage::Chat {
//!     sender: "alice".into(),
//!     text: "hi".into(),
//! };
//! let bytes = msg.serialize();
//! let restored = NetworkMessage::deserialize(&bytes).unwrap();
//! // restored == msg
//!
//! server.stop();
//! ```

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
