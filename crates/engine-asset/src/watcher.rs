//! File system watcher with debounced change events.
//!
//! Provides [`FileWatcher`] for monitoring asset directories and
//! emitting debounced change events suitable for hot-reload pipelines.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossbeam_channel::{self as channel, Receiver, Sender};

/// A file system change event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// A file was created.
    Created(PathBuf),
    /// A file was modified.
    Modified(PathBuf),
    /// A file was removed.
    Removed(PathBuf),
    /// A file was renamed (from, to).
    Renamed(PathBuf, PathBuf),
}

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Debounce interval — events within this window are coalesced.
    pub debounce: Duration,
    /// File extensions to watch (empty = all files).
    pub extensions: Vec<String>,
    /// Whether to watch subdirectories recursively.
    pub recursive: bool,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(300),
            extensions: Vec::new(),
            recursive: true,
        }
    }
}

/// Debounced file system watcher.
///
/// Monitors configured directories and coalesces rapid file changes
/// into single events, preventing re-import storms during saves.
pub struct FileWatcher {
    /// Watched root directories.
    watched: Vec<PathBuf>,
    /// Configuration.
    config: WatchConfig,
    /// Pending events being debounced.
    pending: HashMap<PathBuf, (FileEvent, Instant)>,
    /// Channel receiver for raw filesystem events.
    rx: Receiver<FileEvent>,
    /// Channel sender (cloned into the OS watcher thread).
    tx: Sender<FileEvent>,
}

impl FileWatcher {
    /// Create a new file watcher with default configuration.
    pub fn new() -> Self {
        Self::with_config(WatchConfig::default())
    }

    /// Create a new file watcher with custom configuration.
    pub fn with_config(config: WatchConfig) -> Self {
        let (tx, rx) = channel::unbounded();
        Self {
            watched: Vec::new(),
            config,
            pending: HashMap::new(),
            rx,
            tx,
        }
    }

    /// Get a sender for injecting events (used by the OS watcher thread).
    pub fn sender(&self) -> Sender<FileEvent> {
        self.tx.clone()
    }

    /// Add a directory to watch. Must be called before `start`.
    pub fn watch(&mut self, path: impl Into<PathBuf>) {
        self.watched.push(path.into());
    }

    /// Get the list of watched directories.
    pub fn watched_dirs(&self) -> &[PathBuf] {
        &self.watched
    }

    /// Get the debounce configuration.
    pub fn config(&self) -> &WatchConfig {
        &self.config
    }

    /// Poll for debounced file change events.
    ///
    /// Call this regularly (e.g., each frame). Returns events that have
    /// settled past the debounce window.
    pub fn poll_events(&mut self) -> Vec<FileEvent> {
        // Drain raw events from the channel
        while let Ok(event) = self.rx.try_recv() {
            let path = match &event {
                FileEvent::Created(p) | FileEvent::Modified(p) | FileEvent::Removed(p) => p.clone(),
                FileEvent::Renamed(_, to) => to.clone(),
            };

            // Filter by extension
            if !self.config.extensions.is_empty() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if !self.config.extensions.iter().any(|e| e == ext) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Coalesce: last event for a path wins
            self.pending.insert(path, (event, Instant::now()));
        }

        // Collect events past the debounce window
        let now = Instant::now();
        let debounce = self.config.debounce;

        let ready: Vec<FileEvent> = self
            .pending
            .extract_if(|_, (_, time)| now.duration_since(*time) >= debounce)
            .map(|(_, (event, _))| event)
            .collect();

        ready
    }

    /// Check if there are pending (not yet debounced) events.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Clear all pending events without processing them.
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a file watcher with an OS-level notify backend.
///
/// This spawns a background thread that uses the `notify` crate to
/// watch directories and forward events through the channel.
///
/// Returns the FileWatcher and a handle to the background thread.
/// Drop the handle to stop the watcher thread.
pub fn create_notify_watcher(
    config: WatchConfig,
    dirs: &[PathBuf],
) -> Result<(FileWatcher, notify::RecommendedWatcher), WatcherError> {
    use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

    let mut watcher = FileWatcher::with_config(config.clone());
    let tx = watcher.sender();

    let mut os_watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                let file_events = convert_notify_event(&event);
                for fe in file_events {
                    let _ = tx.send(fe);
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| WatcherError::NotifyError(e.to_string()))?;

    let recursive = if config.recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };

    for dir in dirs {
        os_watcher
            .watch(dir, recursive)
            .map_err(|e| WatcherError::WatchFailed {
                path: dir.display().to_string(),
                message: e.to_string(),
            })?;
        watcher.watch(dir.clone());
    }

    Ok((watcher, os_watcher))
}

/// Convert a notify event into our FileEvent types.
fn convert_notify_event(event: &notify::Event) -> Vec<FileEvent> {
    use notify::EventKind;

    match event.kind {
        EventKind::Create(_) => event
            .paths
            .iter()
            .map(|p| FileEvent::Created(p.clone()))
            .collect(),
        EventKind::Modify(_) => event
            .paths
            .iter()
            .map(|p| FileEvent::Modified(p.clone()))
            .collect(),
        EventKind::Remove(_) => event
            .paths
            .iter()
            .map(|p| FileEvent::Removed(p.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Errors from the file watcher.
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("Notify error: {0}")]
    NotifyError(String),
    #[error("Failed to watch '{path}': {message}")]
    WatchFailed { path: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.watched_dirs().is_empty());
        assert!(!watcher.has_pending());
    }

    #[test]
    fn test_watcher_with_config() {
        let config = WatchConfig {
            debounce: Duration::from_millis(100),
            extensions: vec!["png".to_string()],
            recursive: true,
        };
        let watcher = FileWatcher::with_config(config);
        assert_eq!(watcher.config().debounce, Duration::from_millis(100));
    }

    #[test]
    fn test_watcher_watch_dir() {
        let mut watcher = FileWatcher::new();
        watcher.watch("/some/path");
        assert_eq!(watcher.watched_dirs().len(), 1);
    }

    #[test]
    fn test_poll_empty() {
        let mut watcher = FileWatcher::new();
        let events = watcher.poll_events();
        assert!(events.is_empty());
    }

    #[test]
    fn test_inject_and_poll_debounced() {
        let config = WatchConfig {
            debounce: Duration::from_millis(50),
            extensions: Vec::new(),
            recursive: true,
        };
        let mut watcher = FileWatcher::with_config(config);
        let tx = watcher.sender();

        // Inject an event
        tx.send(FileEvent::Created(PathBuf::from("test.txt")))
            .unwrap();

        // Immediately poll — should be pending (not yet debounced)
        let events = watcher.poll_events();
        assert!(events.is_empty());
        assert!(watcher.has_pending());

        // Wait for debounce
        std::thread::sleep(Duration::from_millis(80));
        let events = watcher.poll_events();
        assert_eq!(events.len(), 1);
        assert!(!watcher.has_pending());
    }

    #[test]
    fn test_extension_filtering() {
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

        // First poll to drain channel into pending
        watcher.poll_events();
        // Wait for debounce
        std::thread::sleep(Duration::from_millis(30));
        let events = watcher.poll_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], FileEvent::Created(PathBuf::from("image.png")));
    }

    #[test]
    fn test_coalesce_last_wins() {
        let config = WatchConfig {
            debounce: Duration::from_millis(50),
            extensions: Vec::new(),
            recursive: true,
        };
        let mut watcher = FileWatcher::with_config(config);
        let tx = watcher.sender();

        tx.send(FileEvent::Created(PathBuf::from("f.txt"))).unwrap();
        tx.send(FileEvent::Modified(PathBuf::from("f.txt")))
            .unwrap();

        // First poll to drain channel into pending
        watcher.poll_events();
        // Wait for debounce
        std::thread::sleep(Duration::from_millis(80));
        let events = watcher.poll_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], FileEvent::Modified(PathBuf::from("f.txt")));
    }

    #[test]
    fn test_clear_pending() {
        let config = WatchConfig {
            debounce: Duration::from_millis(50),
            extensions: Vec::new(),
            recursive: true,
        };
        let mut watcher = FileWatcher::with_config(config);
        let tx = watcher.sender();

        tx.send(FileEvent::Created(PathBuf::from("a.txt"))).unwrap();

        // First poll to drain channel into pending
        watcher.poll_events();
        assert!(watcher.has_pending());

        watcher.clear_pending();
        assert!(!watcher.has_pending());
    }
}
