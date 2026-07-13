use std::any::Any;

use serde::{Serialize, de::DeserializeOwned};

/// Base trait for scriptable data assets (like Unity's ScriptableObject).
/// Provides lifecycle callbacks, naming, and asset path tracking.
pub trait ScriptableObject: Any + Send + Sync + Serialize + DeserializeOwned {
    /// Called when the ScriptableObject is created.
    fn on_create(&mut self) {}

    /// Called when the ScriptableObject is enabled.
    fn on_enable(&mut self) {}

    /// Called when the ScriptableObject is disabled.
    fn on_disable(&mut self) {}

    /// Called when the ScriptableObject is destroyed.
    fn on_destroy(&mut self) {}

    /// Get the name of this ScriptableObject.
    fn name(&self) -> &str;

    /// Set the name of this ScriptableObject.
    fn set_name(&mut self, _name: &str) {}

    /// Get the asset path of this ScriptableObject.
    fn asset_path(&self) -> Option<&str> {
        None
    }

    /// Set the asset path of this ScriptableObject.
    fn set_asset_path(&mut self, _path: &str) {}

    /// Get this object as Any for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Get this object as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Check if the ScriptableObject is enabled.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Set the enabled state.
    fn set_enabled(&mut self, _enabled: bool) {}
}

/// Wrapper that stores a boxed ScriptableObject.
pub struct ScriptableObjectHolder<T: ScriptableObject> {
    inner: Box<T>,
    enabled: bool,
}

impl<T: ScriptableObject> ScriptableObjectHolder<T> {
    /// Create a new holder wrapping a ScriptableObject.
    /// Calls `on_create()` on the wrapped object.
    pub fn new(obj: T) -> Self {
        let mut holder = Self {
            inner: Box::new(obj),
            enabled: true,
        };
        holder.inner.on_create();
        holder
    }

    /// Create a new holder wrapping a ScriptableObject with a name and asset path.
    /// Calls `on_create()` on the wrapped object after setting the name and path.
    pub fn with_path(obj: T, name: &str, path: &str) -> Self {
        let mut inner = Box::new(obj);
        inner.set_name(name);
        inner.set_asset_path(path);
        let mut holder = Self {
            inner,
            enabled: true,
        };
        holder.inner.on_create();
        holder
    }

    /// Get a reference to the inner ScriptableObject.
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner ScriptableObject.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Check if the holder is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.inner.is_enabled()
    }

    /// Set the enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get the name of the wrapped object.
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get the asset path of the wrapped object.
    pub fn asset_path(&self) -> Option<&str> {
        self.inner.asset_path()
    }

    /// Get the type name (for debugging).
    pub fn type_name(&self) -> &str {
        std::any::type_name_of_val(&*self.inner)
    }

    /// Downcast the inner object to a concrete type.
    pub fn downcast_ref(&self) -> Option<&T> {
        Some(&*self.inner)
    }

    /// Downcast the inner object to a concrete type mutably.
    pub fn downcast_mut(&mut self) -> Option<&mut T> {
        Some(&mut *self.inner)
    }
}

impl<T: ScriptableObject> Drop for ScriptableObjectHolder<T> {
    fn drop(&mut self) {
        self.inner.on_destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static CREATE_CALLED: AtomicBool = AtomicBool::new(false);
    static DESTROY_CALLED: AtomicBool = AtomicBool::new(false);

    #[derive(Debug, Serialize, serde::Deserialize)]
    struct TestAsset {
        name: String,
        asset_path: Option<String>,
        value: i32,
        enabled: bool,
    }

    impl ScriptableObject for TestAsset {
        fn on_create(&mut self) {
            CREATE_CALLED.store(true, Ordering::SeqCst);
        }

        fn on_destroy(&mut self) {
            DESTROY_CALLED.store(true, Ordering::SeqCst);
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn set_name(&mut self, name: &str) {
            self.name = name.to_string();
        }

        fn asset_path(&self) -> Option<&str> {
            self.asset_path.as_deref()
        }

        fn set_asset_path(&mut self, path: &str) {
            self.asset_path = Some(path.to_string());
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    fn make_test_asset() -> TestAsset {
        TestAsset {
            name: "TestAsset".to_string(),
            asset_path: None,
            value: 42,
            enabled: true,
        }
    }

    #[test]
    fn test_scriptable_object_trait() {
        let holder = ScriptableObjectHolder::new(make_test_asset());

        assert!(holder.is_enabled());
        assert_eq!(holder.name(), "TestAsset");
        assert_eq!(holder.get().value, 42);
    }

    #[test]
    fn test_scriptable_object_enabled() {
        let mut holder = ScriptableObjectHolder::new(make_test_asset());

        holder.set_enabled(false);
        assert!(!holder.is_enabled());

        holder.set_enabled(true);
        assert!(holder.is_enabled());
    }

    #[test]
    fn test_scriptable_object_downcast_mut() {
        let mut holder = ScriptableObjectHolder::new(make_test_asset());

        {
            let inner = holder.get_mut();
            inner.value = 100;
        }

        assert_eq!(holder.get().value, 100);
    }

    #[test]
    fn test_scriptable_object_asset_path() {
        let holder = ScriptableObjectHolder::new(make_test_asset());
        assert!(holder.asset_path().is_none());
    }

    #[test]
    fn test_lifecycle_on_create_and_on_destroy() {
        CREATE_CALLED.store(false, Ordering::SeqCst);
        DESTROY_CALLED.store(false, Ordering::SeqCst);

        let holder = ScriptableObjectHolder::new(make_test_asset());
        assert!(
            CREATE_CALLED.load(Ordering::SeqCst),
            "on_create should be called on construction"
        );

        drop(holder);
        assert!(
            DESTROY_CALLED.load(Ordering::SeqCst),
            "on_destroy should be called on drop"
        );
    }

    #[test]
    fn test_with_path_sets_name_and_asset_path() {
        CREATE_CALLED.store(false, Ordering::SeqCst);
        DESTROY_CALLED.store(false, Ordering::SeqCst);

        let holder =
            ScriptableObjectHolder::with_path(make_test_asset(), "MyData", "/Game/Data/MyData");

        assert_eq!(holder.name(), "MyData");
        assert_eq!(holder.asset_path(), Some("/Game/Data/MyData"));
        assert!(CREATE_CALLED.load(Ordering::SeqCst));

        drop(holder);
        assert!(DESTROY_CALLED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_serde_roundtrip() {
        let holder = ScriptableObjectHolder::new(make_test_asset());
        let json = serde_json::to_string(holder.get()).unwrap();
        let deserialized: TestAsset = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "TestAsset");
        assert_eq!(deserialized.value, 42);
    }
}
