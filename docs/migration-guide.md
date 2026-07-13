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
| `EventSystem` | `EventBus` | Type-safe event dispatch |
| `SendMessage` | `EventBus::send` | Type-safe alternative to string-based messaging |
| `Physics` | `PhysicsPlugin` | Rigid bodies, colliders, joints |
| `AudioSource` | `SpatialAudioSource` | 3D positional audio |
| `Canvas` | egui UI | Immediate mode UI |

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

## Next Steps

- [Quick Start](quick-start.md) — Get started with RustEngine
- [ECS Tutorial](ecs-tutorial.md) — Learn the ECS pattern
- [Rendering Pipeline](rendering-pipeline.md) — Set up rendering
- [Physics System](physics-system.md) — Add physics
- [Audio System](audio-system.md) — Add audio
- [Editor Guide](editor-guide.md) — Use the editor
