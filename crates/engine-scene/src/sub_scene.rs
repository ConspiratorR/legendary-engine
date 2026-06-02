//! Streaming sub-scene management with distance-based loading and unloading.
//!
//! [`StreamingSubSceneManager`] tracks sub-scenes that should be loaded or
//! unloaded based on the distance from a reference point (typically the
//! camera or player). Sub-scenes beyond the unload distance are removed;
//! those within the load distance are added.
//!
//! # Example
//!
//! ```rust,no_run
//! use engine_scene::sub_scene::{StreamingSubSceneManager, SubSceneConfig};
//! use engine_scene::multi_scene::MultiSceneManager;
//! use engine_scene::scene_layer::SceneLayer;
//! use engine_scene::serialization::SceneData;
//!
//! let mut multi = MultiSceneManager::new();
//! let mut streamer = StreamingSubSceneManager::new();
//!
//! streamer.register(SubSceneConfig {
//!     name: "chunk_0_0".into(),
//!     center: [0.0, 0.0, 0.0],
//!     load_distance: 100.0,
//!     unload_distance: 150.0,
//!     layers: SceneLayer::DEFAULT,
//!     priority: 0,
//! });
//!
//! // In the game loop, call `update` with the player position and a
//! // scene loader callback.
//! ```

use std::collections::HashMap;

use engine_math::Vec3;

use crate::multi_scene::MultiSceneManager;
use crate::scene_layer::SceneLayer;
use crate::serialization::SceneData;

// ── Error Type ──────────────────────────────────────────────────────

use thiserror::Error;

/// Errors from streaming sub-scene operations.
#[derive(Error, Debug)]
pub enum SubSceneError {
    /// A sub-scene with this name is already registered.
    #[error("sub-scene '{0}' is already registered")]
    AlreadyRegistered(String),

    /// No sub-scene with this name is registered.
    #[error("sub-scene '{0}' is not registered")]
    NotRegistered(String),

    /// Error from the underlying multi-scene manager.
    #[error("multi-scene error: {0}")]
    MultiScene(#[from] crate::multi_scene::MultiSceneError),
}

// ── Sub-Scene Config ────────────────────────────────────────────────

/// Configuration for a single sub-scene that can be streamed in and out.
#[derive(Debug, Clone)]
pub struct SubSceneConfig {
    /// Unique identifier for this sub-scene.
    pub name: String,
    /// World-space center position for distance calculations.
    pub center: [f32; 3],
    /// Distance threshold at which the sub-scene should be loaded.
    pub load_distance: f32,
    /// Distance threshold at which the sub-scene should be unloaded.
    /// Must be >= `load_distance` to prevent thrashing.
    pub unload_distance: f32,
    /// Scene layers to assign when loaded.
    pub layers: SceneLayer,
    /// Loading priority (higher = loaded first when multiple are needed).
    pub priority: i32,
}

// ── Sub-Scene State ─────────────────────────────────────────────────

/// Current loading state of a sub-scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubSceneState {
    /// Not loaded.
    Unloaded,
    /// Currently loaded and active.
    Loaded,
}

/// Runtime info for a tracked sub-scene.
#[derive(Debug, Clone)]
pub struct SubSceneInfo {
    /// Configuration for this sub-scene.
    pub config: SubSceneConfig,
    /// Current loading state.
    pub state: SubSceneState,
    /// Last computed distance from the reference point.
    pub distance: f32,
}

// ── Streaming Sub-Scene Manager ─────────────────────────────────────

/// Manages streaming of sub-scenes based on distance from a reference point.
///
/// Sub-scenes are registered with [`register`](Self::register). On each
/// [`update`](Self::update) call the manager computes distances and
/// triggers loads/unloads through the provided [`MultiSceneManager`].
pub struct StreamingSubSceneManager {
    sub_scenes: HashMap<String, SubSceneInfo>,
}

impl StreamingSubSceneManager {
    /// Create an empty streaming manager.
    pub fn new() -> Self {
        Self {
            sub_scenes: HashMap::new(),
        }
    }

    /// Register a sub-scene for streaming.
    pub fn register(&mut self, config: SubSceneConfig) -> Result<(), SubSceneError> {
        if self.sub_scenes.contains_key(&config.name) {
            return Err(SubSceneError::AlreadyRegistered(config.name.clone()));
        }

        let name = config.name.clone();
        self.sub_scenes.insert(
            name,
            SubSceneInfo {
                config,
                state: SubSceneState::Unloaded,
                distance: f32::MAX,
            },
        );
        Ok(())
    }

    /// Unregister a sub-scene, removing it from tracking.
    ///
    /// If the sub-scene is currently loaded, it is **not** automatically
    /// unloaded from the multi-scene manager — the caller must handle that.
    pub fn unregister(&mut self, name: &str) -> Result<(), SubSceneError> {
        self.sub_scenes
            .remove(name)
            .ok_or_else(|| SubSceneError::NotRegistered(name.to_string()))?;
        Ok(())
    }

    /// Check if a sub-scene is registered.
    pub fn is_registered(&self, name: &str) -> bool {
        self.sub_scenes.contains_key(name)
    }

    /// Get info for a sub-scene.
    pub fn get(&self, name: &str) -> Option<&SubSceneInfo> {
        self.sub_scenes.get(name)
    }

    /// Return all registered sub-scene names.
    pub fn registered(&self) -> Vec<&str> {
        self.sub_scenes.keys().map(|s| s.as_str()).collect()
    }

    /// Return all sub-scenes that are currently loaded.
    pub fn loaded(&self) -> Vec<&SubSceneInfo> {
        self.sub_scenes
            .values()
            .filter(|s| s.state == SubSceneState::Loaded)
            .collect()
    }

    /// Update streaming state based on a reference position.
    ///
    /// Computes distances and returns a list of actions (load/unload)
    /// that the caller should execute. The `scene_loader` callback is
    /// invoked for each sub-scene that needs loading — it should return
    /// the `SceneData` for that sub-scene.
    pub fn update(
        &mut self,
        reference: Vec3,
        multi: &mut MultiSceneManager,
        scene_loader: impl Fn(&str) -> Option<SceneData>,
    ) -> Result<Vec<StreamAction>, SubSceneError> {
        let mut actions = Vec::new();

        // Update distances
        for info in self.sub_scenes.values_mut() {
            let center = Vec3::new(
                info.config.center[0],
                info.config.center[1],
                info.config.center[2],
            );
            info.distance = (reference - center).length();
        }

        // Collect names and decisions first to avoid borrow conflicts
        let decisions: Vec<(String, StreamDecision)> = self
            .sub_scenes
            .values()
            .map(|info| {
                let decision = match info.state {
                    SubSceneState::Unloaded => {
                        if info.distance <= info.config.load_distance {
                            StreamDecision::Load
                        } else {
                            StreamDecision::Nothing
                        }
                    }
                    SubSceneState::Loaded => {
                        if info.distance > info.config.unload_distance {
                            StreamDecision::Unload
                        } else {
                            StreamDecision::Nothing
                        }
                    }
                };
                (info.config.name.clone(), decision)
            })
            .collect();

        // Sort load actions by priority (higher first)
        let mut load_names: Vec<&str> = decisions
            .iter()
            .filter_map(|(name, dec)| {
                if *dec == StreamDecision::Load {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .collect();
        load_names.sort_by_key(|name| {
            -self
                .sub_scenes
                .get(*name)
                .map(|s| s.config.priority)
                .unwrap_or(0)
        });

        // Execute unloads first
        for (name, dec) in &decisions {
            if *dec == StreamDecision::Unload {
                multi.remove_scene(name)?;
                if let Some(info) = self.sub_scenes.get_mut(name) {
                    info.state = SubSceneState::Unloaded;
                }
                actions.push(StreamAction::Unloaded(name.clone()));
            }
        }

        // Execute loads
        for name in load_names {
            if let Some(scene_data) = scene_loader(name) {
                let config = &self.sub_scenes[name].config;
                let layers = config.layers;
                multi.add_scene(name, scene_data, layers)?;
                if let Some(info) = self.sub_scenes.get_mut(name) {
                    info.state = SubSceneState::Loaded;
                }
                actions.push(StreamAction::Loaded(name.to_string()));
            }
        }

        Ok(actions)
    }

    /// Force-load a specific sub-scene regardless of distance.
    pub fn force_load(
        &mut self,
        name: &str,
        multi: &mut MultiSceneManager,
        scene_data: SceneData,
    ) -> Result<(), SubSceneError> {
        let info = self
            .sub_scenes
            .get_mut(name)
            .ok_or_else(|| SubSceneError::NotRegistered(name.to_string()))?;
        let layers = info.config.layers;
        multi.add_scene(name, scene_data, layers)?;
        info.state = SubSceneState::Loaded;
        Ok(())
    }

    /// Force-unload a specific sub-scene regardless of distance.
    pub fn force_unload(
        &mut self,
        name: &str,
        multi: &mut MultiSceneManager,
    ) -> Result<(), SubSceneError> {
        let info = self
            .sub_scenes
            .get_mut(name)
            .ok_or_else(|| SubSceneError::NotRegistered(name.to_string()))?;
        if info.state == SubSceneState::Loaded {
            multi.remove_scene(name)?;
            info.state = SubSceneState::Unloaded;
        }
        Ok(())
    }

    /// Return the number of registered sub-scenes.
    pub fn len(&self) -> usize {
        self.sub_scenes.len()
    }

    /// Return `true` if no sub-scenes are registered.
    pub fn is_empty(&self) -> bool {
        self.sub_scenes.is_empty()
    }
}

impl Default for StreamingSubSceneManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Action / Decision ───────────────────────────────────────────────

/// Action taken by the streaming manager during an update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamAction {
    /// A sub-scene was loaded.
    Loaded(String),
    /// A sub-scene was unloaded.
    Unloaded(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamDecision {
    Load,
    Unload,
    Nothing,
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{SceneData, SceneEntityData};

    fn make_scene_data(name: &str) -> SceneData {
        let mut scene = SceneData::new(name);
        scene.add_entity(SceneEntityData::new(1, "Entity"));
        scene
    }

    fn default_config(name: &str, center: [f32; 3]) -> SubSceneConfig {
        SubSceneConfig {
            name: name.into(),
            center,
            load_distance: 10.0,
            unload_distance: 15.0,
            layers: SceneLayer::DEFAULT,
            priority: 0,
        }
    }

    #[test]
    fn test_register_and_unregister() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("a", [0.0, 0.0, 0.0])).unwrap();
        assert!(sm.is_registered("a"));
        assert_eq!(sm.len(), 1);

        sm.unregister("a").unwrap();
        assert!(!sm.is_registered("a"));
        assert_eq!(sm.len(), 0);
    }

    #[test]
    fn test_duplicate_register_error() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("a", [0.0, 0.0, 0.0])).unwrap();
        let result = sm.register(default_config("a", [0.0, 0.0, 0.0]));
        assert!(matches!(result, Err(SubSceneError::AlreadyRegistered(_))));
    }

    #[test]
    fn test_unregister_nonexistent_error() {
        let mut sm = StreamingSubSceneManager::new();
        let result = sm.unregister("nope");
        assert!(matches!(result, Err(SubSceneError::NotRegistered(_))));
    }

    #[test]
    fn test_load_when_within_distance() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("chunk", [5.0, 0.0, 0.0]))
            .unwrap();

        let mut multi = MultiSceneManager::new();
        let actions = sm
            .update(Vec3::ZERO, &mut multi, |name| Some(make_scene_data(name)))
            .unwrap();

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], StreamAction::Loaded("chunk".into()));
        assert!(multi.has_scene("chunk"));
        assert_eq!(sm.get("chunk").unwrap().state, SubSceneState::Loaded);
    }

    #[test]
    fn test_no_load_when_beyond_distance() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("far", [100.0, 0.0, 0.0]))
            .unwrap();

        let mut multi = MultiSceneManager::new();
        let actions = sm
            .update(Vec3::ZERO, &mut multi, |name| Some(make_scene_data(name)))
            .unwrap();

        assert!(actions.is_empty());
        assert!(!multi.has_scene("far"));
    }

    #[test]
    fn test_unload_when_beyond_unload_distance() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("chunk", [0.0, 0.0, 0.0]))
            .unwrap();

        let mut multi = MultiSceneManager::new();

        // Load first
        sm.update(Vec3::ZERO, &mut multi, |name| Some(make_scene_data(name)))
            .unwrap();
        assert!(multi.has_scene("chunk"));

        // Move far away
        let actions = sm
            .update(Vec3::new(100.0, 0.0, 0.0), &mut multi, |name| {
                Some(make_scene_data(name))
            })
            .unwrap();

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], StreamAction::Unloaded("chunk".into()));
        assert!(!multi.has_scene("chunk"));
        assert_eq!(sm.get("chunk").unwrap().state, SubSceneState::Unloaded);
    }

    #[test]
    fn test_no_thrash_at_boundary() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("edge", [12.0, 0.0, 0.0]))
            .unwrap();

        let mut multi = MultiSceneManager::new();

        // At distance 12, within unload (15) but beyond load (10) — should not load
        let actions = sm
            .update(Vec3::ZERO, &mut multi, |name| Some(make_scene_data(name)))
            .unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_priority_order() {
        let mut sm = StreamingSubSceneManager::new();

        let mut low = default_config("low", [1.0, 0.0, 0.0]);
        low.priority = 1;
        let mut high = default_config("high", [2.0, 0.0, 0.0]);
        high.priority = 10;

        sm.register(low).unwrap();
        sm.register(high).unwrap();

        let mut multi = MultiSceneManager::new();
        let actions = sm
            .update(Vec3::ZERO, &mut multi, |name| Some(make_scene_data(name)))
            .unwrap();

        assert_eq!(actions.len(), 2);
        // Higher priority loads first
        assert_eq!(actions[0], StreamAction::Loaded("high".into()));
        assert_eq!(actions[1], StreamAction::Loaded("low".into()));
    }

    #[test]
    fn test_force_load_and_unload() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("forced", [100.0, 0.0, 0.0]))
            .unwrap();

        let mut multi = MultiSceneManager::new();

        // Force load even though far away
        sm.force_load("forced", &mut multi, make_scene_data("forced"))
            .unwrap();
        assert!(multi.has_scene("forced"));
        assert_eq!(sm.get("forced").unwrap().state, SubSceneState::Loaded);

        // Force unload
        sm.force_unload("forced", &mut multi).unwrap();
        assert!(!multi.has_scene("forced"));
        assert_eq!(sm.get("forced").unwrap().state, SubSceneState::Unloaded);
    }

    #[test]
    fn test_force_nonexistent_error() {
        let mut sm = StreamingSubSceneManager::new();
        let mut multi = MultiSceneManager::new();

        let result = sm.force_load("nope", &mut multi, make_scene_data("nope"));
        assert!(matches!(result, Err(SubSceneError::NotRegistered(_))));
    }

    #[test]
    fn test_multiple_subscenes() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("a", [0.0, 0.0, 0.0])).unwrap();
        sm.register(default_config("b", [5.0, 0.0, 0.0])).unwrap();
        sm.register(default_config("c", [50.0, 0.0, 0.0])).unwrap();

        let mut multi = MultiSceneManager::new();
        let actions = sm
            .update(Vec3::ZERO, &mut multi, |name| Some(make_scene_data(name)))
            .unwrap();

        // a and b are within load_distance (10), c is not
        assert_eq!(actions.len(), 2);
        assert!(multi.has_scene("a"));
        assert!(multi.has_scene("b"));
        assert!(!multi.has_scene("c"));
    }

    #[test]
    fn test_loader_returns_none() {
        let mut sm = StreamingSubSceneManager::new();
        sm.register(default_config("missing", [0.0, 0.0, 0.0]))
            .unwrap();

        let mut multi = MultiSceneManager::new();
        let actions = sm.update(Vec3::ZERO, &mut multi, |_| None).unwrap();

        // No action because loader returned None
        assert!(actions.is_empty());
        assert!(!multi.has_scene("missing"));
    }
}
