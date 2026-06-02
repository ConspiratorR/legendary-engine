use crate::component::ComponentRegistry;
use crate::entity::Entity;
use std::any::TypeId;
use std::collections::HashMap;

/// Summary of memory usage for a single component type.
#[derive(Debug, Clone)]
pub struct ComponentMemoryInfo {
    pub type_id: TypeId,
    pub live_entries: usize,
    pub wasted_slots: usize,
}

/// The central ECS container.
///
/// `World` owns all entities, their components, and global resources.
/// Entities are created with [`spawn`](Self::spawn) and destroyed with
/// [`despawn`](Self::despawn). Components are attached with
/// [`add_component`](Self::add_component) and queried with
/// [`get`](Self::get) / [`get_mut`](Self::get_mut).
///
/// # Example
///
/// ```
/// use engine_ecs::world::World;
///
/// struct Health(i32);
///
/// let mut world = World::new();
/// let entity = world.spawn();
/// world.add_component(entity, Health(100));
///
/// if let Some(health) = world.get::<Health>(entity) {
///     assert_eq!(health.0, 100);
/// }
/// ```
pub struct World {
    next_index: u32,
    free_list: Vec<u32>,
    generations: Vec<u32>,
    components: ComponentRegistry,
    resources: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
    /// Total number of entities that have ever been spawned (for metrics).
    total_spawned: u64,
    /// Total number of entities that have been despawned (for metrics).
    total_despawned: u64,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Create an empty world with no entities or resources.
    pub fn new() -> Self {
        Self {
            next_index: 0,
            free_list: Vec::new(),
            generations: Vec::new(),
            components: ComponentRegistry::new(),
            resources: HashMap::new(),
            total_spawned: 0,
            total_despawned: 0,
        }
    }

    /// Spawn a new entity and return its handle.
    pub fn spawn(&mut self) -> Entity {
        let index = if let Some(free) = self.free_list.pop() {
            free
        } else {
            let i = self.next_index;
            self.next_index += 1;
            self.generations.push(0);
            i
        };
        self.total_spawned += 1;
        Entity::new(index, self.generations[index as usize])
    }

    /// Despawn an entity, removing all its components.
    ///
    /// After this call the entity handle is invalid and will not match
    /// any future entity reusing the same index.
    pub fn despawn(&mut self, entity: Entity) {
        let idx = entity.index();
        if idx as usize >= self.generations.len() {
            return;
        }
        self.components.remove_entity(idx);
        self.generations[idx as usize] = entity.generation() + 1;
        self.free_list.push(idx);
        self.total_despawned += 1;
    }

    /// Attach a component to an entity.
    pub fn add_component<T: Send + Sync + 'static>(&mut self, entity: Entity, component: T) {
        self.components
            .storage::<T>()
            .insert(entity.index(), component);
    }

    /// Get a shared reference to a component on an entity.
    ///
    /// Returns `None` if the entity does not exist, has been despawned,
    /// or does not have the requested component.
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        if self.generations.get(entity.index() as usize).copied()? != entity.generation() {
            return None;
        }
        let storage = self.components.try_get_storage::<T>()?;
        storage.get(entity.index())
    }

    /// Get an exclusive reference to a component on an entity.
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        if self.generations.get(entity.index() as usize).copied()? != entity.generation() {
            return None;
        }
        let storage = self.components.try_get_storage_mut::<T>()?;
        storage.get_mut(entity.index())
    }

    /// Remove a component from an entity, returning it if present.
    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let storage = self.components.try_get_storage_mut::<T>()?;
        storage.remove(entity.index())
    }

    /// Return the entity indices that have a component of type `T`.
    pub fn component_entities<T: 'static>(&self) -> Vec<u32> {
        self.components
            .try_get_storage::<T>()
            .map(|s| s.entities().to_vec())
            .unwrap_or_default()
    }

    /// Get a component by raw entity index (bypasses generation check).
    pub fn get_by_index<T: 'static>(&self, index: u32) -> Option<&T> {
        self.components.try_get_storage::<T>()?.get(index)
    }

    /// Get a mutable component by raw entity index (bypasses generation check).
    pub fn get_by_index_mut<T: 'static>(&mut self, index: u32) -> Option<&mut T> {
        self.components.try_get_storage_mut::<T>()?.get_mut(index)
    }

    /// Insert a global resource (singleton) of type `T`.
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    /// Get a shared reference to a global resource.
    pub fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Get an exclusive reference to a global resource.
    pub fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<T>()
    }

    /// Remove a global resource, returning it if present.
    ///
    /// This is useful when a system needs exclusive access to a resource
    /// while also accessing other resources from the same world.
    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> {
        self.resources
            .remove(&TypeId::of::<T>())?
            .downcast::<T>()
            .ok()
            .map(|b| *b)
    }

    // -------------------------------------------------------------------------
    // Memory management
    // -------------------------------------------------------------------------

    /// Compact all component storages, reclaiming unused sparse slots.
    ///
    /// Returns the total number of freed slots across all storages.
    pub fn compact_all(&mut self) -> usize {
        self.components.compact_all()
    }

    /// Return a memory usage summary for all component storages.
    pub fn memory_summary(&self) -> Vec<ComponentMemoryInfo> {
        self.components
            .memory_summary()
            .into_iter()
            .map(|(tid, live, wasted)| ComponentMemoryInfo {
                type_id: tid,
                live_entries: live,
                wasted_slots: wasted,
            })
            .collect()
    }

    /// Number of currently alive entities (spawned - despawned + free).
    pub fn entity_count(&self) -> u32 {
        self.next_index - self.free_list.len() as u32
    }

    /// Total entities spawned over the lifetime of this world.
    pub fn total_spawned(&self) -> u64 {
        self.total_spawned
    }

    /// Total entities despawned over the lifetime of this world.
    pub fn total_despawned(&self) -> u64 {
        self.total_despawned
    }

    /// Return a compact report string of memory usage.
    pub fn memory_report(&self) -> String {
        let summary = self.memory_summary();
        let total_wasted: usize = summary.iter().map(|s| s.wasted_slots).sum();
        let total_live: usize = summary.iter().map(|s| s.live_entries).sum();

        let mut report = String::new();
        report.push_str(&format!(
            "World Memory Report:\n\
             \x20 Entities: {} alive ({} spawned, {} despawned, {} free slots)\n\
             \x20 Components: {} live entries, {} wasted sparse slots\n",
            self.entity_count(),
            self.total_spawned,
            self.total_despawned,
            self.free_list.len(),
            total_live,
            total_wasted,
        ));

        if !summary.is_empty() {
            report.push_str("  Per-type:\n");
            for info in &summary {
                report.push_str(&format!(
                    "    {:?}: {} live, {} wasted\n",
                    info.type_id, info.live_entries, info.wasted_slots
                ));
            }
        }

        report
    }
}

impl Drop for World {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            let alive = self.entity_count();
            if alive > 0 {
                log::warn!(
                    "World dropped with {alive} entities still alive \
                     ({} spawned, {} despawned)",
                    self.total_spawned,
                    self.total_despawned
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use crate::world::World;

    struct Position(f32, f32, f32);
    struct Velocity(f32, f32);

    #[test]
    fn test_spawn_and_get_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Position(1.0, 2.0, 3.0));
        let pos = world.get::<Position>(e);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().0, 1.0);
    }

    #[test]
    fn test_spawn_without_component() {
        let mut world = World::new();
        let e = world.spawn();
        let pos = world.get::<Position>(e);
        assert!(pos.is_none());
    }

    #[test]
    fn test_despawn_removes_component_access() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Position(1.0, 2.0, 3.0));
        world.despawn(e);
        let pos = world.get::<Position>(e);
        assert!(pos.is_none());
    }

    #[test]
    fn test_entity_reuse() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.despawn(e1);
        let e3 = world.spawn();
        assert_ne!(e3, e1);
        assert_ne!(e3, e2);
    }

    #[test]
    fn test_compact_all() {
        let mut world = World::new();
        // Spawn entities at various indices
        let entities: Vec<_> = (0..50).map(|_| world.spawn()).collect();

        // Add components to some
        for &e in &entities[..25] {
            world.add_component(e, Position(0.0, 0.0, 0.0));
        }

        // Despawn half — creates wasted sparse slots
        for &e in &entities[..25] {
            world.despawn(e);
        }

        let summary_before = world.memory_summary();
        let wasted_before: usize = summary_before.iter().map(|s| s.wasted_slots).sum();
        assert!(wasted_before > 0);

        let freed = world.compact_all();
        assert!(freed > 0);

        let summary_after = world.memory_summary();
        let wasted_after: usize = summary_after.iter().map(|s| s.wasted_slots).sum();
        assert!(wasted_after < wasted_before);
    }

    #[test]
    fn test_memory_summary() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.add_component(e1, Position(1.0, 2.0, 3.0));
        world.add_component(e2, Velocity(1.0, 0.0));
        world.add_component(e1, Velocity(0.0, 1.0));

        let summary = world.memory_summary();
        assert_eq!(summary.len(), 2); // Position and Velocity storages
        let total_live: usize = summary.iter().map(|s| s.live_entries).sum();
        assert_eq!(total_live, 3); // 1 Position + 2 Velocity
    }

    #[test]
    fn test_entity_count() {
        let mut world = World::new();
        assert_eq!(world.entity_count(), 0);

        let e1 = world.spawn();
        let e2 = world.spawn();
        assert_eq!(world.entity_count(), 2);

        world.despawn(e1);
        assert_eq!(world.entity_count(), 1);

        world.despawn(e2);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_spawn_despawn_counters() {
        let mut world = World::new();
        let e1 = world.spawn();
        let _e2 = world.spawn();
        world.despawn(e1);
        let _e3 = world.spawn();

        assert_eq!(world.total_spawned(), 3);
        assert_eq!(world.total_despawned(), 1);
    }

    #[test]
    fn test_memory_report() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Position(1.0, 2.0, 3.0));

        let report = world.memory_report();
        assert!(report.contains("World Memory Report"));
        assert!(report.contains("Entities:"));
        assert!(report.contains("Components:"));
    }

    #[test]
    fn test_stress_10k_entities_no_leak() {
        let mut world = World::new();

        // Spawn 10,000 entities with components
        let entities: Vec<_> = (0..10_000)
            .map(|i| {
                let e = world.spawn();
                world.add_component(e, Position(i as f32, 0.0, 0.0));
                world.add_component(e, Velocity(1.0, 0.0));
                e
            })
            .collect();

        assert_eq!(world.entity_count(), 10_000);
        assert_eq!(world.total_spawned(), 10_000);

        // Verify all components are accessible
        for (i, &e) in entities.iter().enumerate() {
            let pos = world.get::<Position>(e).unwrap();
            assert_eq!(pos.0, i as f32);
            assert!(world.get::<Velocity>(e).is_some());
        }

        // Despawn all
        for e in entities {
            world.despawn(e);
        }

        assert_eq!(world.entity_count(), 0);
        assert_eq!(world.total_despawned(), 10_000);

        // Compact to reclaim memory
        let freed = world.compact_all();
        assert!(freed > 0);

        // After compaction, wasted slots should be minimal
        let summary = world.memory_summary();
        let total_wasted: usize = summary.iter().map(|s| s.wasted_slots).sum();
        assert_eq!(
            total_wasted, 0,
            "all sparse slots should be reclaimed after compaction"
        );
    }

    #[test]
    fn test_stress_repeated_spawn_despawn_cycles() {
        let mut world = World::new();

        // Simulate 1000 cycles of spawn/despawn
        for cycle in 0..1000 {
            let e = world.spawn();
            world.add_component(e, Position(cycle as f32, 0.0, 0.0));
            world.despawn(e);
        }

        assert_eq!(world.entity_count(), 0);
        assert_eq!(world.total_spawned(), 1000);
        assert_eq!(world.total_despawned(), 1000);

        // Compact and verify
        world.compact_all();
        let summary = world.memory_summary();
        let total_wasted: usize = summary.iter().map(|s| s.wasted_slots).sum();
        assert_eq!(total_wasted, 0);
    }

    #[test]
    fn test_compact_preserves_existing_data() {
        let mut world = World::new();

        let e1 = world.spawn();
        let e2 = world.spawn();
        world.add_component(e1, Position(1.0, 2.0, 3.0));
        world.add_component(e2, Position(4.0, 5.0, 6.0));

        // Despawn e1, compact
        world.despawn(e1);
        world.compact_all();

        // e2's data should still be accessible
        let pos = world.get::<Position>(e2).unwrap();
        assert_eq!(pos.0, 4.0);
        assert_eq!(pos.1, 5.0);
        assert_eq!(pos.2, 6.0);
    }
}
