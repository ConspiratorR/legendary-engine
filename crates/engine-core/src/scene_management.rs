//! Scene management (matches Unity's SceneManager).

/// Scene handle (matches Unity's Scene).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneHandle(pub u32);

impl SceneHandle {
    pub const INVALID: Self = Self(u32::MAX);
}

/// Scene information.
#[derive(Debug, Clone)]
pub struct SceneInfo {
    pub name: String,
    pub path: String,
    pub handle: SceneHandle,
    pub is_loaded: bool,
    pub root_count: usize,
}

/// Scene manager (matches Unity's `SceneManager`).
pub struct SceneManager {
    scenes: Vec<SceneInfo>,
    active_scene: Option<SceneHandle>,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self {
            scenes: Vec::new(),
            active_scene: None,
        }
    }
}

impl SceneManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the active scene (matches `SceneManager.GetActiveScene`).
    pub fn GetActiveScene(&self) -> Option<&SceneInfo> {
        self.active_scene
            .and_then(|h| self.scenes.iter().find(|s| s.handle == h))
    }

    /// Load a scene by name (matches `SceneManager.LoadScene`).
    pub fn LoadScene(&mut self, name: &str) -> Result<SceneHandle, String> {
        let handle = SceneHandle(self.scenes.len() as u32);
        self.scenes.push(SceneInfo {
            name: name.to_string(),
            path: format!("Scenes/{}.unity", name),
            handle,
            is_loaded: true,
            root_count: 0,
        });
        self.active_scene = Some(handle);
        Ok(handle)
    }

    /// Unload a scene (matches `SceneManager.UnloadScene`).
    pub fn UnloadScene(&mut self, handle: SceneHandle) -> Result<(), String> {
        if let Some(scene) = self.scenes.iter_mut().find(|s| s.handle == handle) {
            scene.is_loaded = false;
            Ok(())
        } else {
            Err("Scene not found".to_string())
        }
    }

    /// Get scene count (matches `SceneManager.sceneCount`).
    pub fn SceneCount(&self) -> usize {
        self.scenes.len()
    }

    /// Get all loaded scenes (matches `SceneManager.GetLoadedScenes`).
    pub fn GetLoadedScenes(&self) -> Vec<&SceneInfo> {
        self.scenes.iter().filter(|s| s.is_loaded).collect()
    }
}
