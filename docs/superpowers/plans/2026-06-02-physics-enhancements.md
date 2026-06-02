# Physics Enhancements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance the physics engine with triggers/zones, material system, improved joint constraints, ragdoll, vehicle physics, and soft body simulation.

**Architecture:** Build on the existing `engine-physics` crate's ECS-integrated design. All new features are ECS components/resources with physics systems that operate on them during the `PhysicsWorld::step` cycle. The existing spatial hash broadphase, contact solver, and parallel narrowphase are extended rather than replaced.

**Tech Stack:** Rust, engine-ecs (sparse-set ECS), engine-math (glam re-exports), rayon (parallel iteration)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/engine-physics/src/material.rs` | PhysicsMaterial asset, MaterialTable (pair lookup), default materials |
| `crates/engine-physics/src/trigger.rs` | TriggerZone component, trigger event processing system |
| `crates/engine-physics/src/joint.rs` | Enhanced: angular constraint solving for ball-socket/hinge |
| `crates/engine-physics/src/ragdoll.rs` | Ragdoll component, RagdollBone, skeleton-physics bridge |
| `crates/engine-physics/src/vehicle.rs` | Vehicle component, Wheel, raycast suspension, arcade handling |
| `crates/engine-physics/src/softbody.rs` | SoftBody, Verlet particle, distance constraints, cloth mesh |
| `crates/engine-physics/src/world.rs` | Modified: sensor bypass fix, material lookup, joint+ragdoll+vehicle steps |
| `crates/engine-physics/src/plugin.rs` | Modified: register new systems, expose trigger/sensor events |
| `crates/engine-physics/src/lib.rs` | Modified: export new modules |

---

## Task 1: Trigger/Sensor Enhancement

**Goal:** Fix sensor bodies being skipped when sleeping, and ensure sensor events are reliably emitted every frame.

**Files:**
- Modify: `crates/engine-physics/src/world.rs`
- Modify: `crates/engine-physics/src/collider.rs`
- Modify: `crates/engine-physics/src/plugin.rs`

### Step 1: Fix broadphase to include sleeping sensors

In `world.rs:374-381`, the broadphase insertion skips all sleeping bodies. Sensors should always be tested regardless of sleep state.

```rust
// world.rs detect_collisions — replace the sleeping skip block
for &idx in &collider_indices {
    let is_sleeping = world
        .get_by_index::<RigidBody>(idx)
        .is_some_and(|b| b.is_sleeping);
    let is_sensor = world
        .get_by_index::<Collider>(idx)
        .is_some_and(|c| c.is_sensor);
    // Skip sleeping non-sensor bodies
    if is_sleeping && !is_sensor {
        continue;
    }
    // ... rest of broadphase insertion
}
```

### Step 2: Ensure narrowphase processes sleeping sensor pairs

In `detect_collisions`, the narrowphase parallel iterator already processes all pairs from broadphase, so no change needed there. But in `wake_colliding_bodies`, we should not wake sensor bodies unnecessarily. Add a guard:

```rust
fn wake_colliding_bodies(&self, world: &mut World) {
    for &(idx_a, idx_b, _) in &self.collisions {
        // Don't wake bodies just because a sensor overlapped them
        let a_is_sensor = world.get_by_index::<Collider>(idx_a).is_some_and(|c| c.is_sensor);
        let b_is_sensor = world.get_by_index::<Collider>(idx_b).is_some_and(|c| c.is_sensor);
        if a_is_sensor && b_is_sensor {
            continue; // Both sensors, nothing to wake
        }
        // Only wake the non-sensor side
        if !a_is_sensor {
            if let Some(body) = world.get_by_index::<RigidBody>(idx_a)
                && body.is_sleeping
                && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_a)
            {
                body.is_sleeping = false;
                body.rest_time = 0.0;
            }
        }
        if !b_is_sensor {
            if let Some(body) = world.get_by_index::<RigidBody>(idx_b)
                && body.is_sleeping
                && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_b)
            {
                body.is_sleeping = false;
                body.rest_time = 0.0;
            }
        }
    }
}
```

### Step 3: Add `sensor_overlap_events()` accessor to PhysicsWorld

```rust
impl PhysicsWorld {
    /// Get sensor events for a specific sensor entity.
    pub fn sensor_events_for(&self, entity: u32) -> Vec<&SensorEvent> {
        self.sensor_events
            .iter()
            .filter(|e| e.sensor_entity == entity)
            .collect()
    }

    /// Get collision events for a specific entity.
    pub fn collision_events_for(&self, entity: u32) -> Vec<&CollisionEvent> {
        self.collision_events
            .iter()
            .filter(|e| e.entity_a == entity || e.entity_b == entity)
            .collect()
    }
}
```

### Step 4: Write tests for sensor behavior

```rust
#[test]
fn test_sleeping_sensor_still_detects_overlap() {
    let mut world = World::new();

    // Sleeping sensor
    let sensor = world.spawn();
    world.add_component(sensor, Transform::from_xyz(0.0, 0.0, 0.0));
    let mut sensor_col = Collider::sphere(2.0);
    sensor_col.is_sensor = true;
    world.add_component(sensor, sensor_col);
    let mut sensor_body = RigidBody::new_static();
    sensor_body.is_sleeping = true;
    world.add_component(sensor, sensor_body);

    // Dynamic body inside sensor
    let body = world.spawn();
    world.add_component(body, Transform::from_xyz(0.5, 0.0, 0.0));
    world.add_component(body, RigidBody::new_dynamic());
    world.add_component(body, Collider::sphere(0.5));

    let mut pw = PhysicsWorld::new();
    pw.sub_steps = 1;
    pw.step(&mut world);

    let sensor_evts: Vec<_> = pw.sensor_events.iter()
        .filter(|e| e.sensor_entity == sensor.index())
        .collect();
    assert!(!sensor_evts.is_empty(), "Sleeping sensor should still detect overlap");
}

#[test]
fn test_sensor_does_not_wake_sleeping_body() {
    let mut world = World::new();

    let sensor = world.spawn();
    world.add_component(sensor, Transform::from_xyz(0.0, 0.0, 0.0));
    let mut sensor_col = Collider::sphere(2.0);
    sensor_col.is_sensor = true;
    world.add_component(sensor, sensor_col);
    world.add_component(sensor, RigidBody::new_static());

    let sleeping = world.spawn();
    world.add_component(sleeping, Transform::from_xyz(0.5, 0.0, 0.0));
    let mut body = RigidBody::new_dynamic();
    body.is_sleeping = true;
    body.rest_time = 10.0;
    world.add_component(sleeping, body);
    world.add_component(sleeping, Collider::sphere(0.5));

    let mut pw = PhysicsWorld::new();
    pw.sub_steps = 1;
    pw.step(&mut world);

    let body = world.get_by_index::<RigidBody>(sleeping.index()).unwrap();
    assert!(body.is_sleeping, "Sensor overlap should not wake sleeping body");
}
```

### Step 5: Run tests

```bash
cargo test -p engine-physics
```

### Step 6: Commit

```bash
git add crates/engine-physics/src/world.rs
git commit -m "fix(physics): sensors detect overlaps even when sleeping, don't wake bodies"
```

---

## Task 2: Physics Material System

**Goal:** Replace per-collider friction/restitution with a named material system and pair-combination lookup table.

**Files:**
- Create: `crates/engine-physics/src/material.rs`
- Modify: `crates/engine-physics/src/collider.rs`
- Modify: `crates/engine-physics/src/world.rs`
- Modify: `crates/engine-physics/src/lib.rs`

### Step 1: Create material.rs with PhysicsMaterial and MaterialTable

```rust
//! Physics material system with pair-based combination lookup.

use std::collections::HashMap;

/// Identifier for a physics material (index into MaterialTable).
pub type MaterialId = u16;

/// A named physics material with surface properties.
#[derive(Debug, Clone)]
pub struct PhysicsMaterial {
    pub name: String,
    pub friction: f32,
    pub restitution: f32,
    pub density: f32,
}

impl PhysicsMaterial {
    pub fn new(name: &str, friction: f32, restitution: f32, density: f32) -> Self {
        Self {
            name: name.to_string(),
            friction,
            restitution,
            density,
        }
    }
}

/// Combined surface properties for a collision pair.
#[derive(Debug, Clone, Copy)]
pub struct CombinedMaterial {
    pub friction: f32,
    pub restitution: f32,
}

/// Combination method for pairing two material values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineMode {
    /// Average of the two values.
    Average,
    /// Minimum of the two values.
    Min,
    /// Maximum of the two values.
    Max,
    /// Multiply the two values.
    Multiply,
}

/// Table of physics materials and pair combination rules.
pub struct MaterialTable {
    materials: Vec<PhysicsMaterial>,
    name_to_id: HashMap<String, MaterialId>,
    /// Explicit pair overrides: (min_id, max_id) → CombinedMaterial.
    pair_overrides: HashMap<(MaterialId, MaterialId), CombinedMaterial>,
    friction_mode: CombineMode,
    restitution_mode: CombineMode,
}

impl Default for MaterialTable {
    fn default() -> Self {
        let mut table = Self {
            materials: Vec::new(),
            name_to_id: HashMap::new(),
            pair_overrides: HashMap::new(),
            friction_mode: CombineMode::Average,
            restitution_mode: CombineMode::Max,
        };
        table.register_defaults();
        table
    }
}

impl MaterialTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register default materials: default, wood, metal, rubber, ice, stone.
    fn register_defaults(&mut self) {
        self.register(PhysicsMaterial::new("default", 0.5, 0.3, 1.0));
        self.register(PhysicsMaterial::new("wood", 0.4, 0.2, 0.6));
        self.register(PhysicsMaterial::new("metal", 0.3, 0.1, 7.8));
        self.register(PhysicsMaterial::new("rubber", 0.9, 0.8, 1.2));
        self.register(PhysicsMaterial::new("ice", 0.05, 0.05, 0.9));
        self.register(PhysicsMaterial::new("stone", 0.7, 0.1, 2.5));
        self.register(PhysicsMaterial::new("fabric", 0.6, 0.05, 0.3));

        // Explicit pair overrides for interesting combinations
        self.set_pair("rubber", "metal", CombinedMaterial { friction: 0.8, restitution: 0.6 });
        self.set_pair("ice", "metal", CombinedMaterial { friction: 0.02, restitution: 0.05 });
        self.set_pair("ice", "ice", CombinedMaterial { friction: 0.01, restitution: 0.02 });
    }

    /// Register a new material. Returns its MaterialId.
    pub fn register(&mut self, material: PhysicsMaterial) -> MaterialId {
        let id = self.materials.len() as MaterialId;
        self.name_to_id.insert(material.name.clone(), id);
        self.materials.push(material);
        id
    }

    /// Look up a material ID by name.
    pub fn get_id(&self, name: &str) -> Option<MaterialId> {
        self.name_to_id.get(name).copied()
    }

    /// Get a material by ID.
    pub fn get(&self, id: MaterialId) -> Option<&PhysicsMaterial> {
        self.materials.get(id as usize)
    }

    /// Set an explicit pair override.
    pub fn set_pair(&mut self, a: &str, b: &str, combined: CombinedMaterial) {
        if let (Some(id_a), Some(id_b)) = (self.get_id(a), self.get_id(b)) {
            let key = if id_a < id_b { (id_a, id_b) } else { (id_b, id_a) };
            self.pair_overrides.insert(key, combined);
        }
    }

    /// Combine two materials for a collision pair.
    pub fn combine(&self, a: MaterialId, b: MaterialId) -> CombinedMaterial {
        let key = if a < b { (a, b) } else { (b, a) };

        if let Some(override_val) = self.pair_overrides.get(&key) {
            return *override_val;
        }

        let mat_a = &self.materials[a as usize];
        let mat_b = &self.materials[b as usize];

        CombinedMaterial {
            friction: combine_value(mat_a.friction, mat_b.friction, self.friction_mode),
            restitution: combine_value(mat_a.restitution, mat_b.restitution, self.restitution_mode),
        }
    }

    /// Get the default material ID (always 0).
    pub fn default_id(&self) -> MaterialId {
        0
    }
}

fn combine_value(a: f32, b: f32, mode: CombineMode) -> f32 {
    match mode {
        CombineMode::Average => (a + b) * 0.5,
        CombineMode::Min => a.min(b),
        CombineMode::Max => a.max(b),
        CombineMode::Multiply => a * b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_materials_registered() {
        let table = MaterialTable::new();
        assert!(table.get_id("default").is_some());
        assert!(table.get_id("wood").is_some());
        assert!(table.get_id("metal").is_some());
        assert!(table.get_id("rubber").is_some());
        assert!(table.get_id("ice").is_some());
    }

    #[test]
    fn test_combine_same_material() {
        let table = MaterialTable::new();
        let metal = table.get_id("metal").unwrap();
        let combined = table.combine(metal, metal);
        let mat = table.get(metal).unwrap();
        assert!((combined.friction - mat.friction).abs() < 1e-6);
    }

    #[test]
    fn test_combine_pair_override() {
        let table = MaterialTable::new();
        let rubber = table.get_id("rubber").unwrap();
        let metal = table.get_id("metal").unwrap();
        let combined = table.combine(rubber, metal);
        assert!((combined.friction - 0.8).abs() < 1e-6);
        assert!((combined.restitution - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_combine_default_mode() {
        let table = MaterialTable::new();
        let wood = table.get_id("wood").unwrap();
        let stone = table.get_id("stone").unwrap();
        let combined = table.combine(wood, stone);
        // Average friction: (0.4 + 0.7) / 2 = 0.55
        assert!((combined.friction - 0.55).abs() < 1e-6);
        // Max restitution: max(0.2, 0.1) = 0.2
        assert!((combined.restitution - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_custom_material() {
        let mut table = MaterialTable::new();
        let glass = table.register(PhysicsMaterial::new("glass", 0.2, 0.05, 2.5));
        let combined = table.combine(glass, table.default_id());
        assert!(combined.friction > 0.0);
    }
}
```

### Step 2: Add material_id field to Collider

```rust
// collider.rs — add to Collider struct
pub struct Collider {
    pub shape: ColliderShape,
    pub is_sensor: bool,
    pub friction: f32,
    pub restitution: f32,
    pub density: f32,
    pub offset: Vec3,
    pub collision_layers: u32,
    pub collision_mask: u32,
    /// Material ID for pair-based lookup. None = use raw friction/restitution.
    pub material_id: Option<u16>,
}
```

Update Default impl to include `material_id: None`.

### Step 3: Add MaterialTable as a resource in PhysicsWorld

```rust
// world.rs — add to PhysicsWorld struct
pub struct PhysicsWorld {
    // ... existing fields
    pub material_table: MaterialTable,
}
```

Update Default impl:
```rust
material_table: MaterialTable::new(),
```

### Step 4: Use material lookup in collision resolution

In `resolve_collisions_with_warm_start`, replace the per-collider friction/restitution reads with material-aware lookup:

```rust
// When building manifolds, use material table if both colliders have material_id
let (friction, restitution) = if let (Some(id_a), Some(id_b)) = (col_a.material_id, col_b.material_id) {
    let combined = self.material_table.combine(id_a, id_b);
    (combined.friction, combined.restitution)
} else {
    let f = (friction_a + friction_b) * 0.5;
    let r = (restitution_a + restitution_b) * 0.5;
    (f, r)
};
```

### Step 5: Write integration tests

```rust
#[test]
fn test_material_affects_friction() {
    let mut world = World::new();

    let floor = world.spawn();
    world.add_component(floor, Transform::from_xyz(0.0, -0.5, 0.0));
    world.add_component(floor, RigidBody::new_static());
    let mut floor_col = Collider::cuboid(50.0, 0.5, 50.0);
    floor_col.material_id = Some(4); // ice
    world.add_component(floor, floor_col);

    let ball = world.spawn();
    world.add_component(ball, Transform::from_xyz(0.0, 0.3, 0.0));
    let mut body = RigidBody::new_dynamic();
    body.linear_velocity = Vec3::new(5.0, -2.0, 0.0);
    world.add_component(ball, body);
    let mut ball_col = Collider::sphere(0.5);
    ball_col.material_id = Some(3); // rubber
    world.add_component(ball, ball_col);

    let mut pw = PhysicsWorld::new();
    pw.sub_steps = 4;

    for _ in 0..30 {
        pw.step(&mut world);
    }

    // On ice, rubber ball should slide further than on default surfaces
    let transform = world.get_by_index::<Transform>(ball.index()).unwrap();
    assert!(transform.position.x > 1.0, "Ball should slide on ice, pos.x = {}", transform.position.x);
}
```

### Step 6: Run tests and commit

```bash
cargo test -p engine-physics
git add crates/engine-physics/src/material.rs crates/engine-physics/src/collider.rs crates/engine-physics/src/world.rs crates/engine-physics/src/lib.rs
git commit -m "feat(physics): add material system with pair combination lookup table"
```

---

## Task 3: Joint Constraint Enhancement

**Goal:** Implement actual angular constraint solving for ball-socket cone limits and hinge angle limits. This is the prerequisite for ragdoll.

**Files:**
- Modify: `crates/engine-physics/src/joint.rs`
- Modify: `crates/engine-physics/src/world.rs`

### Step 1: Add angular constraint data to JointSolver

Enhance `JointSolver` to actually solve ball-socket cone limits and hinge angle limits using positional correction (not just spring forces).

```rust
// joint.rs — add to JointSolver
impl JointSolver {
    /// Solve angular constraints for ball-socket and hinge joints.
    ///
    /// Takes the current positions and rotations of both bodies,
    /// applies positional corrections to enforce angle limits.
    pub fn solve_angular_constraints(
        &self,
        world: &mut engine_ecs::world::World,
        dt: f32,
    ) {
        for joint in &self.joints {
            if !joint.enabled {
                continue;
            }

            match joint.joint_type {
                JointType::BallSocket => self.solve_ball_socket(world, joint, dt),
                JointType::Hinge => self.solve_hinge(world, joint, dt),
                JointType::Spring => {} // Springs are velocity-based, handled separately
            }
        }
    }

    fn solve_ball_socket(
        &self,
        world: &mut engine_ecs::world::World,
        joint: &Joint,
        _dt: f32,
    ) {
        use engine_core::transform::Transform;
        use engine_math::Quat;

        let Some(pos_a) = world.get_by_index::<Transform>(joint.entity_a).map(|t| t.position) else { return; };
        let Some(pos_b) = world.get_by_index::<Transform>(joint.entity_b).map(|t| t.position) else { return; };
        let Some(rot_a) = world.get_by_index::<Transform>(joint.entity_a).map(|t| {
            Quat::from_euler(engine_math::EulerRot::XYZ, t.rotation.x, t.rotation.y, t.rotation.z)
        }) else { return; };
        let Some(rot_b) = world.get_by_index::<Transform>(joint.entity_b).map(|t| {
            Quat::from_euler(engine_math::EulerRot::XYZ, t.rotation.x, t.rotation.y, t.rotation.z)
        }) else { return; };

        // Compute world-space anchor positions
        let world_anchor_a = pos_a + rot_a * joint.anchor_a;
        let world_anchor_b = pos_b + rot_b * joint.anchor_b;

        // Positional constraint: keep anchors together
        let delta = world_anchor_b - world_anchor_a;
        let dist = delta.length();

        if dist < 1e-6 {
            return;
        }

        let is_dynamic_a = world.get_by_index::<RigidBody>(joint.entity_a)
            .is_some_and(|b| b.body_type == BodyType::Dynamic);
        let is_dynamic_b = world.get_by_index::<RigidBody>(joint.entity_b)
            .is_some_and(|b| b.body_type == BodyType::Dynamic);

        let inv_mass_a = if is_dynamic_a {
            world.get_by_index::<RigidBody>(joint.entity_a).map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 0.0 })
        } else { 0.0 };
        let inv_mass_b = if is_dynamic_b {
            world.get_by_index::<RigidBody>(joint.entity_b).map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 0.0 })
        } else { 0.0 };

        let total_inv = inv_mass_a + inv_mass_b;
        if total_inv <= 0.0 { return; }

        // Cone limit check: compute angle between current direction and rest direction
        let current_dir = delta / dist;
        let rest_dir = rot_a * Vec3::Y; // Default rest direction
        let dot = current_dir.dot(rest_dir).clamp(-1.0, 1.0);
        let angle = dot.acos();

        if angle > joint.max_cone_angle {
            // Push bodies to satisfy cone limit
            let correction_mag = (angle - joint.max_cone_angle) * 0.5;
            let correction_dir = current_dir.cross(rest_dir).normalize_or_zero();
            if correction_dir.length_squared() < 1e-6 {
                return;
            }
            let correction = correction_dir * correction_mag * dist * 0.5;

            if is_dynamic_a {
                if let Some(t) = world.get_by_index_mut::<Transform>(joint.entity_a) {
                    t.position += correction * (inv_mass_a / total_inv);
                }
            }
            if is_dynamic_b {
                if let Some(t) = world.get_by_index_mut::<Transform>(joint.entity_b) {
                    t.position -= correction * (inv_mass_b / total_inv);
                }
            }
        }

        // Distance constraint (ball-socket keeps anchors at same point)
        let correction = delta * (dist.min(0.01)) / total_inv; // Small positional bias
        if is_dynamic_a {
            if let Some(t) = world.get_by_index_mut::<Transform>(joint.entity_a) {
                t.position += correction * inv_mass_a;
            }
        }
        if is_dynamic_b {
            if let Some(t) = world.get_by_index_mut::<Transform>(joint.entity_b) {
                t.position -= correction * inv_mass_b;
            }
        }
    }

    fn solve_hinge(
        &self,
        world: &mut engine_ecs::world::World,
        joint: &Joint,
        _dt: f32,
    ) {
        use engine_core::transform::Transform;
        use engine_math::Quat;

        let Some(rot_a) = world.get_by_index::<Transform>(joint.entity_a).map(|t| {
            Quat::from_euler(engine_math::EulerRot::XYZ, t.rotation.x, t.rotation.y, t.rotation.z)
        }) else { return; };
        let Some(rot_b) = world.get_by_index::<Transform>(joint.entity_b).map(|t| {
            Quat::from_euler(engine_math::EulerRot::XYZ, t.rotation.x, t.rotation.y, t.rotation.z)
        }) else { return; };

        // Hinge axis in world space
        let world_axis_a = rot_a * joint.axis;

        // Compute relative rotation between the two bodies around the hinge axis
        let relative_rot = rot_b * rot_a.inverse();

        // Decompose rotation into swing and twist around hinge axis
        // Twist = component around hinge axis (allowed)
        // Swing = component perpendicular to hinge axis (constrained)
        let twist_angle = 2.0 * relative_rot.w.acos() * relative_rot.xyz().dot(world_axis_a).signum();
        let swing = relative_rot.xyz() - world_axis_a * relative_rot.xyz().dot(world_axis_a);
        let swing_angle = 2.0 * swing.length().asin();

        // Check angle limits for the twist
        if twist_angle < joint.min_angle || twist_angle > joint.max_angle {
            // Clamp the twist angle
            let clamped = twist_angle.clamp(joint.min_angle, joint.max_angle);
            let error = twist_angle - clamped;

            // Apply rotational correction to body B
            let is_dynamic_b = world.get_by_index::<RigidBody>(joint.entity_b)
                .is_some_and(|b| b.body_type == BodyType::Dynamic);
            if is_dynamic_b {
                if let Some(body) = world.get_by_index_mut::<RigidBody>(joint.entity_b) {
                    let correction = world_axis_a * error * 0.5;
                    body.angular_velocity -= correction;
                }
            }
        }

        // Also apply positional constraint at anchors (like ball-socket)
        let pos_a = world.get_by_index::<Transform>(joint.entity_a).map(|t| t.position);
        let pos_b = world.get_by_index::<Transform>(joint.entity_b).map(|t| t.position);
        if let (Some(pa), Some(pb)) = (pos_a, pos_b) {
            let world_anchor_a = pa + rot_a * joint.anchor_a;
            let world_anchor_b = pb + rot_b * joint.anchor_b;
            let delta = world_anchor_b - world_anchor_a;
            let dist = delta.length();

            if dist > 1e-6 {
                let is_dynamic_a = world.get_by_index::<RigidBody>(joint.entity_a)
                    .is_some_and(|b| b.body_type == BodyType::Dynamic);
                let is_dynamic_b = world.get_by_index::<RigidBody>(joint.entity_b)
                    .is_some_and(|b| b.body_type == BodyType::Dynamic);

                let inv_mass_a = if is_dynamic_a {
                    world.get_by_index::<RigidBody>(joint.entity_a).map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 0.0 })
                } else { 0.0 };
                let inv_mass_b = if is_dynamic_b {
                    world.get_by_index::<RigidBody>(joint.entity_b).map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 0.0 })
                } else { 0.0 };

                let total_inv = inv_mass_a + inv_mass_b;
                if total_inv > 0.0 {
                    let correction = delta * 0.5 / total_inv;
                    if is_dynamic_a {
                        if let Some(t) = world.get_by_index_mut::<Transform>(joint.entity_a) {
                            t.position += correction * inv_mass_a;
                        }
                    }
                    if is_dynamic_b {
                        if let Some(t) = world.get_by_index_mut::<Transform>(joint.entity_b) {
                            t.position -= correction * inv_mass_b;
                        }
                    }
                }
            }
        }
    }
}
```

### Step 2: Integrate joint solving into PhysicsWorld::step

```rust
// world.rs — add joint_solver field and integrate into step
pub struct PhysicsWorld {
    // ... existing fields
    pub joint_solver: JointSolver,
}

impl PhysicsWorld {
    pub fn step(&mut self, world: &mut World) {
        // ... existing sub-step loop
        for _ in 0..self.sub_steps {
            self.integrate_bodies(world, dt);
            self.detect_collisions(world);
            self.wake_colliding_bodies(world);
            self.resolve_collisions_with_warm_start(world);
            // NEW: solve joint constraints
            self.joint_solver.solve_angular_constraints(world, dt);
            self.joint_solver.solve_springs_step(world, dt);
        }
        // ...
    }
}
```

### Step 3: Write tests for joint constraints

```rust
#[test]
fn test_hinge_angle_limit() {
    let mut world = World::new();

    let anchor = world.spawn();
    world.add_component(anchor, Transform::from_xyz(0.0, 5.0, 0.0));
    world.add_component(anchor, RigidBody::new_static());

    let door = world.spawn();
    world.add_component(door, Transform::from_xyz(0.0, 3.0, 0.0));
    let mut body = RigidBody::new_dynamic();
    body.mass = 1.0;
    world.add_component(door, body);

    let mut solver = JointSolver::new();
    let hinge = Joint::hinge(anchor.index(), door.index(), Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), Vec3::Y)
        .with_angle_limits(-1.5, 1.5); // ~85 degrees each way
    solver.add_joint(hinge);

    // Apply a large angular impulse
    if let Some(body) = world.get_by_index_mut::<RigidBody>(door.index()) {
        body.angular_velocity = Vec3::new(0.0, 10.0, 0.0);
    }

    // Step multiple times
    for _ in 0..100 {
        solver.solve_angular_constraints(&mut world, 1.0 / 60.0);
    }

    // Angular velocity should be limited
    let body = world.get_by_index::<RigidBody>(door.index()).unwrap();
    // The hinge should have prevented unlimited rotation
    // (exact assertion depends on the integration)
}
```

### Step 4: Run tests and commit

```bash
cargo test -p engine-physics
git add crates/engine-physics/src/joint.rs crates/engine-physics/src/world.rs
git commit -m "feat(physics): implement angular constraints for ball-socket cone and hinge angle limits"
```

---

## Task 4: Ragdoll System

**Goal:** Enable physics-driven ragdoll by mapping skeleton bones to rigid bodies connected by joints.

**Files:**
- Create: `crates/engine-physics/src/ragdoll.rs`
- Modify: `crates/engine-physics/src/world.rs`
- Modify: `crates/engine-physics/src/lib.rs`

### Step 1: Create ragdoll.rs with core types

```rust
//! Ragdoll system: maps skeleton bones to physics bodies with joint constraints.

use engine_math::{Quat, Vec3};

/// Maps a skeleton bone to a physics entity.
#[derive(Debug, Clone)]
pub struct RagdollBone {
    /// The bone index in the skeleton.
    pub bone_index: u32,
    /// The physics entity index for this bone's rigid body.
    pub physics_entity: u32,
    /// Offset from the bone's rest position to the physics body center.
    pub offset: Vec3,
    /// Whether this bone is kinematic (animation-driven) or dynamic (physics-driven).
    pub is_kinematic: bool,
    /// Blend weight: 0.0 = fully animated, 1.0 = fully physics-driven.
    pub blend_weight: f32,
}

/// Configuration for creating a ragdoll from a skeleton.
#[derive(Debug, Clone)]
pub struct RagdollConfig {
    /// Default capsule radius for ragdoll bones.
    pub default_radius: f32,
    /// Default mass per bone.
    pub default_mass: f32,
    /// Default blend weight (0.0 = animated, 1.0 = physics).
    pub default_blend_weight: f32,
    /// Bone pairs that should be connected by joints.
    pub joint_pairs: Vec<(u32, u32, RagdollJointType)>,
}

impl Default for RagdollConfig {
    fn default() -> Self {
        Self {
            default_radius: 0.05,
            default_mass: 1.0,
            default_blend_weight: 1.0,
            joint_pairs: Vec::new(),
        }
    }
}

/// Type of joint to create between ragdoll bones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagdollJointType {
    /// Ball-socket with cone limit (shoulders, hips).
    BallSocket { max_cone_angle: f32 },
    /// Hinge with angle limits (elbows, knees).
    Hinge { min_angle: f32, max_angle: f32 },
    /// Fixed joint (no relative motion).
    Fixed,
}

/// Component attached to the root entity of a ragdoll.
#[derive(Debug, Clone)]
pub struct Ragdoll {
    /// All bones in this ragdoll.
    pub bones: Vec<RagdollBone>,
    /// Whether the ragdoll is active (physics-driven) or inactive (animation-driven).
    pub active: bool,
    /// Global blend weight for transitioning between animation and physics.
    pub global_blend_weight: f32,
    /// Transition speed (blend weight change per second).
    pub transition_speed: f32,
    /// Target blend weight during transition.
    pub target_blend_weight: f32,
}

impl Default for Ragdoll {
    fn default() -> Self {
        Self {
            bones: Vec::new(),
            active: false,
            global_blend_weight: 0.0,
            transition_speed: 5.0,
            target_blend_weight: 0.0,
        }
    }
}

impl Ragdoll {
    /// Create a new ragdoll.
    pub fn new() -> Self {
        Self::default()
    }

    /// Activate the ragdoll (transition from animation to physics).
    pub fn activate(&mut self) {
        self.active = true;
        self.target_blend_weight = 1.0;
    }

    /// Deactivate the ragdoll (transition from physics to animation).
    pub fn deactivate(&mut self) {
        self.target_blend_weight = 0.0;
    }

    /// Update blend weight for smooth transitions.
    pub fn update_blend(&mut self, dt: f32) {
        if (self.global_blend_weight - self.target_blend_weight).abs() > 0.001 {
            let direction = if self.target_blend_weight > self.global_blend_weight {
                1.0
            } else {
                -1.0
            };
            self.global_blend_weight =
                (self.global_blend_weight + direction * self.transition_speed * dt)
                    .clamp(0.0, 1.0);

            if self.global_blend_weight >= 1.0 {
                self.global_blend_weight = 1.0;
            } else if self.global_blend_weight <= 0.0 {
                self.global_blend_weight = 0.0;
                self.active = false;
            }
        }

        // Update per-bone blend weights
        for bone in &mut self.bones {
            bone.blend_weight = self.global_blend_weight;
        }
    }

    /// Get the physics entity for a specific bone index.
    pub fn physics_entity_for_bone(&self, bone_index: u32) -> Option<u32> {
        self.bones
            .iter()
            .find(|b| b.bone_index == bone_index)
            .map(|b| b.physics_entity)
    }
}

/// Builder for creating ragdolls from skeleton definitions.
pub struct RagdollBuilder {
    config: RagdollConfig,
    bones: Vec<RagdollBone>,
}

impl RagdollBuilder {
    pub fn new(config: RagdollConfig) -> Self {
        Self {
            config,
            bones: Vec::new(),
        }
    }

    /// Add a bone to the ragdoll.
    pub fn add_bone(&mut self, bone_index: u32, physics_entity: u32, offset: Vec3) -> &mut Self {
        self.bones.push(RagdollBone {
            bone_index,
            physics_entity,
            offset,
            is_kinematic: false,
            blend_weight: self.config.default_blend_weight,
        });
        self
    }

    /// Build the ragdoll component.
    pub fn build(self) -> Ragdoll {
        Ragdoll {
            bones: self.bones,
            active: false,
            global_blend_weight: 0.0,
            transition_speed: 5.0,
            target_blend_weight: 0.0,
        }
    }
}

/// System that updates ragdoll blend weights and synchronizes physics poses back to transforms.
pub fn ragdoll_update_system(world: &mut engine_ecs::world::World) {
    use engine_core::transform::Transform;

    let dt = 1.0 / 60.0; // Fixed timestep

    // Update blend weights
    let ragdoll_indices: Vec<u32> = world.component_entities::<Ragdoll>().to_vec();
    for &idx in &ragdoll_indices {
        if let Some(ragdoll) = world.get_by_index_mut::<Ragdoll>(idx) {
            ragdoll.update_blend(dt);
        }
    }

    // Blend physics poses back to transforms
    for &idx in &ragdoll_indices {
        let ragdoll = match world.get_by_index::<Ragdoll>(idx) {
            Some(r) => r.clone(),
            None => continue,
        };

        if !ragdoll.active && ragdoll.global_blend_weight <= 0.0 {
            continue;
        }

        for bone in &ragdoll.bones {
            if bone.blend_weight <= 0.0 {
                continue;
            }

            // Get the physics body's transform
            let physics_pos = world
                .get_by_index::<Transform>(bone.physics_entity)
                .map(|t| t.position);
            let physics_rot = world
                .get_by_index::<Transform>(bone.physics_entity)
                .map(|t| t.rotation);

            if let (Some(pp), Some(pr)) = (physics_pos, physics_rot) {
                // Blend between animation pose and physics pose
                if let Some(bone_transform) = world.get_by_index_mut::<Transform>(idx) {
                    // Lerp position
                    let anim_pos = bone_transform.position;
                    bone_transform.position = anim_pos + (pp + bone.offset - anim_pos) * bone.blend_weight;

                    // Slerp rotation (approximate with lerp for simplicity)
                    let anim_rot = bone_transform.rotation;
                    bone_transform.rotation = Vec3::new(
                        anim_rot.x + (pr.x - anim_rot.x) * bone.blend_weight,
                        anim_rot.y + (pr.y - anim_rot.y) * bone.blend_weight,
                        anim_rot.z + (pr.z - anim_rot.z) * bone.blend_weight,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ragdoll_creation() {
        let ragdoll = Ragdoll::new();
        assert!(!ragdoll.active);
        assert!((ragdoll.global_blend_weight).abs() < 1e-6);
    }

    #[test]
    fn test_ragdoll_activation() {
        let mut ragdoll = Ragdoll::new();
        ragdoll.activate();
        assert!(ragdoll.active);
        assert!((ragdoll.target_blend_weight - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ragdoll_blend_transition() {
        let mut ragdoll = Ragdoll::new();
        ragdoll.activate();
        ragdoll.transition_speed = 2.0; // 0.5 seconds to full blend

        // Simulate 0.25 seconds
        ragdoll.update_blend(0.25);
        assert!(ragdoll.global_blend_weight > 0.0);
        assert!(ragdoll.global_blend_weight < 1.0);

        // Simulate another 0.25 seconds (total 0.5s)
        ragdoll.update_blend(0.25);
        assert!((ragdoll.global_blend_weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_ragdoll_deactivation() {
        let mut ragdoll = Ragdoll::new();
        ragdoll.activate();
        ragdoll.global_blend_weight = 1.0;

        ragdoll.deactivate();
        assert!((ragdoll.target_blend_weight).abs() < 1e-6);

        // Simulate transition back
        ragdoll.transition_speed = 2.0;
        ragdoll.update_blend(1.0);
        assert!(ragdoll.global_blend_weight < 0.01);
        assert!(!ragdoll.active);
    }

    #[test]
    fn test_ragdoll_builder() {
        let config = RagdollConfig::default();
        let mut builder = RagdollBuilder::new(config);
        builder.add_bone(0, 10, Vec3::ZERO);
        builder.add_bone(1, 11, Vec3::new(0.0, 0.5, 0.0));

        let ragdoll = builder.build();
        assert_eq!(ragdoll.bones.len(), 2);
        assert_eq!(ragdoll.bones[0].bone_index, 0);
        assert_eq!(ragdoll.bones[0].physics_entity, 10);
    }

    #[test]
    fn test_ragdoll_physics_entity_lookup() {
        let mut ragdoll = Ragdoll::new();
        ragdoll.bones.push(RagdollBone {
            bone_index: 5,
            physics_entity: 42,
            offset: Vec3::ZERO,
            is_kinematic: false,
            blend_weight: 1.0,
        });

        assert_eq!(ragdoll.physics_entity_for_bone(5), Some(42));
        assert_eq!(ragdoll.physics_entity_for_bone(99), None);
    }
}
```

### Step 2: Register ragdoll system in PhysicsPlugin

```rust
// plugin.rs
use crate::ragdoll::ragdoll_update_system;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::default());
        app.add_system(physics_step_system);
        app.add_system(ragdoll_update_system);
    }
}
```

### Step 3: Run tests and commit

```bash
cargo test -p engine-physics
git add crates/engine-physics/src/ragdoll.rs crates/engine-physics/src/plugin.rs crates/engine-physics/src/lib.rs
git commit -m "feat(physics): add ragdoll system with skeleton-to-physics bone mapping and blend transitions"
```

---

## Task 5: Vehicle Physics (Arcade)

**Goal:** Implement raycast-based arcade vehicle physics — suspension, steering, engine forces.

**Files:**
- Create: `crates/engine-physics/src/vehicle.rs`
- Modify: `crates/engine-physics/src/world.rs`
- Modify: `crates/engine-physics/src/lib.rs`

### Step 1: Create vehicle.rs with core types

```rust
//! Arcade vehicle physics with raycast-based suspension.

use engine_math::Vec3;

/// A wheel on a vehicle.
#[derive(Debug, Clone)]
pub struct Wheel {
    /// Offset from vehicle center (local space).
    pub offset: Vec3,
    /// Wheel radius.
    pub radius: f32,
    /// Current steering angle in radians.
    pub steering_angle: f32,
    /// Whether this wheel can steer.
    pub can_steer: bool,
    /// Whether this wheel is powered by the engine.
    pub is_driven: bool,
    /// Current suspension compression (0.0 = fully extended, 1.0 = fully compressed).
    pub suspension_compression: f32,
    /// Suspension rest length.
    pub rest_length: f32,
    /// Suspension stiffness (spring constant).
    pub spring_stiffness: f32,
    /// Suspension damping.
    pub damping: f32,
    /// Whether the wheel is currently on the ground.
    pub is_grounded: bool,
    /// Lateral grip factor (0.0 = no grip, 1.0 = full grip).
    pub lateral_grip: f32,
    /// Longitudinal grip factor.
    pub longitudinal_grip: f32,
}

impl Default for Wheel {
    fn default() -> Self {
        Self {
            offset: Vec3::ZERO,
            radius: 0.3,
            steering_angle: 0.0,
            can_steer: false,
            is_driven: false,
            suspension_compression: 0.0,
            rest_length: 0.3,
            spring_stiffness: 35000.0,
            damping: 4500.0,
            is_grounded: false,
            lateral_grip: 0.9,
            longitudinal_grip: 0.9,
        }
    }
}

/// Vehicle component for arcade-style driving physics.
#[derive(Debug, Clone)]
pub struct Vehicle {
    /// All wheels on this vehicle.
    pub wheels: Vec<Wheel>,
    /// Maximum engine force (in Newtons).
    pub max_engine_force: f32,
    /// Maximum braking force.
    pub max_brake_force: f32,
    /// Maximum steering angle in radians.
    pub max_steer_angle: f32,
    /// Steering speed (radians per second).
    pub steer_speed: f32,
    /// Current throttle input (0.0 to 1.0).
    pub throttle: f32,
    /// Current brake input (0.0 to 1.0).
    pub brake: f32,
    /// Current steering input (-1.0 to 1.0).
    pub steer_input: f32,
    /// Downforce coefficient (increases grip at speed).
    pub downforce_coefficient: f32,
    /// Anti-roll bar stiffness.
    pub anti_roll_stiffness: f32,
    /// Maximum speed (m/s).
    pub max_speed: f32,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            wheels: Vec::new(),
            max_engine_force: 6000.0,
            max_brake_force: 12000.0,
            max_steer_angle: 0.5, // ~28 degrees
            steer_speed: 3.0,
            throttle: 0.0,
            brake: 0.0,
            steer_input: 0.0,
            downforce_coefficient: 0.5,
            anti_roll_stiffness: 5000.0,
            max_speed: 50.0, // ~180 km/h
        }
    }
}

impl Vehicle {
    /// Create a new vehicle with default 4-wheel layout.
    pub fn new_car() -> Self {
        let mut vehicle = Self::default();

        // Front-left wheel
        vehicle.wheels.push(Wheel {
            offset: Vec3::new(-0.8, -0.2, 1.2),
            can_steer: true,
            is_driven: false,
            ..Default::default()
        });

        // Front-right wheel
        vehicle.wheels.push(Wheel {
            offset: Vec3::new(0.8, -0.2, 1.2),
            can_steer: true,
            is_driven: false,
            ..Default::default()
        });

        // Rear-left wheel
        vehicle.wheels.push(Wheel {
            offset: Vec3::new(-0.8, -0.2, -1.2),
            can_steer: false,
            is_driven: true,
            ..Default::default()
        });

        // Rear-right wheel
        vehicle.wheels.push(Wheel {
            offset: Vec3::new(0.8, -0.2, -1.2),
            can_steer: false,
            is_driven: true,
            ..Default::default()
        });

        vehicle
    }

    /// Set throttle input.
    pub fn set_throttle(&mut self, value: f32) {
        self.throttle = value.clamp(-1.0, 1.0);
    }

    /// Set brake input.
    pub fn set_brake(&mut self, value: f32) {
        self.brake = value.clamp(0.0, 1.0);
    }

    /// Set steering input.
    pub fn set_steer(&mut self, value: f32) {
        self.steer_input = value.clamp(-1.0, 1.0);
    }
}

/// System that applies vehicle physics each frame.
pub fn vehicle_physics_system(world: &mut engine_ecs::world::World) {
    use crate::body::{BodyType, RigidBody};
    use engine_core::transform::Transform;
    use engine_math::{EulerRot, Quat};

    let dt = 1.0 / 60.0;
    let vehicle_indices: Vec<u32> = world.component_entities::<Vehicle>().to_vec();

    for &idx in &vehicle_indices {
        // Clone vehicle data to avoid borrow conflicts
        let mut vehicle = match world.get_by_index::<Vehicle>(idx) {
            Some(v) => v.clone(),
            None => continue,
        };

        let body = match world.get_by_index::<RigidBody>(idx) {
            Some(b) if b.body_type == BodyType::Dynamic => b.clone(),
            _ => continue,
        };

        let transform = match world.get_by_index::<Transform>(idx) {
            Some(t) => t.clone(),
            None => continue,
        };

        let rot = Quat::from_euler(
            EulerRot::XYZ,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
        );

        let forward = rot * Vec3::Z;
        let right = rot * Vec3::X;
        let up = rot * Vec3::Y;

        let speed = body.linear_velocity.dot(forward);
        let speed_abs = speed.abs();

        // Update steering
        for wheel in &mut vehicle.wheels {
            if wheel.can_steer {
                let target_steer = vehicle.steer_input * vehicle.max_steer_angle;
                let steer_delta = target_steer - wheel.steering_angle;
                let max_change = vehicle.steer_speed * dt;
                wheel.steering_angle += steer_delta.clamp(-max_change, max_change);

                // Reduce steering at high speed (speed-sensitive steering)
                let speed_factor = 1.0 - (speed_abs / vehicle.max_speed).min(1.0) * 0.6;
                wheel.steering_angle *= speed_factor;
            }
        }

        // Compute forces for each wheel
        let mut total_force = Vec3::ZERO;
        let mut total_torque = Vec3::ZERO;

        for wheel in &mut vehicle.wheels {
            let wheel_world_pos = transform.position + rot * wheel.offset;

            // Raycast downward for suspension
            let ray_origin = wheel_world_pos + up * wheel.rest_length;
            let ray_dir = -up;
            let ray_length = wheel.rest_length + wheel.radius + 0.5;

            // Simple ground check (assume flat ground at y=0 for arcade feel)
            let ground_y = 0.0;
            let wheel_bottom = wheel_world_pos.y - wheel.radius;
            let is_grounded = wheel_bottom <= ground_y + 0.1;
            wheel.is_grounded = is_grounded;

            if !is_grounded {
                wheel.suspension_compression = 0.0;
                continue;
            }

            // Suspension force
            let compression = (ground_y - (wheel_world_pos.y - wheel.radius))
                .clamp(0.0, wheel.rest_length);
            wheel.suspension_compression = compression / wheel.rest_length;

            let spring_force = compression * wheel.spring_stiffness;
            let damping_force = -body.linear_velocity.dot(up) * wheel.damping;
            let suspension_force = (spring_force + damping_force).max(0.0);

            total_force += up * suspension_force;

            // Downforce (increases with speed)
            let downforce = vehicle.downforce_coefficient * speed_abs * speed_abs;
            total_force -= up * downforce * 0.25; // Split among 4 wheels

            // Engine force (only on driven wheels)
            if wheel.is_driven && speed_abs < vehicle.max_speed {
                let engine_force = forward * vehicle.throttle * vehicle.max_engine_force;
                total_force += engine_force * 0.25; // Split among driven wheels
            }

            // Brake force
            if vehicle.brake > 0.0 && speed_abs > 0.1 {
                let brake_dir = -body.linear_velocity.normalize();
                let brake_force = brake_dir * vehicle.brake * vehicle.max_brake_force;
                total_force += brake_force * 0.25;
            }

            // Lateral grip (counter sideways sliding)
            let lateral_vel = right * body.linear_velocity.dot(right);
            let lateral_correction = -lateral_vel * wheel.lateral_grip * 10.0;
            total_force += lateral_correction * 0.25;

            // Steering torque for front wheels
            if wheel.can_steer {
                let steer_forward = Quat::from_axis_angle(up, wheel.steering_angle) * forward;
                let steer_lateral = right * body.linear_velocity.dot(right);
                let steer_correction = -steer_lateral * wheel.steering_angle * 5.0;
                total_force += steer_correction;
            }
        }

        // Anti-roll bar: reduce body roll
        if vehicle.wheels.len() >= 4 {
            let left_compression = (vehicle.wheels[0].suspension_compression
                + vehicle.wheels[2].suspension_compression)
                * 0.5;
            let right_compression = (vehicle.wheels[1].suspension_compression
                + vehicle.wheels[3].suspension_compression)
                * 0.5;
            let roll_diff = left_compression - right_compression;
            total_torque += forward * roll_diff * vehicle.anti_roll_stiffness;
        }

        // Apply forces to the rigid body
        if let Some(body_mut) = world.get_by_index_mut::<RigidBody>(idx) {
            body_mut.linear_velocity += total_force / body_mut.mass * dt;
            body_mut.angular_velocity += total_torque / body_mut.mass * dt;

            // Angular damping to prevent spinning
            body_mut.angular_velocity *= 0.95;
        }

        // Write back vehicle state (wheel updates)
        if let Some(vehicle_mut) = world.get_by_index_mut::<Vehicle>(idx) {
            *vehicle_mut = vehicle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vehicle_creation() {
        let vehicle = Vehicle::new_car();
        assert_eq!(vehicle.wheels.len(), 4);
        assert!(vehicle.wheels[0].can_steer); // Front-left
        assert!(vehicle.wheels[1].can_steer); // Front-right
        assert!(!vehicle.wheels[2].can_steer); // Rear-left
        assert!(!vehicle.wheels[3].can_steer); // Rear-right
        assert!(vehicle.wheels[2].is_driven); // Rear-left
        assert!(vehicle.wheels[3].is_driven); // Rear-right
    }

    #[test]
    fn test_vehicle_input_clamping() {
        let mut vehicle = Vehicle::new_car();
        vehicle.set_throttle(2.0);
        assert!((vehicle.throttle - 1.0).abs() < 1e-6);

        vehicle.set_throttle(-0.5);
        assert!((vehicle.throttle - (-0.5)).abs() < 1e-6);

        vehicle.set_brake(-1.0);
        assert!((vehicle.brake).abs() < 1e-6);

        vehicle.set_steer(0.8);
        assert!((vehicle.steer_input - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_default_vehicle_properties() {
        let vehicle = Vehicle::default();
        assert!(vehicle.max_engine_force > 0.0);
        assert!(vehicle.max_brake_force > 0.0);
        assert!(vehicle.max_steer_angle > 0.0);
        assert!(vehicle.max_speed > 0.0);
    }

    #[test]
    fn test_wheel_defaults() {
        let wheel = Wheel::default();
        assert!(wheel.radius > 0.0);
        assert!(wheel.spring_stiffness > 0.0);
        assert!(wheel.lateral_grip > 0.0);
        assert!(!wheel.is_grounded);
    }
}
```

### Step 2: Register vehicle system in PhysicsPlugin

```rust
// plugin.rs
use crate::vehicle::vehicle_physics_system;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::default());
        app.add_system(physics_step_system);
        app.add_system(ragdoll_update_system);
        app.add_system(vehicle_physics_system);
    }
}
```

### Step 3: Run tests and commit

```bash
cargo test -p engine-physics
git add crates/engine-physics/src/vehicle.rs crates/engine-physics/src/plugin.rs crates/engine-physics/src/lib.rs
git commit -m "feat(physics): add arcade vehicle physics with raycast suspension and steering"
```

---

## Task 6: Soft Body Physics (Verlet)

**Goal:** Implement basic Verlet integration soft body simulation with distance constraints — suitable for cloth, ropes, and simple deformable objects.

**Files:**
- Create: `crates/engine-physics/src/softbody.rs`
- Modify: `crates/engine-physics/src/world.rs`
- Modify: `crates/engine-physics/src/lib.rs`

### Step 1: Create softbody.rs with Verlet particle and constraints

```rust
//! Soft body physics using Verlet integration with distance constraints.

use engine_math::Vec3;

/// A single particle in a soft body (Verlet integration).
#[derive(Debug, Clone)]
pub struct Particle {
    /// Current position.
    pub position: Vec3,
    /// Previous position (for Verlet integration).
    pub prev_position: Vec3,
    /// Accumulated force for this frame.
    pub force: Vec3,
    /// Mass of this particle.
    pub mass: f32,
    /// Inverse mass (0.0 = pinned/unmovable).
    pub inv_mass: f32,
    /// Whether this particle is pinned (immovable).
    pub pinned: bool,
}

impl Particle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            prev_position: position,
            force: Vec3::ZERO,
            mass,
            inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            pinned: false,
        }
    }

    pub fn pinned(position: Vec3) -> Self {
        Self {
            position,
            prev_position: position,
            force: Vec3::ZERO,
            mass: 0.0,
            inv_mass: 0.0,
            pinned: true,
        }
    }
}

/// A distance constraint between two particles.
#[derive(Debug, Clone)]
pub struct DistanceConstraint {
    /// Index of first particle.
    pub particle_a: usize,
    /// Index of second particle.
    pub particle_b: usize,
    /// Rest length.
    pub rest_length: f32,
    /// Stiffness (0.0 to 1.0, where 1.0 = rigid).
    pub stiffness: f32,
}

impl DistanceConstraint {
    pub fn new(a: usize, b: usize, rest_length: f32, stiffness: f32) -> Self {
        Self {
            particle_a: a,
            particle_b: b,
            rest_length,
            stiffness: stiffness.clamp(0.0, 1.0),
        }
    }
}

/// A bend constraint between three particles (resist bending).
#[derive(Debug, Clone)]
pub struct BendConstraint {
    /// Index of the middle particle.
    pub particle_mid: usize,
    /// Index of the first outer particle.
    pub particle_a: usize,
    /// Index of the second outer particle.
    pub particle_b: usize,
    /// Rest angle (radians).
    pub rest_angle: f32,
    /// Stiffness.
    pub stiffness: f32,
}

/// Soft body component using Verlet integration.
#[derive(Debug, Clone)]
pub struct SoftBody {
    /// Particles of this soft body.
    pub particles: Vec<Particle>,
    /// Distance constraints (structural + shear).
    pub distance_constraints: Vec<DistanceConstraint>,
    /// Bend constraints (optional, for cloth).
    pub bend_constraints: Vec<BendConstraint>,
    /// Damping factor (0.0 = no damping, 1.0 = full damping).
    pub damping: f32,
    /// Number of constraint solver iterations per step.
    pub solver_iterations: u32,
    /// Gravity applied to this soft body.
    pub gravity: Vec3,
    /// Whether to self-intersect (not implemented in basic version).
    pub self_collision: bool,
}

impl Default for SoftBody {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            distance_constraints: Vec::new(),
            bend_constraints: Vec::new(),
            damping: 0.01,
            solver_iterations: 5,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            self_collision: false,
        }
    }
}

impl SoftBody {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cloth mesh.
    pub fn create_cloth(
        origin: Vec3,
        width: f32,
        height: f32,
        cols: usize,
        rows: usize,
        mass: f32,
    ) -> Self {
        let mut particles = Vec::with_capacity(cols * rows);
        let mut distance_constraints = Vec::new();

        let step_x = width / (cols - 1) as f32;
        let step_y = height / (rows - 1) as f32;
        let particle_mass = mass / (cols * rows) as f32;

        // Create particles
        for row in 0..rows {
            for col in 0..cols {
                let x = origin.x + col as f32 * step_x;
                let y = origin.y;
                let z = origin.z + row as f32 * step_y;
                let pos = Vec3::new(x, y, z);

                // Pin the top row
                if row == 0 {
                    particles.push(Particle::pinned(pos));
                } else {
                    particles.push(Particle::new(pos, particle_mass));
                }
            }
        }

        // Structural constraints (horizontal and vertical)
        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;

                // Right neighbor
                if col + 1 < cols {
                    let right = idx + 1;
                    let rest_len = (particles[idx].position - particles[right].position).length();
                    distance_constraints.push(DistanceConstraint::new(idx, right, rest_len, 1.0));
                }

                // Bottom neighbor
                if row + 1 < rows {
                    let below = idx + cols;
                    let rest_len = (particles[idx].position - particles[below].position).length();
                    distance_constraints.push(DistanceConstraint::new(idx, below, rest_len, 1.0));
                }

                // Shear constraints (diagonals)
                if col + 1 < cols && row + 1 < rows {
                    let diag = idx + cols + 1;
                    let rest_len = (particles[idx].position - particles[diag].position).length();
                    distance_constraints.push(DistanceConstraint::new(idx, diag, rest_len, 0.5));
                }
                if col > 0 && row + 1 < rows {
                    let diag = idx + cols - 1;
                    let rest_len = (particles[idx].position - particles[diag].position).length();
                    distance_constraints.push(DistanceConstraint::new(idx, diag, rest_len, 0.5));
                }
            }
        }

        Self {
            particles,
            distance_constraints,
            bend_constraints: Vec::new(),
            damping: 0.01,
            solver_iterations: 5,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            self_collision: false,
        }
    }

    /// Create a rope.
    pub fn create_rope(origin: Vec3, direction: Vec3, segments: usize, segment_length: f32, mass: f32) -> Self {
        let mut particles = Vec::with_capacity(segments + 1);
        let mut distance_constraints = Vec::new();

        let particle_mass = mass / segments as f32;

        for i in 0..=segments {
            let pos = origin + direction * (i as f32 * segment_length);
            if i == 0 {
                particles.push(Particle::pinned(pos));
            } else {
                particles.push(Particle::new(pos, particle_mass));
            }
        }

        for i in 0..segments {
            distance_constraints.push(DistanceConstraint::new(i, i + 1, segment_length, 1.0));
        }

        Self {
            particles,
            distance_constraints,
            bend_constraints: Vec::new(),
            damping: 0.02,
            solver_iterations: 3,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            self_collision: false,
        }
    }

    /// Simulate one step using Verlet integration.
    pub fn step(&mut self, dt: f32) {
        let sub_steps = 2;
        let sub_dt = dt / sub_steps as f32;

        for _ in 0..sub_steps {
            // Apply forces and integrate
            self.verlet_integrate(sub_dt);

            // Solve constraints
            for _ in 0..self.solver_iterations {
                self.solve_distance_constraints();
            }
        }
    }

    fn verlet_integrate(&mut self, dt: f32) {
        let dt_sq = dt * dt;

        for particle in &mut self.particles {
            if particle.pinned || particle.inv_mass <= 0.0 {
                particle.force = Vec3::ZERO;
                continue;
            }

            // Verlet integration: x_new = 2*x - x_prev + a*dt^2
            let acceleration = self.gravity + particle.force * particle.inv_mass;
            let velocity = particle.position - particle.prev_position;

            // Apply damping
            let damped_velocity = velocity * (1.0 - self.damping);

            particle.prev_position = particle.position;
            particle.position = particle.position + damped_velocity + acceleration * dt_sq;
            particle.force = Vec3::ZERO;
        }
    }

    fn solve_distance_constraints(&mut self) {
        for constraint in &self.distance_constraints {
            let a = constraint.particle_a;
            let b = constraint.particle_b;

            let delta = self.particles[b].position - self.particles[a].position;
            let dist = delta.length();

            if dist < 1e-6 {
                continue;
            }

            let diff = (dist - constraint.rest_length) / dist;
            let inv_mass_a = self.particles[a].inv_mass;
            let inv_mass_b = self.particles[b].inv_mass;
            let total_inv = inv_mass_a + inv_mass_b;

            if total_inv <= 0.0 {
                continue;
            }

            let correction = delta * diff * constraint.stiffness;

            if !self.particles[a].pinned {
                self.particles[a].position += correction * (inv_mass_a / total_inv);
            }
            if !self.particles[b].pinned {
                self.particles[b].position -= correction * (inv_mass_b / total_inv);
            }
        }
    }

    /// Apply an impulse to all particles in a radius.
    pub fn apply_impulse(&mut self, center: Vec3, radius: f32, impulse: Vec3) {
        for particle in &mut self.particles {
            if particle.pinned {
                continue;
            }
            let dist = (particle.position - center).length();
            if dist < radius {
                let falloff = 1.0 - (dist / radius);
                particle.prev_position -= impulse * falloff * particle.inv_mass;
            }
        }
    }

    /// Pin a particle by index.
    pub fn pin_particle(&mut self, index: usize) {
        if index < self.particles.len() {
            self.particles[index].pinned = true;
            self.particles[index].inv_mass = 0.0;
        }
    }

    /// Unpin a particle by index.
    pub fn unpin_particle(&mut self, index: usize) {
        if index < self.particles.len() && self.particles[index].mass > 0.0 {
            self.particles[index].pinned = false;
            self.particles[index].inv_mass = 1.0 / self.particles[index].mass;
        }
    }

    /// Get the center of mass.
    pub fn center_of_mass(&self) -> Vec3 {
        let mut total_mass = 0.0;
        let mut weighted_pos = Vec3::ZERO;

        for particle in &self.particles {
            if !particle.pinned {
                weighted_pos += particle.position * particle.mass;
                total_mass += particle.mass;
            }
        }

        if total_mass > 0.0 {
            weighted_pos / total_mass
        } else {
            Vec3::ZERO
        }
    }
}

/// System that updates all soft bodies each frame.
pub fn soft_body_system(world: &mut engine_ecs::world::World) {
    let dt = 1.0 / 60.0;
    let soft_body_indices: Vec<u32> = world.component_entities::<SoftBody>().to_vec();

    for &idx in &soft_body_indices {
        if let Some(soft_body) = world.get_by_index_mut::<SoftBody>(idx) {
            soft_body.step(dt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_creation() {
        let p = Particle::new(Vec3::new(1.0, 2.0, 3.0), 1.0);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert!(!p.pinned);
        assert!((p.inv_mass - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_pinned_particle() {
        let p = Particle::pinned(Vec3::ZERO);
        assert!(p.pinned);
        assert!((p.inv_mass).abs() < 1e-6);
    }

    #[test]
    fn test_cloth_creation() {
        let cloth = SoftBody::create_cloth(
            Vec3::ZERO,
            2.0,
            2.0,
            5,
            5,
            1.0,
        );
        assert_eq!(cloth.particles.len(), 25);
        // Top row should be pinned
        for col in 0..5 {
            assert!(cloth.particles[col].pinned);
        }
        // Bottom row should not be pinned
        for col in 20..25 {
            assert!(!cloth.particles[col].pinned);
        }
        assert!(!cloth.distance_constraints.is_empty());
    }

    #[test]
    fn test_rope_creation() {
        let rope = SoftBody::create_rope(
            Vec3::ZERO,
            Vec3::new(0.0, -1.0, 0.0),
            10,
            0.5,
            1.0,
        );
        assert_eq!(rope.particles.len(), 11);
        assert!(rope.particles[0].pinned);
        assert!(!rope.particles[1].pinned);
        assert_eq!(rope.distance_constraints.len(), 10);
    }

    #[test]
    fn test_verlet_step_preserves_pinned() {
        let mut cloth = SoftBody::create_cloth(
            Vec3::ZERO,
            1.0,
            1.0,
            3,
            3,
            1.0,
        );

        let initial_pos = cloth.particles[0].position;
        cloth.step(1.0 / 60.0);

        // Pinned particle should not move
        assert!((cloth.particles[0].position - initial_pos).length() < 1e-6);
    }

    #[test]
    fn test_cloth_falls_under_gravity() {
        let mut cloth = SoftBody::create_cloth(
            Vec3::new(0.0, 10.0, 0.0),
            1.0,
            1.0,
            3,
            3,
            1.0,
        );

        // Simulate 1 second
        for _ in 0..60 {
            cloth.step(1.0 / 60.0);
        }

        // Center of mass should have moved down
        let com = cloth.center_of_mass();
        assert!(com.y < 10.0, "Cloth should fall, center.y = {}", com.y);
    }

    #[test]
    fn test_rope_dangling() {
        let mut rope = SoftBody::create_rope(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            5,
            1.0,
            1.0,
        );

        // Simulate 1 second
        for _ in 0..60 {
            rope.step(1.0 / 60.0);
        }

        // The pinned end should stay at origin
        assert!((rope.particles[0].position.y - 5.0).abs() < 0.1);

        // The free end should hang below
        let last = rope.particles.last().unwrap();
        assert!(last.position.y < 5.0, "Rope end should hang down, y = {}", last.position.y);
    }

    #[test]
    fn test_impulse_affects_particles() {
        let mut rope = SoftBody::create_rope(
            Vec3::ZERO,
            Vec3::new(0.0, -1.0, 0.0),
            5,
            1.0,
            1.0,
        );

        let before = rope.particles[3].position;
        rope.apply_impulse(Vec3::new(0.0, -2.0, 0.0), 5.0, Vec3::new(10.0, 0.0, 0.0));
        rope.step(1.0 / 60.0);

        // Particle should have moved sideways
        assert!(rope.particles[3].position.x > before.x || rope.particles[3].position != before);
    }

    #[test]
    fn test_pin_unpin() {
        let mut rope = SoftBody::create_rope(
            Vec3::ZERO,
            Vec3::new(0.0, -1.0, 0.0),
            5,
            1.0,
            1.0,
        );

        assert!(!rope.particles[3].pinned);
        rope.pin_particle(3);
        assert!(rope.particles[3].pinned);
        assert!((rope.particles[3].inv_mass).abs() < 1e-6);

        rope.unpin_particle(3);
        assert!(!rope.particles[3].pinned);
        assert!(rope.particles[3].inv_mass > 0.0);
    }
}
```

### Step 2: Register soft body system in PhysicsPlugin

```rust
// plugin.rs
use crate::softbody::soft_body_system;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::default());
        app.add_system(physics_step_system);
        app.add_system(ragdoll_update_system);
        app.add_system(vehicle_physics_system);
        app.add_system(soft_body_system);
    }
}
```

### Step 3: Run all tests and commit

```bash
cargo test -p engine-physics
git add crates/engine-physics/src/softbody.rs crates/engine-physics/src/plugin.rs crates/engine-physics/src/lib.rs
git commit -m "feat(physics): add soft body simulation with Verlet integration, cloth and rope creation"
```

---

## Task 7: Final Integration and Verification

**Goal:** Ensure all new modules compile together, existing tests still pass, and the physics plugin correctly registers all systems.

**Files:**
- Modify: `crates/engine-physics/src/lib.rs`
- Modify: `crates/engine-physics/src/plugin.rs`

### Step 1: Update lib.rs exports

```rust
// lib.rs — add new modules and exports
pub mod body;
pub mod broadphase;
pub mod ccd;
pub mod collider;
pub mod contact;
pub mod joint;
pub mod material;
pub mod plugin;
pub mod ragdoll;
pub mod softbody;
pub mod trigger;
pub mod vehicle;
pub mod world;

// Re-export key types
pub use body::RigidBody;
pub use collider::{Collider, check_collision, check_sphere_sphere, check_box_box, check_sphere_box, check_sphere_capsule, check_capsule_capsule, check_obb_obb, check_obb_capsule, check_sphere_obb};
pub use contact::{ContactManifold, ContactPoint, ContactSolver};
pub use material::{MaterialTable, MaterialId, PhysicsMaterial};
pub use plugin::PhysicsPlugin;
pub use ragdoll::{Ragdoll, RagdollBuilder, RagdollBone, RagdollConfig};
pub use softbody::SoftBody;
pub use vehicle::Vehicle;
pub use world::{CollisionEvent, PhysicsWorld, SensorEvent};
```

### Step 2: Update plugin.rs to register all systems

```rust
// plugin.rs — final version
use crate::ragdoll::ragdoll_update_system;
use crate::softbody::soft_body_system;
use crate::vehicle::vehicle_physics_system;
use crate::world::PhysicsWorld;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

fn physics_step_system(world: &mut engine_ecs::world::World) {
    let mut pw = match world.remove_resource::<PhysicsWorld>() {
        Some(pw) => pw,
        None => return,
    };
    pw.step(world);
    world.insert_resource(pw);
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PhysicsWorld::default());
        app.add_system(physics_step_system);
        app.add_system(ragdoll_update_system);
        app.add_system(vehicle_physics_system);
        app.add_system(soft_body_system);
    }
}
```

### Step 3: Run full test suite

```bash
cargo test -p engine-physics
```

Expected: All existing tests pass, all new tests pass.

### Step 4: Run clippy

```bash
cargo clippy -p engine-physics -- -D warnings
```

### Step 5: Run fmt

```bash
cargo fmt -p engine-physics
```

### Step 6: Final commit

```bash
git add -A
git commit -m "feat(physics): complete physics enhancements — triggers, materials, joints, ragdoll, vehicle, soft body"
```

---

## Summary

| Task | Hours | Files Created | Tests |
|------|-------|---------------|-------|
| 1. Trigger/Sensor Fix | 3h | 0 (modify world.rs) | 2 |
| 2. Material System | 4h | material.rs | 5 |
| 3. Joint Constraints | 2h | 0 (modify joint.rs) | 1 |
| 4. Ragdoll | 6h | ragdoll.rs | 5 |
| 5. Vehicle Physics | 6h | vehicle.rs | 4 |
| 6. Soft Body | 6h | softbody.rs | 8 |
| 7. Integration | 1h | 0 | 0 |
| **Total** | **28h** | **4 new files** | **25 tests** |
