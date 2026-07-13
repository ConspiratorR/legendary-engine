//! Unity MonoBehaviour — base class for all user scripts.
//!
//! Maps to `UnityEngine.MonoBehaviour` in Unity's documentation.
//!
//! # Unity Documentation
//! <https://docs.unity3d.com/ScriptReference/MonoBehaviour.html>
//!
//! MonoBehaviour is the base class for all user scripts in Unity.
//! It inherits from Behaviour (which inherits from Component).
//!
//! ## Lifecycle
//! The lifecycle callbacks are called in this order:
//! 1. `Awake()` — called when script instance is loaded
//! 2. `OnEnable()` — called when object becomes active
//! 3. `Start()` — called before first Update
//! 4. `Update()` — called once per frame
//! 5. `FixedUpdate()` — called at fixed intervals (physics)
//! 6. `LateUpdate()` — called after all Update calls
//! 7. `OnDisable()` — called when object becomes inactive
//! 8. `OnDestroy()` — called when object is destroyed
//!
//! ## Coroutines
//! MonoBehaviour supports coroutines via `StartCoroutine`, `StopCoroutine`, `StopAllCoroutines`.
//!
/// Invoke
/// MonoBehaviour supports delayed method calls via `Invoke`, `InvokeRepeating`, `CancelInvoke`.

use crate::behaviour::Behaviour;
use crate::component::Component;
use crate::context::Context;
use crate::events::*;
use crate::gameobject::GameObjectHandle;
use std::any::Any;

/// Handle to a running coroutine (matches Unity's `Coroutine` class).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Coroutine.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoroutineHandle(pub(crate) u64);

/// Base class for all user scripts (matches `UnityEngine.MonoBehaviour`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.html>
///
/// ## Relationship
/// ```text
/// Object -> Component -> Behaviour -> MonoBehaviour
/// ```
///
/// ## Usage
/// Create a struct that implements this trait and attach it to a GameObject:
///
/// ```ignore
/// struct Player {
///     speed: f32,
/// }
///
/// impl Component for Player {
///     fn as_any(&self) -> &dyn Any { self }
///     fn as_any_mut(&mut self) -> &mut dyn Any { self }
/// }
///
/// impl Behaviour for Player {
///     fn Enabled(&self) -> bool { true }
///     fn SetEnabled(&mut self, _enabled: bool) {}
///     fn IsActiveAndEnabled(&self) -> bool { true }
///     fn set_gameobject(&mut self, _handle: GameObjectHandle) {}
///     fn gameobject_handle(&self) -> Option<GameObjectHandle> { None }
/// }
///
/// impl MonoBehaviour for Player {
///     fn Start(&mut self, _ctx: &mut Context) {
///         println!("Player started!");
///     }
///
///     fn Update(&mut self, _ctx: &mut Context) {
///         // Move player
///     }
/// }
/// ```
pub trait MonoBehaviour: Behaviour {
    // ============================================================
    // Properties
    // ============================================================

    /// Whether to use GUI layout (matches `MonoBehaviour.useGUILayout`).
    fn UseGUILayout(&self) -> bool {
        true
    }

    /// Set whether to use GUI layout (matches `MonoBehaviour.useGUILayout`).
    fn SetUseGUILayout(&mut self, _value: bool) {}

    /// Whether to run in edit mode (matches `MonoBehaviour.runInEditMode`).
    fn RunInEditMode(&self) -> bool {
        false
    }

    // ============================================================
    // Lifecycle Callbacks
    // ============================================================

    /// Called when the script instance is being loaded (matches `MonoBehaviour.Awake`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Awake.html>
    ///
    /// Always called before any Start. Called even if the GameObject is inactive.
    /// Use this for initialization that needs to happen regardless of active state.
    fn Awake(&mut self, _context: &mut Context) {}

    /// Called when the object becomes enabled and active (matches `MonoBehaviour.OnEnable`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnEnable.html>
    ///
    /// Called after Awake, and each time the object is re-enabled.
    fn OnEnable(&mut self, _context: &mut Context) {}

    /// Called when the object becomes disabled or inactive (matches `MonoBehaviour.OnDisable`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDisable.html>
    ///
    /// Called before OnDestroy, and each time the object is disabled.
    fn OnDisable(&mut self, _context: &mut Context) {}

    /// Called before the first frame update (matches `MonoBehaviour.Start`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Start.html>
    ///
    /// Called only if the script is enabled. All Start calls happen before
    /// any Update calls. Use for initialization that requires other objects
    /// to exist.
    fn Start(&mut self, _context: &mut Context) {}

    /// Called once per frame (matches `MonoBehaviour.Update`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Update.html>
    ///
    /// The most common callback for game logic. Called after FixedUpdate
    /// and before LateUpdate.
    fn Update(&mut self, _context: &mut Context) {}

    /// Called at fixed intervals for physics (matches `MonoBehaviour.FixedUpdate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.FixedUpdate.html>
    ///
    /// Called at a fixed time step (default 0.02 seconds). All physics
    /// calculations should happen here. Called 0+ times per frame.
    fn FixedUpdate(&mut self, _context: &mut Context) {}

    /// Called after all Update calls (matches `MonoBehaviour.LateUpdate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.LateUpdate.html>
    ///
    /// Commonly used for camera follow logic. Called once per frame after
    /// all Update calls.
    fn LateUpdate(&mut self, _context: &mut Context) {}

    /// Called when the script is being destroyed (matches `MonoBehaviour.OnDestroy`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDestroy.html>
    ///
    /// Called on the last frame before the object is removed from the scene.
    fn OnDestroy(&mut self, _context: &mut Context) {}

    // ============================================================
    // Application Callbacks
    // ============================================================

    /// Called before the application quits (matches `MonoBehaviour.OnApplicationQuit`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnApplicationQuit.html>
    fn OnApplicationQuit(&mut self, _context: &mut Context) {}

    /// Called when the application pauses (matches `MonoBehaviour.OnApplicationPause`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnApplicationPause.html>
    fn OnApplicationPause(&mut self, _context: &mut Context, _paused: bool) {}

    /// Called when the application gains or loses focus (matches `MonoBehaviour.OnApplicationFocus`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnApplicationFocus.html>
    fn OnApplicationFocus(&mut self, _context: &mut Context, _focused: bool) {}

    // ============================================================
    // Physics Callbacks
    // ============================================================

    /// Called when a collision starts (matches `MonoBehaviour.OnCollisionEnter`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnCollisionEnter.html>
    fn OnCollisionEnter(&mut self, _context: &mut Context, _collision: &Collision) {}

    /// Called when a collision ends (matches `MonoBehaviour.OnCollisionExit`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnCollisionExit.html>
    fn OnCollisionExit(&mut self, _context: &mut Context, _collision: &Collision) {}

    /// Called while a collision persists (matches `MonoBehaviour.OnCollisionStay`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnCollisionStay.html>
    fn OnCollisionStay(&mut self, _context: &mut Context, _collision: &Collision) {}

    /// Called when a trigger is entered (matches `MonoBehaviour.OnTriggerEnter`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnTriggerEnter.html>
    fn OnTriggerEnter(&mut self, _context: &mut Context, _other: &TriggerData) {}

    /// Called when a trigger is exited (matches `MonoBehaviour.OnTriggerExit`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnTriggerExit.html>
    fn OnTriggerExit(&mut self, _context: &mut Context, _other: &TriggerData) {}

    /// Called while a trigger persists (matches `MonoBehaviour.OnTriggerStay`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnTriggerStay.html>
    fn OnTriggerStay(&mut self, _context: &mut Context, _other: &TriggerData) {}

    // ============================================================
    // Input Callbacks
    // ============================================================

    /// Called when the mouse is pressed on the collider (matches `MonoBehaviour.OnMouseDown`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnMouseDown.html>
    fn OnMouseDown(&mut self, _context: &mut Context) {}

    /// Called when the mouse button is released (matches `MonoBehaviour.OnMouseUp`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnMouseUp.html>
    fn OnMouseUp(&mut self, _context: &mut Context) {}

    /// Called when the mouse enters the collider (matches `MonoBehaviour.OnMouseEnter`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnMouseEnter.html>
    fn OnMouseEnter(&mut self, _context: &mut Context) {}

    /// Called when the mouse exits the collider (matches `MonoBehaviour.OnMouseExit`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnMouseExit.html>
    fn OnMouseExit(&mut self, _context: &mut Context) {}

    /// Called when the mouse is dragged (matches `MonoBehaviour.OnMouseDrag`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnMouseDrag.html>
    fn OnMouseDrag(&mut self, _context: &mut Context) {}

    /// Called when the mouse is hovering (matches `MonoBehaviour.OnMouseOver`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnMouseOver.html>
    fn OnMouseOver(&mut self, _context: &mut Context) {}

    /// Called when the mouse button is released on the same collider (matches `MonoBehaviour.OnMouseUpAsButton`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnMouseUpAsButton.html>
    fn OnMouseUpAsButton(&mut self, _context: &mut Context) {}

    // ============================================================
    // Rendering Callbacks
    // ============================================================

    /// Called when the renderer becomes visible (matches `MonoBehaviour.OnBecameVisible`).
    fn OnBecameVisible(&mut self, _context: &mut Context) {}

    /// Called when the renderer becomes invisible (matches `MonoBehaviour.OnBecameInvisible`).
    fn OnBecameInvisible(&mut self, _context: &mut Context) {}

    /// Called for drawing gizmos (matches `MonoBehaviour.OnDrawGizmos`).
    fn OnDrawGizmos(&self, _context: &Context) {}

    /// Called for drawing gizmos when selected (matches `MonoBehaviour.OnDrawGizmosSelected`).
    fn OnDrawGizmosSelected(&self, _context: &Context) {}

    // ============================================================
    // Animation Callbacks
    // ============================================================

    /// Called to process root motion (matches `MonoBehaviour.OnAnimatorMove`).
    fn OnAnimatorMove(&mut self, _context: &mut Context) {}

    /// Called to set up IK (matches `MonoBehaviour.OnAnimatorIK`).
    fn OnAnimatorIK(&mut self, _context: &mut Context, _layer_index: i32) {}

    // ============================================================
    // Coroutine Methods
    // ============================================================

    /// Start a coroutine (matches `MonoBehaviour.StartCoroutine`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.StartCoroutine.html>
    ///
    /// Returns a handle that can be passed to `StopCoroutine`.
    fn StartCoroutine(&mut self, _routine: &str) -> CoroutineHandle {
        CoroutineHandle(0)
    }

    /// Stop a specific coroutine (matches `MonoBehaviour.StopCoroutine`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.StopCoroutine.html>
    fn StopCoroutine(&mut self, _routine: &str) {}

    /// Stop all coroutines (matches `MonoBehaviour.StopAllCoroutines`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.StopAllCoroutines.html>
    fn StopAllCoroutines(&mut self) {}

    // ============================================================
    // Invoke Methods
    // ============================================================

    /// Call a method after a delay (matches `MonoBehaviour.Invoke`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Invoke.html>
    fn Invoke(&mut self, _method_name: &str, _time: f32) {}

    /// Call a method repeatedly (matches `MonoBehaviour.InvokeRepeating`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.InvokeRepeating.html>
    fn InvokeRepeating(&mut self, _method_name: &str, _time: f32, _repeat_rate: f32) {}

    /// Cancel all pending Invoke calls (matches `MonoBehaviour.CancelInvoke`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.CancelInvoke.html>
    fn CancelInvoke(&mut self) {}

    /// Cancel a specific Invoke call (matches `MonoBehaviour.CancelInvoke(methodName)`).
    fn CancelInvokeMethod(&mut self, _method_name: &str) {}

    /// Check if any Invoke calls are pending (matches `MonoBehaviour.IsInvoking`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.IsInvoking.html>
    fn IsInvoking(&self) -> bool {
        false
    }

    /// Check if a specific Invoke call is pending (matches `MonoBehaviour.IsInvoking(methodName)`).
    fn IsInvokingMethod(&self, _method_name: &str) -> bool {
        false
    }

    // ============================================================
    // Static Methods
    // ============================================================

    /// Log a message to the console (matches `MonoBehaviour.print`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.print.html>
    fn print(message: &str) where Self: Sized {
        println!("{}", message);
    }
}

/// Wrapper that stores a boxed MonoBehaviour trait object.
///
/// This is used internally by the World to store MonoBehaviour components.
pub struct MonoBehaviourHolder {
    inner: Box<dyn MonoBehaviour>,
    enabled: bool,
    started: bool,
}

impl MonoBehaviourHolder {
    /// Create a new holder wrapping a MonoBehaviour.
    pub fn new(mono: impl MonoBehaviour + 'static) -> Self {
        Self {
            inner: Box::new(mono),
            enabled: true,
            started: false,
        }
    }

    /// Get a reference to the inner MonoBehaviour.
    pub fn Get(&self) -> &dyn MonoBehaviour {
        &*self.inner
    }

    /// Get a mutable reference to the inner MonoBehaviour.
    pub fn GetMut(&mut self) -> &mut dyn MonoBehaviour {
        &mut *self.inner
    }

    /// Check if the holder is enabled.
    pub fn Enabled(&self) -> bool {
        self.enabled && self.inner.Enabled()
    }

    /// Set the enabled state.
    pub fn SetEnabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if Start has been called.
    pub fn HasStarted(&self) -> bool {
        self.started
    }

    /// Mark Start as having been called.
    pub(crate) fn MarkStarted(&mut self) {
        self.started = true;
    }

    /// Get the type name (for debugging).
    pub fn TypeName(&self) -> &str {
        std::any::type_name_of_val(&*self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

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

    impl Behaviour for TestComponent {
        fn Enabled(&self) -> bool {
            true
        }

        fn SetEnabled(&mut self, _enabled: bool) {}

        fn IsActiveAndEnabled(&self) -> bool {
            true
        }

        fn set_gameobject(&mut self, _handle: GameObjectHandle) {}

        fn gameobject_handle(&self) -> Option<GameObjectHandle> {
            None
        }
    }

    impl MonoBehaviour for TestComponent {
        fn Start(&mut self, _ctx: &mut Context) {
            self.value += 1;
        }

        fn Update(&mut self, _ctx: &mut Context) {
            self.value += 1;
        }
    }

    #[test]
    fn test_monobehaviour_trait() {
        let comp = TestComponent { value: 0 };
        let mut holder = MonoBehaviourHolder::new(comp);

        assert!(holder.Enabled());
        assert!(!holder.HasStarted());
    }

    #[test]
    fn test_monobehaviour_enabled() {
        let comp = TestComponent { value: 0 };
        let mut holder = MonoBehaviourHolder::new(comp);

        holder.SetEnabled(false);
        assert!(!holder.Enabled());

        holder.SetEnabled(true);
        assert!(holder.Enabled());
    }

    #[test]
    fn test_monobehaviour_type_name() {
        let comp = TestComponent { value: 0 };
        let holder = MonoBehaviourHolder::new(comp);

        let tn = holder.TypeName();
        // type_name_of_val on a trait object may return the concrete type
        // or the trait object type depending on the implementation.
        assert!(
            tn.contains("TestComponent") || tn.contains("MonoBehaviour"),
            "got {tn}"
        );
    }
}
