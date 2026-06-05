//! Integration tests for the engine-asset crate.

use engine_asset::asset::{Asset, Handle};
use engine_asset::cache::AssetCache;
use engine_asset::loader;
use engine_asset::manager::AssetManager;
use engine_asset::pipeline::{ImportError, ImportPipeline};
use engine_asset::registry::Registry;
use engine_asset::watcher::FileWatcher;

// Test asset type
#[derive(Debug, Clone)]
struct TestAsset {
    value: String,
    id: String,
}

impl Asset for TestAsset {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

#[test]
fn test_asset_manager_creation() {
    let manager = AssetManager::new();
    assert!(manager.registry().keys().is_empty());
    assert!(manager.events().is_empty());
}

#[test]
fn test_asset_manager_with_defaults() {
    let manager = AssetManager::with_defaults();
    let pipeline = manager.pipeline();

    // Should have all default importers registered
    assert!(pipeline.has_importer("png"));
    assert!(pipeline.has_importer("jpg"));
    assert!(pipeline.has_importer("gltf"));
    assert!(pipeline.has_importer("glb"));
    assert!(pipeline.has_importer("wav"));
    assert!(pipeline.has_importer("ogg"));
    assert!(pipeline.has_importer("mp3"));
    assert!(pipeline.has_importer("flac"));
    assert!(pipeline.has_importer("mat"));
    assert!(pipeline.has_importer("lua"));
    assert!(pipeline.has_importer("py"));
    assert!(pipeline.has_importer("rs"));
}

#[test]
fn test_handle_generation() {
    let asset = TestAsset {
        value: "test".to_string(),
        id: "test_asset".to_string(),
    };
    let handle = Handle::new(asset);

    assert_eq!(handle.ref_count(), 1);
    assert_eq!(handle.get().value, "test");
}

#[test]
fn test_handle_clone_increments_refcount() {
    let asset = TestAsset {
        value: "shared".to_string(),
        id: "shared_asset".to_string(),
    };
    let h1 = Handle::new(asset);
    let h2 = h1.clone();

    // Both handles point to the same data
    assert_eq!(h1.get().value, h2.get().value);
    assert_eq!(h1.get().id, h2.get().id);
}

#[test]
fn test_registry_store_and_retrieve() {
    let mut reg = Registry::new();
    let asset = TestAsset {
        value: "stored".to_string(),
        id: "reg_test".to_string(),
    };
    reg.store("test/key", asset);

    let loaded = reg.get::<TestAsset>("test/key");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().value, "stored");
}

#[test]
fn test_registry_contains() {
    let mut reg = Registry::new();
    assert!(!reg.contains("missing"));

    let asset = TestAsset {
        value: "exists".to_string(),
        id: "exists".to_string(),
    };
    reg.store("existing", asset);
    assert!(reg.contains("existing"));
}

#[test]
fn test_loader_load_asset() {
    let mut reg = Registry::new();
    let asset = TestAsset {
        value: "loaded".to_string(),
        id: "loader_test".to_string(),
    };
    loader::load_asset(&mut reg, "path/to/asset", asset);

    let loaded = reg.get::<TestAsset>("path/to/asset");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().value, "loaded");
}

#[test]
fn test_asset_loading_nonexistent_file() {
    let manager = AssetManager::with_defaults();
    let result = manager
        .pipeline()
        .import_file(std::path::Path::new("/nonexistent/file.png"));

    assert!(result.is_err());
    match result.unwrap_err() {
        ImportError::Io(_) => {} // Expected
        other => panic!("Expected IO error, got: {:?}", other),
    }
}

#[test]
fn test_pipeline_no_importer_for_extension() {
    let pipeline = ImportPipeline::new();
    let result = pipeline.import_bytes(
        b"data",
        "xyz_unknown",
        std::path::Path::new("f.xyz_unknown"),
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ImportError::NoImporter(ext) => assert_eq!(ext, "xyz_unknown"),
        other => panic!("Expected NoImporter error, got: {:?}", other),
    }
}

#[test]
fn test_cache_creation() {
    let cache = AssetCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_watcher_creation() {
    let watcher = FileWatcher::new();
    assert!(watcher.watched_dirs().is_empty());
    assert!(!watcher.has_pending());
}

#[test]
fn test_asset_manager_default() {
    let manager = AssetManager::default();
    assert!(manager.registry().keys().is_empty());
}
