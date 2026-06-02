//! Reconnection v2: session resumption, snapshot recovery, and incremental sync.
//!
//! Provides mechanisms for clients to reconnect to a server after disconnection,
//! resuming their session with minimal state loss. The server stores per-client
//! snapshots and computes deltas for efficient reconnection.

use crate::message::{EntityComponentData, NetworkMessage};
use crate::snapshot::WorldSnapshot;
use std::collections::HashMap;
use std::time::Instant;

/// A token issued to a client on first connection, required for reconnection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReconnectToken(pub u64);

impl ReconnectToken {
    /// Generate a random reconnect token.
    pub fn generate() -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut hasher = DefaultHasher::new();
        Instant::now().elapsed().as_nanos().hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        COUNTER.fetch_add(1, Ordering::Relaxed).hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// Stores a disconnected client's session for potential reconnection.
#[derive(Debug)]
pub struct DisconnectedSession {
    /// Client ID.
    pub client_id: u64,
    /// Reconnect token issued to this client.
    pub token: ReconnectToken,
    /// Last snapshot received by the client before disconnection.
    pub last_snapshot: WorldSnapshot,
    /// Last tick the client acknowledged.
    pub last_tick: u64,
    /// When the client disconnected.
    pub disconnected_at: Instant,
    /// Pending deltas accumulated while client was disconnected.
    pub pending_deltas: Vec<(u64, EntityComponentData, Vec<u32>)>,
}

/// Configuration for session resumption.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// How long to keep disconnected sessions (seconds).
    pub session_timeout_secs: u64,
    /// Maximum pending deltas to store per disconnected client.
    pub max_pending_deltas: usize,
    /// Threshold for sending full snapshot vs delta (tick gap).
    pub full_snapshot_threshold: u64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            session_timeout_secs: 60,
            max_pending_deltas: 300,
            full_snapshot_threshold: 60,
        }
    }
}

/// Manages disconnected sessions and handles reconnection logic.
#[derive(Debug)]
pub struct ReconnectManager {
    config: ReconnectConfig,
    disconnected: HashMap<u64, DisconnectedSession>,
    tokens: HashMap<u64, ReconnectToken>,
}

impl ReconnectManager {
    /// Create a new reconnect manager.
    pub fn new(config: ReconnectConfig) -> Self {
        Self {
            config,
            disconnected: HashMap::new(),
            tokens: HashMap::new(),
        }
    }

    /// Issue a reconnect token for a client.
    pub fn issue_token(&mut self, client_id: u64) -> ReconnectToken {
        let token = ReconnectToken::generate();
        self.tokens.insert(client_id, token);
        token
    }

    /// Get the token for a client.
    pub fn get_token(&self, client_id: u64) -> Option<ReconnectToken> {
        self.tokens.get(&client_id).copied()
    }

    /// Store a disconnected client's session.
    pub fn store_session(
        &mut self,
        client_id: u64,
        token: ReconnectToken,
        last_snapshot: WorldSnapshot,
        last_tick: u64,
    ) {
        let session = DisconnectedSession {
            client_id,
            token,
            last_snapshot,
            last_tick,
            disconnected_at: Instant::now(),
            pending_deltas: Vec::new(),
        };
        self.disconnected.insert(client_id, session);
    }

    /// Add a delta snapshot for a disconnected client.
    pub fn push_delta(
        &mut self,
        client_id: u64,
        tick: u64,
        changed: EntityComponentData,
        despawned: Vec<u32>,
    ) {
        if let Some(session) = self.disconnected.get_mut(&client_id)
            && session.pending_deltas.len() < self.config.max_pending_deltas
        {
            session.pending_deltas.push((tick, changed, despawned));
        }
    }

    /// Attempt to reconnect a client. Returns the reconnection message if successful.
    pub fn try_reconnect(
        &mut self,
        client_id: u64,
        token: ReconnectToken,
        last_tick: u64,
        current_tick: u64,
    ) -> Option<NetworkMessage> {
        let session = self.disconnected.remove(&client_id)?;
        if session.token != token {
            self.disconnected.insert(client_id, session);
            return None;
        }

        let tick_gap = current_tick.saturating_sub(last_tick);

        if tick_gap > self.config.full_snapshot_threshold {
            // Send full snapshot
            let (tick, entities, despawned) = session.last_snapshot.to_wire_format();
            Some(NetworkMessage::ReconnectSnapshot {
                client_id,
                tick,
                entities,
                despawned,
                missed_ticks: tick_gap,
            })
        } else {
            // Compute delta from last known tick
            let mut all_changed = Vec::new();
            let mut all_despawned = Vec::new();
            for (delta_tick, changed, despawned) in &session.pending_deltas {
                if *delta_tick > last_tick {
                    all_changed.extend(changed.iter().cloned());
                    all_despawned.extend(despawned.iter().copied());
                }
            }
            let (tick, entities, despawned) = session.last_snapshot.to_wire_format();
            if all_changed.is_empty() {
                Some(NetworkMessage::ReconnectSnapshot {
                    client_id,
                    tick,
                    entities,
                    despawned,
                    missed_ticks: tick_gap,
                })
            } else {
                Some(NetworkMessage::ReconnectSnapshot {
                    client_id,
                    tick: current_tick,
                    entities: all_changed,
                    despawned: all_despawned,
                    missed_ticks: tick_gap,
                })
            }
        }
    }

    /// Remove expired disconnected sessions.
    pub fn cleanup_expired(&mut self) -> Vec<u64> {
        let timeout = std::time::Duration::from_secs(self.config.session_timeout_secs);
        let expired: Vec<u64> = self
            .disconnected
            .iter()
            .filter(|(_, s)| s.disconnected_at.elapsed() > timeout)
            .map(|(&id, _)| id)
            .collect();
        for id in &expired {
            self.disconnected.remove(id);
            self.tokens.remove(id);
        }
        expired
    }

    /// Check if a client has a stored disconnected session.
    pub fn has_session(&self, client_id: u64) -> bool {
        self.disconnected.contains_key(&client_id)
    }

    /// Get the number of disconnected sessions being tracked.
    pub fn disconnected_count(&self) -> usize {
        self.disconnected.len()
    }

    /// Get the configuration.
    pub fn config(&self) -> &ReconnectConfig {
        &self.config
    }
}

impl Default for ReconnectManager {
    fn default() -> Self {
        Self::new(ReconnectConfig::default())
    }
}

/// Compute a delta snapshot for reconnection from the client's last known tick.
///
/// Returns the delta between the client's last snapshot and the current world state.
pub fn compute_reconnect_delta(
    last_snapshot: &WorldSnapshot,
    current_snapshot: &WorldSnapshot,
) -> (EntityComponentData, Vec<u32>) {
    current_snapshot.delta_from(last_snapshot)
}

/// Generate a `ReconnectSnapshot` message from a world snapshot.
pub fn make_reconnect_snapshot(
    client_id: u64,
    snapshot: &WorldSnapshot,
    missed_ticks: u64,
) -> NetworkMessage {
    let (tick, entities, despawned) = snapshot.to_wire_format();
    NetworkMessage::ReconnectSnapshot {
        client_id,
        tick,
        entities,
        despawned,
        missed_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(tick: u64) -> WorldSnapshot {
        let mut snap = WorldSnapshot::new(tick);
        snap.add_component(0, 1, vec![1, 2, 3]);
        snap.add_component(1, 1, vec![4, 5, 6]);
        snap
    }

    #[test]
    fn test_reconnect_token_generate() {
        let t1 = ReconnectToken::generate();
        let t2 = ReconnectToken::generate();
        // Tokens should be different (probabilistically)
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_reconnect_manager_issue_token() {
        let mut mgr = ReconnectManager::default();
        let token = mgr.issue_token(42);
        assert_eq!(mgr.get_token(42), Some(token));
        assert_eq!(mgr.get_token(99), None);
    }

    #[test]
    fn test_reconnect_manager_store_and_retrieve() {
        let mut mgr = ReconnectManager::default();
        let token = mgr.issue_token(1);
        let snapshot = make_snapshot(100);

        mgr.store_session(1, token, snapshot, 100);
        assert!(mgr.has_session(1));
        assert_eq!(mgr.disconnected_count(), 1);
    }

    #[test]
    fn test_reconnect_manager_try_reconnect_success() {
        let mut mgr = ReconnectManager::default();
        let token = mgr.issue_token(1);
        let snapshot = make_snapshot(100);

        mgr.store_session(1, token, snapshot, 100);

        let msg = mgr.try_reconnect(1, token, 100, 105);
        assert!(msg.is_some());
        match msg.unwrap() {
            NetworkMessage::ReconnectSnapshot {
                client_id,
                missed_ticks,
                ..
            } => {
                assert_eq!(client_id, 1);
                assert_eq!(missed_ticks, 5);
            }
            _ => panic!("expected ReconnectSnapshot"),
        }
    }

    #[test]
    fn test_reconnect_manager_try_reconnect_wrong_token() {
        let mut mgr = ReconnectManager::default();
        let token = mgr.issue_token(1);
        let wrong_token = ReconnectToken(9999);
        let snapshot = make_snapshot(100);

        mgr.store_session(1, token, snapshot, 100);

        let msg = mgr.try_reconnect(1, wrong_token, 100, 105);
        assert!(msg.is_none());
        assert!(mgr.has_session(1)); // session still stored
    }

    #[test]
    fn test_reconnect_manager_try_reconnect_no_session() {
        let mut mgr = ReconnectManager::default();
        let token = ReconnectToken(42);
        let msg = mgr.try_reconnect(999, token, 0, 10);
        assert!(msg.is_none());
    }

    #[test]
    fn test_reconnect_manager_full_snapshot_threshold() {
        let mut mgr = ReconnectManager::new(ReconnectConfig {
            full_snapshot_threshold: 10,
            ..Default::default()
        });
        let token = mgr.issue_token(1);
        let snapshot = make_snapshot(100);
        mgr.store_session(1, token, snapshot, 100);

        // Gap > threshold: full snapshot
        let msg = mgr.try_reconnect(1, token, 100, 200);
        assert!(msg.is_some());
        match msg.unwrap() {
            NetworkMessage::ReconnectSnapshot { missed_ticks, .. } => {
                assert_eq!(missed_ticks, 100);
            }
            _ => panic!("expected ReconnectSnapshot"),
        }
    }

    #[test]
    fn test_reconnect_manager_push_delta() {
        let mut mgr = ReconnectManager::default();
        let token = mgr.issue_token(1);
        let snapshot = make_snapshot(100);
        mgr.store_session(1, token, snapshot, 100);

        mgr.push_delta(1, 101, vec![(0, vec![(1, vec![7, 8, 9])])], vec![]);
        mgr.push_delta(1, 102, vec![(1, vec![(1, vec![10, 11])])], vec![5]);

        assert!(mgr.has_session(1));
    }

    #[test]
    fn test_reconnect_manager_cleanup_expired() {
        let mut mgr = ReconnectManager::new(ReconnectConfig {
            session_timeout_secs: 0, // expire immediately
            ..Default::default()
        });
        let token = mgr.issue_token(1);
        let snapshot = make_snapshot(100);
        mgr.store_session(1, token, snapshot, 100);

        // Small delay to ensure expiration
        std::thread::sleep(std::time::Duration::from_millis(10));

        let expired = mgr.cleanup_expired();
        assert_eq!(expired, vec![1]);
        assert!(!mgr.has_session(1));
    }

    #[test]
    fn test_compute_reconnect_delta() {
        let snap1 = make_snapshot(100);
        let mut snap2 = WorldSnapshot::new(105);
        snap2.add_component(0, 1, vec![10, 20, 30]); // changed
        snap2.add_component(1, 1, vec![4, 5, 6]); // unchanged
        snap2.add_despawned(5);

        let (changed, despawned) = compute_reconnect_delta(&snap1, &snap2);
        assert_eq!(changed.len(), 1); // only entity 0 changed
        assert_eq!(despawned, vec![5]);
    }

    #[test]
    fn test_make_reconnect_snapshot() {
        let snapshot = make_snapshot(42);
        let msg = make_reconnect_snapshot(7, &snapshot, 10);
        match msg {
            NetworkMessage::ReconnectSnapshot {
                client_id,
                tick,
                missed_ticks,
                ..
            } => {
                assert_eq!(client_id, 7);
                assert_eq!(tick, 42);
                assert_eq!(missed_ticks, 10);
            }
            _ => panic!("expected ReconnectSnapshot"),
        }
    }

    #[test]
    fn test_reconnect_snapshot_roundtrip() {
        let msg = NetworkMessage::ReconnectSnapshot {
            client_id: 42,
            tick: 100,
            entities: vec![(0, vec![(1, vec![1, 2, 3])])],
            despawned: vec![5],
            missed_ticks: 10,
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::ReconnectSnapshot {
                client_id,
                tick,
                entities,
                despawned,
                missed_ticks,
            } => {
                assert_eq!(client_id, 42);
                assert_eq!(tick, 100);
                assert_eq!(entities.len(), 1);
                assert_eq!(despawned, vec![5]);
                assert_eq!(missed_ticks, 10);
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_reconnect_request_roundtrip() {
        let msg = NetworkMessage::ReconnectRequest {
            client_id: 42,
            reconnect_token: 12345,
            last_tick: 100,
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::ReconnectRequest {
                client_id,
                reconnect_token,
                last_tick,
            } => {
                assert_eq!(client_id, 42);
                assert_eq!(reconnect_token, 12345);
                assert_eq!(last_tick, 100);
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_reconnect_ack_roundtrip() {
        let msg = NetworkMessage::ReconnectAck {
            client_id: 42,
            server_tick: 200,
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::ReconnectAck {
                client_id,
                server_tick,
            } => {
                assert_eq!(client_id, 42);
                assert_eq!(server_tick, 200);
            }
            _ => panic!("wrong type"),
        }
    }
}
