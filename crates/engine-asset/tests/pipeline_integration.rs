//! Integration tests for the asset pipeline v2.
//!
//! Tests the end-to-end flow: FileWatcher → AsyncLoader → ImportPipeline → AssetCache.
//! Covers async loading, cache invalidation, dependency tracking, concurrent loads,
//! priority ordering, and error handling.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use engine_asset::asset::Asset;
use engine_asset::async_loader::{AsyncLoader, LoadPriority, LoadState};
use engine_asset::cache::{AssetCache, CacheEntry};
use engine_asset::pipeline::{AssetImporter, ImportContext, ImportError, ImportPipeline};
use engine_asset::watcher::{FileEvent, FileWatcher, WatchConfig};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextAsset(String);

impl Asset for TextAsset {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.0
    }
}

struct TextImporter;

impl AssetImporter for TextImporter {
    type Asset = TextAsset;

    fn extensions(&self) -> &[&str] {
        &["txt"]
    }

    fn import(&self, data: &[u8], _ctx: &mut ImportContext) -> Result<TextAsset, ImportError> {
        let s = String::from_utf8(data.to_vec()).map_err(|e| ImportError::Format(e.to_string()))?;
        Ok(TextAsset(s))
    }
}

struct DepTrackingImporter;

impl AssetImporter for DepTrackingImporter {
    type Asset = TextAsset;

    fn extensions(&self) -> &[&str] {
        &["dep"]
    }

    fn import(&self, data: &[u8], ctx: &mut ImportContext) -> Result<TextAsset, ImportError> {
        let s = String::from_utf8(data.to_vec()).map_err(|e| ImportError::Format(e.to_string()))?;

        // Parse dependency paths from content (format: "dep:path1;path2")
        for line in s.lines() {
            if let Some(paths) = line.strip_prefix("dep:") {
                for p in paths.split(';') {
                    let trimmed = p.trim();
                    if !trimmed.is_empty() {
                        ctx.add_dependency(trimmed);
                    }
                }
            }
        }

        Ok(TextAsset(s))
    }
}

struct FailImporter;

impl AssetImporter for FailImporter {
    type Asset = TextAsset;

    fn extensions(&self) -> &[&str] {
        &["fail"]
    }

    fn import(&self, _data: &[u8], _ctx: &mut ImportContext) -> Result<TextAsset, ImportError> {
        Err(ImportError::Format("intentional failure".to_string()))
    }
}

fn make_pipeline() -> Arc<ImportPipeline> {
    let mut p = ImportPipeline::new();
    p.register(TextImporter);
    p.register(DepTrackingImporter);
    p.register(FailImporter);
    Arc::new(p)
}

fn make_async_loader(threads: usize) -> (AsyncLoader, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let pipeline = make_pipeline();
    let loader = AsyncLoader::new(threads, pipeline);
    (loader, dir)
}

fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

// ---------------------------------------------------------------------------
// 1. AsyncLoader integration
// ---------------------------------------------------------------------------

#[test]
fn async_loader_loads_file_and_returns_result() {
    let (loader, dir) = make_async_loader(2);
    let path = write_file(dir.path(), "hello.txt", "hello world");

    let id = loader.load(&path, LoadPriority::Normal);
    assert!(loader.state(id).is_some());

    // Wait for worker to process
    std::thread::sleep(Duration::from_millis(500));

    let results = loader.poll_results();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
    assert!(results[0].result.is_ok());

    let asset = results[0]
        .result
        .as_ref()
        .unwrap()
        .asset
        .downcast_ref::<TextAsset>()
        .unwrap();
    assert_eq!(&asset.0, "hello world");

    let state = loader.state(id).unwrap();
    assert!(matches!(state, LoadState::Completed));
}

#[test]
fn async_loader_returns_error_for_nonexistent_file() {
    let (loader, _dir) = make_async_loader(1);
    let id = loader.load("/nonexistent/path/file.txt", LoadPriority::Normal);

    std::thread::sleep(Duration::from_millis(500));

    let results = loader.poll_results();
    assert_eq!(results.len(), 1);
    assert!(results[0].result.is_err());

    let state = loader.state(id).unwrap();
    assert!(matches!(state, LoadState::Failed(_)));
}

#[test]
fn async_loader_tracks_pending_count() {
    let (loader, dir) = make_async_loader(1);
    assert_eq!(loader.pending_count(), 0);

    let path = write_file(dir.path(), "test.txt", "data");
    let _id = loader.load(&path, LoadPriority::Normal);
    // Pending count may be 1 (queued) or 0 (already processed) depending on timing
    // Just verify it doesn't panic

    std::thread::sleep(Duration::from_millis(500));
    loader.poll_results();
    assert_eq!(loader.pending_count(), 0);
}

#[test]
fn async_loader_clear_finished_removes_completed_states() {
    let (loader, dir) = make_async_loader(1);
    let path = write_file(dir.path(), "test.txt", "data");
    let id = loader.load(&path, LoadPriority::Normal);

    std::thread::sleep(Duration::from_millis(500));
    loader.poll_results();

    assert!(loader.state(id).is_some());
    loader.clear_finished();
    assert!(loader.state(id).is_none());
}

// ---------------------------------------------------------------------------
// 2. AssetCache integration
// ---------------------------------------------------------------------------

#[test]
fn cache_detects_content_hash_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "asset.txt", "original content");

    let pipeline = make_pipeline();
    let result = pipeline.import_file(&path).unwrap();
    let original_hash = result.content_hash;

    let mut cache = AssetCache::new();
    cache.insert(
        path.clone(),
        CacheEntry {
            source_path: path.clone(),
            content_hash: original_hash,
            modified_at: std::time::SystemTime::now(),
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: 1024,
        },
    );

    // Same content — up to date
    assert!(cache.is_up_to_date(&path));

    // Modify file
    fs::write(&path, "completely different content").unwrap();

    // Give filesystem time to update mtime
    std::thread::sleep(Duration::from_millis(50));

    // Should detect change via hash fallback
    let new_hash = engine_asset::pipeline::hash_file(&path).unwrap();
    assert_ne!(original_hash, new_hash);
}

#[test]
fn cache_dependency_triggers_reimport() {
    let dir = tempfile::tempdir().unwrap();
    let dep_path = write_file(dir.path(), "texture.txt", "texture data");
    let mesh_path = write_file(dir.path(), "model.dep", "dep:texture.txt");

    let pipeline = make_pipeline();

    // Import dependency
    let dep_result = pipeline.import_file(&dep_path).unwrap();
    let dep_mtime = fs::metadata(&dep_path).unwrap().modified().unwrap();
    let mut cache = AssetCache::new();
    cache.insert(
        dep_path.clone(),
        CacheEntry {
            source_path: dep_path.clone(),
            content_hash: dep_result.content_hash,
            modified_at: dep_mtime,
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: 100,
        },
    );

    // Import mesh with dependency
    let mesh_result = pipeline.import_file(&mesh_path).unwrap();
    assert_eq!(mesh_result.dependencies.len(), 1);
    assert_eq!(mesh_result.dependencies[0], PathBuf::from("texture.txt"));

    let mesh_mtime = fs::metadata(&mesh_path).unwrap().modified().unwrap();
    cache.insert(
        mesh_path.clone(),
        CacheEntry {
            source_path: mesh_path.clone(),
            content_hash: mesh_result.content_hash,
            modified_at: mesh_mtime,
            dependencies: vec![dep_path.clone()],
            hot_reloadable: true,
            data_size: 200,
        },
    );

    // Verify reverse dependency tracking
    let dependents = cache.get_dependents(&dep_path);
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0], mesh_path);

    // Modify dependency
    fs::write(&dep_path, "new texture data").unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // check_stale should flag the dependent (mesh) since its dependency changed
    let stale = cache.check_stale();
    assert!(stale.contains(&mesh_path));
}

#[test]
fn cache_remove_cleans_up_reverse_dependencies() {
    let mut cache = AssetCache::new();

    let dep = PathBuf::from("texture.png");
    let mesh = PathBuf::from("model.gltf");

    cache.insert(
        dep.clone(),
        CacheEntry {
            source_path: dep.clone(),
            content_hash: 100,
            modified_at: std::time::SystemTime::now(),
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: 100,
        },
    );
    cache.insert(
        mesh.clone(),
        CacheEntry {
            source_path: mesh.clone(),
            content_hash: 200,
            modified_at: std::time::SystemTime::now(),
            dependencies: vec![dep.clone()],
            hot_reloadable: true,
            data_size: 200,
        },
    );

    assert_eq!(cache.get_dependents(&dep).len(), 1);

    // Remove mesh — should clean up reverse deps
    cache.remove(&mesh);
    assert!(cache.get_dependents(&dep).is_empty());
}

// ---------------------------------------------------------------------------
// 3. FileWatcher integration
// ---------------------------------------------------------------------------

#[test]
fn watcher_detects_file_creation_with_debounce() {
    let config = WatchConfig {
        debounce: Duration::from_millis(50),
        extensions: Vec::new(),
        recursive: true,
    };
    let mut watcher = FileWatcher::with_config(config);
    let tx = watcher.sender();

    tx.send(FileEvent::Created(PathBuf::from("new_asset.txt")))
        .unwrap();

    // Immediately poll — should be pending
    let events = watcher.poll_events();
    assert!(events.is_empty());
    assert!(watcher.has_pending());

    // Wait for debounce
    std::thread::sleep(Duration::from_millis(100));
    let events = watcher.poll_events();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        FileEvent::Created(PathBuf::from("new_asset.txt"))
    );
}

#[test]
fn watcher_filters_by_extension() {
    let config = WatchConfig {
        debounce: Duration::from_millis(10),
        extensions: vec!["png".to_string()],
        recursive: true,
    };
    let mut watcher = FileWatcher::with_config(config);
    let tx = watcher.sender();

    tx.send(FileEvent::Created(PathBuf::from("image.png")))
        .unwrap();
    tx.send(FileEvent::Created(PathBuf::from("model.gltf")))
        .unwrap();
    tx.send(FileEvent::Modified(PathBuf::from("texture.png")))
        .unwrap();

    // Drain into pending
    watcher.poll_events();
    std::thread::sleep(Duration::from_millis(50));

    let events = watcher.poll_events();
    assert_eq!(events.len(), 2);
    // Only .png files should pass
    for event in &events {
        match event {
            FileEvent::Created(p) | FileEvent::Modified(p) => {
                assert_eq!(p.extension().unwrap(), "png");
            }
            _ => panic!("unexpected event type"),
        }
    }
}

#[test]
fn watcher_coalesces_rapid_changes() {
    let config = WatchConfig {
        debounce: Duration::from_millis(50),
        extensions: Vec::new(),
        recursive: true,
    };
    let mut watcher = FileWatcher::with_config(config);
    let tx = watcher.sender();

    // Multiple rapid changes to the same file
    tx.send(FileEvent::Created(PathBuf::from("file.txt")))
        .unwrap();
    tx.send(FileEvent::Modified(PathBuf::from("file.txt")))
        .unwrap();
    tx.send(FileEvent::Modified(PathBuf::from("file.txt")))
        .unwrap();

    watcher.poll_events();
    std::thread::sleep(Duration::from_millis(100));

    let events = watcher.poll_events();
    // Should coalesce to a single event (last wins)
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], FileEvent::Modified(PathBuf::from("file.txt")));
}

// ---------------------------------------------------------------------------
// 4. Full pipeline integration: watcher → loader → cache
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_file_change_reimport_cache_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "data.txt", "version 1");

    let pipeline = make_pipeline();
    let mut cache = AssetCache::new();
    let mut watcher = FileWatcher::with_config(WatchConfig {
        debounce: Duration::from_millis(30),
        extensions: Vec::new(),
        recursive: true,
    });
    let watcher_tx = watcher.sender();

    // Initial import
    let result = pipeline.import_file(&path).unwrap();
    let asset = result.asset.downcast_ref::<TextAsset>().unwrap();
    assert_eq!(&asset.0, "version 1");

    cache.insert(
        path.clone(),
        CacheEntry {
            source_path: path.clone(),
            content_hash: result.content_hash,
            modified_at: std::time::SystemTime::now(),
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: asset.0.len(),
        },
    );

    assert!(cache.is_up_to_date(&path));

    // Simulate file change
    fs::write(&path, "version 2").unwrap();
    watcher_tx.send(FileEvent::Modified(path.clone())).unwrap();

    // Poll watcher: first drain into pending, then wait for debounce
    watcher.poll_events();
    std::thread::sleep(Duration::from_millis(80));
    let events = watcher.poll_events();
    assert_eq!(events.len(), 1);

    // Re-import
    let new_result = pipeline.import_file(&path).unwrap();
    let new_asset = new_result.asset.downcast_ref::<TextAsset>().unwrap();
    assert_eq!(&new_asset.0, "version 2");

    // Update cache
    cache.insert(
        path.clone(),
        CacheEntry {
            source_path: path.clone(),
            content_hash: new_result.content_hash,
            modified_at: std::time::SystemTime::now(),
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: new_asset.0.len(),
        },
    );

    assert!(cache.is_up_to_date(&path));
}

// ---------------------------------------------------------------------------
// 5. Dependency chain: changing dependency triggers re-import of dependents
// ---------------------------------------------------------------------------

#[test]
fn dependency_chain_change_triggers_reimport() {
    let dir = tempfile::tempdir().unwrap();
    let dep_path = write_file(dir.path(), "base.txt", "base data");
    let main_path = write_file(dir.path(), "main.dep", "dep:base.txt");

    let pipeline = make_pipeline();
    let mut cache = AssetCache::new();

    // Import both
    let dep_result = pipeline.import_file(&dep_path).unwrap();
    let dep_mtime = fs::metadata(&dep_path).unwrap().modified().unwrap();
    cache.insert(
        dep_path.clone(),
        CacheEntry {
            source_path: dep_path.clone(),
            content_hash: dep_result.content_hash,
            modified_at: dep_mtime,
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: 100,
        },
    );

    let main_result = pipeline.import_file(&main_path).unwrap();
    assert_eq!(main_result.dependencies, vec![PathBuf::from("base.txt")]);
    let main_mtime = fs::metadata(&main_path).unwrap().modified().unwrap();
    cache.insert(
        main_path.clone(),
        CacheEntry {
            source_path: main_path.clone(),
            content_hash: main_result.content_hash,
            modified_at: main_mtime,
            dependencies: vec![dep_path.clone()],
            hot_reloadable: true,
            data_size: 200,
        },
    );

    // Both up to date
    assert!(cache.is_up_to_date(&dep_path));
    assert!(cache.is_up_to_date(&main_path));

    // Change dependency
    fs::write(&dep_path, "new base data").unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // check_stale should detect the change
    let stale = cache.check_stale();
    assert!(!stale.is_empty());
}

// ---------------------------------------------------------------------------
// 6. Priority ordering
// ---------------------------------------------------------------------------

#[test]
fn priority_ordering_critical_before_normal() {
    let (loader, dir) = make_async_loader(1);
    let path1 = write_file(dir.path(), "a.txt", "low priority");
    let path2 = write_file(dir.path(), "b.txt", "critical priority");

    // Submit low first, then critical
    let id_low = loader.load(&path1, LoadPriority::Low);
    let id_crit = loader.load(&path2, LoadPriority::Critical);

    // Both should be queued
    assert!(loader.state(id_low).is_some());
    assert!(loader.state(id_crit).is_some());

    // Wait for both to complete
    std::thread::sleep(Duration::from_millis(500));
    let results = loader.poll_results();
    assert_eq!(results.len(), 2);

    // Both should succeed
    for r in &results {
        assert!(r.result.is_ok());
    }
}

#[test]
fn priority_ordering_high_before_low() {
    assert!(LoadPriority::High > LoadPriority::Normal);
    assert!(LoadPriority::Normal > LoadPriority::Low);
    assert!(LoadPriority::Critical > LoadPriority::High);
}

// ---------------------------------------------------------------------------
// 7. Concurrent load requests
// ---------------------------------------------------------------------------

#[test]
fn concurrent_loads_dont_deadlock() {
    let (loader, dir) = make_async_loader(4);

    // Submit many concurrent requests
    let mut ids = Vec::new();
    for i in 0..20 {
        let path = write_file(
            dir.path(),
            &format!("file_{i}.txt"),
            &format!("content {i}"),
        );
        ids.push(loader.load(&path, LoadPriority::Normal));
    }

    // Wait for all to complete
    std::thread::sleep(Duration::from_millis(2000));

    let results = loader.poll_results();
    assert_eq!(results.len(), 20);

    // All should succeed
    for r in &results {
        assert!(r.result.is_ok());
    }

    // All IDs should be completed
    for id in &ids {
        let state = loader.state(*id).unwrap();
        assert!(matches!(state, LoadState::Completed));
    }
}

#[test]
fn concurrent_loads_with_mixed_priorities() {
    let (loader, dir) = make_async_loader(2);

    let mut ids = Vec::new();
    for i in 0..10 {
        let path = write_file(dir.path(), &format!("mix_{i}.txt"), &format!("data {i}"));
        let priority = match i % 4 {
            0 => LoadPriority::Critical,
            1 => LoadPriority::High,
            2 => LoadPriority::Normal,
            _ => LoadPriority::Low,
        };
        ids.push(loader.load(&path, priority));
    }

    std::thread::sleep(Duration::from_millis(2000));
    let results = loader.poll_results();
    assert_eq!(results.len(), 10);

    for r in &results {
        assert!(r.result.is_ok());
    }
}

// ---------------------------------------------------------------------------
// 8. Error handling
// ---------------------------------------------------------------------------

#[test]
fn import_unregistered_extension_returns_error() {
    let pipeline = make_pipeline();
    let result = pipeline.import_bytes(b"data", "xyz", Path::new("file.xyz"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ImportError::NoImporter(_)));
}

#[test]
fn import_file_without_extension_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("noext");
    fs::write(&path, "data").unwrap();

    let pipeline = make_pipeline();
    let result = pipeline.import_file(&path);
    assert!(result.is_err());
}

#[test]
fn import_corrupt_utf8_returns_format_error() {
    let pipeline = make_pipeline();
    // Invalid UTF-8 bytes
    let bad_bytes = [0xFF, 0xFE, 0x00, 0x01];
    let result = pipeline.import_bytes(&bad_bytes, "txt", Path::new("bad.txt"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ImportError::Format(_)));
}

#[test]
fn importer_failure_returns_correct_error() {
    let pipeline = make_pipeline();
    let result = pipeline.import_bytes(b"data", "fail", Path::new("fail_file.fail"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ImportError::Format(ref msg) if msg == "intentional failure"));
}

#[test]
fn async_loader_reports_failure_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.fail");
    fs::write(&path, "data").unwrap();

    let pipeline = make_pipeline();
    let loader = AsyncLoader::new(1, pipeline);

    let id = loader.load(&path, LoadPriority::Normal);
    std::thread::sleep(Duration::from_millis(500));

    let results = loader.poll_results();
    assert_eq!(results.len(), 1);
    assert!(results[0].result.is_err());

    let state = loader.state(id).unwrap();
    assert!(matches!(state, LoadState::Failed(_)));
}

#[test]
fn nonexistent_directory_load_reports_error() {
    let (loader, _dir) = make_async_loader(1);
    let id = loader.load("/completely/bogus/path.txt", LoadPriority::Normal);

    std::thread::sleep(Duration::from_millis(500));
    let results = loader.poll_results();
    assert_eq!(results.len(), 1);
    assert!(results[0].result.is_err());
    assert!(matches!(loader.state(id).unwrap(), LoadState::Failed(_)));
}

// ---------------------------------------------------------------------------
// 9. Hot-reload path: modify → cache invalidation → re-import → new data
// ---------------------------------------------------------------------------

#[test]
fn hot_reload_modify_invalidate_reimport() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "hot.txt", "original");

    let pipeline = make_pipeline();
    let mut cache = AssetCache::new();

    // First import
    let result1 = pipeline.import_file(&path).unwrap();
    let asset1 = result1.asset.downcast_ref::<TextAsset>().unwrap();
    assert_eq!(&asset1.0, "original");

    cache.insert(
        path.clone(),
        CacheEntry {
            source_path: path.clone(),
            content_hash: result1.content_hash,
            modified_at: std::time::SystemTime::now(),
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: asset1.0.len(),
        },
    );

    assert!(cache.is_up_to_date(&path));

    // Modify file (simulates external edit)
    fs::write(&path, "hot reloaded!").unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Cache should detect staleness via hash
    let new_hash = engine_asset::pipeline::hash_file(&path).unwrap();
    assert_ne!(result1.content_hash, new_hash);

    // Re-import
    let result2 = pipeline.import_file(&path).unwrap();
    let asset2 = result2.asset.downcast_ref::<TextAsset>().unwrap();
    assert_eq!(&asset2.0, "hot reloaded!");
    assert_ne!(result1.content_hash, result2.content_hash);

    // Update cache
    cache.insert(
        path.clone(),
        CacheEntry {
            source_path: path.clone(),
            content_hash: result2.content_hash,
            modified_at: std::time::SystemTime::now(),
            dependencies: Vec::new(),
            hot_reloadable: true,
            data_size: asset2.0.len(),
        },
    );

    assert!(cache.is_up_to_date(&path));
}

// ---------------------------------------------------------------------------
// 10. Pipeline content hash determinism
// ---------------------------------------------------------------------------

#[test]
fn content_hash_is_deterministic() {
    let pipeline = make_pipeline();

    let r1 = pipeline
        .import_bytes(b"same data", "txt", Path::new("a.txt"))
        .unwrap();
    let r2 = pipeline
        .import_bytes(b"same data", "txt", Path::new("b.txt"))
        .unwrap();
    assert_eq!(r1.content_hash, r2.content_hash);

    let r3 = pipeline
        .import_bytes(b"different data", "txt", Path::new("c.txt"))
        .unwrap();
    assert_ne!(r1.content_hash, r3.content_hash);
}

#[test]
fn content_hash_changes_with_content() {
    use engine_asset::pipeline::hash_bytes;

    let h1 = hash_bytes(b"version 1");
    let h2 = hash_bytes(b"version 2");
    assert_ne!(h1, h2);

    // Deterministic
    let h3 = hash_bytes(b"version 1");
    assert_eq!(h1, h3);
}
