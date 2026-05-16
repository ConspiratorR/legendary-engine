use crate::component::ComponentRegistry;
use crate::entity::Entity;

pub struct World {
    next_index: u32,
    free_list: Vec<u32>,
    generations: Vec<u32>,
    components: ComponentRegistry,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            next_index: 0,
            free_list: Vec::new(),
            generations: Vec::new(),
            components: ComponentRegistry::new(),
        }
    }

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

    pub fn despawn(&mut self, entity: Entity) {
        let idx = entity.index();
        if idx as usize >= self.generations.len() {
            return;
        }
        self.generations[idx as usize] = entity.generation() + 1;
        self.free_list.push(idx);
    }

    pub fn add_component<T: 'static>(&mut self, entity: Entity, component: T) {
        self.components
            .storage::<T>()
            .insert(entity.index(), component);
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        if self.generations.get(entity.index() as usize).copied()? != entity.generation() {
            return None;
        }
        let storage = self.components.try_get_storage::<T>()?;
        storage.get(entity.index())
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        if self.generations.get(entity.index() as usize).copied()? != entity.generation() {
            return None;
        }
        let storage = self.components.try_get_storage_mut::<T>()?;
        storage.get_mut(entity.index())
    }

    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let storage = self.components.try_get_storage_mut::<T>()?;
        storage.remove(entity.index())
    }

    pub fn component_entities<T: 'static>(&self) -> Vec<u32> {
        self.components
            .try_get_storage::<T>()
            .map(|s| s.entities().to_vec())
            .unwrap_or_default()
    }

    pub fn get_by_index<T: 'static>(&self, index: u32) -> Option<&T> {
        self.components.try_get_storage::<T>()?.get(index)
    }

    pub fn get_by_index_mut<T: 'static>(&mut self, index: u32) -> Option<&mut T> {
        self.components.try_get_storage_mut::<T>()?.get_mut(index)
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
