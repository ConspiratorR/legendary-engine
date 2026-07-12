# Unity Parity Refactoring Design

**Date:** 2026-07-12  
**Status:** Approved  
**Approach:** Clean Break Rewrite  
**Scope:** Full Unity Parity (Core + Scripting + Editor + Serialization)

---

## Overview

This design document outlines a comprehensive refactoring of RustEngine to achieve Unity-like API familiarity and architecture. The goal is to make RustEngine feel like Unity while maintaining Rust's performance and safety guarantees.

### Key Changes

1. **Core Architecture** — Replace ECS with Unity-like GameObject/Component model
2. **Player Loop** — Add Unity-like lifecycle stages (Update, FixedUpdate, LateUpdate, etc.)
3. **MonoBehaviour** — Add component trait with lifecycle callbacks
4. **ScriptableObject** — Add data asset system
5. **Transform & Hierarchy** — Unity-like transform with parent-child relationships
6. **Events & Messaging** — Type-safe event system
7. **Coroutines & Async** — Coroutine and async/await support
8. **Editor Improvements** — Hierarchy, Inspector, Scene View, Prefab, Serialization

---

## Section 1: Core Architecture — GameObject & Component

### New Types

```rust
// engine-core/src/gameobject.rs

/// Base class for all entities in the scene (like Unity's GameObject).
pub struct GameObject {
    name: String,
    tag: String,
    layer: u32,
    active: bool,
    components: Vec<Box<dyn Component>>,
    parent: Option<GameObjectHandle>,
    children: Vec<GameObjectHandle>,
}

/// Lightweight handle to a GameObject (index + generation).
pub struct GameObjectHandle {
    index: u32,
    generation: u32,
}

/// Base trait for all components (like Unity's Component).
pub trait Component: Any + Send + Sync {
    /// Called when the component is added to a GameObject.
    fn on_added(&mut self, _context: &mut Context) {}
    
    /// Called when the component is removed from a GameObject.
    fn on_removed(&mut self, _context: &mut Context) {}
    
    /// Called when the GameObject becomes active.
    fn on_enable(&mut self, _context: &mut Context) {}
    
    /// Called when the GameObject becomes inactive.
    fn on_disable(&mut self, _context: &mut Context) {}
    
    /// Called when the GameObject is destroyed.
    fn on_destroy(&mut self, _context: &mut Context) {}
    
    /// Get the component as Any for downcasting.
    fn as_any(&self) -> &dyn Any;
    
    /// Get the component as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

### GameObject API (Unity-like)

```rust
impl GameObject {
    /// Create a new GameObject (like Unity's new GameObject()).
    pub fn new(name: &str) -> Self { ... }
    
    /// Add a component (like Unity's AddComponent<T>()).
    pub fn add_component<T: Component + 'static>(&mut self, component: T) { ... }
    
    /// Get a component (like Unity's GetComponent<T>()).
    pub fn get_component<T: Component + 'static>(&self) -> Option<&T> { ... }
    
    /// Get a component mutably (like Unity's GetComponent<T>()).
    pub fn get_component_mut<T: Component + 'static>(&mut self) -> Option<&mut T> { ... }
    
    /// Get component in children (like Unity's GetComponentInChildren<T>()).
    pub fn get_componentInChildren<T: Component + 'static>(&self) -> Option<&T> { ... }
    
    /// Set parent (like Unity's SetParent()).
    pub fn set_parent(&mut self, parent: Option<GameObjectHandle>, world_position_stays: bool) { ... }
    
    /// Set active state (like Unity's SetActive()).
    pub fn set_active(&mut self, active: bool) { ... }
    
    /// Find child by name (like Unity's Transform.Find()).
    pub fn find(&self, name: &str) -> Option<GameObjectHandle> { ... }
}
```

### World Container

```rust
// engine-core/src/world.rs

/// Central container for all GameObjects (replaces ECS World).
pub struct World {
    gameobjects: Vec<Option<GameObject>>,
    free_list: Vec<u32>,
    transforms: Vec<Transform>,
    global_transforms: Vec<GlobalTransform>,
    parent_indices: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
}
```

---

## Section 2: Player Loop & Lifecycle

### Unity's Player Loop Stages

Unity's Player Loop has these stages (in order):
1. `Initialization` — Time, Input, Program
2. `EarlyUpdate` — Camera, NavMesh, Timeline
3. `FixedUpdate` — Physics, Animation, UI
4. `Update` — Game logic, input
5. `PreLateUpdate` — Animation, ParticleSystem
6. `PostLateUpdate` — EndOfFrame

### Proposed Player Loop

```rust
// engine-core/src/player_loop.rs

/// The main game loop (like Unity's PlayerLoop).
pub struct PlayerLoop {
    stages: Vec<LoopStage>,
    current_stage: usize,
}

/// A stage in the Player Loop (like Unity's PlayerLoopSystem).
pub struct LoopStage {
    name: String,
    systems: Vec<Box<dyn System>>,
    subsystems: Vec<LoopStage>, // Nested stages
}

/// Execution phase (like Unity's PlayerLoopTiming).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Before FixedUpdate.
    PreFixedUpdate,
    /// Fixed timestep (physics, animation).
    FixedUpdate,
    /// After FixedUpdate.
    PostFixedUpdate,
    /// Before Update.
    PreUpdate,
    /// Main update (game logic, input).
    Update,
    /// After Update.
    PostUpdate,
    /// Before LateUpdate.
    PreLateUpdate,
    /// Late update (camera follow, etc.).
    LateUpdate,
    /// After LateUpdate.
    PostLateUpdate,
    /// Rendering.
    Render,
    /// After rendering.
    AfterRender,
}

/// Context passed to systems during execution.
pub struct Context {
    pub world: &mut World,
    pub time: Time,
    pub input: InputState,
    pub phase: Phase,
}
```

### System Registration

```rust
impl PlayerLoop {
    /// Register a system for a specific phase (like Unity's [ExecuteInEditMode]).
    pub fn add_system(&mut self, phase: Phase, system: impl System + 'static) { ... }
    
    /// Register a system for multiple phases.
    pub fn add_system_for_phases(&mut self, phases: &[Phase], system: impl System + 'static) { ... }
    
    /// Execute one frame.
    pub fn run(&mut self, world: &mut World) { ... }
}

// Convenience methods on AppBuilder
impl AppBuilder {
    /// Add a system that runs every frame (Update phase).
    pub fn add_system(&mut self, system: impl System + 'static) -> &mut Self { ... }
    
    /// Add a system that runs at fixed timestep (FixedUpdate phase).
    pub fn add_fixed_update_system(&mut self, system: impl System + 'static) -> &mut Self { ... }
    
    /// Add a system that runs after all other updates (LateUpdate phase).
    pub fn add_late_update_system(&mut self, system: impl System + 'static) -> &mut Self { ... }
    
    /// Add a startup system (runs once before first frame).
    pub fn add_startup_system(&mut self, system: impl System + 'static) -> &mut Self { ... }
}
```

### Time Management

```rust
// engine-core/src/time.rs

/// Time information (like Unity's Time class).
pub struct Time {
    /// Time since last frame (deltaTime).
    pub delta_time: f32,
    /// Total time since application start (time).
    pub elapsed_time: f32,
    /// Fixed timestep (fixedDeltaTime).
    pub fixed_delta_time: f32,
    /// Time scale (timeScale).
    pub time_scale: f32,
    /// Frame count (frameCount).
    pub frame_count: u64,
    /// Whether we're in FixedUpdate.
    pub in_fixed_update: bool,
}

impl Time {
    /// Get delta time (scaled by timeScale).
    pub fn deltaTime(&self) -> f32 { ... }
    
    /// Get unscaled delta time.
    pub fn unscaledDeltaTime(&self) -> f32 { ... }
    
    /// Get fixed delta time.
    pub fn fixedDeltaTime(&self) -> f32 { ... }
    
    /// Get total elapsed time.
    pub fn time(&self) -> f32 { ... }
}
```

---

## Section 3: MonoBehaviour & Lifecycle Callbacks

### MonoBehaviour Trait

```rust
// engine-core/src/monobehaviour.rs

/// Base class for user scripts (like Unity's MonoBehaviour).
/// Components can implement this trait to receive lifecycle callbacks.
pub trait MonoBehaviour: Component {
    /// Called when the script instance is being loaded (like Unity's Awake).
    fn awake(&mut self, _context: &mut Context) {}
    
    /// Called on the frame when the script is enabled (like Unity's OnEnable).
    fn on_enable(&mut self, _context: &mut Context) {}
    
    /// Called on the frame when the script is disabled (like Unity's OnDisable).
    fn on_disable(&mut self, _context: &mut Context) {}
    
    /// Called before the first frame update (like Unity's Start).
    fn start(&mut self, _context: &mut Context) {}
    
    /// Called once per frame (like Unity's Update).
    fn update(&mut self, _context: &mut Context) {}
    
    /// Called at fixed intervals (like Unity's FixedUpdate).
    fn fixed_update(&mut self, _context: &mut Context) {}
    
    /// Called after all Update calls (like Unity's LateUpdate).
    fn late_update(&mut self, _context: &mut Context) {}
    
    /// Called when the script is destroyed (like Unity's OnDestroy).
    fn on_destroy(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse enters the Collider (like Unity's OnMouseEnter).
    fn on_mouse_enter(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse exits the Collider (like Unity's OnMouseExit).
    fn on_mouse_exit(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse is pressed on the Collider (like Unity's OnMouseDown).
    fn on_mouse_down(&mut self, _context: &mut Context) {}
    
    /// Called when the mouse button is released (like Unity's OnMouseUp).
    fn on_mouse_up(&mut self, _context: &mut Context) {}
    
    /// Called when a collision starts (like Unity's OnCollisionEnter).
    fn on_collision_enter(&mut self, _context: &mut Context, collision: &Collision) {}
    
    /// Called when a collision ends (like Unity's OnCollisionExit).
    fn on_collision_exit(&mut self, _context: &mut Context, collision: &Collision) {}
    
    /// Called when a trigger is entered (like Unity's OnTriggerEnter).
    fn on_trigger_enter(&mut self, _context: &mut Context, other: &Collider) {}
    
    /// Called when a trigger is exited (like Unity's OnTriggerExit).
    fn on_trigger_exit(&mut self, _context: &mut Context, other: &Collider) {}
    
    /// Called for drawing gizmos (like Unity's OnDrawGizmos).
    fn on_draw_gizmos(&self, _context: &Context) {}
}
```

### Automatic System Generation

The `MonoBehaviour` derive macro automatically generates ECS systems from MonoBehaviour implementations. This bridges the gap between Unity's component model and ECS's system-based execution.

```rust
// Procedural macro to automatically generate systems from MonoBehaviour implementations

/// Derive macro to automatically generate systems from MonoBehaviour implementations.
/// 
/// # Example
/// ```rust
/// #[derive(Component, MonoBehaviour)]
/// struct PlayerController {
///     speed: f32,
/// }
/// 
/// impl MonoBehaviour for PlayerController {
///     fn update(&mut self, context: &mut Context) {
///         // Game logic
///     }
/// }
/// ```
/// 
/// This automatically generates:
/// - A system that calls `update()` on all `PlayerController` components
/// - Registration in the Update phase
/// - Lifecycle management (enable/disable)
/// - Component registration in the World
#[proc_macro_derive(MonoBehaviour)]
pub fn monobehaviour_derive(input: TokenStream) -> TokenStream { ... }
```

### Generated Code Example

For the `PlayerController` example above, the macro generates:

```rust
// Auto-generated system
struct PlayerControllerUpdateSystem;

impl System for PlayerControllerUpdateSystem {
    fn run(&self, world: &mut World) {
        for handle in world.all_gameobjects() {
            let gameobject = world.get_gameobject_mut(handle);
            if let Some(controller) = gameobject.get_component_mut::<PlayerController>() {
                if gameobject.is_active() && controller.is_enabled() {
                    let mut context = Context::new(world);
                    controller.update(&mut context);
                }
            }
        }
    }
}

// Auto-registration in PlayerLoop
impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(Phase::Update, PlayerControllerUpdateSystem);
    }
}
```

### Usage Example

```rust
// User code (looks like Unity C#)
#[derive(Component, MonoBehaviour)]
struct PlayerController {
    speed: f32,
    jump_force: f32,
    is_grounded: bool,
}

impl MonoBehaviour for PlayerController {
    fn start(&mut self, context: &mut Context) {
        println!("Player started!");
    }
    
    fn update(&mut self, context: &mut Context) {
        let input = &context.input;
        let transform = context.world.get_component_mut::<Transform>(self.entity).unwrap();
        
        // Movement
        let horizontal = input.get_axis("Horizontal");
        transform.translation.x += horizontal * self.speed * context.time.deltaTime();
        
        // Jump
        if input.get_button_down("Jump") && self.is_grounded {
            // Apply jump force
        }
    }
    
    fn fixed_update(&mut self, context: &mut Context) {
        // Physics-based movement
    }
    
    fn on_collision_enter(&mut self, _context: &mut Context, collision: &Collision) {
        if collision.normal.y > 0.7 {
            self.is_grounded = true;
        }
    }
}

// Setup code
fn main() {
    let mut app = AppBuilder::new();
    
    let mut player = GameObject::new("Player");
    player.add_component(Transform::from_xyz(0.0, 1.0, 0.0));
    player.add_component(RigidBody::new_dynamic());
    player.add_component(BoxCollider::new(Vec3::ONE));
    player.add_component(PlayerController { speed: 5.0, jump_force: 10.0, is_grounded: false });
    
    app.add_gameobject(player);
    app.run();
}
```

---

## Section 4: ScriptableObject & Data Assets

### ScriptableObject Trait

```rust
// engine-core/src/scriptable_object.rs

/// Base class for data assets (like Unity's ScriptableObject).
/// Independent of GameObjects, can be shared across multiple instances.
pub trait ScriptableObject: Any + Send + Sync + Serialize + DeserializeOwned {
    /// Called when the ScriptableObject is created (like Unity's OnCreate).
    fn on_create(&mut self) {}
    
    /// Called when the ScriptableObject is loaded (like Unity's OnEnable).
    fn on_enable(&mut self) {}
    
    /// Called when the ScriptableObject is disabled (like Unity's OnDisable).
    fn on_disable(&mut self) {}
    
    /// Called when the ScriptableObject is destroyed (like Unity's OnDestroy).
    fn on_destroy(&mut self) {}
    
    /// Get the name of the asset.
    fn name(&self) -> &str;
    
    /// Get the asset path (if loaded from disk).
    fn asset_path(&self) -> Option<&str> { None }
}
```

### Asset Handle

```rust
// engine-asset/src/handle.rs

/// Strong reference to a ScriptableObject asset (like Unity's asset reference).
/// Uses reference counting for automatic cleanup.
pub struct AssetHandle<T: ScriptableObject> {
    inner: Arc<T>,
    path: Option<String>,
}

impl<T: ScriptableObject> AssetHandle<T> {
    /// Get a reference to the asset.
    pub fn get(&self) -> &T { &self.inner }
    
    /// Get a mutable reference to the asset.
    pub fn get_mut(&mut self) -> &mut T { &mut Arc::make_mut(&mut self.inner) }
    
    /// Check if the asset is loaded.
    pub fn is_loaded(&self) -> bool { true }
    
    /// Get the asset path.
    pub fn path(&self) -> Option<&str> { self.path.as_deref() }
}
```

### Asset Database

```rust
// engine-asset/src/database.rs

/// Central registry for all assets (like Unity's AssetDatabase).
pub struct AssetDatabase {
    assets: HashMap<String, Box<dyn Any + Send + Sync>>,
    watchers: Vec<Box<dyn AssetWatcher>>,
}

impl AssetDatabase {
    /// Load an asset from disk (like Unity's AssetDatabase.LoadAssetAtPath<T>()).
    pub fn load<T: ScriptableObject + 'static>(&self, path: &str) -> Result<AssetHandle<T>, AssetError> { ... }
    
    /// Load an asset asynchronously.
    pub fn load_async<T: ScriptableObject + 'static>(&self, path: &str) -> AssetFuture<T> { ... }
    
    /// Save an asset to disk (like Unity's AssetDatabase.SaveAssets()).
    pub fn save<T: ScriptableObject>(&self, asset: &AssetHandle<T>, path: &str) -> Result<(), AssetError> { ... }
    
    /// Create a new asset (like Unity's ScriptableObject.CreateInstance<T>()).
    pub fn create_instance<T: ScriptableObject + Default>(&mut self) -> AssetHandle<T> { ... }
    
    /// Find assets of a type (like Unity's Resources.FindObjectsOfTypeAll<T>()).
    pub fn find_assets<T: ScriptableObject + 'static>(&self) -> Vec<AssetHandle<T>> { ... }
    
    /// Get all assets in a folder.
    pub fn get_assets_in_folder(&self, folder: &str) -> Vec<String> { ... }
}
```

### Example: Game Data Assets

```rust
// Example: Creating a ScriptableObject for game data

#[derive(Serialize, Deserialize, ScriptableObject)]
struct CharacterStats {
    name: String,
    max_health: f32,
    speed: f32,
    attack_power: f32,
    defense: f32,
}

impl ScriptableObject for CharacterStats {
    fn name(&self) -> &str { &self.name }
}

// Usage
fn main() {
    let mut db = AssetDatabase::new();
    
    // Create new asset
    let mut stats = db.create_instance::<CharacterStats>();
    stats.name = "Player".to_string();
    stats.max_health = 100.0;
    stats.speed = 5.0;
    
    // Save to disk
    db.save(&stats, "Assets/Data/PlayerStats.asset").unwrap();
    
    // Load from disk
    let loaded_stats = db.load::<CharacterStats>("Assets/Data/PlayerStats.asset").unwrap();
    println!("Loaded: {} (HP: {})", loaded_stats.get().name, loaded_stats.get().max_health);
}
```

### Resources Folder

```rust
// Special folder for runtime-loadable assets (like Unity's Resources folder)

/// Load an asset from the Resources folder (like Unity's Resources.Load<T>()).
impl World {
    pub fn load_resource<T: ScriptableObject + 'static>(&self, name: &str) -> Option<AssetHandle<T>> { ... }
}
```

---

## Section 5: Transform & Hierarchy

### Transform Component

```rust
// engine-core/src/transform.rs

/// Transform component (like Unity's Transform).
/// Stores position, rotation, scale relative to parent.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    /// Local position (relative to parent).
    pub local_position: Vec3,
    /// Local rotation (relative to parent).
    pub local_rotation: Quat,
    /// Local scale (relative to parent).
    pub local_scale: Vec3,
    
    // Cached world transform (computed by sync system)
    #[serde(skip)]
    world_position: Vec3,
    #[serde(skip)]
    world_rotation: Quat,
    #[serde(skip)]
    world_scale: Vec3,
}

impl Transform {
    /// Get/set world position (like Unity's Transform.position).
    pub fn position(&self) -> Vec3 { self.world_position }
    pub fn set_position(&mut self, position: Vec3) { ... }
    
    /// Get/set world rotation (like Unity's Transform.rotation).
    pub fn rotation(&self) -> Quat { self.world_rotation }
    pub fn set_rotation(&mut self, rotation: Quat) { ... }
    
    /// Get/set world scale (like Unity's Transform.lossyScale).
    pub fn lossy_scale(&self) -> Vec3 { self.world_scale }
    
    /// Get/set local position (like Unity's Transform.localPosition).
    pub fn local_position(&self) -> Vec3 { self.local_position }
    pub fn set_local_position(&mut self, position: Vec3) { ... }
    
    /// Get/set local rotation (like Unity's Transform.localRotation).
    pub fn local_rotation(&self) -> Quat { self.local_rotation }
    pub fn set_local_rotation(&mut self, rotation: Quat) { ... }
    
    /// Get/set local scale (like Unity's Transform.localScale).
    pub fn local_scale(&self) -> Vec3 { self.local_scale }
    pub fn set_local_scale(&mut self, scale: Vec3) { ... }
    
    /// Get forward direction (like Unity's Transform.forward).
    pub fn forward(&self) -> Vec3 { ... }
    
    /// Get right direction (like Unity's Transform.right).
    pub fn right(&self) -> Vec3 { ... }
    
    /// Get up direction (like Unity's Transform.up).
    pub fn up(&self) -> Vec3 { ... }
    
    /// Transform a point from local to world space (like Unity's Transform.TransformPoint).
    pub fn transform_point(&self, point: Vec3) -> Vec3 { ... }
    
    /// Transform a point from world to local space (like Unity's Transform.InverseTransformPoint).
    pub fn inverse_transform_point(&self, point: Vec3) -> Vec3 { ... }
    
    /// Transform a direction from local to world space (like Unity's Transform.TransformDirection).
    pub fn transform_direction(&self, direction: Vec3) -> Vec3 { ... }
    
    /// Transform a direction from world to local space (like Unity's Transform.InverseTransformDirection).
    pub fn inverse_transform_direction(&self, direction: Vec3) -> Vec3 { ... }
    
    /// Look at a target position (like Unity's Transform.LookAt).
    pub fn look_at(&mut self, target: Vec3) { ... }
    
    /// Rotate around a point (like Unity's Transform.RotateAround).
    pub fn rotate_around(&mut self, point: Vec3, axis: Vec3, angle: f32) { ... }
    
    /// Translate in world/local space (like Unity's Transform.Translate).
    pub fn translate(&mut self, translation: Vec3, space: Space) { ... }
}
```

### Hierarchy System

```rust
// engine-core/src/hierarchy.rs

/// System that synchronizes world transforms from local transforms.
/// Runs in the PreUpdate phase (before gameplay systems).
pub fn sync_transforms(world: &mut World) {
    // For each GameObject with a Transform:
    // 1. If parent exists, compute world = parent.world * local
    // 2. If no parent, world = local
    // 3. Recursively update children
}

/// System that handles parent-child relationships.
pub struct HierarchySystem;

impl System for HierarchySystem {
    fn run(&self, world: &mut World) {
        sync_transforms(world);
    }
}

// GameObject hierarchy methods
impl GameObject {
    /// Get parent (like Unity's Transform.parent).
    pub fn parent(&self) -> Option<GameObjectHandle> { ... }
    
    /// Get children (like Unity's Transform.childCount, Transform.GetChild()).
    pub fn child_count(&self) -> usize { ... }
    pub fn get_child(&self, index: usize) -> Option<GameObjectHandle> { ... }
    
    /// Get root (like Unity's Transform.root).
    pub fn root(&self) -> GameObjectHandle { ... }
    
    /// Set parent (like Unity's Transform.SetParent()).
    pub fn set_parent(&mut self, parent: Option<GameObjectHandle>, world_position_stays: bool) { ... }
    
    /// Detach all children (like Unity's Transform.DetachChildren).
    pub fn detach_children(&mut self) { ... }
    
    /// Find child by name (like Unity's Transform.Find()).
    pub fn find(&self, name: &str) -> Option<GameObjectHandle> { ... }
    
    /// Get sibling index (like Unity:: Transform.GetSiblingIndex).
    pub fn get_sibling_index(&self) -> usize { ... }
    
    /// Set sibling index (like Unity:: Transform.SetSiblingIndex).
    pub fn set_sibling_index(&mut self, index: usize) { ... }
}
```

### Example: Hierarchy

```rust
// Example: Creating a parent-child hierarchy

fn main() {
    let mut world = World::new();
    
    // Create parent
    let mut parent = GameObject::new("Parent");
    parent.add_component(Transform::from_xyz(0.0, 0.0, 0.0));
    let parent_handle = world.spawn(parent);
    
    // Create child
    let mut child = GameObject::new("Child");
    child.add_component(Transform::from_xyz(1.0, 0.0, 0.0));
    let child_handle = world.spawn(child);
    
    // Set parent-child relationship
    world.get_gameobject_mut(child_handle).set_parent(Some(parent_handle), true);
    
    // Move parent
    world.get_gameobject_mut(parent_handle)
        .get_component_mut::<Transform>()
        .set_position(Vec3::new(5.0, 0.0, 0.0));
    
    // Child world position is now (6.0, 0.0, 0.0)
    let child_transform = world.get_gameobject(child_handle)
        .get_component::<Transform>();
    println!("Child world position: {:?}", child_transform.position());
}
```

---

## Section 6: Events & Messaging

### Event System

```rust
// engine-core/src/events.rs

/// Type-safe event bus (like Unity's SendMessage, but type-safe).
pub struct EventBus {
    handlers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
}

/// Trait for event handlers.
pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &dyn Any, context: &mut Context);
}

/// Event marker trait.
pub trait Event: Any + Send + Sync + Clone {}

impl EventBus {
    /// Register a handler for an event type.
    pub fn on<E: Event + 'static>(&mut self, handler: impl EventHandler + 'static) { ... }
    
    /// Send an event to all handlers (like Unity's SendMessage).
    pub fn send<E: Event + 'static>(&mut self, event: E, context: &mut Context) { ... }
    
    /// Send an event to a specific GameObject (like Unity's SendMessage with target).
    pub fn send_to<E: Event + 'static>(&mut self, target: GameObjectHandle, event: E, context: &mut Context) { ... }
}

// Built-in events (like Unity's built-in messages)
#[derive(Event, Clone)]
pub struct CollisionEnter {
    pub entity: GameObjectHandle,
    pub collision: Collision,
}

#[derive(Event, Clone)]
pub struct CollisionExit {
    pub entity: GameObjectHandle,
    pub collision: Collision,
}

#[derive(Event, Clone)]
pub struct TriggerEnter {
    pub entity: GameObjectHandle,
    pub other: GameObjectHandle,
}

#[derive(Event, Clone)]
pub struct TriggerExit {
    pub entity: GameObjectHandle,
    pub other: GameObjectHandle,
}

#[derive(Event, Clone)]
pub struct MouseEnter {
    pub entity: GameObjectHandle,
}

#[derive(Event, Clone)]
pub struct MouseExit {
    pub entity: GameObjectHandle,
}

#[derive(Event, Clone)]
pub struct MouseDown {
    pub entity: GameObjectHandle,
    pub button: MouseButton,
}

#[derive(Event, Clone)]
pub struct MouseUp {
    pub entity: GameObjectHandle,
    pub button: MouseButton,
}
```

### Unity-like SendMessage

```rust
// Convenience methods on GameObject
impl GameObject {
    /// Send a message to all components on this GameObject (like Unity's SendMessage).
    pub fn send_message(&mut self, method_name: &str, context: &mut Context) { ... }
    
    /// Send a message to all components on this GameObject and its children (like Unity's BroadcastMessage).
    pub fn broadcast_message(&mut self, method_name: &str, context: &mut Context) { ... }
    
    /// Send a message to all components on this GameObject and its parents (like Unity's SendMessageUpwards).
    pub fn send_message_upwards(&mut self, method_name: &str, context: &mut Context) { ... }
}
```

### Example: Events

```rust
// Example: Using events

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Event, Clone)]
pub struct HealthChanged {
    pub entity: GameObjectHandle,
    pub old_health: f32,
    pub new_health: f32,
}

#[derive(Event, Clone)]
pub struct EntityDied {
    pub entity: GameObjectHandle,
}

impl MonoBehaviour for Health {
    fn take_damage(&mut self, context: &mut Context, amount: f32) {
        let old_health = self.current;
        self.current = (self.current - amount).max(0.0);
        
        // Send health changed event
        context.events.send(HealthChanged {
            entity: self.entity,
            old_health,
            new_health: self.current,
        });
        
        // Check for death
        if self.current <= 0.0 {
            context.events.send(EntityDied {
                entity: self.entity,
            });
        }
    }
}

// Listen for events
struct DeathHandler;

impl EventHandler for DeathHandler {
    fn handle(&mut self, event: &dyn Any, context: &mut Context) {
        if let Some(death) = event.downcast_ref::<EntityDied>() {
            println!("Entity {:?} died!", death.entity);
            // Play death animation, drop loot, etc.
        }
    }
}
```

---

## Section 7: Coroutines & Async

### Coroutine System

```rust
// engine-core/src/coroutine.rs

/// A coroutine that can yield control and resume later (like Unity's Coroutine).
pub struct Coroutine {
    id: u64,
    state: CoroutineState,
    generator: Box<dyn Generator<Yield = Yield, Return = ()>>,
}

/// State of a coroutine.
enum CoroutineState {
    Running,
    Yielded(Yield),
    Completed,
}

/// Yield instruction (like Unity's YieldInstruction).
pub enum Yield {
    /// Wait for one frame (like yield return null).
    WaitOneFrame,
    /// Wait for seconds (like yield return new WaitForSeconds(time)).
    WaitSeconds(f32),
    /// Wait for a condition (like yield return new WaitUntil(condition)).
    WaitUntil(Box<dyn Fn() -> bool + Send + Sync>),
    /// Wait for a coroutine to finish (like yield return coroutine).
    WaitCoroutine(u64),
    /// Wait for end of frame (like yield return new WaitForEndOfFrame()).
    WaitForEndOfFrame,
    /// Wait for fixed update (like yield return new WaitForFixedUpdate()).
    WaitForFixedUpdate,
}

/// Coroutine runner system.
pub struct CoroutineRunner {
    coroutines: Vec<Coroutine>,
    next_id: u64,
}

impl CoroutineRunner {
    /// Start a new coroutine (like Unity's StartCoroutine).
    pub fn start_coroutine(&mut self, generator: impl Generator<Yield = Yield, Return = ()> + 'static) -> u64 { ... }
    
    /// Stop a coroutine (like Unity's StopCoroutine).
    pub fn stop_coroutine(&mut self, id: u64) { ... }
    
    /// Stop all coroutines (like Unity's StopAllCoroutines).
    pub fn stop_all_coroutines(&mut self) { ... }
    
    /// Update all coroutines (called by the coroutine system).
    pub fn update(&mut self, context: &mut Context) { ... }
}
```

### MonoBehaviour Coroutine Support

```rust
// Convenience methods on MonoBehaviour
impl dyn MonoBehaviour {
    /// Start a coroutine on this component (like Unity's StartCoroutine).
    pub fn start_coroutine(&mut self, context: &mut Context, generator: impl Generator<Yield = Yield, Return = ()> + 'static) -> u64 { ... }
    
    /// Stop a coroutine (like Unity's StopCoroutine).
    pub fn stop_coroutine(&mut self, context: &mut Context, id: u64) { ... }
    
    /// Stop all coroutines on this component (like Unity's StopAllCoroutines).
    pub fn stop_all_coroutines(&mut self, context: &mut Context) { ... }
}
```

### Async/Await Support

```rust
// Rust-native async/await support (more idiomatic than coroutines)

/// Async task that can be spawned on the world (like Unity's async operations).
pub struct AsyncTask<T> {
    id: u64,
    future: Pin<Box<dyn Future<Output = T>>>,
    state: TaskState<T>,
}

impl<T> AsyncTask<T> {
    /// Poll the task (returns Ready if complete, Pending otherwise).
    pub fn poll(&mut self, context: &mut Context) -> Poll<T> { ... }
    
    /// Check if the task is complete.
    pub fn is_complete(&self) -> bool { ... }
    
    /// Get the result (panics if not complete).
    pub fn result(&self) -> &T { ... }
}

// Convenience methods on World
impl World {
    /// Spawn an async task (like Unity's Task.Run).
    pub async fn spawn_async<T: Send + 'static>(&mut self, future: impl Future<Output = T> + Send + 'static) -> AsyncTask<T> { ... }
    
    /// Load a resource asynchronously (like Unity's Addressables.LoadAssetAsync).
    pub async fn load_asset_async<T: ScriptableObject + 'static>(&mut self, path: &str) -> AssetHandle<T> { ... }
}
```

### Example: Coroutines

```rust
// Example: Using coroutines

impl MonoBehaviour for PlayerController {
    fn start(&mut self, context: &mut Context) {
        // Start a coroutine
        self.start_coroutine(context, respawn_coroutine());
    }
}

// Coroutine function (uses Rust's generator syntax)
fn respawn_coroutine() -> impl Generator<Yield = Yield, Return = ()> {
    move || {
        // Wait for 2 seconds
        yield Yield::WaitSeconds(2.0);
        
        // Reset position
        // ... reset logic ...
        
        // Wait for end of frame
        yield Yield::WaitForEndOfFrame;
        
        // Enable player
        // ... enable logic ...
    }
}

// Example: Using async/await
impl MonoBehaviour for PlayerController {
    fn update(&mut self, context: &mut Context) {
        // Check if we're loading a level
        if self.loading_level.is_none() {
            // Start async load
            let handle = context.world.spawn_async(load_level_async("Level1".to_string()));
            self.loading_level = Some(handle);
        }
        
        // Check if load is complete
        if let Some(handle) = &self.loading_level {
            if handle.is_complete() {
                println!("Level loaded!");
                self.loading_level = None;
            }
        }
    }
}

async fn load_level_async(level_name: String) -> Level {
    // Async level loading
    let assets = load_assets_async(&level_name).await;
    let level = instantiate_level(&assets).await;
    level
}
```

---

## Section 8: Editor Improvements

### Scene Hierarchy

```rust
// engine-editor/src/hierarchy.rs

/// Scene hierarchy panel (like Unity's Hierarchy window).
pub struct HierarchyPanel {
    selected: Option<GameObjectHandle>,
    expanded: HashSet<GameObjectHandle>,
    search_query: String,
    drag_state: Option<DragState>,
}

impl HierarchyPanel {
    /// Render the hierarchy panel.
    pub fn render(&mut self, ui: &mut egui::Ui, world: &mut World) {
        // Draw root GameObjects
        for root in world.root_gameobjects() {
            self.render_gameobject(ui, world, root, 0);
        }
    }
    
    /// Render a GameObject and its children.
    fn render_gameobject(&mut self, ui: &mut egui::Ui, world: &mut World, handle: GameObjectHandle, depth: usize) {
        let gameobject = world.get_gameobject(handle);
        let is_selected = self.selected == Some(handle);
        let has_children = gameobject.child_count() > 0;
        let is_expanded = self.expanded.contains(&handle);
        
        // Draw selection highlight
        let response = ui.horizontal(|ui| {
            // Indent based on depth
            ui.add_space(depth as f32 * 16.0);
            
            // Expand/collapse arrow
            if has_children {
                let arrow = if is_expanded { "▼" } else { "▶" };
                if ui.small_button(arrow).clicked() {
                    self.toggle_expanded(handle);
                }
            } else {
                ui.add_space(16.0);
            }
            
            // Active checkbox
            let mut active = gameobject.is_active();
            ui.checkbox(&mut active, "");
            
            // Icon (based on components)
            let icon = self.get_icon(world, handle);
            ui.label(icon);
            
            // Name (editable)
            let name = gameobject.name().to_string();
            let response = ui.text_edit_singleline(&mut name.clone());
            if response.changed() {
                world.get_gameobject_mut(handle).set_name(&name);
            }
        });
        
        // Handle selection
        if response.inner.interact(egui::Sense::click()).clicked() {
            self.selected = Some(handle);
        }
        
        // Handle drag-and-drop
        if response.inner.interact(egui::Sense::drag()).drag_started() {
            self.drag_state = Some(DragState::Dragging(handle));
        }
        
        // Render children if expanded
        if is_expanded {
            for child in gameobject.children() {
                self.render_gameobject(ui, world, child, depth + 1);
            }
        }
    }
}
```

### Inspector Panel

```rust
// engine-editor/src/inspector.rs

/// Inspector panel (like Unity's Inspector window).
pub struct InspectorPanel {
    selected: Option<GameObjectHandle>,
    scroll_position: f32,
}

impl InspectorPanel {
    /// Render the inspector panel.
    pub fn render(&mut self, ui: &mut egui::Ui, world: &mut World) {
        let Some(handle) = self.selected else {
            ui.centered_and_justified(|ui| ui.label("No selection"));
            return;
        };
        
        let gameobject = world.get_gameobject(handle);
        
        // Header
        ui.horizontal(|ui| {
            let mut active = gameobject.is_active();
            ui.checkbox(&mut active, "");
            
            let name = gameobject.name().to_string();
            ui.text_edit_singleline(&mut name.clone());
            
            // Static checkbox
            let mut is_static = gameobject.is_static();
            ui.checkbox(&mut is_static, "Static");
            
            // Tag
            let tag = gameobject.tag().to_string();
            ui.label("Tag:");
            ui.text_edit_singleline(&mut tag.clone());
            
            // Layer
            let layer = gameobject.layer();
            ui.label("Layer:");
            uiComboBox(ui, &mut layer.clone(), &LAYERS);
        });
        
        ui.separator();
        
        // Components
        for component in gameobject.components() {
            self.render_component(ui, world, handle, component);
        }
        
        // Add Component button
        if ui.button("Add Component").clicked() {
            // Show component picker menu
        }
    }
    
    /// Render a component in the inspector.
    fn render_component(&mut self, ui: &mut egui::Ui, world: &mut World, handle: GameObjectHandle, component: &dyn Component) {
        let type_name = std::any::type_name_of_val(component);
        
        // Collapsible header
        let header = egui::CollapsingHeader::new(type_name)
            .default_open(true)
            .show(ui, |ui| {
                // Render component properties
                // Use reflection or manual implementation
            });
    }
}
```

### Scene View

```rust
// engine-editor/src/scene_view.rs

/// Scene view (like Unity's Scene View).
pub struct SceneView {
    camera: EditorCamera,
    gizmo_mode: GizmoMode,
    grid_visible: bool,
    selected_objects: Vec<GameObjectHandle>,
}

impl SceneView {
    /// Render the scene view.
    pub fn render(&mut self, ui: &mut egui::Ui, world: &mut World) {
        // Draw grid
        if self.grid_visible {
            self.draw_grid(ui);
        }
        
        // Draw all GameObjects
        for handle in world.all_gameobjects() {
            self.draw_gameobject(ui, world, handle);
        }
        
        // Draw gizmos for selected objects
        for handle in &self.selected_objects {
            self.draw_gizmos(ui, world, *handle);
        }
        
        // Handle mouse input for selection, navigation, etc.
        self.handle_input(ui, world);
    }
    
    /// Draw a GameObject in the scene view.
    fn draw_gameobject(&self, ui: &mut egui::Ui, world: &mut World, handle: GameObjectHandle) {
        let gameobject = world.get_gameobject(handle);
        let transform = gameobject.get_component::<Transform>().unwrap();
        
        // Draw based on components
        if gameobject.has_component::<MeshRenderer>() {
            self.draw_mesh(ui, world, handle);
        } else if gameobject.has_component::<SpriteRenderer>() {
            self.draw_sprite(ui, world, handle);
        } else if gameobject.has_component::<Camera>() {
            self.draw_camera(ui, world, handle);
        } else if gameobject.has_component::<Light>() {
            self.draw_light(ui, world, handle);
        } else {
            // Draw icon for empty GameObjects
            self.draw_icon(ui, transform.position(), "GameObject");
        }
    }
    
    /// Draw gizmos for a GameObject.
    fn draw_gizmos(&self, ui: &mut egui::Ui, world: &mut World, handle: GameObjectHandle) {
        let gameobject = world.get_gameobject(handle);
        
        // Transform gizmo
        match self.gizmo_mode {
            GizmoMode::Translate => self.draw_translate_gizmo(ui, world, handle),
            GizmoMode::Rotate => self.draw_rotate_gizmo(ui, world, handle),
            GizmoMode::Scale => self.draw_scale_gizmo(ui, world, handle),
        }
        
        // Collider gizmos
        if let Some(collider) = gameobject.get_component::<BoxCollider>() {
            self.draw_box_collider_gizmo(ui, world, handle, collider);
        }
        if let Some(collider) = gameobject.get_component::<SphereCollider>() {
            self.draw_sphere_collider_gizmo(ui, world, handle, collider);
        }
        
        // Custom gizmos
        if let Some(monobehaviour) = gameobject.get_component::<dyn MonoBehaviour>() {
            monobehaviour.on_draw_gizmos(&Context::new(world));
        }
    }
}
```

### Prefab System

```rust
// engine-core/src/prefab.rs

/// Prefab asset (like Unity's Prefab).
pub struct Prefab {
    name: String,
    root: GameObject,
    overrides: HashMap<String, serde_json::Value>,
}

impl Prefab {
    /// Create a new prefab from a GameObject (like Unity's PrefabUtility.SaveAsPrefabAsset).
    pub fn create(gameobject: &GameObject) -> Self { ... }
    
    /// Instantiate the prefab (like Unity's Instantiate(prefab)).
    pub fn instantiate(&self, world: &mut World) -> GameObjectHandle { ... }
    
    /// Apply overrides from instance to prefab (like Unity's PrefabUtility.ApplyPrefabInstance).
    pub fn apply_overrides(&mut self, instance: &GameObject) { ... }
    
    /// Revert instance to prefab values (like Unity's PrefabUtility.RevertPrefabInstance).
    pub fn revert(&self, instance: &mut GameObject) { ... }
    
    /// Check if instance has been modified from prefab (like Unity's PrefabUtility.HasPrefabInstanceAnyOverrides).
    pub fn has_overrides(&self) -> bool { ... }
    
    /// Get override paths (like Unity's PrefabUtility.GetRemovedComponents).
    pub fn get_overrides(&self) -> Vec<String> { ... }
}
```

### Serialization

```rust
// engine-core/src/serialization.rs

/// Scene serialization (like Unity's SceneUtility).
pub struct SceneSerializer {
    formatters: HashMap<TypeId, Box<dyn ComponentFormatter>>,
}

impl SceneSerializer {
    /// Save scene to file (like Unity's EditorSceneManager.SaveScene).
    pub fn save_scene(&self, world: &World, path: &str) -> Result<(), SerializationError> { ... }
    
    /// Load scene from file (like Unity's EditorSceneManager.OpenScene).
    pub fn load_scene(&self, path: &str) -> Result<World, SerializationError> { ... }
    
    /// Serialize a GameObject to JSON.
    pub fn serialize_gameobject(&self, gameobject: &GameObject) -> Result<String, SerializationError> { ... }
    
    /// Deserialize a GameObject from JSON.
    pub fn deserialize_gameobject(&self, json: &str) -> Result<GameObject, SerializationError> { ... }
}

/// Format for serializing a component.
pub trait ComponentFormatter: Send + Sync {
    fn serialize(&self, component: &dyn Component) -> Result<serde_json::Value, SerializationError>;
    fn deserialize(&self, value: &serde_json::Value) -> Result<Box<dyn Component>, SerializationError>;
}
```

### Undo/Redo System

```rust
// engine-editor/src/undo.rs

/// Undo/redo system (like Unity's Undo class).
pub struct UndoSystem {
    undo_stack: Vec<Box<dyn UndoCommand>>,
    redo_stack: Vec<Box<dyn UndoCommand>>,
    current_group: usize,
}

/// Command that can be undone.
pub trait UndoCommand: Send + Sync {
    fn execute(&mut self, world: &mut World);
    fn undo(&mut self, world: &mut World);
    fn redo(&mut self, world: &mut World);
    fn name(&self) -> &str;
}

impl UndoSystem {
    /// Record an undo operation (like Unity:: Undo.RegisterCreatedObjectUndo).
    pub fn register_created_object(&mut self, handle: GameObjectHandle, name: &str) { ... }
    
    /// Record a destruction operation (like Unity:: Undo.DestroyObjectImmediate).
    pub fn destroy_object(&mut self, handle: GameObjectHandle, name: &str) { ... }
    
    /// Record a property change (like Unity:: Undo.RecordObject).
    pub fn record_object(&mut self, handle: GameObjectHandle, name: &str) { ... }
    
    /// Undo the last operation (like Unity:: Undo.PerformUndo).
    pub fn undo(&mut self, world: &mut World) { ... }
    
    /// Redo the last undone operation (like Unity:: Undo.PerformRedo).
    pub fn redo(&mut self, world: &mut World) { ... }
    
    /// Check if undo is possible.
    pub fn can_undo(&self) -> bool { ... }
    
    /// Check if redo is possible.
    pub fn can_redo(&self) -> bool { ... }
    
    /// Clear all undo/redo history (like Unity:: Undo.ClearUndo).
    pub fn clear(&mut self) { ... }
}
```

---

## Migration Strategy

### Backward Compatibility

The old ECS architecture will be preserved as an optional backend:

```toml
# Cargo.toml features
[features]
default = ["unity-api"]
unity-api = []  # New Unity-like API
legacy-ecs = []  # Old ECS backend
```

### Migration Path

1. **Phase 1:** Implement new Unity-like API alongside old ECS
2. **Phase 2:** Add `engine-ecs-legacy` crate that wraps old ECS
3. **Phase 3:** Provide migration utilities to convert ECS code to new API
4. **Phase 4:** Deprecate old ECS API with warnings
5. **Phase 5:** Remove old ECS API in next major version

### Coexistence Example

```rust
// Old ECS code (still works with legacy-ecs feature)
#[cfg(feature = "legacy-ecs")]
fn old_system(world: &mut World) {
    let query = QueryPair::<Position, Velocity>::new();
    for (pos, vel) in query.iter_mut(world) {
        pos.0 += vel.0 * 0.016;
    }
}

// New Unity-like API
fn new_system(context: &mut Context) {
    let query = Query::<(Transform, Velocity)>::new();
    for (transform, velocity) in query.iter(context.world) {
        transform.set_position(transform.position() + velocity.0 * context.time.deltaTime());
    }
}
```

---

## Migration Plan

### Phase 1: Core Architecture (Weeks 1-2)

1. Create new `engine-core` module structure
2. Implement `GameObject`, `Component`, `GameObjectHandle`
3. Implement `World` container
4. Add basic `Transform` component
5. Implement `PlayerLoop` with basic phases
6. Update `AppBuilder` to use new architecture

### Phase 2: MonoBehaviour & Lifecycle (Weeks 3-4)

1. Implement `MonoBehaviour` trait
2. Create procedural macro for automatic system generation
3. Implement lifecycle callbacks (Awake, Start, Update, etc.)
4. Add `Time` management
5. Implement `Coroutine` system

### Phase 3: Events & Messaging (Week 5)

1. Implement `EventBus`
2. Add built-in events (Collision, Trigger, Mouse)
3. Implement `SendMessage`/`BroadcastMessage`
4. Add `EventHandler` trait

### Phase 4: ScriptableObject & Assets (Weeks 6-7)

1. Implement `ScriptableObject` trait
2. Create `AssetHandle` with reference counting
3. Implement `AssetDatabase`
4. Add `Resources` folder support
5. Implement async asset loading

### Phase 5: Editor Improvements (Weeks 8-10)

1. Update Hierarchy panel for new architecture
2. Update Inspector panel for component rendering
3. Update Scene View for new rendering
4. Implement Prefab system
5. Update Serialization system
6. Update Undo/Redo system

### Phase 6: Testing & Polish (Weeks 11-12)

1. Update all existing examples
2. Add new examples demonstrating Unity-like API
3. Update documentation
4. Performance testing and optimization
5. Bug fixes and polish

---

## Risks & Mitigations

### Risk 1: Breaking Changes
- **Mitigation:** Provide migration guide and tooling
- **Mitigation:** Keep old ECS as optional backend via feature flag `legacy-ecs`
- **Mitigation:** Create `engine-ecs-legacy` crate that wraps the old ECS

### Risk 2: Performance Regression
- **Mitigation:** Profile before and after
- **Mitigation:** Optimize hot paths
- **Mitigation:** Consider hybrid approach if needed

### Risk 3: Borrow Checker Issues
- **Mitigation:** Use interior mutability where needed
- **Mitigation:** Design APIs to minimize borrow conflicts
- **Mitigation:** Use raw pointers with safety guarantees

### Risk 4: Scope Creep
- **Mitigation:** Strict phase boundaries
- **Mitigation:** Regular check-ins and adjustments

### Risk 5: Unstable Rust Features (Generators)
- **Mitigation:** Use `std::ops::Generator` behind `#![feature(generators)]` or use async/await as primary approach
- **Mitigation:** Provide stable coroutine implementation using state machines
- **Mitigation:** Document unstable feature requirements clearly

---

## Success Criteria

1. **API Familiarity** — Unity developers can use RustEngine without reading extensive documentation
2. **Feature Parity** — All major Unity concepts are implemented
3. **Performance** — No significant performance regression
4. **Documentation** — Comprehensive docs and examples
5. **Testing** — All existing tests pass, new tests added

---

## References

- [Unity Manual](https://docs.unity.cn/cn/2022.2/Manual/index.html)
- [Unity Script API](https://docs.unity.cn/cn/2022.2/ScriptReference/index.html)
- [Bevy ECS](https://docs.rs/bevy/latest/bevy/ecs/)
- [Unity Architecture](https://docs.unity.cn/cn/2022.2/Manual/unity-architecture.html)
