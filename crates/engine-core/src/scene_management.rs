//! Scene management (matches Unity's SceneManager).
//!
//! # Unity Documentation
//! <https://docs.unity3d.com/Manual/SceneManagement.html>
//!
//! Scenes are loadable content units. Multiple scenes can be loaded additively.
//! Objects marked DontDestroyOnLoad survive scene loads.
//!
//! ## Runtime model
//! [`SceneManager`] owns scene metadata and root GameObject handles in the
//! Unity-style [`World`](crate::world::World). Loading a scene spawns its
//! serialized roots; unloading destroys those roots (skipping DontDestroyOnLoad).

use crate::gameobject::GameObjectHandle;
use crate::serialization::{self, SceneData, SceneSerializer};
use crate::world::World;
use std::path::Path;

/// Scene handle (matches Unity's Scene handle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneHandle(pub u32);

impl SceneHandle {
    pub const INVALID: Self = Self(u32::MAX);
}

/// How a scene is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadSceneMode {
    /// Replace/keep other scenes; this becomes the active scene.
    /// (Unity Single mode unloads others — we keep them unless Unload is called,
    ///  so callers can implement either policy.)
    Single,
    /// Additive: keep existing loaded scenes.
    Additive,
}

/// Scene information.
#[derive(Debug, Clone)]
pub struct SceneInfo {
    pub name: String,
    pub path: String,
    pub handle: SceneHandle,
    pub is_loaded: bool,
    pub root_count: usize,
    /// Root GameObjects belonging to this scene (used for unload).
    pub(crate) roots: Vec<GameObjectHandle>,
}

/// Scene manager (matches Unity's `SceneManager`).
pub struct SceneManager {
    scenes: Vec<SceneInfo>,
    active_scene: Option<SceneHandle>,
    next_handle: u32,
    serializer: SceneSerializer,
    /// Optional search path for `LoadScene(name)`.
    scene_root: String,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
            active_scene: None,
            next_handle: 0,
            serializer: SceneSerializer::new(),
            scene_root: "Scenes".to_string(),
        }
    }

    /// Set the directory used when resolving scene names to files.
    pub fn set_scene_root(&mut self, root: impl Into<String>) {
        self.scene_root = root.into();
    }

    /// Get the active scene (matches `SceneManager.GetActiveScene`).
    pub fn GetActiveScene(&self) -> Option<&SceneInfo> {
        self.active_scene
            .and_then(|h| self.scenes.iter().find(|s| s.handle == h))
    }

    /// Scene count including unloaded entries (matches `SceneManager.sceneCount`).
    pub fn SceneCount(&self) -> usize {
        self.scenes.len()
    }

    /// Get all loaded scenes (matches `SceneManager.GetLoadedScenes`).
    pub fn GetLoadedScenes(&self) -> Vec<&SceneInfo> {
        self.scenes.iter().filter(|s| s.is_loaded).collect()
    }

    /// Resolve a scene name to a file path under the scene root.
    pub fn scene_path_for_name(&self, name: &str) -> String {
        format!("{}/{}.scene.json", self.scene_root, name)
    }

    /// Create an empty loaded scene (matches creating a new scene at runtime).
    pub fn CreateScene(&mut self, name: &str) -> SceneHandle {
        let handle = SceneHandle(self.next_handle);
        self.next_handle += 1;
        self.scenes.push(SceneInfo {
            name: name.to_string(),
            path: String::new(),
            handle,
            is_loaded: true,
            root_count: 0,
            roots: Vec::new(),
        });
        self.active_scene = Some(handle);
        handle
    }

    /// Load a scene by name without spawning into a World (metadata only).
    ///
    /// Prefer [`SceneManager::LoadScene`] when a World is available.
    pub fn LoadScene(&mut self, name: &str) -> Result<SceneHandle, String> {
        let handle = SceneHandle(self.next_handle);
        self.next_handle += 1;
        self.scenes.push(SceneInfo {
            name: name.to_string(),
            path: self.scene_path_for_name(name),
            handle,
            is_loaded: true,
            root_count: 0,
            roots: Vec::new(),
        });
        self.active_scene = Some(handle);
        Ok(handle)
    }

    /// Load a scene from JSON string into `world`.
    ///
    /// # Unity Documentation
    /// `SceneManager.LoadScene` / `LoadSceneMode`
    pub fn LoadSceneJson(
        &mut self,
        world: &mut World,
        name: &str,
        json: &str,
        mode: LoadSceneMode,
    ) -> Result<SceneHandle, String> {
        let data: SceneData =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse scene JSON: {e}"))?;

        if mode == LoadSceneMode::Single {
            self.UnloadAllLoaded(world);
        }

        let roots = self.serializer.load(&data, world);
        let root_count = roots.len();

        let handle = SceneHandle(self.next_handle);
        self.next_handle += 1;
        self.scenes.push(SceneInfo {
            name: name.to_string(),
            path: self.scene_path_for_name(name),
            handle,
            is_loaded: true,
            root_count,
            roots,
        });
        self.active_scene = Some(handle);
        Ok(handle)
    }

    /// Load a scene from a file path into `world`.
    pub fn LoadSceneFromFile(
        &mut self,
        world: &mut World,
        path: &Path,
        mode: LoadSceneMode,
    ) -> Result<SceneHandle, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read scene file {}: {e}", path.display()))?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Scene")
            .to_string();
        self.LoadSceneJson(world, &name, &json, mode)
    }

    /// Save the current Unity World as a scene JSON string.
    ///
    /// Refreshes pose/hierarchy caches from ECS when `unity-world-primary` is on.
    pub fn SaveSceneJson(&mut self, world: &mut World, name: &str) -> Result<String, String> {
        serialization::SaveSceneJsonPrepared(world, name).map_err(|e| e.to_string())
    }

    /// Register existing root handles as a loaded scene (editor/runtime bridge).
    pub fn register_loaded_scene(
        &mut self,
        name: &str,
        roots: Vec<GameObjectHandle>,
    ) -> SceneHandle {
        let handle = SceneHandle(self.next_handle);
        self.next_handle += 1;
        self.scenes.push(SceneInfo {
            name: name.to_string(),
            path: self.scene_path_for_name(name),
            handle,
            is_loaded: true,
            root_count: roots.len(),
            roots,
        });
        self.active_scene = Some(handle);
        handle
    }

    /// Unload a scene without a World (metadata only).
    ///
    /// For Unity-accurate unload that destroys roots, use [`UnloadSceneWithWorld`](Self::UnloadSceneWithWorld).
    pub fn UnloadScene(&mut self, handle: SceneHandle) -> Result<(), String> {
        self.UnloadSceneMetadata(handle)
    }

    /// Unload a scene and destroy its roots in `world` (Unity-accurate).
    ///
    /// Destroys scene roots, skipping DontDestroyOnLoad objects.
    pub fn UnloadSceneWithWorld(
        &mut self,
        handle: SceneHandle,
        world: &mut World,
    ) -> Result<(), String> {
        let scene = self
            .scenes
            .iter_mut()
            .find(|s| s.handle == handle)
            .ok_or_else(|| "Scene not found".to_string())?;

        if !scene.is_loaded {
            return Ok(());
        }

        let roots = std::mem::take(&mut scene.roots);
        for root in roots {
            if world.is_dont_destroy_on_load(root) {
                continue;
            }
            if world.is_valid(root) {
                world.DestroyImmediate(root);
            }
        }

        scene.is_loaded = false;
        scene.root_count = 0;

        if self.active_scene == Some(handle) {
            self.active_scene = self.scenes.iter().find(|s| s.is_loaded).map(|s| s.handle);
        }

        world.prune_dont_destroy();
        Ok(())
    }

    /// Unload all loaded scenes (DontDestroyOnLoad objects survive).
    pub fn UnloadAllLoaded(&mut self, world: &mut World) {
        let handles: Vec<SceneHandle> = self
            .scenes
            .iter()
            .filter(|s| s.is_loaded)
            .map(|s| s.handle)
            .collect();
        for h in handles {
            let _ = self.UnloadSceneWithWorld(h, world);
        }
    }

    /// Unload a scene without a World (metadata only).
    pub fn UnloadSceneMetadata(&mut self, handle: SceneHandle) -> Result<(), String> {
        if let Some(scene) = self.scenes.iter_mut().find(|s| s.handle == handle) {
            scene.is_loaded = false;
            scene.root_count = 0;
            scene.roots.clear();
            if self.active_scene == Some(handle) {
                self.active_scene = self.scenes.iter().find(|s| s.is_loaded).map(|s| s.handle);
            }
            Ok(())
        } else {
            Err("Scene not found".to_string())
        }
    }

    /// Set the active scene.
    pub fn SetActiveScene(&mut self, handle: SceneHandle) -> Result<(), String> {
        if self
            .scenes
            .iter()
            .any(|s| s.handle == handle && s.is_loaded)
        {
            self.active_scene = Some(handle);
            Ok(())
        } else {
            Err("Scene is not loaded".to_string())
        }
    }

    /// Get scene info by handle.
    pub fn GetScene(&self, handle: SceneHandle) -> Option<&SceneInfo> {
        self.scenes.iter().find(|s| s.handle == handle)
    }

    /// Find a loaded scene by name.
    pub fn FindSceneByName(&self, name: &str) -> Option<&SceneInfo> {
        self.scenes.iter().find(|s| s.is_loaded && s.name == name)
    }

    /// Roots of a loaded scene.
    pub fn scene_roots(&self, handle: SceneHandle) -> &[GameObjectHandle] {
        self.scenes
            .iter()
            .find(|s| s.handle == handle)
            .map(|s| s.roots.as_slice())
            .unwrap_or(&[])
    }
}
