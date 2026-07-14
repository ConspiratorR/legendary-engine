//! Unity Component trait — base class for all components attached to GameObjects.
//!
//! Maps to `UnityEngine.Component` in Unity's documentation.
//!
//! Components are the functional pieces of a GameObject. They cannot exist
//! independently — they must be attached to a GameObject.
//!
//! # Unity Documentation
//! <https://docs.unity3d.com/ScriptReference/Component.html>

use crate::gameobject::GameObjectHandle;
use std::any::Any;

/// Base trait for all components attached to GameObjects (matches `UnityEngine.Component`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Component.html>
///
/// ## Key Concepts
/// - A Component is always attached to exactly one GameObject
/// - Components cannot be created directly — they are added via `GameObject.AddComponent<T>()`
/// - Components receive lifecycle callbacks (via MonoBehaviour)
/// - Components provide access to their owning GameObject, Transform, and tag
///
/// ## Relationship
/// ```text
/// Object -> Component -> Transform (built-in, not user-attached)
/// Object -> Component -> Behaviour -> MonoBehaviour (user scripts)
/// Object -> Component -> Rigidbody
/// Object -> Component -> Collider (BoxCollider, SphereCollider, etc.)
/// Object -> Component -> Renderer (MeshRenderer, SpriteRenderer)
/// Object -> Component -> Camera
/// Object -> Component -> Light
/// Object -> Component -> AudioSource
/// Object -> Component -> Animator
/// etc.
/// ```
///
/// ## Rust Implementation
/// In Unity, Component is a C# class with virtual methods. In Rust, we use a trait.
/// All component types must implement this trait to be attached to GameObjects.
///
/// The trait provides:
/// - `as_any()` / `as_any_mut()` — for downcasting to concrete types
/// - Lifecycle callbacks (on_added, on_removed, on_enable, on_disable, on_destroy)
/// - Component name for debugging
pub trait Component: Any + Send + Sync {
    /// Get a reference to this component as `Any` for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Get a mutable reference to this component as `Any` for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the name of this component type (for debugging).
    fn component_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Called when this component is added to a GameObject.
    ///
    /// # Unity Documentation
    /// Not directly mapped in Unity, but corresponds to the behavior
    /// that happens after AddComponent<T>() is called.
    fn on_added(&mut self, _handle: GameObjectHandle) {}

    /// Called when this component is removed from a GameObject.
    ///
    /// # Unity Documentation
    /// Not directly mapped in Unity, but corresponds to the behavior
    /// that happens when a component is destroyed.
    fn on_removed(&mut self, _handle: GameObjectHandle) {}

    /// Called when the owning GameObject becomes active.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Behaviour.OnEnable.html>
    fn on_enable(&mut self, _handle: GameObjectHandle) {}

    /// Called when the owning GameObject becomes inactive.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Behaviour.OnDisable.html>
    fn on_disable(&mut self, _handle: GameObjectHandle) {}

    /// Called when the owning GameObject is destroyed.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDestroy.html>
    fn on_destroy(&mut self, _handle: GameObjectHandle) {}
}

/// Macro to implement Component trait for a struct.
///
/// # Example
/// ```ignore
/// use engine_core::impl_component;
/// use engine_core::gameobject::GameObjectHandle;
///
/// #[derive(Debug)]
/// struct MyComponent {
///     value: i32,
/// }
///
/// impl_component!(MyComponent);
/// ```
#[macro_export]
macro_rules! impl_component {
    ($type:ty) => {
        impl $crate::component::Component for $type {
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
                self
            }
        }
    };
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
    fn test_component_as_any() {
        let comp = TestComponent { value: 42 };
        let any_ref = comp.as_any();
        let downcasted = any_ref.downcast_ref::<TestComponent>().unwrap();
        assert_eq!(downcasted.value, 42);
    }

    #[test]
    fn test_component_as_any_mut() {
        let mut comp = TestComponent { value: 42 };
        {
            let any_ref = comp.as_any_mut();
            let downcasted = any_ref.downcast_mut::<TestComponent>().unwrap();
            downcasted.value = 100;
        }
        let any_ref = comp.as_any();
        let downcasted = any_ref.downcast_ref::<TestComponent>().unwrap();
        assert_eq!(downcasted.value, 100);
    }

    #[test]
    fn test_component_name() {
        let comp = TestComponent { value: 42 };
        assert!(comp.component_name().contains("TestComponent"));
    }

    #[test]
    fn test_component_lifecycle() {
        let mut comp = TestComponent { value: 42 };
        let handle = GameObjectHandle::new(0, 0);

        comp.on_added(handle);
        comp.on_enable(handle);
        comp.on_disable(handle);
        comp.on_destroy(handle);
    }

    #[test]
    fn test_impl_component_macro() {
        #[derive(Debug)]
        struct MacroComponent {
            data: String,
        }

        impl_component!(MacroComponent);

        let comp = MacroComponent {
            data: "hello".to_string(),
        };
        assert_eq!(
            comp.as_any().downcast_ref::<MacroComponent>().unwrap().data,
            "hello"
        );
    }
}
