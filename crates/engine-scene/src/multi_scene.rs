//! Multi-scene management: load, merge, and query multiple scenes simultaneously.
//!
//! [`MultiSceneManager`] holds several named scenes, each with its own
//! [`SceneLayer`] mask and entity namespace. When scenes are loaded they are
//! merged into a single [`World`] with remapped entity IDs to prevent
//! cross-scene conflicts.
//!
//! # Example
//!
//! ```rust,no_run
//! use engine_scene::multi_scene::MultiSceneManager;
//! use engine_scene::scene_layer::SceneLayer;
//! use engine_scene::serialization::SceneData;
//!
//! let mut mgr = MultiSceneManager::new();
//!
//! let mut base = SceneData::new("level_base");
//! // ... populate base scene ...
//! mgr.add_scene("level_base", base, SceneLayer::DEFAULT);
//!
//! let mut props = SceneData::new("level_props");
//! // ... populate props scene ...
//! mgr.add_scene("level_props", props, SceneLayer::GAMEPLAY);
//!
//! assert_eq!(mgr.loaded_scenes().len(), 2);
//! ```

use std::collections::HashMap;

use engine_ecs::entity::Entity;
use engine_ecs::gameobject::GameObjectHandle;
use engine_ecs::world::World;

use crate::hierarchy::{Children, Parent};
use crate::node::SceneNode;
use crate::scene_layer::SceneLayer;
use crate::serialization::{SceneData, SceneEntityData};
use crate::transform::{GlobalTransform, Transform};

// ── Error Type ──────────────────────────────────────────────────────

use thiserror::Error;

/// Errors from multi-scene operations.
#[derive(Error, Debug)]
pub enum MultiSceneError {
    /// A scene with the given name is already loaded.
    #[error("scene '{0}' is already loaded")]
    AlreadyLoaded(String),

    /// No scene with the given name is loaded.
    #[error("scene '{0}' is not loaded")]
    NotLoaded(String),

    /// Entity namespace conflict during merge.
    #[error("entity ID conflict in scene '{scene}': entity {entity_id}")]
    EntityConflict { scene: String, entity_id: u64 },
}

// ── Scene Handle ────────────────────────────────────────────────────

/// Metadata for a loaded scene within the [`MultiSceneManager`].
#[derive(Debug, Clone)]
pub struct LoadedScene {
    /// Unique scene name / identifier.
    pub name: String,
    /// Layer mask for this scene.
    pub layers: SceneLayer,
    /// Mapping from original entity IDs (in `SceneData`) to merged `Entity` handles.
    pub entity_map: HashMap<u64, Entity>,
    /// Whether this scene is currently active (visible / ticked).
    pub active: bool,
}

// ── Multi-Scene Manager ─────────────────────────────────────────────

/// Manages multiple scenes loaded simultaneously, merging their entities
/// into a single ECS [`World`] with namespaced entity IDs.
pub struct MultiSceneManager {
    world: World,
    scenes: HashMap<String, LoadedScene>,
    /// Global root node under which all scene roots are parented.
    root: SceneNode,
    /// Mapping from Entity index to GameObjectHandle.
    entity_to_handle: HashMap<u32, GameObjectHandle>,
    /// Mapping from GameObjectHandle index to Entity.
    handle_to_entity: HashMap<u32, Entity>,
    /// Counter for generating unique GameObjectHandle indices.
    next_handle_index: u32,
}

impl MultiSceneManager {
    /// Create an empty multi-scene manager.
    pub fn new() -> Self {
        let mut world = World::new();
        let root_entity = world.spawn();
        world.add_component(root_entity, Children::new());
        world.add_component(root_entity, Transform::default());
        world.add_component(root_entity, GlobalTransform::default());

        let mut entity_to_handle = HashMap::new();
        let mut handle_to_entity = HashMap::new();
        let root_handle = GameObjectHandle::new(0, 0);
        entity_to_handle.insert(root_entity.index(), root_handle);
        handle_to_entity.insert(0, root_entity);

        let root = SceneNode::new(root_handle);
        Self {
            world,
            scenes: HashMap::new(),
            root,
            entity_to_handle,
            handle_to_entity,
            next_handle_index: 1,
        }
    }

    /// Return the global root node.
    pub fn root(&self) -> SceneNode {
        self.root
    }

    /// Resolve a [`GameObjectHandle`] to its underlying ECS [`Entity`].
    fn resolve_entity_from_handle(&self, handle: GameObjectHandle) -> Entity {
        *self
            .handle_to_entity
            .get(&handle.index())
            .expect("SceneNode handle has no mapped Entity")
    }

    /// Create a [`GameObjectHandle`] for a newly spawned [`Entity`].
    fn create_handle_for_entity(&mut self, entity: Entity) -> GameObjectHandle {
        let handle_index = self.next_handle_index;
        self.next_handle_index += 1;
        let handle = GameObjectHandle::new(handle_index, 0);
        self.entity_to_handle.insert(entity.index(), handle);
        self.handle_to_entity.insert(handle_index, entity);
        handle
    }

    /// Add a scene to the multi-scene world.
    ///
    /// All entities from `scene_data` are spawned into the internal [`World`]
    /// with remapped IDs and parented under a scene-specific root node.
    pub fn add_scene(
        &mut self,
        name: &str,
        scene_data: SceneData,
        layers: SceneLayer,
    ) -> Result<(), MultiSceneError> {
        if self.scenes.contains_key(name) {
            return Err(MultiSceneError::AlreadyLoaded(name.to_string()));
        }

        // Create a root node for this scene
        let scene_root_entity = self.world.spawn();
        self.world.add_component(scene_root_entity, Children::new());
        self.world
            .add_component(scene_root_entity, Transform::default());
        self.world
            .add_component(scene_root_entity, GlobalTransform::default());
        let _scene_root_handle = self.create_handle_for_entity(scene_root_entity);

        // Attach scene root to global root
        let root_entity = self.resolve_entity_from_handle(self.root.gameobject());
        self.world
            .add_component(scene_root_entity, Parent(root_entity));
        if let Some(children) = self.world.get_mut::<Children>(root_entity) {
            children.0.push(scene_root_entity);
        }

        // Spawn all entities and build the ID map
        let mut entity_map: HashMap<u64, Entity> = HashMap::new();

        // First pass: spawn entities
        for entity_data in &scene_data.entities {
            let new_entity = self.world.spawn();
            self.world.add_component(new_entity, Transform::default());
            self.world
                .add_component(new_entity, GlobalTransform::default());
            self.world.add_component(new_entity, Children::new());
            entity_map.insert(entity_data.id, new_entity);
        }

        // Second pass: set up hierarchy and transforms
        for entity_data in &scene_data.entities {
            let new_entity = entity_map[&entity_data.id];

            // Apply transform
            if let Some(t) = self.world.get_mut::<Transform>(new_entity) {
                t.translation = engine_math::Vec3::new(
                    entity_data.transform.translation[0],
                    entity_data.transform.translation[1],
                    entity_data.transform.translation[2],
                );
                t.rotation = engine_math::Quat::from_xyzw(
                    entity_data.transform.rotation[0],
                    entity_data.transform.rotation[1],
                    entity_data.transform.rotation[2],
                    entity_data.transform.rotation[3],
                );
                t.scale = engine_math::Vec3::new(
                    entity_data.transform.scale[0],
                    entity_data.transform.scale[1],
                    entity_data.transform.scale[2],
                );
            }

            // Set up parent-child relationships
            if let Some(parent_id) = entity_data.parent {
                if let Some(&parent_entity) = entity_map.get(&parent_id) {
                    self.world.add_component(new_entity, Parent(parent_entity));
                    if let Some(children) = self.world.get_mut::<Children>(parent_entity) {
                        children.0.push(new_entity);
                    }
                }
            } else {
                // Root-level entity: parent to scene root
                self.world
                    .add_component(new_entity, Parent(scene_root_entity));
                if let Some(children) = self.world.get_mut::<Children>(scene_root_entity) {
                    children.0.push(new_entity);
                }
            }
        }

        self.scenes.insert(
            name.to_string(),
            LoadedScene {
                name: name.to_string(),
                layers,
                entity_map,
                active: true,
            },
        );

        Ok(())
    }

    /// Remove a scene, despawning all its entities from the world.
    pub fn remove_scene(&mut self, name: &str) -> Result<(), MultiSceneError> {
        let loaded = self
            .scenes
            .remove(name)
            .ok_or_else(|| MultiSceneError::NotLoaded(name.to_string()))?;

        // Despawn all mapped entities
        for entity in loaded.entity_map.values() {
            self.world.despawn(*entity);
        }

        Ok(())
    }

    /// Check if a scene is loaded.
    pub fn has_scene(&self, name: &str) -> bool {
        self.scenes.contains_key(name)
    }

    /// Get metadata for a loaded scene.
    pub fn get_scene(&self, name: &str) -> Option<&LoadedScene> {
        self.scenes.get(name)
    }

    /// Return names of all loaded scenes.
    pub fn loaded_scenes(&self) -> Vec<&str> {
        self.scenes.keys().map(|s| s.as_str()).collect()
    }

    /// Activate or deactivate a scene.
    ///
    /// Deactivated scenes remain loaded but are excluded from queries.
    pub fn set_scene_active(&mut self, name: &str, active: bool) -> Result<(), MultiSceneError> {
        let loaded = self
            .scenes
            .get_mut(name)
            .ok_or_else(|| MultiSceneError::NotLoaded(name.to_string()))?;
        loaded.active = active;
        Ok(())
    }

    /// Get the merged [`Entity`] for an entity in a specific scene.
    pub fn resolve_entity(&self, scene: &str, original_id: u64) -> Option<Entity> {
        self.scenes
            .get(scene)?
            .entity_map
            .get(&original_id)
            .copied()
    }

    /// Get a shared reference to the internal ECS world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Get an exclusive reference to the internal ECS world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get the layers for a loaded scene.
    pub fn scene_layers(&self, name: &str) -> Option<SceneLayer> {
        self.scenes.get(name).map(|s| s.layers)
    }

    /// Return all scenes matching a layer mask.
    pub fn scenes_with_layer(&self, layer: SceneLayer) -> Vec<&LoadedScene> {
        self.scenes
            .values()
            .filter(|s| s.layers.contains(layer) && s.active)
            .collect()
    }

    /// Recompute all [`GlobalTransform`]s across all loaded scenes.
    pub fn sync_transforms(&mut self) {
        let root_entity = self.resolve_entity_from_handle(self.root.gameobject());
        let mut stack = vec![(root_entity, engine_math::Mat4::IDENTITY)];
        while let Some((entity, parent_global)) = stack.pop() {
            let local_matrix = self
                .world
                .get::<Transform>(entity)
                .map(|t| t.to_matrix())
                .unwrap_or(engine_math::Mat4::IDENTITY);
            let global = parent_global * local_matrix;
            if let Some(gt) = self.world.get_mut::<GlobalTransform>(entity) {
                gt.0 = global;
            }
            if let Some(children) = self.world.get::<Children>(entity) {
                for child in children.0.iter().rev() {
                    stack.push((*child, global));
                }
            }
        }
    }

    /// Merge all active scenes into a single [`SceneData`] for serialization.
    ///
    /// Entity IDs are renumbered sequentially. The resulting scene captures
    /// the combined state of all loaded scenes.
    pub fn merge_to_scene_data(&self) -> SceneData {
        let mut merged = SceneData::new("merged");
        let mut next_id: u64 = 1;

        for loaded in self.scenes.values().filter(|s| s.active) {
            for (&orig_id, &entity) in &loaded.entity_map {
                let transform = self
                    .world
                    .get::<Transform>(entity)
                    .cloned()
                    .unwrap_or_default();

                let parent = self.world.get::<Parent>(entity).and_then(|p| {
                    // Find the orig_id that maps to this parent entity
                    loaded.entity_map.iter().find_map(
                        |(&oid, &e)| {
                            if e == p.0 { Some(oid) } else { None }
                        },
                    )
                });

                let children: Vec<u64> = self
                    .world
                    .get::<Children>(entity)
                    .map(|c| {
                        c.0.iter()
                            .filter_map(|child| {
                                loaded.entity_map.iter().find_map(|(&oid, &e)| {
                                    if e == *child { Some(oid) } else { None }
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let mut scene_entity =
                    SceneEntityData::new(next_id, format!("{}_{}", loaded.name, orig_id));
                scene_entity.transform.translation = [
                    transform.translation.x,
                    transform.translation.y,
                    transform.translation.z,
                ];
                scene_entity.transform.rotation = [
                    transform.rotation.x,
                    transform.rotation.y,
                    transform.rotation.z,
                    transform.rotation.w,
                ];
                scene_entity.transform.scale =
                    [transform.scale.x, transform.scale.y, transform.scale.z];
                scene_entity.children = children;
                scene_entity.parent = parent;

                merged.add_entity(scene_entity);
                next_id += 1;
            }
        }

        merged
    }
}

impl Default for MultiSceneManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::SceneData;

    fn make_scene(name: &str, entity_count: u64) -> SceneData {
        let mut scene = SceneData::new(name);
        for i in 1..=entity_count {
            let mut e = SceneEntityData::new(i, format!("Entity_{}", i));
            e.transform.translation = [i as f32, 0.0, 0.0];
            scene.add_entity(e);
        }
        scene
    }

    #[test]
    fn test_add_and_remove_scene() {
        let mut mgr = MultiSceneManager::new();
        let scene = make_scene("test", 3);
        mgr.add_scene("test", scene, SceneLayer::DEFAULT).unwrap();

        assert!(mgr.has_scene("test"));
        assert_eq!(mgr.loaded_scenes().len(), 1);

        mgr.remove_scene("test").unwrap();
        assert!(!mgr.has_scene("test"));
        assert_eq!(mgr.loaded_scenes().len(), 0);
    }

    #[test]
    fn test_duplicate_scene_name_error() {
        let mut mgr = MultiSceneManager::new();
        let scene = make_scene("dup", 1);
        mgr.add_scene("dup", scene.clone(), SceneLayer::DEFAULT)
            .unwrap();
        let result = mgr.add_scene("dup", scene, SceneLayer::DEFAULT);
        assert!(matches!(result, Err(MultiSceneError::AlreadyLoaded(_))));
    }

    #[test]
    fn test_remove_nonexistent_scene_error() {
        let mut mgr = MultiSceneManager::new();
        let result = mgr.remove_scene("ghost");
        assert!(matches!(result, Err(MultiSceneError::NotLoaded(_))));
    }

    #[test]
    fn test_two_scenes_entity_no_conflict() {
        let mut mgr = MultiSceneManager::new();

        // Both scenes have entity ID 1
        let scene_a = make_scene("scene_a", 2);
        let scene_b = make_scene("scene_b", 2);

        mgr.add_scene("scene_a", scene_a, SceneLayer::DEFAULT)
            .unwrap();
        mgr.add_scene("scene_b", scene_b, SceneLayer::GAMEPLAY)
            .unwrap();

        // Both should resolve entity 1 to different merged entities
        let e_a = mgr.resolve_entity("scene_a", 1).unwrap();
        let e_b = mgr.resolve_entity("scene_b", 1).unwrap();
        assert_ne!(e_a, e_b, "cross-scene entity IDs must not conflict");

        // Verify both entities exist and have the expected transform
        let t_a = mgr.world().get::<Transform>(e_a).unwrap();
        let t_b = mgr.world().get::<Transform>(e_b).unwrap();
        assert_eq!(t_a.translation.x, 1.0);
        assert_eq!(t_b.translation.x, 1.0);
    }

    #[test]
    fn test_scene_layers() {
        let mut mgr = MultiSceneManager::new();
        let scene = make_scene("layered", 1);
        let layers = SceneLayer::DEFAULT | SceneLayer::ENVIRONMENT;
        mgr.add_scene("layered", scene, layers).unwrap();

        assert_eq!(
            mgr.scene_layers("layered"),
            Some(SceneLayer::DEFAULT | SceneLayer::ENVIRONMENT)
        );
    }

    #[test]
    fn test_scenes_with_layer_filter() {
        let mut mgr = MultiSceneManager::new();
        mgr.add_scene(
            "a",
            make_scene("a", 1),
            SceneLayer::DEFAULT | SceneLayer::ENVIRONMENT,
        )
        .unwrap();
        mgr.add_scene("b", make_scene("b", 1), SceneLayer::GAMEPLAY)
            .unwrap();
        mgr.add_scene(
            "c",
            make_scene("c", 1),
            SceneLayer::DEFAULT | SceneLayer::GAMEPLAY,
        )
        .unwrap();

        let default_scenes = mgr.scenes_with_layer(SceneLayer::DEFAULT);
        assert_eq!(default_scenes.len(), 2);

        let gameplay_scenes = mgr.scenes_with_layer(SceneLayer::GAMEPLAY);
        assert_eq!(gameplay_scenes.len(), 2);
    }

    #[test]
    fn test_activate_deactivate_scene() {
        let mut mgr = MultiSceneManager::new();
        mgr.add_scene("s", make_scene("s", 1), SceneLayer::DEFAULT)
            .unwrap();

        mgr.set_scene_active("s", false).unwrap();
        let active_scenes: Vec<_> = mgr
            .scenes
            .values()
            .filter(|s| s.active)
            .map(|s| s.name.as_str())
            .collect();
        assert!(active_scenes.is_empty());

        mgr.set_scene_active("s", true).unwrap();
        let active_scenes: Vec<_> = mgr
            .scenes
            .values()
            .filter(|s| s.active)
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(active_scenes, vec!["s"]);
    }

    #[test]
    fn test_set_active_nonexistent_error() {
        let mut mgr = MultiSceneManager::new();
        let result = mgr.set_scene_active("nope", true);
        assert!(matches!(result, Err(MultiSceneError::NotLoaded(_))));
    }

    #[test]
    fn test_merge_to_scene_data() {
        let mut mgr = MultiSceneManager::new();
        mgr.add_scene("a", make_scene("a", 2), SceneLayer::DEFAULT)
            .unwrap();
        mgr.add_scene("b", make_scene("b", 3), SceneLayer::GAMEPLAY)
            .unwrap();

        let merged = mgr.merge_to_scene_data();
        assert_eq!(merged.name, "merged");
        assert_eq!(merged.entities.len(), 5);
    }

    #[test]
    fn test_sync_transforms() {
        let mut mgr = MultiSceneManager::new();
        let mut scene = SceneData::new("t");
        let mut e = SceneEntityData::new(1, "Box");
        e.transform.translation = [5.0, 0.0, 0.0];
        scene.add_entity(e);
        mgr.add_scene("t", scene, SceneLayer::DEFAULT).unwrap();

        mgr.sync_transforms();

        let entity = mgr.resolve_entity("t", 1).unwrap();
        let gt = mgr.world().get::<GlobalTransform>(entity).unwrap();
        // The global transform should incorporate the translation
        assert_ne!(gt.0, engine_math::Mat4::IDENTITY);
    }

    #[test]
    fn test_parent_child_hierarchy() {
        let mut mgr = MultiSceneManager::new();
        let mut scene = SceneData::new("hier");

        let mut parent = SceneEntityData::new(1, "Parent");
        parent.children = vec![2];

        let mut child = SceneEntityData::new(2, "Child");
        child.parent = Some(1);
        child.transform.translation = [0.0, 1.0, 0.0];

        scene.add_entity(parent);
        scene.add_entity(child);

        mgr.add_scene("hier", scene, SceneLayer::DEFAULT).unwrap();

        let parent_entity = mgr.resolve_entity("hier", 1).unwrap();
        let child_entity = mgr.resolve_entity("hier", 2).unwrap();

        // Check parent component
        let p = mgr.world().get::<Parent>(child_entity).unwrap();
        assert_eq!(p.0, parent_entity);

        // Check children component
        let c = mgr.world().get::<Children>(parent_entity).unwrap();
        assert!(c.0.contains(&child_entity));
    }
}
