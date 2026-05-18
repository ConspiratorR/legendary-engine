//! Physics world for managing and simulating physics.
use std::collections::HashMap;
use engine_math::Vec3;
use crate::body::{RigidBody, BodyType};
use crate::collider::{Collider, CollisionInfo, check_sphere_sphere};
use engine_ecs::world::World;

/// Physics world configuration.
#[derive(Debug, Clone)]
pub struct PhysicsWorld {
    pub gravity: Vec3,
    pub delta_time: f32,
    pub sub_steps: u32,
    pub body_count: usize,
    pub collider_count: usize,
    /// Current frame collisions
    pub collisions: Vec<(u64, u64, CollisionInfo)>,
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

        for _ in 0..self.sub_steps {
            self.apply_forces(world, dt);
            self.integrate_bodies(world, dt);
            self.detect_collisions(world);
            self.resolve_collisions(world);
        }
    }

    /// Apply forces to all dynamic bodies.
    fn apply_forces(&mut self, world: &mut World, dt: f32) {
        // This would iterate over entities with RigidBody and Position components
        // For now, we'll keep it simple for demonstration
    }

    /// Integrate body velocities and positions.
    fn integrate_bodies(&mut self, world: &mut World, dt: f32) {
        // ECS integration would happen here
        // For now, just a placeholder
    }

    /// Detect collisions between colliders.
    fn detect_collisions(&mut self, world: &World) {
        // Clear previous collisions
        self.collisions.clear();
        
        // For demonstration purposes, we're keeping it simple
        // In a real implementation, we would use ECS queries and broad/narrow phase
    }

    /// Resolve detected collisions.
    fn resolve_collisions(&mut self, world: &mut World) {
        // Iterate through detected collisions and resolve them
        for (_, _, collision) in &self.collisions {
            // Apply impulse response here
            println!("Collision resolved: normal={:?}, depth={}", collision.normal, collision.depth);
        }
    }
}
