use std::any::Any;
use std::fmt;

/// Lightweight handle to a GameObject (index + generation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameObjectHandle {
    index: u32,
    generation: u32,
}

impl GameObjectHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the index (for internal use).
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the generation (for internal use).
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for GameObjectHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameObject({}:{})", self.index, self.generation)
    }
}

/// Base trait for all components (like Unity's Component).
pub trait Component: Any + Send + Sync {
    /// Called when the component is added to a GameObject.
    fn on_added(&mut self, _handle: GameObjectHandle) {}

    /// Called when the component is removed from a GameObject.
    fn on_removed(&mut self, _handle: GameObjectHandle) {}

    /// Called when the GameObject becomes active.
    fn on_enable(&mut self, _handle: GameObjectHandle) {}

    /// Called when the GameObject becomes inactive.
    fn on_disable(&mut self, _handle: GameObjectHandle) {}

    /// Called when the GameObject is destroyed.
    fn on_destroy(&mut self, _handle: GameObjectHandle) {}

    /// Get the component as Any for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Get the component as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the component name (for debugging).
    fn component_name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Base class for all entities in the scene (like Unity's GameObject).
pub struct GameObject {
    name: String,
    tag: String,
    layer: u32,
    active: bool,
    components: Vec<Box<dyn Component>>,
    pub(crate) parent: Option<GameObjectHandle>,
    pub(crate) children: Vec<GameObjectHandle>,
}

impl GameObject {
    /// Create a new GameObject (like Unity's new GameObject()).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tag: "Untagged".to_string(),
            layer: 0,
            active: true,
            components: Vec::new(),
            parent: None,
            children: Vec::new(),
        }
    }

    /// Get the name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Get the tag.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Set the tag.
    pub fn set_tag(&mut self, tag: &str) {
        self.tag = tag.to_string();
    }

    /// Get the layer.
    pub fn layer(&self) -> u32 {
        self.layer
    }

    /// Set the layer.
    pub fn set_layer(&mut self, layer: u32) {
        self.layer = layer;
    }

    /// Check if the GameObject is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set the active state.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Get the parent handle.
    pub fn parent(&self) -> Option<GameObjectHandle> {
        self.parent
    }

    /// Get the children handles.
    pub fn children(&self) -> &[GameObjectHandle] {
        &self.children
    }

    /// Get the number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Add a component (like Unity's AddComponent<T>()).
    pub fn add_component<T: Component + 'static>(&mut self, component: T) {
        self.components.push(Box::new(component));
    }

    /// Get a component by type (like Unity's GetComponent<T>()).
    pub fn get_component<T: Component + 'static>(&self) -> Option<&T> {
        self.components
            .iter()
            .find_map(|c| c.as_any().downcast_ref::<T>())
    }

    /// Get a component mutably by type (like Unity's GetComponent<T>()).
    pub fn get_component_mut<T: Component + 'static>(&mut self) -> Option<&mut T> {
        self.components
            .iter_mut()
            .find_map(|c| c.as_any_mut().downcast_mut::<T>())
    }

    /// Check if the GameObject has a component.
    pub fn has_component<T: Component + 'static>(&self) -> bool {
        self.get_component::<T>().is_some()
    }

    /// Get all components.
    pub fn components(&self) -> &[Box<dyn Component>] {
        &self.components
    }

    /// Remove a component by type.
    pub fn remove_component<T: Component + 'static>(&mut self) -> Option<Box<dyn Component>> {
        if let Some(pos) = self.components.iter().position(|c| c.as_any().is::<T>()) {
            Some(self.components.remove(pos))
        } else {
            None
        }
    }
}

impl fmt::Debug for GameObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameObject")
            .field("name", &self.name)
            .field("tag", &self.tag)
            .field("layer", &self.layer)
            .field("active", &self.active)
            .field("components", &self.components.len())
            .field("children", &self.children.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestComponent {
        value: i32,
    }

    impl Component for TestComponent {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_gameobject_creation() {
        let go = GameObject::new("TestObject");
        assert_eq!(go.name(), "TestObject");
        assert_eq!(go.tag(), "Untagged");
        assert_eq!(go.layer(), 0);
        assert!(go.is_active());
    }

    #[test]
    fn test_add_and_get_component() {
        let mut go = GameObject::new("TestObject");
        go.add_component(TestComponent { value: 42 });

        let component = go.get_component::<TestComponent>().unwrap();
        assert_eq!(component.value, 42);
    }

    #[test]
    fn test_get_component_mut() {
        let mut go = GameObject::new("TestObject");
        go.add_component(TestComponent { value: 42 });

        {
            let component = go.get_component_mut::<TestComponent>().unwrap();
            component.value = 100;
        }

        let component = go.get_component::<TestComponent>().unwrap();
        assert_eq!(component.value, 100);
    }

    #[test]
    fn test_has_component() {
        let mut go = GameObject::new("TestObject");
        assert!(!go.has_component::<TestComponent>());

        go.add_component(TestComponent { value: 42 });
        assert!(go.has_component::<TestComponent>());
    }

    #[test]
    fn test_remove_component() {
        let mut go = GameObject::new("TestObject");
        go.add_component(TestComponent { value: 42 });

        let removed = go.remove_component::<TestComponent>();
        assert!(removed.is_some());
        assert!(!go.has_component::<TestComponent>());
    }
}
