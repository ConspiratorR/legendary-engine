# RustEngine Unity API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite RustEngine to be a faithful Rust implementation of Unity's engine architecture, with exact API matches to Unity's documented classes, methods, and properties.

**Architecture:** Unity's architecture: GameObject is the fundamental unit, always has Transform. Components are stored on GameObjects. MonoBehaviour is the base script class. Everything uses PascalCase for public API. ECS is internal implementation detail.

**Tech Stack:** Rust, engine-ecs (internal sparse-set), engine-core (public API), engine-scene, engine-editor

---

## Phase 1: Core Foundation (engine-core rewrite)

### Task 1.1: Rewrite engine-core/src/gameobject.rs — Unity GameObject

**Files:** Rewrite: `crates/engine-core/src/gameobject.rs`

```rust
// crates/engine-core/src/gameobject.rs
// Exact Unity API match: UnityEngine.GameObject

use crate::transform::Transform;
use crate::component::Component;
use std::any::Any;

pub struct GameObject {
    pub(crate) name: String,
    pub(crate) tag: String,
    pub(crate) layer: i32,
    pub(crate) active_self: bool,
    pub(crate) components: Vec<Box<dyn Component>>,
}

impl GameObject {
    // Constructors (matches Unity's GameObject constructors)
    pub fn new() -> Self { ... }
    pub fn new_with_name(name: &str) -> Self { ... }
    pub fn new_with_components(name: &str, components: Vec<Box<dyn Component>>) -> Self { ... }

    // Properties (matches Unity's GameObject properties)
    pub fn ActiveInHierarchy(&self) -> bool { ... }
    pub fn ActiveSelf(&self) -> bool { ... }
    pub fn IsStatic(&self) -> bool { ... }
    pub fn SetStatic(&mut self, value: bool) { ... }
    pub fn Layer(&self) -> i32 { ... }
    pub fn SetLayer(&mut self, layer: i32) { ... }
    pub fn Tag(&self) -> &str { ... }
    pub fn SetTag(&mut self, tag: &str) { ... }

    // Instance ID (matches Unity's Object.GetInstanceID)
    pub fn GetInstanceID(&self) -> i32 { ... }

    // Component access (matches Unity's Component methods)
    pub fn AddComponent<T: Component + 'static>(&mut self) -> &mut T { ... }
    pub fn GetComponent<T: Component + 'static>(&self) -> Option<&T> { ... }
    pub fn GetComponentMut<T: Component + 'static>(&mut self) -> Option<&mut T> { ... }
    pub fn GetComponentInChildren<T: Component + 'static>(&self) -> Option<&T> { ... }
    pub fn GetComponentInParent<T: Component + 'static>(&self) -> Option<&T> { ... }
    pub fn GetComponents<T: Component + 'static>(&self) -> Vec<&T> { ... }
    pub fn GetComponentsInChildren<T: Component + 'static>(&self) -> Vec<&T> { ... }
    pub fn GetComponentsInParent<T: Component + 'static>(&self) -> Vec<&T> { ... }
    pub fn TryGetComponent<T: Component + 'static>(&self) -> Option<&T> { ... }
    pub fn HasComponent<T: Component + 'static>(&self) -> bool { ... }

    // Messaging (matches Unity's SendMessage/BroadcastMessage)
    pub fn SendMessage(&mut self, method_name: &str) { ... }
    pub fn SendMessageWithValue(&mut self, method_name: &str, value: &dyn Any) { ... }
    pub fn SendMessageUpwards(&mut self, method_name: &str) { ... }
    pub fn BroadcastMessage(&mut self, method_name: &str) { ... }

    // Tag comparison (matches Unity's CompareTag)
    pub fn CompareTag(&self, tag: &str) -> bool { ... }

    // Active state (matches Unity's SetActive)
    pub fn SetActive(&mut self, value: bool) { ... }

    // Name (inherited from Object)
    pub fn Name(&self) -> &str { ... }
    pub fn SetName(&mut self, name: &str) { ... }
}
```

### Task 1.2: Rewrite engine-core/src/component.rs — Unity Component

**Files:** Rewrite: `crates/engine-core/src/component.rs`

```rust
// crates/engine-core/src/component.rs
// Exact Unity API match: UnityEngine.Component

use std::any::Any;
use crate::gameobject::GameObjectHandle;

pub trait Component: Any + Send + Sync {
    // Properties (matches Unity's Component properties)
    fn GameObject(&self) -> Option<GameObjectHandle> { None }
    fn Tag(&self) -> &str { "Untagged" }
    fn Transform(&self) -> Option<GameObjectHandle> { None }

    // Methods (matches Unity's Component methods)
    fn CompareTag(&self, tag: &str) -> bool { false }
    fn GetComponent<T: Component + 'static>(&self) -> Option<&T> { None }
    fn GetComponentInChildren<T: Component + 'static>(&self) -> Option<&T> { None }
    fn GetComponentInParent<T: Component + 'static>(&self) -> Option<&T> { None }
    fn GetComponents<T: Component + 'static>(&self) -> Vec<&T> { Vec::new() }
    fn GetComponentsInChildren<T: Component + 'static>(&self) -> Vec<&T> { Vec::new() }
    fn GetComponentsInParent<T: Component + 'static>(&self) -> Vec<&T> { Vec::new() }

    // Messaging
    fn SendMessage(&mut self, method_name: &str) {}
    fn SendMessageWithValue(&mut self, method_name: &str, value: &dyn Any) {}
    fn SendMessageUpwards(&mut self, method_name: &str) {}
    fn BroadcastMessage(&mut self, method_name: &str) {}

    // Lifecycle (internal, called by engine)
    fn on_added(&mut self, _handle: GameObjectHandle) {}
    fn on_removed(&mut self, _handle: GameObjectHandle) {}
    fn on_enable(&mut self, _handle: GameObjectHandle) {}
    fn on_disable(&mut self, _handle: GameObjectHandle) {}
    fn on_destroy(&mut self, _handle: GameObjectHandle) {}

    // Downcast support
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

### Task 1.3: Rewrite engine-core/src/behaviour.cs — Unity Behaviour

**Files:** Create: `crates/engine-core/src/behaviour.rs`

```rust
// crates/engine-core/src/behaviour.rs
// Exact Unity API match: UnityEngine.Behaviour

use crate::component::Component;

pub trait Behaviour: Component {
    // Properties (matches Unity's Behaviour properties)
    fn Enabled(&self) -> bool { true }
    fn SetEnabled(&mut self, enabled: bool) {}
    fn IsActiveAndEnabled(&self) -> bool { false }
}
```

### Task 1.4: Rewrite engine-core/src/transform.rs — Unity Transform

**Files:** Rewrite: `crates/engine-core/src/transform.rs`

```rust
// crates/engine-core/src/transform.rs
// Exact Unity API match: UnityEngine.Transform
// Transform is BUILT-IN, not a Component. Every GameObject has one.

use engine_math::{Mat4, Quat, Vec3};
use crate::gameobject::GameObjectHandle;
use crate::space::Space;

pub struct Transform {
    // Local space
    pub(crate) local_position: Vec3,
    pub(crate) local_rotation: Quat,
    pub(crate) local_scale: Vec3,

    // World space (cached)
    pub(crate) world_position: Vec3,
    pub(crate) world_rotation: Quat,
    pub(crate) world_scale: Vec3,

    // Hierarchy (built-in, not separate component)
    pub(crate) parent: Option<GameObjectHandle>,
    pub(crate) children: Vec<GameObjectHandle>,
    pub(crate) has_changed: bool,
}

impl Transform {
    // Properties (matches Unity's Transform properties EXACTLY)
    pub fn ChildCount(&self) -> usize { ... }
    pub fn EulerAngles(&self) -> Vec3 { ... }
    pub fn SetEulerAngles(&mut self, eulers: Vec3) { ... }
    pub fn Forward(&self) -> Vec3 { ... }
    pub fn SetForward(&mut self, forward: Vec3) { ... }
    pub fn HasChanged(&self) -> bool { ... }
    pub fn SetHasChanged(&mut self, changed: bool) { ... }
    pub fn LocalEulerAngles(&self) -> Vec3 { ... }
    pub fn SetLocalEulerAngles(&mut self, eulers: Vec3) { ... }
    pub fn LocalPosition(&self) -> Vec3 { ... }
    pub fn SetLocalPosition(&mut self, pos: Vec3) { ... }
    pub fn LocalRotation(&self) -> Quat { ... }
    pub fn SetLocalRotation(&mut self, rot: Quat) { ... }
    pub fn LocalScale(&self) -> Vec3 { ... }
    pub fn SetLocalScale(&mut self, scale: Vec3) { ... }
    pub fn LocalToWorldMatrix(&self) -> Mat4 { ... }
    pub fn LossyScale(&self) -> Vec3 { ... }
    pub fn Parent(&self) -> Option<GameObjectHandle> { ... }
    pub fn SetParent(&mut self, parent: Option<GameObjectHandle>) { ... }
    pub fn Position(&self) -> Vec3 { ... }
    pub fn SetPosition(&mut self, pos: Vec3) { ... }
    pub fn Right(&self) -> Vec3 { ... }
    pub fn SetRight(&mut self, right: Vec3) { ... }
    pub fn Root(&self) -> GameObjectHandle { ... }
    pub fn Rotation(&self) -> Quat { ... }
    pub fn SetRotation(&mut self, rot: Quat) { ... }
    pub fn Up(&self) -> Vec3 { ... }
    pub fn SetUp(&mut self, up: Vec3) { ... }
    pub fn WorldToLocalMatrix(&self) -> Mat4 { ... }

    // Methods (matches Unity's Transform methods EXACTLY)
    pub fn DetachChildren(&mut self) { ... }
    pub fn Find(&self, name: &str) -> Option<GameObjectHandle> { ... }
    pub fn GetChild(&self, index: usize) -> Option<GameObjectHandle> { ... }
    pub fn GetLocalPositionAndRotation(&self) -> (Vec3, Quat) { ... }
    pub fn GetPositionAndRotation(&self) -> (Vec3, Quat) { ... }
    pub fn GetSiblingIndex(&self) -> usize { ... }
    pub fn InverseTransformDirection(&self, direction: Vec3) -> Vec3 { ... }
    pub fn InverseTransformPoint(&self, position: Vec3) -> Vec3 { ... }
    pub fn InverseTransformVector(&self, vector: Vec3) -> Vec3 { ... }
    pub fn IsChildOf(&self, parent: GameObjectHandle) -> bool { ... }
    pub fn LookAt(&mut self, target: Vec3) { ... }
    pub fn LookAtWithUp(&mut self, target: Vec3, world_up: Vec3) { ... }
    pub fn Rotate(&mut self, eulers: Vec3) { ... }
    pub fn RotateWithSpace(&mut self, eulers: Vec3, relative_to: Space) { ... }
    pub fn RotateAround(&mut self, point: Vec3, axis: Vec3, angle: f32) { ... }
    pub fn SetAsFirstSibling(&mut self) { ... }
    pub fn SetAsLastSibling(&mut self) { ... }
    pub fn SetLocalPositionAndRotation(&mut self, pos: Vec3, rot: Quat) { ... }
    pub fn SetPositionAndRotation(&mut self, pos: Vec3, rot: Quat) { ... }
    pub fn SetSiblingIndex(&mut self, index: usize) { ... }
    pub fn TransformDirection(&self, direction: Vec3) -> Vec3 { ... }
    pub fn TransformPoint(&self, position: Vec3) -> Vec3 { ... }
    pub fn TransformVector(&self, vector: Vec3) -> Vec3 { ... }
    pub fn Translate(&mut self, translation: Vec3) { ... }
    pub fn TranslateWithSpace(&mut self, translation: Vec3, relative_to: Space) { ... }
    pub fn TranslateRelative(&mut self, translation: Vec3, relative_to: Transform) { ... }
}
```

### Task 1.5: Rewrite engine-core/src/monobehaviour.rs — Unity MonoBehaviour

**Files:** Rewrite: `crates/engine-core/src/monobehaviour.rs`

```rust
// crates/engine-core/src/monobehaviour.rs
// Exact Unity API match: UnityEngine.MonoBehaviour

use crate::behaviour::Behaviour;
use crate::context::Context;
use crate::events::*;

pub trait MonoBehaviour: Behaviour {
    // Properties (matches Unity's MonoBehaviour properties)
    // destroyCancellationToken — future
    // runInEditMode — future
    // useGUILayout — future

    // Lifecycle callbacks (matches Unity's MonoBehaviour EXACTLY)
    fn Awake(&mut self, _context: &mut Context) {}
    fn OnEnable(&mut self, _context: &mut Context) {}
    fn OnDisable(&mut self, _context: &mut Context) {}
    fn Start(&mut self, _context: &mut Context) {}
    fn Update(&mut self, _context: &mut Context) {}
    fn FixedUpdate(&mut self, _context: &mut Context) {}
    fn LateUpdate(&mut self, _context: &mut Context) {}
    fn OnDestroy(&mut self, _context: &mut Context) {}

    // Application callbacks (matches Unity's MonoBehaviour)
    fn OnApplicationQuit(&mut self, _context: &mut Context) {}
    fn OnApplicationPause(&mut self, _context: &mut Context, _paused: bool) {}
    fn OnApplicationFocus(&mut self, _context: &mut Context, _focused: bool) {}

    // Physics callbacks (matches Unity's MonoBehaviour EXACTLY)
    fn OnCollisionEnter(&mut self, _context: &mut Context, _collision: &Collision) {}
    fn OnCollisionExit(&mut self, _context: &mut Context, _collision: &Collision) {}
    fn OnCollisionStay(&mut self, _context: &mut Context, _collision: &Collision) {}
    fn OnTriggerEnter(&mut self, _context: &mut Context, _other: Collider) {}
    fn OnTriggerExit(&mut self, _context: &mut Context, _other: Collider) {}
    fn OnTriggerStay(&mut self, _context: &mut Context, _other: Collider) {}

    // Input callbacks (matches Unity's MonoBehaviour EXACTLY)
    fn OnMouseDown(&mut self, _context: &mut Context) {}
    fn OnMouseUp(&mut self, _context: &mut Context) {}
    fn OnMouseEnter(&mut self, _context: &mut Context) {}
    fn OnMouseExit(&mut self, _context: &mut Context) {}
    fn OnMouseDrag(&mut self, _context: &mut Context) {}
    fn OnMouseOver(&mut self, _context: &mut Context) {}
    fn OnMouseUpAsButton(&mut self, _context: &mut Context) {}

    // Rendering callbacks
    fn OnBecameVisible(&mut self, _context: &mut Context) {}
    fn OnBecameInvisible(&mut self, _context: &mut Context) {}
    fn OnWillRenderObject(&mut self, _context: &mut Context) {}
    fn OnPreCull(&mut self, _context: &mut Context) {}
    fn OnPreRender(&mut self, _context: &mut Context) {}
    fn OnPostRender(&mut self, _context: &mut Context) {}
    fn OnRenderObject(&mut self, _context: &mut Context) {}
    fn OnDrawGizmos(&self, _context: &Context) {}
    fn OnDrawGizmosSelected(&self, _context: &Context) {}

    // Animation callbacks
    fn OnAnimatorMove(&mut self, _context: &mut Context) {}
    fn OnAnimatorIK(&mut self, _context: &mut Context, _layer_index: i32) {}

    // Joint callbacks
    fn OnJointBreak(&mut self, _context: &mut Context, _break_force: f32) {}

    // Coroutine methods (matches Unity's MonoBehaviour EXACTLY)
    fn StartCoroutine(&mut self, _routine: &str) -> CoroutineHandle { ... }
    fn StopCoroutine(&mut self, _routine: &str) { ... }
    fn StopAllCoroutines(&mut self) { ... }

    // Invoke methods (matches Unity's MonoBehaviour EXACTLY)
    fn Invoke(&mut self, _method_name: &str, _time: f32) { ... }
    fn InvokeRepeating(&mut self, _method_name: &str, _time: f32, _repeat_rate: f32) { ... }
    fn CancelInvoke(&mut self) { ... }
    fn CancelInvokeMethod(&mut self, _method_name: &str) { ... }
    fn IsInvoking(&self) -> bool { false }
    fn IsInvokingMethod(&self, _method_name: &str) -> bool { false }

    // Static methods
    fn print(message: &str) { println!("{}", message); }
}
```

### Task 1.6: Rewrite engine-core/src/scriptable_object.rs — Unity ScriptableObject

**Files:** Rewrite: `crates/engine-core/src/scriptable_object.rs`

```rust
// crates/engine-core/src/scriptable_object.rs
// Exact Unity API match: UnityEngine.ScriptableObject

use crate::object::Object;

pub trait ScriptableObject: Object {
    // Static methods (matches Unity's ScriptableObject)
    fn CreateInstance<T: ScriptableObject + Default>() -> T { T::default() }

    // Lifecycle (matches Unity's ScriptableObject)
    fn Awake(&mut self) {}
    fn OnEnable(&mut self) {}
    fn OnDisable(&mut self) {}
    fn OnDestroy(&mut self) {}
    fn OnValidate(&mut self) {}
    fn Reset(&mut self) {}
}
```

### Task 1.7: Rewrite engine-core/src/object.rs — Unity Object

**Files:** Create: `crates/engine-core/src/object.rs`

```rust
// crates/engine-core/src/object.rs
// Exact Unity API match: UnityEngine.Object

pub trait Object: Send + Sync {
    // Properties (matches Unity's Object)
    fn Name(&self) -> &str { "" }
    fn SetName(&mut self, name: &str) {}

    // Instance ID
    fn GetInstanceID(&self) -> i32;

    // ToString
    fn ToString(&self) -> String { self.Name().to_string() }
}

// Static methods (matches Unity's Object static methods)
pub struct ObjectStatic;

impl ObjectStatic {
    pub fn Destroy(obj: &mut dyn Object) { ... }
    pub fn DestroyDelayed(obj: &mut dyn Object, t: f32) { ... }
    pub fn DestroyImmediate(obj: &mut dyn Object) { ... }
    pub fn DontDestroyOnLoad(obj: &mut dyn Object) { ... }
    pub fn FindObjectOfType<T: Object>() -> Option<T> { ... }
    pub fn FindObjectsOfType<T: Object>() -> Vec<T> { ... }
    pub fn Instantiate<T: Object>(original: &T) -> T { ... }
}
```

---

## Phase 2: World Rewrite (engine-core/src/world.rs)

### Task 2.1: Unified World

**Files:** Rewrite: `crates/engine-core/src/world.rs`

```rust
// crates/engine-core/src/world.rs
// Unified World: sparse-set ECS internally, Unity-style API externally

pub struct World {
    // Internal ECS (hidden)
    ecs: engine_ecs::World,
    // GameObject storage
    gameobjects: Vec<Option<GameObjectHandle>>,
    generations: Vec<u32>,
    free_list: Vec<u32>,
    // Transform storage (built-in, not on GameObject)
    transforms: Vec<Option<Transform>>,
    // Hierarchy
    name_to_handles: HashMap<String, Vec<GameObjectHandle>>,
    tag_to_handles: HashMap<String, Vec<GameObjectHandle>>,
    // Pending destroys
    pending_destroy: Vec<GameObjectHandle>,
}

impl World {
    // === Object static methods (matches Unity's Object) ===
    pub fn CreateGameObject(&mut self, name: &str) -> GameObjectHandle { ... }
    pub fn CreateGameObjectWithComponents(&mut self, name: &str, components: Vec<Box<dyn Component>>) -> GameObjectHandle { ... }
    pub fn Destroy(&mut self, handle: GameObjectHandle) { ... }
    pub fn DestroyDelayed(&mut self, handle: GameObjectHandle, t: f32) { ... }
    pub fn DestroyImmediate(&mut self, handle: GameObjectHandle) { ... }
    pub fn DontDestroyOnLoad(&mut self, handle: GameObjectHandle) { ... }
    pub fn Instantiate(&mut self, template: GameObjectHandle) -> GameObjectHandle { ... }
    pub fn InstantiateAtPosition(&mut self, template: GameObjectHandle, pos: Vec3, rot: Quat) -> GameObjectHandle { ... }

    // === Find methods (matches Unity's Object/GameObject) ===
    pub fn Find(&self, name: &str) -> Option<GameObjectHandle> { ... }
    pub fn FindWithTag(&self, tag: &str) -> Option<GameObjectHandle> { ... }
    pub fn FindGameObjectsWithTag(&self, tag: &str) -> Vec<GameObjectHandle> { ... }
    pub fn FindObjectOfType<T: Component + 'static>(&self) -> Option<GameObjectHandle> { ... }
    pub fn FindObjectsOfType<T: Component + 'static>(&self) -> Vec<GameObjectHandle> { ... }

    // === Component access (matches Unity's Component methods) ===
    pub fn AddComponent<T: Component + 'static>(&mut self, handle: GameObjectHandle) -> &mut T { ... }
    pub fn GetComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }
    pub fn GetComponentMut<T: Component + 'static>(&mut self, handle: GameObjectHandle) -> Option<&mut T> { ... }
    pub fn GetComponentInChildren<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }
    pub fn GetComponentInParent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }
    pub fn GetComponents<T: Component + 'static>(&self, handle: GameObjectHandle) -> Vec<&T> { ... }
    pub fn GetComponentsInChildren<T: Component + 'static>(&self, handle: GameObjectHandle) -> Vec<&T> { ... }
    pub fn GetComponentsInParent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Vec<&T> { ... }
    pub fn TryGetComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> Option<&T> { ... }
    pub fn HasComponent<T: Component + 'static>(&self, handle: GameObjectHandle) -> bool { ... }

    // === Transform access (built-in) ===
    pub fn GetTransform(&self, handle: GameObjectHandle) -> Option<&Transform> { ... }
    pub fn GetTransformMut(&mut self, handle: GameObjectHandle) -> Option<&mut Transform> { ... }

    // === Hierarchy (built into Transform) ===
    pub fn SetParent(&mut self, child: GameObjectHandle, parent: Option<GameObjectHandle>) { ... }
    pub fn GetParent(&self, handle: GameObjectHandle) -> Option<GameObjectHandle> { ... }
    pub fn GetChildren(&self, handle: GameObjectHandle) -> Vec<GameObjectHandle> { ... }
    pub fn GetChildCount(&self, handle: GameObjectHandle) -> usize { ... }
    pub fn GetRootGameObjects(&self) -> Vec<GameObjectHandle> { ... }

    // === Active state ===
    pub fn SetActive(&mut self, handle: GameObjectHandle, active: bool) { ... }
    pub fn IsActive(&self, handle: GameObjectHandle) -> bool { ... }
    pub fn IsActiveInHierarchy(&self, handle: GameObjectHandle) -> bool { ... }

    // === Name/Tag/Layer ===
    pub fn SetName(&mut self, handle: GameObjectHandle, name: &str) { ... }
    pub fn GetName(&self, handle: GameObjectHandle) -> &str { ... }
    pub fn SetTag(&mut self, handle: GameObjectHandle, tag: &str) { ... }
    pub fn GetTag(&self, handle: GameObjectHandle) -> &str { ... }
    pub fn CompareTag(&self, handle: GameObjectHandle, tag: &str) -> bool { ... }
    pub fn SetLayer(&mut self, handle: GameObjectHandle, layer: i32) { ... }
    pub fn GetLayer(&self, handle: GameObjectHandle) -> i32 { ... }

    // === Messaging ===
    pub fn SendMessage(&mut self, handle: GameObjectHandle, method: &str) { ... }
    pub fn SendMessageWithValue(&mut self, handle: GameObjectHandle, method: &str, value: &dyn Any) { ... }
    pub fn SendMessageUpwards(&mut self, handle: GameObjectHandle, method: &str) { ... }
    pub fn BroadcastMessage(&mut self, handle: GameObjectHandle, method: &str) { ... }

    // === Lifecycle dispatch (internal) ===
    pub(crate) fn run_lifecycle(&mut self, context: &mut Context) { ... }
    pub(crate) fn sync_transforms(&mut self) { ... }
    pub(crate) fn flush_destroy(&mut self) { ... }
}
```

---

## Phase 3: Engine Entry Points

### Task 3.1: Rewrite engine-core/src/engine.rs

**Files:** Rewrite: `crates/engine-core/src/engine.rs`

Unity-like entry point with correct lifecycle order:
1. FixedUpdate (0+ times per frame)
2. Update
3. LateUpdate
4. Sync transforms
5. Flush destroy
6. Render

### Task 3.2: Rewrite engine-core/src/app.rs

**Files:** Rewrite: `crates/engine-core/src/app.rs`

AppBuilder uses unified World, registers MonoBehaviourRunner as a system.

---

## Phase 4: Physics Components

### Task 4.1: Unity Rigidbody

**Files:** Create: `crates/engine-physics/src/rigidbody.rs`

```rust
// Matches Unity's Rigidbody EXACTLY
pub struct Rigidbody {
    pub mass: f32,
    pub drag: f32,
    pub angular_drag: f32,
    pub use_gravity: bool,
    pub is_kinematic: bool,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub constraints: RigidbodyConstraints,
    pub collision_detection: CollisionDetectionMode,
    pub interpolation: RigidbodyInterpolation,
    // ... all Unity properties
}

impl Rigidbody {
    // Methods (matches Unity's Rigidbody EXACTLY)
    pub fn AddForce(&mut self, force: Vec3) { ... }
    pub fn AddForceWithMode(&mut self, force: Vec3, mode: ForceMode) { ... }
    pub fn AddTorque(&mut self, torque: Vec3) { ... }
    pub fn AddRelativeForce(&mut self, force: Vec3) { ... }
    pub fn AddRelativeTorque(&mut self, torque: Vec3) { ... }
    pub fn AddForceAtPosition(&mut self, force: Vec3, position: Vec3) { ... }
    pub fn AddExplosionForce(&mut self, force: f32, position: Vec3, radius: f32) { ... }
    pub fn MovePosition(&mut self, position: Vec3) { ... }
    pub fn MoveRotation(&mut self, rotation: Quat) { ... }
    pub fn Sleep(&mut self) { ... }
    pub fn WakeUp(&mut self) { ... }
    pub fn IsSleeping(&self) -> bool { ... }
    pub fn GetPointVelocity(&self, world_point: Vec3) -> Vec3 { ... }
    pub fn GetRelativePointVelocity(&self, relative_point: Vec3) -> Vec3 { ... }
    pub fn SweepTest(&self, direction: Vec3, max_distance: f32) -> Option<RaycastHit> { ... }
    pub fn SweepTestAll(&self, direction: Vec3, max_distance: f32) -> Vec<RaycastHit> { ... }
}
```

### Task 4.2: Unity Colliders

**Files:** Create: `crates/engine-physics/src/colliders.rs`

```rust
// Matches Unity's BoxCollider EXACTLY
pub struct BoxCollider {
    pub center: Vec3,
    pub size: Vec3,
    pub is_trigger: bool,
    pub shared_material: Option<PhysicMaterial>,
    pub contact_offset: f32,
}

// Matches Unity's SphereCollider EXACTLY
pub struct SphereCollider {
    pub center: Vec3,
    pub radius: f32,
    pub is_trigger: bool,
    pub shared_material: Option<PhysicMaterial>,
    pub contact_offset: f32,
}

// Matches Unity's CapsuleCollider EXACTLY
pub struct CapsuleCollider {
    pub center: Vec3,
    pub radius: f32,
    pub height: f32,
    pub direction: i32, // 0=X, 1=Y, 2=Z
    pub is_trigger: bool,
    pub shared_material: Option<PhysicMaterial>,
    pub contact_offset: f32,
}

// Base Collider trait (matches Unity's Collider)
pub trait Collider: Component {
    fn AttachedRigidbody(&self) -> Option<GameObjectHandle> { None }
    fn Bounds(&self) -> Bounds { ... }
    fn ContactOffset(&self) -> f32 { 0.01 }
    fn IsTrigger(&self) -> bool { false }
    fn SetIsTrigger(&mut self, value: bool) {}
    fn ClosestPoint(&self, position: Vec3) -> Vec3 { position }
    fn ClosestPointOnBounds(&self, position: Vec3) -> Vec3 { position }
    fn Raycast(&self, ray: &Ray, max_distance: f32) -> Option<RaycastHit> { None }
}
```

### Task 4.3: Unity Physics Static Class

**Files:** Create: `crates/engine-physics/src/physics.rs`

```rust
// Matches Unity's Physics static class EXACTLY
pub struct Physics;

impl Physics {
    // Properties
    pub fn Gravity() -> Vec3 { Vec3::new(0.0, -9.81, 0.0) }
    pub fn SetGravity(gravity: Vec3) { ... }

    // Raycast (matches Unity's Physics.Raycast EXACTLY)
    pub fn Raycast(origin: Vec3, direction: Vec3) -> bool { ... }
    pub fn RaycastWithMaxDistance(origin: Vec3, direction: Vec3, max_distance: f32) -> bool { ... }
    pub fn RaycastWithHit(origin: Vec3, direction: Vec3, hit: &mut RaycastHit) -> bool { ... }
    pub fn RaycastWithHitMaxDistance(origin: Vec3, direction: Vec3, hit: &mut RaycastHit, max_distance: f32) -> bool { ... }
    pub fn RaycastAll(origin: Vec3, direction: Vec3) -> Vec<RaycastHit> { ... }
    pub fn RaycastNonAlloc(origin: Vec3, direction: Vec3, results: &mut [RaycastHit]) -> usize { ... }

    // Overlap (matches Unity's Physics Overlap methods)
    pub fn OverlapSphere(position: Vec3, radius: f32) -> Vec<GameObjectHandle> { ... }
    pub fn OverlapBox(center: Vec3, half_extents: Vec3) -> Vec<GameObjectHandle> { ... }
    pub fn OverlapCapsule(point0: Vec3, point1: Vec3, radius: f32) -> Vec<GameObjectHandle> { ... }
    pub fn CheckSphere(position: Vec3, radius: f32) -> bool { ... }
    pub fn CheckBox(center: Vec3, half_extents: Vec3) -> bool { ... }
    pub fn CheckCapsule(start: Vec3, end: Vec3, radius: f32) -> bool { ... }

    // Sweep (matches Unity's Physics Sweep methods)
    pub fn BoxCast(center: Vec3, half_extents: Vec3, direction: Vec3) -> bool { ... }
    pub fn SphereCast(origin: Vec3, radius: f32, direction: Vec3) -> bool { ... }
    pub fn CapsuleCast(start: Vec3, end: Vec3, radius: f32, direction: Vec3) -> bool { ... }

    // Simulation
    pub fn Simulate(step: f32) { ... }
    pub fn SyncTransforms() { ... }
}
```

---

## Phase 5: Rendering Components

### Task 5.1: Unity Camera

**Files:** Create: `crates/engine-render/src/camera.rs`

```rust
// Matches Unity's Camera EXACTLY
pub struct Camera {
    pub clear_flags: CameraClearFlags,
    pub background_color: Color,
    pub culling_mask: i32,
    pub depth: f32,
    pub field_of_view: f32,
    pub near_clip_plane: f32,
    pub far_clip_plane: f32,
    pub orthographic: bool,
    pub orthographic_size: f32,
    pub aspect: f32,
    pub rect: Rect,
    pub projection_matrix: Mat4,
    // ... all Unity Camera properties
}

impl Camera {
    // Static
    pub fn Main() -> Option<GameObjectHandle> { ... }
    pub fn Current() -> Option<GameObjectHandle> { ... }
    pub fn AllCameras() -> Vec<GameObjectHandle> { ... }

    // Methods
    pub fn WorldToScreenPoint(&self, position: Vec3) -> Vec3 { ... }
    pub fn ScreenToWorldPoint(&self, position: Vec3) -> Vec3 { ... }
    pub fn ViewportToWorldPoint(&self, position: Vec3) -> Vec3 { ... }
    pub fn WorldToViewportPoint(&self, position: Vec3) -> Vec3 { ... }
    pub fn ScreenPointToRay(&self, position: Vec3) -> Ray { ... }
    pub fn ViewportPointToRay(&self, position: Vec3) -> Ray { ... }
}
```

### Task 5.2: Unity Light

**Files:** Create: `crates/engine-render/src/light.rs`

```rust
// Matches Unity's Light EXACTLY
pub struct Light {
    pub light_type: LightType,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub spot_angle: f32,
    pub inner_spot_angle: f32,
    pub shadows: LightShadows,
    pub shadow_strength: f32,
    pub shadow_bias: f32,
    pub shadow_normal_bias: f32,
    pub shadow_near_plane: f32,
    // ... all Unity Light properties
}
```

---

## Phase 6: Scene Management

### Task 6.1: Unity Scene struct

**Files:** Create: `crates/engine-scene/src/scene.rs`

```rust
// Matches Unity's SceneManagement.Scene EXACTLY
pub struct Scene {
    pub build_index: i32,
    pub is_dirty: bool,
    pub is_loaded: bool,
    pub name: String,
    pub path: String,
    pub root_count: usize,
}

impl Scene {
    pub fn GetRootGameObjects(&self) -> Vec<GameObjectHandle> { ... }
    pub fn IsValid(&self) -> bool { ... }
}
```

---

## Phase 7: Editor Rewrite

### Task 7.1: Editor uses unified World

**Files:** Rewrite: `crates/engine-editor/src/state.rs`

Editor reads/writes directly from the unified World. Remove separate HashMap data model. Inspector uses GetComponent::<T>(). Hierarchy reads from World.GetRootGameObjects().

---

## Phase 8: Scripting Integration

### Task 8.1: Script bridge uses Unity API

**Files:** Modify: `crates/engine-script/src/bridge.rs`

Lua/WASM scripts call GetComponent::<T>(), AddComponent::<T>(), SendMessage(), etc.

---

## Execution Order

| Phase | Tasks | Description |
|---|---|---|
| **Phase 1** | 1.1-1.7 | Core traits: Object, Component, Behaviour, MonoBehaviour, Transform, ScriptableObject, GameObject |
| **Phase 2** | 2.1 | Unified World with all Unity API methods |
| **Phase 3** | 3.1-3.2 | Engine entry points with correct lifecycle |
| **Phase 4** | 4.1-4.3 | Physics: Rigidbody, Colliders, Physics static |
| **Phase 5** | 5.1-5.2 | Rendering: Camera, Light |
| **Phase 6** | 6.1 | Scene management |
| **Phase 7** | 7.1 | Editor rewrite |
| **Phase 8** | 8.1 | Scripting integration |

## Key Design Principles

1. **Every public API method is PascalCase** — `GetComponent`, `AddComponent`, `FindObjectOfType`
2. **Every public property is PascalCase** — `Position`, `Rotation`, `LocalScale`
3. **Transform is built-in** — not a Component, always present on every GameObject
4. **Components stored on GameObject** — `Vec<Box<dyn Component>>`
5. **Destroy is deferred** — like Unity, processed at end of frame
6. **MonoBehaviour lifecycle is auto-dispatched** — engine calls Awake/Start/Update/etc.
7. **SendMessage uses string dispatch** — like Unity, calls named methods on MonoBehaviours
8. **Physics is a static class** — `Physics::Raycast()`, `Physics::OverlapSphere()`
