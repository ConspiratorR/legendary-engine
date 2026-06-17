use crate::wasm::{WasmComponentBridge, WasmRuntime, WasmSandbox, WasmSystem};
use engine_ecs::system::System;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Mod manifest loaded from a `mod.json` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    /// Mod name (unique identifier).
    pub name: String,
    /// Semantic version string (e.g. "1.0.0").
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Mod author.
    pub author: String,
    /// Entry point WASM file name.
    pub entry_point: String,
    /// Required engine version (semver range).
    pub engine_version: String,
    /// Mod dependencies (name → version range).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// Assets provided by this mod.
    #[serde(default)]
    pub assets: Vec<String>,
    /// Components registered by this mod.
    #[serde(default)]
    pub components: Vec<ComponentDef>,
    /// Systems registered by this mod.
    #[serde(default)]
    pub systems: Vec<SystemDef>,
}

/// Component definition from a mod manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDef {
    /// Component name.
    pub name: String,
    /// Component type (e.g. "f32", "Vec3", "custom").
    pub component_type: String,
    /// Size in bytes (for custom types).
    pub size: u32,
}

/// System definition from a mod manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDef {
    /// System name.
    pub name: String,
    /// System execution order (lower runs first).
    pub order: i32,
    /// Dependencies (system names that must run before this one).
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// A loaded mod with its manifest, WASM system, and sandbox.
pub struct Mod {
    pub manifest: ModManifest,
    pub system: WasmSystem,
    pub sandbox: WasmSandbox,
}

impl Mod {
    /// Get the mod's manifest.
    pub fn manifest(&self) -> &ModManifest {
        &self.manifest
    }

    /// Get the mod's name.
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// Get the mod's version.
    pub fn version(&self) -> &str {
        &self.manifest.version
    }
}

/// Mod loader that manages mod loading, dependency resolution, and execution.
pub struct ModLoader {
    runtime: Arc<WasmRuntime>,
    bridge: Arc<RwLock<WasmComponentBridge>>,
    loaded_mods: Vec<Mod>,
    mod_dirs: Vec<PathBuf>,
}

impl ModLoader {
    /// Create a new mod loader.
    pub fn new() -> Result<Self, ModLoadError> {
        let runtime = Arc::new(WasmRuntime::new().map_err(ModLoadError::WasmError)?);
        let bridge = Arc::new(RwLock::new(WasmComponentBridge::new()));
        Ok(Self {
            runtime,
            bridge,
            loaded_mods: Vec::new(),
            mod_dirs: Vec::new(),
        })
    }

    /// Add a directory to search for mods.
    pub fn add_mod_dir(&mut self, dir: PathBuf) {
        self.mod_dirs.push(dir);
    }

    /// Scan all mod directories and load mods.
    pub fn load_all(&mut self) -> Result<(), ModLoadError> {
        let mut manifests = Vec::new();

        // Scan all mod directories for mod.json files
        for dir in &self.mod_dirs {
            if !dir.exists() {
                continue;
            }
            for entry in
                std::fs::read_dir(dir).map_err(|e| ModLoadError::IoError(dir.clone(), e))?
            {
                let entry = entry.map_err(|e| ModLoadError::IoError(dir.clone(), e))?;
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("mod.json");
                    if manifest_path.exists() {
                        let manifest_str = std::fs::read_to_string(&manifest_path)
                            .map_err(|e| ModLoadError::IoError(manifest_path.clone(), e))?;
                        let manifest: ModManifest = serde_json::from_str(&manifest_str)
                            .map_err(ModLoadError::InvalidManifest)?;
                        manifests.push((manifest, path));
                    }
                }
            }
        }

        // Resolve dependencies and sort
        let sorted = resolve_dependencies(&manifests)?;

        // Load mods in dependency order
        for (manifest, dir) in sorted {
            let wasm_path = dir.join(&manifest.entry_point);
            if !wasm_path.exists() {
                return Err(ModLoadError::WasmNotFound(wasm_path));
            }

            let wasm_bytes = std::fs::read(&wasm_path)
                .map_err(|e| ModLoadError::IoError(wasm_path.clone(), e))?;

            let sandbox = WasmSandbox::default();
            let system = WasmSystem::new(
                &manifest.name,
                &wasm_bytes,
                self.runtime.clone(),
                self.bridge.clone(),
            )
            .map_err(ModLoadError::WasmError)?;

            log::info!("Loaded mod: {} v{}", manifest.name, manifest.version);
            self.loaded_mods.push(Mod {
                manifest,
                system,
                sandbox,
            });
        }

        Ok(())
    }

    /// Get all loaded mods.
    pub fn mods(&self) -> &[Mod] {
        &self.loaded_mods
    }

    /// Get a mod by name.
    pub fn get_mod(&self, name: &str) -> Option<&Mod> {
        self.loaded_mods.iter().find(|m| m.name() == name)
    }

    /// Run all mod systems.
    pub fn run_systems(&self, world: &mut engine_ecs::world::World) {
        for m in &self.loaded_mods {
            m.system.run(world);
        }
    }

    /// Get the component bridge.
    pub fn bridge(&self) -> &Arc<RwLock<WasmComponentBridge>> {
        &self.bridge
    }

    /// Get a mutable reference to the component bridge.
    pub fn bridge_mut(&mut self) -> &mut Arc<RwLock<WasmComponentBridge>> {
        &mut self.bridge
    }

    /// Get the list of loaded mod manifests.
    pub fn loaded_manifests(&self) -> Vec<&ModManifest> {
        self.loaded_mods.iter().map(|m| m.manifest()).collect()
    }
}

/// Errors that can occur during mod loading.
#[derive(Debug, thiserror::Error)]
pub enum ModLoadError {
    #[error("IO error on {0}: {1}")]
    IoError(PathBuf, std::io::Error),
    #[error("invalid mod manifest: {0}")]
    InvalidManifest(serde_json::Error),
    #[error("WASM file not found: {0}")]
    WasmNotFound(PathBuf),
    #[error("WASM error: {0}")]
    WasmError(anyhow::Error),
    #[error("circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("missing dependency: {0} requires {1}")]
    MissingDependency(String, String),
}

/// Resolve mod dependencies and return mods in topological order.
fn resolve_dependencies(
    mods: &[(ModManifest, PathBuf)],
) -> Result<Vec<(ModManifest, PathBuf)>, ModLoadError> {
    let mut name_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, (manifest, _)) in mods.iter().enumerate() {
        name_to_idx.insert(&manifest.name, i);
    }

    // Check for missing dependencies
    for (manifest, _) in mods {
        for dep_name in manifest.dependencies.keys() {
            if !name_to_idx.contains_key(dep_name.as_str()) {
                return Err(ModLoadError::MissingDependency(
                    manifest.name.clone(),
                    dep_name.clone(),
                ));
            }
        }
    }

    // Topological sort using Kahn's algorithm
    let n = mods.len();
    let mut in_degree = vec![0; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, (manifest, _)) in mods.iter().enumerate() {
        for dep_name in manifest.dependencies.keys() {
            if let Some(&dep_idx) = name_to_idx.get(dep_name.as_str()) {
                adj[dep_idx].push(i);
                in_degree[i] += 1;
            }
        }
    }

    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut sorted = Vec::new();

    while let Some(idx) = queue.pop() {
        sorted.push(idx);
        for &next in &adj[idx] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push(next);
            }
        }
    }

    if sorted.len() != n {
        return Err(ModLoadError::CircularDependency(
            "circular dependency detected in mod dependencies".to_string(),
        ));
    }

    // Return mods in topological order
    let mods_map: HashMap<usize, (ModManifest, PathBuf)> =
        mods.iter().cloned().enumerate().collect();
    Ok(sorted.into_iter().map(|i| mods_map[&i].clone()).collect())
}
