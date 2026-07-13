# Unity Architecture Refactoring Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor RustEngine's core architecture to match Unity's documented architecture — unified World, GameObject-based entity model, built-in Transform, and PascalCase public API.

**Architecture:** Merge the two separate World types (engine-core World + engine-ecs World) into one unified World that uses sparse-set ECS internally but exposes a Unity-style GameObject API. Transform is mandatory on every entity and cannot be removed. Components are stored on GameObjects. Editor operates directly on the same World.

**Tech Stack:** Rust, engine-ecs (sparse-set), engine-core, engine-scene, engine-editor

---

## Architecture Overview

### Current State (Dual World)
```
engine-core::World (OO, GameObject-based)     engine-ecs::World (Sparse-set ECS)
  Vec<Option<GameObject>>                       ComponentRegistry (HashMap<TypeId, SparseSet<T>>)
  HashMap<String, Vec<Handle>>                  GenerationalEntity IDs
  ↓                                              ↓
  engine-scene::SceneManager                    engine-scene::SceneManager
  (uses engine-ecs::World internally)           (used by scripting, editor runtime)
```

### Target State (Unified World)
```
engine-core::World
  ├── SparseSet storage (from engine-ecs) — internal implementation
  ├── GameObject API layer — public interface
  ├── Built-in Transform on every entity
  ├── Component storage on GameObjects (Vec<Box<dyn Component>>)
  ├── Parent/child hierarchy (built into Transform)
  └── MonoBehaviour lifecycle dispatch
```

---

## Task 1: Rewrite engine-core/src/world.rs — Unified World

**Files:**
- Rewrite: `crates/engine-core/src/world.rs`
- Modify: `crates/engine-core/src/gameobject.rs`

### Current world.rs (383 lines)
Uses `Vec<Option<GameObject>>` with manual generation tracking. No ECS integration.

### Target architecture
The World should:
1. Use `engine_ecs::World` internally for entity generation and sparse-set storage
2. Store `GameObject` structs separately (not as ECS components)
3. Maintain bidirectional entity<->handle mapping
4. Provide Unity-style API: `Find()`, `FindWithTag()`, `Instantiate()`, `Destroy()`

### Implementation

```rust
// crates/engine-core/src/world.rs

use crate::gameobject::{GameObject, GameObjectHandle, Component};
use crate::transform::Transform;
use std::collections::HashMap;

/// Unified world container (matches Unity's scene concept).
/// Uses sparse-set ECS internally, exposes GameObject API externally.
pub struct World {
    // Internal ECS — hidden from public API
    ecs: engine_ecs::World,

    // GameObject storage — indexed by entity index
    gameobjects: Vec<Option<GameObjectHandle>>,
    generations: Vec<u32>,
    free_list: Vec<u32>,

    // Name lookup
    name_to_handles: HashMap<String, Vec<GameObjectHandle>>,

    // Tag lookup
    tag_to_handles: HashMap<String, Vec<GameObjectHandle>>,

    // Pending destroy list (deferred like Unity)
    pending_destroy: Vec<GameObjectHandle>,
}

impl World {
    pub fn new() -> Self { ... }

    // === GameObject Lifecycle (Unity API) ===

    /// Create a new empty GameObject (like Unity's new GameObject("name")).
    pub fn CreateGameObject(&mut self, name: &str) -> GameObjectHandle { ... }

    /// Instantiate a clone of a prefab/template (like Unity's Instantiate()).
    pub fn Instantiate(&mut self, template: GameObjectHandle) -> GameObjectHandle { ... }

    /// Destroy a GameObject at end of frame (like Unity's Destroy()).
    pub fn Destroy(&mut self, handle: GameObjectHandle) { ... }

    /// Immediately destroy a GameObject.
    pub fn DestroyImmediate(&mut self, handle: GameObjectHandle) { ... }

    /// Don't destroy when loading a new scene (like Unity's DontDestroyOnLoad()).
    pub fn DontDestroyOnLoad(&mut self, handle: GameObjectHandle) { ... }

    // === Find API (Unity style) ===

    /// Find a GameObject by name (like Unity's GameObject.Find()).
    pub fn Find(&self, name: &str) -> Option<GameObjectHandle> { ... }

    /// Find the first GameObject with a tag (like Unity's GameObject.FindWithTag()).
    pub fn FindWithTag(&self, tag: &str) -> Option<GameObjectHandle> { ... }

    /// Find all GameObjects with a tag (like Unity's GameObject.FindGameObjectsWithTag()).
    pub fn FindGameObjectsWithTag(&self, tag: &str) -> Vec<GameObjectHandle> { ... }

    /// Find first object of type T (like Unity's FindObjectOfType<T>()).
    pub fn FindObjectOfType<T: Component + 'static>(&self) -> Option<GameObjectHandle> { ... }

    /// Find all objects of type T (like Unity's FindObjectsOfType<T>()).
    pub fn FindObjectsOfType<T: Component + 'static>(&self) -> Vec<GameObjectHandle> { ... }

    // === Component API on GameObject ===

    /// Add component to GameObject (like Unity's AddComponent<T>()).
    pub fn AddComponent<T: Component + 'static>(&mut self, handle: GameObjectHandle, component: T) { ... }

    /// Get component from GameObject (like Unity's GetComponent<T>()).
    pub fn GetComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }

    /// Get mutable component (like Unity's GetComponent<T>() with write access).
    pub fn GetComponentMut<T: Component + 'static>(&mut self, handle: GameObjectHandle) -> Option<&mut T> { ... }

    /// Try get component (like Unity's TryGetComponent<T>()).
    pub fn TryGetComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }

    /// Check if has component (like Unity's GetComponent<T>() != null).
    pub fn HasComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> bool { ... }

    /// Remove component (like Unity's Object.Destroy(component)).
    pub fn RemoveComponent<T: Component + 'static>(&mut self, handle: GameObjectHandle) -> bool { ... }

    /// Get component in children (like Unity's GetComponentInChildren<T>()).
    pub fn GetComponentInChildren<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }

    /// Get component in parents (like Unity's GetComponentInParent<T>()).
    pub fn GetComponentInParent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }

    /// Get all components of type on children (like Unity's GetComponentsInChildren<T>()).
    pub fn GetComponentsInChildren<T: Component + 'static>(&self, handle: GameObjectHandle) -> Vec<&T> { ... }

    // === Transform API (built-in, on every GameObject) ===

    /// Get transform (always returns Some, Transform is mandatory).
    pub fn GetTransform(&self, handle: GameObjectHandle) -> Option<&Transform> { ... }

    /// Get mutable transform.
    pub fn GetTransformMut(&mut self, handle: GameObjectHandle) -> Option<&mut Transform> { ... }

    // === Hierarchy API (built into Transform) ===

    /// Set parent (like Unity's Transform.SetParent()).
    pub fn SetParent(&mut self, child: GameObjectHandle, parent: Option<GameObjectHandle>) { ... }

    /// Get parent handle.
    pub fn GetParent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> { ... }

    /// Get children handles.
    pub fn GetChildren(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> { ... }

    /// Get child count.
    pub fn GetChildCount(&self, handle: GameObjectHandle) -> usize { ... }

    /// Get root GameObjects (like Unity's Scene.GetRootGameObjects()).
    pub fn GetRootGameObjects(&self) -> Vec<GameObjectHandle> { ... }

    // === Active State (Unity style) ===

    /// Set active state (like Unity's GameObject.SetActive()).
    pub fn SetActive(&mut self, handle: GameObjectHandle, active: bool) { ... }

    /// Check if active (like Unity's GameObject.activeSelf).
    pub fn IsActive(&self, handle: GameObjectHandle) -> bool { ... }

    /// Check if active in hierarchy (like Unity's GameObject.activeInHierarchy).
    pub fn IsActiveInHierarchy(&self, handle: GameObjectHandle) -> bool { ... }

    // === Tag/Layer (Unity style) ===

    /// Set tag (like Unity's GameObject.tag = "xxx").
    pub fn SetTag(&mut self, handle: GameObjectHandle, tag: &str) { ... }

    /// Get tag.
    pub fn GetTag(&self, handle: GameObjectHandle) -> &str { ... }

    /// Compare tag (like Unity's GameObject.CompareTag()).
    pub fn CompareTag(&self, handle: GameObjectHandle, tag: &str) -> bool { ... }

    /// Set layer.
    pub fn SetLayer(&mut self, handle: GameObjectHandle, layer: u32) { ... }

    /// Get layer.
    pub fn GetLayer(&self, handle: GameObjectHandle) -> u32 { ... }

    // === Name ===

    /// Set name.
    pub fn SetName(&mut self, handle: GameObjectHandle, name: &str) { ... }

    /// Get name.
    pub fn GetName(&self, handle: GameObjectHandle) -> &str { ... }

    // === SendMessage (Unity style) ===

    /// Send message to all components on this GameObject (like Unity's SendMessage).
    pub fn SendMessage(&mut self, handle: GameObjectHandle, method: &str) { ... }

    /// Send message to this and all children (like Unity's BroadcastMessage).
    pub fn BroadcastMessage(&mut self, handle: GameObjectHandle, method: &str) { ... }

    /// Send message to this and all parents (like Unity's SendMessageUpwards).
    pub fn SendMessageUpwards(&mut self, handle: GameObjectHandle, method: &str) { ... }

    // === Internal ===

    /// Process pending destroys (called at end of frame).
    pub(crate) fn flush_destroy(&mut self) { ... }

    /// Sync all transforms (called by update system).
    pub(crate) fn sync_transforms(&mut self) { ... }
}
```

### Key design decisions

1. **Public methods are PascalCase** (Unity style): `CreateGameObject`, `AddComponent`, `GetComponent`, `Find`, `Instantiate`, `Destroy`
2. **Private/internal methods are snake_case** (Rust style): `flush_destroy`, `sync_transforms`
3. **Transform is mandatory**: Every `CreateGameObject` automatically adds a Transform
4. **Destroy is deferred**: `Destroy()` adds to pending list, `flush_destroy()` processes at end of frame
5. **Components stored on GameObject**: `Vec<Box<dyn Component>>` inside each GameObject

---

## Task 2: Rewrite engine-core/src/gameobject.rs — GameObject

**Files:**
- Rewrite: `crates/engine-core/src/gameobject.rs`

### Current state
`GameObject` has components, parent/child, name, tag, layer, active. Parent/child is separate from Transform.

### Target architecture
- Transform is **built into** GameObject (not a separate component)
- Parent/child is **part of Transform** (not separate fields on GameObject)
- Component storage uses `Vec<Box<dyn Component>>`
- Public API matches Unity's GameObject

```rust
// crates/engine-core/src/gameobject.rs

use crate::transform::Transform;
use std::any::Any;

/// Base trait for all components (like Unity's Component).
pub trait Component: Any + Send + Sync {
    fn on_added(&mut self, _handle: GameObjectHandle) {}
    fn on_removed(&mut self, _handle: GameObjectHandle) {}
    fn on_enable(&mut self, _handle: GameObjectHandle) {}
    fn on_disable(&mut self, _handle: GameObjectHandle) {}
    fn on_destroy(&mut self, _handle: GameObjectHandle) {}
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn component_name(&self) -> &str { std::any::type_name::<Self>() }
}

/// Unity-like handle to a GameObject (generational ID).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameObjectHandle {
    index: u32,
    generation: u32,
}

/// The fundamental building block (like Unity's GameObject).
/// Always has a Transform. Parent/child is part of Transform.
pub(crate) struct GameObject {
    name: String,
    tag: String,
    layer: u32,
    active: bool,
    active_in_hierarchy: bool, // computed from parent chain
    components: Vec<Box<dyn Component>>,
    // Transform is stored separately in World's transform array
    // Parent/child is stored in World's hierarchy arrays
}
```

### Key changes from current
1. **Remove `parent` and `children` fields** from GameObject — these move to Transform
2. **Keep `components` as `Vec<Box<dyn Component>>`** — components stored ON the GameObject
3. **Add `active_in_hierarchy`** — computed from parent chain

---

## Task 3: Rewrite engine-core/src/transform.rs — Built-in Transform

**Files:**
- Rewrite: `crates/engine-core/src/transform.rs`

### Current state
Transform is a Component with local/world position, rotation, scale. Parent/child is separate.

### Target architecture
Transform is **not a Component** — it's a built-in part of every entity. Parent/child is part of Transform.

```rust
// crates/engine-core/src/transform.rs

use engine_math::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Built-in Transform component (like Unity's Transform).
/// Every GameObject has exactly one Transform. It cannot be removed.
/// Parent/child relationships are part of Transform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    // Local space
    pub(crate) local_position: Vec3,
    pub(crate) local_rotation: Quat,
    pub(crate) local_scale: Vec3,

    // World space (cached, computed by sync system)
    pub(crate) world_position: Vec3,
    pub(crate) world_rotation: Quat,
    pub(crate) world_scale: Vec3,

    // Hierarchy (built-in, not a separate component)
    pub(crate) parent: Option<GameObjectHandle>,
    pub(crate) children: Vec<GameObjectHandle>,
    pub(crate) root_order: i32, // sibling index
}

impl Transform {
    // === Local Space ===
    pub fn LocalPosition(&self) -> Vec3 { ... }
    pub fn SetLocalPosition(&mut self, pos: Vec3) { ... }
    pub fn LocalRotation(&self) -> Quat { ... }
    pub fn SetLocalRotation(&mut self, rot: Quat) { ... }
    pub fn LocalScale(&self) -> Vec3 { ... }
    pub fn SetLocalScale(&mut self, scale: Vec3) { ... }

    // === World Space ===
    pub fn Position(&self) -> Vec3 { ... }
    pub fn SetPosition(&mut self, pos: Vec3) { ... }
    pub fn Rotation(&self) -> Quat { ... }
    pub fn SetRotation(&mut self, rot: Quat) { ... }
    pub fn LossyScale(&self) -> Vec3 { ... }

    // === Direction Vectors ===
    pub fn Forward(&self) -> Vec3 { ... }
    pub fn Right(&self) -> Vec3 { ... }
    pub fn Up(&self) -> Vec3 { ... }

    // === Space Conversion ===
    pub fn TransformPoint(&self, point: Vec3) -> Vec3 { ... }
    pub fn InverseTransformPoint(&self, point: Vec3) -> Vec3 { ... }
    pub fn TransformDirection(&self, dir: Vec3) -> Vec3 { ... }
    pub fn InverseTransformDirection(&self, dir: Vec3) -> Vec3 { ... }

    // === LookAt / Rotate / Translate ===
    pub fn LookAt(&mut self, target: Vec3) { ... }
    pub fn RotateAround(&mut self, point: Vec3, axis: Vec3, angle: f32) { ... }
    pub fn Translate(&mut self, translation: Vec3, space: Space) { ... }

    // === Hierarchy (built-in) ===
    pub fn Parent(&self) -> Option<GameObjectHandle> { ... }
    pub fn SetParent(&mut self, parent: Option<GameObjectHandle>) { ... }
    pub fn ChildCount(&self) -> usize { ... }
    pub fn GetChild(&self, index: usize) -> Option<GameObjectHandle> { ... }
    pub fn Root(&self) -> GameObjectHandle { ... }
    pub fn IsChildOf(&self, parent: GameObjectHandle) -> bool { ... }
    pub fn SetAsFirstSibling(&mut self) { ... }
    pub fn SetAsLastSibling(&mut self) { ... }
    pub fn SetSiblingIndex(&mut self, index: usize) { ... }
    pub fn DetachChildren(&mut self) { ... }
    pub fn Find(&self, name: &str) -> Option<GameObjectHandle> { ... }

    // === Matrix ===
    pub fn LocalToWorldMatrix(&self) -> Mat4 { ... }
    pub fn WorldToLocalMatrix(&self) -> Mat4 { ... }

    // === Internal sync ===
    pub(crate) fn UpdateWorldTransform(&mut self, parent_pos: Vec3, parent_rot: Quat, parent_scale: Vec3) { ... }
    pub(crate) fn UpdateWorldTransformRoot(&mut self) { ... }
}
```

### Key differences from current
1. **Not a `Component`** — it's a built-in struct, not stored in `Vec<Box<dyn Component>>`
2. **Parent/child is part of Transform** — not separate `Parent`/`Children` components
3. **PascalCase public API** — `Position()`, `SetPosition()`, `Forward()`, etc.
4. **`GameObjectHandle`** stored in Transform — references back to the owning GameObject

---

## Task 4: Rewrite engine-core/src/monobehaviour.rs — MonoBehaviour Lifecycle

**Files:**
- Rewrite: `crates/engine-core/src/monobehaviour.rs`
- Rewrite: `crates/engine-core/src/monobehaviour_runner.rs`

### Current state
MonoBehaviour trait defined with lifecycle methods, but runner is placeholder.

### Target architecture
MonoBehaviour is a Component trait with full lifecycle. Runner dispatches callbacks.

```rust
// crates/engine-core/src/monobehaviour.rs

use crate::context::Context;
use crate::gameobject::{Component, GameObjectHandle};
use crate::events::*;

/// Base class for all user scripts (like Unity's MonoBehaviour).
pub trait MonoBehaviour: Component {
    // === Lifecycle (auto-dispatched by engine) ===
    fn Awake(&mut self, _context: &mut Context) {}
    fn OnEnable(&mut self, _context: &mut Context) {}
    fn OnDisable(&mut self, _context: &mut Context) {}
    fn Start(&mut self, _context: &mut Context) {}
    fn Update(&mut self, _context: &mut Context) {}
    fn FixedUpdate(&mut self, _context: &mut Context) {}
    fn LateUpdate(&mut self, _context: &mut Context) {}
    fn OnDestroy(&mut self, _context: &mut Context) {}

    // === Application callbacks ===
    fn OnApplicationQuit(&mut self, _context: &mut Context) {}
    fn OnApplicationPause(&mut self, _context: &mut Context, _paused: bool) {}
    fn OnApplicationFocus(&mut self, _context: &mut Context, _focused: bool) {}

    // === Physics callbacks (dispatched by physics system) ===
    fn OnCollisionEnter(&mut self, _context: &mut Context, _collision: &Collision) {}
    fn OnCollisionExit(&mut self, _context: &mut Context, _collision: &Collision) {}
    fn OnCollisionStay(&mut self, _context: &mut Context, _collision: &Collision) {}
    fn OnTriggerEnter(&mut self, _context: &mut Context, _other: GameObjectHandle) {}
    fn OnTriggerExit(&mut self, _context: &mut Context, _other: GameObjectHandle) {}
    fn OnTriggerStay(&mut self, _context: &mut Context, _other: GameObjectHandle) {}

    // === Input callbacks ===
    fn OnMouseDown(&mut self, _context: &mut Context) {}
    fn OnMouseUp(&mut self, _context: &mut Context) {}
    fn OnMouseEnter(&mut self, _context: &mut Context) {}
    fn OnMouseExit(&mut self, _context: &mut Context) {}
    fn OnMouseDrag(&mut self, _context: &mut Context) {}
    fn OnMouseOver(&mut self, _context: &mut Context) {}

    // === Rendering callbacks ===
    fn OnBecameVisible(&mut self, _context: &mut Context) {}
    fn OnBecameInvisible(&mut self, _context: &mut Context) {}

    // === Gizmo ===
    fn OnDrawGizmos(&self, _context: &Context) {}

    // === Coroutines (future) ===
    // fn StartCoroutine(&mut self, routine: Coroutine) -> CoroutineHandle { ... }
    // fn StopCoroutine(&mut self, handle: CoroutineHandle) { ... }
    // fn StopAllCoroutines(&mut self) { ... }

    // === Utility ===
    fn IsEnabled(&self) -> bool { true }
    fn SetEnabled(&mut self, _enabled: bool) {}

    // === SendMessage support ===
    fn HandleMessage(&mut self, _method: &str, _data: Option<&dyn std::any::Any>) {}
}
```

### MonoBehaviourRunner — lifecycle dispatch

```rust
// crates/engine-core/src/monobehaviour_runner.rs

use crate::context::Context;
use crate::gameobject::{GameObjectHandle, Component};
use crate::world::World;

/// Dispatches lifecycle callbacks on MonoBehaviours.
pub(crate) struct MonoBehaviourRunner;

impl MonoBehaviourRunner {
    /// Run Awake on a specific GameObject's MonoBehaviours.
    pub(crate) fn RunAwake(world: &mut World, handle: GameObjectHandle) {
        // Get all MonoBehaviour components on this GameObject
        // Call Awake on each
    }

    /// Run Start on all MonoBehaviours that haven't started yet.
    pub(crate) fn RunStart(world: &mut World, context: &mut Context) {
        // Iterate all GameObjects
        // For each, call Start on MonoBehaviours
    }

    /// Run Update on all enabled MonoBehaviours.
    pub(crate) fn RunUpdate(world: &mut World, context: &mut Context) {
        // Iterate all active GameObjects
        // For each, call Update on enabled MonoBehaviours
    }

    /// Run FixedUpdate on all enabled MonoBehaviours.
    pub(crate) fn RunFixedUpdate(world: &mut World, context: &mut Context) {
        // Same pattern as Update
    }

    /// Run LateUpdate on all enabled MonoBehaviours.
    pub(crate) fn RunLateUpdate(world: &mut World, context: &mut Context) {
        // Same pattern as Update
    }

    /// Run OnDestroy on a specific GameObject's MonoBehaviours.
    pub(crate) fn RunOnDestroy(world: &mut World, handle: GameObjectHandle) {
        // Get all MonoBehaviour components
        // Call OnDestroy on each
    }

    /// Run OnEnable on a specific GameObject's MonoBehaviours.
    pub(crate) fn RunOnEnable(world: &mut World, handle: GameObjectHandle) {
        // Get all MonoBehaviour components
        // Call OnEnable on each
    }

    /// Run OnDisable on a specific GameObject's MonoBehaviours.
    pub(crate) fn RunOnDisable(world: &mut World, handle: GameObjectHandle) {
        // Get all MonoBehaviour components
        // Call OnDisable on each
    }
}
```

---

## Task 5: Add Coroutine System

**Files:**
- Create: `crates/engine-core/src/coroutine.rs`
- Modify: `crates/engine-core/src/lib.rs`

### Implementation

```rust
// crates/engine-core/src/coroutine.rs

use std::collections::HashMap;

/// Yield instruction types (like Unity's yield return values).
pub enum YieldInstruction {
    /// Wait one frame (like yield return null)
    WaitOneFrame,
    /// Wait N frames
    WaitForFrames(u32),
    /// Wait for seconds (like yield return new WaitForSeconds(t))
    WaitForSeconds(f32),
    /// Wait for fixed update (like yield return new WaitForFixedUpdate())
    WaitForFixedUpdate,
    /// Wait for end of frame (like yield return new WaitForEndOfFrame())
    WaitForEndOfFrame,
    /// Wait until condition is true (like yield return new WaitUntil(|| condition))
    WaitUntil(Box<dyn Fn() -> bool + Send + Sync>),
    /// Wait while condition is true (like yield return new WaitWhile(|| condition))
    WaitWhile(Box<dyn Fn() -> bool + Send + Sync>),
    /// Chain another coroutine (like yield return StartCoroutine(other))
    StartCoroutine(CoroutineId),
}

/// Coroutine ID handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoroutineId(u64);

/// A running coroutine.
struct RunningCoroutine {
    id: CoroutineId,
    instructions: Vec<YieldInstruction>,
    current_index: usize,
    elapsed: f32,
    frames_waited: u32,
    owner: GameObjectHandle,
}

/// Manages all running coroutines.
pub(crate) struct CoroutineRunner {
    next_id: u64,
    active: HashMap<CoroutineId, RunningCoroutine>,
}
```

---

## Task 6: Add SendMessage / BroadcastMessage Dispatch

**Files:**
- Modify: `crates/engine-core/src/gameobject.rs`
- Modify: `crates/engine-core/src/world.rs`

### Implementation
Messages are dispatched by calling `HandleMessage` on all MonoBehaviour components:

```rust
impl World {
    pub fn SendMessage(&mut self, handle: GameObjectHandle, method: &str) {
        if let Some(go) = self.get_gameobject_mut(handle) {
            for component in go.components.iter_mut() {
                component.as_any_mut().downcast_mut::<dyn MonoBehaviour>()
                    .map(|m| m.HandleMessage(method, None));
            }
        }
    }

    pub fn BroadcastMessage(&mut self, handle: GameObjectHandle, method: &str) {
        // Send to self
        self.SendMessage(handle, method);
        // Send to all children recursively
        let children = self.get_children(handle);
        for child in children {
            self.BroadcastMessage(child, method);
        }
    }

    pub fn SendMessageUpwards(&mut self, handle: GameObjectHandle, method: &str) {
        // Send to self
        self.SendMessage(handle, method);
        // Send to parent recursively
        if let Some(parent) = self.get_parent(handle) {
            self.SendMessageUpwards(parent, method);
        }
    }
}
```

---

## Task 7: Integrate with Engine Entry Points

**Files:**
- Modify: `crates/engine-core/src/engine.rs`
- Modify: `crates/engine-core/src/app.rs`

### Changes
1. `Engine::new()` returns `AppBuilder` that uses the unified World
2. `run_default()` calls MonoBehaviourRunner lifecycle methods in the correct order:
   - FixedUpdate (0+ times per frame)
   - Update
   - LateUpdate
   - Flush destroy
   - Sync transforms

---

## Task 8: Update Serialization

**Files:**
- Modify: `crates/engine-core/src/serialization.rs`

### Changes
1. Serialize/deserialize Transform as built-in (not as a Component)
2. Serialize/deserialize parent/child hierarchy
3. Serialize/deserialize component data with type registry

---

## Task 9: Update Editor to Use Unified World

**Files:**
- Rewrite: `crates/engine-editor/src/state.rs`
- Modify: `crates/engine-editor/src/panels/`

### Changes
1. Remove separate `HashMap<u64, ...>` data model
2. Editor reads/writes directly from the unified World
3. Inspector accesses components via `GetComponent::<T>()`
4. Hierarchy panel reads from World's root GameObjects

---

## Task 10: Update Scripting Bridge

**Files:**
- Modify: `crates/engine-script/src/bridge.rs`
- Modify: `crates/engine-script/src/system.rs`

### Changes
1. Lua/WASM scripts use `GetComponent::<T>()` API
2. ScriptSystem operates on unified World
3. ComponentBridge maps Rust types to Lua/WASM

---

## Task 11: Update Engine-Scene

**Files:**
- Modify: `crates/engine-scene/src/scene_manager.rs`
- Modify: `crates/engine-scene/src/hierarchy.rs`

### Changes
1. SceneManager uses unified World
2. Remove duplicate Parent/Children components (now part of Transform)
3. Scene serialization uses unified World's format

---

## Task 12: Update All Examples and Tests

**Files:**
- Modify: `crates/engine-core/examples/*.rs`
- Modify: `crates/engine-core/src/**/*.rs` (tests)

### Changes
1. Update all examples to use new API (PascalCase)
2. Update all tests
3. Ensure all existing functionality still works

---

## Execution Order

| Phase | Tasks | Depends On |
|---|---|---|
| **Phase 1: Core** | Tasks 1-3 (World, GameObject, Transform) | — |
| **Phase 2: Lifecycle** | Tasks 4-5 (MonoBehaviour, Coroutines) | Phase 1 |
| **Phase 3: Messaging** | Task 6 (SendMessage) | Phase 1 |
| **Phase 4: Integration** | Tasks 7-8 (Engine, Serialization) | Phase 2 |
| **Phase 5: Editor** | Task 9 (Editor) | Phase 4 |
| **Phase 6: Scripts** | Task 10 (Scripting) | Phase 4 |
| **Phase 7: Scene** | Task 11 (Scene) | Phase 4 |
| **Phase 8: Cleanup** | Task 12 (Examples/Tests) | All |

## Self-Review

1. **Spec coverage:** Plan covers unified World, built-in Transform, MonoBehaviour lifecycle, SendMessage, coroutines, Editor integration, Scripting integration, Scene integration.
2. **Placeholder scan:** No TBD/TODO — each task has concrete architecture.
3. **Type consistency:** `GameObjectHandle` used consistently across all tasks. PascalCase for public API, snake_case for internal.
4. **Dependency chain:** Clear phase ordering with no circular dependencies.
