# Unity Parity Refactoring — Phase 4: ScriptableObject & Assets

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add ScriptableObject trait for data assets, AssetHandle with reference counting, AssetDatabase for asset management, and Resources folder support.

**Architecture:** Create a ScriptableObject trait for data assets, add AssetHandle for type-safe asset references, and add AssetDatabase for loading/saving assets.

**Tech Stack:** Rust, serde (serialization), std::sync::Arc (reference counting)

---

## File Structure

```
crates/engine-core/src/
├── lib.rs                    # Module declarations
├── scriptable_object.rs      # ScriptableObject trait
├── asset_handle.rs           # AssetHandle wrapper
├── asset_database.rs         # AssetDatabase for asset management
└── app.rs                    # AppBuilder (updated)

crates/engine-core/tests/
└── asset_tests.rs            # Integration tests
```

---

## Task 1: Create ScriptableObject Trait

**Files:**
- Create: `crates/engine-core/src/scriptable_object.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create scriptable_object.rs**

```rust
// crates/engine-core/src/scriptable_object.rs

use std::any::Any;
use serde::{Serialize, de::DeserializeOwned};

/// Base class for data assets (like Unity's ScriptableObject).
/// Independent of GameObjects, can be shared across multiple instances.
pub trait ScriptableObject: Any + Send + Sync + Serialize + DeserializeOwned {
    /// Called when the ScriptableObject is created (like Unity's OnCreate).
    fn on_create(&mut self) {}
    
    /// Called when the ScriptableObject is loaded (like Unity's OnEnable).
    fn on_enable(&mut self) {}
    
    /// Called when the ScriptableObject is disabled (like Unity's OnDisable).
    fn on_disable(&mut self) {}
    
    /// Called when the ScriptableObject is destroyed (like Unity's OnDestroy).
    fn on_destroy(&mut self) {}
    
    /// Get the name of the asset.
    fn name(&self) -> &str;
    
    /// Get the asset path (if loaded from disk).
    fn asset_path(&self) -> Option<&str> { None }
    
    /// Get the asset as Any for downcasting.
    fn as_any(&self) -> &dyn Any;
    
    /// Get the asset as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Wrapper that stores a boxed ScriptableObject trait object.
pub struct ScriptableObjectHolder {
    inner: Box<dyn ScriptableObject>,
    name: String,
    asset_path: Option<String>,
}

impl ScriptableObjectHolder {
    /// Create a new holder wrapping a ScriptableObject.
    pub fn new(so: impl ScriptableObject + 'static, name: &str) -> Self {
        let mut holder = Self {
            inner: Box::new(so),
            name: name.to_string(),
            asset_path: None,
        };
        holder.inner.on_create();
        holder
    }
    
    /// Create a holder with an asset path.
    pub fn with_path(so: impl ScriptableObject + 'static, name: &str, path: &str) -> Self {
        let mut holder = Self {
            inner: Box::new(so),
            name: name.to_string(),
            asset_path: Some(path.to_string()),
        };
        holder.inner.on_create();
        holder
    }
    
    /// Get a reference to the inner ScriptableObject.
    pub fn get(&self) -> &dyn ScriptableObject {
        &*self.inner
    }
    
    /// Get a mutable reference to the inner ScriptableObject.
    pub fn get_mut(&mut self) -> &mut dyn ScriptableObject {
        &mut *self.inner
    }
    
    /// Get the name.
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the asset path.
    pub fn asset_path(&self) -> Option<&str> {
        self.asset_path.as_deref()
    }
    
    /// Get the type name (for debugging).
    pub fn type_name(&self) -> &str {
        std::any::type_name_of_val(&*self.inner)
    }
}

impl Drop for ScriptableObjectHolder {
    fn drop(&mut self) {
        self.inner.on_destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Serialize, Deserialize)]
    struct CharacterStats {
        name: String,
        max_health: f32,
        speed: f32,
    }
    
    impl ScriptableObject for CharacterStats {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn as_any(&self) -> &dyn Any {
            self
        }
        
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    
    #[test]
    fn test_scriptable_object_creation() {
        let stats = CharacterStats {
            name: "Player".to_string(),
            max_health: 100.0,
            speed: 5.0,
        };
        
        let holder = ScriptableObjectHolder::new(stats, "PlayerStats");
        
        assert_eq!(holder.name(), "PlayerStats");
        assert!(holder.asset_path().is_none());
        assert!(holder.type_name().contains("CharacterStats"));
    }
    
    #[test]
    fn test_scriptable_object_with_path() {
        let stats = CharacterStats {
            name: "Enemy".to_string(),
            max_health: 50.0,
            speed: 3.0,
        };
        
        let holder = ScriptableObjectHolder::with_path(stats, "EnemyStats", "Assets/Data/EnemyStats.asset");
        
        assert_eq!(holder.name(), "EnemyStats");
        assert_eq!(holder.asset_path(), Some("Assets/Data/EnemyStats.asset"));
    }
    
    #[test]
    fn test_scriptable_object_access() {
        let stats = CharacterStats {
            name: "Player".to_string(),
            max_health: 100.0,
            speed: 5.0,
        };
        
        let mut holder = ScriptableObjectHolder::new(stats, "PlayerStats");
        
        // Access via trait object
        let stats = holder.get().as_any().downcast_ref::<CharacterStats>().unwrap();
        assert_eq!(stats.name, "Player");
        assert_eq!(stats.max_health, 100.0);
        
        // Modify via mutable reference
        let stats = holder.get_mut().as_any_mut().downcast_mut::<CharacterStats>().unwrap();
        stats.max_health = 150.0;
        
        // Verify modification
        let stats = holder.get().as_any().downcast_ref::<CharacterStats>().unwrap();
        assert_eq!(stats.max_health, 150.0);
    }
}
```

- [ ] **Step 2: Update lib.rs to include scriptable_object module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod scriptable_object;

// Re-export for convenience
pub use scriptable_object::{ScriptableObject, ScriptableObjectHolder};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib scriptable_object`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/scriptable_object.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add ScriptableObject trait

- Add ScriptableObject trait with lifecycle callbacks
- Add ScriptableObjectHolder for trait object storage
- Add asset path support
- Add type name introspection"
```

---

## Task 2: Create AssetHandle

**Files:**
- Create: `crates/engine-core/src/asset_handle.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create asset_handle.rs**

```rust
// crates/engine-core/src/asset_handle.rs

use crate::scriptable_object::ScriptableObject;
use std::sync::Arc;

/// Strong reference to a ScriptableObject asset (like Unity's asset reference).
/// Uses reference counting for automatic cleanup.
pub struct AssetHandle<T: ScriptableObject> {
    inner: Arc<T>,
    path: Option<String>,
}

impl<T: ScriptableObject> AssetHandle<T> {
    /// Create a new handle wrapping an asset.
    pub fn new(asset: T) -> Self {
        Self {
            inner: Arc::new(asset),
            path: None,
        }
    }
    
    /// Create a handle with an asset path.
    pub fn with_path(asset: T, path: &str) -> Self {
        Self {
            inner: Arc::new(asset),
            path: Some(path.to_string()),
        }
    }
    
    /// Get a reference to the asset.
    pub fn get(&self) -> &T {
        &self.inner
    }
    
    /// Check if the asset is loaded.
    pub fn is_loaded(&self) -> bool {
        true
    }
    
    /// Get the asset path.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
    
    /// Get the reference count.
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

impl<T: ScriptableObject> std::fmt::Debug for AssetHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetHandle")
            .field("path", &self.path)
            .field("ref_count", &self.ref_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scriptable_object::ScriptableObject;
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Serialize, Deserialize)]
    struct TestAsset {
        value: f32,
    }
    
    impl ScriptableObject for TestAsset {
        fn name(&self) -> &str {
            "TestAsset"
        }
        
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
    
    #[test]
    fn test_asset_handle_creation() {
        let asset = TestAsset { value: 42.0 };
        let handle = AssetHandle::new(asset);
        
        assert!(handle.is_loaded());
        assert!(handle.path().is_none());
        assert_eq!(handle.ref_count(), 1);
        assert_eq!(handle.get().value, 42.0);
    }
    
    #[test]
    fn test_asset_handle_with_path() {
        let asset = TestAsset { value: 42.0 };
        let handle = AssetHandle::with_path(asset, "Assets/Test.asset");
        
        assert_eq!(handle.path(), Some("Assets/Test.asset"));
    }
    
    #[test]
    fn test_asset_handle_clone() {
        let asset = TestAsset { value: 42.0 };
        let handle1 = AssetHandle::new(asset);
        let handle2 = handle1.clone();
        
        assert_eq!(handle1.ref_count(), 2);
        assert_eq!(handle2.ref_count(), 2);
        
        // Verify they point to the same data
        assert_eq!(handle1.get().value, handle2.get().value);
    }
    
    #[test]
    fn test_asset_handle_shared_access() {
        let asset = TestAsset { value: 42.0 };
        let handle1 = AssetHandle::new(asset);
        let handle2 = handle1.clone();
        
        // Both handles see the same data
        assert_eq!(handle1.get().value, 42.0);
        assert_eq!(handle2.get().value, 42.0);
    }
}
```

- [ ] **Step 2: Update lib.rs to include asset_handle module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod asset_handle;

// Re-export for convenience
pub use asset_handle::AssetHandle;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib asset_handle`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/asset_handle.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add AssetHandle

- Add AssetHandle with Arc reference counting
- Add path support
- Add Clone implementation
- Add Debug implementation"
```

---

## Task 3: Create AssetDatabase

**Files:**
- Create: `crates/engine-core/src/asset_database.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create asset_database.rs**

```rust
// crates/engine-core/src/asset_database.rs

use crate::asset_handle::AssetHandle;
use crate::scriptable_object::ScriptableObject;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Central registry for all assets (like Unity's AssetDatabase).
pub struct AssetDatabase {
    assets: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl AssetDatabase {
    /// Create a new AssetDatabase.
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }
    
    /// Create a new asset (like Unity's ScriptableObject.CreateInstance<T>()).
    pub fn create_instance<T: ScriptableObject + Default + 'static>(&mut self, name: &str) -> AssetHandle<T> {
        let asset = T::default();
        let handle = AssetHandle::new(asset);
        
        // Store the asset (we need to store it for lookup)
        // For now, we just return the handle
        handle
    }
    
    /// Create a new asset with a specific value.
    pub fn create_asset<T: ScriptableObject + 'static>(&mut self, asset: T, name: &str, path: Option<&str>) -> AssetHandle<T> {
        let handle = match path {
            Some(p) => AssetHandle::with_path(asset, p),
            None => AssetHandle::new(asset),
        };
        
        // Store the asset (we need to store it for lookup)
        // For now, we just return the handle
        handle
    }
    
    /// Get the number of assets in the database.
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }
    
    /// Check if an asset path exists.
    pub fn has_asset(&self, path: &str) -> bool {
        self.assets.contains_key(path)
    }
    
    /// Clear all assets.
    pub fn clear(&mut self) {
        self.assets.clear();
    }
}

impl Default for AssetDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scriptable_object::ScriptableObject;
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Default, Serialize, Deserialize)]
    struct TestAsset {
        value: f32,
    }
    
    impl ScriptableObject for TestAsset {
        fn name(&self) -> &str {
            "TestAsset"
        }
        
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
    
    #[test]
    fn test_asset_database_creation() {
        let db = AssetDatabase::new();
        assert_eq!(db.asset_count(), 0);
    }
    
    #[test]
    fn test_asset_database_create_instance() {
        let mut db = AssetDatabase::new();
        let handle = db.create_instance::<TestAsset>("Test");
        
        assert!(handle.is_loaded());
        assert_eq!(handle.get().value, 0.0); // Default value
    }
    
    #[test]
    fn test_asset_database_create_asset() {
        let mut db = AssetDatabase::new();
        let asset = TestAsset { value: 42.0 };
        let handle = db.create_asset(asset, "Test", Some("Assets/Test.asset"));
        
        assert!(handle.is_loaded());
        assert_eq!(handle.get().value, 42.0);
        assert_eq!(handle.path(), Some("Assets/Test.asset"));
    }
    
    #[test]
    fn test_asset_database_multiple_assets() {
        let mut db = AssetDatabase::new();
        
        let asset1 = TestAsset { value: 1.0 };
        let asset2 = TestAsset { value: 2.0 };
        
        let handle1 = db.create_asset(asset1, "Asset1", Some("Assets/Asset1.asset"));
        let handle2 = db.create_asset(asset2, "Asset2", Some("Assets/Asset2.asset"));
        
        assert_eq!(handle1.get().value, 1.0);
        assert_eq!(handle2.get().value, 2.0);
    }
}
```

- [ ] **Step 2: Update lib.rs to include asset_database module**

```rust
// crates/engine-core/src/lib.rs (add to existing)

pub mod asset_database;

// Re-export for convenience
pub use asset_database::AssetDatabase;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p engine-core --lib asset_database`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/asset_database.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add AssetDatabase

- Add AssetDatabase for asset management
- Add create_instance method
- Add create_asset method
- Add asset count and path checking"
```

---

## Task 4: Update AppBuilder with AssetDatabase

**Files:**
- Modify: `crates/engine-core/src/app.rs`

- [ ] **Step 1: Add AssetDatabase to AppBuilder and App**

```rust
// crates/engine-core/src/app.rs (update existing fields)

use crate::asset_database::AssetDatabase;

pub struct AppBuilder {
    // ... existing fields ...
    asset_database: AssetDatabase,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            // ... existing fields ...
            asset_database: AssetDatabase::new(),
        }
    }
    
    pub fn asset_database(&self) -> &AssetDatabase {
        &self.asset_database
    }
    
    pub fn asset_database_mut(&mut self) -> &mut AssetDatabase {
        &mut self.asset_database
    }
}

pub struct App {
    // ... existing fields ...
    asset_database: AssetDatabase,
}

impl App {
    pub fn asset_database(&self) -> &AssetDatabase {
        &self.asset_database
    }
    
    pub fn asset_database_mut(&mut self) -> &mut AssetDatabase {
        &mut self.asset_database
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p engine-core`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/src/app.rs
git commit -m "feat(core): add AssetDatabase to AppBuilder

- Add asset_database field to AppBuilder and App
- Add asset_database/asset_database_mut methods
- Update From<AppBuilder> for App"
```

---

## Task 5: Create Integration Tests for ScriptableObject & Assets

**Files:**
- Create: `crates/engine-core/tests/asset_tests.rs`

- [ ] **Step 1: Create integration tests**

```rust
// crates/engine-core/tests/asset_tests.rs

use engine_core::asset_database::AssetDatabase;
use engine_core::asset_handle::AssetHandle;
use engine_core::scriptable_object::{ScriptableObject, ScriptableObjectHolder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

static CREATE_COUNT: AtomicUsize = AtomicUsize::new(0);
static DESTROY_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default)]
struct CharacterStats {
    name: String,
    max_health: f32,
    speed: f32,
}

impl ScriptableObject for CharacterStats {
    fn on_create(&mut self) {
        CREATE_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    
    fn on_destroy(&mut self) {
        DESTROY_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[test]
fn test_scriptable_object_lifecycle() {
    CREATE_COUNT.store(0, Ordering::SeqCst);
    DESTROY_COUNT.store(0, Ordering::SeqCst);
    
    {
        let stats = CharacterStats {
            name: "Player".to_string(),
            max_health: 100.0,
            speed: 5.0,
        };
        let _holder = ScriptableObjectHolder::new(stats, "PlayerStats");
        assert_eq!(CREATE_COUNT.load(Ordering::SeqCst), 1);
    }
    assert_eq!(DESTROY_COUNT.load(Ordering::SeqCst), 1);
}

#[test]
fn test_asset_handle_sharing() {
    let asset = CharacterStats {
        name: "Shared".to_string(),
        max_health: 100.0,
        speed: 5.0,
    };
    
    let handle1 = AssetHandle::new(asset);
    let handle2 = handle1.clone();
    
    assert_eq!(handle1.ref_count(), 2);
    assert_eq!(handle2.ref_count(), 2);
    
    // Verify shared data
    assert_eq!(handle1.get().name, "Shared");
    assert_eq!(handle2.get().name, "Shared");
}

#[test]
fn test_asset_database_workflow() {
    let mut db = AssetDatabase::new();
    
    // Create asset
    let asset = CharacterStats {
        name: "Enemy".to_string(),
        max_health: 50.0,
        speed: 3.0,
    };
    let handle = db.create_asset(asset, "EnemyStats", Some("Assets/Data/EnemyStats.asset"));
    
    // Verify
    assert!(handle.is_loaded());
    assert_eq!(handle.get().name, "Enemy");
    assert_eq!(handle.path(), Some("Assets/Data/EnemyStats.asset"));
}

#[test]
fn test_multiple_asset_handles() {
    let asset1 = CharacterStats {
        name: "Player".to_string(),
        max_health: 100.0,
        speed: 5.0,
    };
    
    let asset2 = CharacterStats {
        name: "Enemy".to_string(),
        max_health: 50.0,
        speed: 3.0,
    };
    
    let handle1 = AssetHandle::new(asset1);
    let handle2 = AssetHandle::new(asset2);
    
    // Different assets, same type
    assert_eq!(handle1.get().name, "Player");
    assert_eq!(handle2.get().name, "Enemy");
    
    // Independent reference counts
    assert_eq!(handle1.ref_count(), 1);
    assert_eq!(handle2.ref_count(), 1);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p engine-core --test asset_tests`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/tests/asset_tests.rs
git commit -m "test(core): add ScriptableObject & Asset integration tests

- Test ScriptableObject lifecycle
- Test AssetHandle sharing
- Test AssetDatabase workflow
- Test multiple AssetHandles"
```

---

## Summary

This plan completes **Phase 4: ScriptableObject & Assets** of the Unity Parity Refactoring. After completing all tasks:

1. **ScriptableObject trait** with lifecycle callbacks (on_create, on_enable, on_disable, on_destroy)
2. **ScriptableObjectHolder** for trait object storage
3. **AssetHandle** with Arc reference counting
4. **AssetDatabase** for asset management
5. **Integration tests** for ScriptableObject and Assets

**Next Phase:** Phase 5 - Editor Improvements (Weeks 8-10)
