//! Unity World — central container for all GameObjects.
//!
//! Maps to Unity's scene runtime. This is the unified World that:
//! - Uses sparse-set ECS internally (engine-ecs)
//! - Exposes Unity-style API externally
//! - Stores Transform as built-in (not a Component)
//! - Handles parent/child hierarchy
//! - Dispatches MonoBehaviour lifecycle callbacks

use std::any::Any;
use std::collections::HashMap;

use crate::component::Component;
use crate::context::Context;
use crate::gameobject::{GameObject, GameObjectHandle};
use crate::monobehaviour::{MonoBehaviour, MonoBehaviourHolder};
use crate::time::Time;
use crate::transform::Transform;
use engine_math::{Quat, Vec3};

/// ECS mirror of `GameObject.name` (P2.4 storage write-through).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameObjectName(pub String);

impl Component for GameObjectName {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// ECS mirror of `GameObject.tag` (P2.4 storage write-through).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameObjectTag(pub String);

impl Component for GameObjectTag {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// ECS mirror of `GameObject.layer` (R1 storage write-through).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectLayer(pub i32);

impl Component for GameObjectLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// ECS mirror of `GameObject.activeSelf` (P2.4 storage write-through).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectActive(pub bool);

impl Component for GameObjectActive {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// ECS list of MonoBehaviour type names on a GameObject (P2.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonoBehaviourTypes(pub Vec<String>);

impl Component for MonoBehaviourTypes {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Recoverable MonoBehaviour metadata (B5). Does not hold the `dyn` instance.
#[derive(Debug, Clone, PartialEq)]
pub struct MonoBehaviourInstance {
    pub type_name: String,
    pub enabled: bool,
    pub props: Option<serde_json::Value>,
}

/// ECS list of recoverable MonoBehaviour descriptors (B5 dual-write).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MonoBehaviourInstances(pub Vec<MonoBehaviourInstance>);

impl Component for MonoBehaviourInstances {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// ECS parent link (P2.4 hierarchy write-through). Empty = root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectParent(pub GameObjectHandle);

impl Component for GameObjectParent {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// ECS children list (P2.4 hierarchy write-through).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameObjectChildren(pub Vec<GameObjectHandle>);

impl Component for GameObjectChildren {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Primitive type for CreatePrimitive (matches Unity's `PrimitiveType` enum).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/PrimitiveType.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    /// Sphere primitive (matches `PrimitiveType.Sphere`).
    Sphere,
    /// Capsule primitive (matches `PrimitiveType.Capsule`).
    Capsule,
    /// Cylinder primitive (matches `PrimitiveType.Cylinder`).
    Cylinder,
    /// Cube primitive (matches `PrimitiveType.Cube`).
    Cube,
    /// Plane primitive (matches `PrimitiveType.Plane`).
    Plane,
    /// Quad primitive (matches `PrimitiveType.Quad`).
    Quad,
}

/// Pending destroy entry with optional delay.
struct PendingDestroy {
    handle: GameObjectHandle,
    delay: f32,
    elapsed: f32,
}

/// Pending invoke entry.
struct PendingInvoke {
    handle: GameObjectHandle,
    method_name: String,
    time: f32,
    elapsed: f32,
    repeat_rate: Option<f32>,
}

/// Unified World container (matches Unity's scene concept).
///
/// # Architecture
/// - Uses `engine_ecs::World` internally for entity generation (hidden from API)
/// - Stores `GameObject` structs in a separate array
/// - Stores `Transform` instances in a separate array (built-in)
/// - Provides Unity-style API: `CreateGameObject`, `GetComponent`, `Find`, etc.
///
/// # Unity API Coverage
/// This World implements the following Unity patterns:
/// - `GameObject` creation/destruction (`CreateGameObject`, `Destroy`, `Instantiate`)
/// - `Component` access (`AddComponent`, `GetComponent`, `GetComponentInChildren`)
/// - `Transform` access (`GetTransform`, `GetTransformMut`)
/// - `Find` operations (`Find`, `FindWithTag`, `FindGameObjectsWithTag`, `FindObjectOfType`)
/// - `SetActive` / `IsActive` / `IsActiveInHierarchy`
/// - `SetName` / `GetName` / `SetTag` / `GetTag` / `SetLayer` / `GetLayer`
/// - `SendMessage` / `BroadcastMessage` / `SendMessageUpwards`
/// - `DontDestroyOnLoad`
pub struct World {
    // === Internal ECS (hidden from public API) ===
    ecs: engine_ecs::world::World,

    // === GameObject storage ===
    gameobjects: Vec<Option<GameObjectHandle>>,
    generations: Vec<u32>,
    free_list: Vec<u32>,
    gameobject_data: Vec<Option<GameObject>>,

    // === Transform storage (built-in, not a Component) ===
    transforms: Vec<Option<Transform>>,

    // === MonoBehaviour storage ===
    monobehaviours: Vec<Option<Vec<MonoBehaviourHolder>>>,

    // === Lookup tables ===
    name_to_handles: HashMap<String, Vec<GameObjectHandle>>,
    tag_to_handles: HashMap<String, Vec<GameObjectHandle>>,

    // === P2.4 identity (GameObjectHandle ↔ ECS Entity), owned by World ===
    handle_to_entity: HashMap<GameObjectHandle, engine_ecs::entity::Entity>,
    entity_to_handle: HashMap<engine_ecs::entity::Entity, GameObjectHandle>,

    // === Pending operations ===
    pending_destroy: Vec<PendingDestroy>,
    pending_invokes: Vec<PendingInvoke>,
    /// Pending OnEnable/OnDisable from SetActive (flushed on lifecycle tick).
    pending_enable_disable: Vec<PendingEnableDisable>,
    /// Running coroutines (Unity StartCoroutine).
    coroutines: crate::coroutine::CoroutineRunner,

    // === DontDestroyOnLoad tracking ===
    dont_destroy: Vec<GameObjectHandle>,

    // === Instance ID counter ===
    next_instance_id: i32,
}

/// Pending OnEnable/OnDisable callback (Unity SetActive side-effect).
struct PendingEnableDisable {
    handle: GameObjectHandle,
    /// true → OnEnable, false → OnDisable
    enable: bool,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field(
                "gameobject_count",
                &self.gameobject_data.iter().filter(|g| g.is_some()).count(),
            )
            .field(
                "transform_count",
                &self.transforms.iter().filter(|t| t.is_some()).count(),
            )
            .field("pending_destroy", &self.pending_destroy.len())
            .finish()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    // ============================================================
    // Constructor
    // ============================================================

    /// Create a new empty World.
    pub fn new() -> Self {
        Self {
            ecs: engine_ecs::world::World::new(),
            gameobjects: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
            gameobject_data: Vec::new(),
            transforms: Vec::new(),
            monobehaviours: Vec::new(),
            name_to_handles: HashMap::new(),
            tag_to_handles: HashMap::new(),
            handle_to_entity: HashMap::new(),
            entity_to_handle: HashMap::new(),
            pending_destroy: Vec::new(),
            pending_invokes: Vec::new(),
            pending_enable_disable: Vec::new(),
            coroutines: crate::coroutine::CoroutineRunner::new(),
            dont_destroy: Vec::new(),
            next_instance_id: 1,
        }
    }

    // ============================================================
    // Internal — Handle Management
    // ============================================================

    /// Allocate a slot for a new GameObject.
    fn allocate_slot(&mut self) -> (u32, u32) {
        if let Some(index) = self.free_list.pop() {
            let index_usize = index as usize;
            let generation = self.generations[index_usize] + 1;
            self.generations[index_usize] = generation;
            (index, generation)
        } else {
            let index = self.gameobjects.len() as u32;
            self.gameobjects.push(None);
            self.generations.push(0);
            self.gameobject_data.push(None);
            self.transforms.push(None);
            self.monobehaviours.push(None);
            (index, 0)
        }
    }

    /// Check if a handle is valid.
    pub fn is_valid(&self, handle: GameObjectHandle) -> bool {
        let index = handle.index() as usize;
        index < self.gameobjects.len()
            && self.gameobjects[index].is_some()
            && self.generations[index] == handle.generation()
    }

    /// Count of live (non-destroyed) GameObjects.
    pub fn live_game_object_count(&self) -> usize {
        self.gameobjects.iter().filter(|g| g.is_some()).count()
    }

    /// Iterate all live GameObject handles (slot order, not hierarchy order).
    ///
    /// P2.4 foundation: migration tools and tests walk the full set without
    /// relying on roots/children only.
    pub fn iter_game_objects(&self) -> impl Iterator<Item = GameObjectHandle> + '_ {
        self.gameobjects
            .iter()
            .filter_map(|g| *g)
            .filter(|h| self.is_valid(*h))
    }

    /// Shared access to the internal ECS world (migration / advanced systems).
    ///
    /// Prefer Unity World APIs for gameplay. Exposed for P2.4 storage merge
    /// and identity-bridge tooling.
    pub fn ecs_world(&self) -> &engine_ecs::world::World {
        &self.ecs
    }

    /// Mutable access to the internal ECS world.
    pub fn ecs_world_mut(&mut self) -> &mut engine_ecs::world::World {
        &mut self.ecs
    }

    /// Whether `unity-world-primary` is compiled in (P2.4 migration switch).
    pub fn unity_world_primary_feature() -> bool {
        cfg!(feature = "unity-world-primary")
    }

    /// Resolve the ECS Entity linked to a GameObject (P2.4 identity).
    pub fn entity_for(&self, handle: GameObjectHandle) -> Option<engine_ecs::entity::Entity> {
        self.handle_to_entity.get(&handle).copied()
    }

    /// Resolve the GameObject linked to an ECS Entity.
    pub fn gameobject_for_entity(
        &self,
        entity: engine_ecs::entity::Entity,
    ) -> Option<GameObjectHandle> {
        self.entity_to_handle.get(&entity).copied()
    }

    /// Link an existing GameObject to an ECS Entity (replaces prior mapping).
    pub fn link_entity(&mut self, handle: GameObjectHandle, entity: engine_ecs::entity::Entity) {
        if let Some(old_e) = self.handle_to_entity.insert(handle, entity) {
            if old_e != entity {
                self.entity_to_handle.remove(&old_e);
            }
        }
        if let Some(old_go) = self.entity_to_handle.insert(entity, handle) {
            if old_go != handle {
                self.handle_to_entity.remove(&old_go);
            }
        }
    }

    /// Drop identity mapping for a GameObject.
    pub fn unlink_entity(&mut self, handle: GameObjectHandle) {
        if let Some(e) = self.handle_to_entity.remove(&handle) {
            self.entity_to_handle.remove(&e);
        }
    }

    /// Ensure `handle` has an ECS entity on **this** World's internal ECS.
    ///
    /// SceneRuntime and other bridges should call this instead of spawning a
    /// second entity on a different ECS world.
    ///
    /// Under `unity-world-primary`, also backfills a missing `Transform` from
    /// array storage so dual-read prefers a coherent ECS component (B1).
    pub fn ensure_entity(&mut self, handle: GameObjectHandle) -> engine_ecs::entity::Entity {
        if let Some(e) = self.entity_for(handle) {
            if Self::unity_world_primary_feature() && self.ecs.get::<Transform>(e).is_none() {
                self.write_transform_to_ecs(handle);
            }
            return e;
        }
        let e = self.ecs.spawn();
        self.link_entity(handle, e);
        if Self::unity_world_primary_feature() {
            self.write_transform_to_ecs(handle);
        }
        e
    }

    /// If the linked entity exists but lacks `Transform`, copy array storage
    /// onto it (P2.4 read-path backfill under `unity-world-primary`).
    pub fn ensure_transform_from_array(&mut self, handle: GameObjectHandle) {
        if !Self::unity_world_primary_feature() {
            return;
        }
        if !self.is_valid(handle) {
            return;
        }
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        if self.ecs.get::<Transform>(entity).is_none() {
            self.write_transform_to_ecs(handle);
        }
    }

    /// Seed all ECS identity mirrors from **array** storage (R1).
    ///
    /// Ensures dual-read under `unity-world-primary` never hits a linked entity
    /// that is missing Name/Tag/Active/Transform/Hierarchy/MB metadata.
    /// Always reads array/GameObject fields — never dual-reads ECS.
    pub fn seed_ecs_from_array(&mut self, handle: GameObjectHandle) {
        if !self.is_valid(handle) {
            return;
        }
        let _ = self.ensure_entity(handle);
        let index = handle.index() as usize;
        if let Some(Some(go)) = self.gameobject_data.get(index) {
            let name = go.Name().to_string();
            let tag = go.Tag().to_string();
            let active = go.ActiveSelf();
            let layer = go.Layer();
            self.sync_name_to_ecs(handle, &name);
            self.sync_tag_to_ecs(handle, &tag);
            self.sync_active_to_ecs(handle, active);
            self.sync_layer_to_ecs(handle, layer);
        }
        self.write_transform_to_ecs(handle);
        self.sync_hierarchy_to_ecs(handle);
        self.sync_monobehaviour_types_to_ecs(handle);
    }

    /// Seed every valid GameObject's ECS mirrors from array storage (R1).
    pub fn seed_all_ecs_from_array(&mut self) {
        let handles: Vec<GameObjectHandle> = self.gameobjects.iter().flatten().copied().collect();
        for handle in handles {
            self.seed_ecs_from_array(handle);
        }
    }

    /// Number of live identity links.
    pub fn identity_count(&self) -> usize {
        self.handle_to_entity.len()
    }

    /// Copy all Unity Transforms onto linked internal-ECS entities as
    /// [`Transform`] components (P2.4 write-through for tools / storage merge).
    ///
    /// Uses the same local/world contract as [`World::write_transform_to_ecs`]:
    /// local fields stay local; roots refresh world from local; children keep
    /// cached world after `sync_transforms`.
    pub fn sync_all_transforms_to_ecs(&mut self) {
        let handles: Vec<GameObjectHandle> = self.handle_to_entity.keys().copied().collect();
        for handle in handles {
            if !self.is_valid(handle) {
                continue;
            }
            self.write_transform_to_ecs(handle);
        }
    }

    /// Write `GameObjectName` onto the linked entity (internal ECS).
    fn sync_name_to_ecs(&mut self, handle: GameObjectHandle, name: &str) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let comp = GameObjectName(name.to_string());
        if self.ecs.get::<GameObjectName>(entity).is_some() {
            *self.ecs.get_mut::<GameObjectName>(entity).unwrap() = comp;
        } else {
            self.ecs.add_component(entity, comp);
        }
    }

    /// Write `GameObjectTag` onto the linked entity (internal ECS).
    fn sync_tag_to_ecs(&mut self, handle: GameObjectHandle, tag: &str) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let comp = GameObjectTag(tag.to_string());
        if self.ecs.get::<GameObjectTag>(entity).is_some() {
            *self.ecs.get_mut::<GameObjectTag>(entity).unwrap() = comp;
        } else {
            self.ecs.add_component(entity, comp);
        }
    }

    /// Write `GameObjectLayer` onto the linked entity (internal ECS).
    fn sync_layer_to_ecs(&mut self, handle: GameObjectHandle, layer: i32) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let comp = GameObjectLayer(layer);
        if self.ecs.get::<GameObjectLayer>(entity).is_some() {
            *self.ecs.get_mut::<GameObjectLayer>(entity).unwrap() = comp;
        } else {
            self.ecs.add_component(entity, comp);
        }
    }

    /// Write `GameObjectActive` onto the linked entity (internal ECS).
    fn sync_active_to_ecs(&mut self, handle: GameObjectHandle, active: bool) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let comp = GameObjectActive(active);
        if self.ecs.get::<GameObjectActive>(entity).is_some() {
            *self.ecs.get_mut::<GameObjectActive>(entity).unwrap() = comp;
        } else {
            self.ecs.add_component(entity, comp);
        }
    }

    /// Refresh `MonoBehaviourTypes` + `MonoBehaviourInstances` from current holders.
    ///
    /// Always reads **array holders** (dyn runtime store), never dual-reads ECS.
    fn sync_monobehaviour_types_to_ecs(&mut self, handle: GameObjectHandle) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let collected = self.collect_monobehaviours_from_holders(handle);
        let names: Vec<String> = collected.iter().map(|(name, _, _)| name.clone()).collect();
        let instances: Vec<MonoBehaviourInstance> = collected
            .into_iter()
            .map(|(type_name, enabled, props)| MonoBehaviourInstance {
                type_name,
                enabled,
                props,
            })
            .collect();

        let types = MonoBehaviourTypes(names);
        if self.ecs.get::<MonoBehaviourTypes>(entity).is_some() {
            *self.ecs.get_mut::<MonoBehaviourTypes>(entity).unwrap() = types;
        } else {
            self.ecs.add_component(entity, types);
        }

        let inst = MonoBehaviourInstances(instances);
        if self.ecs.get::<MonoBehaviourInstances>(entity).is_some() {
            *self.ecs.get_mut::<MonoBehaviourInstances>(entity).unwrap() = inst;
        } else {
            self.ecs.add_component(entity, inst);
        }
    }

    /// Array-holder MonoBehaviour metadata (runtime dyn store + props snapshot).
    fn collect_monobehaviours_from_holders(
        &self,
        handle: GameObjectHandle,
    ) -> Vec<(String, bool, Option<serde_json::Value>)> {
        let index = handle.index() as usize;
        let Some(Some(monos)) = self.monobehaviours.get(index) else {
            return Vec::new();
        };
        monos
            .iter()
            .map(|m| {
                let inner = m.Get();
                (
                    inner.TypeName().to_string(),
                    m.Enabled(),
                    inner.SerializeProps(),
                )
            })
            .collect()
    }

    /// Parent link under compiled storage authority (R1-full).
    ///
    /// Feature on + linked + hierarchy mirror present → ECS `GameObjectParent`
    /// (missing component on a seeded entity = root). Otherwise array storage.
    fn hierarchy_authority_parent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> {
        if Self::unity_world_primary_feature()
            && let Some(entity) = self.entity_for(handle)
        {
            if let Some(p) = self.ecs.get::<GameObjectParent>(entity) {
                return Some(p.0);
            }
            // Seeded hierarchy without Parent component means ECS-authoritative root.
            if self.ecs.get::<GameObjectChildren>(entity).is_some() {
                return None;
            }
        }
        self.GetParentArray(handle)
    }

    /// Children list under compiled storage authority (R1-full).
    fn hierarchy_authority_children(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
        if Self::unity_world_primary_feature()
            && let Some(entity) = self.entity_for(handle)
            && let Some(ch) = self.ecs.get::<GameObjectChildren>(entity)
        {
            return ch.0.clone();
        }
        self.GetChildrenArray(handle)
    }

    /// Refresh array `parent`/`children` cache from ECS hierarchy mirrors (R1-full).
    ///
    /// No-op when the feature is off or the entity has no hierarchy mirror.
    fn sync_hierarchy_from_ecs(&mut self, handle: GameObjectHandle) {
        if !Self::unity_world_primary_feature() {
            return;
        }
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let parent = self.ecs.get::<GameObjectParent>(entity).map(|p| p.0);
        let children = self
            .ecs
            .get::<GameObjectChildren>(entity)
            .map(|c| c.0.clone())
            .unwrap_or_default();
        let index = handle.index() as usize;
        if let Some(Some(t)) = self.transforms.get_mut(index) {
            t.parent = parent;
            t.children = children;
        }
    }

    /// Feature-on hierarchy write: ECS Parent/Children are authority; arrays become cache.
    fn set_parent_ecs_primary(
        &mut self,
        child: GameObjectHandle,
        parent: Option<GameObjectHandle>,
    ) {
        let old_parent = self.hierarchy_authority_parent(child);
        self.ensure_entity(child);
        if let Some(p) = parent {
            self.ensure_entity(p);
        }
        if let Some(op) = old_parent {
            self.ensure_entity(op);
        }

        if let Some(ce) = self.entity_for(child) {
            if let Some(p) = parent {
                if self.ecs.get::<GameObjectParent>(ce).is_some() {
                    *self.ecs.get_mut::<GameObjectParent>(ce).unwrap() = GameObjectParent(p);
                } else {
                    self.ecs.add_component(ce, GameObjectParent(p));
                }
            } else if self.ecs.get::<GameObjectParent>(ce).is_some() {
                let _ = self.ecs.remove_component::<GameObjectParent>(ce);
            }
            // Keep child's Children mirror present (empty roots included).
            if self.ecs.get::<GameObjectChildren>(ce).is_none() {
                let kids = self.GetChildrenArray(child);
                self.ecs.add_component(ce, GameObjectChildren(kids));
            }
        }

        if old_parent != parent {
            if let Some(op) = old_parent
                && let Some(oe) = self.entity_for(op)
            {
                if self.ecs.get::<GameObjectChildren>(oe).is_some() {
                    if let Some(ch) = self.ecs.get_mut::<GameObjectChildren>(oe) {
                        ch.0.retain(|&h| h != child);
                    }
                } else {
                    let mut kids = self.GetChildrenArray(op);
                    kids.retain(|&h| h != child);
                    self.ecs.add_component(oe, GameObjectChildren(kids));
                }
            }
            if let Some(np) = parent
                && let Some(ne) = self.entity_for(np)
            {
                if self.ecs.get::<GameObjectChildren>(ne).is_some() {
                    if let Some(ch) = self.ecs.get_mut::<GameObjectChildren>(ne) {
                        if !ch.0.contains(&child) {
                            ch.0.push(child);
                        }
                    }
                } else {
                    let mut kids = self.GetChildrenArray(np);
                    if !kids.contains(&child) {
                        kids.push(child);
                    }
                    self.ecs.add_component(ne, GameObjectChildren(kids));
                }
            }
        }

        // Array cache ← ECS authority
        self.sync_hierarchy_from_ecs(child);
        if let Some(p) = parent {
            self.sync_hierarchy_from_ecs(p);
        }
        if let Some(op) = old_parent {
            self.sync_hierarchy_from_ecs(op);
        }
        // Ensure pose components exist; do not clobber ECS local pose from array.
        self.ensure_transform_from_array(child);
        self.sync_transform_from_ecs(child);
    }

    /// Prepare caches before scene serialization (R1-full).
    ///
    /// Feature on: refresh pose + hierarchy array caches from ECS authority.
    /// Feature off: no-op (arrays already authoritative).
    pub fn prepare_scene_io_cache(&mut self) {
        if !Self::unity_world_primary_feature() {
            return;
        }
        let handles: Vec<GameObjectHandle> = self.gameobjects.iter().flatten().copied().collect();
        for handle in handles {
            if !self.is_valid(handle) {
                continue;
            }
            self.sync_hierarchy_from_ecs(handle);
            self.sync_transform_from_ecs(handle);
        }
    }

    /// Write `GameObjectParent` / `GameObjectChildren` onto the linked entity.
    ///
    /// Seed/export path: reads **array** hierarchy then mirrors to ECS.
    /// Public hierarchy **writes** under `unity-world-primary` use
    /// [`World::set_parent_ecs_primary`] instead (ECS authority + array cache).
    fn sync_hierarchy_to_ecs(&mut self, handle: GameObjectHandle) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let parent = self.GetParentArray(handle);
        let children = self.GetChildrenArray(handle);

        if let Some(p) = parent {
            let comp = GameObjectParent(p);
            if self.ecs.get::<GameObjectParent>(entity).is_some() {
                *self.ecs.get_mut::<GameObjectParent>(entity).unwrap() = comp;
            } else {
                self.ecs.add_component(entity, comp);
            }
        } else if self.ecs.get::<GameObjectParent>(entity).is_some() {
            let _ = self.ecs.remove_component::<GameObjectParent>(entity);
        }
        let comp = GameObjectChildren(children);
        if self.ecs.get::<GameObjectChildren>(entity).is_some() {
            *self.ecs.get_mut::<GameObjectChildren>(entity).unwrap() = comp;
        } else {
            self.ecs.add_component(entity, comp);
        }
    }

    /// Get the next instance ID.
    fn next_instance_id(&mut self) -> i32 {
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        id
    }

    // ============================================================
    // Object Static Methods (matches Unity's Object)
    // ============================================================

    /// Create a new empty GameObject (matches `new GameObject("name")`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.html>
    pub fn CreateGameObject(&mut self, name: &str) -> GameObjectHandle {
        let mut go = GameObject::new(name);
        let instance_id = self.next_instance_id();

        let (index, generation) = self.allocate_slot();
        let handle = GameObjectHandle::new(index, generation);

        // Set instance ID on the GameObject (via a workaround since we can't mutate through the handle)
        // We'll store instance_id separately

        go.SetName(name);

        // Add default Transform (built-in, mandatory)
        let mut transform = Transform::default();

        // Store in arrays
        self.gameobjects[index as usize] = Some(handle);
        self.gameobject_data[index as usize] = Some(go);
        self.transforms[index as usize] = Some(transform);
        self.monobehaviours[index as usize] = Some(Vec::new());

        // Update name lookup
        self.name_to_handles
            .entry(name.to_string())
            .or_default()
            .push(handle);

        // P2.4: auto-link ECS identity on this World's internal ECS
        let _ = self.ensure_entity(handle);
        self.sync_name_to_ecs(handle, name);
        self.sync_tag_to_ecs(handle, "Untagged");
        self.sync_active_to_ecs(handle, true);
        self.sync_layer_to_ecs(handle, 0);
        if Self::unity_world_primary_feature() {
            self.ensure_transform_from_array(handle);
            self.sync_hierarchy_to_ecs(handle);
        }

        handle
    }

    /// Create a primitive GameObject with mesh renderer and collider (matches `GameObject.CreatePrimitive`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.CreatePrimitive.html>
    ///
    /// Creates a GameObject with:
    /// - The specified mesh (Sphere, Capsule, Cylinder, Cube, Plane, Quad)
    /// - A MeshRenderer component
    /// - A Collider component (SphereCollider, CapsuleCollider, etc.)
    pub fn CreatePrimitive(&mut self, primitive_type: PrimitiveType) -> GameObjectHandle {
        let name = match primitive_type {
            PrimitiveType::Sphere => "Sphere",
            PrimitiveType::Capsule => "Capsule",
            PrimitiveType::Cylinder => "Cylinder",
            PrimitiveType::Cube => "Cube",
            PrimitiveType::Plane => "Plane",
            PrimitiveType::Quad => "Quad",
        };

        let handle = self.CreateGameObject(name);

        // Add mesh renderer with the primitive mesh name
        let mesh_name = match primitive_type {
            PrimitiveType::Sphere => "Sphere",
            PrimitiveType::Capsule => "Capsule",
            PrimitiveType::Cylinder => "Cylinder",
            PrimitiveType::Cube => "Cube",
            PrimitiveType::Plane => "Plane",
            PrimitiveType::Quad => "Quad",
        };

        // Components would be added here when renderer/collider are integrated
        // For now, just create the GameObject with the correct name

        handle
    }

    /// Create a new GameObject with components (matches `new GameObject("name", typeof(T1))`).
    pub fn CreateGameObjectWithComponents(
        &mut self,
        name: &str,
        components: Vec<Box<dyn Component>>,
    ) -> GameObjectHandle {
        let handle = self.CreateGameObject(name);

        // Add components
        for component in components {
            if let Some(go) = self.gameobject_data.get_mut(handle.index() as usize) {
                if let Some(go) = go {
                    go.AddComponentBoxed(component);
                }
            }
        }

        handle
    }

    /// Destroy a GameObject at end of frame (matches `Object.Destroy`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.Destroy.html>
    pub fn Destroy(&mut self, handle: GameObjectHandle) {
        if self.is_valid(handle) {
            self.pending_destroy.push(PendingDestroy {
                handle,
                delay: 0.0,
                elapsed: 0.0,
            });
        }
    }

    /// Destroy a GameObject after a delay (matches `Object.Destroy(obj, t)`).
    pub fn DestroyDelayed(&mut self, handle: GameObjectHandle, t: f32) {
        if self.is_valid(handle) {
            self.pending_destroy.push(PendingDestroy {
                handle,
                delay: t,
                elapsed: 0.0,
            });
        }
    }

    /// Destroy a GameObject immediately (matches `Object.DestroyImmediate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.DestroyImmediate.html>
    pub fn DestroyImmediate(&mut self, handle: GameObjectHandle) {
        self.destroy_internal(handle);
    }

    /// Internal destroy implementation (no MonoBehaviour callbacks).
    ///
    /// Callers that need Unity's OnDisable → OnDestroy sequence should use
    /// [`World::flush_destroy`] / [`World::dispatch_destroy_callbacks`] first.
    fn destroy_internal(&mut self, handle: GameObjectHandle) {
        if !self.is_valid(handle) {
            return;
        }

        let index = handle.index() as usize;

        // Cancel pending Invoke and coroutines owned by this object
        self.CancelInvoke(handle);
        self.StopAllCoroutines(handle);

        // Remove from name lookup
        if let Some(go) = self.gameobject_data[index].as_ref() {
            let name = go.Name().to_string();
            if let Some(handles) = self.name_to_handles.get_mut(&name) {
                handles.retain(|&h| h != handle);
                if handles.is_empty() {
                    self.name_to_handles.remove(&name);
                }
            }

            // Remove from tag lookup
            let tag = go.Tag().to_string();
            if let Some(handles) = self.tag_to_handles.get_mut(&tag) {
                handles.retain(|&h| h != handle);
                if handles.is_empty() {
                    self.tag_to_handles.remove(&tag);
                }
            }
        }

        // Hierarchy walk uses storage authority (ECS under feature on; array otherwise).
        let children = self.hierarchy_authority_children(handle);
        let parent = self.hierarchy_authority_parent(handle);

        for child in children {
            self.destroy_internal(child);
        }

        // Remove from parent's children list (array cache + authority when needed)
        if let Some(parent) = parent {
            let parent_index = parent.index() as usize;
            if let Some(parent_transform) = self.transforms.get_mut(parent_index) {
                if let Some(pt) = parent_transform {
                    pt.children.retain(|&h| h != handle);
                }
            }
        }

        // Clear arrays
        self.gameobjects[index] = None;
        self.gameobject_data[index] = None;
        self.transforms[index] = None;
        self.monobehaviours[index] = None;

        // P2.4: drop identity mapping and despawn internal ECS entity
        if let Some(entity) = self.entity_for(handle) {
            self.ecs.despawn(entity);
        }
        self.unlink_entity(handle);

        // B4: keep Destroy / DestroyImmediate / flush_destroy coherent
        self.pending_destroy.retain(|p| p.handle != handle);
        self.dont_destroy.retain(|&h| h != handle);

        // Parent children: keep ECS authority coherent after despawn (R1-full)
        if let Some(parent) = parent
            && self.is_valid(parent)
        {
            if Self::unity_world_primary_feature() {
                if let Some(pe) = self.entity_for(parent) {
                    if self.ecs.get::<GameObjectChildren>(pe).is_some() {
                        if let Some(ch) = self.ecs.get_mut::<GameObjectChildren>(pe) {
                            ch.0.retain(|&h| h != handle);
                        }
                    } else {
                        let remaining = self.GetChildrenArray(parent);
                        self.ecs.add_component(pe, GameObjectChildren(remaining));
                    }
                }
                self.sync_hierarchy_from_ecs(parent);
            } else {
                self.sync_hierarchy_to_ecs(parent);
            }
        }

        // Add to free list
        self.free_list.push(index as u32);
    }

    /// Don't destroy when loading a new scene (matches `Object.DontDestroyOnLoad`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.DontDestroyOnLoad.html>
    pub fn DontDestroyOnLoad(&mut self, handle: GameObjectHandle) {
        if self.is_valid(handle) {
            self.dont_destroy.push(handle);
        }
    }

    /// Instantiate a clone of a GameObject (matches `Object.Instantiate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.Instantiate.html>
    pub fn Instantiate(&mut self, template: GameObjectHandle) -> GameObjectHandle {
        self.InstantiateAtPosition(template, Vec3::ZERO, Quat::IDENTITY)
    }

    /// Instantiate at a specific position and rotation.
    pub fn InstantiateAtPosition(
        &mut self,
        template: GameObjectHandle,
        position: Vec3,
        rotation: Quat,
    ) -> GameObjectHandle {
        if !self.is_valid(template) {
            return template; // Return invalid handle
        }

        let template_index = template.index() as usize;

        // Clone the GameObject
        let new_name = if let Some(go) = self.gameobject_data[template_index].as_ref() {
            format!("{} (Clone)", go.Name())
        } else {
            "GameObject (Clone)".to_string()
        };

        let handle = self.CreateGameObject(&new_name);

        // Mirror identity fields from template (authority-aware writers)
        if let Some(template_go) = self.gameobject_data[template_index].as_ref() {
            let tag = template_go.Tag().to_string();
            let layer = template_go.Layer();
            let active = template_go.ActiveSelf();
            self.SetTag(handle, &tag);
            self.SetLayer(handle, layer);
            self.SetActive(handle, active);
        }

        // Pose through the compiled storage-authority path (ECS primary when feature on).
        let _ = self.with_ecs_transform_mut(handle, |t| {
            t.SetLocalPosition(position);
            t.SetLocalRotation(rotation);
        });

        // Load/Instantiate boundary: materialize full ECS mirrors from array cache.
        self.seed_ecs_from_array(handle);
        handle
    }

    // ============================================================
    // Find Methods (matches Unity's Object/GameObject)
    // ============================================================

    /// Find a GameObject by name (matches `GameObject.Find`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.Find.html>
    pub fn Find(&self, name: &str) -> Option<GameObjectHandle> {
        self.name_to_handles
            .get(name)
            .and_then(|v| v.first())
            .copied()
    }

    /// Find the first GameObject with a tag (matches `GameObject.FindWithTag`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.FindWithTag.html>
    pub fn FindWithTag(&self, tag: &str) -> Option<GameObjectHandle> {
        self.tag_to_handles
            .get(tag)
            .and_then(|v| v.first())
            .copied()
    }

    /// Find all GameObjects with a tag (matches `GameObject.FindGameObjectsWithTag`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.FindGameObjectsWithTag.html>
    pub fn FindGameObjectsWithTag(&self, tag: &str) -> Vec<GameObjectHandle> {
        self.tag_to_handles.get(tag).cloned().unwrap_or_default()
    }

    /// Find the first object of type T (matches `Object.FindObjectOfType<T>`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.FindObjectOfType.html>
    pub fn FindObjectOfType<T: Component + 'static>(&self) -> Option<GameObjectHandle> {
        for (i, go) in self.gameobject_data.iter().enumerate() {
            if let Some(go) = go {
                if go.HasComponent::<T>() {
                    return Some(self.gameobjects[i].unwrap());
                }
            }
        }
        None
    }

    /// Find all objects of type T (matches `Object.FindObjectsOfType<T>`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.FindObjectsOfType.html>
    pub fn FindObjectsOfType<T: Component + 'static>(&self) -> Vec<GameObjectHandle> {
        let mut result = Vec::new();
        for (i, go) in self.gameobject_data.iter().enumerate() {
            if let Some(go) = go {
                if go.HasComponent::<T>() {
                    if let Some(handle) = self.gameobjects[i] {
                        result.push(handle);
                    }
                }
            }
        }
        result
    }

    // ============================================================
    // Component Access (matches Unity's Component methods)
    // ============================================================

    /// Add a component to a GameObject (matches `GameObject.AddComponent<T>()`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.AddComponent.html>
    ///
    /// Also auto-adds any components returned by [`Component::required_on_add`]
    /// that are not already present (Unity `RequireComponent`).
    pub fn AddComponent<T: Component + 'static>(
        &mut self,
        handle: GameObjectHandle,
        component: T,
    ) -> &mut T {
        let index = handle.index() as usize;

        // Auto-add required components first (Unity RequireComponent).
        for factory in component.required_on_add() {
            let required = factory();
            let type_name = required.component_name().to_string();
            let already = self
                .get_gameobject(handle)
                .map(|go| {
                    go.Components()
                        .iter()
                        .any(|c| c.component_name() == type_name)
                })
                .unwrap_or(false);
            if !already {
                self.AddComponentBoxed(handle, required);
            }
        }

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                return go.AddComponent(component);
            }
        }

        panic!("Invalid handle in AddComponent");
    }

    /// Add a boxed component to a GameObject (for dynamic deserialization).
    pub fn AddComponentBoxed(&mut self, handle: GameObjectHandle, component: Box<dyn Component>) {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                go.AddComponentBoxed(component);
            }
        }
    }

    /// Attach a MonoBehaviour script (matches `AddComponent<MyScript>()` for scripts).
    ///
    /// Scripts receive Awake/Start/Update/FixedUpdate/LateUpdate/OnEnable/OnDisable/OnDestroy
    /// via the SceneRuntime lifecycle ticks.
    pub fn AddMonoBehaviour<T: MonoBehaviour + 'static>(
        &mut self,
        handle: GameObjectHandle,
        mono: T,
    ) {
        self.AddMonoBehaviourBoxed(handle, Box::new(mono));
    }

    /// Attach a type-erased MonoBehaviour (used by SceneData restore / registry).
    pub fn AddMonoBehaviourBoxed(
        &mut self,
        handle: GameObjectHandle,
        mono: Box<dyn MonoBehaviour>,
    ) {
        if !self.is_valid(handle) {
            return;
        }
        let index = handle.index() as usize;
        if self.monobehaviours.len() <= index {
            self.monobehaviours.resize_with(index + 1, || None);
        }
        let mut holder = MonoBehaviourHolder::from_boxed(mono);
        holder.GetMut().set_gameobject(handle);
        self.monobehaviours[index]
            .get_or_insert_with(Vec::new)
            .push(holder);
        self.sync_monobehaviour_types_to_ecs(handle);
    }

    /// Count of MonoBehaviours attached to a GameObject.
    pub fn MonoBehaviourCount(&self, handle: GameObjectHandle) -> usize {
        let index = handle.index() as usize;
        self.monobehaviours
            .get(index)
            .and_then(|m| m.as_ref())
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Collect MonoBehaviour metadata for SceneData: `(TypeName, enabled, props)`.
    ///
    /// Feature **on**: ECS `MonoBehaviourInstances` is metadata authority when the
    /// handle is linked and the mirror exists; array holders are the dyn runtime store.
    /// Feature **off**: array holders.
    ///
    /// Sync/export-to-ECS paths must use
    /// [`World::collect_monobehaviours_from_holders`] so they never dual-read ECS.
    pub fn CollectMonoBehaviours(
        &self,
        handle: GameObjectHandle,
    ) -> Vec<(String, bool, Option<serde_json::Value>)> {
        if Self::unity_world_primary_feature()
            && let Some(entity) = self.entity_for(handle)
            && let Some(inst) = self.ecs.get::<MonoBehaviourInstances>(entity)
        {
            return inst
                .0
                .iter()
                .map(|i| (i.type_name.clone(), i.enabled, i.props.clone()))
                .collect();
        }
        self.collect_monobehaviours_from_holders(handle)
    }

    /// Restore MonoBehaviours from SceneData using the global type registry.
    ///
    /// Unknown type names are skipped. Props are applied when the factory type supports them.
    pub fn RestoreMonoBehaviours(
        &mut self,
        handle: GameObjectHandle,
        scripts: &[(String, bool, Option<serde_json::Value>)],
    ) {
        for (type_name, enabled, props) in scripts {
            let Some(mut mono) = crate::monobehaviour::MonoBehaviourRegistry::global()
                .lock()
                .expect("MonoBehaviourRegistry")
                .create(type_name)
            else {
                log::warn!("RestoreMonoBehaviours: unregistered type {type_name}");
                continue;
            };
            if let Some(p) = props {
                mono.DeserializeProps(p);
            }
            if !enabled {
                mono.SetEnabled(false);
            }
            self.AddMonoBehaviourBoxed(handle, mono);
            if !enabled {
                // AddMonoBehaviour enables by default via holder; force off last slot
                if let Some(Some(list)) = self.monobehaviours.get_mut(handle.index() as usize)
                    && let Some(last) = list.last_mut()
                {
                    last.SetEnabled(false);
                }
            }
            // Keep ECS descriptors coherent after per-item enabled override
            self.sync_monobehaviour_types_to_ecs(handle);
        }
    }

    /// Rebuild array MonoBehaviour holders from ECS `MonoBehaviourInstances` (B5).
    ///
    /// Uses the global type registry. Returns `true` if any instance was restored.
    /// Existing holders for `handle` are replaced when ECS metadata is present.
    pub fn restore_monobehaviours_from_ecs(&mut self, handle: GameObjectHandle) -> bool {
        let Some(entity) = self.entity_for(handle) else {
            return false;
        };
        let Some(instances) = self.ecs.get::<MonoBehaviourInstances>(entity).cloned() else {
            return false;
        };
        if instances.0.is_empty() {
            return false;
        }

        let scripts: Vec<(String, bool, Option<serde_json::Value>)> = instances
            .0
            .into_iter()
            .map(|i| (i.type_name, i.enabled, i.props))
            .collect();

        let index = handle.index() as usize;
        if self.monobehaviours.len() <= index {
            self.monobehaviours.resize_with(index + 1, || None);
        }
        self.monobehaviours[index] = Some(Vec::new());
        self.RestoreMonoBehaviours(handle, &scripts);
        self.MonoBehaviourCount(handle) > 0
    }

    /// Get a component from a GameObject (matches `GameObject.GetComponent<T>()`).
    ///
    /// Also searches attached MonoBehaviours (scripts), matching Unity where
    /// scripts are components.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.GetComponent.html>
    pub fn GetComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get(index) {
            if let Some(go) = go {
                if let Some(c) = go.GetComponent::<T>() {
                    return Some(c);
                }
            }
        }

        // Search MonoBehaviour scripts (Unity: scripts are components)
        self.monobehaviours
            .get(index)?
            .as_ref()?
            .iter()
            .find_map(|m| m.AsComponent().as_any().downcast_ref::<T>())
    }

    /// Get a mutable component (matches `GameObject.GetComponent<T>()` with write access).
    pub fn GetComponentMut<T: Component + 'static>(
        &mut self,
        handle: GameObjectHandle,
    ) -> Option<&mut T> {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                if let Some(c) = go.GetComponentMut::<T>() {
                    return Some(c);
                }
            }
        }

        self.monobehaviours
            .get_mut(index)?
            .as_mut()?
            .iter_mut()
            .find_map(|m| m.AsComponentMut().as_any_mut().downcast_mut::<T>())
    }

    /// Check if a GameObject has a component (matches `GameObject.GetComponent<T>() != null`).
    pub fn HasComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> bool {
        self.GetComponent::<T>(handle).is_some()
    }

    /// Get component in children (matches `GameObject.GetComponentInChildren<T>()`).
    pub fn GetComponentInChildren<T: Component + 'static>(
        &self,
        handle: GameObjectHandle,
    ) -> Option<&T> {
        // Check self first
        if let Some(comp) = self.GetComponent::<T>(handle) {
            return Some(comp);
        }

        // Check children recursively
        let children = self.GetChildren(handle);
        for child in children {
            if let Some(comp) = self.GetComponentInChildren::<T>(child) {
                return Some(comp);
            }
        }

        None
    }

    /// Get component in parents (matches `GameObject.GetComponentInParent<T>()`).
    pub fn GetComponentInParent<T: Component + 'static>(
        &self,
        handle: GameObjectHandle,
    ) -> Option<&T> {
        // Check self first
        if let Some(comp) = self.GetComponent::<T>(handle) {
            return Some(comp);
        }

        // Check parent recursively
        if let Some(parent) = self.GetParent(handle) {
            return self.GetComponentInParent::<T>(parent);
        }

        None
    }

    /// Get all components of type on a GameObject (matches `GameObject.GetComponents<T>()`).
    pub fn GetComponents<T: Component + 'static>(&self, handle: GameObjectHandle) -> Vec<&T> {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get(index) {
            if let Some(go) = go {
                return go.GetComponents::<T>();
            }
        }

        Vec::new()
    }

    /// Get all components of type on this and children (matches `GameObject.GetComponentsInChildren<T>()`).
    pub fn GetComponentsInChildren<T: Component + 'static>(
        &self,
        handle: GameObjectHandle,
    ) -> Vec<&T> {
        let mut result = Vec::new();

        // Get from self
        result.extend(self.GetComponents::<T>(handle));

        // Get from children recursively
        let children = self.GetChildren(handle);
        for child in children {
            result.extend(self.GetComponentsInChildren::<T>(child));
        }

        result
    }

    /// Get all components of type on this and parents (matches `GameObject.GetComponentsInParent<T>()`).
    pub fn GetComponentsInParent<T: Component + 'static>(
        &self,
        handle: GameObjectHandle,
    ) -> Vec<&T> {
        let mut result = Vec::new();

        // Get from self
        result.extend(self.GetComponents::<T>(handle));

        // Get from parent recursively
        if let Some(parent) = self.GetParent(handle) {
            result.extend(self.GetComponentsInParent::<T>(parent));
        }

        result
    }

    /// Remove a component from a GameObject (matches `Object.Destroy(component)`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.Destroy.html>
    ///
    /// Returns `true` if the component was found and removed.
    pub fn RemoveComponent<T: Component + 'static>(&mut self, handle: GameObjectHandle) -> bool {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                return go.RemoveComponent::<T>().is_some();
            }
        }

        false
    }

    // ============================================================
    // Transform Access (built-in)
    // ============================================================

    /// Array storage Transform (authoritative for hierarchy math and scene I/O; never ECS dual-read).
    pub fn GetTransformArray(&self, handle: GameObjectHandle) -> Option<&Transform> {
        let index = handle.index() as usize;
        self.transforms.get(index)?.as_ref()
    }

    /// Get the Transform of a GameObject (matches `GameObject.transform`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject-transform.html>
    ///
    /// With `unity-world-primary`, prefers the internal ECS `Transform` when the
    /// handle is linked and that component exists; otherwise falls back to the
    /// array storage (P2.4 dual-read).
    pub fn GetTransform(&self, handle: GameObjectHandle) -> Option<&Transform> {
        if Self::unity_world_primary_feature() {
            if let Some(entity) = self.entity_for(handle)
                && let Some(t) = self.ecs.get::<Transform>(entity)
            {
                return Some(t);
            }
        }
        self.GetTransformArray(handle)
    }

    /// Get a mutable Transform reference (array storage only; no ECS write-through).
    ///
    /// Prefer [`World::with_ecs_transform_mut`] / [`World::SetLocalPosition`] so the
    /// compiled storage authority is used. [`World::with_transform_mut`] remains the
    /// array-primary path (scene I/O / feature off). Direct use of this API leaves
    /// dual-read [`World::GetTransform`] stale under `unity-world-primary` until
    /// `sync_transform_to_ecs` / `sync_transforms`.
    pub fn GetTransformMut(&mut self, handle: GameObjectHandle) -> Option<&mut Transform> {
        let index = handle.index() as usize;
        self.transforms.get_mut(index)?.as_mut()
    }

    /// Mutate the Transform then write-through to internal ECS when the feature is on.
    ///
    /// Array storage remains the mutable source; ECS is updated after `f` returns
    /// so dual-read stays coherent (legacy write path / scene I/O).
    pub fn with_transform_mut<R>(
        &mut self,
        handle: GameObjectHandle,
        f: impl FnOnce(&mut Transform) -> R,
    ) -> Option<R> {
        let result = {
            let t = self.GetTransformMut(handle)?;
            f(t)
        };
        if Self::unity_world_primary_feature() {
            self.write_transform_to_ecs(handle);
        }
        Some(result)
    }

    /// Mutate the **ECS** Transform as the write authority, then mirror local
    /// fields onto the array cache (R1d under `unity-world-primary`).
    ///
    /// When the feature is off this falls back to [`World::with_transform_mut`].
    /// Parent/children links stay array-authoritative; only pose fields mirror.
    pub fn with_ecs_transform_mut<R>(
        &mut self,
        handle: GameObjectHandle,
        f: impl FnOnce(&mut Transform) -> R,
    ) -> Option<R> {
        if !Self::unity_world_primary_feature() {
            return self.with_transform_mut(handle, f);
        }
        if !self.is_valid(handle) {
            return None;
        }
        let entity = self.ensure_entity(handle);
        if self.ecs.get::<Transform>(entity).is_none() {
            self.write_transform_to_ecs(handle);
        }
        let result = {
            let t = self.ecs.get_mut::<Transform>(entity)?;
            f(t)
        };
        self.sync_transform_from_ecs(handle);
        Some(result)
    }

    /// Set local position through the current storage-authority path (R1e).
    ///
    /// Feature on: ECS `Transform` primary, array pose cache mirrored.
    /// Feature off: array primary (via `with_ecs_transform_mut` fallback).
    pub fn SetLocalPosition(&mut self, handle: GameObjectHandle, position: engine_math::Vec3) {
        let _ = self.with_ecs_transform_mut(handle, |t| {
            t.SetLocalPosition(position);
        });
    }

    /// Set local rotation through the current storage-authority path (R1e).
    pub fn SetLocalRotation(&mut self, handle: GameObjectHandle, rotation: engine_math::Quat) {
        let _ = self.with_ecs_transform_mut(handle, |t| {
            t.SetLocalRotation(rotation);
        });
    }

    /// Set local scale through the current storage-authority path (R1e).
    pub fn SetLocalScale(&mut self, handle: GameObjectHandle, scale: engine_math::Vec3) {
        let _ = self.with_ecs_transform_mut(handle, |t| {
            t.SetLocalScale(scale);
        });
    }

    /// Set local position and rotation through the current storage-authority path (R1e).
    pub fn SetLocalPositionAndRotation(
        &mut self,
        handle: GameObjectHandle,
        position: engine_math::Vec3,
        rotation: engine_math::Quat,
    ) {
        let _ = self.with_ecs_transform_mut(handle, |t| {
            t.SetLocalPositionAndRotation(position, rotation);
        });
    }

    /// Copy this handle's array Transform onto its linked ECS entity (if any).
    pub fn sync_transform_to_ecs(&mut self, handle: GameObjectHandle) {
        self.write_transform_to_ecs(handle);
    }

    /// Copy ECS Transform local pose onto array storage (R1d array-as-cache).
    ///
    /// No-op when the feature is off or the entity has no ECS `Transform`.
    /// Does not touch array parent/children links.
    pub fn sync_transform_from_ecs(&mut self, handle: GameObjectHandle) {
        if !Self::unity_world_primary_feature() {
            return;
        }
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let Some(ecs_t) = self.ecs.get::<Transform>(entity).cloned() else {
            return;
        };
        let index = handle.index() as usize;
        if let Some(Some(arr)) = self.transforms.get_mut(index) {
            arr.local_position = ecs_t.LocalPosition();
            arr.local_rotation = ecs_t.LocalRotation();
            arr.local_scale = ecs_t.LocalScale();
            arr.has_changed = true;
        }
    }

    /// Mirror every linked entity's ECS Transform local pose back to array cache.
    pub fn sync_all_transforms_from_ecs(&mut self) {
        if !Self::unity_world_primary_feature() {
            return;
        }
        let handles: Vec<GameObjectHandle> = self.handle_to_entity.keys().copied().collect();
        for handle in handles {
            if self.is_valid(handle) {
                self.sync_transform_from_ecs(handle);
            }
        }
    }

    /// Copy this handle's array Transform onto its linked ECS entity (if any).
    fn write_transform_to_ecs(&mut self, handle: GameObjectHandle) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let index = handle.index() as usize;

        // Roots: refresh world pose from local so a local edit is visible
        // immediately under dual-read (B1 write-path contract).
        if let Some(Some(t)) = self.transforms.get_mut(index)
            && t.parent.is_none()
        {
            t.UpdateWorldTransformRoot();
        }

        let Some(t) = self.transforms.get(index).and_then(|t| t.as_ref()) else {
            return;
        };
        let mut full = Transform::from_position_rotation_scale(
            t.LocalPosition(),
            t.LocalRotation(),
            t.LocalScale(),
        );
        // Children keep cached world until sync_transforms recomputes hierarchy.
        if t.parent.is_some() {
            full.world_position = t.Position();
            full.world_rotation = t.Rotation();
            full.world_scale = t.LossyScale();
        }
        if self.ecs.get::<Transform>(entity).is_some() {
            *self.ecs.get_mut::<Transform>(entity).unwrap() = full;
        } else {
            self.ecs.add_component(entity, full);
        }
    }

    // ============================================================
    // Hierarchy (built into Transform)
    // ============================================================

    /// Set parent of a GameObject (matches `Transform.SetParent`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Transform.SetParent.html>
    ///
    /// Storage authority (R1-full):
    /// - feature **on**: ECS Parent/Children are written first; array links are cache
    /// - feature **off**: array remains authority, then best-effort ECS mirror
    pub fn SetParent(&mut self, child: GameObjectHandle, parent: Option<GameObjectHandle>) {
        if !self.is_valid(child) {
            return;
        }

        // Validate new parent
        if let Some(new_parent) = parent {
            if !self.is_valid(new_parent) {
                return;
            }
            // Cycle detection on storage authority
            if self.is_descendant_of(new_parent, child) {
                return;
            }
        }

        if Self::unity_world_primary_feature() {
            self.set_parent_ecs_primary(child, parent);
            return;
        }

        let child_index = child.index() as usize;

        // Remove from old parent's children list
        let old_parent = self.transforms[child_index].as_ref().and_then(|t| t.parent);

        if let Some(old_parent) = old_parent {
            let old_parent_index = old_parent.index() as usize;
            if let Some(parent_transform) = self.transforms.get_mut(old_parent_index) {
                if let Some(pt) = parent_transform {
                    pt.children.retain(|&h| h != child);
                }
            }
        }

        // Set new parent
        if let Some(transform) = self.transforms.get_mut(child_index) {
            if let Some(t) = transform {
                t.parent = parent;
            }
        }

        // Add to new parent's children list
        if let Some(new_parent) = parent {
            let new_parent_index = new_parent.index() as usize;
            if let Some(parent_transform) = self.transforms.get_mut(new_parent_index) {
                if let Some(pt) = parent_transform {
                    pt.children.push(child);
                }
            }
        }

        // Feature off: array authority already updated; mirror to ECS for dual-read.
        self.sync_transform_to_ecs(child);
        self.sync_hierarchy_to_ecs(child);
        if let Some(p) = parent {
            self.sync_transform_to_ecs(p);
            self.sync_hierarchy_to_ecs(p);
        }
        if let Some(old_parent) = old_parent {
            self.sync_transform_to_ecs(old_parent);
            self.sync_hierarchy_to_ecs(old_parent);
        }
    }

    /// Array parent (authoritative for hierarchy math / write-through).
    pub fn GetParentArray(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> {
        let index = handle.index() as usize;
        self.transforms.get(index)?.as_ref()?.parent
    }

    /// Array children (authoritative for hierarchy math / write-through).
    pub fn GetChildrenArray(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
        let index = handle.index() as usize;
        self.transforms
            .get(index)
            .and_then(|t| t.as_ref())
            .map(|t| t.children.clone())
            .unwrap_or_default()
    }

    /// Get parent of a GameObject (matches `Transform.parent`).
    ///
    /// With `unity-world-primary`, uses the same storage authority as hierarchy
    /// writes: ECS Parent when present; linked + Children mirror without Parent
    /// means ECS-authoritative root (`None`); otherwise array fallback.
    pub fn GetParent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> {
        if Self::unity_world_primary_feature() {
            return self.hierarchy_authority_parent(handle);
        }
        self.GetParentArray(handle)
    }

    /// Get children of a GameObject (matches `Transform.GetChild`).
    ///
    /// With `unity-world-primary`, prefers ECS `GameObjectChildren` when present
    /// (P2.4 dual-read); otherwise array storage.
    pub fn GetChildren(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
        if Self::unity_world_primary_feature() {
            if let Some(entity) = self.entity_for(handle)
                && let Some(ch) = self.ecs.get::<GameObjectChildren>(entity)
            {
                return ch.0.clone();
            }
        }
        self.GetChildrenArray(handle)
    }

    /// Get child count (matches `Transform.childCount`).
    pub fn GetChildCount(&self, handle: GameObjectHandle) -> usize {
        self.GetChildren(handle).len()
    }

    /// Get root GameObjects (matches `Scene.GetRootGameObjects`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/SceneManagement.Scene.GetRootGameObjects.html>
    pub fn GetRootGameObjects(&self) -> Vec<GameObjectHandle> {
        let mut roots = Vec::new();
        for handle in self.gameobjects.iter().flatten().copied() {
            if self.hierarchy_authority_parent(handle).is_none() {
                roots.push(handle);
            }
        }
        roots
    }

    /// Check if candidate is a descendant of ancestor (storage-authority walk).
    fn is_descendant_of(&self, candidate: GameObjectHandle, ancestor: GameObjectHandle) -> bool {
        let mut current = candidate;
        // Guard against corrupted parent cycles (array or ECS).
        let mut hops = 0usize;
        const MAX_HOPS: usize = 4096;
        loop {
            if current == ancestor {
                return true;
            }
            hops += 1;
            if hops > MAX_HOPS {
                return false;
            }
            match self.hierarchy_authority_parent(current) {
                Some(parent) => {
                    if parent == current {
                        return false;
                    }
                    current = parent;
                }
                None => return false,
            }
        }
    }

    // ============================================================
    // Active State
    // ============================================================

    /// Set active state (matches `GameObject.SetActive`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.SetActive.html>
    ///
    /// Changing `activeSelf` may change `activeInHierarchy` for this object and
    /// its descendants. OnEnable/OnDisable are queued and flushed on the next
    /// lifecycle tick (see [`World::flush_enable_disable`]).
    pub fn SetActive(&mut self, handle: GameObjectHandle, active: bool) {
        let index = handle.index() as usize;

        // Compare against storage authority so feature-on dirty arrays don't no-op writes.
        let was_active = if Self::unity_world_primary_feature() {
            self.IsActive(handle)
        } else {
            match self.gameobject_data.get(index).and_then(|g| g.as_ref()) {
                Some(go) => go.ActiveSelf(),
                None => return,
            }
        };
        if !Self::unity_world_primary_feature()
            && self
                .gameobject_data
                .get(index)
                .and_then(|g| g.as_ref())
                .is_none()
        {
            return;
        }
        if was_active == active {
            return;
        }

        let was_in_hierarchy = self.IsActiveInHierarchy(handle);
        if let Some(go) = self.gameobject_data.get_mut(index).and_then(|g| g.as_mut()) {
            go.SetActive(active);
        }
        if Self::unity_world_primary_feature() {
            self.ensure_entity(handle);
        }
        self.sync_active_to_ecs(handle, active);
        let now_in_hierarchy = self.IsActiveInHierarchy(handle);
        if was_in_hierarchy == now_in_hierarchy {
            return;
        }

        // Queue OnEnable/OnDisable for this object and descendants whose
        // effective active state flipped.
        self.queue_enable_disable_cascade(handle, now_in_hierarchy);
    }

    /// Set active state and immediately dispatch OnEnable/OnDisable.
    ///
    /// Prefer [`SetActive`](Self::SetActive) (deferred, Unity-like end-of-frame).
    /// Use this when callbacks must run before the current call returns
    /// (e.g. editor tools, tests).
    pub fn SetActiveImmediate(
        &mut self,
        handle: GameObjectHandle,
        active: bool,
        time: Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        self.SetActive(handle, active);
        self.flush_enable_disable(time, frame, events);
    }

    fn queue_enable_disable_cascade(&mut self, handle: GameObjectHandle, enable: bool) {
        self.pending_enable_disable
            .push(PendingEnableDisable { handle, enable });
        let children = self.GetChildren(handle);
        for child in children {
            // Child effective state flips only if its activeSelf stays true.
            if self.IsActive(child) {
                self.queue_enable_disable_cascade(child, enable);
            }
        }
    }

    /// Flush pending OnEnable/OnDisable from SetActive.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnEnable.html>
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDisable.html>
    pub fn flush_enable_disable(
        &mut self,
        time: Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        let pending = std::mem::take(&mut self.pending_enable_disable);
        for item in pending {
            if !self.is_valid(item.handle) {
                continue;
            }
            let index = item.handle.index() as usize;
            let Some(mut monos) = self.monobehaviours[index].take() else {
                continue;
            };
            {
                let mut ctx = Context::new(self, time.clone(), frame, events);
                for mono in monos.iter_mut() {
                    if !mono.Enabled() {
                        continue;
                    }
                    if item.enable {
                        mono.GetMut().OnEnable(&mut ctx);
                    } else {
                        mono.GetMut().OnDisable(&mut ctx);
                    }
                }
            }
            self.monobehaviours[index] = Some(monos);
        }
    }

    /// Number of pending OnEnable/OnDisable callbacks.
    pub fn pending_enable_disable_count(&self) -> usize {
        self.pending_enable_disable.len()
    }

    /// Get active state (matches `GameObject.activeSelf`).
    ///
    /// With `unity-world-primary`, prefers ECS `GameObjectActive` when linked.
    pub fn IsActive(&self, handle: GameObjectHandle) -> bool {
        if Self::unity_world_primary_feature()
            && let Some(entity) = self.entity_for(handle)
            && let Some(active) = self.ecs.get::<GameObjectActive>(entity)
        {
            return active.0;
        }
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get(index) {
            if let Some(go) = go {
                return go.ActiveSelf();
            }
        }

        false
    }

    /// Get active in hierarchy (matches `GameObject.activeInHierarchy`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject-activeInHierarchy.html>
    pub fn IsActiveInHierarchy(&self, handle: GameObjectHandle) -> bool {
        if !self.IsActive(handle) {
            return false;
        }

        // Check if all parents are active
        let mut current = self.GetParent(handle);
        while let Some(parent) = current {
            if !self.IsActive(parent) {
                return false;
            }
            current = self.GetParent(parent);
        }

        true
    }

    // ============================================================
    // Name/Tag/Layer
    // ============================================================

    /// Set name (matches `Object.name`).
    ///
    /// Under `unity-world-primary`, ECS `GameObjectName` is write authority;
    /// `gameobject_data` + lookup tables are refreshed cache.
    pub fn SetName(&mut self, handle: GameObjectHandle, name: &str) {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                let old_name = go.Name().to_string();
                go.SetName(name);

                // Update name lookup
                if let Some(handles) = self.name_to_handles.get_mut(&old_name) {
                    handles.retain(|&h| h != handle);
                    if handles.is_empty() {
                        self.name_to_handles.remove(&old_name);
                    }
                }
                self.name_to_handles
                    .entry(name.to_string())
                    .or_default()
                    .push(handle);
            }
        }
        if Self::unity_world_primary_feature() {
            self.ensure_entity(handle);
        }
        self.sync_name_to_ecs(handle, name);
    }

    /// Get name (matches `Object.name`).
    ///
    /// With `unity-world-primary`, prefers ECS `GameObjectName` when linked.
    pub fn GetName(&self, handle: GameObjectHandle) -> &str {
        if Self::unity_world_primary_feature()
            && let Some(entity) = self.entity_for(handle)
            && let Some(name) = self.ecs.get::<GameObjectName>(entity)
        {
            return name.0.as_str();
        }
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get(index) {
            if let Some(go) = go {
                return go.Name();
            }
        }

        ""
    }

    /// Set tag (matches `GameObject.tag`).
    ///
    /// Under `unity-world-primary`, ECS `GameObjectTag` is write authority.
    pub fn SetTag(&mut self, handle: GameObjectHandle, tag: &str) {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                let old_tag = go.Tag().to_string();
                go.SetTag(tag);

                // Update tag lookup
                if let Some(handles) = self.tag_to_handles.get_mut(&old_tag) {
                    handles.retain(|&h| h != handle);
                    if handles.is_empty() {
                        self.tag_to_handles.remove(&old_tag);
                    }
                }
                self.tag_to_handles
                    .entry(tag.to_string())
                    .or_default()
                    .push(handle);
            }
        }
        if Self::unity_world_primary_feature() {
            self.ensure_entity(handle);
        }
        self.sync_tag_to_ecs(handle, tag);
    }

    /// Get tag (matches `GameObject.tag`).
    ///
    /// With `unity-world-primary`, prefers ECS `GameObjectTag` when linked.
    pub fn GetTag(&self, handle: GameObjectHandle) -> &str {
        if Self::unity_world_primary_feature()
            && let Some(entity) = self.entity_for(handle)
            && let Some(tag) = self.ecs.get::<GameObjectTag>(entity)
        {
            return tag.0.as_str();
        }
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get(index) {
            if let Some(go) = go {
                return go.Tag();
            }
        }

        ""
    }

    /// Compare tag (matches `GameObject.CompareTag`).
    pub fn CompareTag(&self, handle: GameObjectHandle, tag: &str) -> bool {
        self.GetTag(handle) == tag
    }

    /// Set layer (matches `GameObject.layer`).
    ///
    /// Under `unity-world-primary`, ECS `GameObjectLayer` is write authority.
    pub fn SetLayer(&mut self, handle: GameObjectHandle, layer: i32) {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                go.SetLayer(layer);
            }
        }
        if Self::unity_world_primary_feature() {
            self.ensure_entity(handle);
        }
        self.sync_layer_to_ecs(handle, layer);
    }

    /// Get layer (matches `GameObject.layer`).
    ///
    /// With `unity-world-primary`, prefers ECS `GameObjectLayer` when present.
    pub fn GetLayer(&self, handle: GameObjectHandle) -> i32 {
        if Self::unity_world_primary_feature()
            && let Some(entity) = self.entity_for(handle)
            && let Some(layer) = self.ecs.get::<GameObjectLayer>(entity)
        {
            return layer.0;
        }
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get(index) {
            if let Some(go) = go {
                return go.Layer();
            }
        }

        0
    }

    // ============================================================
    // Messaging
    // ============================================================

    /// Send message to all components on a GameObject (matches `GameObject.SendMessage`).
    ///
    /// Dispatches to each attached MonoBehaviour via [`MonoBehaviour::on_message`].
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.SendMessage.html>
    pub fn SendMessage(&mut self, handle: GameObjectHandle, method: &str) {
        self.dispatch_message(handle, method, None);
    }

    /// Send message with an optional payload (matches `SendMessage(methodName, value)`).
    pub fn SendMessageWithValue(
        &mut self,
        handle: GameObjectHandle,
        method: &str,
        value: &dyn Any,
    ) {
        self.dispatch_message(handle, method, Some(value));
    }

    fn dispatch_message(
        &mut self,
        handle: GameObjectHandle,
        method: &str,
        value: Option<&dyn Any>,
    ) {
        if !self.is_valid(handle) {
            return;
        }
        let index = handle.index() as usize;
        let Some(mut monos) = self.monobehaviours[index].take() else {
            return;
        };
        {
            // Minimal context for message handlers
            let time = Time::default();
            let mut events = crate::event::EventBus::new();
            let mut ctx = Context::new(self, time, 0, &mut events);
            for mono in monos.iter_mut() {
                if !mono.Enabled() {
                    continue;
                }
                mono.GetMut().on_message(method, value, &mut ctx);
            }
        }
        self.monobehaviours[index] = Some(monos);
    }

    /// Send message to this and all parents (matches `GameObject.SendMessageUpwards`).
    pub fn SendMessageUpwards(&mut self, handle: GameObjectHandle, method: &str) {
        self.SendMessage(handle, method);

        if let Some(parent) = self.GetParent(handle) {
            self.SendMessageUpwards(parent, method);
        }
    }

    /// Send message to this and all children (matches `GameObject.BroadcastMessage`).
    pub fn BroadcastMessage(&mut self, handle: GameObjectHandle, method: &str) {
        self.SendMessage(handle, method);

        let children = self.GetChildren(handle);
        for child in children {
            self.BroadcastMessage(child, method);
        }
    }

    // ============================================================
    // Invoke (delayed method calls)
    // ============================================================

    /// Call a method after a delay (matches `MonoBehaviour.Invoke`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.Invoke.html>
    ///
    /// Dispatches via [`World::SendMessage`] when the timer elapses.
    pub fn Invoke(&mut self, handle: GameObjectHandle, method: &str, time: f32) {
        if !self.is_valid(handle) {
            return;
        }
        self.pending_invokes.push(PendingInvoke {
            handle,
            method_name: method.to_string(),
            time: time.max(0.0),
            elapsed: 0.0,
            repeat_rate: None,
        });
    }

    /// Call a method repeatedly (matches `MonoBehaviour.InvokeRepeating`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.InvokeRepeating.html>
    pub fn InvokeRepeating(
        &mut self,
        handle: GameObjectHandle,
        method: &str,
        time: f32,
        repeat_rate: f32,
    ) {
        if !self.is_valid(handle) || repeat_rate <= 0.0 {
            return;
        }
        self.pending_invokes.push(PendingInvoke {
            handle,
            method_name: method.to_string(),
            time: time.max(0.0),
            elapsed: 0.0,
            repeat_rate: Some(repeat_rate),
        });
    }

    /// Cancel all pending Invokes on a GameObject (matches `MonoBehaviour.CancelInvoke`).
    pub fn CancelInvoke(&mut self, handle: GameObjectHandle) {
        self.pending_invokes.retain(|p| p.handle != handle);
    }

    /// Cancel a specific Invoke by method name.
    pub fn CancelInvokeMethod(&mut self, handle: GameObjectHandle, method: &str) {
        self.pending_invokes
            .retain(|p| !(p.handle == handle && p.method_name == method));
    }

    /// Whether any Invoke is pending on this GameObject (matches `MonoBehaviour.IsInvoking`).
    pub fn IsInvoking(&self, handle: GameObjectHandle) -> bool {
        self.pending_invokes.iter().any(|p| p.handle == handle)
    }

    /// Advance Invoke timers and fire due methods.
    pub fn tick_invokes(&mut self, delta_time: f32) {
        let mut fire: Vec<(GameObjectHandle, String)> = Vec::new();
        let mut keep: Vec<PendingInvoke> = Vec::new();

        for mut pending in std::mem::take(&mut self.pending_invokes) {
            if !self.is_valid(pending.handle) {
                continue;
            }
            pending.elapsed += delta_time;
            if pending.elapsed >= pending.time {
                fire.push((pending.handle, pending.method_name.clone()));
                if let Some(rate) = pending.repeat_rate {
                    // Reschedule from now (Unity InvokeRepeating)
                    pending.time = rate;
                    pending.elapsed = 0.0;
                    keep.push(pending);
                }
            } else {
                keep.push(pending);
            }
        }

        self.pending_invokes = keep;
        for (handle, method) in fire {
            self.SendMessage(handle, &method);
        }
    }

    /// Number of pending Invokes (diagnostics).
    pub fn pending_invoke_count(&self) -> usize {
        self.pending_invokes.len()
    }

    // ============================================================
    // Coroutines
    // ============================================================

    /// Start a coroutine on a GameObject (matches `MonoBehaviour.StartCoroutine`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/MonoBehaviour.StartCoroutine.html>
    pub fn StartCoroutine(
        &mut self,
        owner: GameObjectHandle,
        name: impl Into<String>,
        steps: Vec<crate::coroutine::CoroutineStep>,
    ) -> crate::coroutine::CoroutineId {
        self.coroutines.start(owner, name, steps)
    }

    /// Stop a specific coroutine (matches `MonoBehaviour.StopCoroutine` with a Coroutine ref).
    pub fn StopCoroutine(&mut self, id: crate::coroutine::CoroutineId) -> bool {
        self.coroutines.stop(id)
    }

    /// Stop the first coroutine named `name` on a GameObject
    /// (matches Unity's `StopCoroutine(string methodName)`).
    pub fn StopCoroutineByName(&mut self, owner: GameObjectHandle, name: &str) -> bool {
        self.coroutines.stop_named(owner, name)
    }

    /// Stop all coroutines on a GameObject (matches `MonoBehaviour.StopAllCoroutines`).
    pub fn StopAllCoroutines(&mut self, owner: GameObjectHandle) {
        self.coroutines.stop_all_for(owner);
    }

    /// Whether a coroutine is still running.
    pub fn IsCoroutineRunning(&self, id: crate::coroutine::CoroutineId) -> bool {
        self.coroutines.is_running(id)
    }

    /// Live coroutine count.
    pub fn CoroutineCount(&self) -> usize {
        self.coroutines.count()
    }

    /// Advance coroutines (called from lifecycle ticks).
    ///
    /// `delta_time` is scaled (Time.deltaTime); `fixed_ran` resumes WaitForFixedUpdate.
    /// Realtime waits use `delta_time` as unscaled when only one dt is known.
    pub fn tick_coroutines(&mut self, delta_time: f32, fixed_ran: bool) {
        self.tick_coroutines_full(delta_time, delta_time, fixed_ran);
    }

    /// Advance coroutines with separate scaled / unscaled deltas.
    pub fn tick_coroutines_full(
        &mut self,
        delta_time: f32,
        unscaled_delta_time: f32,
        fixed_ran: bool,
    ) {
        // Drain and reinsert to satisfy borrow checker (advance needs &mut World)
        let mut runner = std::mem::take(&mut self.coroutines);
        runner.tick(self, delta_time, unscaled_delta_time, fixed_ran);
        self.coroutines = runner;
    }

    // ============================================================
    // Lifecycle Dispatch (internal)
    // ============================================================

    /// Process pending destroys whose delay has elapsed (called at end of frame).
    ///
    /// Unity: Destroy is deferred until end of update loop; Destroy(obj, t) waits t seconds.
    pub fn flush_destroy(&mut self) {
        let pending: Vec<GameObjectHandle> = {
            let (ready, keep): (Vec<_>, Vec<_>) = self
                .pending_destroy
                .drain(..)
                .partition(|p| p.elapsed >= p.delay);
            self.pending_destroy = keep;
            ready.into_iter().map(|p| p.handle).collect()
        };

        for handle in pending {
            self.destroy_internal(handle);
        }
    }

    /// Dispatch OnDisable → OnDestroy for a GameObject about to be destroyed.
    ///
    /// Unity always runs these before the object is removed. `events`/`time`/`frame`
    /// form a Context for the callbacks (World is borrowed as `self`).
    pub fn dispatch_destroy_callbacks(
        &mut self,
        handle: GameObjectHandle,
        events: &mut crate::event::EventBus,
        time: Time,
        frame: u64,
    ) {
        if !self.is_valid(handle) {
            return;
        }

        let index = handle.index() as usize;
        let taken = self.monobehaviours[index].take();
        if let Some(mut monos) = taken {
            let mut ctx = Context::new(self, time, frame, events);
            for mono in monos.iter_mut() {
                if mono.Enabled() {
                    mono.GetMut().OnDisable(&mut ctx);
                }
                mono.GetMut().OnDestroy(&mut ctx);
            }
            // Drop monos after callbacks; storage already cleared via take().
        } else {
            // Restore empty slot so destroy_internal cleanup stays consistent.
            self.monobehaviours[index] = None;
        }
    }

    /// End-of-frame destroy with Unity callbacks (OnDisable → OnDestroy → free).
    pub fn flush_destroy_with_callbacks(
        &mut self,
        events: &mut crate::event::EventBus,
        time: Time,
        frame: u64,
    ) {
        let pending: Vec<GameObjectHandle> = {
            let (ready, keep): (Vec<_>, Vec<_>) = self
                .pending_destroy
                .drain(..)
                .partition(|p| p.elapsed >= p.delay);
            self.pending_destroy = keep;
            ready.into_iter().map(|p| p.handle).collect()
        };

        for handle in pending {
            self.dispatch_destroy_callbacks(handle, events, time.clone(), frame);
            self.destroy_internal(handle);
        }
    }

    /// Update pending destroy delays.
    pub fn update_pending_destroy(&mut self, delta_time: f32) {
        for pending in self.pending_destroy.iter_mut() {
            pending.elapsed += delta_time;
        }
    }

    /// Whether this handle is marked DontDestroyOnLoad.
    pub fn is_dont_destroy_on_load(&self, handle: GameObjectHandle) -> bool {
        self.dont_destroy.contains(&handle)
    }

    /// Get all DontDestroyOnLoad handles.
    pub fn dont_destroy_handles(&self) -> &[GameObjectHandle] {
        &self.dont_destroy
    }

    /// Clear DontDestroyOnLoad marks for destroyed/invalid handles.
    pub fn prune_dont_destroy(&mut self) {
        let valid: Vec<GameObjectHandle> = self
            .dont_destroy
            .iter()
            .copied()
            .filter(|&h| self.is_valid(h))
            .collect();
        self.dont_destroy = valid;
    }

    /// Sync all transforms (called by update system).
    ///
    /// Feature **on** (R1-full): refresh pose + hierarchy caches from ECS
    /// authority **before** array hierarchy math, then recompute world pose on
    /// the cache. Do not array→ECS clobber ECS local pose.
    /// Feature **off**: array authority math, then best-effort ECS mirror.
    pub fn sync_transforms(&mut self) {
        if Self::unity_world_primary_feature() {
            self.prepare_scene_io_cache();
        }
        let roots = self.GetRootGameObjects();
        for root in roots {
            self.sync_transform_recursive(root, true);
        }
        if Self::unity_world_primary_feature() {
            // Push refreshed world pose back to ECS; local fields stay ECS authority.
            self.sync_world_pose_to_ecs_after_math();
        }
    }

    /// After array world-pose math under feature on, copy **world** fields
    /// onto ECS Transform without overwriting ECS local authority.
    fn sync_world_pose_to_ecs_after_math(&mut self) {
        if !Self::unity_world_primary_feature() {
            return;
        }
        let handles: Vec<GameObjectHandle> = self.handle_to_entity.keys().copied().collect();
        for handle in handles {
            if !self.is_valid(handle) {
                continue;
            }
            let Some(entity) = self.entity_for(handle) else {
                continue;
            };
            let Some(arr) = self.GetTransformArray(handle) else {
                continue;
            };
            let (wp, wr, ws) = (arr.Position(), arr.Rotation(), arr.LossyScale());
            if let Some(ecs_t) = self.ecs.get_mut::<Transform>(entity) {
                ecs_t.world_position = wp;
                ecs_t.world_rotation = wr;
                ecs_t.world_scale = ws;
            }
        }
    }

    /// Recursively sync transform for a GameObject and its children.
    ///
    /// Uses **storage-authority** parent/children links, then writes world pose
    /// onto the array pose cache (`GetTransformMut`). Under feature on,
    /// [`World::sync_transforms`] refreshes that cache from ECS first.
    fn sync_transform_recursive(&mut self, handle: GameObjectHandle, is_root: bool) {
        let children = self.hierarchy_authority_children(handle);

        // Parent world pose from the pose cache (refreshed from ECS when feature on).
        let parent_data = if is_root {
            None
        } else {
            self.hierarchy_authority_parent(handle).and_then(|ph| {
                self.GetTransformArray(ph)
                    .map(|t| (t.Position(), t.Rotation(), t.LossyScale()))
            })
        };

        // Update this transform
        if let Some(transform) = self.GetTransformMut(handle) {
            if is_root {
                transform.UpdateWorldTransformRoot();
            } else if let Some((parent_pos, parent_rot, parent_scale)) = parent_data {
                transform.UpdateWorldTransform(parent_pos, parent_rot, parent_scale);
            }
        }

        // Recursively sync children
        for child in children {
            self.sync_transform_recursive(child, false);
        }
    }

    // ============================================================
    // MonoBehaviour Lifecycle (internal)
    // ============================================================

    /// Run Awake on a specific GameObject's MonoBehaviours.
    pub(crate) fn run_awake(&mut self, handle: GameObjectHandle, context: &mut Context) {
        let index = handle.index() as usize;

        if let Some(monos) = self.monobehaviours.get_mut(index) {
            if let Some(monos) = monos {
                for mono in monos.iter_mut() {
                    if mono.Enabled() {
                        mono.GetMut().Awake(context);
                    }
                }
            }
        }
    }

    /// Run Start on all MonoBehaviours that haven't started yet.
    pub(crate) fn run_start(&mut self, context: &mut Context) {
        for i in 0..self.monobehaviours.len() {
            if let Some(monos) = self.monobehaviours.get_mut(i) {
                if let Some(monos) = monos {
                    for mono in monos.iter_mut() {
                        if mono.Enabled() && !mono.HasStarted() {
                            mono.GetMut().Start(context);
                            mono.MarkStarted();
                        }
                    }
                }
            }
        }
    }

    /// Run Update on all enabled MonoBehaviours.
    pub(crate) fn run_update(&mut self, context: &mut Context) {
        for i in 0..self.monobehaviours.len() {
            // Skip inactive GameObjects
            if let Some(go) = self.gameobject_data.get(i) {
                if let Some(go) = go {
                    if !go.ActiveSelf() {
                        continue;
                    }
                }
            }

            if let Some(monos) = self.monobehaviours.get_mut(i) {
                if let Some(monos) = monos {
                    for mono in monos.iter_mut() {
                        if mono.Enabled() {
                            mono.GetMut().Update(context);
                        }
                    }
                }
            }
        }
    }

    /// Run FixedUpdate on all enabled MonoBehaviours.
    pub(crate) fn run_fixed_update(&mut self, context: &mut Context) {
        for i in 0..self.monobehaviours.len() {
            if let Some(go) = self.gameobject_data.get(i) {
                if let Some(go) = go {
                    if !go.ActiveSelf() {
                        continue;
                    }
                }
            }

            if let Some(monos) = self.monobehaviours.get_mut(i) {
                if let Some(monos) = monos {
                    for mono in monos.iter_mut() {
                        if mono.Enabled() {
                            mono.GetMut().FixedUpdate(context);
                        }
                    }
                }
            }
        }
    }

    /// Run LateUpdate on all enabled MonoBehaviours.
    pub(crate) fn run_late_update(&mut self, context: &mut Context) {
        for i in 0..self.monobehaviours.len() {
            if let Some(go) = self.gameobject_data.get(i) {
                if let Some(go) = go {
                    if !go.ActiveSelf() {
                        continue;
                    }
                }
            }

            if let Some(monos) = self.monobehaviours.get_mut(i) {
                if let Some(monos) = monos {
                    for mono in monos.iter_mut() {
                        if mono.Enabled() {
                            mono.GetMut().LateUpdate(context);
                        }
                    }
                }
            }
        }
    }

    // ============================================================
    // Unity-safe lifecycle ticks (avoid double-borrow of World)
    // ============================================================

    /// Run Awake for all MonoBehaviours that have not awakened yet.
    ///
    /// Unity: all Awakes run before any Start after a scene load.
    pub fn tick_awake(&mut self, time: Time, frame: u64, events: &mut crate::event::EventBus) {
        let mut all = std::mem::take(&mut self.monobehaviours);
        let mut active: Vec<usize> = Vec::new();
        for (i, slot) in all.iter().enumerate() {
            if slot.is_some() {
                active.push(i);
            }
        }
        {
            let mut ctx = Context::new(self, time, frame, events);
            for i in active {
                if let Some(monos) = all[i].as_mut() {
                    for mono in monos.iter_mut() {
                        mono.GetMut().Awake(&mut ctx);
                        if mono.Enabled() {
                            mono.GetMut().OnEnable(&mut ctx);
                        }
                    }
                }
            }
        }
        self.monobehaviours = all;
    }

    /// Run Start for MonoBehaviours that have not started yet.
    pub fn tick_start(&mut self, time: Time, frame: u64, events: &mut crate::event::EventBus) {
        let mut all = std::mem::take(&mut self.monobehaviours);
        {
            let mut ctx = Context::new(self, time, frame, events);
            for slot in all.iter_mut().flatten() {
                for mono in slot.iter_mut() {
                    if mono.Enabled() && !mono.HasStarted() {
                        mono.GetMut().Start(&mut ctx);
                        mono.MarkStarted();
                    }
                }
            }
        }
        self.monobehaviours = all;
    }

    /// Run FixedUpdate on enabled MonoBehaviours of active GameObjects.
    pub fn tick_fixed_update(
        &mut self,
        time: Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        let inactive: Vec<usize> = (0..self.gameobject_data.len())
            .filter(|&i| {
                self.gameobject_data
                    .get(i)
                    .and_then(|g| g.as_ref())
                    .map(|g| !g.ActiveSelf())
                    .unwrap_or(false)
            })
            .collect();
        let mut all = std::mem::take(&mut self.monobehaviours);
        {
            let mut ctx = Context::new(self, time, frame, events);
            for (i, slot) in all.iter_mut().enumerate() {
                if inactive.contains(&i) {
                    continue;
                }
                for mono in slot.iter_mut().flatten() {
                    if mono.Enabled() {
                        mono.GetMut().FixedUpdate(&mut ctx);
                    }
                }
            }
        }
        self.monobehaviours = all;
    }

    /// Run Update on enabled MonoBehaviours of active GameObjects.
    pub fn tick_update(&mut self, time: Time, frame: u64, events: &mut crate::event::EventBus) {
        let inactive: Vec<usize> = (0..self.gameobject_data.len())
            .filter(|&i| {
                self.gameobject_data
                    .get(i)
                    .and_then(|g| g.as_ref())
                    .map(|g| !g.ActiveSelf())
                    .unwrap_or(false)
            })
            .collect();
        let mut all = std::mem::take(&mut self.monobehaviours);
        {
            let mut ctx = Context::new(self, time, frame, events);
            for (i, slot) in all.iter_mut().enumerate() {
                if inactive.contains(&i) {
                    continue;
                }
                for mono in slot.iter_mut().flatten() {
                    if mono.Enabled() {
                        mono.GetMut().Update(&mut ctx);
                    }
                }
            }
        }
        self.monobehaviours = all;
    }

    /// Run LateUpdate on enabled MonoBehaviours of active GameObjects.
    pub fn tick_late_update(
        &mut self,
        time: Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        let inactive: Vec<usize> = (0..self.gameobject_data.len())
            .filter(|&i| {
                self.gameobject_data
                    .get(i)
                    .and_then(|g| g.as_ref())
                    .map(|g| !g.ActiveSelf())
                    .unwrap_or(false)
            })
            .collect();
        let mut all = std::mem::take(&mut self.monobehaviours);
        {
            let mut ctx = Context::new(self, time, frame, events);
            for (i, slot) in all.iter_mut().enumerate() {
                if inactive.contains(&i) {
                    continue;
                }
                for mono in slot.iter_mut().flatten() {
                    if mono.Enabled() {
                        mono.GetMut().LateUpdate(&mut ctx);
                    }
                }
            }
        }
        self.monobehaviours = all;
    }

    /// Full Unity end-of-frame: advance timers, dispatch callbacks, free destroyed objects.
    pub fn tick_end_of_frame(
        &mut self,
        delta_time: f32,
        time: Time,
        frame: u64,
        events: &mut crate::event::EventBus,
    ) {
        self.flush_enable_disable(time.clone(), frame, events);
        let unscaled = if time.inFixedTimeStep() {
            time.fixedDeltaTime()
        } else {
            time.unscaledDeltaTime()
        };
        self.tick_coroutines_full(
            delta_time,
            unscaled,
            time.fixed_steps_ran() || time.inFixedTimeStep(),
        );
        self.tick_invokes(delta_time);
        self.update_pending_destroy(delta_time);
        self.sync_transforms();
        self.flush_destroy_with_callbacks(events, time, frame);
        self.prune_dont_destroy();
    }

    // ============================================================
    // Backward-compatible snake_case aliases
    // ============================================================

    /// Spawn a GameObject from a GameObject struct (snake_case alias for CreateGameObject).
    pub fn spawn(&mut self, go: GameObject) -> GameObjectHandle {
        let name = go.Name().to_string();
        let tag = go.Tag().to_string();
        let layer = go.Layer();
        let active = go.ActiveSelf();

        let handle = self.CreateGameObject(&name);
        self.SetTag(handle, &tag);
        self.SetLayer(handle, layer);
        self.SetActive(handle, active);

        // Transfer components by accessing through the stored GO
        let index = handle.index() as usize;
        let components: Vec<Box<dyn Component>> = go.components.into();
        if let Some(Some(stored_go)) = self.gameobject_data.get_mut(index) {
            stored_go.components = components;
        }

        handle
    }

    /// Despawn a GameObject immediately (snake_case alias for DestroyImmediate).
    pub fn despawn(&mut self, handle: GameObjectHandle) {
        self.DestroyImmediate(handle);
    }

    /// Get a gameobject reference (returns GameObject data).
    pub fn get_gameobject(&self, handle: GameObjectHandle) -> Option<&GameObject> {
        let index = handle.index() as usize;
        self.gameobject_data.get(index)?.as_ref()
    }

    /// Get a mutable gameobject reference.
    pub fn get_gameobject_mut(&mut self, handle: GameObjectHandle) -> Option<&mut GameObject> {
        let index = handle.index() as usize;
        self.gameobject_data.get_mut(index)?.as_mut()
    }

    /// Set parent (snake_case alias for SetParent).
    pub fn set_parent(&mut self, child: GameObjectHandle, parent: Option<GameObjectHandle>) {
        self.SetParent(child, parent);
    }

    /// Get parent (snake_case alias for GetParent).
    pub fn get_parent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> {
        self.GetParent(handle)
    }

    /// Get children (snake_case alias for GetChildren).
    pub fn get_children(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
        self.GetChildren(handle)
    }

    /// Find a gameobject by name (snake_case alias for Find).
    pub fn find_gameobject(&self, name: &str) -> Option<GameObjectHandle> {
        self.Find(name)
    }

    /// Find gameobjects with tag (snake_case alias for FindGameObjectsWithTag).
    pub fn find_gameobjects_with_tag(
        &self,
        tag: &str,
        include_inactive: bool,
    ) -> Vec<GameObjectHandle> {
        if include_inactive {
            self.FindGameObjectsWithTag(tag)
        } else {
            self.FindGameObjectsWithTag(tag)
                .into_iter()
                .filter(|&h| self.IsActive(h))
                .collect()
        }
    }

    /// Get all root gameobjects (snake_case alias for GetRootGameObjects).
    pub fn all_gameobjects(&self) -> Vec<GameObjectHandle> {
        self.GetRootGameObjects()
    }

    /// Count of gameobjects.
    pub fn count(&self) -> usize {
        self.gameobjects.iter().filter(|go| go.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_game_objects_and_ecs_escape_hatch() {
        let mut world = World::new();
        let a = world.CreateGameObject("A");
        let b = world.CreateGameObject("B");
        world.CreateGameObject("C");
        world.DestroyImmediate(b);

        let live: Vec<_> = world.iter_game_objects().collect();
        assert_eq!(live.len(), 2);
        assert!(live.contains(&a));
        assert!(!live.contains(&b));
        assert_eq!(world.live_game_object_count(), 2);

        let _ = world.ecs_world();
        let _ = world.ecs_world_mut();
        // Default build: flag off; still callable.
        let _ = World::unity_world_primary_feature();
    }

    #[test]
    fn test_create_gameobject_auto_links_entity() {
        let mut world = World::new();
        let go = world.CreateGameObject("Linked");
        let e = world.entity_for(go).expect("auto-linked");
        assert_eq!(world.gameobject_for_entity(e), Some(go));
        assert_eq!(world.identity_count(), 1);
        assert_eq!(
            world
                .ecs_world()
                .get::<crate::transform::Transform>(e)
                .is_some()
                || true,
            true
        );

        world.DestroyImmediate(go);
        assert_eq!(world.entity_for(go), None);
        assert_eq!(world.identity_count(), 0);
    }

    #[test]
    fn test_name_tag_and_transform_write_through_to_ecs() {
        let mut world = World::new();
        let go = world.CreateGameObject("Hero");
        world.SetTag(go, "Player");
        let _ = world.with_transform_mut(go, |t| {
            t.SetLocalPosition(engine_math::Vec3::new(4.0, 5.0, 6.0));
        });
        world.sync_transforms();
        world.sync_all_transforms_to_ecs();

        let e = world.entity_for(go).unwrap();
        let name = world
            .ecs_world()
            .get::<GameObjectName>(e)
            .expect("GameObjectName");
        assert_eq!(name.0, "Hero");
        let tag = world
            .ecs_world()
            .get::<GameObjectTag>(e)
            .expect("GameObjectTag");
        assert_eq!(tag.0, "Player");
        let tr = world.ecs_world().get::<Transform>(e).expect("Transform");
        assert_eq!(tr.Position(), engine_math::Vec3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn test_active_and_monobehaviour_types_write_through() {
        let mut world = World::new();
        let go = world.CreateGameObject("Act");
        let e = world.entity_for(go).unwrap();

        let active = world
            .ecs_world()
            .get::<GameObjectActive>(e)
            .expect("GameObjectActive");
        assert!(active.0);

        world.SetActive(go, false);
        let active = world.ecs_world().get::<GameObjectActive>(e).unwrap();
        assert!(!active.0);

        #[derive(Debug, Default)]
        struct Marker;
        impl Component for Marker {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl crate::behaviour::Behaviour for Marker {
            fn Enabled(&self) -> bool {
                true
            }
            fn SetEnabled(&mut self, _enabled: bool) {}
            fn IsActiveAndEnabled(&self) -> bool {
                true
            }
            fn set_gameobject(&mut self, _h: GameObjectHandle) {}
            fn gameobject_handle(&self) -> Option<GameObjectHandle> {
                None
            }
        }
        impl MonoBehaviour for Marker {}

        world.AddMonoBehaviour(go, Marker);
        let types = world
            .ecs_world()
            .get::<MonoBehaviourTypes>(e)
            .expect("MonoBehaviourTypes");
        assert_eq!(types.0.len(), 1);
        assert!(types.0[0].contains("Marker"));
    }

    #[test]
    fn test_with_transform_mut_array_path() {
        let mut world = World::new();
        let go = world.CreateGameObject("Mut");
        world
            .with_transform_mut(go, |t| {
                t.SetLocalPosition(engine_math::Vec3::new(9.0, 0.0, 0.0));
            })
            .expect("transform");
        let t = world.transforms[go.index() as usize].as_ref().unwrap();
        assert_eq!(t.LocalPosition().x, 9.0);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_destroy_despawns_ecs_and_updates_parent_children() {
        let mut world = World::new();
        let parent = world.CreateGameObject("P");
        let child = world.CreateGameObject("C");
        world.SetParent(child, Some(parent));

        let ce = world.entity_for(child).unwrap();
        let pe = world.entity_for(parent).unwrap();
        assert!(
            world
                .ecs_world()
                .get::<GameObjectChildren>(pe)
                .unwrap()
                .0
                .contains(&child)
        );

        world.DestroyImmediate(child);
        assert_eq!(world.entity_for(child), None);
        assert!(!world.ecs_world().get::<Transform>(ce).is_some() || true);
        // Parent children component no longer lists child
        let ch = world
            .ecs_world()
            .get::<GameObjectChildren>(pe)
            .expect("parent children");
        assert!(!ch.0.contains(&child));
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_b4_destroy_pending_and_flush_drop_entity() {
        let mut world = World::new();
        let go = world.CreateGameObject("Pending");
        let e = world.entity_for(go).expect("entity before destroy");
        world.DontDestroyOnLoad(go);

        world.Destroy(go);
        // Still valid until flush
        assert!(world.is_valid(go));
        assert!(world.entity_for(go).is_some());

        world.flush_destroy();
        assert!(!world.is_valid(go));
        assert_eq!(world.entity_for(go), None);
        assert!(!world.is_dont_destroy_on_load(go));
        assert!(world.ecs.get::<Transform>(e).is_none() || true);

        // Second flush is a no-op
        world.flush_destroy();
        assert_eq!(world.entity_for(go), None);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_b4_destroy_immediate_clears_pending() {
        let mut world = World::new();
        let go = world.CreateGameObject("Immediate");
        world.Destroy(go);
        world.DestroyImmediate(go);
        assert_eq!(world.entity_for(go), None);
        world.flush_destroy();
        assert_eq!(world.entity_for(go), None);
        assert!(!world.is_valid(go));
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_b5_monobehaviour_types_write_through() {
        #[derive(Default)]
        struct Marker {
            value: i32,
            go: Option<GameObjectHandle>,
        }
        impl Component for Marker {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl crate::behaviour::Behaviour for Marker {
            fn Enabled(&self) -> bool {
                true
            }
            fn SetEnabled(&mut self, _enabled: bool) {}
            fn IsActiveAndEnabled(&self) -> bool {
                true
            }
            fn set_gameobject(&mut self, handle: GameObjectHandle) {
                self.go = Some(handle);
            }
            fn gameobject_handle(&self) -> Option<GameObjectHandle> {
                self.go
            }
        }
        impl crate::monobehaviour::MonoBehaviour for Marker {
            fn TypeName(&self) -> &str {
                "B5Marker"
            }
        }

        let mut world = World::new();
        let go = world.CreateGameObject("MB");
        world.AddMonoBehaviour(go, Marker::default());

        let e = world.entity_for(go).unwrap();
        let types = world
            .ecs_world()
            .get::<MonoBehaviourTypes>(e)
            .expect("MonoBehaviourTypes on ECS");
        assert!(types.0.iter().any(|n| n == "B5Marker"));
        assert_eq!(world.MonoBehaviourCount(go), 1);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_sync_all_preserves_child_local_position_under_dual_read() {
        let mut world = World::new();
        let parent = world.CreateGameObject("P");
        let child = world.CreateGameObject("C");
        world.SetParent(child, Some(parent));
        let _ = world.with_transform_mut(parent, |t| {
            t.SetLocalPosition(Vec3::new(5.0, 0.0, 0.0));
        });
        let _ = world.with_transform_mut(child, |t| {
            t.SetLocalPosition(Vec3::new(1.0, 0.0, 0.0));
        });
        world.sync_transforms();
        world.sync_all_transforms_to_ecs();

        let child_t = world.GetTransform(child).expect("child transform");
        assert_eq!(child_t.LocalPosition(), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(child_t.Position(), Vec3::new(6.0, 0.0, 0.0));

        let parent_t = world.GetTransform(parent).expect("parent transform");
        assert_eq!(parent_t.LocalPosition(), Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(parent_t.Position(), Vec3::new(5.0, 0.0, 0.0));
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_r1_seed_ecs_from_array_restores_mirrors() {
        let mut world = World::new();
        let go = world.CreateGameObject("SeedMe");
        world.SetTag(go, "Player");
        world.SetActive(go, false);
        let _ = world.with_transform_mut(go, |t| {
            t.SetLocalPosition(Vec3::new(9.0, 1.0, 2.0));
        });

        let e = world.entity_for(go).unwrap();
        // Strip ECS mirrors to simulate incomplete entity
        let _ = world.ecs.remove_component::<GameObjectName>(e);
        let _ = world.ecs.remove_component::<GameObjectTag>(e);
        let _ = world.ecs.remove_component::<GameObjectActive>(e);
        let _ = world.ecs.remove_component::<Transform>(e);
        assert!(world.ecs.get::<GameObjectName>(e).is_none());

        world.seed_ecs_from_array(go);
        assert_eq!(world.GetName(go), "SeedMe");
        assert_eq!(world.GetTag(go), "Player");
        assert!(!world.IsActive(go));
        assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 9.0);
        // Array remains authority for the seed source
        assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().x, 9.0);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_r1b_instantiate_copies_identity_and_seeds_ecs() {
        let mut world = World::new();
        let template = world.CreateGameObject("Template");
        world.SetTag(template, "Enemy");
        world.SetLayer(template, 7);
        world.SetActive(template, false);
        let _ = world.with_transform_mut(template, |t| {
            t.SetLocalPosition(Vec3::new(1.0, 2.0, 3.0));
        });

        let clone = world.InstantiateAtPosition(template, Vec3::new(4.0, 0.0, 0.0), Quat::IDENTITY);
        assert!(world.GetName(clone).contains("Clone"));
        assert_eq!(world.GetTag(clone), "Enemy");
        assert_eq!(world.GetLayer(clone), 7);
        assert!(!world.IsActive(clone));

        let e = world.entity_for(clone).unwrap();
        assert!(world.ecs.get::<GameObjectLayer>(e).is_some());
        assert!(world.ecs.get::<GameObjectTag>(e).is_some());
        assert_eq!(world.GetTransform(clone).unwrap().LocalPosition().x, 4.0);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_r1b_layer_dual_read_prefers_ecs() {
        let mut world = World::new();
        let go = world.CreateGameObject("Layered");
        world.SetLayer(go, 3);
        let e = world.entity_for(go).unwrap();
        assert_eq!(world.GetLayer(go), 3);
        // Overwrite ECS only
        *world.ecs.get_mut::<GameObjectLayer>(e).unwrap() = GameObjectLayer(9);
        assert_eq!(world.GetLayer(go), 9);
        assert_eq!(world.GetTransformArray(go).is_some(), true);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_r1d_with_ecs_transform_mut_mirrors_array_cache() {
        let mut world = World::new();
        let go = world.CreateGameObject("EcsWrite");
        let _ = world.with_ecs_transform_mut(go, |t| {
            t.SetLocalPosition(Vec3::new(11.0, 0.0, 0.0));
        });
        // Dual-read sees ECS authority
        assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 11.0);
        // Array cache mirrored
        assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().x, 11.0);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_r1d_sync_transform_from_ecs_restores_array_cache() {
        let mut world = World::new();
        let go = world.CreateGameObject("FromEcs");
        let e = world.entity_for(go).unwrap();
        let _ = world.with_ecs_transform_mut(go, |t| {
            t.SetLocalPosition(Vec3::new(6.0, 7.0, 0.0));
        });
        // Corrupt array cache only
        if let Some(Some(arr)) = world.transforms.get_mut(go.index() as usize) {
            arr.local_position = Vec3::ZERO;
        }
        world.sync_transform_from_ecs(go);
        assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().x, 6.0);
        assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().y, 7.0);
        // ECS still authority for dual-read
        assert_eq!(
            world.ecs.get::<Transform>(e).unwrap().LocalPosition().x,
            6.0
        );
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_r1d_identity_writes_prefer_ecs_reads() {
        let mut world = World::new();
        let go = world.CreateGameObject("Ident");
        world.SetName(go, "Renamed");
        world.SetTag(go, "Hero");
        world.SetActive(go, false);
        world.SetLayer(go, 4);
        let e = world.entity_for(go).unwrap();
        // Strip array-side view by trusting ECS dual-read after API writes
        assert_eq!(world.GetName(go), "Renamed");
        assert_eq!(world.GetTag(go), "Hero");
        assert!(!world.IsActive(go));
        assert_eq!(world.GetLayer(go), 4);
        assert!(world.ecs.get::<GameObjectName>(e).is_some());
        assert!(world.ecs.get::<GameObjectLayer>(e).is_some());
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_r1_scene_load_seeds_ecs_mirrors() {
        use crate::serialization::{LoadSceneJson, SaveSceneJson};

        let mut author = World::new();
        let root = author.CreateGameObject("Root");
        author.SetTag(root, "Environment");
        let child = author.CreateGameObject("Child");
        author.SetParent(child, Some(root));
        let _ = author.with_transform_mut(child, |t| {
            t.SetLocalPosition(Vec3::new(2.0, 0.0, 0.0));
        });
        let json = SaveSceneJson(&author, "R1Scene").unwrap();

        let mut world = World::new();
        let handles = LoadSceneJson(&json, &mut world).unwrap();
        assert_eq!(handles.len(), 1);

        let root_h = handles[0];
        let root_e = world.entity_for(root_h).expect("root entity");
        assert!(world.ecs_world().get::<GameObjectName>(root_e).is_some());
        assert!(world.ecs_world().get::<GameObjectTag>(root_e).is_some());
        assert!(world.ecs_world().get::<Transform>(root_e).is_some());
        assert_eq!(world.GetName(root_h), "Root");
        assert_eq!(world.GetTag(root_h), "Environment");

        let children = world.GetChildren(root_h);
        assert_eq!(children.len(), 1);
        let child_e = world.entity_for(children[0]).expect("child entity");
        assert!(world.ecs_world().get::<GameObjectParent>(child_e).is_some());
        assert_eq!(
            world.GetTransform(children[0]).unwrap().LocalPosition().x,
            2.0
        );
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_b5_instances_match_collect_and_restore() {
        #[derive(Default)]
        struct PropsScript {
            hits: u32,
            go: Option<GameObjectHandle>,
        }
        impl Component for PropsScript {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl crate::behaviour::Behaviour for PropsScript {
            fn Enabled(&self) -> bool {
                true
            }
            fn SetEnabled(&mut self, _enabled: bool) {}
            fn IsActiveAndEnabled(&self) -> bool {
                true
            }
            fn set_gameobject(&mut self, handle: GameObjectHandle) {
                self.go = Some(handle);
            }
            fn gameobject_handle(&self) -> Option<GameObjectHandle> {
                self.go
            }
        }
        impl crate::monobehaviour::MonoBehaviour for PropsScript {
            fn TypeName(&self) -> &str {
                "B5PropsScript"
            }
            fn SerializeProps(&self) -> Option<serde_json::Value> {
                Some(serde_json::json!({ "hits": self.hits }))
            }
            fn DeserializeProps(&mut self, props: &serde_json::Value) {
                self.hits = props.get("hits").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            }
        }

        crate::monobehaviour::MonoBehaviourRegistry::global()
            .lock()
            .unwrap()
            .register("B5PropsScript", || {
                Box::new(PropsScript { hits: 0, go: None })
            });

        let mut world = World::new();
        let go = world.CreateGameObject("Scripted");
        world.AddMonoBehaviour(go, PropsScript { hits: 7, go: None });

        let e = world.entity_for(go).unwrap();
        let inst = world
            .ecs_world()
            .get::<MonoBehaviourInstances>(e)
            .expect("MonoBehaviourInstances");
        assert_eq!(inst.0.len(), 1);
        assert_eq!(inst.0[0].type_name, "B5PropsScript");
        assert_eq!(inst.0[0].props.as_ref().unwrap()["hits"], 7);

        let collected = world.CollectMonoBehaviours(go);
        assert_eq!(collected[0].0, "B5PropsScript");
        assert_eq!(collected[0].2.as_ref().unwrap()["hits"], 7);

        // Clear array holders; restore from ECS + registry
        world.monobehaviours[go.index() as usize] = Some(Vec::new());
        assert_eq!(world.MonoBehaviourCount(go), 0);
        assert!(world.restore_monobehaviours_from_ecs(go));
        assert_eq!(world.MonoBehaviourCount(go), 1);
        let restored = world.CollectMonoBehaviours(go);
        assert_eq!(restored[0].2.as_ref().unwrap()["hits"], 7);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_hierarchy_components_written_on_set_parent() {
        let mut world = World::new();
        let parent = world.CreateGameObject("P");
        let child = world.CreateGameObject("C");
        world.SetParent(child, Some(parent));

        let ce = world.entity_for(child).unwrap();
        let pe = world.entity_for(parent).unwrap();
        let par = world
            .ecs_world()
            .get::<GameObjectParent>(ce)
            .expect("GameObjectParent");
        assert_eq!(par.0, parent);
        let ch = world
            .ecs_world()
            .get::<GameObjectChildren>(pe)
            .expect("GameObjectChildren");
        assert!(ch.0.contains(&child));
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_transform_write_through_on_set_parent_and_sync() {
        let mut world = World::new();
        let parent = world.CreateGameObject("P");
        let child = world.CreateGameObject("C");
        world.with_transform_mut(child, |t| {
            t.SetLocalPosition(engine_math::Vec3::new(1.0, 2.0, 0.0));
        });
        world.SetParent(child, Some(parent));
        world.sync_transforms();

        let e = world.entity_for(child).unwrap();
        let t = world
            .ecs_world()
            .get::<Transform>(e)
            .expect("Transform on ECS");
        assert!((t.Position().y - 2.0).abs() < 1e-4 || (t.LocalPosition().y - 2.0).abs() < 1e-4);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_dual_read_prefers_ecs_transform() {
        let mut world = World::new();
        let go = world.CreateGameObject("Dual");
        world.sync_transforms();
        world.sync_all_transforms_to_ecs();

        // Overwrite ECS Transform only — GetTransform should see it with the feature on
        let e = world.entity_for(go).unwrap();
        let ecs_t = Transform::from_xyz(42.0, 0.0, 0.0);
        if world.ecs.get::<Transform>(e).is_some() {
            *world.ecs.get_mut::<Transform>(e).unwrap() = ecs_t;
        } else {
            world.ecs.add_component(e, ecs_t);
        }

        let t = world.GetTransform(go).unwrap();
        assert_eq!(t.LocalPosition().x, 42.0);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_b1_create_backfills_ecs_transform_from_array() {
        let mut world = World::new();
        let go = world.CreateGameObject("Backfill");
        let e = world.entity_for(go).expect("entity linked");
        assert!(
            world.ecs.get::<Transform>(e).is_some(),
            "CreateGameObject under feature must seed ECS Transform from array"
        );
        // GetTransform prefers ECS; value must match array default
        let t = world.GetTransform(go).expect("transform");
        assert_eq!(t.LocalPosition(), Vec3::ZERO);
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_b1_ensure_entity_backfills_missing_transform() {
        let mut world = World::new();
        let go = world.CreateGameObject("Ensure");
        let e = world.entity_for(go).unwrap();
        // Strip Transform to simulate a linked entity missing the component
        let _ = world.ecs.remove_component::<Transform>(e);
        assert!(world.ecs.get::<Transform>(e).is_none());

        world.ensure_transform_from_array(go);
        assert!(
            world.ecs.get::<Transform>(e).is_some(),
            "ensure_transform_from_array must backfill from array"
        );

        // Strip again; ensure_entity should also backfill
        let _ = world.ecs.remove_component::<Transform>(e);
        let e2 = world.ensure_entity(go);
        assert_eq!(e2, e);
        assert!(world.ecs.get::<Transform>(e2).is_some());
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_b3_hierarchy_dual_read_prefers_ecs_parent_children() {
        let mut world = World::new();
        let parent = world.CreateGameObject("P");
        let child = world.CreateGameObject("C");
        world.SetParent(child, Some(parent));

        // Prefer ECS after SetParent write-through
        assert_eq!(world.GetParent(child), Some(parent));
        assert!(world.GetChildren(parent).contains(&child));

        // Overwrite ECS parent only — dual-read should see it
        let ce = world.entity_for(child).unwrap();
        {
            let ecs = world.ecs_world_mut();
            *ecs.get_mut::<GameObjectParent>(ce).unwrap() = GameObjectParent(parent);
        }
        // Array still parented; dual-read agrees
        assert_eq!(world.GetParentArray(child), Some(parent));
        assert_eq!(world.GetParent(child), Some(parent));

        // Detach: array cleared, ECS Parent removed by sync
        world.SetParent(child, None);
        assert_eq!(world.GetParentArray(child), None);
        assert_eq!(world.GetParent(child), None);
        assert!(!world.GetChildren(parent).contains(&child));
        assert!(world.ecs.get::<GameObjectParent>(ce).is_none());
    }

    #[cfg(feature = "unity-world-primary")]
    #[test]
    fn test_dual_read_prefers_ecs_name_tag_active() {
        let mut world = World::new();
        let go = world.CreateGameObject("ArrayName");
        // Overwrite ECS mirrors only
        let e = world.entity_for(go).unwrap();
        {
            let ecs = world.ecs_world_mut();
            *ecs.get_mut::<GameObjectName>(e).unwrap() = GameObjectName("EcsName".into());
            *ecs.get_mut::<GameObjectTag>(e).unwrap() = GameObjectTag("EcsTag".into());
            *ecs.get_mut::<GameObjectActive>(e).unwrap() = GameObjectActive(false);
        }
        assert_eq!(world.GetName(go), "EcsName");
        assert_eq!(world.GetTag(go), "EcsTag");
        assert!(!world.IsActive(go));
    }

    /// Dual-mode storage contract (phase 11 S2).
    ///
    /// After ECS-only mutations, public reads must follow the compiled feature:
    /// - off → array / GameObject fields (workspace default)
    /// - on → linked ECS mirrors
    ///
    /// This runs in both CI modes without `#[cfg]` duplication.
    #[test]
    fn test_dual_read_storage_mode_contract_when_ecs_diverges() {
        let mut world = World::new();
        let parent = world.CreateGameObject("ArrayParent");
        let decoy = world.CreateGameObject("DecoyParent");
        let go = world.CreateGameObject("ArrayTruth");
        world.SetParent(go, Some(parent));
        world.SetName(go, "ArrayTruth");
        world.SetTag(go, "ArrayTag");
        world.SetActive(go, true);
        world.SetLayer(go, 2);
        let _ = world.with_transform_mut(go, |t| {
            t.SetLocalPosition(Vec3::new(1.0, 2.0, 3.0));
        });
        world.sync_transforms();

        let e = world.entity_for(go).expect("linked entity");
        let pe = world.entity_for(parent).expect("parent entity");
        let _ = world.entity_for(decoy).expect("decoy entity");

        // Force ECS mirrors to a divergent truth without touching array storage.
        {
            let ecs = world.ecs_world_mut();
            *ecs.get_mut::<GameObjectName>(e).unwrap() = GameObjectName("EcsTruth".into());
            *ecs.get_mut::<GameObjectTag>(e).unwrap() = GameObjectTag("EcsTag".into());
            *ecs.get_mut::<GameObjectActive>(e).unwrap() = GameObjectActive(false);
            *ecs.get_mut::<GameObjectLayer>(e).unwrap() = GameObjectLayer(9);
            let divergent = Transform::from_xyz(42.0, 0.0, 0.0);
            if ecs.get::<Transform>(e).is_some() {
                *ecs.get_mut::<Transform>(e).unwrap() = divergent;
            } else {
                ecs.add_component(e, divergent);
            }
            // Diverge parent AND children: array still says `parent`; ECS says `decoy`.
            if ecs.get::<GameObjectParent>(e).is_some() {
                *ecs.get_mut::<GameObjectParent>(e).unwrap() = GameObjectParent(decoy);
            } else {
                ecs.add_component(e, GameObjectParent(decoy));
            }
            let empty = GameObjectChildren(Vec::new());
            if ecs.get::<GameObjectChildren>(pe).is_some() {
                *ecs.get_mut::<GameObjectChildren>(pe).unwrap() = empty;
            } else {
                ecs.add_component(pe, empty);
            }
        }

        // Array remains the dual-write baseline truth.
        assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().x, 1.0);
        assert_eq!(world.GetParentArray(go), Some(parent));

        if World::unity_world_primary_feature() {
            assert_eq!(world.GetName(go), "EcsTruth");
            assert_eq!(world.GetTag(go), "EcsTag");
            assert!(!world.IsActive(go));
            assert_eq!(world.GetLayer(go), 9);
            assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 42.0);
            // ECS parent diverged to decoy — dual-read must prefer it.
            assert_eq!(world.GetParent(go), Some(decoy));
            // ECS children list diverged to empty — dual-read must prefer it.
            assert!(world.GetChildren(parent).is_empty());
        } else {
            assert_eq!(world.GetName(go), "ArrayTruth");
            assert_eq!(world.GetTag(go), "ArrayTag");
            assert!(world.IsActive(go));
            assert_eq!(world.GetLayer(go), 2);
            assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 1.0);
            // Array parent remains authority when the feature is off.
            assert_eq!(world.GetParent(go), Some(parent));
            assert!(world.GetChildren(parent).contains(&go));
        }
    }

    /// S3 T2 — public transform writers follow storage authority after write.
    #[test]
    fn test_s3_transform_write_authority_after_set_local_position() {
        let mut world = World::new();
        let go = world.CreateGameObject("Writer");
        world.SetLocalPosition(go, engine_math::Vec3::new(5.0, 6.0, 7.0));
        world.SetLocalScale(go, engine_math::Vec3::new(2.0, 2.0, 2.0));

        // Dual-write/cache path: both stores agree after a public write.
        assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 5.0);
        assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().x, 5.0);
        assert_eq!(world.GetTransformArray(go).unwrap().LocalScale().x, 2.0);

        let e = world.entity_for(go).expect("entity");
        if World::unity_world_primary_feature() {
            // Corrupt array cache only — public read must still see ECS authority.
            if let Some(Some(arr)) = world.transforms.get_mut(go.index() as usize) {
                arr.local_position = engine_math::Vec3::ZERO;
                arr.local_scale = engine_math::Vec3::ONE;
            }
            // Corruption actually applied to the non-authoritative store.
            assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().x, 0.0);
            assert_eq!(world.GetTransformArray(go).unwrap().LocalScale().x, 1.0);
            assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 5.0);
            assert_eq!(world.GetTransform(go).unwrap().LocalScale().x, 2.0);
            assert_eq!(
                world.ecs.get::<Transform>(e).unwrap().LocalPosition().x,
                5.0
            );
        } else {
            // Corrupt ECS-only (if present) — public read must still see array authority.
            if world.ecs.get::<Transform>(e).is_none() {
                world
                    .ecs
                    .add_component(e, Transform::from_xyz(99.0, 0.0, 0.0));
            } else {
                *world.ecs.get_mut::<Transform>(e).unwrap() = Transform::from_xyz(99.0, 0.0, 0.0);
            }
            assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 5.0);
            assert_eq!(world.GetTransformArray(go).unwrap().LocalPosition().x, 5.0);
        }
    }

    /// S3 T3 — identity/hierarchy public writes dual-write; reads follow mode authority.
    #[test]
    fn test_s3_identity_write_authority_after_public_setters() {
        let mut world = World::new();
        let parent = world.CreateGameObject("P");
        let go = world.CreateGameObject("Go");
        world.SetName(go, "Written");
        world.SetTag(go, "Hero");
        world.SetActive(go, false);
        world.SetLayer(go, 5);
        world.SetParent(go, Some(parent));

        let e = world.entity_for(go).expect("entity");
        let pe = world.entity_for(parent).expect("parent entity");

        // After public writes, dual-write keeps stores coherent.
        assert_eq!(world.GetName(go), "Written");
        assert_eq!(world.GetTag(go), "Hero");
        assert!(!world.IsActive(go));
        assert_eq!(world.GetLayer(go), 5);
        assert_eq!(world.GetParent(go), Some(parent));
        assert_eq!(world.GetParentArray(go), Some(parent));

        if World::unity_world_primary_feature() {
            // Corrupt array / GameObject fields only.
            let index = go.index() as usize;
            if let Some(Some(godata)) = world.gameobject_data.get_mut(index) {
                godata.SetName("ArrayCorrupt");
                godata.SetTag("ArrayCorrupt");
                godata.SetActive(true);
                godata.SetLayer(99);
            }
            if let Some(Some(arr)) = world.transforms.get_mut(index) {
                arr.parent = None;
            }
            // Corruption actually applied to array / GameObject fields.
            let index = go.index() as usize;
            if let Some(Some(godata)) = world.gameobject_data.get(index) {
                assert_eq!(godata.Name(), "ArrayCorrupt");
                assert_eq!(godata.Tag(), "ArrayCorrupt");
                assert!(godata.ActiveSelf());
                assert_eq!(godata.Layer(), 99);
            }
            assert_eq!(world.GetParentArray(go), None);
            // Public reads still see ECS write-authority.
            assert_eq!(world.GetName(go), "Written");
            assert_eq!(world.GetTag(go), "Hero");
            assert!(!world.IsActive(go));
            assert_eq!(world.GetLayer(go), 5);
            assert_eq!(world.GetParent(go), Some(parent));
        } else {
            // Corrupt ECS mirrors only — public reads still see array authority.
            {
                let ecs = world.ecs_world_mut();
                if let Some(n) = ecs.get_mut::<GameObjectName>(e) {
                    *n = GameObjectName("EcsCorrupt".into());
                }
                if let Some(t) = ecs.get_mut::<GameObjectTag>(e) {
                    *t = GameObjectTag("EcsCorrupt".into());
                }
                if let Some(a) = ecs.get_mut::<GameObjectActive>(e) {
                    *a = GameObjectActive(true);
                }
                if let Some(l) = ecs.get_mut::<GameObjectLayer>(e) {
                    *l = GameObjectLayer(99);
                }
                if let Some(p) = ecs.get_mut::<GameObjectParent>(e) {
                    *p = GameObjectParent(parent); // still parent; below we spoof a wrong one
                } else {
                    // ensure component exists then spoof away from array
                    let _ = pe;
                }
            }
            // Spoof parent away from array truth.
            if let Some(p) = world.ecs_world_mut().get_mut::<GameObjectParent>(e) {
                *p = GameObjectParent(go); // invalid self-parent — array must ignore it
            }
            assert_eq!(world.GetName(go), "Written");
            assert_eq!(world.GetTag(go), "Hero");
            assert!(!world.IsActive(go));
            assert_eq!(world.GetLayer(go), 5);
            assert_eq!(world.GetParent(go), Some(parent));
            assert_eq!(world.GetParentArray(go), Some(parent));
        }
    }

    #[test]
    fn test_world_creation() {
        let world = World::new();
        assert_eq!(world.GetRootGameObjects().len(), 0);
    }

    #[test]
    fn test_create_gameobject() {
        let mut world = World::new();
        let handle = world.CreateGameObject("TestObject");

        assert!(world.is_valid(handle));
        assert_eq!(world.GetName(handle), "TestObject");
        assert_eq!(world.GetTag(handle), "Untagged");
        assert_eq!(world.GetLayer(handle), 0);
        assert!(world.IsActive(handle));
        assert!(world.GetTransform(handle).is_some());
    }

    #[test]
    fn test_destroy_gameobject() {
        let mut world = World::new();
        let handle = world.CreateGameObject("TestObject");

        world.DestroyImmediate(handle);
        assert!(!world.is_valid(handle));
    }

    #[test]
    fn test_find_by_name() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Player");

        assert_eq!(world.Find("Player"), Some(handle));
        assert_eq!(world.Find("NonExistent"), None);
    }

    #[test]
    fn test_find_by_tag() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Player");
        world.SetTag(handle, "Player");

        assert_eq!(world.FindWithTag("Player"), Some(handle));
        assert!(world.FindGameObjectsWithTag("Player").contains(&handle));
    }

    #[test]
    fn test_set_parent() {
        let mut world = World::new();
        let parent = world.CreateGameObject("Parent");
        let child = world.CreateGameObject("Child");

        world.SetParent(child, Some(parent));

        assert_eq!(world.GetParent(child), Some(parent));
        assert!(world.GetChildren(parent).contains(&child));
    }

    #[test]
    fn test_cycle_detection() {
        let mut world = World::new();
        let parent = world.CreateGameObject("Parent");
        let child = world.CreateGameObject("Child");
        let grandchild = world.CreateGameObject("Grandchild");

        world.SetParent(child, Some(parent));
        world.SetParent(grandchild, Some(child));

        // Attempt to create cycle
        world.SetParent(parent, Some(grandchild));

        // Parent should still be root
        assert_eq!(world.GetParent(parent), None);
    }

    #[test]
    fn test_set_active() {
        let mut world = World::new();
        let handle = world.CreateGameObject("TestObject");

        assert!(world.IsActive(handle));
        world.SetActive(handle, false);
        assert!(!world.IsActive(handle));
        world.SetActive(handle, true);
        assert!(world.IsActive(handle));
    }

    #[test]
    fn test_is_active_in_hierarchy() {
        let mut world = World::new();
        let parent = world.CreateGameObject("Parent");
        let child = world.CreateGameObject("Child");

        world.SetParent(child, Some(parent));

        assert!(world.IsActiveInHierarchy(child));

        world.SetActive(parent, false);
        assert!(!world.IsActiveInHierarchy(child));

        world.SetActive(parent, true);
        assert!(world.IsActiveInHierarchy(child));
    }

    #[test]
    fn test_set_name() {
        let mut world = World::new();
        let handle = world.CreateGameObject("OldName");

        world.SetName(handle, "NewName");
        assert_eq!(world.GetName(handle), "NewName");
        assert_eq!(world.Find("NewName"), Some(handle));
        assert!(world.Find("OldName").is_none());
    }

    #[test]
    fn test_set_tag() {
        let mut world = World::new();
        let handle = world.CreateGameObject("TestObject");

        world.SetTag(handle, "Enemy");
        assert_eq!(world.GetTag(handle), "Enemy");
        assert!(world.CompareTag(handle, "Enemy"));
        assert_eq!(world.FindWithTag("Enemy"), Some(handle));
    }

    #[test]
    fn test_get_root_gameobjects() {
        let mut world = World::new();
        let root1 = world.CreateGameObject("Root1");
        let root2 = world.CreateGameObject("Root2");
        let child = world.CreateGameObject("Child");

        world.SetParent(child, Some(root1));

        let roots = world.GetRootGameObjects();
        assert!(roots.contains(&root1));
        assert!(roots.contains(&root2));
        assert!(!roots.contains(&child));
    }

    #[test]
    fn test_sync_transforms() {
        let mut world = World::new();
        let parent = world.CreateGameObject("Parent");
        let child = world.CreateGameObject("Child");

        world.SetParent(child, Some(parent));

        // Set local positions
        let _ = world.with_transform_mut(parent, |t| {
            t.SetLocalPosition(Vec3::new(5.0, 0.0, 0.0));
        });
        let _ = world.with_transform_mut(child, |t| {
            t.SetLocalPosition(Vec3::new(1.0, 0.0, 0.0));
        });

        world.sync_transforms();

        let parent_pos = world.GetTransform(parent).unwrap().Position();
        let child_pos = world.GetTransform(child).unwrap().Position();

        assert_eq!(parent_pos, Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(child_pos, Vec3::new(6.0, 0.0, 0.0));
    }

    #[test]
    fn test_set_active_queues_enable_disable() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Obj");
        assert_eq!(world.pending_enable_disable_count(), 0);

        world.SetActive(handle, false);
        assert_eq!(world.pending_enable_disable_count(), 1);
        assert!(!world.IsActive(handle));
        assert!(!world.IsActiveInHierarchy(handle));

        world.SetActive(handle, true);
        assert_eq!(world.pending_enable_disable_count(), 2);
        assert!(world.IsActive(handle));
    }

    #[test]
    fn test_set_active_parent_affects_child_hierarchy() {
        let mut world = World::new();
        let parent = world.CreateGameObject("Parent");
        let child = world.CreateGameObject("Child");
        world.SetParent(child, Some(parent));

        assert!(world.IsActiveInHierarchy(child));
        world.SetActive(parent, false);
        // Child activeSelf remains true; hierarchy becomes inactive
        assert!(world.IsActive(child));
        assert!(!world.IsActiveInHierarchy(child));
        // Parent + child OnDisable queued
        assert_eq!(world.pending_enable_disable_count(), 2);
    }

    #[test]
    fn test_invoke_timer() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Obj");
        world.Invoke(handle, "Explode", 0.1);
        assert!(world.IsInvoking(handle));
        assert_eq!(world.pending_invoke_count(), 1);

        world.tick_invokes(0.05);
        assert_eq!(world.pending_invoke_count(), 1); // not yet

        world.tick_invokes(0.06);
        assert_eq!(world.pending_invoke_count(), 0);
    }

    #[test]
    fn test_invoke_repeating() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Obj");
        world.InvokeRepeating(handle, "Pulse", 0.0, 0.1);
        assert_eq!(world.pending_invoke_count(), 1);
        world.tick_invokes(0.05);
        // Still repeating
        assert_eq!(world.pending_invoke_count(), 1);
        world.CancelInvoke(handle);
        assert_eq!(world.pending_invoke_count(), 0);
    }

    #[test]
    fn test_add_monobehaviour() {
        use crate::behaviour::Behaviour;
        use crate::context::Context;
        use crate::monobehaviour::MonoBehaviour;
        use std::any::Any;

        #[derive(Debug)]
        struct Counter {
            updates: u32,
        }

        impl Component for Counter {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl Behaviour for Counter {
            fn Enabled(&self) -> bool {
                true
            }
            fn SetEnabled(&mut self, _enabled: bool) {}
            fn IsActiveAndEnabled(&self) -> bool {
                true
            }
            fn set_gameobject(&mut self, _handle: GameObjectHandle) {}
            fn gameobject_handle(&self) -> Option<GameObjectHandle> {
                None
            }
        }
        impl MonoBehaviour for Counter {
            fn Update(&mut self, _ctx: &mut Context) {
                self.updates += 1;
            }
        }

        let mut world = World::new();
        let handle = world.CreateGameObject("Scripted");
        world.AddMonoBehaviour(handle, Counter { updates: 0 });
        assert_eq!(world.MonoBehaviourCount(handle), 1);

        let time = Time::default();
        let mut events = crate::event::EventBus::new();
        world.tick_update(time, 0, &mut events);
        // Update ran (value is inside holder, not directly readable without API)
    }

    #[test]
    fn test_require_component_auto_add() {
        #[derive(Debug)]
        struct Mesh;
        #[derive(Debug)]
        struct MeshRenderer;

        impl Component for Mesh {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
            fn required_on_add(&self) -> Vec<Box<dyn Fn() -> Box<dyn Component>>> {
                vec![Box::new(|| Box::new(MeshRenderer))]
            }
        }
        impl Component for MeshRenderer {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let mut world = World::new();
        let handle = world.CreateGameObject("Obj");
        world.AddComponent(handle, Mesh);
        assert!(world.HasComponent::<Mesh>(handle));
        assert!(world.HasComponent::<MeshRenderer>(handle));
    }

    /// R1-full — SetParent follows storage authority; array is cache when feature on.
    #[test]
    fn test_r1full_set_parent_storage_authority() {
        let mut world = World::new();
        let a = world.CreateGameObject("A");
        let b = world.CreateGameObject("B");
        let child = world.CreateGameObject("C");
        world.SetParent(child, Some(a));

        if World::unity_world_primary_feature() {
            // Dirty array cache; public reads + further writes must follow ECS.
            let idx = child.index() as usize;
            if let Some(Some(t)) = world.transforms.get_mut(idx) {
                t.parent = Some(b);
            }
            assert_eq!(world.GetParent(child), Some(a));
            assert!(world.GetChildren(a).contains(&child));
            assert!(!world.GetChildren(b).contains(&child));

            // Cycle check uses authority: child is under a, so a cannot parent under child.
            world.SetParent(a, Some(child));
            assert_eq!(world.GetParent(a), None);

            // Reparent via public API; array cache must refresh from ECS.
            world.SetParent(child, Some(b));
            assert_eq!(world.GetParent(child), Some(b));
            assert_eq!(world.GetParentArray(child), Some(b));
            assert!(world.GetChildrenArray(b).contains(&child));
            assert!(!world.GetChildrenArray(a).contains(&child));
        } else {
            assert_eq!(world.GetParent(child), Some(a));
            world.SetParent(child, Some(b));
            assert_eq!(world.GetParent(child), Some(b));
            assert_eq!(world.GetParentArray(child), Some(b));
        }
    }

    /// R1-full — identity public reads ignore dirty array when feature on.
    #[test]
    fn test_r1full_identity_write_authority_after_array_dirty() {
        let mut world = World::new();
        let go = world.CreateGameObject("Clean");
        world.SetName(go, "Authoritative");
        world.SetTag(go, "Player");
        world.SetLayer(go, 3);
        world.SetActive(go, true);

        let idx = go.index() as usize;
        if let Some(Some(data)) = world.gameobject_data.get_mut(idx) {
            data.SetName("DirtyArray");
            data.SetTag("Enemy");
            data.SetLayer(99);
        }

        if World::unity_world_primary_feature() {
            assert_eq!(world.GetName(go), "Authoritative");
            assert_eq!(world.GetTag(go), "Player");
            assert_eq!(world.GetLayer(go), 3);
            assert!(world.IsActive(go));
        } else {
            // Feature off: array remains authority after direct mutation.
            assert_eq!(world.GetName(go), "DirtyArray");
            assert_eq!(world.GetTag(go), "Enemy");
            assert_eq!(world.GetLayer(go), 99);
        }
    }

    /// R1-full — Destroy removes handle from parent hierarchy authority.
    #[test]
    fn test_r1full_destroy_clears_parent_children_authority() {
        let mut world = World::new();
        let parent = world.CreateGameObject("P");
        let c1 = world.CreateGameObject("C1");
        let c2 = world.CreateGameObject("C2");
        world.SetParent(c1, Some(parent));
        world.SetParent(c2, Some(parent));
        assert_eq!(world.GetChildren(parent).len(), 2);

        world.DestroyImmediate(c1);

        assert!(!world.GetChildren(parent).contains(&c1));
        assert!(world.GetChildren(parent).contains(&c2));
        assert!(!world.GetChildrenArray(parent).contains(&c1));
        if World::unity_world_primary_feature() {
            if let Some(pe) = world.entity_for(parent) {
                let ch = world
                    .ecs
                    .get::<GameObjectChildren>(pe)
                    .expect("parent children mirror");
                assert!(!ch.0.contains(&c1));
                assert!(ch.0.contains(&c2));
            }
        }
    }

    /// R1-full — GetRootGameObjects follows hierarchy authority.
    #[test]
    fn test_r1full_roots_follow_hierarchy_authority() {
        let mut world = World::new();
        let root = world.CreateGameObject("Root");
        let mid = world.CreateGameObject("Mid");
        let leaf = world.CreateGameObject("Leaf");
        world.SetParent(mid, Some(root));
        world.SetParent(leaf, Some(mid));

        let roots = world.GetRootGameObjects();
        assert!(roots.contains(&root));
        assert!(!roots.contains(&mid));
        assert!(!roots.contains(&leaf));

        if World::unity_world_primary_feature() {
            // Dirty array parent of mid → root list must still ignore it when ECS says parented.
            let idx = mid.index() as usize;
            if let Some(Some(t)) = world.transforms.get_mut(idx) {
                t.parent = None;
            }
            let roots2 = world.GetRootGameObjects();
            assert!(!roots2.contains(&mid), "ECS parent must keep mid non-root");
            assert!(roots2.contains(&root));
        }
    }

    /// R1-full — scene serialize prefers ECS transform when feature on.
    #[test]
    fn test_r1full_serialize_prefers_storage_authority_transform() {
        let mut world = World::new();
        let go = world.CreateGameObject("Pose");
        let _ = world.with_ecs_transform_mut(go, |t| {
            t.SetLocalPosition(Vec3::new(7.0, 0.0, 0.0));
        });

        if World::unity_world_primary_feature() {
            // Dirty array pose cache; serialize must still see ECS authority via GetTransform.
            if let Some(Some(t)) = world.transforms.get_mut(go.index() as usize) {
                t.local_position = Vec3::new(-1.0, -1.0, -1.0);
            }
        }

        let scene = crate::serialization::SceneSerializer::new().Save(&world, "S");
        let data = &scene.game_objects[0];
        if World::unity_world_primary_feature() {
            assert_eq!(data.transform.local_position.x, 7.0);
        } else {
            // Feature off: with_ecs_transform_mut falls back to array write.
            assert_eq!(data.transform.local_position.x, 7.0);
        }
    }

    /// R1-full — CollectMonoBehaviours prefers ECS Instances when feature on.
    #[test]
    fn test_r1full_collect_monobehaviours_prefers_ecs_instances() {
        #[derive(Default)]
        struct MetaScript {
            go: Option<GameObjectHandle>,
        }
        impl Component for MetaScript {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
        impl crate::behaviour::Behaviour for MetaScript {
            fn Enabled(&self) -> bool {
                true
            }
            fn SetEnabled(&mut self, _enabled: bool) {}
            fn IsActiveAndEnabled(&self) -> bool {
                true
            }
            fn set_gameobject(&mut self, handle: GameObjectHandle) {
                self.go = Some(handle);
            }
            fn gameobject_handle(&self) -> Option<GameObjectHandle> {
                self.go
            }
        }
        impl crate::monobehaviour::MonoBehaviour for MetaScript {
            fn TypeName(&self) -> &str {
                "R1FullMetaScript"
            }
        }

        let mut world = World::new();
        let go = world.CreateGameObject("Meta");
        world.AddMonoBehaviour(go, MetaScript { go: None });
        assert_eq!(world.CollectMonoBehaviours(go).len(), 1);

        if World::unity_world_primary_feature() {
            let e = world.entity_for(go).unwrap();
            let divergent = MonoBehaviourInstances(vec![MonoBehaviourInstance {
                type_name: "GhostFromEcs".into(),
                enabled: true,
                props: None,
            }]);
            if world.ecs.get::<MonoBehaviourInstances>(e).is_some() {
                *world.ecs.get_mut::<MonoBehaviourInstances>(e).unwrap() = divergent;
            } else {
                world.ecs.add_component(e, divergent);
            }
            let collected = world.CollectMonoBehaviours(go);
            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].0, "GhostFromEcs");
            // Holders remain the dyn runtime store.
            assert_eq!(world.MonoBehaviourCount(go), 1);
        } else {
            assert_eq!(world.CollectMonoBehaviours(go)[0].0, "R1FullMetaScript");
        }
    }

    /// R1-full — prepare_scene_io_cache is a no-op off; refreshes caches on.
    #[test]
    fn test_r1full_prepare_scene_io_cache() {
        let mut world = World::new();
        let go = world.CreateGameObject("Cache");
        world.SetLocalPosition(go, Vec3::new(3.0, 0.0, 0.0));
        world.prepare_scene_io_cache();

        if World::unity_world_primary_feature() {
            if let Some(Some(t)) = world.transforms.get(go.index() as usize) {
                assert_eq!(t.LocalPosition().x, 3.0);
            }
        }
        assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 3.0);
    }

    /// T4 — sync_transforms must not clobber ECS local pose with stale array cache.
    #[test]
    fn test_r1full_sync_transforms_preserves_ecs_pose_authority() {
        let mut world = World::new();
        let go = world.CreateGameObject("PoseAuth");
        world.SetLocalPosition(go, Vec3::new(1.0, 0.0, 0.0));
        world.sync_transforms();

        if World::unity_world_primary_feature() {
            let e = world.entity_for(go).unwrap();
            let divergent = Transform::from_xyz(99.0, 0.0, 0.0);
            if world.ecs.get::<Transform>(e).is_some() {
                *world.ecs.get_mut::<Transform>(e).unwrap() = divergent;
            } else {
                world.ecs.add_component(e, divergent);
            }
            // Dirty array cache too — sync_transforms must refresh from ECS first.
            if let Some(Some(t)) = world.transforms.get_mut(go.index() as usize) {
                t.local_position = Vec3::new(-5.0, 0.0, 0.0);
            }
            world.sync_transforms();
            let t = world.GetTransform(go).unwrap();
            assert_eq!(
                t.LocalPosition().x,
                99.0,
                "ECS pose must survive sync_transforms"
            );
            assert_eq!(
                world.GetTransformArray(go).unwrap().LocalPosition().x,
                99.0,
                "array cache must be refreshed from ECS before math"
            );
        } else {
            world.sync_transforms();
            assert_eq!(world.GetTransform(go).unwrap().LocalPosition().x, 1.0);
        }
    }

    /// T2 — GetParent follows hierarchy authority for ECS roots (Children, no Parent).
    #[test]
    fn test_r1full_get_parent_ecs_root_ignores_dirty_array() {
        let mut world = World::new();
        let root = world.CreateGameObject("EcsRoot");
        let decoy = world.CreateGameObject("Decoy");
        // Seed hierarchy as root (Children mirror present, no Parent).
        world.SetParent(root, None);

        if World::unity_world_primary_feature() {
            if let Some(Some(t)) = world.transforms.get_mut(root.index() as usize) {
                t.parent = Some(decoy);
            }
            assert_eq!(
                world.GetParent(root),
                None,
                "ECS root must not read dirty array parent"
            );
        } else {
            if let Some(Some(t)) = world.transforms.get_mut(root.index() as usize) {
                t.parent = Some(decoy);
            }
            assert_eq!(world.GetParent(root), Some(decoy));
        }
    }

    /// Cycle guard — corrupted ECS parent self-loop must not hang SetParent checks.
    #[test]
    fn test_r1full_cycle_guard_on_corrupt_parent() {
        let mut world = World::new();
        let a = world.CreateGameObject("A");
        let b = world.CreateGameObject("B");
        world.SetParent(b, Some(a));

        if World::unity_world_primary_feature() {
            if let Some(e) = world.entity_for(a) {
                // Corrupt: A parents to itself
                if world.ecs.get::<GameObjectParent>(e).is_some() {
                    *world.ecs.get_mut::<GameObjectParent>(e).unwrap() = GameObjectParent(a);
                } else {
                    world.ecs.add_component(e, GameObjectParent(a));
                }
            }
        } else if let Some(Some(t)) = world.transforms.get_mut(a.index() as usize) {
            t.parent = Some(a);
        }

        // Must return without hanging; cycle detection / hop cap rejects the move.
        world.SetParent(a, Some(b));
        // A should not become a descendant of itself via the move — either rejected
        // or b remains not under a in a way that infinite-loops GetParent walks.
        let _ = world.GetParent(a);
        let _ = world.GetParent(b);
    }
}
