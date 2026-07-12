use crate::hierarchy::{Children, Parent};
use crate::node::SceneNode;
use crate::scene_layer::SceneLayer;
use crate::transform::{GlobalTransform, Transform};
use engine_ecs::entity::Entity;
use engine_ecs::gameobject::GameObjectHandle;
use engine_ecs::world::World;
use engine_math::Mat4;
use std::collections::HashMap;

/// Manages a scene graph of [`SceneNode`]s with hierarchical transforms.
///
/// Each node has a [`Transform`] (local) and a
/// [`GlobalTransform`] (world-space).
/// Call [`sync_transforms`](Self::sync_transforms) after modifying local
/// transforms to recompute the global ones.
pub struct SceneManager {
    world: World,
    root: SceneNode,
    names: Vec<String>,
    layers: SceneLayer,
    namespace: Option<String>,
    /// Mapping from Entity index to GameObjectHandle.
    entity_to_handle: HashMap<u32, GameObjectHandle>,
    /// Mapping from GameObjectHandle index to Entity.
    handle_to_entity: HashMap<u32, Entity>,
    /// Counter for generating unique GameObjectHandle indices.
    next_handle_index: u32,
}

impl SceneManager {
    /// Create a new scene manager with a single root node.
    pub fn new() -> Self {
        let mut world = World::new();
        let root_entity = world.spawn();
        world.add_component(root_entity, Children::new());
        world.add_component(root_entity, Transform::default());
        world.add_component(root_entity, GlobalTransform::default());

        let mut entity_to_handle = HashMap::new();
        let mut handle_to_entity = HashMap::new();
        let root_handle = GameObjectHandle::new(0, 0);
        entity_to_handle.insert(root_entity.index(), root_handle);
        handle_to_entity.insert(0, root_entity);

        let root = SceneNode::new(root_handle);
        let names = vec!["root".to_string()];
        Self {
            world,
            root,
            names,
            layers: SceneLayer::DEFAULT,
            namespace: None,
            entity_to_handle,
            handle_to_entity,
            next_handle_index: 1,
        }
    }

    /// Return the root node of the scene.
    pub fn root(&self) -> SceneNode {
        self.root
    }

    /// Resolve a [`SceneNode`] to its underlying ECS [`Entity`].
    ///
    /// This is useful for direct ECS world access when needed.
    pub fn resolve_entity(&self, node: SceneNode) -> Entity {
        self.resolve_entity_handle(node.gameobject())
    }

    fn resolve_entity_handle(&self, handle: GameObjectHandle) -> Entity {
        *self
            .handle_to_entity
            .get(&handle.index())
            .expect("SceneNode handle has no mapped Entity")
    }

    /// Begin building a new child node with the given `name`.
    ///
    /// The node is automatically parented to the root. Use
    /// [`SceneNodeBuilder::with_transform`] to set an initial transform,
    /// then call [`SceneNodeBuilder::build`] or convert with `Into`.
    pub fn add_node(&mut self, name: &str) -> SceneNodeBuilder<'_> {
        let entity = self.world.spawn();
        let handle_index = self.next_handle_index;
        self.next_handle_index += 1;
        let handle = GameObjectHandle::new(handle_index, 0);
        self.entity_to_handle.insert(entity.index(), handle);
        self.handle_to_entity.insert(handle_index, entity);

        let node = SceneNode::new(handle);
        let idx = entity.index() as usize;
        if idx >= self.names.len() {
            self.names.resize_with(idx + 1, String::new);
        }
        self.names[idx] = name.to_string();
        self.world.add_component(entity, Transform::default());
        self.world.add_component(entity, GlobalTransform::default());
        self.world.add_component(entity, Children::new());
        self.set_parent_internal(node, self.root);
        SceneNodeBuilder {
            scene_manager: self,
            node,
        }
    }

    fn set_parent_internal(&mut self, child: SceneNode, parent: SceneNode) {
        let child_entity = self.resolve_entity_handle(child.gameobject());
        let parent_entity = self.resolve_entity_handle(parent.gameobject());
        self.world
            .add_component(child_entity, Parent(parent_entity));
        if let Some(children) = self.world.get_mut::<Children>(parent_entity) {
            children.0.push(child_entity);
        }
    }

    /// Reparent `child` under `parent`, removing it from its previous parent.
    pub fn set_parent(&mut self, child: SceneNode, parent: SceneNode) {
        if let Some(old_parent) = self.parent(child) {
            let old_parent_entity = self.resolve_entity_handle(old_parent.gameobject());
            let child_entity = self.resolve_entity_handle(child.gameobject());
            if let Some(children) = self.world.get_mut::<Children>(old_parent_entity) {
                children.0.retain(|e| *e != child_entity);
            }
        }
        self.set_parent_internal(child, parent);
    }

    /// Return the parent of a node, or `None` for the root.
    pub fn parent(&self, node: SceneNode) -> Option<SceneNode> {
        let entity = self.resolve_entity_handle(node.gameobject());
        self.world.get::<Parent>(entity).and_then(|p| {
            self.entity_to_handle
                .get(&p.0.index())
                .map(|&h| SceneNode::new(h))
        })
    }

    /// Return the name of a node.
    pub fn name(&self, node: SceneNode) -> &str {
        let entity = self.resolve_entity_handle(node.gameobject());
        let idx = entity.index() as usize;
        if idx < self.names.len() {
            &self.names[idx]
        } else {
            ""
        }
    }

    /// Get a shared reference to a node's local transform.
    ///
    /// # Panics
    /// Panics if the entity does not have a `Transform` component (should never
    /// happen for nodes created via [`add_node`](Self::add_node)).
    pub fn transform(&self, node: SceneNode) -> &Transform {
        let entity = self.resolve_entity_handle(node.gameobject());
        self.world
            .get::<Transform>(entity)
            .expect("SceneNode entity must have Transform component")
    }

    /// Get an exclusive reference to a node's local transform.
    ///
    /// # Panics
    /// Panics if the entity does not have a `Transform` component (should never
    /// happen for nodes created via [`add_node`](Self::add_node)).
    pub fn transform_mut(&mut self, node: SceneNode) -> &mut Transform {
        let entity = self.resolve_entity_handle(node.gameobject());
        self.world
            .get_mut::<Transform>(entity)
            .expect("SceneNode entity must have Transform component")
    }

    /// Get mutable access to the underlying ECS world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get the scene layer mask.
    pub fn layers(&self) -> SceneLayer {
        self.layers
    }

    /// Set the scene layer mask.
    pub fn set_layers(&mut self, layers: SceneLayer) {
        self.layers = layers;
    }

    /// Get the entity namespace, if any.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Set the entity namespace.
    pub fn set_namespace(&mut self, namespace: Option<String>) {
        self.namespace = namespace;
    }

    /// Recompute all [`GlobalTransform`]s from the local transform hierarchy.
    pub fn sync_transforms(&mut self) {
        let root_entity = self.resolve_entity_handle(self.root.gameobject());
        let mut stack = vec![(root_entity, Mat4::IDENTITY)];
        while let Some((entity, parent_global)) = stack.pop() {
            let local_matrix = match self.world.get::<Transform>(entity) {
                Some(t) => t.to_matrix(),
                None => Mat4::IDENTITY,
            };
            let global = parent_global * local_matrix;
            if let Some(gt) = self.world.get_mut::<GlobalTransform>(entity) {
                gt.0 = global;
            }
            if let Some(children) = self.world.get::<Children>(entity) {
                for child in children.0.iter().rev() {
                    stack.push((*child, global));
                }
            }
        }
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring a newly created [`SceneNode`].
pub struct SceneNodeBuilder<'a> {
    scene_manager: &'a mut SceneManager,
    node: SceneNode,
}

impl<'a> SceneNodeBuilder<'a> {
    /// Set the local transform of the node being built.
    pub fn with_transform(self, transform: Transform) -> SceneNodeBuilder<'a> {
        *self.scene_manager.transform_mut(self.node) = transform;
        self
    }

    /// Add an existing node as a child of the node being built.
    pub fn with_child(self, child: SceneNode) -> SceneNodeBuilder<'a> {
        self.scene_manager.set_parent(child, self.node);
        self
    }

    /// Finish building and return the node.
    pub fn build(self) -> SceneNode {
        self.node
    }
}

impl<'a> From<SceneNodeBuilder<'a>> for SceneNode {
    fn from(builder: SceneNodeBuilder<'a>) -> Self {
        builder.node
    }
}

#[cfg(test)]
mod tests {
    use crate::scene_manager::SceneManager;
    use engine_math::Vec3;

    #[test]
    fn test_add_node() {
        let mut sm = SceneManager::new();
        let node: crate::node::SceneNode = sm.add_node("test").into();
        let name = sm.name(node);
        assert_eq!(name, "test");
    }

    #[test]
    fn test_node_parent_child() {
        let mut sm = SceneManager::new();
        let parent: crate::node::SceneNode = sm.add_node("parent").into();
        let child: crate::node::SceneNode = sm.add_node("child").into();
        sm.set_parent(child, parent);
        assert_eq!(sm.parent(child), Some(parent));
    }

    #[test]
    fn test_add_node_with_transform() {
        let mut sm = SceneManager::new();
        let node: crate::node::SceneNode = sm
            .add_node("camera")
            .with_transform(crate::transform::Transform::from_xyz(0.0, 5.0, 10.0))
            .into();
        let transform = sm.transform(node);
        assert_eq!(transform.translation, Vec3::new(0.0, 5.0, 10.0));
    }

    #[test]
    fn test_root_exists() {
        let sm = SceneManager::new();
        let root = sm.root();
        let name = sm.name(root);
        assert_eq!(name, "root");
    }
}
