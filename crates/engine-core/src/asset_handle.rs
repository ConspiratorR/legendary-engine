use std::fmt;
use std::sync::Arc;

use crate::scriptable_object::ScriptableObject;

/// Strong reference to a ScriptableObject asset (like Unity's asset reference).
/// Uses reference counting for automatic cleanup.
pub struct AssetHandle<T: ScriptableObject> {
    inner: Arc<T>,
    path: Option<String>,
}

impl<T: ScriptableObject> AssetHandle<T> {
    pub fn new(asset: T) -> Self {
        Self {
            inner: Arc::new(asset),
            path: None,
        }
    }

    pub fn with_path(asset: T, path: &str) -> Self {
        Self {
            inner: Arc::new(asset),
            path: Some(path.to_string()),
        }
    }

    pub fn get(&self) -> &T {
        &self.inner
    }

    pub fn is_loaded(&self) -> bool {
        true
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }
}

impl<T: ScriptableObject> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            path: self.path.clone(),
        }
    }
}

impl<T: ScriptableObject> fmt::Debug for AssetHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetHandle")
            .field("path", &self.path)
            .field("ref_count", &Arc::strong_count(&self.inner))
            .finish()
    }
}

impl<T: ScriptableObject> fmt::Display for AssetHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{p}"),
            None => write!(f, "<no path>"),
        }
    }
}

impl<T: ScriptableObject> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: ScriptableObject> Eq for AssetHandle<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::object::{InstanceId, Object};
    use crate::scriptable_object::ScriptableObject;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestAsset {
        name: String,
        value: i32,
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

    impl Component for TestAsset {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    impl ScriptableObject for TestAsset {}

    fn test_asset(name: &str, value: i32) -> TestAsset {
        TestAsset {
            name: name.to_string(),
            value,
        }
    }

    #[test]
    fn test_creation_new() {
        let asset = test_asset("sky", 42);
        let handle = AssetHandle::new(asset);
        assert_eq!(handle.get().Name(), "sky");
        assert_eq!(handle.path(), None);
        assert!(handle.is_loaded());
    }

    #[test]
    fn test_creation_with_path() {
        let asset = test_asset("mesh", 7);
        let handle = AssetHandle::with_path(asset, "assets/models/mesh.glb");
        assert_eq!(handle.get().Name(), "mesh");
        assert_eq!(handle.path(), Some("assets/models/mesh.glb"));
    }

    #[test]
    fn test_clone_shares_arc() {
        let h1 = AssetHandle::with_path(test_asset("a", 1), "a.txt");
        let h2 = h1.clone();
        assert_eq!(h1.get().Name(), h2.get().Name());
        assert!(Arc::ptr_eq(&h1.inner, &h2.inner));
        assert_eq!(h1.ref_count(), 2);
    }

    #[test]
    fn test_ref_count_decrements_on_drop() {
        let h1 = AssetHandle::new(test_asset("b", 2));
        assert_eq!(h1.ref_count(), 1);
        let h2 = h1.clone();
        assert_eq!(h1.ref_count(), 2);
        drop(h2);
        assert_eq!(h1.ref_count(), 1);
    }

    #[test]
    fn test_debug_format() {
        let h = AssetHandle::with_path(test_asset("test", 0), "test.txt");
        let dbg = format!("{:?}", h);
        assert!(dbg.contains("AssetHandle"));
        assert!(dbg.contains("test.txt"));
    }

    #[test]
    fn test_display_format_with_path() {
        let h = AssetHandle::with_path(test_asset("m", 0), "assets/model.fbx");
        assert_eq!(format!("{h}"), "assets/model.fbx");
    }

    #[test]
    fn test_display_format_no_path() {
        let h = AssetHandle::new(test_asset("m", 0));
        assert_eq!(format!("{h}"), "<no path>");
    }

    #[test]
    fn test_equality_same_handle() {
        let h1 = AssetHandle::new(test_asset("x", 0));
        let h2 = h1.clone();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_inequality_different_handles() {
        let h1 = AssetHandle::new(test_asset("a", 0));
        let h2 = AssetHandle::new(test_asset("b", 0));
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_get_returns_inner_asset() {
        let asset = test_asset("player", 100);
        let handle = AssetHandle::new(asset);
        assert_eq!(handle.get().Name(), "player");
    }

    #[test]
    fn test_is_loaded_always_true() {
        let handle = AssetHandle::new(test_asset("x", 0));
        assert!(handle.is_loaded());
    }
}
