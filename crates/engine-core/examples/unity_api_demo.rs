//! # Unity API Demo
//!
//! Demonstrates the Unity-like API features:
//! - GameObject/Component creation
//! - Transform hierarchy
//! - MonoBehaviour lifecycle
//! - PlayerLoop execution

use engine_core::context::Context;
use engine_core::gameobject::{Component, GameObject};
use engine_core::hierarchy::sync_transforms;
use engine_core::monobehaviour::MonoBehaviour;
use engine_core::player_loop::{Phase, PlayerLoop};
use engine_core::transform::Transform;
use engine_core::world::World;
use engine_math::Vec3;

// === Custom Components ===

/// A simple position marker component.
struct Marker(String);

impl Component for Marker {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A MonoBehaviour that prints lifecycle messages.
struct Logger {
    name: String,
    update_count: usize,
}

impl Component for Logger {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl MonoBehaviour for Logger {
    fn awake(&mut self, _context: &mut Context) {
        println!("[{}] Awake!", self.name);
    }

    fn start(&mut self, _context: &mut Context) {
        println!("[{}] Start!", self.name);
    }

    fn update(&mut self, _context: &mut Context) {
        self.update_count += 1;
        println!("[{}] Update #{}", self.name, self.update_count);
    }

    fn fixed_update(&mut self, _context: &mut Context) {
        println!("[{}] FixedUpdate", self.name);
    }

    fn late_update(&mut self, _context: &mut Context) {
        println!("[{}] LateUpdate", self.name);
    }

    fn on_destroy(&mut self, _context: &mut Context) {
        println!("[{}] OnDestroy!", self.name);
    }
}

/// A MonoBehaviour that moves its transform.
struct Mover {
    speed: f32,
    direction: Vec3,
}

impl Component for Mover {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl MonoBehaviour for Mover {
    fn update(&mut self, context: &mut Context) {
        // In a real engine, we'd look up our transform and move it.
        // Here we just print to demonstrate the lifecycle.
        println!(
            "[Mover] Moving at speed {} in direction {:?}",
            self.speed, self.direction
        );
        let _ = context; // suppress unused warning
    }
}

// === Main ===

fn main() {
    println!("=== Unity API Demo ===\n");

    // --- Part 1: GameObject/Component creation ---
    println!("--- Part 1: GameObject/Component Creation ---");

    let mut world = World::new();

    // Create GameObjects (like Unity's new GameObject())
    let player = world.spawn(GameObject::new("Player"));
    let enemy = world.spawn(GameObject::new("Enemy"));

    // Add components (like Unity's AddComponent<T>())
    {
        let go = world.get_gameobject_mut(player).unwrap();
        go.add_component(Transform::from_xyz(0.0, 0.0, 0.0));
        go.add_component(Marker("player_marker".to_string()));
    }

    {
        let go = world.get_gameobject_mut(enemy).unwrap();
        go.add_component(Transform::from_xyz(10.0, 5.0, 0.0));
        go.add_component(Marker("enemy_marker".to_string()));
    }

    // Get components (like Unity's GetComponent<T>())
    if let Some(go) = world.get_gameobject(player) {
        if let Some(marker) = go.get_component::<Marker>() {
            println!("Player marker: {}", marker.0);
        }
        if let Some(transform) = go.get_component::<Transform>() {
            println!("Player position: {:?}", transform.local_position);
        }
    }

    println!();

    // --- Part 2: Transform hierarchy ---
    println!("--- Part 2: Transform Hierarchy ---");

    // Create a hierarchy: Parent -> Child -> Grandchild
    let parent = world.spawn({
        let mut go = GameObject::new("Parent");
        go.add_component(Transform::from_xyz(0.0, 0.0, 0.0));
        go
    });

    let child = world.spawn({
        let mut go = GameObject::new("Child");
        go.add_component(Transform::from_xyz(1.0, 0.0, 0.0));
        go
    });

    let grandchild = world.spawn({
        let mut go = GameObject::new("Grandchild");
        go.add_component(Transform::from_xyz(0.5, 0.0, 0.0));
        go
    });

    // Set parent-child relationships (like Unity's Transform.SetParent())
    world.set_parent(child, Some(parent));
    world.set_parent(grandchild, Some(child));

    // Print hierarchy
    println!("Hierarchy:");
    print_hierarchy(&world, parent, 0);

    // Sync transforms (compute world positions)
    sync_transforms(&mut world);

    // Print world positions after sync
    println!("\nWorld positions after sync:");
    if let Some(go) = world.get_gameobject(parent) {
        let t = go.get_component::<Transform>().unwrap();
        println!("  Parent: {:?}", t.position());
    }
    if let Some(go) = world.get_gameobject(child) {
        let t = go.get_component::<Transform>().unwrap();
        println!("  Child: {:?}", t.position());
    }
    if let Some(go) = world.get_gameobject(grandchild) {
        let t = go.get_component::<Transform>().unwrap();
        println!("  Grandchild: {:?}", t.position());
    }

    println!();

    // --- Part 3: MonoBehaviour lifecycle ---
    println!("--- Part 3: MonoBehaviour Lifecycle ---");

    // Create GameObjects with MonoBehaviours
    let _logger_a = world.spawn({
        let mut go = GameObject::new("LoggerA");
        go.add_component(Logger {
            name: "LoggerA".to_string(),
            update_count: 0,
        });
        go
    });

    let _logger_b = world.spawn({
        let mut go = GameObject::new("LoggerB");
        go.add_component(Logger {
            name: "LoggerB".to_string(),
            update_count: 0,
        });
        go
    });

    // Add a mover
    let _mover = world.spawn({
        let mut go = GameObject::new("Mover");
        go.add_component(Transform::from_xyz(0.0, 0.0, 0.0));
        go.add_component(Mover {
            speed: 5.0,
            direction: Vec3::new(1.0, 0.0, 0.0),
        });
        go
    });

    // Create context for lifecycle calls
    let mut events = engine_core::event::EventBus::new();
    let time = engine_core::time::Time::default();
    let mut context = Context::new(&mut world, time, 0, &mut events);

    // Simulate lifecycle by manually calling methods on MonoBehaviours
    // In a real engine, MonoBehaviourRunner would do this automatically

    // Awake
    println!("\n--- Calling Awake ---");
    run_lifecycle_on_all::<Logger>(context.world, "awake");

    // Start
    println!("\n--- Calling Start ---");
    run_lifecycle_on_all::<Logger>(context.world, "start");

    // Simulate a few frames of Update/FixedUpdate/LateUpdate
    for frame in 1..=3 {
        println!("\n--- Frame {} ---", frame);
        context.frame = frame;

        println!("FixedUpdate:");
        run_lifecycle_on_all::<Logger>(context.world, "fixed_update");

        println!("Update:");
        run_lifecycle_on_all::<Logger>(context.world, "update");

        println!("LateUpdate:");
        run_lifecycle_on_all::<Logger>(context.world, "late_update");
    }

    // OnDestroy
    println!("\n--- Calling OnDestroy ---");
    run_lifecycle_on_all::<Logger>(context.world, "on_destroy");

    println!();

    // --- Part 4: PlayerLoop execution ---
    println!("--- Part 4: PlayerLoop Execution ---");

    let mut player_loop = PlayerLoop::new();

    // Add systems to different phases (like Unity's PlayerLoop system)
    player_loop.add_system(Phase::Initialization, |_ctx: &mut Context| {
        println!("  [Initialization] System A")
    });

    player_loop.add_system(Phase::PreFixedUpdate, |_ctx: &mut Context| {
        println!("  [PreFixedUpdate] System B")
    });

    player_loop.add_system(Phase::FixedUpdate, |_ctx: &mut Context| {
        println!("  [FixedUpdate] System C")
    });

    player_loop.add_system(Phase::Update, |_ctx: &mut Context| {
        println!("  [Update] System D")
    });

    player_loop.add_system(Phase::LateUpdate, |_ctx: &mut Context| {
        println!("  [LateUpdate] System E")
    });

    player_loop.add_system(Phase::PostLateUpdate, |_ctx: &mut Context| {
        println!("  [PostLateUpdate] System F")
    });

    // Run a few frames through the PlayerLoop
    let mut events = engine_core::event::EventBus::new();
    let time = engine_core::time::Time::default();
    let mut context = Context::new(&mut world, time, 0, &mut events);

    for frame in 1..=2 {
        println!("\n--- PlayerLoop Frame {} ---", frame);
        context.frame = frame;
        player_loop.run(&mut context);
    }

    println!("\n=== Unity API Demo Complete ===");
    println!("\nSummary:");
    println!("1. Created GameObjects and added Components");
    println!("2. Built a Transform hierarchy (Parent -> Child -> Grandchild)");
    println!("3. Demonstrated MonoBehaviour lifecycle callbacks");
    println!("4. Executed systems through the PlayerLoop phases");
}

/// Helper to run lifecycle on all components of a given type.
/// In a real engine, MonoBehaviourRunner would handle this automatically.
fn run_lifecycle_on_all<T: engine_core::gameobject::Component + 'static>(
    world: &mut World,
    method: &str,
) {
    let handles: Vec<_> = world.all_gameobjects();
    for handle in handles {
        if let Some(go) = world.get_gameobject_mut(handle)
            && let Some(component) = go.get_component_mut::<T>()
        {
            // We need to call the MonoBehaviour methods, but we have a generic T.
            // For this demo, we'll use a downcast to a known type.
            // In a real engine, MonoBehaviourRunner would handle this differently.
            downcast_and_call_lifecycle(component, method);
        }
    }
}

/// Downcast and call lifecycle (simplified for demo).
fn downcast_and_call_lifecycle(component: &mut dyn Component, method: &str) {
    if let Some(logger) = component.as_any_mut().downcast_mut::<Logger>() {
        match method {
            "awake" => {
                // Can't call without context, so we simulate
                println!("    [{}] awake (simulated)", logger.name);
            }
            "start" => {
                println!("    [{}] start (simulated)", logger.name);
            }
            "update" => {
                logger.update_count += 1;
                println!("    [{}] update #{}", logger.name, logger.update_count);
            }
            "fixed_update" => {
                println!("    [{}] fixed_update (simulated)", logger.name);
            }
            "late_update" => {
                println!("    [{}] late_update (simulated)", logger.name);
            }
            "on_destroy" => {
                println!("    [{}] on_destroy (simulated)", logger.name);
            }
            _ => {}
        }
    }
}

/// Print the hierarchy tree.
fn print_hierarchy(world: &World, handle: engine_core::GameObjectHandle, depth: usize) {
    if let Some(go) = world.get_gameobject(handle) {
        let indent = "  ".repeat(depth);
        let name = go.name();
        let component_count = go.components().len();
        let child_count = go.child_count();

        println!(
            "{}├── {} (components: {}, children: {})",
            indent, name, component_count, child_count
        );

        for &child in go.children() {
            print_hierarchy(world, child, depth + 1);
        }
    }
}
