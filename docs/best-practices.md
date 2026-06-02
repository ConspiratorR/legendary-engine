# Best Practices Guide

Recommended patterns and conventions for building games with RustEngine.

## Project Structure

Organize your game as a Cargo workspace:

```
my_game/
├── Cargo.toml          # workspace root
├── src/
│   └── main.rs         # entry point
├── crates/
│   ├── game-core/      # game-specific ECS systems and components
│   ├── game-assets/    # asset definitions and loaders
│   └── game-ui/        # UI screens and widgets
└── assets/             # game assets (textures, models, audio)
```

## ECS Patterns

### Component Design

Keep components as pure data. Logic goes in systems.

```rust
// Good: component is just data
struct Health {
    current: f32,
    max: f32,
}

// Bad: component has logic
impl Health {
    fn take_damage(&mut self, amount: f32) { // Don't do this
        self.current -= amount;
    }
}
```

Use systems for logic:

```rust
fn damage_system(world: &mut World) {
    let query = QueryPair::<Health, DamageEvent>::new();
    for (health, event) in query.iter_mut(world) {
        health.current = (health.current - event.amount).max(0.0);
    }
}
```

### System Ordering

Use the `Schedule` to control execution order:

```rust
let mut schedule = Schedule::new();

// These run in registration order
schedule.add_system(input_system);
schedule.add_system(movement_system);  // depends on input
schedule.add_system(physics_system);   // depends on movement
schedule.add_system(render_system);    // depends on physics
```

### Resource vs Component

- **Resource**: Global singleton state (time, input, config)
- **Component**: Per-entity state (position, health, velocity)

```rust
// Resource: there's only one game clock
struct GameClock {
    elapsed: f32,
}

// Component: each entity has its own position
struct Position(Vec3);
```

## Plugin Architecture

### Creating a Plugin

Group related functionality into plugins:

```rust
struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::new());
        app.add_system(physics_step_system);
        app.add_system(collision_response_system);
        app.add_system(sync_transforms_system);
    }
}
```

### Plugin Dependencies

Document and enforce dependencies:

```rust
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Ensure asset system is available
        assert!(
            app.resources().contains::<AssetManager>(),
            "AudioPlugin requires AssetPlugin"
        );
        app.add_system(audio_update_system);
    }
}
```

## Asset Management

### Loading Assets

Use the asset manager for all asset loading:

```rust
fn setup_system(world: &mut World) {
    let asset_mgr = world.get_resource::<AssetManager>().unwrap();
    
    // Load with handle (async, non-blocking)
    let texture_handle = asset_mgr.load::<Texture>("sprites/player.png");
    let mesh_handle = asset_mgr.load::<Mesh>("models/cube.gltf");
    
    // Spawn entity with loaded assets
    let entity = world.spawn();
    world.add_component(entity, Sprite { texture: texture_handle, .. });
}
```

### Asset Hot-Reload

The file watcher automatically reloads changed assets during development:

```rust
// No special code needed — just use AssetManager::load()
// Changed files are detected and reloaded automatically
```

## Rendering

### Sprite Batching

Sprites are automatically batched by texture. Minimize texture switches:

```rust
// Good: sprites sharing a texture batch together
for i in 0..100 {
    commands.spawn(Sprite {
        texture: sprite_sheet.clone(), // same texture
        uv_region: get_frame_region(i),
        ..Default::default()
    });
}

// Bad: each unique texture creates a separate batch
for i in 0..100 {
    commands.spawn(Sprite {
        texture: unique_textures[i].clone(), // different textures
        ..Default::default()
    });
}
```

### Camera Setup

Use the appropriate camera type for your game:

```rust
// 2D game
let camera = Camera::orthographic(
    -width / 2.0, width / 2.0,
    -height / 2.0, height / 2.0,
    0.0, 100.0,
);

// 3D game
let camera = Camera::perspective(
    std::f32::consts::FRAC_PI_4, // 45° FOV
    width / height,
    0.1,   // near
    1000.0, // far
);
```

## Physics

### Collision Layers

Use collision layers to control which objects interact:

```rust
let mut body = RigidBody::new_dynamic();
body.collision_group = CollisionGroup::PLAYER;
body.collision_mask = CollisionGroup::WALL | CollisionGroup::ENEMY;
```

### Fixed Timestep

Physics should run at a fixed rate independent of frame rate:

```rust
fn physics_system(world: &mut World) {
    let dt = 1.0 / 60.0; // Fixed 60Hz
    let physics = world.get_resource_mut::<PhysicsWorld>().unwrap();
    physics.step(dt);
}
```

## Game State Management

### State Stack

Use the framework's state stack for game flow:

```rust
struct GameplayState {
    score: i32,
}

impl GameState for GameplayState {
    fn on_enter(&mut self, ctx: &mut StateCtx) {
        // Initialize level
    }
    
    fn update(&mut self, ctx: &mut StateCtx, dt: f32) {
        self.score += 1;
        
        if self.is_game_over() {
            ctx.push(GameOverState { score: self.score });
        }
    }
    
    fn on_exit(&mut self, ctx: &mut StateCtx) {
        // Cleanup
    }
}
```

### Transitions

Use `GameStateAction` for standard transitions:

```rust
// Push pause menu (gameplay still in stack)
resources.insert(GameStateAction::PushPause);

// Pop back to gameplay
resources.insert(GameStateAction::Pop);

// Replace with new state
resources.insert(GameStateAction::StartGame);
```

## Error Handling

### System Errors

Systems should handle errors gracefully:

```rust
fn load_level_system(world: &mut World) {
    let result = load_level("levels/level1.json");
    match result {
        Ok(level) => { /* apply level */ }
        Err(e) => {
            log::error!("Failed to load level: {}", e);
            // Fall back to default level or show error screen
        }
    }
}
```

### Asset Errors

Missing assets should not crash the game:

```rust
fn sprite_system(world: &mut World) {
    let asset_mgr = world.get_resource::<AssetManager>().unwrap();
    
    for (entity, sprite) in world.query::<&Sprite>() {
        match asset_mgr.get(&sprite.texture) {
            Some(texture) => { /* render */ }
            None => { /* use placeholder or skip */ }
        }
    }
}
```

## Performance Tips

### Query Efficiency

Use specific queries to iterate only over relevant entities:

```rust
// Good: only iterates entities with both components
let query = QueryPair::<Position, Velocity>::new();
for (pos, vel) in query.iter_mut(world) {
    pos.0 += vel.0 * dt;
}

// Bad: iterates all entities, checks each one
for entity in world.entities() {
    if let (Some(pos), Some(vel)) = (
        world.get::<Position>(entity),
        world.get::<Velocity>(entity),
    ) {
        // slower
    }
}
```

### Avoid Allocations in Hot Paths

Pre-allocate buffers for systems that run every frame:

```rust
struct RenderState {
    sprite_buffer: Vec<SpriteDraw>, // reused each frame
}

fn collect_sprites(world: &mut World) {
    let state = world.get_resource_mut::<RenderState>().unwrap();
    state.sprite_buffer.clear(); // reuse allocation
    
    for (_, sprite) in world.query::<&Sprite>() {
        state.sprite_buffer.push(sprite.to_draw());
    }
}
```

### Use Profiler

Profile your game to find bottlenecks:

```rust
let mut profiler = Profiler::new(120);

profiler.start("physics");
// ... physics ...
profiler.end("physics");

profiler.start("render");
// ... rendering ...
profiler.end("render");

profiler.record_frame();
profiler.print_stats();
```

## Testing

### Unit Tests

Test ECS logic in isolation:

```rust
#[test]
fn test_damage_system() {
    let mut world = World::new();
    let entity = world.spawn();
    world.add_component(entity, Health { current: 100.0, max: 100.0 });
    world.add_component(entity, DamageEvent { amount: 25.0 });
    
    damage_system(&mut world);
    
    let health = world.get::<Health>(entity).unwrap();
    assert_eq!(health.current, 75.0);
}
```

### Integration Tests

Test system interactions:

```rust
#[test]
fn test_game_flow() {
    let mut app = AppBuilder::new();
    app.add_plugin(GamePlugin);
    
    // Simulate frames
    for _ in 0..10 {
        app.run();
    }
    
    let session = app.resources().get::<GameSession>().unwrap();
    assert!(session.is_running);
}
```
