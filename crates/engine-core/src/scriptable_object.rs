//! Unity ScriptableObject — data container for sharing data.
//!
//! Maps to `UnityEngine.ScriptableObject` in Unity's documentation.
//!
//! # Unity Documentation
//! <https://docs.unity3d.com/ScriptReference/ScriptableObject.html>
//!
//! ScriptableObject is a data container that lives independently of GameObjects.
//! It's used for shared data that avoids duplication across prefab instances.
//!
//! ## Key Concepts
//! - ScriptableObject inherits from Object (NOT MonoBehaviour)
//! - Cannot be attached to GameObjects — stored as project assets
//! - Created via `ScriptableObject.CreateInstance<T>()`
//! - Serialized using the same rules as MonoBehaviour fields
//! - Lifecycle: `Awake()`, `OnEnable()`, `OnDisable()`, `OnDestroy()`
//!
//! ## Usage
//! ```ignore
//! #[derive(Serialize, Deserialize)]
//! struct EnemyData {
//!     name: String,
//!     health: f32,
//!     speed: f32,
//! }
//!
//! impl ScriptableObject for EnemyData {
//!     fn Name(&self) -> &str { &self.name }
//!     fn SetName(&mut self, name: &str) { self.name = name.to_string(); }
//!     fn GetInstanceID(&self) -> i32 { 0 }
//! }
//! ```

use std::any::Any;
use std::sync::atomic::{AtomicI32, Ordering};

use serde::{Deserialize, Serialize};

use crate::object::{InstanceId, Object};

/// Global counter for instance IDs.
static NEXT_INSTANCE_ID: AtomicI32 = AtomicI32::new(1);

/// Base trait for scriptable data assets (matches `UnityEngine.ScriptableObject`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/ScriptableObject.html>
///
/// ## Key Differences from MonoBehaviour
/// - ScriptableObject is NOT attached to GameObjects
/// - ScriptableObject is stored as project assets
/// - ScriptableObject does NOT receive Update/FixedUpdate/LateUpdate
/// - ScriptableObject has simpler lifecycle (Awake, OnEnable, OnDisable, OnDestroy)
///
/// ## Static Methods
/// - `CreateInstance<T>()` — creates a new instance
///
/// ## Lifecycle
/// - `Awake()` — called when ScriptableObject is created
/// - `OnEnable()` — called when ScriptableObject is loaded
/// - `OnDisable()` — called when ScriptableObject goes out of scope
/// - `OnDestroy()` — called when ScriptableObject is destroyed
pub trait ScriptableObject:
    Object + Any + Send + Sync + Serialize + for<'de> Deserialize<'de>
{
    /// Create a new instance of this ScriptableObject (matches `ScriptableObject.CreateInstance<T>()`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/ScriptableObject.CreateInstance.html>
    ///
    /// This is a default implementation that requires `Default`.
    /// Override this for custom creation logic.
    fn CreateInstance<T: ScriptableObject + Default>() -> T {
        T::default()
    }

    /// Called when the ScriptableObject is created (matches `ScriptableObject.Awake`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/ScriptableObject.Awake.html>
    fn Awake(&mut self) {}

    /// Called when the ScriptableObject is loaded (matches `ScriptableObject.OnEnable`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/ScriptableObject.OnEnable.html>
    fn OnEnable(&mut self) {}

    /// Called when the ScriptableObject goes out of scope (matches `ScriptableObject.OnDisable`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/ScriptableObject.OnDisable.html>
    fn OnDisable(&mut self) {}

    /// Called when the ScriptableObject is destroyed (matches `ScriptableObject.OnDestroy`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/ScriptableObject.OnDestroy.html>
    fn OnDestroy(&mut self) {}

    /// Called when script properties are set in the editor (matches `ScriptableObject.OnValidate`).
    fn OnValidate(&mut self) {}

    /// Get the asset path of this ScriptableObject.
    fn AssetPath(&self) -> Option<&str> {
        None
    }

    /// Set the asset path of this ScriptableObject.
    fn SetAssetPath(&mut self, _path: &str) {}
}

/// Wrapper that stores a boxed ScriptableObject.
pub struct ScriptableObjectHolder<T: ScriptableObject> {
    inner: Box<T>,
    enabled: bool,
    instance_id: InstanceId,
}

impl<T: ScriptableObject> ScriptableObjectHolder<T> {
    /// Create a new holder wrapping a ScriptableObject.
    /// Calls `Awake()` on the wrapped object.
    pub fn new(obj: T) -> Self {
        let instance_id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::SeqCst);
        let mut holder = Self {
            inner: Box::new(obj),
            enabled: true,
            instance_id,
        };
        holder.inner.Awake();
        holder.inner.OnEnable();
        holder
    }

    /// Create a new holder wrapping a ScriptableObject with a name and asset path.
    pub fn with_path(obj: T, name: &str, path: &str) -> Self {
        let instance_id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::SeqCst);
        let mut inner = Box::new(obj);
        inner.SetName(name);
        inner.SetAssetPath(path);
        let mut holder = Self {
            inner,
            enabled: true,
            instance_id,
        };
        holder.inner.Awake();
        holder.inner.OnEnable();
        holder
    }

    /// Get a reference to the inner ScriptableObject.
    pub fn Get(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner ScriptableObject.
    pub fn GetMut(&mut self) -> &mut T {
        &mut *self.inner
    }

    /// Check if the holder is enabled.
    pub fn Enabled(&self) -> bool {
        self.enabled
    }

    /// Set the enabled state.
    pub fn SetEnabled(&mut self, enabled: bool) {
        if self.enabled && !enabled {
            self.inner.OnDisable();
        } else if !self.enabled && enabled {
            self.inner.OnEnable();
        }
        self.enabled = enabled;
    }

    /// Get the instance ID.
    pub fn GetInstanceID(&self) -> InstanceId {
        self.instance_id
    }

    /// Get the type name (for debugging).
    pub fn TypeName(&self) -> &str {
        std::any::type_name_of_val(&*self.inner)
    }

    /// Downcast the inner object to a concrete type.
    pub fn DowncastRef(&self) -> Option<&T> {
        Some(&*self.inner)
    }

    /// Downcast the inner object to a concrete type mutably.
    pub fn DowncastMut(&mut self) -> Option<&mut T> {
        Some(&mut *self.inner)
    }
}

impl<T: ScriptableObject> Drop for ScriptableObjectHolder<T> {
    fn drop(&mut self) {
        self.inner.OnDisable();
        self.inner.OnDestroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static AWAKE_CALLED: AtomicBool = AtomicBool::new(false);
    static DESTROY_CALLED: AtomicBool = AtomicBool::new(false);

    #[derive(Debug, Serialize, Deserialize)]
    struct TestAsset {
        name: String,
        asset_path: Option<String>,
        value: i32,
        enabled: bool,
    }

    impl Default for TestAsset {
        fn default() -> Self {
            Self {
                name: "TestAsset".to_string(),
                asset_path: None,
                value: 42,
                enabled: true,
            }
        }
    }

    impl Object for TestAsset {
        fn Name(&self) -> &str {
            &self.name
        }

        fn SetName(&mut self, name: &str) {
            self.name = name.to_string();
        }

        fn GetInstanceID(&self) -> InstanceId {
            0
        }
    }

    impl ScriptableObject for TestAsset {
        fn Awake(&mut self) {
            AWAKE_CALLED.store(true, Ordering::SeqCst);
        }

        fn OnDestroy(&mut self) {
            DESTROY_CALLED.store(true, Ordering::SeqCst);
        }

        fn AssetPath(&self) -> Option<&str> {
            self.asset_path.as_deref()
        }

        fn SetAssetPath(&mut self, path: &str) {
            self.asset_path = Some(path.to_string());
        }
    }

    fn make_test_asset() -> TestAsset {
        TestAsset::default()
    }

    #[test]
    fn test_scriptable_object_trait() {
        let holder = ScriptableObjectHolder::new(make_test_asset());

        assert!(holder.Enabled());
        assert_eq!(holder.Get().name, "TestAsset");
        assert_eq!(holder.Get().value, 42);
    }

    #[test]
    fn test_scriptable_object_enabled() {
        let mut holder = ScriptableObjectHolder::new(make_test_asset());

        holder.SetEnabled(false);
        assert!(!holder.Enabled());

        holder.SetEnabled(true);
        assert!(holder.Enabled());
    }

    #[test]
    fn test_scriptable_object_downcast_mut() {
        let mut holder = ScriptableObjectHolder::new(make_test_asset());

        {
            let inner = holder.GetMut();
            inner.value = 100;
        }

        assert_eq!(holder.Get().value, 100);
    }

    #[test]
    fn test_scriptable_object_asset_path() {
        let holder = ScriptableObjectHolder::new(make_test_asset());
        assert!(holder.Get().AssetPath().is_none());
    }

    #[test]
    fn test_lifecycle_awake_and_destroy() {
        AWAKE_CALLED.store(false, Ordering::SeqCst);
        DESTROY_CALLED.store(false, Ordering::SeqCst);

        let holder = ScriptableObjectHolder::new(make_test_asset());
        assert!(
            AWAKE_CALLED.load(Ordering::SeqCst),
            "Awake should be called on construction"
        );

        drop(holder);
        assert!(
            DESTROY_CALLED.load(Ordering::SeqCst),
            "OnDestroy should be called on drop"
        );
    }

    #[test]
    fn test_with_path_sets_name_and_asset_path() {
        AWAKE_CALLED.store(false, Ordering::SeqCst);
        DESTROY_CALLED.store(false, Ordering::SeqCst);

        let holder =
            ScriptableObjectHolder::with_path(make_test_asset(), "MyData", "/Game/Data/MyData");

        assert_eq!(holder.Get().Name(), "MyData");
        assert_eq!(holder.Get().AssetPath(), Some("/Game/Data/MyData"));
        assert!(AWAKE_CALLED.load(Ordering::SeqCst));

        drop(holder);
        assert!(DESTROY_CALLED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_serde_roundtrip() {
        let holder = ScriptableObjectHolder::new(make_test_asset());
        let json = serde_json::to_string(holder.Get()).unwrap();
        let deserialized: TestAsset = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "TestAsset");
        assert_eq!(deserialized.value, 42);
    }

    #[test]
    fn test_create_instance() {
        let asset = TestAsset::CreateInstance::<TestAsset>();
        assert_eq!(asset.name, "TestAsset");
        assert_eq!(asset.value, 42);
    }
}
