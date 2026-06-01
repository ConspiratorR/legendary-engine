# Authoritative Server State Synchronization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement authoritative server mode where the server is the single source of truth, clients are presentation-only, with state snapshot synchronization and input forwarding.

**Architecture:** Extend the existing `engine-network` crate with three new modules: `snapshot` (world state serialization/registry), `authority` (server tick + snapshot broadcast + client-side apply), and new `NetworkMessage` variants for the authoritative protocol. The server serializes registered component types into snapshots at a configurable tick rate, broadcasts them to clients, and processes forwarded client inputs. Clients apply received snapshots to their local ECS world for rendering.

**Tech Stack:** Rust 2024, engine-ecs, engine-network, thiserror (already in deps)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/engine-network/src/message.rs` | Modify | Add `StateSnapshot`, `DeltaSnapshot`, `PlayerInput`, `InputAck`, `Correction` message variants |
| `crates/engine-network/src/snapshot.rs` | Create | `NetworkSync` trait, `SnapshotRegistry`, `WorldSnapshot`, component registration |
| `crates/engine-network/src/authority.rs` | Create | `AuthoritativeServer` (tick + broadcast), `ClientAuthority` (receive + apply), `AuthorityConfig` |
| `crates/engine-network/src/lib.rs` | Modify | Export new modules and types |
| `crates/engine-network/src/plugin.rs` | Modify | Add authority systems to `NetworkPlugin` |

---

### Task 1: Extend NetworkMessage with Authoritative Server Variants

**Files:**
- Modify: `crates/engine-network/src/message.rs`

- [ ] **Step 1: Add new message variants**

Add to the `NetworkMessage` enum after the `Heartbeat` variant:

```rust
/// Full world state snapshot (server → client).
StateSnapshot {
    /// Server tick number when this snapshot was taken.
    tick: u64,
    /// Serialized world state: (entity_index, component_type_id, data_bytes).
    entities: Vec<(u32, Vec<(u64, Vec<u8>)>)>,
    /// Entity indices that were despawned since last snapshot.
    despawned: Vec<u32>,
},
/// Delta snapshot with only changed entities (server → client).
DeltaSnapshot {
    /// Server tick number.
    tick: u64,
    /// Changed entities: (entity_index, component_type_id, data_bytes).
    changed: Vec<(u32, Vec<(u64, Vec<u8>)>)>,
    /// Despawned entity indices.
    despawned: Vec<u32>,
},
/// Player input forwarded to server (client → server).
PlayerInput {
    /// Client-side tick when input was captured.
    client_tick: u64,
    /// Serialized input data.
    input_data: Vec<u8>,
},
/// Server acknowledgment of processed input (server → client).
InputAck {
    /// The client tick whose input was processed.
    client_tick: u64,
    /// Server tick at processing time.
    server_tick: u64,
},
/// Server correction to client state (server → client).
Correction {
    /// Server tick of the correction.
    tick: u64,
    /// Corrected entity states: (entity_index, component_type_id, data_bytes).
    entities: Vec<(u32, Vec<(u64, Vec<u8>)>)>,
},
```

- [ ] **Step 2: Update serialize() method**

Add serialization cases for each new variant after the `Heartbeat` case (tag `5`). Use tags `6` through `10`:

```rust
// Tag 6: StateSnapshot
NetworkMessage::StateSnapshot { tick, entities, despawned } => {
    let mut bytes = vec![6];
    bytes.extend(&tick.to_le_bytes());
    bytes.extend(&(entities.len() as u32).to_le_bytes());
    for (entity_idx, components) in entities {
        bytes.extend(&entity_idx.to_le_bytes());
        bytes.extend(&(components.len() as u32).to_le_bytes());
        for (type_id, data) in components {
            bytes.extend(&type_id.to_le_bytes());
            bytes.extend(&(data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }
    }
    bytes.extend(&(despawned.len() as u32).to_le_bytes());
    for idx in despawned {
        bytes.extend(&idx.to_le_bytes());
    }
    bytes
}
// Tag 7: DeltaSnapshot
NetworkMessage::DeltaSnapshot { tick, changed, despawned } => {
    let mut bytes = vec![7];
    bytes.extend(&tick.to_le_bytes());
    bytes.extend(&(changed.len() as u32).to_le_bytes());
    for (entity_idx, components) in changed {
        bytes.extend(&entity_idx.to_le_bytes());
        bytes.extend(&(components.len() as u32).to_le_bytes());
        for (type_id, data) in components {
            bytes.extend(&type_id.to_le_bytes());
            bytes.extend(&(data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }
    }
    bytes.extend(&(despawned.len() as u32).to_le_bytes());
    for idx in despawned {
        bytes.extend(&idx.to_le_bytes());
    }
    bytes
}
// Tag 8: PlayerInput (new format with client_tick)
NetworkMessage::PlayerInput { client_tick, input_data } => {
    let mut bytes = vec![8];
    bytes.extend(&client_tick.to_le_bytes());
    bytes.extend(&(input_data.len() as u32).to_le_bytes());
    bytes.extend(input_data);
    bytes
}
// Tag 9: InputAck
NetworkMessage::InputAck { client_tick, server_tick } => {
    let mut bytes = vec![9];
    bytes.extend(&client_tick.to_le_bytes());
    bytes.extend(&server_tick.to_le_bytes());
    bytes
}
// Tag 10: Correction
NetworkMessage::Correction { tick, entities } => {
    let mut bytes = vec![10];
    bytes.extend(&tick.to_le_bytes());
    bytes.extend(&(entities.len() as u32).to_le_bytes());
    for (entity_idx, components) in entities {
        bytes.extend(&entity_idx.to_le_bytes());
        bytes.extend(&(components.len() as u32).to_le_bytes());
        for (type_id, data) in components {
            bytes.extend(&type_id.to_le_bytes());
            bytes.extend(&(data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }
    }
    bytes
}
```

- [ ] **Step 3: Update deserialize() method**

Add deserialization for tags `6` through `10`. Each follows the same pattern: read tag, read tick, read entity count, read components per entity.

- [ ] **Step 4: Replace old PlayerInput**

Remove the old `PlayerInput { input_data: Vec<u8> }` variant (tag `2`) and replace with the new one that includes `client_tick`. Update the old tag `2` to still deserialize for backward compatibility (map to new format with `client_tick: 0`).

- [ ] **Step 5: Add tests**

```rust
#[test]
fn test_state_snapshot_roundtrip() {
    let msg = NetworkMessage::StateSnapshot {
        tick: 42,
        entities: vec![
            (0, vec![(1, vec![1.0f32.to_le_bytes(), 2.0f32.to_le_bytes(), 3.0f32.to_le_bytes()].concat())]),
        ],
        despawned: vec![5],
    };
    let bytes = msg.serialize();
    let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
    match deserialized {
        NetworkMessage::StateSnapshot { tick, entities, despawned } => {
            assert_eq!(tick, 42);
            assert_eq!(entities.len(), 1);
            assert_eq!(entities[0].0, 0);
            assert_eq!(despawned, vec![5]);
        }
        _ => panic!("wrong message type"),
    }
}

#[test]
fn test_delta_snapshot_roundtrip() {
    let msg = NetworkMessage::DeltaSnapshot {
        tick: 10,
        changed: vec![(1, vec![(2, vec![42])])],
        despawned: vec![],
    };
    let bytes = msg.serialize();
    let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
    assert!(matches!(deserialized, NetworkMessage::DeltaSnapshot { tick: 10, .. }));
}

#[test]
fn test_input_ack_roundtrip() {
    let msg = NetworkMessage::InputAck { client_tick: 5, server_tick: 100 };
    let bytes = msg.serialize();
    let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
    assert!(matches!(deserialized, NetworkMessage::InputAck { client_tick: 5, server_tick: 100 }));
}

#[test]
fn test_correction_roundtrip() {
    let msg = NetworkMessage::Correction {
        tick: 50,
        entities: vec![(0, vec![(1, vec![9, 8, 7])])],
    };
    let bytes = msg.serialize();
    let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
    assert!(matches!(deserialized, NetworkMessage::Correction { tick: 50, .. }));
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p engine-network --lib message`
Expected: All existing + new tests pass.

---

### Task 2: Create Snapshot System

**Files:**
- Create: `crates/engine-network/src/snapshot.rs`

- [ ] **Step 1: Define NetworkSync trait and WorldSnapshot**

```rust
//! World snapshot system for authoritative server synchronization.

use std::any::TypeId;
use std::collections::HashMap;

/// Trait for components that can be synchronized over the network.
///
/// Implement this on any component type that the authoritative server
/// should replicate to clients.
pub trait NetworkSync: 'static {
    /// Serialize the component to bytes.
    fn serialize(&self) -> Vec<u8>;

    /// Deserialize from bytes. Returns None on failure.
    fn deserialize(data: &[u8]) -> Option<Self>
    where
        Self: Sized;

    /// Return the TypeId for this component type (default implementation).
    fn type_id() -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}

/// A snapshot of the world state at a specific tick.
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    /// Server tick when this snapshot was taken.
    pub tick: u64,
    /// Entity data: entity_index → (type_id → serialized_bytes).
    pub entities: HashMap<u32, HashMap<u64, Vec<u8>>>,
    /// Entities that were despawned since the last snapshot.
    pub despawned: Vec<u32>,
}

impl WorldSnapshot {
    /// Create a new empty snapshot for the given tick.
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            entities: HashMap::new(),
            despawned: Vec::new(),
        }
    }

    /// Add component data for an entity.
    pub fn add_component(&mut self, entity_index: u32, type_hash: u64, data: Vec<u8>) {
        self.entities
            .entry(entity_index)
            .or_default()
            .insert(type_hash, data);
    }

    /// Mark an entity as despawned.
    pub fn add_despawned(&mut self, entity_index: u32) {
        self.despawned.push(entity_index);
    }

    /// Convert to the wire format used by NetworkMessage::StateSnapshot.
    pub fn to_wire_format(&self) -> (u64, Vec<(u32, Vec<(u64, Vec<u8>)>)>, Vec<u32>) {
        let entities: Vec<(u32, Vec<(u64, Vec<u8>)>)> = self
            .entities
            .iter()
            .map(|(&idx, components)| {
                let comps: Vec<(u64, Vec<u8>)> = components
                    .iter()
                    .map(|(&tid, data)| (tid, data.clone()))
                    .collect();
                (idx, comps)
            })
            .collect();
        (self.tick, entities, self.despawned.clone())
    }

    /// Compute a delta between this snapshot and a previous one.
    /// Returns only entities that changed or were despawned.
    pub fn delta_from(&self, previous: &WorldSnapshot) -> (Vec<(u32, Vec<(u64, Vec<u8>)>)>, Vec<u32>) {
        let mut changed = Vec::new();

        for (entity_idx, components) in &self.entities {
            if let Some(prev_components) = previous.entities.get(entity_idx) {
                // Entity existed before — check for changes
                let mut changed_comps = Vec::new();
                for (type_hash, data) in components {
                    if prev_components.get(type_hash) != Some(data) {
                        changed_comps.push((*type_hash, data.clone()));
                    }
                }
                if !changed_comps.is_empty() {
                    changed.push((*entity_idx, changed_comps));
                }
            } else {
                // New entity
                let comps: Vec<(u64, Vec<u8>)> = components
                    .iter()
                    .map(|(&tid, data)| (tid, data.clone()))
                    .collect();
                changed.push((*entity_idx, comps));
            }
        }

        // Entities despawned since previous snapshot
        let despawned = self.despawned.clone();

        (changed, despawned)
    }
}
```

- [ ] **Step 2: Define SnapshotRegistry**

```rust
/// Type-erased snapshot function for a component type.
struct SnapshotFn {
    /// Extract component data from the world for a given entity index.
    extract: fn(&engine_ecs::world::World, u32) -> Option<Vec<u8>>,
    /// Apply component data to the world for a given entity index.
    apply: fn(&mut engine_ecs::world::World, u32, &[u8]) -> bool,
    /// TypeId of the component.
    type_id: TypeId,
    /// A hash of the type for wire identification.
    type_hash: u64,
}

/// Registry of component types that participate in network synchronization.
pub struct SnapshotRegistry {
    registrations: Vec<SnapshotFn>,
    /// Map from TypeId to index in registrations for fast lookup.
    by_type: HashMap<TypeId, usize>,
}

impl SnapshotRegistry {
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
            by_type: HashMap::new(),
        }
    }

    /// Register a component type for network synchronization.
    pub fn register<T: NetworkSync>(&mut self) {
        let type_id = TypeId::of::<T>();
        if self.by_type.contains_key(&type_id) {
            return; // Already registered
        }

        let type_hash = Self::type_hash::<T>();
        let idx = self.registrations.len();

        self.registrations.push(SnapshotFn {
            extract: |world, entity_idx| {
                let comp = world.get_by_index::<T>(entity_idx)?;
                Some(comp.serialize())
            },
            apply: |world, entity_idx, data| {
                let value = match T::deserialize(data) {
                    Some(v) => v,
                    None => return false,
                };
                // We need an Entity to call add_component. Use get_by_index pattern.
                // For apply, we'll use a synthetic entity with the raw index.
                let entity = engine_ecs::entity::Entity::new(entity_idx, 0);
                world.add_component(entity, value);
                true
            },
            type_id,
            type_hash,
        });
        self.by_type.insert(type_id, idx);
    }

    /// Create a full snapshot of the world for the given entity indices.
    pub fn snapshot_entities(
        &self,
        world: &engine_ecs::world::World,
        entity_indices: &[u32],
        tick: u64,
    ) -> WorldSnapshot {
        let mut snapshot = WorldSnapshot::new(tick);

        for &entity_idx in entity_indices {
            for reg in &self.registrations {
                if let Some(data) = (reg.extract)(world, entity_idx) {
                    snapshot.add_component(entity_idx, reg.type_hash, data);
                }
            }
        }

        snapshot
    }

    /// Apply a snapshot to the world.
    pub fn apply_snapshot(
        &self,
        world: &mut engine_ecs::world::World,
        snapshot: &WorldSnapshot,
    ) {
        for (entity_idx, components) in &snapshot.entities {
            for reg in &self.registrations {
                if let Some(data) = components.get(&reg.type_hash) {
                    (reg.apply)(world, *entity_idx, data);
                }
            }
        }

        // Despawn entities
        for &entity_idx in &snapshot.despawned {
            let entity = engine_ecs::entity::Entity::new(entity_idx, 0);
            world.despawn(entity);
        }
    }

    /// Apply a delta (changed + despawned) to the world.
    pub fn apply_delta(
        &self,
        world: &mut engine_ecs::world::World,
        changed: &[(u32, Vec<(u64, Vec<u8>)>)],
        despawned: &[u32],
    ) {
        for (entity_idx, components) in changed {
            for reg in &self.registrations {
                if let Some(data) = components.iter().find(|(tid, _)| *tid == reg.type_hash) {
                    (reg.apply)(world, *entity_idx, &data.1);
                }
            }
        }

        for &entity_idx in despawned {
            let entity = engine_ecs::entity::Entity::new(entity_idx, 0);
            world.despawn(entity);
        }
    }

    /// Get the list of registered type hashes.
    pub fn registered_types(&self) -> Vec<u64> {
        self.registrations.iter().map(|r| r.type_hash).collect()
    }

    /// Compute a stable hash for a type (using TypeId's debug format).
    fn type_hash<T: 'static>() -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        TypeId::of::<T>().hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for SnapshotRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Test component
    #[derive(Debug, Clone, PartialEq)]
    struct TestPosition {
        x: f32,
        y: f32,
        z: f32,
    }

    impl NetworkSync for TestPosition {
        fn serialize(&self) -> Vec<u8> {
            let mut bytes = Vec::new();
            bytes.extend(&self.x.to_le_bytes());
            bytes.extend(&self.y.to_le_bytes());
            bytes.extend(&self.z.to_le_bytes());
            bytes
        }

        fn deserialize(data: &[u8]) -> Option<Self> {
            if data.len() < 12 {
                return None;
            }
            Some(Self {
                x: f32::from_le_bytes(data[0..4].try_into().ok()?),
                y: f32::from_le_bytes(data[4..8].try_into().ok()?),
                z: f32::from_le_bytes(data[8..12].try_into().ok()?),
            })
        }
    }

    #[test]
    fn test_network_sync_roundtrip() {
        let pos = TestPosition { x: 1.0, y: 2.0, z: 3.0 };
        let bytes = pos.serialize();
        let restored = TestPosition::deserialize(&bytes).unwrap();
        assert_eq!(pos, restored);
    }

    #[test]
    fn test_world_snapshot_wire_format() {
        let mut snapshot = WorldSnapshot::new(42);
        snapshot.add_component(0, 1, vec![1, 2, 3]);
        snapshot.add_despawned(5);

        let (tick, entities, despawned) = snapshot.to_wire_format();
        assert_eq!(tick, 42);
        assert_eq!(entities.len(), 1);
        assert_eq!(despawned, vec![5]);
    }

    #[test]
    fn test_snapshot_delta() {
        let mut snap1 = WorldSnapshot::new(1);
        snap1.add_component(0, 1, vec![1, 2, 3]);

        let mut snap2 = WorldSnapshot::new(2);
        snap2.add_component(0, 1, vec![1, 2, 4]); // changed
        snap2.add_component(1, 1, vec![5, 6, 7]); // new
        snap2.add_despawned(3);

        let (changed, despawned) = snap2.delta_from(&snap1);
        assert_eq!(changed.len(), 2); // entity 0 changed + entity 1 new
        assert_eq!(despawned, vec![3]);
    }

    #[test]
    fn test_snapshot_registry_roundtrip() {
        let mut registry = SnapshotRegistry::new();
        registry.register::<TestPosition>();

        let mut world = engine_ecs::world::World::new();
        let entity = world.spawn();
        world.add_component(entity, TestPosition { x: 1.0, y: 2.0, z: 3.0 });

        let snapshot = registry.snapshot_entities(&world, &[entity.index()], 1);
        assert!(snapshot.entities.contains_key(&entity.index()));

        // Apply to a fresh world
        let mut world2 = engine_ecs::world::World::new();
        let e2 = world2.spawn();
        registry.apply_snapshot(&world2, &snapshot);
        let pos = world2.get::<TestPosition>(e2);
        assert!(pos.is_some());
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p engine-network --lib snapshot`
Expected: All tests pass.

---

### Task 3: Create Authority Module

**Files:**
- Create: `crates/engine-network/src/authority.rs`

- [ ] **Step 1: Define AuthoritativeServer**

```rust
//! Authoritative server and client-side state management.
//!
//! The authoritative server is the single source of truth for the game world.
//! It runs the full ECS simulation, collects component snapshots, and
//! broadcasts them to clients. Clients receive snapshots and apply them
//! to their local ECS world for rendering.

use crate::message::NetworkMessage;
use crate::server::GameServer;
use crate::snapshot::{SnapshotRegistry, WorldSnapshot};
use engine_ecs::world::World;
use std::collections::{HashMap, HashSet};

/// Configuration for the authoritative server.
#[derive(Debug, Clone)]
pub struct AuthorityConfig {
    /// How often to send full snapshots (in ticks). 0 = never (deltas only).
    pub full_snapshot_interval: u64,
    /// Maximum entities per snapshot message before splitting.
    pub max_entities_per_snapshot: usize,
    /// Whether to send delta snapshots between full snapshots.
    pub send_deltas: bool,
}

impl Default for AuthorityConfig {
    fn default() -> Self {
        Self {
            full_snapshot_interval: 60, // Full snapshot every 60 ticks
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
    /// Current server tick.
    tick: u64,
    /// Configuration.
    config: AuthorityConfig,
    /// Snapshot registry for component serialization.
    registry: SnapshotRegistry,
    /// The last full snapshot sent to clients.
    last_full_snapshot: Option<WorldSnapshot>,
    /// Entity indices tracked for snapshotting.
    tracked_entities: HashSet<u32>,
    /// Pending client inputs: (client_tick, input_data).
    pending_inputs: Vec<(u64, Vec<u8>)>,
}

impl AuthoritativeServer {
    /// Create a new authoritative server with the given config and registry.
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

    /// Register an entity index for snapshot tracking.
    pub fn track_entity(&mut self, entity_index: u32) {
        self.tracked_entities.insert(entity_index);
    }

    /// Unregister an entity index from snapshot tracking.
    pub fn untrack_entity(&mut self, entity_index: u32) {
        self.tracked_entities.remove(&entity_index);
    }

    /// Queue a client input for processing.
    pub fn push_input(&mut self, client_tick: u64, input_data: Vec<u8>) {
        self.pending_inputs.push((client_tick, input_data));
    }

    /// Drain all pending client inputs.
    pub fn drain_inputs(&mut self) -> Vec<(u64, Vec<u8>)> {
        std::mem::take(&mut self.pending_inputs)
    }

    /// Generate and broadcast the appropriate snapshot to all clients.
    ///
    /// Call this each tick after the ECS simulation has run.
    pub fn broadcast_state(&mut self, world: &World, server: &mut GameServer) {
        let should_full = self.tick % self.config.full_snapshot_interval == 0
            || self.last_full_snapshot.is_none();

        let indices: Vec<u32> = self.tracked_entities.iter().copied().collect();

        if should_full {
            let snapshot = self.registry.snapshot_entities(world, &indices, self.tick);
            let (tick, entities, despawned) = snapshot.to_wire_format();

            // Split into chunks if needed
            for chunk in entities.chunks(self.config.max_entities_per_snapshot) {
                let msg = NetworkMessage::StateSnapshot {
                    tick,
                    entities: chunk.to_vec(),
                    despawned: if chunk.len() == entities.len() {
                        despawned.clone()
                    } else {
                        vec![] // Only send despawns with the last chunk
                    },
                };
                server.broadcast(msg);
            }

            self.last_full_snapshot = Some(snapshot);
        } else if self.config.send_deltas {
            if let Some(ref prev) = self.last_full_snapshot {
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

                // Update last snapshot
                self.last_full_snapshot = Some(current);
            }
        }
    }

    /// Send InputAck messages to clients for processed inputs.
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
}
```

- [ ] **Step 2: Define ClientAuthority**

```rust
/// Client-side state that manages snapshot reception and application.
pub struct ClientAuthority {
    /// Snapshot registry (must match server's registrations).
    registry: SnapshotRegistry,
    /// The latest snapshot received from the server.
    latest_snapshot: Option<WorldSnapshot>,
    /// Buffer of delta snapshots not yet applied.
    pending_deltas: Vec<(u64, Vec<(u32, Vec<(u64, Vec<u8>)>)>, Vec<u32>)>,
    /// Client-side tick counter.
    client_tick: u64,
    /// Input history for prediction/reconciliation.
    input_history: Vec<(u64, Vec<u8>)>,
    /// Maximum input history to keep.
    max_input_history: usize,
}

impl ClientAuthority {
    /// Create a new client authority with the given registry.
    pub fn new(registry: SnapshotRegistry) -> Self {
        Self {
            registry,
            latest_snapshot: None,
            pending_deltas: Vec::new(),
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

    /// Record an input for potential prediction/reconciliation.
    pub fn record_input(&mut self, client_tick: u64, input_data: Vec<u8>) {
        self.input_history.push((client_tick, input_data));
        if self.input_history.len() > self.max_input_history {
            self.input_history.remove(0);
        }
    }

    /// Process a received NetworkMessage, applying snapshots to the world.
    ///
    /// Returns true if the message was handled.
    pub fn handle_message(&mut self, world: &mut World, msg: &NetworkMessage) -> bool {
        match msg {
            NetworkMessage::StateSnapshot { tick, entities, despawned } => {
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
            NetworkMessage::DeltaSnapshot { tick, changed, despawned } => {
                self.registry.apply_delta(world, changed, despawned);

                // If we have a full snapshot, update it with the delta
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
                // Apply corrections directly
                for (entity_idx, components) in entities {
                    for reg in self.registry.registered_types() {
                        if let Some((_, data)) = components.iter().find(|(tid, _)| *tid == reg) {
                            // Find the registration and apply
                            for r_idx in 0..self.registry.registered_types().len() {
                                // Use apply_delta which handles this
                            }
                        }
                    }
                }
                // Use apply_delta for corrections too
                self.registry.apply_delta(world, entities, &[]);
                true
            }
            NetworkMessage::InputAck { client_tick, server_tick: _ } => {
                // Remove acknowledged inputs from history
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
}
```

- [ ] **Step 3: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::NetworkSync;
    use crate::server::ServerConfig;

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
            if data.len() < 12 { return None; }
            Some(Pos(
                f32::from_le_bytes(data[0..4].try_into().ok()?),
                f32::from_le_bytes(data[4..8].try_into().ok()?),
                f32::from_le_bytes(data[8..12].try_into().ok()?),
            ))
        }
    }

    #[test]
    fn test_authoritative_server_tick() {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        let mut server = AuthoritativeServer::new(AuthorityConfig::default(), reg);
        assert_eq!(server.tick(), 0);
        server.advance_tick();
        assert_eq!(server.tick(), 1);
    }

    #[test]
    fn test_authoritative_server_input_queue() {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        let mut server = AuthoritativeServer::new(AuthorityConfig::default(), reg);
        server.push_input(1, vec![1, 2, 3]);
        server.push_input(2, vec![4, 5, 6]);
        let inputs = server.drain_inputs();
        assert_eq!(inputs.len(), 2);
        assert!(server.drain_inputs().is_empty());
    }

    #[test]
    fn test_authoritative_server_broadcast() {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        let mut server = AuthoritativeServer::new(AuthorityConfig::default(), reg);
        server.track_entity(0);
        server.track_entity(1);

        let mut world = engine_ecs::world::World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.add_component(e1, Pos(1.0, 2.0, 3.0));
        world.add_component(e2, Pos(4.0, 5.0, 6.0));

        let mut game_server = GameServer::new(ServerConfig::default());
        server.broadcast_state(&world, &mut game_server);

        // Should have pending messages in router
        // (no clients connected, so nothing actually sent, but router has pending)
        assert!(server.last_full_snapshot.is_some());
    }

    #[test]
    fn test_client_authority_handle_snapshot() {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        let mut client = ClientAuthority::new(reg);
        let mut world = engine_ecs::world::World::new();
        let _e = world.spawn(); // entity 0

        let msg = NetworkMessage::StateSnapshot {
            tick: 10,
            entities: vec![(0, vec![(123, vec![0.0f32.to_le_bytes(), 0.0f32.to_le_bytes(), 0.0f32.to_le_bytes()].concat())])],
            despawned: vec![],
        };

        let handled = client.handle_message(&mut world, &msg);
        assert!(handled);
        assert_eq!(client.server_tick(), Some(10));
    }

    #[test]
    fn test_client_authority_handle_delta() {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        let mut client = ClientAuthority::new(reg);
        let mut world = engine_ecs::world::World::new();
        let _e = world.spawn();

        let msg = NetworkMessage::DeltaSnapshot {
            tick: 11,
            changed: vec![(0, vec![(123, vec![1.0f32.to_le_bytes(), 2.0f32.to_le_bytes(), 3.0f32.to_le_bytes()].concat())])],
            despawned: vec![],
        };

        let handled = client.handle_message(&mut world, &msg);
        assert!(handled);
    }

    #[test]
    fn test_client_input_history() {
        let mut reg = SnapshotRegistry::new();
        reg.register::<Pos>();
        let mut client = ClientAuthority::new(reg);

        client.record_input(1, vec![1]);
        client.record_input(2, vec![2]);
        assert_eq!(client.input_history().len(), 2);

        // Ack tick 1
        let msg = NetworkMessage::InputAck { client_tick: 1, server_tick: 100 };
        let mut world = engine_ecs::world::World::new();
        client.handle_message(&mut world, &msg);
        assert_eq!(client.input_history().len(), 1);
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p engine-network --lib authority`
Expected: All tests pass.

---

### Task 4: Update lib.rs Exports

**Files:**
- Modify: `crates/engine-network/src/lib.rs`

- [ ] **Step 1: Add module declarations**

Add after the existing `pub mod` declarations:

```rust
pub mod authority;
pub mod snapshot;
```

- [ ] **Step 2: Add re-exports**

Add to the `pub use` section:

```rust
pub use authority::{AuthorityConfig, AuthoritativeServer, ClientAuthority};
pub use snapshot::{NetworkSync, SnapshotRegistry, WorldSnapshot};
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p engine-network`
Expected: Compiles without errors.

---

### Task 5: Update NetworkPlugin with Authority Systems

**Files:**
- Modify: `crates/engine-network/src/plugin.rs`

- [ ] **Step 1: Add authority ECS systems**

Add after the existing systems:

```rust
/// ECS system that processes authoritative server logic each tick.
///
/// Advances the server tick, processes queued client inputs,
/// and broadcasts the current world state to all clients.
pub fn authority_server_system(world: &mut World) {
    // Process authoritative server
    if let Some(mut auth) = world.remove_resource::<AuthoritativeServer>() {
        auth.advance_tick();

        // Drain and process client inputs
        let inputs = auth.drain_inputs();
        for (_client_tick, _input_data) in inputs {
            // Input processing would be handled by game-specific systems
            // that read from the authoritative server's input queue
        }

        // Re-insert for broadcast (need world ref)
        world.insert_resource(auth);
    }

    // Broadcast state (needs both world and server)
    if let Some(mut auth) = world.remove_resource::<AuthoritativeServer>() {
        if let Some(server) = world.get_resource_mut::<GameServer>() {
            auth.broadcast_state(world, server);
        }
        world.insert_resource(auth);
    }
}

/// ECS system that processes incoming authoritative messages on the client side.
pub fn authority_client_system(world: &mut World) {
    if let Some(mut client_auth) = world.remove_resource::<ClientAuthority>() {
        // Get messages from the game client
        if let Some(client) = world.get_resource_mut::<GameClient>() {
            let messages = client.receive();
            for msg in &messages {
                client_auth.handle_message(world, msg);
            }
        }
        world.insert_resource(client_auth);
    }
}
```

- [ ] **Step 2: Register authority systems in NetworkPlugin::build**

Update the `build` method:

```rust
impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world_mut();
        world.insert_resource(NetworkConfig::default());

        // Register ECS systems for network processing
        app.add_system(network_send_system);
        app.add_system(network_receive_system);
        app.add_system(authority_server_system);
        app.add_system(authority_client_system);
    }
}
```

- [ ] **Step 3: Add tests**

```rust
#[test]
fn test_authority_server_system_no_panic() {
    let mut world = World::new();
    let mut registry = crate::snapshot::SnapshotRegistry::new();
    registry.register::<crate::snapshot::tests::TestPos>();
    let auth = crate::authority::AuthoritativeServer::new(
        crate::authority::AuthorityConfig::default(),
        registry,
    );
    world.insert_resource(auth);
    authority_server_system(&mut world);
}

#[test]
fn test_authority_client_system_no_panic() {
    let mut world = World::new();
    let mut registry = crate::snapshot::SnapshotRegistry::new();
    let client_auth = crate::authority::ClientAuthority::new(registry);
    world.insert_resource(client_auth);
    authority_client_system(&mut world);
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p engine-network`
Expected: All tests pass (existing + new).

---

### Task 6: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test -p engine-network`
Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p engine-network`
Expected: No warnings.

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt -p engine-network --check`
Expected: All files formatted.

- [ ] **Step 4: Build release**

Run: `cargo build -p engine-network --release`
Expected: Successful build.
