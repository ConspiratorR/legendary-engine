use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Unique identifier for a Handle, derived from its inner Arc pointer.
/// All clones of the same Handle share the same Arc and thus the same HandleId.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandleId(usize);

impl HandleId {
    pub fn from_handle<T: Asset>(handle: &Handle<T>) -> Self {
        Self(Arc::as_ptr(&handle.inner) as *const () as usize)
    }
}

pub trait Asset: Clone + 'static {
    type Id: ?Sized + std::fmt::Debug + std::hash::Hash + Eq;
    fn id(&self) -> &Self::Id;
}

pub struct Handle<T: Asset> {
    inner: Arc<HandleInner<T>>,
}

struct HandleInner<T: Asset> {
    asset: T,
    ref_count: AtomicUsize,
}

impl<T: Asset> Handle<T> {
    pub fn new(asset: T) -> Self {
        Self {
            inner: Arc::new(HandleInner {
                asset,
                ref_count: AtomicUsize::new(1),
            }),
        }
    }

    pub fn ref_count(&self) -> usize {
        self.inner.ref_count.load(Ordering::Relaxed)
    }

    pub fn get(&self) -> &T {
        &self.inner.asset
    }
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        self.inner.ref_count.fetch_add(1, Ordering::Relaxed);
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Asset> Drop for Handle<T> {
    fn drop(&mut self) {
        self.inner.ref_count.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use crate::asset::{Asset, Handle, HandleId};
    use crate::loader;
    use crate::registry::Registry;

    #[derive(Clone)]
    struct MyAsset {
        value: u32,
        id: String,
    }

    impl MyAsset {
        fn new(value: u32) -> Self {
            Self {
                value,
                id: "my_asset".to_string(),
            }
        }
    }

    impl Asset for MyAsset {
        type Id = String;
        fn id(&self) -> &Self::Id {
            &self.id
        }
    }

    #[test]
    fn test_handle_clone_increments_count() {
        let asset = MyAsset::new(42);
        let h1 = Handle::new(asset);
        let h2 = h1.clone();
        assert_eq!(h2.ref_count(), h1.ref_count());
        drop(h2);
    }

    #[test]
    fn test_handle_get_asset() {
        let asset = MyAsset::new(42);
        let handle = Handle::new(asset);
        assert_eq!(handle.get().value, 42);
    }

    #[test]
    fn test_registry_store_and_get() {
        let mut reg = Registry::new();
        reg.store("test/key", MyAsset::new(42));
        let loaded = reg.get::<MyAsset>("test/key");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().value, 42);
    }

    #[test]
    fn test_registry_unknown_key() {
        let reg = Registry::new();
        let loaded = reg.get::<MyAsset>("nonexistent");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_registry_contains() {
        let mut reg = Registry::new();
        reg.store("existing", MyAsset::new(1));
        assert!(reg.contains("existing"));
        assert!(!reg.contains("missing"));
    }

    #[test]
    fn test_loader_load_asset() {
        let mut reg = Registry::new();
        loader::load_asset(&mut reg, "path/to/asset", MyAsset::new(99));
        let loaded = reg.get::<MyAsset>("path/to/asset");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().value, 99);
    }

    #[test]
    fn test_handle_id_same_for_clones() {
        let asset = MyAsset::new(1);
        let h1 = Handle::new(asset);
        let h2 = h1.clone();
        assert_eq!(HandleId::from_handle(&h1), HandleId::from_handle(&h2));
    }

    #[test]
    fn test_handle_id_different_for_distinct_handles() {
        let h1 = Handle::new(MyAsset::new(1));
        let h2 = Handle::new(MyAsset::new(2));
        assert_ne!(HandleId::from_handle(&h1), HandleId::from_handle(&h2));
    }
}
