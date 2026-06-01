//! Authoritative server and client-side state management.
//!
//! The authoritative server is the single source of truth for the game world.
//! It runs the full ECS simulation, collects component snapshots, and
//! broadcasts them to clients. Clients receive snapshots and apply them
//! to their local ECS world for rendering.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                  Server                          │
//! │  ECS World → AuthoritativeServer → GameServer    │
//! │    (full sim)   (tick + snapshot)   (network)    │
//! └──────────────────────┬──────────────────────────┘
//!                        │ StateSnapshot / DeltaSnapshot
//!                        ▼
//! ┌─────────────────────────────────────────────────┐
//! │                  Client                          │
//! │  GameClient → ClientAuthority → ECS World        │
//! │   (network)    (apply snapshot)  (render only)   │
//! └─────────────────────────────────────────────────┘
//! ```

use crate::message::{EntityComponentData, NetworkMessage};
use crate::server::GameServer;
use crate::snapshot::{SnapshotRegistry, WorldSnapshot};
use engine_ecs::world::World;
use std::collections::HashSet;

/// Configuration for the authoritative server.
#[derive(Debug, Clone)]
pub struct AuthorityConfig {
    /// How often to send full snapshots (in ticks). 0 = every tick.
    pub full_snapshot_interval: u64,
    /// Maximum entities per snapshot message before splitting.
    pub max_entities_per_snapshot: usize,
    /// Whether to send delta snapshots between full snapshots.
    pub send_deltas: bool,
}

impl Default for AuthorityConfig {
    fn default() -> Self {
        Self {
            full_snapshot_interval: 60,
            max_entities_per_snapshot: 256,
            send_deltas: true,
        }
    }
}

/// Server-side authoritative state manager.
///
/// Manages the server tick counter, snapshot generation, and
/// broadcasting state to connected clients.
pub struct AuthoritativeServer {
    tick: u64,
    config: AuthorityConfig,
    registry: SnapshotRegistry,
    last_full_snapshot: Option<WorldSnapshot>,
    tracked_entities: HashSet<u32>,
    pending_inputs: Vec<PendingInput>,
}

/// A client input waiting to be processed.
#[derive(Debug, Clone)]
pub struct PendingInput {
    /// Client ID that sent this input.
    pub client_id: u64,
    /// Client-side tick when input was captured.
    pub client_tick: u64,
    /// Serialized input data.
    pub data: Vec<u8>,
}

impl AuthoritativeServer {
    /// Create a new authoritative server.
    pub fn new(config: AuthorityConfig, registry: SnapshotRegistry) -> Self {
        Self {
            tick: 0,
            config,
            registry,
            last_full_snapshot: None,
            tracked_entities: HashSet::new(),
            pending_inputs: Vec::new(),
        }
    }

    /// Advance the server tick by one.
    pub fn advance_tick(&mut self) {
        self.tick += 1;
    }

    /// Get the current server tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Get the snapshot registry.
    pub fn registry(&self) -> &SnapshotRegistry {
        &self.registry
    }

    /// Get a mutable reference to the snapshot registry.
    pub fn registry_mut(&mut self) -> &mut SnapshotRegistry {
        &mut self.registry
    }

    /// Get the configuration.
    pub fn config(&self) -> &AuthorityConfig {
        &self.config
    }

    /// Register an entity index for snapshot tracking.
    pub fn track_entity(&mut self, entity_index: u32) {
        self.tracked_entities.insert(entity_index);
    }

    /// Unregister an entity index from snapshot tracking.
    pub fn untrack_entity(&mut self, entity_index: u32) {
        self.tracked_entities.remove(&entity_index);
    }

    /// Check if an entity is being tracked.
    pub fn is_tracked(&self, entity_index: u32) -> bool {
        self.tracked_entities.contains(&entity_index)
    }

    /// Get the set of tracked entity indices.
    pub fn tracked_entities(&self) -> &HashSet<u32> {
        &self.tracked_entities
    }

    /// Queue a client input for processing.
    pub fn push_input(&mut self, client_id: u64, client_tick: u64, data: Vec<u8>) {
        self.pending_inputs.push(PendingInput {
            client_id,
            client_tick,
            data,
        });
    }

    /// Drain all pending client inputs.
    pub fn drain_inputs(&mut self) -> Vec<PendingInput> {
        std::mem::take(&mut self.pending_inputs)
    }

    /// Get the number of pending inputs.
    pub fn pending_input_count(&self) -> usize {
        self.pending_inputs.len()
    }

    /// Generate and broadcast the appropriate snapshot to all clients.
    ///
    /// Call this each tick after the ECS simulation has run.
    pub fn broadcast_state(&mut self, world: &World, server: &mut GameServer) {
        let should_full = self.config.full_snapshot_interval == 0
            || self.tick.is_multiple_of(self.config.full_snapshot_interval)
            || self.last_full_snapshot.is_none();

        let indices: Vec<u32> = self.tracked_entities.iter().copied().collect();

        if should_full {
            let snapshot = self.registry.snapshot_entities(world, &indices, self.tick);
            let (tick, entities, despawned) = snapshot.to_wire_format();

            for chunk in entities.chunks(self.config.max_entities_per_snapshot) {
                let msg = NetworkMessage::StateSnapshot {
                    tick,
                    entities: chunk.to_vec(),
                    despawned: if chunk.len() == entities.len() {
                        despawned.clone()
                    } else {
                        vec![]
                    },
                };
                server.broadcast(msg);
            }

            self.last_full_snapshot = Some(snapshot);
        } else if self.config.send_deltas
            && let Some(ref prev) = self.last_full_snapshot
        {
            let current = self.registry.snapshot_entities(world, &indices, self.tick);
            let (changed, despawned) = current.delta_from(prev);

            if !changed.is_empty() || !despawned.is_empty() {
                for chunk in changed.chunks(self.config.max_entities_per_snapshot) {
                    let msg = NetworkMessage::DeltaSnapshot {
                        tick: self.tick,
                        changed: chunk.to_vec(),
                        despawned: if chunk.len() == changed.len() {
                            despawned.clone()
                        } else {
                            vec![]
                        },
                    };
                    server.broadcast(msg);
                }
            }

            self.last_full_snapshot = Some(current);
        }
    }

    /// Send InputAck messages to a specific client for processed input ticks.
    pub fn ack_inputs(&self, server: &mut GameServer, client_id: u64, client_ticks: &[u64]) {
        for &client_tick in client_ticks {
            server.send_to(
                client_id,
                NetworkMessage::InputAck {
                    client_tick,
                    server_tick: self.tick,
                },
            );
        }
    }

    /// Send a correction to a specific client.
    pub fn send_correction(
        &self,
        server: &mut GameServer,
        client_id: u64,
        entities: EntityComponentData,
    ) {
        server.send_to(
            client_id,
            NetworkMessage::Correction {
                tick: self.tick,
                entities,
            },
        );
    }
}

/// Client-side state that manages snapshot reception and application.
pub struct ClientAuthority {
    registry: SnapshotRegistry,
    latest_snapshot: Option<WorldSnapshot>,
    client_tick: u64,
    input_history: Vec<(u64, Vec<u8>)>,
    max_input_history: usize,
}

impl ClientAuthority {
    /// Create a new client authority with the given registry.
    pub fn new(registry: SnapshotRegistry) -> Self {
        Self {
            registry,
            latest_snapshot: None,
            client_tick: 0,
            input_history: Vec::new(),
            max_input_history: 120,
        }
    }

    /// Get the snapshot registry.
    pub fn registry(&self) -> &SnapshotRegistry {
        &self.registry
    }

    /// Get a mutable reference to the snapshot registry.
    pub fn registry_mut(&mut self) -> &mut SnapshotRegistry {
        &mut self.registry
    }

    /// Advance the client tick by one.
    pub fn advance_tick(&mut self) {
        self.client_tick += 1;
    }

    /// Get the current client tick.
    pub fn client_tick(&self) -> u64 {
        self.client_tick
    }

    /// Get the latest server tick received.
    pub fn server_tick(&self) -> Option<u64> {
        self.latest_snapshot.as_ref().map(|s| s.tick)
    }

    /// Get a reference to the latest snapshot.
    pub fn latest_snapshot(&self) -> Option<&WorldSnapshot> {
        self.latest_snapshot.as_ref()
    }

    /// Record an input for potential prediction/reconciliation.
    pub fn record_input(&mut self, client_tick: u64, input_data: Vec<u8>) {
        self.input_history.push((client_tick, input_data));
        if self.input_history.len() > self.max_input_history {
            self.input_history.remove(0);
        }
    }

    /// Process a received [`NetworkMessage`], applying snapshots to the world.
    ///
    /// Returns `true` if the message was handled.
    pub fn handle_message(&mut self, world: &mut World, msg: &NetworkMessage) -> bool {
        match msg {
            NetworkMessage::StateSnapshot {
                tick,
                entities,
                despawned,
            } => {
                let mut snapshot = WorldSnapshot::new(*tick);
                for (entity_idx, components) in entities {
                    for (type_hash, data) in components {
                        snapshot.add_component(*entity_idx, *type_hash, data.clone());
                    }
                }
                for &idx in despawned {
                    snapshot.add_despawned(idx);
                }

                self.registry.apply_snapshot(world, &snapshot);
                self.latest_snapshot = Some(snapshot);
                true
            }
            NetworkMessage::DeltaSnapshot {
                tick,
                changed,
                despawned,
            } => {
                self.registry.apply_delta(world, changed, despawned);

                if let Some(ref mut snapshot) = self.latest_snapshot {
                    snapshot.tick = *tick;
                    for (entity_idx, components) in changed {
                        for (type_hash, data) in components {
                            snapshot.add_component(*entity_idx, *type_hash, data.clone());
                        }
                    }
                    for &idx in despawned {
                        snapshot.add_despawned(idx);
                    }
                }
                true
            }
            NetworkMessage::Correction { tick: _, entities } => {
                self.registry.apply_delta(world, entities, &[]);
                true
            }
            NetworkMessage::InputAck {
                client_tick,
                server_tick: _,
            } => {
                self.input_history.retain(|(tick, _)| *tick > *client_tick);
                true
            }
            _ => false,
        }
    }

    /// Get the input history (for prediction/reconciliation).
    pub fn input_history(&self) -> &[(u64, Vec<u8>)] {
        &self.input_history
    }

    /// Set the maximum input history length.
    pub fn set_max_input_history(&mut self, max: usize) {
        self.max_input_history = max;
        while self.input_history.len() > self.max_input_history {
            self.input_history.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ServerConfig;
    use crate::snapshot::NetworkSync;

    #[derive(Debug, Clone, PartialEq)]
    struct Pos(f32, f32, f32);

    impl NetworkSync for Pos {
        fn serialize(&self) -> Vec<u8> {
            let mut b = Vec::new();
            b.extend(&self.0.to_le_bytes());
            b.extend(&self.1.to_le_bytes());
            b.extend(&self.2.to_le_bytes());
            b
        }
        fn deserialize(data: &[u8]) -> Option<Self> {
            if data.len() < 12 {
                return None;
            }
            Some(Pos(
                f32::from_le_bytes(data[0..4].try_into().ok()?),
                f32::from_le_bytes(data[4..8].try_into().ok()?),
                f32::from_le_bytes(data[8..12].try_into().ok()?),
            ))
        }
    }

    fn make_server() -> AuthoritativeServer {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        AuthoritativeServer::new(AuthorityConfig::default(), reg)
    }

    fn make_client() -> ClientAuthority {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        ClientAuthority::new(reg)
    }

    #[test]
    fn test_authoritative_server_tick() {
        let mut server = make_server();
        assert_eq!(server.tick(), 0);
        server.advance_tick();
        assert_eq!(server.tick(), 1);
        server.advance_tick();
        assert_eq!(server.tick(), 2);
    }

    #[test]
    fn test_authoritative_server_track_entities() {
        let mut server = make_server();
        server.track_entity(0);
        server.track_entity(1);
        assert!(server.is_tracked(0));
        assert!(server.is_tracked(1));
        assert!(!server.is_tracked(2));

        server.untrack_entity(0);
        assert!(!server.is_tracked(0));
        assert!(server.is_tracked(1));
    }

    #[test]
    fn test_authoritative_server_input_queue() {
        let mut server = make_server();
        server.push_input(1, 10, vec![1, 2, 3]);
        server.push_input(2, 11, vec![4, 5, 6]);
        assert_eq!(server.pending_input_count(), 2);

        let inputs = server.drain_inputs();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].client_id, 1);
        assert_eq!(inputs[0].client_tick, 10);
        assert_eq!(inputs[1].client_id, 2);

        assert!(server.drain_inputs().is_empty());
    }

    #[test]
    fn test_authoritative_server_broadcast_full() {
        let mut server = make_server();
        server.track_entity(0);

        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Pos(1.0, 2.0, 3.0));

        let mut game_server = GameServer::new(ServerConfig::default());
        server.broadcast_state(&world, &mut game_server);

        assert!(server.last_full_snapshot.is_some());
        assert_eq!(server.last_full_snapshot.as_ref().unwrap().tick, 0);
    }

    #[test]
    fn test_authoritative_server_broadcast_delta() {
        let mut server = make_server();
        server.config.full_snapshot_interval = 10; // Full every 10 ticks
        server.config.send_deltas = true;
        server.track_entity(0);

        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Pos(1.0, 2.0, 3.0));

        let mut game_server = GameServer::new(ServerConfig::default());

        // Tick 0: full snapshot
        server.advance_tick(); // tick 1
        server.broadcast_state(&world, &mut game_server);
        assert!(server.last_full_snapshot.is_some());

        // Tick 1: delta
        server.advance_tick(); // tick 2
        server.broadcast_state(&world, &mut game_server);
        // Delta should be sent (no changes, so no delta messages)
    }

    #[test]
    fn test_authoritative_server_ack_inputs() {
        let mut server = make_server();
        server.advance_tick(); // tick 1

        let mut game_server = GameServer::new(ServerConfig::default());
        server.ack_inputs(&mut game_server, 1, &[10, 11]);
        // Messages are queued in the router, not directly testable without
        // a running server, but we verify no panic.
    }

    #[test]
    fn test_authoritative_server_send_correction() {
        let mut server = make_server();
        server.advance_tick();

        let mut game_server = GameServer::new(ServerConfig::default());
        server.send_correction(&mut game_server, 1, vec![(0, vec![(123, vec![1, 2, 3])])]);
        // Verify no panic
    }

    #[test]
    fn test_client_authority_tick() {
        let mut client = make_client();
        assert_eq!(client.client_tick(), 0);
        client.advance_tick();
        assert_eq!(client.client_tick(), 1);
    }

    #[test]
    fn test_client_authority_handle_state_snapshot() {
        let mut client = make_client();
        let mut world = World::new();
        let _e = world.spawn(); // entity 0

        let pos_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::any::TypeId::of::<Pos>().hash(&mut hasher);
            hasher.finish()
        };

        let msg = NetworkMessage::StateSnapshot {
            tick: 10,
            entities: vec![(0, vec![(pos_hash, Pos(1.0, 2.0, 3.0).serialize())])],
            despawned: vec![],
        };

        let handled = client.handle_message(&mut world, &msg);
        assert!(handled);
        assert_eq!(client.server_tick(), Some(10));

        let pos = world.get_by_index::<Pos>(0);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().0, 1.0);
    }

    #[test]
    fn test_client_authority_handle_delta_snapshot() {
        let mut client = make_client();
        let mut world = World::new();
        let _e = world.spawn();

        let pos_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::any::TypeId::of::<Pos>().hash(&mut hasher);
            hasher.finish()
        };

        // First, apply a full snapshot
        let msg = NetworkMessage::StateSnapshot {
            tick: 10,
            entities: vec![(0, vec![(pos_hash, Pos(1.0, 2.0, 3.0).serialize())])],
            despawned: vec![],
        };
        client.handle_message(&mut world, &msg);

        // Then apply a delta
        let msg = NetworkMessage::DeltaSnapshot {
            tick: 11,
            changed: vec![(0, vec![(pos_hash, Pos(4.0, 5.0, 6.0).serialize())])],
            despawned: vec![],
        };
        let handled = client.handle_message(&mut world, &msg);
        assert!(handled);

        let pos = world.get_by_index::<Pos>(0);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().0, 4.0);
    }

    #[test]
    fn test_client_authority_handle_correction() {
        let mut client = make_client();
        let mut world = World::new();
        let _e = world.spawn();

        let pos_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::any::TypeId::of::<Pos>().hash(&mut hasher);
            hasher.finish()
        };

        let msg = NetworkMessage::Correction {
            tick: 50,
            entities: vec![(0, vec![(pos_hash, Pos(9.0, 8.0, 7.0).serialize())])],
        };

        let handled = client.handle_message(&mut world, &msg);
        assert!(handled);

        let pos = world.get_by_index::<Pos>(0);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().0, 9.0);
    }

    #[test]
    fn test_client_authority_handle_input_ack() {
        let mut client = make_client();
        let mut world = World::new();

        client.record_input(1, vec![1]);
        client.record_input(2, vec![2]);
        client.record_input(3, vec![3]);
        assert_eq!(client.input_history().len(), 3);

        let msg = NetworkMessage::InputAck {
            client_tick: 2,
            server_tick: 100,
        };
        client.handle_message(&mut world, &msg);

        // Only inputs with tick > 2 should remain
        assert_eq!(client.input_history().len(), 1);
        assert_eq!(client.input_history()[0].0, 3);
    }

    #[test]
    fn test_client_authority_ignores_unrelated_messages() {
        let mut client = make_client();
        let mut world = World::new();

        let msg = NetworkMessage::Chat {
            sender: "test".to_string(),
            text: "hello".to_string(),
        };
        let handled = client.handle_message(&mut world, &msg);
        assert!(!handled);
    }

    #[test]
    fn test_client_authority_input_history_limit() {
        let mut client = make_client();
        client.set_max_input_history(2);

        client.record_input(1, vec![1]);
        client.record_input(2, vec![2]);
        client.record_input(3, vec![3]);

        assert_eq!(client.input_history().len(), 2);
        assert_eq!(client.input_history()[0].0, 2);
        assert_eq!(client.input_history()[1].0, 3);
    }
}
