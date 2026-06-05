use crate::hierarchy::{Children, Parent};
use crate::node::SceneNode;
use crate::scene_layer::SceneLayer;
use crate::transform::{GlobalTransform, Transform};
use engine_ecs::world::World;
use engine_math::Mat4;

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
}

impl SceneManager {
    /// Create a new scene manager with a single root node.
    pub fn new() -> Self {
        let mut world = World::new();
        let root_entity = world.spawn();
        world.add_component(root_entity, Children::new());
        world.add_component(root_entity, Transform::default());
        world.add_component(root_entity, GlobalTransform::default());
        let root = SceneNode::new(root_entity);
        let names = vec!["root".to_string()];
        Self {
            world,
            root,
            names,
            layers: SceneLayer::DEFAULT,
            namespace: None,
        }
    }

    /// Return the root node of the scene.
    pub fn root(&self) -> SceneNode {
        self.root
    }

    /// Begin building a new child node with the given `name`.
    ///
    /// The node is automatically parented to the root. Use
    /// [`SceneNodeBuilder::with_transform`] to set an initial transform,
    /// then call [`SceneNodeBuilder::build`] or convert with `Into`.
    pub fn add_node(&mut self, name: &str) -> SceneNodeBuilder<'_> {
        let entity = self.world.spawn();
        let node = SceneNode::new(entity);
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
        self.world
            .add_component(child.entity(), Parent(parent.entity()));
        if let Some(children) = self.world.get_mut::<Children>(parent.entity()) {
            children.0.push(child.entity());
        }
    }

    /// Reparent `child` under `parent`, removing it from its previous parent.
    pub fn set_parent(&mut self, child: SceneNode, parent: SceneNode) {
        if let Some(old_parent) = self.parent(child)
            && let Some(children) = self.world.get_mut::<Children>(old_parent.entity())
        {
            children.0.retain(|e| *e != child.entity());
        }
        self.set_parent_internal(child, parent);
    }

    /// Return the parent of a node, or `None` for the root.
    pub fn parent(&self, node: SceneNode) -> Option<SceneNode> {
        self.world
            .get::<Parent>(node.entity())
            .map(|p| SceneNode::new(p.0))
    }

    /// Return the name of a node.
    pub fn name(&self, node: SceneNode) -> &str {
        let idx = node.entity().index() as usize;
        if idx < self.names.len() {
            &self.names[idx]
        } else {
            ""
        }
    }

    /// Get a shared reference to a node's local transform.
    pub fn transform(&self, node: SceneNode) -> &Transform {
        self.world
            .get::<Transform>(node.entity())
            .expect("SceneNode entity must have Transform component")
    }

    /// Get an exclusive reference to a node's local transform.
    pub fn transform_mut(&mut self, node: SceneNode) -> &mut Transform {
        self.world
            .get_mut::<Transform>(node.entity())
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
        let root_entity = self.root.entity();
        let mut stack = vec![(root_entity, Mat4::IDENTITY)];
        while let Some((entity, parent_global)) = stack.pop() {
            let local_matrix = self
                .world
                .get::<Transform>(entity)
                .map(|t| t.to_matrix())
                .unwrap_or(Mat4::IDENTITY);
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
