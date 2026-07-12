use std::any::Any;

use crate::context::Context;
use crate::gameobject::Component;

/// Base class for user scripts (like Unity's MonoBehaviour).
/// Components can implement this trait to receive lifecycle callbacks.
pub trait MonoBehaviour: Component {
    /// Called when the script instance is being loaded (like Unity's Awake).
    fn awake(&mut self, _context: &mut Context) {}

    /// Called on the frame when the script is enabled (like Unity's OnEnable).
    fn on_enable(&mut self, _context: &mut Context) {}

    /// Called on the frame when the script is disabled (like Unity's OnDisable).
    fn on_disable(&mut self, _context: &mut Context) {}

    /// Called before the first frame update (like Unity's Start).
    fn start(&mut self, _context: &mut Context) {}

    /// Called once per frame (like Unity's Update).
    fn update(&mut self, _context: &mut Context) {}

    /// Called at fixed intervals (like Unity's FixedUpdate).
    fn fixed_update(&mut self, _context: &mut Context) {}

    /// Called after all Update calls (like Unity's LateUpdate).
    fn late_update(&mut self, _context: &mut Context) {}

    /// Called when the script is destroyed (like Unity's OnDestroy).
    fn on_destroy(&mut self, _context: &mut Context) {}

    /// Called when the mouse enters the Collider (like Unity's OnMouseEnter).
    fn on_mouse_enter(&mut self, _context: &mut Context) {}

    /// Called when the mouse exits the Collider (like Unity's OnMouseExit).
    fn on_mouse_exit(&mut self, _context: &mut Context) {}

    /// Called when the mouse is pressed on the Collider (like Unity's OnMouseDown).
    fn on_mouse_down(&mut self, _context: &mut Context) {}

    /// Called when the mouse button is released (like Unity's OnMouseUp).
    fn on_mouse_up(&mut self, _context: &mut Context) {}

    /// Called when a collision starts (like Unity's OnCollisionEnter).
    fn on_collision_enter(&mut self, _context: &mut Context, _collision: &dyn Any) {}

    /// Called when a collision ends (like Unity's OnCollisionExit).
    fn on_collision_exit(&mut self, _context: &mut Context, _collision: &dyn Any) {}

    /// Called when a trigger is entered (like Unity's OnTriggerEnter).
    fn on_trigger_enter(&mut self, _context: &mut Context, _other: &dyn Any) {}

    /// Called when a trigger is exited (like Unity's OnTriggerExit).
    fn on_trigger_exit(&mut self, _context: &mut Context, _other: &dyn Any) {}

    /// Called for drawing gizmos (like Unity's OnDrawGizmos).
    fn on_draw_gizmos(&self, _context: &Context) {}

    /// Check if the MonoBehaviour is enabled.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Set the enabled state.
    fn set_enabled(&mut self, _enabled: bool) {}
}

/// Wrapper that stores a boxed MonoBehaviour trait object.
pub struct MonoBehaviourHolder {
    inner: Box<dyn MonoBehaviour>,
    enabled: bool,
}

impl MonoBehaviourHolder {
    /// Create a new holder wrapping a MonoBehaviour.
    pub fn new(mono: impl MonoBehaviour + 'static) -> Self {
        Self {
            inner: Box::new(mono),
            enabled: true,
        }
    }

    /// Get a reference to the inner MonoBehaviour.
    pub fn get(&self) -> &dyn MonoBehaviour {
        &*self.inner
    }

    /// Get a mutable reference to the inner MonoBehaviour.
    pub fn get_mut(&mut self) -> &mut dyn MonoBehaviour {
        &mut *self.inner
    }

    /// Check if the holder is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.inner.is_enabled()
    }

    /// Set the enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get the type name (for debugging).
    pub fn type_name(&self) -> &str {
        std::any::type_name_of_val(&*self.inner)
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

    impl MonoBehaviour for TestComponent {
        fn update(&mut self, _context: &mut Context) {
            self.value += 1;
        }
    }

    #[test]
    fn test_monobehaviour_trait() {
        let comp = TestComponent { value: 0 };
        let mut holder = MonoBehaviourHolder::new(comp);

        assert!(holder.is_enabled());
        assert_eq!(
            holder
                .get_mut()
                .as_any_mut()
                .downcast_mut::<TestComponent>()
                .unwrap()
                .value,
            0
        );
    }

    #[test]
    fn test_monobehaviour_enabled() {
        let comp = TestComponent { value: 0 };
        let mut holder = MonoBehaviourHolder::new(comp);

        holder.set_enabled(false);
        assert!(!holder.is_enabled());

        holder.set_enabled(true);
        assert!(holder.is_enabled());
    }
}
