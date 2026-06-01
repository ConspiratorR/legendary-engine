# ECS Usage Tutorial

The Entity Component System (ECS) is the foundation of RustEngine. This tutorial covers the core concepts.

## Core Concepts

### Entities

An `Entity` is a lightweight handle (index + generation) that identifies an object in the world.

```rust
use engine_ecs::world::World;

let mut world = World::new();
let entity = world.spawn(); // Create a new entity
world.despawn(entity);      // Destroy it
```

### Components

Components are plain data structs attached to entities. Any `'static` type can be a component.

```rust
struct Position { x: f32, y: f32 }
struct Velocity { x: f32, y: f32 }
struct Health(i32);

let entity = world.spawn();
world.add_component(entity, Position { x: 0.0, y: 0.0 });
world.add_component(entity, Velocity { x: 1.0, y: 0.5 });
world.add_component(entity, Health(100));
```

### Querying Components

Read or write components on specific entities:

```rust
// Read
if let Some(pos) = world.get::<Position>(entity) {
    println!("({}, {})", pos.x, pos.y);
}

// Write
if let Some(pos) = world.get_mut::<Position>(entity) {
    pos.x += 1.0;
}
```

### Queries

`Query<T>` iterates over all entities that have component `T`:

```rust
use engine_ecs::query::Query;

let query = Query::<Position>::new();
for pos in query.iter(&world) {
    println!("({}, {})", pos.x, pos.y);
}

// Mutable iteration
for pos in query.iter_mut(&mut world) {
    pos.x += 1.0;
}
```

`QueryPair<A, B>` iterates over entities that have **both** components:

```rust
use engine_ecs::query::QueryPair;

let query = QueryPair::<Position, Velocity>::new();
for (pos, vel) in query.iter(&world) {
    println!("pos=({}, {}), vel=({}, {})", pos.x, pos.y, vel.x, vel.y);
}
```

### Systems

Systems are functions that operate on the world:

```rust
use engine_ecs::system::IntoSystem;

fn movement_system(world: &mut World) {
    let query = QueryPair::<Position, Velocity>::new();
    for (pos, vel) in query.iter_mut(world) {
        pos.x += vel.x * 0.016; // ~60fps timestep
        pos.y += vel.y * 0.016;
    }
}

// Register the system
let mut schedule = engine_ecs::schedule::Schedule::new();
schedule.add_system(movement_system.system());
schedule.run(&mut world);
```

### Resources

Resources are global singletons stored on the world:

```rust
struct DeltaTime(f32);

world.insert_resource(DeltaTime(0.016));

if let Some(dt) = world.get_resource::<DeltaTime>() {
    println!("dt = {}", dt.0);
}
```

## Using the App Builder

The `AppBuilder` provides a convenient way to wire everything together:

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(DeltaTime(0.016));
        app.add_system(movement_system);
    }
}

let mut app = AppBuilder::new()
    .add_plugin(GamePlugin)
    .build();

loop {
    app.run();
}
```
