# Migration Guide

This guide helps developers migrating from other game engines to RustEngine.

## Unity → RustEngine

### Concepts Mapping

| Unity | RustEngine | Notes |
|-------|------------|-------|
| `GameObject` | `Entity` | Lightweight handle (index + generation) |
| `Component` | Component struct | Any `'static` type can be a component |
| `MonoBehaviour` | System function | Functions that operate on the world |
| `Transform` | `Transform` + `GlobalTransform` | Separate local and global transforms |
| `Scene` | Scene JSON file | JSON serialization with `.meta` files |
| `Prefab` | `PrefabDef` | Reusable scene templates |
| `Inspector` | Inspector panel | Property editing for selected entity |
| `AssetDatabase` | `AssetStore` | Asset handles with `Arc` ref-counting |
| `Project Settings` | `EngineConfig` | Runtime configuration |
| `Player Loop` | `Schedule` | Ordered system execution |
| `ScriptableObject` | Resource | Global singleton data |
| `Coroutine` | ECS system + timer | No coroutines, use systems with state |
| `Physics` | `PhysicsPlugin` | Rigid bodies, colliders, joints |
| `AudioSource` | `SpatialAudioSource` | 3D positional audio |
| `Canvas` | egui UI | Immediate mode UI |

### Code Examples

**Creating an Entity (Unity vs RustEngine):**

```csharp
// Unity
GameObject obj = new GameObject("Player");
obj.AddComponent<Transform>();
obj.AddComponent<Rigidbody>();
```

```rust
// RustEngine
let entity = world.spawn();
world.add_component(entity, Transform::default());
world.add_component(entity, RigidBody::new_dynamic());
```

**Movement System (Unity vs RustEngine):**

```csharp
// Unity (MonoBehaviour)
void Update() {
    transform.position += velocity * Time.deltaTime;
}
```

```rust
// RustEngine (System)
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
| `Node` | Scene node | Hierarchical scene tree |
| `Node2D` / `Node3D` | `Transform` component | Position, rotation, scale |
| `Sprite2D` | `Sprite` component | 2D sprite rendering |
| `MeshInstance3D` | `MeshRenderer` component | 3D mesh rendering |
| `GDScript` | Lua/WASM scripting | Script systems |
| `Scene` | Scene JSON file | JSON serialization |
| `Resource` | Asset handle | `Arc` ref-counting |
| `Signal` | Event channel | Type-safe event system |
| `PhysicsBody` | `RigidBody` component | Physics simulation |
| `Area` | Trigger collider | Collision detection |
| `AnimationPlayer` | `AnimationPlayer` component | Keyframe animation |
| `Control` | egui UI | Immediate mode UI |
| `TileMap` | `Tilemap` component | 2D tile-based maps |

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
let entity = world.spawn();
world.add_component(entity, Transform {
    position: Vec3::new(100.0, 200.0, 0.0),
    ..Default::default()
});
```

**Movement (Godot vs RustEngine):**

```gdscript
# Godot (GDScript)
func _process(delta):
    position += velocity * delta
```

```rust
// RustEngine (System)
fn movement_system(world: &mut World) {
    let dt = world.get_resource::<Time>().unwrap().delta_seconds();
    let query = QueryPair::<Transform, Velocity>::new();
    for (transform, vel) in query.iter_mut(world) {
        transform.position += vel.0 * dt;
    }
}
```

---

## Bevy → RustEngine

### Concepts Mapping

| Bevy | RustEngine | Notes |
|------|------------|-------|
| `App` | `AppBuilder` | Application builder |
| `Plugin` | `Plugin` trait | Same concept |
| `Entity` | `Entity` | Same concept |
| `Component` | Component struct | Same concept |
| `System` | System function | Same concept |
| `Query` | `Query<T>` | Component queries |
| `Resource` | Resource | Global singleton data |
| `Schedule` | `Schedule` | Ordered system execution |
| `World` | `World` | ECS world |
| `Bundle` | Multiple `add_component` | No bundle abstraction |
| `Event` | Event channel | Type-safe events |
| `State` | `GameState` | Game state machine |
| `Asset` | Asset handle | `Arc` ref-counting |

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

**Query (Bevy vs RustEngine):**

```rust
// Bevy
fn movement_system(query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter() {
        transform.translation += velocity.0 * 0.016;
    }
}
```

```rust
// RustEngine
fn movement_system(world: &mut World) {
    let query = QueryPair::<Transform, Velocity>::new();
    for (transform, vel) in query.iter_mut(world) {
        transform.position += vel.0 * 0.016;
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

### ECS Pattern

RustEngine uses an **archetypal ECS** with sparse-set storage:

- **No inheritance**: Components are data-only structs
- **No methods on components**: Systems operate on components
- **Composition over inheritance**: Combine components to create behaviors
- **Data-oriented design**: Optimize for cache-friendly iteration

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
