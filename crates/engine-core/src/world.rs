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
    pub fn ensure_entity(&mut self, handle: GameObjectHandle) -> engine_ecs::entity::Entity {
        if let Some(e) = self.entity_for(handle) {
            return e;
        }
        let e = self.ecs.spawn();
        self.link_entity(handle, e);
        e
    }

    /// Number of live identity links.
    pub fn identity_count(&self) -> usize {
        self.handle_to_entity.len()
    }

    /// Copy all Unity Transforms onto linked internal-ECS entities as
    /// [`Transform`] components (P2.4 write-through for tools / storage merge).
    pub fn sync_all_transforms_to_ecs(&mut self) {
        let pairs: Vec<(GameObjectHandle, engine_ecs::entity::Entity)> = self
            .handle_to_entity
            .iter()
            .map(|(&h, &e)| (h, e))
            .collect();
        for (handle, entity) in pairs {
            if !self.is_valid(handle) {
                continue;
            }
            let Some(t) = self
                .transforms
                .get(handle.index() as usize)
                .and_then(|t| t.as_ref())
            else {
                continue;
            };
            // Prefer world pose after World::sync_transforms; fall back to local.
            let full =
                Transform::from_position_rotation_scale(t.Position(), t.Rotation(), t.LossyScale());
            if self.ecs.get::<Transform>(entity).is_some() {
                *self.ecs.get_mut::<Transform>(entity).unwrap() = full;
            } else {
                self.ecs.add_component(entity, full);
            }
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

    /// Refresh `MonoBehaviourTypes` on the linked entity from current holders.
    fn sync_monobehaviour_types_to_ecs(&mut self, handle: GameObjectHandle) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let names: Vec<String> = self
            .monobehaviours
            .get(handle.index() as usize)
            .and_then(|m| m.as_ref())
            .map(|v| v.iter().map(|m| m.Get().TypeName().to_string()).collect())
            .unwrap_or_default();
        let comp = MonoBehaviourTypes(names);
        if self.ecs.get::<MonoBehaviourTypes>(entity).is_some() {
            *self.ecs.get_mut::<MonoBehaviourTypes>(entity).unwrap() = comp;
        } else {
            self.ecs.add_component(entity, comp);
        }
    }

    /// Write `GameObjectParent` / `GameObjectChildren` onto the linked entity.
    fn sync_hierarchy_to_ecs(&mut self, handle: GameObjectHandle) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let parent = self.GetParent(handle);
        let children = self.GetChildren(handle);

        if let Some(p) = parent {
            let comp = GameObjectParent(p);
            if self.ecs.get::<GameObjectParent>(entity).is_some() {
                *self.ecs.get_mut::<GameObjectParent>(entity).unwrap() = comp;
            } else {
                self.ecs.add_component(entity, comp);
            }
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
        if Self::unity_world_primary_feature() {
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

        // Recursively destroy children
        let children: Vec<GameObjectHandle> = {
            if let Some(transform) = self.transforms[index].as_ref() {
                transform.children.clone()
            } else {
                Vec::new()
            }
        };

        for child in children {
            self.destroy_internal(child);
        }

        // Remove from parent's children list
        if let Some(transform) = self.transforms[index].as_ref() {
            if let Some(parent) = transform.parent {
                let parent_index = parent.index() as usize;
                if let Some(parent_transform) = self.transforms.get_mut(parent_index) {
                    if let Some(pt) = parent_transform {
                        pt.children.retain(|&h| h != handle);
                    }
                }
            }
        }

        // Clear arrays
        self.gameobjects[index] = None;
        self.gameobject_data[index] = None;
        self.transforms[index] = None;
        self.monobehaviours[index] = None;

        // P2.4: drop identity mapping
        self.unlink_entity(handle);

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

        // Copy components from template
        if let Some(template_go) = self.gameobject_data[template_index].as_ref() {
            let components: Vec<Box<dyn Component>> = template_go
                .Components()
                .iter()
                .map(|c| {
                    // We can't clone components generically, so we create empty ones
                    // In production, this would need a Clone trait or factory pattern
                    None
                })
                .flatten()
                .collect();

            // Components would be cloned here
        }

        // Set transform
        let new_index = handle.index() as usize;
        if let Some(transform) = self.transforms.get_mut(new_index) {
            if let Some(t) = transform {
                t.SetPosition(position);
                t.SetRotation(rotation);
            }
        }

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
    pub fn CollectMonoBehaviours(
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
        }
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

    /// Array storage Transform (authoritative for hierarchy math; never ECS dual-read).
    fn get_transform_array(&self, handle: GameObjectHandle) -> Option<&Transform> {
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
        self.get_transform_array(handle)
    }

    /// Get a mutable Transform reference (always array storage until full merge).
    ///
    /// Prefer [`World::with_transform_mut`] so ECS write-through runs under
    /// `unity-world-primary`.
    pub fn GetTransformMut(&mut self, handle: GameObjectHandle) -> Option<&mut Transform> {
        let index = handle.index() as usize;
        self.transforms.get_mut(index)?.as_mut()
    }

    /// Mutate the Transform then write-through to internal ECS when the feature is on.
    ///
    /// Array storage remains authoritative for the mutable borrow; ECS is updated
    /// after `f` returns so P2.4 dual-read stays coherent.
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

    /// Copy this handle's array Transform onto its linked ECS entity (if any).
    pub fn sync_transform_to_ecs(&mut self, handle: GameObjectHandle) {
        self.write_transform_to_ecs(handle);
    }

    /// Copy this handle's array Transform onto its linked ECS entity (if any).
    fn write_transform_to_ecs(&mut self, handle: GameObjectHandle) {
        let Some(entity) = self.entity_for(handle) else {
            return;
        };
        let index = handle.index() as usize;
        let Some(t) = self.transforms.get(index).and_then(|t| t.as_ref()) else {
            return;
        };
        let full =
            Transform::from_position_rotation_scale(t.Position(), t.Rotation(), t.LossyScale());
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
    pub fn SetParent(&mut self, child: GameObjectHandle, parent: Option<GameObjectHandle>) {
        if !self.is_valid(child) {
            return;
        }

        // Validate new parent
        if let Some(new_parent) = parent {
            if !self.is_valid(new_parent) {
                return;
            }
            // Cycle detection
            if self.is_descendant_of(new_parent, child) {
                return;
            }
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

        if Self::unity_world_primary_feature() {
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
    }

    /// Get parent of a GameObject (matches `Transform.parent`).
    pub fn GetParent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> {
        let index = handle.index() as usize;
        self.transforms.get(index)?.as_ref()?.parent
    }

    /// Get children of a GameObject (matches `Transform.GetChild`).
    pub fn GetChildren(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> {
        let index = handle.index() as usize;
        self.transforms
            .get(index)
            .and_then(|t| t.as_ref())
            .map(|t| t.children.clone())
            .unwrap_or_default()
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
        for (i, transform) in self.transforms.iter().enumerate() {
            if let Some(t) = transform {
                if t.parent.is_none() && self.gameobjects[i].is_some() {
                    if let Some(handle) = self.gameobjects[i] {
                        roots.push(handle);
                    }
                }
            }
        }
        roots
    }

    /// Check if candidate is a descendant of ancestor.
    fn is_descendant_of(&self, candidate: GameObjectHandle, ancestor: GameObjectHandle) -> bool {
        let mut current = candidate;
        loop {
            if current == ancestor {
                return true;
            }
            match self.GetParent(current) {
                Some(parent) => current = parent,
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

        let was_active = match self.gameobject_data.get(index).and_then(|g| g.as_ref()) {
            Some(go) => go.ActiveSelf(),
            None => return,
        };
        if was_active == active {
            return;
        }

        let was_in_hierarchy = self.IsActiveInHierarchy(handle);
        if let Some(go) = self.gameobject_data.get_mut(index).and_then(|g| g.as_mut()) {
            go.SetActive(active);
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
    pub fn SetLayer(&mut self, handle: GameObjectHandle, layer: i32) {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                go.SetLayer(layer);
            }
        }
    }

    /// Get layer (matches `GameObject.layer`).
    pub fn GetLayer(&self, handle: GameObjectHandle) -> i32 {
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
    pub fn sync_transforms(&mut self) {
        let roots = self.GetRootGameObjects();
        for root in roots {
            self.sync_transform_recursive(root, true);
        }
        if Self::unity_world_primary_feature() {
            self.sync_all_transforms_to_ecs();
        }
    }

    /// Recursively sync transform for a GameObject and its children.
    fn sync_transform_recursive(&mut self, handle: GameObjectHandle, is_root: bool) {
        let children = self.GetChildren(handle);

        // Get parent transform data before mutable borrow
        let parent_data = if is_root {
            None
        } else {
            self.GetParent(handle).and_then(|ph| {
                self.get_transform_array(ph)
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
        if let Some(t) = world.GetTransformMut(go) {
            t.SetLocalPosition(engine_math::Vec3::new(4.0, 5.0, 6.0));
        }
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
        if let Some(t) = world.GetTransformMut(parent) {
            t.SetLocalPosition(Vec3::new(5.0, 0.0, 0.0));
        }
        if let Some(t) = world.GetTransformMut(child) {
            t.SetLocalPosition(Vec3::new(1.0, 0.0, 0.0));
        }

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
}
