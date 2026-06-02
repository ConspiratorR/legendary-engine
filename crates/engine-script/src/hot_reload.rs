use crate::bridge::ComponentBridge;
use crate::system::ScriptSystem;
use mlua::prelude::*;
use notify_debouncer_mini::new_debouncer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Manages hot-reloading of Lua script files.
///
/// Watches a directory for `.lua` file changes and automatically
/// reloads the corresponding [`ScriptSystem`] instances.
pub struct HotReloader {
    /// Map from file path → script system name
    scripts: HashMap<PathBuf, String>,
    /// The component bridge shared across all script systems
    #[allow(dead_code)]
    bridge: Arc<RwLock<ComponentBridge>>,
    /// Debouncer for file system events
    _debouncer: Option<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>>,
    /// Pending reloads (file paths that changed)
    pending_reloads: Arc<RwLock<Vec<PathBuf>>>,
}

impl HotReloader {
    /// Create a new hot-reloader.
    ///
    /// `bridge` is the shared component registry for creating new script systems.
    pub fn new(bridge: Arc<RwLock<ComponentBridge>>) -> Self {
        Self {
            scripts: HashMap::new(),
            bridge,
            _debouncer: None,
            pending_reloads: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a Lua script file for hot-reloading.
    ///
    /// `path` is the file to watch. `name` is used to identify the script system.
    pub fn watch(&mut self, path: impl AsRef<Path>, name: impl Into<String>) -> LuaResult<()> {
        let path = path.as_ref().to_path_buf();
        self.scripts.insert(path, name.into());
        Ok(())
    }

    /// Start watching a directory for `.lua` file changes.
    ///
    /// When a `.lua` file is modified, it will be flagged for reload
    /// on the next [`check_reloads`] call.
    pub fn start_watching(&mut self, dir: impl AsRef<Path>) -> notify::Result<()> {
        let pending = self.pending_reloads.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            move |events: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                if let Ok(events) = events {
                    for event in events {
                        let path = &event.path;
                        if path.extension().is_some_and(|ext| ext == "lua")
                            && let Ok(mut pending) = pending.write()
                        {
                            pending.push(path.clone());
                        }
                    }
                }
            },
        )?;

        debouncer
            .watcher()
            .watch(dir.as_ref(), notify::RecursiveMode::Recursive)?;

        self._debouncer = Some(debouncer);
        Ok(())
    }

    /// Check for pending reloads and return the script names that need reloading.
    ///
    /// Call this each frame before running systems. For each returned name,
    /// call [`reload_script`] to apply the changes.
    pub fn check_reloads(&self) -> Vec<String> {
        let mut result = Vec::new();
        if let Ok(mut pending) = self.pending_reloads.write() {
            for path in pending.drain(..) {
                if let Some(name) = self.scripts.get(&path) {
                    result.push(name.clone());
                }
            }
        }
        result
    }

    /// Reload a specific script system from its file.
    ///
    /// `systems` is a mutable map of name → ScriptSystem.
    pub fn reload_script(
        &self,
        name: &str,
        systems: &mut HashMap<String, ScriptSystem>,
    ) -> LuaResult<()> {
        // Find the file path for this script name
        let path = self
            .scripts
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(p, _)| p.clone());

        if let Some(path) = path
            && let Some(system) = systems.get_mut(name)
        {
            println!(
                "[HotReload] Reloading script: {} from {}",
                name,
                path.display()
            );
            system.reload_from_file(&path)?;
        }
        Ok(())
    }

    /// Get the file path for a registered script name.
    pub fn script_path(&self, name: &str) -> Option<&Path> {
        self.scripts
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(p, _)| p.as_path())
    }

    /// Get all registered script names.
    pub fn script_names(&self) -> Vec<&str> {
        self.scripts.values().map(|s| s.as_str()).collect()
    }
}
