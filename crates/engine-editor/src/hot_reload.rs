//! Hot reload system for assets and shaders.
//!
//! Watches file system changes and triggers recompilation/reload
//! of modified assets, shaders, and scripts.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use engine_asset::types::ResourceType;
use log::{info, warn};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebouncedEvent, Debouncer, new_debouncer};

/// A pending reload request for a changed file.
#[derive(Debug, Clone)]
pub struct ReloadRequest {
    pub path: PathBuf,
    pub resource_type: ResourceType,
    pub timestamp: Instant,
}

/// Watches directories for file changes using a debounced file system watcher.
pub struct FileWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    receiver: Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
    watched_paths: Vec<PathBuf>,
    pending_reload: Vec<ReloadRequest>,
}

impl FileWatcher {
    /// Creates a new `FileWatcher` with a 500ms debounce delay.
    pub fn new() -> Result<Self, String> {
        let (tx, receiver) = std::sync::mpsc::channel();
        let debouncer = new_debouncer(std::time::Duration::from_millis(500), tx)
            .map_err(|e| format!("Failed to create file debouncer: {e}"))?;

        Ok(Self {
            _debouncer: debouncer,
            receiver,
            watched_paths: Vec::new(),
            pending_reload: Vec::new(),
        })
    }

    /// Watches a directory recursively for file changes.
    pub fn watch(&mut self, path: &Path) -> Result<(), String> {
        self._debouncer
            .watcher()
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path {}: {e}", path.display()))?;
        self.watched_paths.push(path.to_path_buf());
        Ok(())
    }

    /// Polls for file change events and queues reload requests.
    pub fn poll(&mut self) {
        while let Ok(Ok(events)) = self.receiver.try_recv() {
            for event in events {
                if let Some(resource_type) = Self::detect_resource_type(&event.path) {
                    self.pending_reload.push(ReloadRequest {
                        path: event.path,
                        resource_type,
                        timestamp: Instant::now(),
                    });
                }
            }
        }
    }

    /// Drains and returns all pending reload requests.
    pub fn take_pending(&mut self) -> Vec<ReloadRequest> {
        std::mem::take(&mut self.pending_reload)
    }

    /// Maps a file extension to a `ResourceType`.
    pub fn detect_resource_type(path: &Path) -> Option<ResourceType> {
        let ext = path.extension()?.to_str()?;
        Some(ResourceType::from_extension(ext))
    }
}

/// Manages hot reload lifecycle: watching, polling, and logging.
pub struct ReloadManager {
    file_watcher: FileWatcher,
    reload_log: Vec<String>,
    start_time: Instant,
}

impl ReloadManager {
    /// Creates a new `ReloadManager` that watches the given path.
    pub fn new(watch_path: &Path) -> Result<Self, String> {
        let mut file_watcher = FileWatcher::new()?;
        file_watcher.watch(watch_path)?;
        info!("Hot reload watching: {}", watch_path.display());

        Ok(Self {
            file_watcher,
            reload_log: Vec::new(),
            start_time: Instant::now(),
        })
    }

    /// Polls for changes and logs any reload events.
    pub fn update(&mut self) {
        self.file_watcher.poll();
        let requests = self.file_watcher.take_pending();

        for req in &requests {
            let elapsed = self.start_time.elapsed();
            let hours = elapsed.as_secs() / 3600;
            let minutes = (elapsed.as_secs() % 3600) / 60;
            let seconds = elapsed.as_secs() % 60;
            let ts = format!("{hours:02}:{minutes:02}:{seconds:02}");
            let entry = format!(
                "[{ts}] Reload {:?}: {}",
                req.resource_type,
                req.path.display()
            );
            info!("{}", entry);
            self.reload_log.push(entry);
        }

        if !requests.is_empty() {
            warn!("Queued {} resource reload(s)", requests.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_resource_type() {
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("texture.png")),
            Some(ResourceType::Texture)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("sound.wav")),
            Some(ResourceType::Audio)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("model.gltf")),
            Some(ResourceType::Mesh)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("mat.mat")),
            Some(ResourceType::Material)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("script.lua")),
            Some(ResourceType::Script)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("scene.scene")),
            Some(ResourceType::Scene)
        ));
        assert!(FileWatcher::detect_resource_type(Path::new("noext")).is_none());
        assert!(FileWatcher::detect_resource_type(Path::new("unknown.xyz")).is_some());
    }

    #[test]
    fn test_file_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }
}
