use crate::gameobject::{GameObject, GameObjectHandle};
use std::collections::HashMap;

/// Central container for all GameObjects (replaces ECS World).
pub struct World {
    gameobjects: Vec<Option<GameObject>>,
    generations: Vec<u32>,
    free_list: Vec<u32>,
    name_to_handles: HashMap<String, Vec<GameObjectHandle>>,
}

impl World {
    /// Create a new empty World.
    pub fn new() -> Self {
        Self {
            gameobjects: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
            name_to_handles: HashMap::new(),
        }
    }

    /// Spawn a new GameObject (like Unity's Instantiate).
    pub fn spawn(&mut self, gameobject: GameObject) -> GameObjectHandle {
        let name = gameobject.name().to_string();

        let handle = if let Some(index) = self.free_list.pop() {
            let index_usize = index as usize;
            let generation = self.generations[index_usize];
            self.gameobjects[index_usize] = Some(gameobject);
            self.generations[index_usize] = generation + 1;
            GameObjectHandle::new(index, generation + 1)
        } else {
            let index = self.gameobjects.len() as u32;
            self.gameobjects.push(Some(gameobject));
            self.generations.push(0);
            GameObjectHandle::new(index, 0)
        };

        self.name_to_handles.entry(name).or_default().push(handle);
        handle
    }

    /// Despawn a GameObject (like Unity's Destroy).
    pub fn despawn(&mut self, handle: GameObjectHandle) -> Option<GameObject> {
        if self.is_valid(handle) {
            let index = handle.index() as usize;
            let gameobject = self.gameobjects[index].take();

            // Remove from name map
            if let Some(go) = &gameobject
                && let Some(handles) = self.name_to_handles.get_mut(go.name())
            {
                handles.retain(|&h| h != handle);
                if handles.is_empty() {
                    self.name_to_handles.remove(go.name());
                }
            }

            // Add to free list
            self.free_list.push(index as u32);

            gameobject
        } else {
            None
        }
    }

    /// Check if a handle is valid.
    pub fn is_valid(&self, handle: GameObjectHandle) -> bool {
        let index = handle.index() as usize;
        index < self.gameobjects.len()
            && self.gameobjects[index].is_some()
            && self.generations[index] == handle.generation()
    }

    /// Get a reference to a GameObject.
    pub fn get_gameobject(&self, handle: GameObjectHandle) -> Option<&GameObject> {
        if self.is_valid(handle) {
            self.gameobjects[handle.index() as usize].as_ref()
        } else {
            None
        }
    }

    /// Get a mutable reference to a GameObject.
    pub fn get_gameobject_mut(&mut self, handle: GameObjectHandle) -> Option<&mut GameObject> {
        if self.is_valid(handle) {
            self.gameobjects[handle.index() as usize].as_mut()
        } else {
            None
        }
    }

    /// Find a GameObject by name. Returns the first match if duplicates exist.
    pub fn find_gameobject(&self, name: &str) -> Option<GameObjectHandle> {
        self.name_to_handles
            .get(name)
            .and_then(|v| v.first())
            .copied()
    }

    /// Find all GameObjects with a specific tag.
    /// If `include_inactive` is false, only active GameObjects are returned.
    pub fn find_gameobjects_with_tag(
        &self,
        tag: &str,
        include_inactive: bool,
    ) -> Vec<GameObjectHandle> {
        self.gameobjects
            .iter()
            .enumerate()
            .filter_map(|(i, go)| {
                go.as_ref()
                    .filter(|g| g.tag() == tag && (include_inactive || g.is_active()))
                    .map(|_| GameObjectHandle::new(i as u32, self.generations[i]))
            })
            .collect()
    }

    /// Get all root GameObjects (no parent).
    /// If `include_inactive` is false, only active root GameObjects are returned.
    pub fn root_gameobjects(&self, include_inactive: bool) -> Vec<GameObjectHandle> {
        self.gameobjects
            .iter()
            .enumerate()
            .filter_map(|(i, go)| {
                go.as_ref()
                    .filter(|g| g.parent().is_none() && (include_inactive || g.is_active()))
                    .map(|_| GameObjectHandle::new(i as u32, self.generations[i]))
            })
            .collect()
    }

    /// Get all GameObjects.
    pub fn all_gameobjects(&self) -> Vec<GameObjectHandle> {
        self.gameobjects
            .iter()
            .enumerate()
            .filter_map(|(i, go)| {
                go.as_ref()
                    .map(|_| GameObjectHandle::new(i as u32, self.generations[i]))
            })
            .collect()
    }

    /// Get the number of active GameObjects.
    pub fn count(&self) -> usize {
        self.gameobjects.iter().filter(|go| go.is_some()).count()
    }

    /// Set parent of a GameObject.
    /// Pass `None` to detach from the current parent.
    pub fn set_parent(&mut self, child: GameObjectHandle, parent: Option<GameObjectHandle>) {
        if !self.is_valid(child) {
            return;
        }

        // Validate new parent handle
        if let Some(new_parent) = parent {
            if !self.is_valid(new_parent) {
                return;
            }
            // Cycle detection: check if new_parent is a descendant of child
            if self.is_descendant_of(new_parent, child) {
                return;
            }
        }

        // Remove from old parent's children list
        if let Some(old_parent) = self.get_gameobject(child).and_then(|go| go.parent())
            && let Some(parent_go) = self.get_gameobject_mut(old_parent)
        {
            parent_go.children.retain(|&h| h != child);
        }

        // Set new parent
        if let Some(parent_go) = self.get_gameobject_mut(child) {
            parent_go.parent = parent;
        }

        // Add to new parent's children list
        if let Some(new_parent) = parent
            && let Some(parent_go) = self.get_gameobject_mut(new_parent)
        {
            parent_go.children.push(child);
        }
    }

    /// Check if `candidate` is a descendant of `ancestor`.
    fn is_descendant_of(&self, candidate: GameObjectHandle, ancestor: GameObjectHandle) -> bool {
        let mut current = candidate;
        loop {
            if current == ancestor {
                return true;
            }
            match self.get_gameobject(current).and_then(|go| go.parent()) {
                Some(parent) => current = parent,
                None => return false,
            }
        }
    }

    /// Get children of a GameObject.
    pub fn get_children(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
        self.get_gameobject(handle)
            .map(|go| go.children().to_vec())
            .unwrap_or_default()
    }

    /// Get parent of a GameObject.
    pub fn get_parent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> {
        self.get_gameobject(handle).and_then(|go| go.parent())
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("count", &self.count())
            .field("free_slots", &self.free_list.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_spawn() {
        let mut world = World::new();
        let go = GameObject::new("Player");
        let handle = world.spawn(go);

        assert!(world.is_valid(handle));
        assert_eq!(world.get_gameobject(handle).unwrap().name(), "Player");
    }

    #[test]
    fn test_world_despawn() {
        let mut world = World::new();
        let go = GameObject::new("Player");
        let handle = world.spawn(go);

        let removed = world.despawn(handle);
        assert!(removed.is_some());
        assert!(!world.is_valid(handle));
    }

    #[test]
    fn test_world_find_by_name() {
        let mut world = World::new();
        let go = GameObject::new("Player");
        let handle = world.spawn(go);

        let found = world.find_gameobject("Player");
        assert_eq!(found, Some(handle));
    }

    #[test]
    fn test_world_find_duplicate_names() {
        let mut world = World::new();
        let h1 = world.spawn(GameObject::new("Enemy"));
        let h2 = world.spawn(GameObject::new("Enemy"));

        // Both exist in the world
        assert!(world.is_valid(h1));
        assert!(world.is_valid(h2));

        // find_gameobject returns the first one
        let found = world.find_gameobject("Enemy");
        assert_eq!(found, Some(h1));

        // Despawning one doesn't break the other
        world.despawn(h1);
        assert!(!world.is_valid(h1));
        assert!(world.is_valid(h2));
        assert_eq!(world.find_gameobject("Enemy"), Some(h2));
    }

    #[test]
    fn test_world_parent_child() {
        let mut world = World::new();

        let parent = world.spawn(GameObject::new("Parent"));
        let child = world.spawn(GameObject::new("Child"));

        world.set_parent(child, Some(parent));

        assert_eq!(world.get_parent(child), Some(parent));
        assert!(world.get_children(parent).contains(&child));
    }

    #[test]
    fn test_world_set_parent_invalid_parent() {
        let mut world = World::new();
        let child = world.spawn(GameObject::new("Child"));

        // Create a handle that is not valid
        let invalid = GameObjectHandle::new(99, 0);
        world.set_parent(child, Some(invalid));

        // Child should have no parent
        assert_eq!(world.get_parent(child), None);
    }

    #[test]
    fn test_world_set_parent_cycle_detection() {
        let mut world = World::new();

        let parent = world.spawn(GameObject::new("Parent"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));

        world.set_parent(child, Some(parent));
        world.set_parent(grandchild, Some(child));

        // Attempt to set parent to grandchild -> would create a cycle
        world.set_parent(parent, Some(grandchild));

        // Parent should still be a root
        assert_eq!(world.get_parent(parent), None);
    }

    #[test]
    fn test_world_recycle_slot() {
        let mut world = World::new();

        let go1 = world.spawn(GameObject::new("First"));
        let _go2 = world.spawn(GameObject::new("Second"));

        world.despawn(go1);

        let go3 = world.spawn(GameObject::new("Third"));

        // go3 should reuse go1's slot
        assert_eq!(go3.index(), go1.index());
        assert_ne!(go3.generation(), go1.generation());
    }

    #[test]
    fn test_world_root_gameobjects() {
        let mut world = World::new();
        let root1 = world.spawn(GameObject::new("Root1"));
        let root2 = world.spawn(GameObject::new("Root2"));
        let child = world.spawn(GameObject::new("Child"));

        world.set_parent(child, Some(root1));

        let roots = world.root_gameobjects(true);
        assert!(roots.contains(&root1));
        assert!(roots.contains(&root2));
        assert!(!roots.contains(&child));
    }

    #[test]
    fn test_world_find_gameobjects_with_tag() {
        let mut world = World::new();
        let mut go1 = GameObject::new("A");
        go1.set_tag("enemy");
        let h1 = world.spawn(go1);

        let mut go2 = GameObject::new("B");
        go2.set_tag("enemy");
        let h2 = world.spawn(go2);

        let mut go3 = GameObject::new("C");
        go3.set_tag("player");
        let h3 = world.spawn(go3);

        let enemies = world.find_gameobjects_with_tag("enemy", true);
        assert!(enemies.contains(&h1));
        assert!(enemies.contains(&h2));
        assert!(!enemies.contains(&h3));
    }
}
