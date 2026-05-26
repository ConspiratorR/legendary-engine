//! Network module for multiplayer game support.
//!
//! This module provides networking capabilities including:
//! - Network messaging
//! - Client/server architecture
//! - Basic connection management

pub mod connection;
pub mod message;
pub mod plugin;

pub use connection::{Connection, ConnectionState};
pub use message::NetworkMessage;
pub use plugin::NetworkPlugin;
