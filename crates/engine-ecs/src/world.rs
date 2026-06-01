use crate::component::ComponentRegistry;
use crate::entity::Entity;
use std::any::TypeId;
use std::collections::HashMap;

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
    resources: HashMap<TypeId, Box<dyn std::any::Any>>,
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
    }

    /// Attach a component to an entity.
    pub fn add_component<T: 'static>(&mut self, entity: Entity, component: T) {
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
    pub fn insert_resource<T: 'static>(&mut self, resource: T) {
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
}
