# Migration Guide

This guide helps developers migrating from other game engines to RustEngine.

## Unity → RustEngine

### Concepts Mapping

| Unity | RustEngine | Notes |
|-------|------------|-------|
| `GameObject` | `GameObject` | Named entity with components, tag, layer, active state |
| `Component` | `Component` trait | Any `'static` type with lifecycle callbacks |
| `MonoBehaviour` | `MonoBehaviour` trait | Scripts with `awake`, `start`, `update`, etc. |
| `Transform` | `Transform` | Local position/rotation/scale relative to parent |
| `Scene` | Scene JSON file | JSON serialization with `.meta` files |
| `Prefab` | `PrefabDef` | Reusable scene templates |
| `Inspector` | Inspector panel | Property editing for selected entity |
| `AssetDatabase` | `AssetStore` | Asset handles with `Arc` ref-counting |
| `Project Settings` | `EngineConfig` | Runtime configuration |
| `Player Loop` | `Schedule` | Ordered system execution |
| `ScriptableObject` | `ScriptableObject` trait | Serializable data assets with lifecycle |
| `.asset` + `.meta` GUID | `scriptable_asset` + `AssetDatabase` | Disk-backed ScriptableObjects |
| `AssetDatabase` | `AssetDatabase` | Name + GUID index, save/load/scan |
| `EventSystem` | `EventBus` | Type-safe event dispatch |
| `SendMessage` | `EventBus::send` | Type-safe alternative to string-based messaging |
| `Physics` | `PhysicsPlugin` | Rigid bodies, colliders, joints |
| `AudioSource` | `SpatialAudioSource` | 3D positional audio |
| `Canvas` | egui UI | Immediate mode UI |
| `RequireComponent` | `Component::required_on_add` | Auto-add missing dependencies |
| `Invoke` | `World::Invoke` | Delayed method dispatch |
| `StartCoroutine` / `WaitForSeconds` | `World::StartCoroutine` + `CoroutineStep::Wait` | Explicit step list |
| `WaitForSecondsRealtime` | `CoroutineStep::WaitRealtime` | Ignores `timeScale` |
| `WaitUntil` / `WaitWhile` | `CoroutineStep::WaitUntil` / `WaitWhile` | Predicate closures |
| `WaitForFixedUpdate` | `CoroutineStep::WaitFixedUpdate` | Resumes after a FixedUpdate this or next frame |
| `StopCoroutine(string)` | `World::StopCoroutineByName` | First matching name on owner |
| `StopAllCoroutines` | `World::StopAllCoroutines` | Stops all on the GameObject |
| Prefab Variant | `Prefab::CreateVariant` | Base + node overrides |
| `SceneManager` | `SceneManager` + `SceneRuntime` | See [lifecycle-and-scenes.md](lifecycle-and-scenes.md) |

### GameObject API

RustEngine's `GameObject` mirrors Unity's: a named entity with components, tag, layer, and parent-child hierarchy.

```rust
// Unity
GameObject obj = new GameObject("Player");
obj.tag = "Player";
obj.layer = LayerMask.NameToLayer("Characters");
obj.transform.SetParent(parentObj.transform);

// RustEngine
let mut go = GameObject::new("Player");
go.set_tag("Player");
go.set_layer(1);
go.set_active(true);
```

### Component API

Components implement the `Component` trait with lifecycle callbacks:

```rust
// Unity
public class Health : MonoBehaviour {
    public float currentHealth = 100f;
}

// RustEngine
use engine_core::gameobject::Component;
use std::any::Any;

#[derive(Debug)]
struct Health {
    current_health: f32,
}

impl Component for Health {
    fn on_added(&mut self, handle: GameObjectHandle) {
        // Called when added to a GameObject
    }

    fn on_removed(&mut self, handle: GameObjectHandle) {
        // Called when removed
    }

    fn on_enable(&mut self, handle: GameObjectHandle) {
        // Called when GameObject becomes active
    }

    fn on_disable(&mut self, handle: GameObjectHandle) {
        // Called when GameObject becomes inactive
    }

    fn on_destroy(&mut self, handle: GameObjectHandle) {
        // Called when GameObject is destroyed
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
```

### Transform Hierarchy

The `Transform` component provides local and world-space transforms:

```rust
// Unity
transform.localPosition = new Vector3(1, 0, 0);
transform.localScale = Vector3.one * 2f;
transform.LookAt(target.position);

// RustEngine
use engine_core::transform::{Transform, Space};
use engine_math::Vec3;

let mut transform = Transform::from_xyz(1.0, 0.0, 0.0);
transform.set_local_scale(Vec3::splat(2.0));
transform.look_at(target_position);

// Access world-space values
let world_pos = transform.position();        // world position
let world_rot = transform.rotation();        // world rotation
let forward = transform.forward();           // world forward direction

// Transform points between spaces
let world_point = transform.transform_point(local_point);
let local_point = transform.inverse_transform_point(world_point);

// Translate in world or local space
transform.translate(Vec3::X * 5.0, Space::World);
transform.translate(Vec3::Z * 2.0, Space::Self_);
```

### MonoBehaviour Lifecycle

Scripts implement `MonoBehaviour` to receive lifecycle callbacks:

```rust
// Unity
public class PlayerMovement : MonoBehaviour {
    public float speed = 5f;

    void Awake() { }
    void Start() { }
    void Update() { }
    void FixedUpdate() { }
    void LateUpdate() { }
    void OnDestroy() { }
    void OnEnable() { }
    void OnDisable() { }
}

// RustEngine
use engine_core::monobehaviour::MonoBehaviour;
use engine_core::gameobject::Component;
use engine_core::context::Context;

#[derive(Debug)]
struct PlayerMovement {
    speed: f32,
}

impl Component for PlayerMovement {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl MonoBehaviour for PlayerMovement {
    fn awake(&mut self, _context: &mut Context) {
        // Called when script instance is loaded
    }

    fn start(&mut self, _context: &mut Context) {
        // Called before first frame update
    }

    fn update(&mut self, context: &mut Context) {
        // Called once per frame
    }

    fn fixed_update(&mut self, context: &mut Context) {
        // Called at fixed intervals (physics)
    }

    fn late_update(&mut self, context: &mut Context) {
        // Called after all Update calls
    }

    fn on_enable(&mut self, _context: &mut Context) {
        // Called when enabled
    }

    fn on_disable(&mut self, _context: &mut Context) {
        // Called when disabled
    }

    fn on_destroy(&mut self, _context: &mut Context) {
        // Called when destroyed
    }
}
```

### Event System

RustEngine uses a type-safe `EventBus` instead of Unity's string-based `SendMessage`:

```rust
// Unity
// Define event
public class OnDamageEvent {
    public float amount;
}

// Send
gameObject.SendMessage("OnDamage", new OnDamageEvent { amount = 25f });

// Receive (string-based, error-prone)
void OnDamage(OnDamageEvent evt) { }

// RustEngine
use engine_core::event::{Event, EventBus, EventBusExt};

// Define event (type-safe)
#[derive(Clone)]
struct OnDamage {
    amount: f32,
}
impl Event for OnDamage {}

// Register handler
let mut bus = EventBus::new();
bus.on_event::<OnDamage>(|event, context| {
    println!("Damage: {}", event.amount);
});

// Send event
bus.send(OnDamage { amount: 25.0 }, &mut context);
```

Built-in events mirror Unity's:

| Unity Event | RustEngine Event |
|-------------|------------------|
| `OnCollisionEnter` | `CollisionEnter` |
| `OnCollisionExit` | `CollisionExit` |
| `OnTriggerEnter` | `TriggerEnter` |
| `OnTriggerExit` | `TriggerExit` |
| `OnMouseDown` | `MouseDown` |
| `OnMouseEnter` | `MouseEnter` |
| `HealthChanged` | `HealthChanged` |
| `EntityDied` | `EntityDied` |

### ScriptableObject System

Create serializable data assets with lifecycle callbacks:

```rust
// Unity
[CreateAssetMenu]
public class WeaponData : ScriptableObject {
    public string weaponName;
    public float damage;
    public float fireRate;
}

// RustEngine
use engine_core::scriptable_object::ScriptableObject;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct WeaponData {
    name: String,
    damage: f32,
    fire_rate: f32,
    #[serde(skip)]
    asset_path: Option<String>,
}

impl ScriptableObject for WeaponData {
    fn on_create(&mut self) {
        // Called when ScriptableObject is created
    }

    fn on_enable(&mut self) {
        // Called when enabled
    }

    fn on_destroy(&mut self) {
        // Called when destroyed
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn asset_path(&self) -> Option<&str> {
        self.asset_path.as_deref()
    }

    fn set_asset_path(&mut self, path: &str) {
        self.asset_path = Some(path.to_string());
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

### ECS Pattern (Alternative)

RustEngine also supports a pure ECS approach with `World` and systems:

```rust
// Bevy
fn movement_system(query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter() {
        transform.translation += velocity.0 * 0.016;
    }
}

// RustEngine ECS
fn movement_system(world: &mut World) {
    let dt = world.get_resource::<Time>().unwrap().delta_seconds();
    let query = QueryPair::<Transform, Velocity>::new();
    for (transform, vel) in query.iter_mut(world) {
        transform.position += vel.0 * dt;
    }
}
```

---

## Godot → RustEngine

### Concepts Mapping

| Godot | RustEngine | Notes |
|-------|------------|-------|
| `Node` | `GameObject` | Named entity with components |
| `Node2D` / `Node3D` | `Transform` component | Position, rotation, scale |
| `Sprite2D` | `Sprite` component | 2D sprite rendering |
| `MeshInstance3D` | `MeshRenderer` component | 3D mesh rendering |
| `GDScript` | `MonoBehaviour` trait | Script with lifecycle callbacks |
| `Scene` | Scene JSON file | JSON serialization |
| `Resource` | `ScriptableObject` trait | Serializable data assets |
| `Signal` | `EventBus` | Type-safe event dispatch |
| `PhysicsBody` | `RigidBody` component | Physics simulation |
| `Area` | Trigger collider | Collision detection |
| `AnimationPlayer` | `AnimationPlayer` component | Keyframe animation |
| `Control` | egui UI | Immediate mode UI |

### Code Examples

**Creating a Node (Godot vs RustEngine):**

```gdscript
# Godot
var node = Node2D.new()
node.position = Vector2(100, 200)
add_child(node)
```

```rust
// RustEngine
let mut go = GameObject::new("MyNode");
go.add_component(Transform::from_xyz(100.0, 200.0, 0.0));
```

**Signals vs Events (Godot vs RustEngine):**

```gdscript
# Godot
signal health_changed(new_health)
emit_signal("health_changed", health)
connect("health_changed", self, "_on_health_changed")
```

```rust
// RustEngine
#[derive(Clone)]
struct HealthChanged { entity: GameObjectHandle, new_health: f32 }
impl Event for HealthChanged {}

// Emit
bus.send(HealthChanged { entity: handle, new_health: 50.0 }, &mut ctx);

// Listen
bus.on_event::<HealthChanged>(|event, ctx| {
    println!("Health: {}", event.new_health);
});
```

---

## Bevy → RustEngine

### Concepts Mapping

| Bevy | RustEngine | Notes |
|------|------------|-------|
| `App` | `AppBuilder` | Application builder |
| `Plugin` | `Plugin` trait | Same concept |
| `Entity` | `GameObjectHandle` | Entity reference |
| `Component` | `Component` trait | Data + lifecycle callbacks |
| `System` | System function | Same concept |
| `Query` | `Query<T>` | Component queries |
| `Resource` | Resource | Global singleton data |
| `Schedule` | `Schedule` | Ordered system execution |
| `World` | `World` | ECS world |
| `Event` | `Event` trait | Type-safe events |

### Code Examples

**Plugin (Bevy vs RustEngine):**

```rust
// Bevy
struct MyPlugin;
impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, movement_system);
    }
}
```

```rust
// RustEngine
struct MyPlugin;
impl Plugin for MyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(movement_system);
    }
}
```

---

## Key Differences

### Rust vs C#/GDScript

1. **Ownership**: Rust's ownership system prevents data races at compile time
2. **Borrowing**: Use `&` for shared access, `&mut` for exclusive access
3. **Lifetimes**: Most game data is `'static`, so lifetimes are rarely needed
4. **Error handling**: Use `Result<T, E>` and `?` operator instead of exceptions
5. **No null**: Use `Option<T>` instead of null pointers

### GameObject vs ECS-Only

RustEngine supports two paradigms:

- **GameObject + Component** (Unity-like): Components are attached to GameObjects with lifecycle callbacks. Best for gameplay scripts that need `update`, `on_collision_enter`, etc.
- **Pure ECS** (Bevy-like): Components are attached to entities via `World`. Best for data-oriented systems and high-performance iteration.

Choose based on your use case. Both can coexist in the same project.

### Immediate Mode UI

RustEngine uses **egui** for UI, which is immediate mode:

- **No retained UI tree**: UI is redrawn every frame
- **Simple API**: `ui.button("Click me")` returns true if clicked
- **No CSS**: Styling is done programmatically
- **No layout engine**: Manual positioning or simple layouts

---

## P2.4 Storage Migration (`unity-world-primary`)

`engine-core` has an optional cargo feature `unity-world-primary` (default **off**). When enabled, Unity `World` APIs dual-read from the internal ECS and write mutations through to ECS components. Array storage remains authoritative for hierarchy math and scene I/O until the full storage merge (see `docs/unity-alignment-roadmap.md`).

### Feature flag

```bash
cargo test -p engine-core --lib --features unity-world-primary
cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary
```

CI runs a dedicated `Unity World Primary` job covering both flag states.

### Dual-read contract (feature **on**)

| API | Prefers | Fallback |
|-----|---------|----------|
| `GetTransform` | ECS `Transform` | array `GetTransformArray` |
| `GetName` / `GetTag` / `IsActive` | ECS name/tag/active | array |
| `GetParent` / `GetChildren` | ECS `GameObjectParent` / `GameObjectChildren` | array `GetParentArray` / `GetChildrenArray` |

Always use the **array** accessors for hierarchy math, world-pose composition, and scene serialization:

- `GetTransformArray` — local + cached world after `sync_transforms`
- `GetParentArray` / `GetChildrenArray` — authoritative parent/child links

Do **not** use dual-read `GetTransform` to compute world coordinates or walk parents.

### Write-through contract

Prefer mutating transforms with:

```rust
// Array-primary write (legacy / scene I/O) — mirrors ECS when feature on
world.with_transform_mut(handle, |t| {
    t.SetLocalPosition(pos);
});

// ECS-primary write under unity-world-primary — array becomes cache
world.with_ecs_transform_mut(handle, |t| {
    t.SetLocalPosition(pos);
});
```

`with_ecs_transform_mut` falls back to `with_transform_mut` when the feature is off. After ECS-only edits, call `sync_transform_from_ecs` / `sync_all_transforms_from_ecs` to refresh array cache (hierarchy math + scene save still use array pose fields).

`GetTransformMut` only mutates the array. After using it, call `sync_transform_to_ecs` / `sync_transforms` or dual-read will see a stale ECS `Transform`.

Other write paths that already go through ECS:

- `CreateGameObject` — Name/Tag/Active/Transform/Children seeded on the linked entity
- `SetParent` — hierarchy components rewritten on child, new parent, and old parent
- `AddMonoBehaviour` / `RestoreMonoBehaviours` — `MonoBehaviourTypes` + `MonoBehaviourInstances` (type_name, enabled, props)
- `restore_monobehaviours_from_ecs` — rebuild array holders from ECS metadata + global registry
- `seed_ecs_from_array` / `seed_all_ecs_from_array` — fill all ECS identity mirrors from array/GameObject fields (scene load, tools)
- `Destroy` / `DestroyImmediate` / `flush_destroy` — ECS entity despawned; pending Destroy and DontDestroyOnLoad entries dropped

### Dual-read contract (feature on)

When `unity-world-primary` is enabled, public Unity World **read** APIs prefer the linked internal-ECS mirror if present:

| Read API | Preferred component | Fallback |
|----------|---------------------|----------|
| `GetTransform` | ECS `Transform` | array (`GetTransformArray`) |
| `GetName` | ECS `GameObjectName` | `gameobject_data` |
| `GetTag` | ECS `GameObjectTag` | `gameobject_data` |
| `IsActive` | ECS `GameObjectActive` | `gameobject_data` |
| `GetLayer` | ECS `GameObjectLayer` | `gameobject_data` |
| `GetParent` | ECS `GameObjectParent` | `GetParentArray` |
| `GetChildren` | ECS `GameObjectChildren` | `GetChildrenArray` |

When the feature is **off** (workspace default), the same public APIs always read **array / GameObject** storage even if ECS mirrors exist. `GetTransformArray` / `GetParentArray` / `GetChildrenArray` stay array-authoritative in both modes.

Phase 11 note: dual-read preference is implemented; full write-authority migration (arrays demoted to cache only when the feature is on) is still in progress under the phase plan.

### Array-authoritative APIs

| API | Use for |
|-----|---------|
| `GetTransformArray` | Scene save/load, hierarchy world math |
| `GetParentArray` / `GetChildrenArray` | Cycle detection, `sync_transforms`, destroy child walk |
| `ensure_transform_from_array` | Backfill missing ECS `Transform` from array |

#### Experimental default-on builds (D1)

`unity-world-primary` stays **off** in workspace `Cargo.toml`. For a local trial build that exercises dual-read/write-through:

```toml
# engine-editor or a game crate Cargo.toml — only for experiment binaries
[dependencies]
engine-core = { path = "../engine-core", features = ["unity-world-primary"] }
```

Or from the CLI without editing Cargo.toml:

```bash
cargo test -p engine-core --features unity-world-primary
cargo run -p engine-core --example unity_gameplay_demo --features unity-world-primary
```

Do **not** flip `default = ["audio"]` / remove it in favor of this flag until full storage authority migration lands.

## Still deferred

- Full authority move of `gameobject_data` / `transforms` / `monobehaviours` into ECS (holders still array-backed) — dual-read preference is done; write-authority migration continues in phase 11
- Enabling the flag by default
- engine-scene `Transform` deprecation (see roadmap P2.5)

`engine_core::TransformProxy` is re-exported from `engine_render::proxy::TransformProxy` so render systems can consume the same component type without a core→render cycle.

---

## Next Steps

- [Quick Start](quick-start.md) — Get started with RustEngine
- [ECS Tutorial](ecs-tutorial.md) — Learn the ECS pattern
- [Rendering Pipeline](rendering-pipeline.md) — Set up rendering
- [Physics System](physics-system.md) — Add physics
- [Audio System](audio-system.md) — Add audio
- [Editor Guide](editor-guide.md) — Use the editor
