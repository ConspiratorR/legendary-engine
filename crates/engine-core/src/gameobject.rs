//! Unity GameObject — fundamental building block of Unity scenes.
//!
//! Maps to `UnityEngine.GameObject` in Unity's documentation.
//!
//! # Unity Documentation
//! <https://docs.unity3d.com/ScriptReference/GameObject.html>
//!
//! GameObjects are the most fundamental concept in Unity. Every entity in a game
//! (characters, props, lights, cameras, particles) is a GameObject.
//! GameObjects themselves are empty containers — they do nothing on their own.
//! They become functional when Components are attached.
//!
//! ## Key Concepts
//! - Every GameObject always has a Transform (mandatory, cannot be removed)
//! - GameObjects contain a list of Components
//! - GameObjects have name, tag, layer, and active state
//! - GameObjects form a parent-child hierarchy via Transform
//! - Components receive lifecycle callbacks (via MonoBehaviour)

pub use engine_ecs::GameObjectHandle;

use crate::component::Component;
use std::any::Any;
use std::fmt;

/// The fundamental building block of Unity scenes (matches `UnityEngine.GameObject`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/GameObject.html>
///
/// ## Properties
/// - `name` — The name of the GameObject
/// - `tag` — The tag of the GameObject
/// - `layer` — The layer the GameObject is in
/// - `activeSelf` — Local active state (read-only)
/// - `activeInHierarchy` — Whether active in scene (read-only)
/// - `isStatic` — Whether the GameObject is static
/// - `transform` — The Transform component (always present, via World)
///
/// ## Methods
/// - `AddComponent<T>()` — Attach a component
/// - `GetComponent<T>()` — Get a component
/// - `SetActive(bool)` — Activate/deactivate
/// - `CompareTag(string)` — Check tag
/// - `SendMessage(string)` — Call method on MonoBehaviours
///
/// # Rust Implementation
/// In Unity, GameObject is a C# class. In Rust, it's a struct that is
/// owned by the World. GameObjects are referenced via `GameObjectHandle`.
pub struct GameObject {
    /// The name of the GameObject (matches `GameObject.name`).
    pub(crate) name: String,

    /// The tag of the GameObject (matches `GameObject.tag`).
    pub(crate) tag: String,

    /// The layer the GameObject is in (matches `GameObject.layer`).
    pub(crate) layer: i32,

    /// Local active state (matches `GameObject.activeSelf`).
    pub(crate) active_self: bool,

    /// Whether the GameObject is static (matches `GameObject.isStatic`).
    pub(crate) is_static: bool,

    /// Components attached to this GameObject (matches `GameObject.GetComponents`).
    pub(crate) components: Vec<Box<dyn Component>>,
}

impl GameObject {
    // ============================================================
    // Constructors
    // ============================================================

    /// Create a new GameObject with a name (matches `new GameObject("name")`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.html>
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tag: "Untagged".to_string(),
            layer: 0,
            active_self: true,
            is_static: false,
            components: Vec::new(),
        }
    }

    /// Create a new unnamed GameObject (matches `new GameObject()`).
    pub fn new_empty() -> Self {
        Self::new("")
    }

    /// Create a new GameObject with components (matches `new GameObject("name", typeof(T1), typeof(T2))`).
    pub fn new_with_components(name: &str, components: Vec<Box<dyn Component>>) -> Self {
        Self {
            name: name.to_string(),
            tag: "Untagged".to_string(),
            layer: 0,
            active_self: true,
            is_static: false,
            components,
        }
    }

    // ============================================================
    // Properties — Name
    // ============================================================

    /// Get the name (matches `Object.name`).
    pub fn Name(&self) -> &str {
        &self.name
    }

    /// Set the name (matches `Object.name`).
    pub fn SetName(&mut self, name: &str) {
        self.name = name.to_string();
    }

    // ============================================================
    // Properties — Tag
    // ============================================================

    /// Get the tag (matches `GameObject.tag`).
    pub fn Tag(&self) -> &str {
        &self.tag
    }

    /// Set the tag (matches `GameObject.tag`).
    pub fn SetTag(&mut self, tag: &str) {
        self.tag = tag.to_string();
    }

    /// Check if this GameObject has the given tag (matches `GameObject.CompareTag`).
    pub fn CompareTag(&self, tag: &str) -> bool {
        self.tag == tag
    }

    // ============================================================
    // Properties — Layer
    // ============================================================

    /// Get the layer (matches `GameObject.layer`).
    pub fn Layer(&self) -> i32 {
        self.layer
    }

    /// Set the layer (matches `GameObject.layer`).
    pub fn SetLayer(&mut self, layer: i32) {
        self.layer = layer;
    }

    // ============================================================
    // Properties — Active State
    // ============================================================

    /// Get the local active state (matches `GameObject.activeSelf`).
    pub fn ActiveSelf(&self) -> bool {
        self.active_self
    }

    /// Set the active state (matches `GameObject.SetActive`).
    pub fn SetActive(&mut self, active: bool) {
        self.active_self = active;
    }

    // NOTE: `activeInHierarchy` requires World to check parent chain.
    // It's implemented on World, not on GameObject.

    // ============================================================
    // Properties — Static State
    // ============================================================

    /// Get whether the GameObject is static (matches `GameObject.isStatic`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject-isStatic.html>
    pub fn IsStatic(&self) -> bool {
        self.is_static
    }

    /// Set whether the GameObject is static (matches `GameObject.isStatic`).
    pub fn SetStatic(&mut self, is_static: bool) {
        self.is_static = is_static;
    }

    // ============================================================
    // Methods — Component Access
    // ============================================================

    /// Add a component to this GameObject (matches `GameObject.AddComponent<T>()`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.AddComponent.html>
    ///
    /// Returns a mutable reference to the added component.
    pub fn AddComponent<T: Component + 'static>(&mut self, component: T) -> &mut T {
        self.components.push(Box::new(component));
        self.components.last_mut().unwrap().as_any_mut().downcast_mut::<T>().unwrap()
    }

    /// Add a boxed component (for dynamic deserialization).
    pub fn AddComponentBoxed(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }

    /// Get a component by type (matches `GameObject.GetComponent<T>()`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.GetComponent.html>
    pub fn GetComponent<T: Component + 'static>(&self) -> Option<&T> {
        self.components
            .iter()
            .find_map(|c| c.as_any().downcast_ref::<T>())
    }

    /// Get a mutable component by type (matches `GameObject.GetComponent<T>()` with write access).
    pub fn GetComponentMut<T: Component + 'static>(&mut self) -> Option<&mut T> {
        self.components
            .iter_mut()
            .find_map(|c| c.as_any_mut().downcast_mut::<T>())
    }

    /// Check if this GameObject has a component of the given type (matches `GameObject.GetComponent<T>() != null`).
    pub fn HasComponent<T: Component + 'static>(&self) -> bool {
        self.GetComponent::<T>().is_some()
    }

    /// Get all components (matches `GameObject.GetComponents<T>()`).
    pub fn GetComponents<T: Component + 'static>(&self) -> Vec<&T> {
        self.components
            .iter()
            .filter_map(|c| c.as_any().downcast_ref::<T>())
            .collect()
    }

    /// Get all components as boxed trait objects.
    pub fn Components(&self) -> &[Box<dyn Component>] {
        &self.components
    }

    /// Get all components as mutable boxed trait objects.
    pub fn ComponentsMut(&mut self) -> &mut [Box<dyn Component>] {
        &mut self.components
    }

    /// Remove a component by type (matches `Object.Destroy(component)`).
    pub fn RemoveComponent<T: Component + 'static>(&mut self) -> Option<Box<dyn Component>> {
        if let Some(pos) = self.components.iter().position(|c| c.as_any().is::<T>()) {
            Some(self.components.remove(pos))
        } else {
            None
        }
    }

    // NOTE: `GetComponentInChildren`, `GetComponentInParent`, `GetComponentsInChildren`,
    // `GetComponentsInParent` require World access to traverse hierarchy.
    // They're implemented on World, not on GameObject.

    // ============================================================
    // Methods — Messaging
    // ============================================================

    /// Send a message to all MonoBehaviours on this GameObject (matches `GameObject.SendMessage`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.SendMessage.html>
    ///
    /// NOTE: This is a placeholder. The actual implementation requires
    /// World access to iterate MonoBehaviours.
    pub fn SendMessage(&mut self, _method_name: &str) {
        // Implemented by World
    }

    /// Send a message with a value (matches `GameObject.SendMessage(methodName, value)`).
    pub fn SendMessageWithValue(&mut self, _method_name: &str, _value: &dyn Any) {
        // Implemented by World
    }

    /// Send a message to this and all parents (matches `GameObject.SendMessageUpwards`).
    pub fn SendMessageUpwards(&mut self, _method_name: &str) {
        // Implemented by World
    }

    /// Send a message to this and all children (matches `GameObject.BroadcastMessage`).
    pub fn BroadcastMessage(&mut self, _method_name: &str) {
        // Implemented by World
    }

    // ============================================================
    // Methods — Instance ID
    // ============================================================

    /// Get the instance ID (matches `Object.GetInstanceID`).
    ///
    /// NOTE: This returns 0 as placeholder. The actual instance ID
    /// is assigned by World when the GameObject is spawned.
    pub fn GetInstanceID(&self) -> i32 {
        0 // Placeholder — set by World
    }

    // ============================================================
    // Methods — ToString
    // ============================================================

    /// Convert to string (matches `Object.ToString`).
    pub fn ToString(&self) -> String {
        if self.name.is_empty() {
            "GameObject".to_string()
        } else {
            self.name.clone()
        }
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Get name (snake_case alias for Name).
    pub fn name(&self) -> &str {
        self.Name()
    }

    /// Set name (snake_case alias for SetName).
    pub fn set_name(&mut self, name: &str) {
        self.SetName(name);
    }

    /// Get tag (snake_case alias for Tag).
    pub fn tag(&self) -> &str {
        self.Tag()
    }

    /// Set tag (snake_case alias for SetTag).
    pub fn set_tag(&mut self, tag: &str) {
        self.SetTag(tag);
    }

    /// Get layer (snake_case alias for Layer).
    pub fn layer(&self) -> i32 {
        self.Layer()
    }

    /// Set layer (snake_case alias for SetLayer).
    pub fn set_layer(&mut self, layer: i32) {
        self.SetLayer(layer);
    }

    /// Get active state (snake_case alias for ActiveSelf).
    pub fn is_active(&self) -> bool {
        self.ActiveSelf()
    }

    /// Set active state (snake_case alias for SetActive).
    pub fn set_active(&mut self, active: bool) {
        self.SetActive(active);
    }

    /// Add component (snake_case alias for AddComponent).
    pub fn add_component<T: Component + 'static>(&mut self, component: T) -> &mut T {
        self.AddComponent(component)
    }

    /// Get component (snake_case alias for GetComponent).
    pub fn get_component<T: Component + 'static>(&self) -> Option<&T> {
        self.GetComponent::<T>()
    }

    /// Get mutable component (snake_case alias for GetComponentMut).
    pub fn get_component_mut<T: Component + 'static>(&mut self) -> Option<&mut T> {
        self.GetComponentMut::<T>()
    }

    /// Has component (snake_case alias for HasComponent).
    pub fn has_component<T: Component + 'static>(&self) -> bool {
        self.HasComponent::<T>()
    }

    /// Get components (snake_case alias for Components).
    pub fn components(&self) -> &[Box<dyn Component>] {
        self.Components()
    }

    /// Get child count.
    pub fn child_count(&self) -> usize {
        0 // Requires World access for hierarchy
    }

    /// Get children.
    pub fn children(&self) -> &[GameObjectHandle] {
        &[] // Requires World access for hierarchy
    }
}

impl Default for GameObject {
    fn default() -> Self {
        Self::new("")
    }
}

impl fmt::Debug for GameObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameObject")
            .field("name", &self.name)
            .field("tag", &self.tag)
            .field("layer", &self.layer)
            .field("active", &self.active_self)
            .field("components", &self.components.len())
            .finish()
    }
}

impl fmt::Display for GameObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ToString())
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
        let go = GameObject::new_with_name("TestObject");
        assert_eq!(go.Name(), "TestObject");
        assert_eq!(go.Tag(), "Untagged");
        assert_eq!(go.Layer(), 0);
        assert!(go.ActiveSelf());
    }

    #[test]
    fn test_gameobject_default_name() {
        let go = GameObject::new_empty();
        assert_eq!(go.Name(), "");
    }

    #[test]
    fn test_gameobject_add_and_get_component() {
        let mut go = GameObject::new_with_name("TestObject");
        go.AddComponent(TestComponent { value: 42 });

        let component = go.GetComponent::<TestComponent>().unwrap();
        assert_eq!(component.value, 42);
    }

    #[test]
    fn test_gameobject_get_component_mut() {
        let mut go = GameObject::new_with_name("TestObject");
        go.AddComponent(TestComponent { value: 42 });

        {
            let component = go.GetComponentMut::<TestComponent>().unwrap();
            component.value = 100;
        }

        let component = go.GetComponent::<TestComponent>().unwrap();
        assert_eq!(component.value, 100);
    }

    #[test]
    fn test_gameobject_has_component() {
        let mut go = GameObject::new_with_name("TestObject");
        assert!(!go.HasComponent::<TestComponent>());

        go.AddComponent(TestComponent { value: 42 });
        assert!(go.HasComponent::<TestComponent>());
    }

    #[test]
    fn test_gameobject_get_components() {
        let mut go = GameObject::new_with_name("TestObject");
        go.AddComponent(TestComponent { value: 1 });
        go.AddComponent(TestComponent { value: 2 });
        go.AddComponent(TestComponent { value: 3 });

        let components = go.GetComponents::<TestComponent>();
        assert_eq!(components.len(), 3);
    }

    #[test]
    fn test_gameobject_remove_component() {
        let mut go = GameObject::new_with_name("TestObject");
        go.AddComponent(TestComponent { value: 42 });

        let removed = go.RemoveComponent::<TestComponent>();
        assert!(removed.is_some());
        assert!(!go.HasComponent::<TestComponent>());
    }

    #[test]
    fn test_gameobject_compare_tag() {
        let mut go = GameObject::new_with_name("TestObject");
        go.SetTag("Player");

        assert!(go.CompareTag("Player"));
        assert!(!go.CompareTag("Enemy"));
    }

    #[test]
    fn test_gameobject_set_active() {
        let mut go = GameObject::new_with_name("TestObject");
        assert!(go.ActiveSelf());

        go.SetActive(false);
        assert!(!go.ActiveSelf());

        go.SetActive(true);
        assert!(go.ActiveSelf());
    }

    #[test]
    fn test_gameobject_to_string() {
        let go = GameObject::new_with_name("Player");
        assert_eq!(go.ToString(), "Player");

        let go2 = GameObject::new_empty();
        assert_eq!(go2.ToString(), "GameObject");
    }
}
