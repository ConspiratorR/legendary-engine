//! Message routing for the game server.

use crate::message::NetworkMessage;
use crate::session::SessionManager;
use std::collections::{HashMap, VecDeque};

/// Routing target for a network message.
#[derive(Debug, Clone)]
pub enum RoutingTarget {
    /// Send to a specific client.
    Client(u64),
    /// Send to all connected clients.
    Broadcast,
    /// Send to all clients in a group.
    Group(String),
    /// Send to all clients except the sender.
    BroadcastExcept(u64),
}

/// A message with routing information.
#[derive(Debug, Clone)]
pub struct RoutedMessage {
    /// The source client ID (0 for server).
    pub from: u64,
    /// The routing target.
    pub target: RoutingTarget,
    /// The message payload.
    pub message: NetworkMessage,
}

/// Message router that handles broadcast, unicast, and group messaging.
pub struct MessageRouter {
    /// Queue of messages to be routed.
    pending: VecDeque<RoutedMessage>,
    /// Per-client outgoing message queues.
    outgoing: HashMap<u64, VecDeque<NetworkMessage>>,
}

impl MessageRouter {
    /// Create a new message router.
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            outgoing: HashMap::new(),
        }
    }

    /// Queue a message for routing.
    pub fn send(&mut self, from: u64, target: RoutingTarget, message: NetworkMessage) {
        self.pending.push_back(RoutedMessage {
            from,
            target,
            message,
        });
    }

    /// Send a message to a specific client.
    pub fn unicast(&mut self, from: u64, to: u64, message: NetworkMessage) {
        self.send(from, RoutingTarget::Client(to), message);
    }

    /// Send a message to all connected clients.
    pub fn broadcast(&mut self, from: u64, message: NetworkMessage) {
        self.send(from, RoutingTarget::Broadcast, message);
    }

    /// Send a message to all clients except the sender.
    pub fn broadcast_except(&mut self, from: u64, message: NetworkMessage) {
        self.send(from, RoutingTarget::BroadcastExcept(from), message);
    }

    /// Send a message to all clients in a group.
    pub fn group_send(&mut self, from: u64, group: &str, message: NetworkMessage) {
        self.send(
            from,
            RoutingTarget::Group(group.to_string()),
            message,
        );
    }

    /// Process all pending messages and distribute to per-client queues.
    pub fn route_pending(&mut self, sessions: &SessionManager) {
        while let Some(routed) = self.pending.pop_front() {
            let targets = match &routed.target {
                RoutingTarget::Client(id) => vec![*id],
                RoutingTarget::Broadcast => sessions.client_ids(),
                RoutingTarget::BroadcastExcept(exclude) => sessions
                    .client_ids()
                    .into_iter()
                    .filter(|id| id != exclude)
                    .collect(),
                RoutingTarget::Group(group) => sessions.clients_in_group(group),
            };

            for client_id in targets {
                self.outgoing
                    .entry(client_id)
                    .or_default()
                    .push_back(routed.message.clone());
            }
        }
    }

    /// Drain outgoing messages for a specific client.
    pub fn drain_outgoing(&mut self, client_id: u64) -> Vec<NetworkMessage> {
        self.outgoing
            .get_mut(&client_id)
            .map(|q| q.drain(..).collect())
            .unwrap_or_default()
    }

    /// Get the number of pending messages.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get the number of queued outgoing messages for a client.
    pub fn outgoing_count(&self, client_id: u64) -> usize {
        self.outgoing.get(&client_id).map(|q| q.len()).unwrap_or(0)
    }

    /// Clear all queues.
    pub fn clear(&mut self) {
        self.pending.clear();
        self.outgoing.clear();
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_router_new() {
        let router = MessageRouter::new();
        assert_eq!(router.pending_count(), 0);
    }

    #[test]
    fn test_routing_target_variants() {
        let target = RoutingTarget::Client(1);
        assert!(matches!(target, RoutingTarget::Client(1)));

        let target = RoutingTarget::Broadcast;
        assert!(matches!(target, RoutingTarget::Broadcast));

        let target = RoutingTarget::Group("test".to_string());
        assert!(matches!(target, RoutingTarget::Group(_)));

        let target = RoutingTarget::BroadcastExcept(5);
        assert!(matches!(target, RoutingTarget::BroadcastExcept(5)));
    }

    #[test]
    fn test_routed_message_creation() {
        let msg = RoutedMessage {
            from: 1,
            target: RoutingTarget::Broadcast,
            message: NetworkMessage::Chat {
                sender: "test".to_string(),
                text: "hello".to_string(),
            },
        };
        assert_eq!(msg.from, 1);
    }

    #[test]
    fn test_message_router_default() {
        let router = MessageRouter::default();
        assert_eq!(router.pending_count(), 0);
    }

    #[test]
    fn test_message_router_clear() {
        let mut router = MessageRouter::new();
        router.send(
            0,
            RoutingTarget::Broadcast,
            NetworkMessage::Chat {
                sender: "server".to_string(),
                text: "test".to_string(),
            },
        );
        assert_eq!(router.pending_count(), 1);
        router.clear();
        assert_eq!(router.pending_count(), 0);
    }
}
