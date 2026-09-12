//! Scene runtime resource — Unity-style World + SceneManager hosted inside the ECS App.
//!
//! # Architecture
//! `App` owns the sparse-set ECS world used by systems. The Unity-facing
//! GameObject/MonoBehaviour World lives here as a resource so plugins and
//! lifecycle ticks can share it without merging storage models yet.

use crate::scene_management::{LoadSceneMode, SceneManager};
use crate::time::Time;
use crate::world::World;

/// Hosts the Unity-style scene runtime (GameObjects, MonoBehaviours, scenes).
pub struct SceneRuntime {
    /// Unity World (GameObjects + Transform hierarchy + MonoBehaviours).
    pub world: World,
    /// Scene manager for load/unload.
    pub scenes: SceneManager,
    /// Whether Awake has been run for newly spawned objects.
    awake_started: bool,
}

impl Default for SceneRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneRuntime {
    /// Create an empty scene runtime.
    pub fn new() -> Self {
        Self {
            world: World::new(),
            scenes: SceneManager::new(),
            awake_started: false,
        }
    }

    /// Spawn a named GameObject in the Unity world.
    pub fn spawn(&mut self, name: &str) -> crate::gameobject::GameObjectHandle {
        self.world.CreateGameObject(name)
    }

    /// Load a scene from JSON (Single or Additive).
    pub fn load_scene_json(
        &mut self,
        name: &str,
        json: &str,
        mode: LoadSceneMode,
    ) -> Result<crate::scene_management::SceneHandle, String> {
        let handle = self.scenes.LoadSceneJson(&mut self.world, name, json, mode);
        self.awake_started = false;
        handle
    }

    /// Unload a scene (destroys roots, skips DontDestroyOnLoad).
    pub fn unload_scene(
        &mut self,
        handle: crate::scene_management::SceneHandle,
    ) -> Result<(), String> {
        self.scenes.UnloadSceneWithWorld(handle, &mut self.world)
    }

    /// Run one Unity-style lifecycle frame (FixedUpdate 0+ → Start → Update → LateUpdate → destroy).
    ///
    /// Call after `Time::update` so the fixed-step accumulator is current.
    pub fn tick(&mut self, time: &Time, frame: u64, events: &mut crate::event::EventBus) {
        // Unity: all Awake/OnEnable before first Start after load
        if !self.awake_started {
            self.world.tick_awake(time.clone(), frame, events);
            self.awake_started = true;
        }
        self.world.tick_start(time.clone(), frame, events);

        let fixed_steps = time.pending_fixed_steps();
        // Caller is responsible for begin/end_fixed_update around each step;
        // we only dispatch callbacks here so Time flag stays consistent.
        for _ in 0..fixed_steps {
            self.world.tick_fixed_update(time.clone(), frame, events);
        }

        self.world.tick_update(time.clone(), frame, events);
        self.world.tick_late_update(time.clone(), frame, events);

        let delta = if time.inFixedTimeStep() {
            time.fixedDeltaTime()
        } else {
            time.deltaTime()
        };
        self.world
            .tick_end_of_frame(delta, time.clone(), frame, events);
    }

    /// Run a single FixedUpdate dispatch (used by App when it drives the accumulator).
    pub fn tick_fixed_only(
        &mut self,
        time: &Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        if !self.awake_started {
            self.world.tick_awake(time.clone(), frame, events);
            self.awake_started = true;
        }
        self.world.tick_fixed_update(time.clone(), frame, events);
    }

    /// Run Update + LateUpdate + end-of-frame (no FixedUpdate).
    pub fn tick_variable_only(
        &mut self,
        time: &Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        if !self.awake_started {
            self.world.tick_awake(time.clone(), frame, events);
            self.awake_started = true;
        }
        self.world.tick_start(time.clone(), frame, events);
        self.world.tick_update(time.clone(), frame, events);
        self.world.tick_late_update(time.clone(), frame, events);
        self.world
            .tick_end_of_frame(time.deltaTime(), time.clone(), frame, events);
    }

    /// Run Awake (if needed) + Start + Update only (App drives LateUpdate separately).
    pub fn tick_variable_only_partial(
        &mut self,
        time: &Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        if !self.awake_started {
            self.world.tick_awake(time.clone(), frame, events);
            self.awake_started = true;
        }
        self.world.tick_start(time.clone(), frame, events);
        self.world.tick_update(time.clone(), frame, events);
    }

    /// Mark that new objects need Awake on next tick (after Instantiate/Load).
    pub fn mark_needs_awake(&mut self) {
        self.awake_started = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBus;

    #[test]
    fn test_scene_runtime_spawn() {
        let mut rt = SceneRuntime::new();
        let h = rt.spawn("Player");
        assert_eq!(rt.world.GetName(h), "Player");
    }

    #[test]
    fn test_scene_runtime_tick_empty() {
        let mut rt = SceneRuntime::new();
        let time = Time::default();
        let mut events = EventBus::new();
        rt.tick(&time, 0, &mut events);
    }

    #[test]
    fn test_scene_runtime_load_json() {
        let mut rt = SceneRuntime::new();
        // glam Vec3/Quat serialize as arrays
        let json = r#"{
            "name": "Level",
            "version": 1,
            "game_objects": [
                {
                    "name": "Player",
                    "tag": "Player",
                    "layer": 0,
                    "active": true,
                    "transform": {
                        "local_position": [1.0, 2.0, 3.0],
                        "local_rotation": [0.0, 0.0, 0.0, 1.0],
                        "local_scale": [1.0, 1.0, 1.0]
                    },
                    "components": [],
                    "children": []
                }
            ]
        }"#;
        let handle = rt
            .load_scene_json("Level", json, LoadSceneMode::Single)
            .unwrap();
        let info = rt.scenes.GetScene(handle).unwrap();
        assert_eq!(info.name, "Level");
        assert_eq!(info.root_count, 1);
        assert!(rt.world.Find("Player").is_some());
    }

    #[test]
    fn test_scene_runtime_unload_skips_dont_destroy() {
        let mut rt = SceneRuntime::new();
        let player = rt.spawn("Player");
        let env = rt.spawn("Env");
        rt.world.DontDestroyOnLoad(player);
        let handle = rt.scenes.register_loaded_scene("Level", vec![player, env]);

        rt.unload_scene(handle).unwrap();
        assert!(rt.world.is_valid(player));
        assert!(!rt.world.is_valid(env));
    }
}
