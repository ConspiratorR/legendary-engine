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
use crate::transform::Transform;
use engine_math::{Quat, Vec3};

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

    // === Pending operations ===
    pending_destroy: Vec<PendingDestroy>,
    pending_invokes: Vec<PendingInvoke>,

    // === DontDestroyOnLoad tracking ===
    dont_destroy: Vec<GameObjectHandle>,

    // === Instance ID counter ===
    next_instance_id: i32,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("gameobject_count", &self.gameobject_data.iter().filter(|g| g.is_some()).count())
            .field("transform_count", &self.transforms.iter().filter(|t| t.is_some()).count())
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
            pending_destroy: Vec::new(),
            pending_invokes: Vec::new(),
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
        let mut go = GameObject::new_with_name(name);
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

    /// Internal destroy implementation.
    fn destroy_internal(&mut self, handle: GameObjectHandle) {
        if !self.is_valid(handle) {
            return;
        }

        let index = handle.index() as usize;

        // Call OnDestroy on MonoBehaviours
        if let Some(monos) = self.monobehaviours.get_mut(index) {
            if let Some(monos) = monos {
                for mono in monos.iter_mut() {
                    // MonoBehaviour lifecycle will be called by runner
                }
            }
        }

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
    pub fn AddComponent<T: Component + 'static>(&mut self, handle: GameObjectHandle, component: T) -> &mut T {
        let index = handle.index() as usize;

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

    /// Get a component from a GameObject (matches `GameObject.GetComponent<T>()`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.GetComponent.html>
    pub fn GetComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get(index) {
            if let Some(go) = go {
                return go.GetComponent::<T>();
            }
        }

        None
    }

    /// Get a mutable component (matches `GameObject.GetComponent<T>()` with write access).
    pub fn GetComponentMut<T: Component + 'static>(
        &mut self,
        handle: GameObjectHandle,
    ) -> Option<&mut T> {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                return go.GetComponentMut::<T>();
            }
        }

        None
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

    // ============================================================
    // Transform Access (built-in)
    // ============================================================

    /// Get the Transform of a GameObject (matches `GameObject.transform`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject-transform.html>
    ///
    /// Returns `Some` if the handle is valid. Transform is mandatory on every GameObject.
    pub fn GetTransform(&self, handle: GameObjectHandle) -> Option<&Transform> {
        let index = handle.index() as usize;
        self.transforms.get(index)?.as_ref()
    }

    /// Get a mutable Transform reference.
    pub fn GetTransformMut(&mut self, handle: GameObjectHandle) -> Option<&mut Transform> {
        let index = handle.index() as usize;
        self.transforms.get_mut(index)?.as_mut()
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
        let old_parent = self.transforms[child_index]
            .as_ref()
            .and_then(|t| t.parent);

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
    pub fn SetActive(&mut self, handle: GameObjectHandle, active: bool) {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                let was_active = go.ActiveSelf();
                go.SetActive(active);

                // Call OnEnable/OnDisable on MonoBehaviours
                if was_active != active {
                    if let Some(monos) = self.monobehaviours.get_mut(index) {
                        if let Some(monos) = monos {
                            for mono in monos.iter_mut() {
                                if active {
                                    // OnEnable will be called by runner
                                } else {
                                    // OnDisable will be called by runner
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get active state (matches `GameObject.activeSelf`).
    pub fn IsActive(&self, handle: GameObjectHandle) -> bool {
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
    }

    /// Get name (matches `Object.name`).
    pub fn GetName(&self, handle: GameObjectHandle) -> &str {
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
    }

    /// Get tag (matches `GameObject.tag`).
    pub fn GetTag(&self, handle: GameObjectHandle) -> &str {
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
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/GameObject.SendMessage.html>
    pub fn SendMessage(&mut self, handle: GameObjectHandle, method: &str) {
        self.SendMessageWithValue(handle, method, &());
    }

    /// Send message with a value (matches `GameObject.SendMessage(methodName, value)`).
    pub fn SendMessageWithValue(&mut self, handle: GameObjectHandle, method: &str, _value: &dyn Any) {
        let index = handle.index() as usize;

        if let Some(go) = self.gameobject_data.get_mut(index) {
            if let Some(go) = go {
                // SendMessage dispatches to MonoBehaviour components
                // For now, this is a placeholder - the actual implementation
                // would use reflection or a message registry
                let _ = method;
            }
        }
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
    // Lifecycle Dispatch (internal)
    // ============================================================

    /// Process pending destroys (called at end of frame).
    pub(crate) fn flush_destroy(&mut self) {
        let pending: Vec<GameObjectHandle> = self
            .pending_destroy
            .drain(..)
            .filter(|p| p.delay <= 0.0)
            .map(|p| p.handle)
            .collect();

        for handle in pending {
            self.destroy_internal(handle);
        }
    }

    /// Update pending destroy delays.
    pub(crate) fn update_pending_destroy(&mut self, delta_time: f32) {
        for pending in self.pending_destroy.iter_mut() {
            pending.elapsed += delta_time;
        }
    }

    /// Sync all transforms (called by update system).
    pub(crate) fn sync_transforms(&mut self) {
        let roots = self.GetRootGameObjects();
        for root in roots {
            self.sync_transform_recursive(root, true);
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
                self.GetTransform(ph).map(|t| (t.Position(), t.Rotation(), t.LossyScale()))
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
}
