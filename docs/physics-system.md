# Physics System Usage

RustEngine includes a built-in physics engine for rigid body dynamics and collision detection.

## Features

- **Rigid Bodies** — Dynamic, static, and kinematic body types
- **Colliders** — Sphere, cuboid, capsule, cylinder shapes
- **Collision Detection** — SAT-based collision for all shape combinations
- **Contact Solving** — Baumgarte stabilization, warm starting, Coulomb friction
- **Joints** — Hinge, ball-socket, spring constraints
- **CCD** — Continuous collision detection for fast-moving objects
- **ECS Integration** — Physics components and systems via the plugin system

## Setup

Add the physics plugin to your app:

```rust
use engine_physics::PhysicsPlugin;

let mut app = AppBuilder::new();
app.add_plugin(PhysicsPlugin);
```

## Rigid Bodies

Create rigid bodies with different types:

```rust
use engine_physics::{RigidBody, BodyType};

// Dynamic body (affected by forces and gravity)
let dynamic = RigidBody::new_dynamic();

// Static body (immovable)
let static_body = RigidBody::new_static();

// Kinematic body (moved by code, not physics)
let kinematic = RigidBody::new_kinematic();
```

Apply forces and impulses:

```rust
let mut body = RigidBody::new_dynamic();
body.apply_force(Vec3::new(0.0, -9.81, 0.0)); // gravity
body.apply_impulse(Vec3::new(10.0, 0.0, 0.0)); // jump
body.set_linear_velocity(Vec3::new(5.0, 0.0, 0.0));
```

## Colliders

Define collision shapes:

```rust
use engine_physics::Collider;

let sphere = Collider::sphere(0.5);      // radius
let cube = Collider::cuboid(1.0, 1.0, 1.0); // half-extents
let capsule = Collider::capsule(0.5, 1.0);  // radius, height
```

## Collision Detection

Check for collisions between shapes:

```rust
use engine_physics::{check_collision, Collider};

let a = Collider::sphere(1.0);
let b = Collider::sphere(1.0);

if let Some(info) = check_collision(
    &a, &Vec3::ZERO, &Quat::IDENTITY,
    &b, &Vec3::new(1.5, 0.0, 0.0), &Quat::IDENTITY,
) {
    println!("Collision depth: {}", info.depth);
    println!("Normal: {:?}", info.normal);
}
```

## Physics World

The `PhysicsWorld` manages the simulation:

```rust
use engine_physics::PhysicsWorld;

let mut physics = PhysicsWorld::new();
physics.set_gravity(Vec3::new(0.0, -9.81, 0.0));

// Step the simulation
physics.step(1.0 / 60.0);
```

## Joints

Connect bodies with joints:

```rust
use engine_physics::joint::{Joint, JointSolver};

let mut solver = JointSolver::new();

// Ball-and-socket joint
let ball = Joint::ball_socket(body_a, body_b, anchor_point);

// Hinge joint (door-like)
let hinge = Joint::hinge(body_a, body_b, anchor, axis);

// Spring joint
let spring = Joint::spring(body_a, body_b, rest_length, stiffness, damping);

solver.add_joint(ball);
```

## Continuous Collision Detection (CCD)

Prevent fast-moving objects from tunneling through thin walls:

```rust
use engine_physics::ccd::{sweep_sphere_sphere, CcdBody};

let result = sweep_sphere_sphere(
    &start_pos, &end_pos, radius_a,
    &target_pos, radius_b,
    max_time,
);

if result.hit {
    println!("Hit at time {}", result.time);
}
```
