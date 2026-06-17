use crate::app::AppBuilder;
use crate::plugin::Plugin;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Plugin manifest loaded from a `plugin.json` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (unique identifier).
    pub name: String,
    /// Semantic version string (e.g. "1.0.0").
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Plugin author.
    pub author: String,
    /// Entry point function name (must return `Box<dyn Plugin>`).
    pub entry_point: String,
    /// Required engine version (semver range).
    pub engine_version: String,
    /// Plugin dependencies (name → version range).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

/// A loaded dynamic plugin with its manifest and library handle.
pub struct DynamicPlugin {
    pub manifest: PluginManifest,
    _lib: libloading::Library,
    plugin: Box<dyn Plugin>,
}

impl DynamicPlugin {
    /// Load a plugin from a directory containing `plugin.json` and a shared library.
    ///
    /// # Safety
    ///
    /// The shared library must export the entry point function specified in the manifest.
    /// The entry point must return a `Box<dyn Plugin>`.
    pub unsafe fn load(plugin_dir: &Path) -> Result<Self, PluginLoadError> {
        let manifest_path = plugin_dir.join("plugin.json");
        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginLoadError::IoError(manifest_path.clone(), e))?;
        let manifest: PluginManifest =
            serde_json::from_str(&manifest_str).map_err(PluginLoadError::InvalidManifest)?;

        // Find the shared library (.dll on Windows, .so on Linux, .dylib on macOS)
        let lib_name = format!("{}plugin_{}", lib_prefix(), manifest.name);
        let lib_path = plugin_dir.join(format!("{}{}", lib_name, lib_suffix()));

        if !lib_path.exists() {
            return Err(PluginLoadError::LibraryNotFound(lib_path));
        }

        let lib = unsafe { libloading::Library::new(&lib_path) }
            .map_err(|e| PluginLoadError::LibraryLoadFailed(lib_path.clone(), e))?;

        let entry_point: libloading::Symbol<unsafe extern "C" fn() -> *mut dyn Plugin> = unsafe {
            lib.get(manifest.entry_point.as_bytes())
        }
        .map_err(|e| PluginLoadError::EntryPointNotFound(manifest.entry_point.clone(), e))?;

        let plugin_ptr = unsafe { entry_point() };
        let plugin = unsafe { Box::from_raw(plugin_ptr) };

        Ok(DynamicPlugin {
            manifest,
            _lib: lib,
            plugin,
        })
    }

    /// Get the plugin's manifest.
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    /// Get a reference to the loaded plugin.
    pub fn plugin(&self) -> &dyn Plugin {
        &*self.plugin
    }
}

/// Registry of installed dynamic plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    /// Map of plugin name → installation directory.
    pub plugins: HashMap<String, PathBuf>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Load the registry from a JSON file.
    pub fn load(path: &Path) -> Result<Self, PluginLoadError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = std::fs::read_to_string(path)
            .map_err(|e| PluginLoadError::IoError(path.to_path_buf(), e))?;
        serde_json::from_str(&json).map_err(PluginLoadError::InvalidManifest)
    }

    /// Save the registry to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), PluginLoadError> {
        let json =
            serde_json::to_string_pretty(self).map_err(PluginLoadError::SerializationError)?;
        std::fs::write(path, json).map_err(|e| PluginLoadError::IoError(path.to_path_buf(), e))
    }

    /// Register a plugin directory.
    pub fn register(&mut self, name: String, dir: PathBuf) {
        self.plugins.insert(name, dir);
    }

    /// Unregister a plugin.
    pub fn unregister(&mut self, name: &str) {
        self.plugins.remove(name);
    }

    /// Get the directory for a plugin.
    pub fn get_dir(&self, name: &str) -> Option<&PathBuf> {
        self.plugins.get(name)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin loader that manages dynamic plugin loading and registration.
pub struct PluginLoader {
    registry: PluginRegistry,
    registry_path: PathBuf,
    loaded_plugins: Vec<DynamicPlugin>,
}

impl PluginLoader {
    /// Create a new plugin loader with the given registry file path.
    pub fn new(registry_path: PathBuf) -> Result<Self, PluginLoadError> {
        let registry = PluginRegistry::load(&registry_path)?;
        Ok(Self {
            registry,
            registry_path,
            loaded_plugins: Vec::new(),
        })
    }

    /// Install a plugin from a directory by copying it to the plugins directory
    /// and registering it in the registry.
    pub fn install(
        &mut self,
        plugin_dir: &Path,
        plugins_dir: &Path,
    ) -> Result<(), PluginLoadError> {
        // Load manifest to get plugin name
        let manifest_path = plugin_dir.join("plugin.json");
        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginLoadError::IoError(manifest_path.clone(), e))?;
        let manifest: PluginManifest =
            serde_json::from_str(&manifest_str).map_err(PluginLoadError::InvalidManifest)?;

        let dest_dir = plugins_dir.join(&manifest.name);

        // Copy plugin files
        if dest_dir.exists() {
            std::fs::remove_dir_all(&dest_dir)
                .map_err(|e| PluginLoadError::IoError(dest_dir.clone(), e))?;
        }
        copy_dir(plugin_dir, &dest_dir)?;

        self.registry.register(manifest.name.clone(), dest_dir);
        self.registry.save(&self.registry_path)?;

        log::info!("Installed plugin: {} v{}", manifest.name, manifest.version);
        Ok(())
    }

    /// Uninstall a plugin by removing its directory and unregistering it.
    pub fn uninstall(&mut self, name: &str) -> Result<(), PluginLoadError> {
        if let Some(dir) = self.registry.get_dir(name)
            && dir.exists()
        {
            std::fs::remove_dir_all(dir).map_err(|e| PluginLoadError::IoError(dir.clone(), e))?;
        }
        self.registry.unregister(name);
        self.registry.save(&self.registry_path)?;
        log::info!("Uninstalled plugin: {}", name);
        Ok(())
    }

    /// Load all registered plugins.
    ///
    /// # Safety
    ///
    /// Each plugin's shared library must be compatible with the current engine version.
    pub unsafe fn load_all(&mut self) -> Result<(), PluginLoadError> {
        let plugin_dirs: Vec<_> = self.registry.plugins.values().cloned().collect();
        for dir in &plugin_dirs {
            if dir.exists() {
                match unsafe { DynamicPlugin::load(dir) } {
                    Ok(plugin) => {
                        log::info!(
                            "Loaded plugin: {} v{}",
                            plugin.manifest().name,
                            plugin.manifest().version
                        );
                        self.loaded_plugins.push(plugin);
                    }
                    Err(e) => {
                        log::warn!("Failed to load plugin from {}: {}", dir.display(), e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Register all loaded plugins with the application builder.
    pub fn register_all(&self, app: &mut AppBuilder) {
        for plugin in &self.loaded_plugins {
            log::info!("Registering plugin: {}", plugin.manifest().name);
            plugin.plugin().build(app);
        }
    }

    /// Get the plugin registry.
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Get a mutable reference to the plugin registry.
    pub fn registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.registry
    }

    /// Get the list of loaded plugin manifests.
    pub fn loaded_manifests(&self) -> Vec<&PluginManifest> {
        self.loaded_plugins.iter().map(|p| p.manifest()).collect()
    }
}

/// Errors that can occur during plugin loading.
#[derive(Debug, thiserror::Error)]
pub enum PluginLoadError {
    #[error("IO error on {0}: {1}")]
    IoError(PathBuf, std::io::Error),
    #[error("invalid plugin manifest: {0}")]
    InvalidManifest(serde_json::Error),
    #[error("serialization error: {0}")]
    SerializationError(serde_json::Error),
    #[error("library not found: {0}")]
    LibraryNotFound(PathBuf),
    #[error("failed to load library {0}: {1}")]
    LibraryLoadFailed(PathBuf, libloading::Error),
    #[error("entry point '{0}' not found: {1}")]
    EntryPointNotFound(String, libloading::Error),
}

fn lib_prefix() -> &'static str {
    if cfg!(target_os = "windows") {
        ""
    } else {
        "lib"
    }
}

fn lib_suffix() -> &'static str {
    if cfg!(target_os = "windows") {
        ".dll"
    } else if cfg!(target_os = "macos") {
        ".dylib"
    } else {
        ".so"
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), PluginLoadError> {
    std::fs::create_dir_all(dst).map_err(|e| PluginLoadError::IoError(dst.to_path_buf(), e))?;
    for entry in
        std::fs::read_dir(src).map_err(|e| PluginLoadError::IoError(src.to_path_buf(), e))?
    {
        let entry = entry.map_err(|e| PluginLoadError::IoError(src.to_path_buf(), e))?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest).map_err(|e| PluginLoadError::IoError(dest.clone(), e))?;
        }
    }
    Ok(())
}
