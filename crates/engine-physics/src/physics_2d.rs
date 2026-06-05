//! Lightweight 2D physics for platformer-style games.
//!
//! Provides AABB collision, simple gravity, ground detection, and trigger support.
//! Designed for tile-based 2D games — no rotation, no circle collision, no constraints.

use engine_math::Vec2;

/// Axis-aligned bounding box in 2D.
#[derive(Debug, Clone, Copy)]
pub struct AABB2D {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB2D {
    /// Create a new AABB from min and max corners.
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Create an AABB centered at a point with given half-extents.
    pub fn from_center(center: Vec2, half_extents: Vec2) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Check overlap with another AABB.
    pub fn overlaps(&self, other: &AABB2D) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Compute the overlap (penetration) between two AABBs.
    /// Returns None if no overlap.
    pub fn intersection(&self, other: &AABB2D) -> Option<(Vec2, f32)> {
        let overlap_x = (self.max.x - other.min.x).min(other.max.x - self.min.x);
        let overlap_y = (self.max.y - other.min.y).min(other.max.y - self.min.y);

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            return None;
        }

        // Minimum separation axis
        if overlap_x < overlap_y {
            let sign = if self.min.x + self.max.x < other.min.x + other.max.x {
                -1.0
            } else {
                1.0
            };
            Some((Vec2::new(sign, 0.0), overlap_x))
        } else {
            let sign = if self.min.y + self.max.y < other.min.y + other.max.y {
                -1.0
            } else {
                1.0
            };
            Some((Vec2::new(0.0, sign), overlap_y))
        }
    }

    /// Width of the AABB.
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height of the AABB.
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Center point.
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

/// Body type for 2D physics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType2D {
    /// Immovable — ground, walls.
    Static,
    /// Moved by code only — moving platforms, doors.
    Kinematic,
    /// Fully simulated — player, enemies.
    Dynamic,
}

/// 2D rigid body component.
#[derive(Debug, Clone)]
pub struct RigidBody2D {
    pub body_type: BodyType2D,
    pub velocity: Vec2,
    pub gravity_scale: f32,
    pub grounded: bool,
    pub linear_damping: f32,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            body_type: BodyType2D::Dynamic,
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }
}

impl RigidBody2D {
    pub fn new_dynamic() -> Self {
        Self::default()
    }

    pub fn new_static() -> Self {
        Self {
            body_type: BodyType2D::Static,
            velocity: Vec2::ZERO,
            gravity_scale: 0.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }

    pub fn new_kinematic() -> Self {
        Self {
            body_type: BodyType2D::Kinematic,
            velocity: Vec2::ZERO,
            gravity_scale: 0.0,
            grounded: false,
            linear_damping: 0.0,
        }
    }
}

/// 2D collider component.
#[derive(Debug, Clone)]
pub struct Collider2D {
    /// Local offset from entity position.
    pub offset: Vec2,
    /// Half-extents of the AABB (half-width, half-height).
    pub half_extents: Vec2,
    pub friction: f32,
    pub restitution: f32,
    pub is_trigger: bool,
    pub collision_layers: u32,
    pub collision_mask: u32,
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            half_extents: Vec2::new(0.5, 0.5),
            friction: 0.5,
            restitution: 0.0,
            is_trigger: false,
            collision_layers: 0xFFFF_FFFF,
            collision_mask: 0xFFFF_FFFF,
        }
    }
}

impl Collider2D {
    /// Create a solid AABB collider with given half-extents.
    pub fn aabb(half_x: f32, half_y: f32) -> Self {
        Self {
            half_extents: Vec2::new(half_x, half_y),
            ..Default::default()
        }
    }

    /// Create a trigger (sensor) AABB collider.
    pub fn trigger(half_x: f32, half_y: f32) -> Self {
        Self {
            half_extents: Vec2::new(half_x, half_y),
            is_trigger: true,
            ..Default::default()
        }
    }

    /// Compute the world-space AABB given an entity position.
    pub fn world_aabb(&self, position: Vec2) -> AABB2D {
        let center = position + self.offset;
        AABB2D::from_center(center, self.half_extents)
    }
}

/// Contact result from 2D collision detection.
#[derive(Debug, Clone)]
pub struct Contact2D {
    pub entity_a: u32,
    pub entity_b: u32,
    pub normal: Vec2,
    pub penetration: f32,
    pub is_trigger: bool,
}

/// 2D physics world — gravity, integration, collision detection & resolution.
#[derive(Debug, Clone)]
pub struct PhysicsWorld2D {
    pub gravity: Vec2,
    pub contacts: Vec<Contact2D>,
}

impl Default for PhysicsWorld2D {
    fn default() -> Self {
        Self {
            gravity: Vec2::new(0.0, -9.81),
            contacts: Vec::new(),
        }
    }
}

impl PhysicsWorld2D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Step the 2D physics simulation.
    ///
    /// 1. Apply gravity to dynamic bodies
    /// 2. Integrate velocities into positions
    /// 3. Detect AABB collisions
    /// 4. Resolve solid collisions (push out + velocity correction)
    /// 5. Record trigger overlaps
    pub fn step(&mut self, world: &mut engine_ecs::world::World, dt: f32) {
        self.contacts.clear();

        // Phase 1: Apply gravity & integrate
        let entities: Vec<u32> = world.component_entities::<RigidBody2D>();
        for &eid in &entities {
            let body = world.get_by_index_mut::<RigidBody2D>(eid).unwrap();
            if body.body_type != BodyType2D::Dynamic {
                continue;
            }
            body.velocity += self.gravity * body.gravity_scale * dt;
            body.velocity *= 1.0 - body.linear_damping * dt;
        }

        // Phase 2: Move entities
        let entities: Vec<u32> = world.component_entities::<RigidBody2D>();
        for &eid in &entities {
            let body = world.get_by_index_mut::<RigidBody2D>(eid).unwrap();
            if body.body_type == BodyType2D::Static {
                continue;
            }
            let vel = body.velocity;
            // Read position from Transform
            if let Some(transform) =
                world.get_by_index_mut::<engine_core::transform::Transform>(eid)
            {
                transform.position.x += vel.x * dt;
                transform.position.y += vel.y * dt;
            }
        }

        // Phase 3: Detect & resolve collisions
        self.detect_and_resolve(world);
    }

    fn detect_and_resolve(&mut self, world: &mut engine_ecs::world::World) {
        // Reset grounded for all dynamic bodies
        let entities: Vec<u32> = world.component_entities::<RigidBody2D>();
        for &eid in &entities {
            if let Some(body) = world.get_by_index_mut::<RigidBody2D>(eid)
                && body.body_type == BodyType2D::Dynamic
            {
                body.grounded = false;
            }
        }

        // Collect all colliders with positions
        let mut colliders: Vec<(u32, Vec2, Collider2D, BodyType2D)> = Vec::new();
        let entities: Vec<u32> = world.component_entities::<Collider2D>();
        for &eid in &entities {
            let collider = world.get_by_index::<Collider2D>(eid).unwrap();
            let body_type = world
                .get_by_index::<RigidBody2D>(eid)
                .map(|b| b.body_type)
                .unwrap_or(BodyType2D::Static);
            if let Some(transform) = world.get_by_index::<engine_core::transform::Transform>(eid) {
                let pos = Vec2::new(transform.position.x, transform.position.y);
                colliders.push((eid, pos, collider.clone(), body_type));
            }
        }

        // Broadphase: check all pairs
        for i in 0..colliders.len() {
            for j in (i + 1)..colliders.len() {
                let (eid_a, pos_a, col_a, type_a) = &colliders[i];
                let (eid_b, pos_b, col_b, type_b) = &colliders[j];

                // Skip if both static
                if *type_a == BodyType2D::Static && *type_b == BodyType2D::Static {
                    continue;
                }

                // Layer check
                if (col_a.collision_layers & col_b.collision_mask) == 0
                    || (col_b.collision_layers & col_a.collision_mask) == 0
                {
                    continue;
                }

                let aabb_a = col_a.world_aabb(*pos_a);
                let aabb_b = col_b.world_aabb(*pos_b);

                if let Some((normal, pen)) = aabb_a.intersection(&aabb_b) {
                    let is_trigger = col_a.is_trigger || col_b.is_trigger;

                    self.contacts.push(Contact2D {
                        entity_a: *eid_a,
                        entity_b: *eid_b,
                        normal,
                        penetration: pen,
                        is_trigger,
                    });

                    // Resolve solid collisions
                    if !is_trigger {
                        self.resolve_collision(world, *eid_a, *eid_b, type_a, type_b, normal, pen);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_collision(
        &self,
        world: &mut engine_ecs::world::World,
        eid_a: u32,
        eid_b: u32,
        type_a: &BodyType2D,
        type_b: &BodyType2D,
        normal: Vec2,
        pen: f32,
    ) {
        // Determine which entity to push (dynamic vs static/kinematic)
        let (push_eid, _other_eid, push_normal) = match (type_a, type_b) {
            (BodyType2D::Dynamic, BodyType2D::Static | BodyType2D::Kinematic) => {
                (eid_a, eid_b, normal)
            }
            (BodyType2D::Static | BodyType2D::Kinematic, BodyType2D::Dynamic) => {
                (eid_b, eid_a, -normal)
            }
            (BodyType2D::Dynamic, BodyType2D::Dynamic) => {
                // Both dynamic: push first entity by half
                (eid_a, eid_b, normal)
            }
            _ => return,
        };

        // Push out
        if let Some(transform) =
            world.get_by_index_mut::<engine_core::transform::Transform>(push_eid)
        {
            transform.position.x += push_normal.x * pen;
            transform.position.y += push_normal.y * pen;
        }

        // Velocity correction: cancel velocity into the collision surface
        if let Some(body) = world.get_by_index_mut::<RigidBody2D>(push_eid) {
            let vel_dot_normal = body.velocity.x * push_normal.x + body.velocity.y * push_normal.y;
            if vel_dot_normal < 0.0 {
                body.velocity.x -= push_normal.x * vel_dot_normal;
                body.velocity.y -= push_normal.y * vel_dot_normal;
            }

            // Ground detection: if normal points upward, entity is grounded
            if push_normal.y > 0.5 {
                body.grounded = true;
            }
        }
    }
}
