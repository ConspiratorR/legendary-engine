//! Unified asset management system.
//!
//! [`AssetManager`] integrates the import pipeline, cache, file watcher,
//! and async loader into a single interface for the rest of the engine.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::asset::{Asset, Handle};
use crate::cache::{AssetCache, CacheEntry};
use crate::format::importers::{
    AudioImporter, GltfImporter, ImageImporter, MaterialImporter, ScriptImporter,
};
use crate::pipeline::{ImportError, ImportPipeline, hash_file};
use crate::registry::Registry;
use crate::watcher::{FileEvent, FileWatcher, WatchConfig};

/// Events emitted by the asset manager.
#[derive(Debug, Clone)]
pub enum AssetEvent {
    /// A new asset was loaded.
    Loaded { path: PathBuf },
    /// An existing asset was hot-reloaded.
    Reloaded { path: PathBuf },
    /// An asset was removed.
    Removed { path: PathBuf },
    /// An import error occurred.
    ImportError { path: PathBuf, error: String },
}

/// Unified asset management system.
///
/// Combines import pipeline, asset registry, content cache, and file
/// watcher into a single update-driven system. Call [`update`](Self::update)
/// each frame to process file changes and emit hot-reload events.
pub struct AssetManager {
    /// The import pipeline with registered importers.
    pipeline: ImportPipeline,
    /// The asset registry storing loaded assets.
    registry: Registry,
    /// Content-hash cache for invalidation.
    cache: AssetCache,
    /// File system watcher for hot-reload.
    watcher: FileWatcher,
    /// Events from the last update.
    events: Vec<AssetEvent>,
}

impl AssetManager {
    /// Create a new asset manager with default configuration.
    pub fn new() -> Self {
        Self {
            pipeline: ImportPipeline::new(),
            registry: Registry::new(),
            cache: AssetCache::new(),
            watcher: FileWatcher::new(),
            events: Vec::new(),
        }
    }

    /// Create with a custom watch configuration.
    pub fn with_watch_config(config: WatchConfig) -> Self {
        Self {
            pipeline: ImportPipeline::new(),
            registry: Registry::new(),
            cache: AssetCache::new(),
            watcher: FileWatcher::with_config(config),
            events: Vec::new(),
        }
    }

    /// Create an asset manager with all default importers registered.
    ///
    /// Registers importers for: images (png/jpg/bmp/tga/hdr), glTF/GLB,
    /// audio (wav/ogg/mp3/flac), materials (.mat), and scripts (lua/rs/py).
    pub fn with_defaults() -> Self {
        let mut manager = Self::new();
        manager.register_default_importers();
        manager
    }

    /// Register all built-in importers.
    ///
    /// - [`ImageImporter`] — PNG, JPG, BMP, TGA, HDR → [`Texture`](crate::types::Texture)
    /// - [`GltfImporter`] — glTF, GLB → [`MeshCollection`](crate::format::importers::MeshCollection)
    /// - [`AudioImporter`] — WAV, OGG, MP3, FLAC → [`AudioClip`](crate::types::AudioClip)
    /// - [`MaterialImporter`] — .mat → [`Material`](crate::types::Material)
    /// - [`ScriptImporter`] — Lua, Rust, Python → [`Script`](crate::types::Script)
    pub fn register_default_importers(&mut self) {
        self.pipeline.register(ImageImporter);
        self.pipeline.register(GltfImporter);
        self.pipeline.register(AudioImporter);
        self.pipeline.register(MaterialImporter);
        self.pipeline.register(ScriptImporter);
    }

    /// Register an asset importer.
    pub fn register_importer<I: crate::pipeline::AssetImporter + 'static>(&mut self, importer: I) {
        self.pipeline.register(importer);
    }

    /// Add a directory to watch for changes.
    pub fn watch(&mut self, path: impl Into<PathBuf>) {
        self.watcher.watch(path);
    }

    /// Get a reference to the import pipeline.
    pub fn pipeline(&self) -> &ImportPipeline {
        &self.pipeline
    }

    /// Get a mutable reference to the import pipeline.
    pub fn pipeline_mut(&mut self) -> &mut ImportPipeline {
        &mut self.pipeline
    }

    /// Get a reference to the asset registry.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Get a mutable reference to the asset registry.
    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }

    /// Get a reference to the asset cache.
    pub fn cache(&self) -> &AssetCache {
        &self.cache
    }

    /// Get a reference to the file watcher.
    pub fn watcher(&self) -> &FileWatcher {
        &self.watcher
    }

    /// Get events from the last [`update`](Self::update) call.
    pub fn events(&self) -> &[AssetEvent] {
        &self.events
    }

    /// Import an asset from a file path, using the cache to skip
    /// re-import if the file hasn't changed.
    ///
    /// Returns a handle to the imported asset.
    pub fn import<T: Asset + Send + Sync + Clone + 'static>(
        &mut self,
        path: &Path,
    ) -> Result<Handle<T>, ImportError> {
        // Check cache
        if self.cache.is_up_to_date(path)
            && let Some(asset) = self.registry.get::<T>(&path.to_string_lossy())
        {
            // Already loaded and up-to-date
            return Ok(Handle::new(asset.clone()));
        }

        // Import fresh
        let result = self.pipeline.import_file(path)?;

        // Update cache
        let modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        let data_size = result
            .asset
            .downcast_ref::<T>()
            .map(|a| std::mem::size_of_val(a))
            .unwrap_or(0);

        self.cache.insert(
            path.to_path_buf(),
            CacheEntry {
                source_path: path.to_path_buf(),
                content_hash: result.content_hash,
                modified_at: modified,
                dependencies: result.dependencies.clone(),
                hot_reloadable: self
                    .pipeline
                    .supports_hot_reload(path.extension().and_then(|e| e.to_str()).unwrap_or("")),
                data_size,
            },
        );

        // Store in registry
        let asset = result
            .asset
            .downcast::<T>()
            .map_err(|_| ImportError::Format("Type mismatch after import".to_string()))?;

        let key = path.to_string_lossy().to_string();
        let handle = self.registry.store(&key, *asset);
        self.events.push(AssetEvent::Loaded {
            path: path.to_path_buf(),
        });

        Ok(handle)
    }

    /// Process file watcher events and hot-reload changed assets.
    ///
    /// Call this once per frame (or at a regular interval).
    pub fn update(&mut self) -> &[AssetEvent] {
        self.events.clear();

        let file_events = self.watcher.poll_events();

        for event in file_events {
            match event {
                FileEvent::Created(path) | FileEvent::Modified(path) => {
                    self.handle_file_change(&path);
                }
                FileEvent::Removed(path) => {
                    self.handle_file_removed(&path);
                }
                FileEvent::Renamed(from, to) => {
                    self.handle_file_removed(&from);
                    self.handle_file_change(&to);
                }
            }
        }

        // Also check for stale dependencies
        let stale = self.cache.check_stale();
        for path in stale {
            self.handle_file_change(&path);
        }

        &self.events
    }

    /// Handle a file change (created or modified).
    fn handle_file_change(&mut self, path: &Path) {
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_string(),
            None => return,
        };

        // Only reimport if the importer supports hot-reload
        if !self.pipeline.supports_hot_reload(&ext) {
            return;
        }

        // Check if content actually changed
        if let Some(entry) = self.cache.get(path)
            && let Ok(hash) = hash_file(path)
            && hash == entry.content_hash
        {
            return; // Content unchanged, skip
        }

        // Re-import
        match self.pipeline.import_file(path) {
            Ok(result) => {
                let modified = std::fs::metadata(path)
                    .and_then(|m| m.modified())
                    .unwrap_or_else(|_| SystemTime::now());

                self.cache.insert(
                    path.to_path_buf(),
                    CacheEntry {
                        source_path: path.to_path_buf(),
                        content_hash: result.content_hash,
                        modified_at: modified,
                        dependencies: result.dependencies,
                        hot_reloadable: true,
                        data_size: 0,
                    },
                );

                // Update the registry with the reloaded asset data
                let key = path.to_string_lossy().to_string();
                self.registry.replace(&key, result.asset);

                self.events.push(AssetEvent::Reloaded {
                    path: path.to_path_buf(),
                });
            }
            Err(e) => {
                self.events.push(AssetEvent::ImportError {
                    path: path.to_path_buf(),
                    error: e.to_string(),
                });
            }
        }
    }

    /// Handle a file removal.
    fn handle_file_removed(&mut self, path: &Path) {
        self.cache.remove(path);
        let key = path.to_string_lossy().to_string();
        self.registry.remove(&key);
        self.events.push(AssetEvent::Removed {
            path: path.to_path_buf(),
        });
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{AssetImporter, ImportContext};

    #[derive(Debug, Clone)]
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
            let s =
                String::from_utf8(data.to_vec()).map_err(|e| ImportError::Format(e.to_string()))?;
            Ok(TextAsset(s))
        }
    }

    fn make_manager() -> AssetManager {
        let mut manager = AssetManager::new();
        manager.register_importer(TextImporter);
        manager
    }

    #[test]
    fn test_asset_manager_import() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello world").unwrap();

        let mut manager = make_manager();
        let handle = manager.import::<TextAsset>(&file).unwrap();
        assert_eq!(handle.get().0, "hello world");
    }

    #[test]
    fn test_asset_manager_cache_hit() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "cached content").unwrap();

        let mut manager = make_manager();

        // First import
        let h1 = manager.import::<TextAsset>(&file).unwrap();
        assert_eq!(h1.get().0, "cached content");

        // Second import should hit cache
        let h2 = manager.import::<TextAsset>(&file).unwrap();
        assert_eq!(h2.get().0, "cached content");
    }

    #[test]
    fn test_asset_manager_update_with_watcher() {
        let dir = tempfile::tempdir().unwrap();

        let mut manager = make_manager();
        manager.watch(dir.path());

        // Create a file
        let file = dir.path().join("dynamic.txt");
        std::fs::write(&file, "new content").unwrap();

        // Inject a Created event via the channel
        let tx = manager.watcher.sender();
        tx.send(FileEvent::Created(file.clone())).unwrap();

        // First update: drains channel into pending, events are within debounce window
        let events = manager.update();
        assert!(
            events.is_empty(),
            "First update should be empty (debouncing)"
        );
        assert!(manager.watcher().has_pending());

        // Wait for debounce to settle
        std::thread::sleep(std::time::Duration::from_millis(400));

        // Second update: pending events are now past debounce window
        let events = manager.update();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssetEvent::Reloaded { path } if path == &file)),
            "Expected Reloaded event, got: {:?}",
            events
        );
    }

    #[test]
    fn test_asset_manager_unsupported_extension() {
        let manager = make_manager();
        let result = manager
            .pipeline()
            .import_bytes(b"data", "xyz", Path::new("f.xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn test_with_defaults_registers_all_importers() {
        let manager = AssetManager::with_defaults();
        let pipeline = manager.pipeline();

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
    }

    #[test]
    fn test_hot_reload_updates_registry() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "version 1").unwrap();

        let mut manager = make_manager();

        // Import initial version
        let handle = manager.import::<TextAsset>(&file).unwrap();
        assert_eq!(handle.get().0, "version 1");

        // Change the file
        std::fs::write(&file, "version 2").unwrap();

        // Wait for debounce and file system sync
        std::thread::sleep(std::time::Duration::from_millis(1000));
        manager.update();

        // Registry should have the updated content
        let key = file.to_string_lossy().to_string();
        let updated = manager.registry().get::<TextAsset>(&key);
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().0, "version 2");
    }
}
