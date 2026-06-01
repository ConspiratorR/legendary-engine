//! Physics world for managing and simulating physics.
use crate::body::{BodyType, RigidBody};
use crate::collider::{Collider, CollisionInfo, check_collision};
use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_math::{EulerRot, Quat, Vec3};

/// Physics world configuration.
#[derive(Debug, Clone)]
pub struct PhysicsWorld {
    pub gravity: Vec3,
    pub delta_time: f32,
    pub sub_steps: u32,
    pub body_count: usize,
    pub collider_count: usize,
    /// Current frame collisions (entity_index_a, entity_index_b, info)
    pub collisions: Vec<(u32, u32, CollisionInfo)>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            delta_time: 1.0 / 60.0,
            sub_steps: 4,
            body_count: 0,
            collider_count: 0,
            collisions: Vec::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = gravity;
    }

    /// Step the physics simulation.
    pub fn step(&mut self, world: &mut World) {
        let dt = self.delta_time / self.sub_steps as f32;

        // Update counts
        self.body_count = world.component_entities::<RigidBody>().len();
        self.collider_count = world.component_entities::<Collider>().len();

        // Apply forces once per frame (not per sub-step)
        self.apply_forces(world, self.delta_time);

        // Sub-step: integrate, detect, resolve
        for _ in 0..self.sub_steps {
            self.integrate_bodies(world, dt);
            self.detect_collisions(world);
            self.resolve_collisions(world);
        }
    }

    /// Apply gravity to all dynamic bodies.
    fn apply_forces(&self, world: &mut World, dt: f32) {
        let indices = world.component_entities::<RigidBody>();
        for &idx in &indices {
            if let Some(body) = world.get_by_index::<RigidBody>(idx) {
                if body.body_type != BodyType::Dynamic || body.is_sleeping {
                    continue;
                }
                let gravity_vel = self.gravity * body.gravity_scale * dt;
                let damping = 1.0 - body.linear_damping * dt;
                let damping = damping.max(0.0);

                if let Some(body) = world.get_by_index_mut::<RigidBody>(idx) {
                    body.linear_velocity += gravity_vel;
                    body.linear_velocity *= damping;
                    body.angular_velocity *= 1.0 - body.angular_damping * dt;
                }
            }
        }
    }

    /// Semi-implicit Euler integration of body positions.
    fn integrate_bodies(&self, world: &mut World, dt: f32) {
        let indices = world.component_entities::<RigidBody>();

        // Phase 1: compute new positions
        let mut updates: Vec<(u32, Vec3)> = Vec::new();
        for &idx in &indices {
            if let Some(body) = world.get_by_index::<RigidBody>(idx) {
                if body.body_type != BodyType::Dynamic || body.is_sleeping {
                    continue;
                }
                let vel = body.linear_velocity;
                if let Some(transform) = world.get_by_index::<Transform>(idx) {
                    let new_pos = transform.position + vel * dt;
                    updates.push((idx, new_pos));
                }
            }
        }

        // Phase 2: apply position updates
        for (idx, new_pos) in updates {
            if let Some(transform) = world.get_by_index_mut::<Transform>(idx) {
                transform.position = new_pos;
            }
        }
    }

    /// Detect collisions between all collider pairs (brute-force for now).
    fn detect_collisions(&mut self, world: &World) {
        self.collisions.clear();

        let collider_indices = world.component_entities::<Collider>();

        // Build a list of entities that have both Transform and Collider
        let entities: Vec<u32> = collider_indices
            .iter()
            .copied()
            .filter(|idx| world.get_by_index::<Transform>(*idx).is_some())
            .collect();

        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let idx_a = entities[i];
                let idx_b = entities[j];

                let transform_a = world.get_by_index::<Transform>(idx_a).unwrap();
                let transform_b = world.get_by_index::<Transform>(idx_b).unwrap();
                let collider_a = world.get_by_index::<Collider>(idx_a).unwrap();
                let collider_b = world.get_by_index::<Collider>(idx_b).unwrap();

                if collider_a.is_sensor || collider_b.is_sensor {
                    continue;
                }

                let rot_a = Quat::from_euler(
                    EulerRot::XYZ,
                    transform_a.rotation.x,
                    transform_a.rotation.y,
                    transform_a.rotation.z,
                );
                let rot_b = Quat::from_euler(
                    EulerRot::XYZ,
                    transform_b.rotation.x,
                    transform_b.rotation.y,
                    transform_b.rotation.z,
                );

                if let Some(mut info) = check_collision(
                    transform_a.position,
                    rot_a,
                    collider_a,
                    transform_b.position,
                    rot_b,
                    collider_b,
                ) {
                    info.other_entity = idx_b as u64;
                    self.collisions.push((idx_a, idx_b, info));
                }
            }
        }
    }

    /// Resolve detected collisions using impulse-based response.
    fn resolve_collisions(&mut self, world: &mut World) {
        // Read sub_steps to compute dt for bias
        let dt = self.delta_time / self.sub_steps as f32;

        for &(idx_a, idx_b, ref collision) in &self.collisions {
            // Read restitution
            let restitution_a = world
                .get_by_index::<Collider>(idx_a)
                .map_or(0.3, |c| c.restitution);
            let restitution_b = world
                .get_by_index::<Collider>(idx_b)
                .map_or(0.3, |c| c.restitution);
            let restitution = (restitution_a + restitution_b) * 0.5;

            // Read friction
            let friction_a = world
                .get_by_index::<Collider>(idx_a)
                .map_or(0.5, |c| c.friction);
            let friction_b = world
                .get_by_index::<Collider>(idx_b)
                .map_or(0.5, |c| c.friction);
            let friction = (friction_a + friction_b) * 0.5;

            // Read velocities and masses
            let vel_a = world
                .get_by_index::<RigidBody>(idx_a)
                .map_or(Vec3::ZERO, |b| b.linear_velocity);
            let vel_b = world
                .get_by_index::<RigidBody>(idx_b)
                .map_or(Vec3::ZERO, |b| b.linear_velocity);

            let is_dynamic_a = world
                .get_by_index::<RigidBody>(idx_a)
                .is_some_and(|b| b.body_type == BodyType::Dynamic);
            let is_dynamic_b = world
                .get_by_index::<RigidBody>(idx_b)
                .is_some_and(|b| b.body_type == BodyType::Dynamic);

            let inv_mass_a = if is_dynamic_a {
                world
                    .get_by_index::<RigidBody>(idx_a)
                    .map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 1.0 })
            } else {
                0.0
            };

            let inv_mass_b = if is_dynamic_b {
                world
                    .get_by_index::<RigidBody>(idx_b)
                    .map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 1.0 })
            } else {
                0.0
            };

            let total_inv_mass = inv_mass_a + inv_mass_b;
            if total_inv_mass <= 0.0 {
                continue;
            }

            let relative_vel = vel_b - vel_a;
            let vel_along_normal = relative_vel.dot(collision.normal);

            // Baumgarte stabilization: bias to push overlapping bodies apart
            let baumgarte = 0.2;
            let slop = 0.005;
            let bias = baumgarte * (collision.depth - slop).max(0.0) / dt;

            // Skip if already separating fast enough (bias handles resting contacts)
            if vel_along_normal > bias && vel_along_normal > 0.0 {
                continue;
            }

            // Impulse magnitude (including bias for position correction)
            let j = -(vel_along_normal - bias) / total_inv_mass;
            // Clamp: at minimum, apply restitution for separating velocity
            let j_restitution = -(1.0 + restitution) * vel_along_normal / total_inv_mass;
            let j = j.max(j_restitution);
            let impulse = collision.normal * j;

            // Apply impulses
            if is_dynamic_a && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_a) {
                body.linear_velocity -= impulse * inv_mass_a;
            }
            if is_dynamic_b && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_b) {
                body.linear_velocity += impulse * inv_mass_b;
            }

            // Additional positional correction for deep penetrations
            let percent = 0.4;
            let correction =
                (collision.depth - slop).max(0.0) / total_inv_mass * percent * collision.normal;

            if is_dynamic_a && let Some(transform) = world.get_by_index_mut::<Transform>(idx_a) {
                transform.position -= correction * inv_mass_a;
            }
            if is_dynamic_b && let Some(transform) = world.get_by_index_mut::<Transform>(idx_b) {
                transform.position += correction * inv_mass_b;
            }

            // Friction impulse (tangential)
            let tangent = relative_vel - collision.normal * vel_along_normal;
            let tangent_len_sq = tangent.length_squared();
            if tangent_len_sq > f32::EPSILON {
                let tangent = tangent / tangent_len_sq.sqrt();
                let jt = -relative_vel.dot(tangent) / total_inv_mass;
                let jt = jt.clamp(-j * friction, j * friction);
                let friction_impulse = tangent * jt;

                if is_dynamic_a && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_a) {
                    body.linear_velocity -= friction_impulse * inv_mass_a;
                }
                if is_dynamic_b && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_b) {
                    body.linear_velocity += friction_impulse * inv_mass_b;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collider::{check_box_box, check_sphere_sphere};

    #[test]
    fn test_physics_world_default() {
        let pw = PhysicsWorld::new();
        assert_eq!(pw.gravity, Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(pw.sub_steps, 4);
    }

    #[test]
    fn test_gravity_applied() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Transform::from_xyz(0.0, 10.0, 0.0));
        world.add_component(e, RigidBody::new_dynamic());
        world.add_component(e, Collider::cuboid(0.5, 0.5, 0.5));

        let mut pw = PhysicsWorld::new();
        pw.sub_steps = 1;
        pw.delta_time = 1.0 / 60.0;

        pw.step(&mut world);

        let body = world.get_by_index::<RigidBody>(e.index()).unwrap();
        // Velocity should have decreased (gravity is negative Y)
        assert!(body.linear_velocity.y < 0.0);
    }

    #[test]
    fn test_static_body_does_not_move() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Transform::from_xyz(0.0, 0.0, 0.0));
        world.add_component(e, RigidBody::new_static());
        world.add_component(e, Collider::cuboid(0.5, 0.5, 0.5));

        let mut pw = PhysicsWorld::new();
        pw.sub_steps = 1;

        pw.step(&mut world);

        let transform = world.get_by_index::<Transform>(e.index()).unwrap();
        assert_eq!(transform.position, Vec3::ZERO);
    }

    #[test]
    fn test_sphere_sphere_collision() {
        let pos1 = Vec3::new(0.0, 0.0, 0.0);
        let pos2 = Vec3::new(1.5, 0.0, 0.0);
        let result = check_sphere_sphere(pos1, 1.0, pos2, 1.0);
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
        // Normal should point from pos1 toward pos2
        assert!(info.normal.x > 0.0);
    }

    #[test]
    fn test_box_box_collision() {
        let pos1 = Vec3::new(0.0, 0.0, 0.0);
        let pos2 = Vec3::new(1.5, 0.0, 0.0);
        let half1 = Vec3::new(1.0, 1.0, 1.0);
        let half2 = Vec3::new(1.0, 1.0, 1.0);
        let result = check_box_box(pos1, half1, pos2, half2);
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    #[test]
    fn test_collision_resolution() {
        let mut world = World::new();

        // Static floor
        let floor = world.spawn();
        world.add_component(floor, Transform::from_xyz(0.0, -0.5, 0.0));
        world.add_component(floor, RigidBody::new_static());
        world.add_component(floor, Collider::cuboid(50.0, 0.5, 50.0));

        // Dynamic sphere falling onto floor
        let sphere = world.spawn();
        world.add_component(sphere, Transform::from_xyz(0.0, 0.3, 0.0));
        let mut body = RigidBody::new_dynamic();
        body.linear_velocity = Vec3::new(0.0, -5.0, 0.0);
        body.mass = 1.0;
        world.add_component(sphere, body);
        world.add_component(sphere, Collider::sphere(0.5));

        let mut pw = PhysicsWorld::new();
        pw.sub_steps = 4;
        pw.delta_time = 1.0 / 60.0;

        // Run several frames
        for _ in 0..10 {
            pw.step(&mut world);
        }

        // Sphere should have bounced (velocity.y should be positive after hitting floor)
        let body = world.get_by_index::<RigidBody>(sphere.index()).unwrap();
        assert!(
            body.linear_velocity.y > 0.0,
            "Sphere should bounce, got velocity.y = {}",
            body.linear_velocity.y
        );
    }
}
