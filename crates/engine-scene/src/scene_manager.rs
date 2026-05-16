use crate::hierarchy::{Children, Parent};
use crate::node::SceneNode;
use crate::transform::{GlobalTransform, Transform};
use engine_ecs::world::World;

pub struct SceneManager {
    world: World,
    root: SceneNode,
    names: Vec<String>,
}

impl SceneManager {
    pub fn new() -> Self {
        let mut world = World::new();
        let root_entity = world.spawn();
        world.add_component(root_entity, Children::new());
        world.add_component(root_entity, Transform::default());
        world.add_component(root_entity, GlobalTransform::default());
        let root = SceneNode::new(root_entity);
        let names = vec!["root".to_string()];
        Self { world, root, names }
    }

    pub fn root(&self) -> SceneNode {
        self.root
    }

    pub fn add_node(&mut self, name: &str) -> SceneNodeBuilder {
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
            scene_manager: self as *mut _,
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

    pub fn set_parent(&mut self, child: SceneNode, parent: SceneNode) {
        if let Some(old_parent) = self.parent(child)
            && let Some(children) = self.world.get_mut::<Children>(old_parent.entity())
        {
            children.0.retain(|e| *e != child.entity());
        }
        self.set_parent_internal(child, parent);
    }

    pub fn parent(&self, node: SceneNode) -> Option<SceneNode> {
        self.world
            .get::<Parent>(node.entity())
            .map(|p| SceneNode::new(p.0))
    }

    pub fn name(&self, node: SceneNode) -> &str {
        let idx = node.entity().index() as usize;
        if idx < self.names.len() {
            &self.names[idx]
        } else {
            ""
        }
    }

    pub fn transform(&self, node: SceneNode) -> &Transform {
        self.world.get::<Transform>(node.entity()).unwrap()
    }

    pub fn transform_mut(&mut self, node: SceneNode) -> &mut Transform {
        self.world.get_mut::<Transform>(node.entity()).unwrap()
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SceneNodeBuilder {
    scene_manager: *mut SceneManager,
    node: SceneNode,
}

impl SceneNodeBuilder {
    pub fn with_transform(self, transform: Transform) -> SceneNodeBuilder {
        let sm = unsafe { &mut *self.scene_manager };
        *sm.transform_mut(self.node) = transform;
        self
    }

    pub fn with_child(self, child: SceneNode) -> SceneNodeBuilder {
        let sm = unsafe { &mut *self.scene_manager };
        sm.set_parent(child, self.node);
        self
    }

    pub fn build(self) -> SceneNode {
        self.node
    }
}

impl From<SceneNodeBuilder> for SceneNode {
    fn from(builder: SceneNodeBuilder) -> Self {
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
