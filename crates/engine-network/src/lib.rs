//! Network module for multiplayer game support.
//!
//! This module provides networking capabilities including:
//! - Network messaging
//! - Client/server architecture
//! - Basic connection management

pub mod message;
pub mod connection;
pub mod plugin;

pub use message::NetworkMessage;
pub use connection::{Connection, ConnectionState};
pub use plugin::NetworkPlugin;
