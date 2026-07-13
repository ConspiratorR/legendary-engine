//! Unity Behaviour trait — base class for components that can be enabled/disabled.
//!
//! Maps to `UnityEngine.Behaviour` in Unity's documentation.
//!
//! Behaviour inherits from Component and adds the `enabled` property.
//! MonoBehaviour inherits from Behaviour.

use crate::component::Component;
use crate::gameobject::GameObjectHandle;

/// Base trait for components that can be enabled or disabled (matches `UnityEngine.Behaviour`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Behaviour.html>
///
/// Behaviour is the base class of components that can be enabled or disabled
/// using the `enabled` property. This includes MonoBehaviour, Camera, Collider, etc.
///
/// ## Properties
/// - `enabled` — Whether the Behaviour is enabled and allowed to update
/// - `isActiveAndEnabled` — Whether the Behaviour is active (its GameObject is active) AND enabled
///
/// ## Relationship
/// ```text
/// Object -> Component -> Behaviour -> MonoBehaviour
/// Object -> Component -> Behaviour -> Camera
/// Object -> Component -> Behaviour -> Collider
/// Object -> Component -> Behaviour -> Renderer
/// etc.
/// ```
pub trait Behaviour: Component {
    /// Get whether this Behaviour is enabled.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Behaviour-enabled.html>
    ///
    /// A disabled Behaviour will not receive Update(), FixedUpdate(), or LateUpdate() calls.
    /// It can still receive other callbacks like OnTriggerEnter, OnCollisionEnter, etc.
    fn Enabled(&self) -> bool {
        true
    }

    /// Set whether this Behaviour is enabled.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Behaviour-enabled.html>
    fn SetEnabled(&mut self, enabled: bool);

    /// Get whether this Behaviour is both active in the hierarchy AND enabled.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Behaviour-isActiveAndEnabled.html>
    ///
    /// Returns `true` only if:
    /// - The GameObject this component is attached to is active in the hierarchy
    /// - AND this Behaviour is enabled
    fn IsActiveAndEnabled(&self) -> bool;

    /// Internal: Set the owning GameObject handle.
    fn set_gameobject(&mut self, handle: GameObjectHandle);

    /// Internal: Get the owning GameObject handle.
    fn gameobject_handle(&self) -> Option<GameObjectHandle>;
}

/// Default implementation helper for Behaviour.
///
/// Use this struct to store enabled state and GameObject reference
/// in your custom Behaviour implementations.
#[derive(Debug, Clone)]
pub struct BehaviourState {
    /// Whether this behaviour is enabled.
    pub enabled: bool,
    /// The owning GameObject handle.
    pub gameobject: Option<GameObjectHandle>,
}

impl Default for BehaviourState {
    fn default() -> Self {
        Self {
            enabled: true,
            gameobject: None,
        }
    }
}

impl BehaviourState {
    /// Create a new BehaviourState.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get enabled state.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Set enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get the GameObject handle.
    pub fn gameobject(&self) -> Option<GameObjectHandle> {
        self.gameobject
    }

    /// Set the GameObject handle.
    pub fn set_gameobject(&mut self, handle: GameObjectHandle) {
        self.gameobject = Some(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    #[derive(Debug)]
    struct TestBehaviour {
        state: BehaviourState,
        value: i32,
    }

    impl Component for TestBehaviour {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn component_name(&self) -> &str {
            "TestBehaviour"
        }
    }

    impl Behaviour for TestBehaviour {
        fn Enabled(&self) -> bool {
            self.state.enabled()
        }

        fn SetEnabled(&mut self, enabled: bool) {
            self.state.set_enabled(enabled);
        }

        fn IsActiveAndEnabled(&self) -> bool {
            self.state.enabled()
        }

        fn set_gameobject(&mut self, handle: GameObjectHandle) {
            self.state.set_gameobject(handle);
        }

        fn gameobject_handle(&self) -> Option<GameObjectHandle> {
            self.state.gameobject()
        }
    }

    #[test]
    fn test_behaviour_enabled() {
        let mut b = TestBehaviour {
            state: BehaviourState::new(),
            value: 42,
        };

        assert!(b.Enabled());
        b.SetEnabled(false);
        assert!(!b.Enabled());
        b.SetEnabled(true);
        assert!(b.Enabled());
    }

    #[test]
    fn test_behaviour_gameobject() {
        let mut b = TestBehaviour {
            state: BehaviourState::new(),
            value: 42,
        };

        assert!(b.gameobject_handle().is_none());

        let handle = GameObjectHandle::new(0, 0);
        b.set_gameobject(handle);
        assert_eq!(b.gameobject_handle(), Some(handle));
    }

    #[test]
    fn test_behaviour_is_active_and_enabled() {
        let mut b = TestBehaviour {
            state: BehaviourState::new(),
            value: 42,
        };

        assert!(b.IsActiveAndEnabled());
        b.SetEnabled(false);
        assert!(!b.IsActiveAndEnabled());
    }
}
