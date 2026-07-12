use crate::gameobject::GameObjectHandle;
use crate::world::World;

/// System that synchronizes world transforms from local transforms.
/// Runs in the PreUpdate phase (before gameplay systems).
///
/// NOTE: This function allocates a Vec for root handles and a Vec for each
/// node's children on every call. For large hierarchies this may be worth
/// optimizing with a pre-allocated iterative approach, but for now the
/// recursive approach is simpler and sufficient.
pub fn sync_transforms(world: &mut World) {
    let roots: Vec<GameObjectHandle> = world.root_gameobjects(true);

    for root in roots {
        sync_transform_recursive(world, root, true);
    }
}

/// Recursively sync transform for a GameObject and its children.
fn sync_transform_recursive(world: &mut World, handle: GameObjectHandle, is_root: bool) {
    // Get children first
    let children = world.get_children(handle);

    // Get parent transform data before mutable borrow
    let parent_data = if is_root {
        None
    } else {
        world.get_parent(handle).and_then(|ph| {
            world.get_gameobject(ph).and_then(|parent_go| {
                parent_go
                    .get_component::<crate::transform::Transform>()
                    .map(|t| (t.position(), t.rotation(), t.lossy_scale()))
            })
        })
    };

    // Now get mutable access to update this transform
    if let Some(transform) = world
        .get_gameobject_mut(handle)
        .and_then(|go| go.get_component_mut::<crate::transform::Transform>())
    {
        if is_root {
            transform.update_world_transform_root();
        } else if let Some((parent_pos, parent_rot, parent_scale)) = parent_data {
            transform.update_world_transform(parent_pos, parent_rot, parent_scale);
        }
    }

    // Recursively sync children
    for child in children {
        sync_transform_recursive(world, child, false);
    }
}

/// Get all ancestors of a GameObject (from immediate parent to root).
pub fn get_ancestors(world: &World, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
    let mut ancestors = Vec::new();
    let mut current = world.get_parent(handle);

    while let Some(parent) = current {
        ancestors.push(parent);
        current = world.get_parent(parent);
    }

    ancestors
}

/// Get the root ancestor of a GameObject.
/// If the handle itself is invalid (no parent lookup succeeds), the handle is
/// returned as-is.
#[inline]
pub fn get_root(world: &World, handle: GameObjectHandle) -> GameObjectHandle {
    let mut current = handle;
    while let Some(parent) = world.get_parent(current) {
        current = parent;
    }
    current
}

/// Check if a GameObject is an ancestor of another.
#[inline]
pub fn is_ancestor(
    world: &World,
    ancestor: GameObjectHandle,
    descendant: GameObjectHandle,
) -> bool {
    let mut current = world.get_parent(descendant);
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        current = world.get_parent(parent);
    }
    false
}

/// Get the depth of a GameObject in the hierarchy (root = 0).
#[inline]
pub fn get_depth(world: &World, handle: GameObjectHandle) -> usize {
    let mut depth = 0;
    let mut current = world.get_parent(handle);

    while let Some(parent) = current {
        depth += 1;
        current = world.get_parent(parent);
    }

    depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameobject::GameObject;
    use crate::transform::Transform;
    use engine_math::Vec3;

    #[test]
    fn test_sync_root_transform() {
        let mut world = World::new();
        let mut go = GameObject::new("Root");
        go.add_component(Transform::from_xyz(1.0, 2.0, 3.0));
        let handle = world.spawn(go);

        sync_transforms(&mut world);

        let transform = world
            .get_gameobject(handle)
            .unwrap()
            .get_component::<Transform>()
            .unwrap();
        assert_eq!(transform.position(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_sync_child_transform_composes_with_parent() {
        let mut world = World::new();

        let mut root_go = GameObject::new("Root");
        root_go.add_component(Transform::from_xyz(5.0, 0.0, 0.0));
        let root = world.spawn(root_go);

        let mut child_go = GameObject::new("Child");
        child_go.add_component(Transform::from_xyz(1.0, 0.0, 0.0));
        let child = world.spawn(child_go);

        world.set_parent(child, Some(root));

        sync_transforms(&mut world);

        let root_t = world
            .get_gameobject(root)
            .unwrap()
            .get_component::<Transform>()
            .unwrap();
        assert_eq!(root_t.position(), Vec3::new(5.0, 0.0, 0.0));

        let child_t = world
            .get_gameobject(child)
            .unwrap()
            .get_component::<Transform>()
            .unwrap();
        assert_eq!(child_t.position(), Vec3::new(6.0, 0.0, 0.0));
    }

    #[test]
    fn test_get_ancestors() {
        let mut world = World::new();

        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));

        world.set_parent(child, Some(root));
        world.set_parent(grandchild, Some(child));

        let ancestors = get_ancestors(&world, grandchild);
        assert_eq!(ancestors, vec![child, root]);
    }

    #[test]
    fn test_get_root() {
        let mut world = World::new();

        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));

        world.set_parent(child, Some(root));
        world.set_parent(grandchild, Some(child));

        assert_eq!(get_root(&world, grandchild), root);
    }

    #[test]
    fn test_is_ancestor() {
        let mut world = World::new();

        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));

        world.set_parent(child, Some(root));
        world.set_parent(grandchild, Some(child));

        assert!(is_ancestor(&world, root, grandchild));
        assert!(is_ancestor(&world, child, grandchild));
        assert!(!is_ancestor(&world, grandchild, root));
    }

    #[test]
    fn test_get_depth() {
        let mut world = World::new();

        let root = world.spawn(GameObject::new("Root"));
        let child = world.spawn(GameObject::new("Child"));
        let grandchild = world.spawn(GameObject::new("Grandchild"));

        world.set_parent(child, Some(root));
        world.set_parent(grandchild, Some(child));

        assert_eq!(get_depth(&world, root), 0);
        assert_eq!(get_depth(&world, child), 1);
        assert_eq!(get_depth(&world, grandchild), 2);
    }
}
