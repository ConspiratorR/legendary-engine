//! Transform hierarchy utilities.
//!
//! Provides helper functions for working with the Transform hierarchy.
//! These functions use Transform's built-in parent/child relationships.

use crate::gameobject::GameObjectHandle;
use crate::world::World;

/// Synchronize world transforms from local transforms.
///
/// This system walks the hierarchy from roots to leaves, computing
/// world position, rotation, and scale from local values and parent transforms.
///
/// Should be called once per frame before any systems that read world-space transform values.
pub fn sync_transforms(world: &mut World) {
    world.sync_transforms();
}

/// Get all ancestors of a GameObject (from immediate parent to root).
pub fn get_ancestors(world: &World, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
    let mut ancestors = Vec::new();
    let mut current = world.GetParent(handle);

    while let Some(parent) = current {
        ancestors.push(parent);
        current = world.GetParent(parent);
    }

    ancestors
}

/// Get the root ancestor of a GameObject.
///
/// If the handle itself is a root (no parent), the handle is returned as-is.
pub fn get_root(world: &World, handle: GameObjectHandle) -> GameObjectHandle {
    let mut current = handle;
    while let Some(parent) = world.GetParent(current) {
        current = parent;
    }
    current
}

/// Check if a GameObject is an ancestor of another.
pub fn is_ancestor(
    world: &World,
    ancestor: GameObjectHandle,
    descendant: GameObjectHandle,
) -> bool {
    let mut current = world.GetParent(descendant);
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        current = world.GetParent(parent);
    }
    false
}

/// Get the depth of a GameObject in the hierarchy (root = 0).
pub fn get_depth(world: &World, handle: GameObjectHandle) -> usize {
    let mut depth = 0;
    let mut current = world.GetParent(handle);

    while let Some(parent) = current {
        depth += 1;
        current = world.GetParent(parent);
    }

    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_root_transform() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Root");

        // Set local position via transform
        if let Some(t) = world.GetTransformMut(handle) {
            t.SetLocalPosition(engine_math::Vec3::new(1.0, 2.0, 3.0));
        }

        sync_transforms(&mut world);

        let transform = world.GetTransform(handle).unwrap();
        assert_eq!(transform.Position(), engine_math::Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_sync_child_transform_composes_with_parent() {
        let mut world = World::new();

        let parent = world.CreateGameObject("Parent");
        let child = world.CreateGameObject("Child");

        world.SetParent(child, Some(parent));

        if let Some(t) = world.GetTransformMut(parent) {
            t.SetLocalPosition(engine_math::Vec3::new(5.0, 0.0, 0.0));
        }
        if let Some(t) = world.GetTransformMut(child) {
            t.SetLocalPosition(engine_math::Vec3::new(1.0, 0.0, 0.0));
        }

        sync_transforms(&mut world);

        let parent_pos = world.GetTransform(parent).unwrap().Position();
        let child_pos = world.GetTransform(child).unwrap().Position();

        assert_eq!(parent_pos, engine_math::Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(child_pos, engine_math::Vec3::new(6.0, 0.0, 0.0));
    }

    #[test]
    fn test_get_ancestors() {
        let mut world = World::new();

        let root = world.CreateGameObject("Root");
        let child = world.CreateGameObject("Child");
        let grandchild = world.CreateGameObject("Grandchild");

        world.SetParent(child, Some(root));
        world.SetParent(grandchild, Some(child));

        let ancestors = get_ancestors(&world, grandchild);
        assert_eq!(ancestors, vec![child, root]);
    }

    #[test]
    fn test_get_root() {
        let mut world = World::new();

        let root = world.CreateGameObject("Root");
        let child = world.CreateGameObject("Child");
        let grandchild = world.CreateGameObject("Grandchild");

        world.SetParent(child, Some(root));
        world.SetParent(grandchild, Some(child));

        assert_eq!(get_root(&world, grandchild), root);
    }

    #[test]
    fn test_is_ancestor() {
        let mut world = World::new();

        let root = world.CreateGameObject("Root");
        let child = world.CreateGameObject("Child");
        let grandchild = world.CreateGameObject("Grandchild");

        world.SetParent(child, Some(root));
        world.SetParent(grandchild, Some(child));

        assert!(is_ancestor(&world, root, grandchild));
        assert!(is_ancestor(&world, child, grandchild));
        assert!(!is_ancestor(&world, grandchild, root));
    }

    #[test]
    fn test_get_depth() {
        let mut world = World::new();

        let root = world.CreateGameObject("Root");
        let child = world.CreateGameObject("Child");
        let grandchild = world.CreateGameObject("Grandchild");

        world.SetParent(child, Some(root));
        world.SetParent(grandchild, Some(child));

        assert_eq!(get_depth(&world, root), 0);
        assert_eq!(get_depth(&world, child), 1);
        assert_eq!(get_depth(&world, grandchild), 2);
    }
}
