//! World snapshot system for authoritative server synchronization.
//!
//! Components that need to be synchronized over the network implement
//! [`NetworkSync`]. A [`SnapshotRegistry`] tracks which component types
//! participate in snapshots and provides methods to extract/apply state.

use crate::message::EntityComponentData;
use std::any::TypeId;
use std::collections::HashMap;

/// Trait for components that can be synchronized over the network.
///
/// Implement this on any component type that the authoritative server
/// should replicate to clients.
pub trait NetworkSync: 'static {
    /// Serialize the component to bytes.
    fn serialize(&self) -> Vec<u8>;

    /// Deserialize from bytes. Returns `None` on failure.
    fn deserialize(data: &[u8]) -> Option<Self>
    where
        Self: Sized;
}

/// A snapshot of the world state at a specific tick.
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    /// Server tick when this snapshot was taken.
    pub tick: u64,
    /// Entity data: `entity_index` → (`type_hash` → serialized_bytes).
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

    /// Convert to the wire format used by [`NetworkMessage::StateSnapshot`](crate::message::NetworkMessage::StateSnapshot).
    pub fn to_wire_format(&self) -> (u64, EntityComponentData, Vec<u32>) {
        let entities: EntityComponentData = self
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
    ///
    /// Returns only entities that changed or are new, plus despawned entities.
    pub fn delta_from(&self, previous: &WorldSnapshot) -> (EntityComponentData, Vec<u32>) {
        let mut changed = Vec::new();

        for (entity_idx, components) in &self.entities {
            if let Some(prev_components) = previous.entities.get(entity_idx) {
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
                let comps: Vec<(u64, Vec<u8>)> = components
                    .iter()
                    .map(|(&tid, data)| (tid, data.clone()))
                    .collect();
                changed.push((*entity_idx, comps));
            }
        }

        let despawned = self.despawned.clone();
        (changed, despawned)
    }
}

/// Type-erased functions for a registered component type.
struct SnapshotFn {
    /// Extract component data from the world for a given entity index.
    extract: fn(&engine_ecs::world::World, u32) -> Option<Vec<u8>>,
    /// Apply component data to the world for a given entity index.
    apply: fn(&mut engine_ecs::world::World, u32, &[u8]) -> bool,
    /// A stable hash of the type for wire identification.
    type_hash: u64,
}

/// Registry of component types that participate in network synchronization.
pub struct SnapshotRegistry {
    registrations: Vec<SnapshotFn>,
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
    pub fn register<T: NetworkSync + Send + Sync>(&mut self) {
        let type_id = TypeId::of::<T>();
        if self.by_type.contains_key(&type_id) {
            return;
        }

        let type_hash = Self::compute_type_hash::<T>();
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
                let entity = engine_ecs::entity::Entity::new(entity_idx, 0);
                world.add_component(entity, value);
                true
            },
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

    /// Apply a full snapshot to the world.
    ///
    /// Components are stored directly by entity index. The entity does not
    /// need to be spawned first — `add_component` works with raw indices.
    pub fn apply_snapshot(&self, world: &mut engine_ecs::world::World, snapshot: &WorldSnapshot) {
        for (entity_idx, components) in &snapshot.entities {
            for reg in &self.registrations {
                if let Some(data) = components.get(&reg.type_hash) {
                    (reg.apply)(world, *entity_idx, data);
                }
            }
        }

        for &entity_idx in &snapshot.despawned {
            let entity = engine_ecs::entity::Entity::new(entity_idx, 0);
            world.despawn(entity);
        }
    }

    /// Apply a delta (changed entities + despawned) to the world.
    pub fn apply_delta(
        &self,
        world: &mut engine_ecs::world::World,
        changed: &EntityComponentData,
        despawned: &[u32],
    ) {
        for (entity_idx, components) in changed {
            for reg in &self.registrations {
                if let Some((_, data)) = components.iter().find(|(tid, _)| *tid == reg.type_hash) {
                    (reg.apply)(world, *entity_idx, data);
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

    fn compute_type_hash<T: 'static>() -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[derive(Debug, Clone, PartialEq)]
    struct TestHealth(i32);

    impl NetworkSync for TestHealth {
        fn serialize(&self) -> Vec<u8> {
            self.0.to_le_bytes().to_vec()
        }

        fn deserialize(data: &[u8]) -> Option<Self> {
            if data.len() < 4 {
                return None;
            }
            Some(TestHealth(i32::from_le_bytes(data[0..4].try_into().ok()?)))
        }
    }

    #[test]
    fn test_network_sync_roundtrip() {
        let pos = TestPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let bytes = pos.serialize();
        let restored = TestPosition::deserialize(&bytes).unwrap();
        assert_eq!(pos, restored);
    }

    #[test]
    fn test_network_sync_deserialize_too_short() {
        assert!(TestPosition::deserialize(&[1, 2, 3]).is_none());
    }

    #[test]
    fn test_world_snapshot_new() {
        let snapshot = WorldSnapshot::new(42);
        assert_eq!(snapshot.tick, 42);
        assert!(snapshot.entities.is_empty());
        assert!(snapshot.despawned.is_empty());
    }

    #[test]
    fn test_world_snapshot_add_component() {
        let mut snapshot = WorldSnapshot::new(1);
        snapshot.add_component(0, 100, vec![1, 2, 3]);
        snapshot.add_component(0, 200, vec![4, 5]);
        snapshot.add_component(1, 100, vec![6]);

        assert_eq!(snapshot.entities.len(), 2);
        assert_eq!(snapshot.entities[&0].len(), 2);
        assert_eq!(snapshot.entities[&1].len(), 1);
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
    fn test_snapshot_delta_no_changes() {
        let snap1 = WorldSnapshot::new(1);
        let snap2 = WorldSnapshot::new(2);

        let (changed, despawned) = snap2.delta_from(&snap1);
        assert!(changed.is_empty());
        assert!(despawned.is_empty());
    }

    #[test]
    fn test_snapshot_delta_new_entity() {
        let snap1 = WorldSnapshot::new(1);

        let mut snap2 = WorldSnapshot::new(2);
        snap2.add_component(0, 1, vec![1, 2, 3]);

        let (changed, _) = snap2.delta_from(&snap1);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].0, 0);
    }

    #[test]
    fn test_snapshot_delta_changed_component() {
        let mut snap1 = WorldSnapshot::new(1);
        snap1.add_component(0, 1, vec![1, 2, 3]);

        let mut snap2 = WorldSnapshot::new(2);
        snap2.add_component(0, 1, vec![1, 2, 4]);

        let (changed, _) = snap2.delta_from(&snap1);
        assert_eq!(changed.len(), 1);
    }

    #[test]
    fn test_snapshot_delta_unchanged_entity() {
        let mut snap1 = WorldSnapshot::new(1);
        snap1.add_component(0, 1, vec![1, 2, 3]);

        let mut snap2 = WorldSnapshot::new(2);
        snap2.add_component(0, 1, vec![1, 2, 3]);

        let (changed, _) = snap2.delta_from(&snap1);
        assert!(changed.is_empty());
    }

    #[test]
    fn test_snapshot_delta_despawned() {
        let snap1 = WorldSnapshot::new(1);
        let mut snap2 = WorldSnapshot::new(2);
        snap2.add_despawned(3);

        let (_, despawned) = snap2.delta_from(&snap1);
        assert_eq!(despawned, vec![3]);
    }

    #[test]
    fn test_snapshot_registry_register() {
        let mut registry = SnapshotRegistry::new();
        registry.register::<TestPosition>();
        registry.register::<TestHealth>();

        // Registering same type twice should be idempotent
        registry.register::<TestPosition>();
        assert_eq!(registry.registered_types().len(), 2);
    }

    #[test]
    fn test_snapshot_registry_extract_and_apply() {
        let mut registry = SnapshotRegistry::new();
        registry.register::<TestPosition>();

        let mut world = engine_ecs::world::World::new();
        let entity = world.spawn();
        world.add_component(
            entity,
            TestPosition {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );

        let snapshot = registry.snapshot_entities(&world, &[entity.index()], 1);
        assert!(snapshot.entities.contains_key(&entity.index()));

        // Apply to a fresh world
        let mut world2 = engine_ecs::world::World::new();
        registry.apply_snapshot(&mut world2, &snapshot);

        // The entity should exist with the component
        let pos = world2.get_by_index::<TestPosition>(entity.index());
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().x, 1.0);
    }

    #[test]
    fn test_snapshot_registry_multiple_components() {
        let mut registry = SnapshotRegistry::new();
        registry.register::<TestPosition>();
        registry.register::<TestHealth>();

        let mut world = engine_ecs::world::World::new();
        let entity = world.spawn();
        world.add_component(
            entity,
            TestPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
        );
        world.add_component(entity, TestHealth(100));

        let snapshot = registry.snapshot_entities(&world, &[entity.index()], 1);
        let entity_data = &snapshot.entities[&entity.index()];
        assert_eq!(entity_data.len(), 2);
    }
}
