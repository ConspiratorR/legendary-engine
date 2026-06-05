//! Physics world for managing and simulating physics.
use crate::body::{BodyType, RigidBody};
use crate::broadphase::{BroadphaseEntry, SpatialHashBroadphase};
use crate::ccd::{CcdBody, sweep_sphere_aabb, sweep_sphere_sphere};
use crate::collider::{Collider, ColliderShape, CollisionInfo, check_collision};
use crate::contact::{ContactManifold, ContactPoint, ContactSolver};
use crate::joint::JointSolver;
use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_math::{EulerRot, Quat, Vec3};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicPtr;

/// Union-Find data structure for grouping collision pairs into independent islands.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new() -> Self {
        Self {
            parent: Vec::new(),
            rank: Vec::new(),
        }
    }

    fn ensure(&mut self, idx: usize) {
        while self.parent.len() <= idx {
            let n = self.parent.len();
            self.parent.push(n);
            self.rank.push(0);
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        self.ensure(x);
        // Path compression
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        // Union by rank
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
        } else {
            self.parent[rb] = ra;
            self.rank[ra] += 1;
        }
    }
}

/// A collision event for gameplay systems.
#[derive(Debug, Clone)]
pub struct CollisionEvent {
    pub entity_a: u32,
    pub entity_b: u32,
    pub normal: Vec3,
    pub depth: f32,
    pub point: Vec3,
    /// Whether this is a new collision (enter) or ongoing.
    pub is_enter: bool,
}

/// A sensor overlap event.
#[derive(Debug, Clone)]
pub struct SensorEvent {
    pub sensor_entity: u32,
    pub other_entity: u32,
    pub overlapping: bool,
    /// Whether this is a new overlap (enter) or continuing.
    pub is_enter: bool,
}

/// Key for identifying a collision pair (order-independent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PairKey(u32, u32);

impl PairKey {
    fn new(a: u32, b: u32) -> Self {
        if a < b { Self(a, b) } else { Self(b, a) }
    }
}

/// Physics world configuration.
pub struct PhysicsWorld {
    pub gravity: Vec3,
    pub delta_time: f32,
    pub sub_steps: u32,
    pub body_count: usize,
    pub collider_count: usize,
    /// Current frame collisions (entity_index_a, entity_index_b, info)
    pub collisions: Vec<(u32, u32, CollisionInfo)>,
    /// Collision events emitted this frame (for gameplay systems to read).
    pub collision_events: Vec<CollisionEvent>,
    /// Sensor overlap events emitted this frame.
    pub sensor_events: Vec<SensorEvent>,
    broadphase: SpatialHashBroadphase,
    /// Cached contact manifolds for warm-starting (pair key → manifold).
    contact_cache: HashMap<PairKey, ContactManifold>,
    /// Contact solver for iterative constraint solving.
    contact_solver: ContactSolver,
    /// Set of pairs that were colliding last frame (for enter/exit tracking).
    previous_collision_pairs: HashSet<PairKey>,
    /// Set of sensor pairs that were overlapping last frame.
    previous_sensor_pairs: HashSet<PairKey>,
    /// Joint solver for hinge, ball-socket, and spring constraints.
    pub joint_solver: JointSolver,
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
            collision_events: Vec::new(),
            sensor_events: Vec::new(),
            broadphase: SpatialHashBroadphase::new(2.0),
            contact_cache: HashMap::new(),
            contact_solver: ContactSolver::new(),
            previous_collision_pairs: HashSet::new(),
            previous_sensor_pairs: HashSet::new(),
            joint_solver: JointSolver::new(),
        }
    }
}

impl PhysicsWorld {
    /// Create a new physics world with default settings (gravity -9.81 Y, 60 Hz, 4 sub-steps).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the gravity vector applied to all dynamic bodies each frame.
    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = gravity;
    }

    /// Set the broadphase cell size. Should be >= the largest collider diameter.
    pub fn set_broadphase_cell_size(&mut self, size: f32) {
        self.broadphase.set_cell_size(size);
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
            self.wake_colliding_bodies(world);
            self.resolve_collisions_with_warm_start(world);
            self.joint_solver.solve_constraints(world, dt);
        }

        // Update sleep states once per frame
        self.update_sleep_states(world, self.delta_time);
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

    /// Semi-implicit Euler integration of body positions with CCD support.
    fn integrate_bodies(&self, world: &mut World, dt: f32) {
        let indices = world.component_entities::<RigidBody>();

        // Phase 1: compute new positions (with CCD if enabled)
        let mut updates: Vec<(u32, Vec3)> = Vec::new();
        for &idx in &indices {
            if let Some(body) = world.get_by_index::<RigidBody>(idx) {
                if body.body_type != BodyType::Dynamic || body.is_sleeping {
                    continue;
                }
                let vel = body.linear_velocity;
                if let Some(transform) = world.get_by_index::<Transform>(idx) {
                    let desired_pos = transform.position + vel * dt;

                    // CCD: sweep if body has CcdBody and speed exceeds threshold
                    let ccd = world.get_by_index::<CcdBody>(idx);
                    let has_ccd = ccd.is_some_and(|c| c.enabled);
                    let threshold = ccd.map_or(1.0, |c| c.activation_threshold);
                    let speed_sq = vel.length_squared();

                    if has_ccd && speed_sq > threshold * threshold {
                        let safe_pos = self.ccd_sweep(world, idx, transform.position, desired_pos);
                        updates.push((idx, safe_pos));
                    } else {
                        updates.push((idx, desired_pos));
                    }
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

    /// CCD sweep: find the safest position along the trajectory by testing
    /// against all static colliders.
    fn ccd_sweep(&self, world: &World, entity_idx: u32, start: Vec3, end: Vec3) -> Vec3 {
        let collider = match world.get_by_index::<Collider>(entity_idx) {
            Some(c) => c.clone(),
            None => return end,
        };

        let direction = end - start;
        let dist = direction.length();
        if dist < f32::EPSILON {
            return end;
        }

        let radius = collider.shape.get_bounding_sphere();
        let mut earliest_toi = 1.0f32;

        // Test against all static colliders
        let all_indices = world.component_entities::<Collider>();
        for &other_idx in &all_indices {
            if other_idx == entity_idx {
                continue;
            }
            let other_body = world.get_by_index::<RigidBody>(other_idx);
            let is_static =
                other_body.is_none() || other_body.is_some_and(|b| b.body_type == BodyType::Static);
            if !is_static {
                continue;
            }

            let other_collider = match world.get_by_index::<Collider>(other_idx) {
                Some(c) => c,
                None => continue,
            };
            let other_transform = match world.get_by_index::<Transform>(other_idx) {
                Some(t) => t,
                None => continue,
            };

            let other_radius = other_collider.shape.get_bounding_sphere();

            let result = match &other_collider.shape {
                ColliderShape::Sphere { .. } => {
                    sweep_sphere_sphere(start, end, radius, other_transform.position, other_radius)
                }
                ColliderShape::Box { half_extents } => {
                    let aabb_min = other_transform.position - *half_extents;
                    let aabb_max = other_transform.position + *half_extents;
                    sweep_sphere_aabb(start, end, radius, aabb_min, aabb_max)
                }
                _ => {
                    // Capsule/Cylinder: approximate as sphere sweep
                    sweep_sphere_sphere(start, end, radius, other_transform.position, other_radius)
                }
            };

            if result.hit && result.toi < earliest_toi {
                earliest_toi = result.toi;
            }
        }

        // Stop slightly before the impact point
        let safe_toi = (earliest_toi - 0.01).max(0.0);
        start + direction * safe_toi
    }

    /// Update sleep states: put bodies to sleep if they've been at rest,
    /// wake them up if they receive a collision impulse.
    fn update_sleep_states(&self, world: &mut World, dt: f32) {
        let sleep_threshold = 0.1; // velocity threshold
        let sleep_time = 0.5; // seconds at rest before sleeping

        let indices = world.component_entities::<RigidBody>();
        for &idx in &indices {
            if let Some(body) = world.get_by_index::<RigidBody>(idx) {
                if body.body_type != BodyType::Dynamic {
                    continue;
                }

                let speed_sq =
                    body.linear_velocity.length_squared() + body.angular_velocity.length_squared();

                if speed_sq < sleep_threshold * sleep_threshold {
                    // Accumulate rest time
                    let rest_time = body.rest_time + dt;
                    if rest_time >= sleep_time {
                        // Put to sleep
                        if let Some(body) = world.get_by_index_mut::<RigidBody>(idx) {
                            body.is_sleeping = true;
                            body.linear_velocity = Vec3::ZERO;
                            body.angular_velocity = Vec3::ZERO;
                            body.rest_time = rest_time;
                        }
                    } else if let Some(body) = world.get_by_index_mut::<RigidBody>(idx) {
                        body.rest_time = rest_time;
                    }
                } else {
                    // Moving — reset rest time and ensure awake
                    if let Some(body) = world.get_by_index_mut::<RigidBody>(idx) {
                        body.rest_time = 0.0;
                        body.is_sleeping = false;
                    }
                }
            }
        }
    }

    /// Wake up sleeping bodies that are involved in a collision.
    fn wake_colliding_bodies(&self, world: &mut World) {
        for &(idx_a, idx_b, _) in &self.collisions {
            if let Some(body) = world.get_by_index::<RigidBody>(idx_a)
                && body.is_sleeping
                && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_a)
            {
                body.is_sleeping = false;
                body.rest_time = 0.0;
            }
            if let Some(body) = world.get_by_index::<RigidBody>(idx_b)
                && body.is_sleeping
                && let Some(body) = world.get_by_index_mut::<RigidBody>(idx_b)
            {
                body.is_sleeping = false;
                body.rest_time = 0.0;
            }
        }
    }

    /// Detect collisions using spatial hash broadphase + parallel narrow-phase.
    fn detect_collisions(&mut self, world: &World) {
        self.collisions.clear();
        self.collision_events.clear();
        self.sensor_events.clear();
        self.broadphase.clear();

        let collider_indices = world.component_entities::<Collider>();

        // Insert all non-sleeping colliders into broadphase
        for &idx in &collider_indices {
            // Skip sleeping bodies — they don't need broadphase testing
            let is_sleeping = world
                .get_by_index::<RigidBody>(idx)
                .is_some_and(|b| b.is_sleeping);
            if is_sleeping {
                continue;
            }

            if let Some(transform) = world.get_by_index::<Transform>(idx)
                && let Some(collider) = world.get_by_index::<Collider>(idx)
            {
                let half_extents = match &collider.shape {
                    crate::collider::ColliderShape::Sphere { radius } => Vec3::splat(*radius),
                    crate::collider::ColliderShape::Box { half_extents } => *half_extents,
                    crate::collider::ColliderShape::Capsule { radius, height } => {
                        Vec3::new(*radius, radius + height * 0.5, *radius)
                    }
                    crate::collider::ColliderShape::Cylinder { radius, height } => {
                        Vec3::new(*radius, height * 0.5, *radius)
                    }
                };
                self.broadphase.insert(BroadphaseEntry {
                    entity_index: idx,
                    center: transform.position,
                    half_extents,
                    collision_layers: collider.collision_layers,
                    collision_mask: collider.collision_mask,
                });
            }
        }

        // Get candidate pairs from broadphase (already filtered by layer mask + AABB)
        let pairs = self.broadphase.compute_pairs();

        // Parallel narrow-phase: check each pair concurrently
        enum NarrowResult {
            Collision(u32, u32, CollisionInfo),
            Sensor(u32, u32),
        }

        let results: Vec<NarrowResult> = pairs
            .par_iter()
            .filter_map(|pair| {
                let idx_a = pair.index_a;
                let idx_b = pair.index_b;

                let transform_a = world.get_by_index::<Transform>(idx_a)?;
                let transform_b = world.get_by_index::<Transform>(idx_b)?;
                let collider_a = world.get_by_index::<Collider>(idx_a)?;
                let collider_b = world.get_by_index::<Collider>(idx_b)?;

                // Layer mask filtering is already done in broadphase,
                // but double-check for safety
                if !collider_a.can_collide_with(collider_b) {
                    return None;
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

                // Sensor pairs: overlap test only
                if collider_a.is_sensor || collider_b.is_sensor {
                    let overlap = check_collision(
                        transform_a.position,
                        rot_a,
                        collider_a,
                        transform_b.position,
                        rot_b,
                        collider_b,
                    );
                    if overlap.is_some() {
                        return Some(NarrowResult::Sensor(idx_a, idx_b));
                    }
                    return None;
                }

                let mut info = check_collision(
                    transform_a.position,
                    rot_a,
                    collider_a,
                    transform_b.position,
                    rot_b,
                    collider_b,
                )?;
                info.other_entity = idx_b as u64;
                Some(NarrowResult::Collision(idx_a, idx_b, info))
            })
            .collect();

        // Split results into collisions and sensor events with enter/exit tracking
        self.collisions.clear();
        self.sensor_events.clear();

        let mut current_collision_pairs = HashSet::new();
        let mut current_sensor_pairs = HashSet::new();

        for result in results {
            match result {
                NarrowResult::Collision(a, b, info) => {
                    let key = PairKey::new(a, b);
                    let is_enter = !self.previous_collision_pairs.contains(&key);
                    current_collision_pairs.insert(key);

                    self.collision_events.push(CollisionEvent {
                        entity_a: a,
                        entity_b: b,
                        normal: info.normal,
                        depth: info.depth,
                        point: info.point,
                        is_enter,
                    });
                    self.collisions.push((a, b, info));
                }
                NarrowResult::Sensor(a, b) => {
                    let key = PairKey::new(a, b);
                    let is_enter = !self.previous_sensor_pairs.contains(&key);
                    current_sensor_pairs.insert(key);

                    // Determine which is the sensor
                    let (sensor_e, other_e) = {
                        let a_is_sensor = world
                            .get_by_index::<Collider>(a)
                            .is_some_and(|c| c.is_sensor);
                        if a_is_sensor { (a, b) } else { (b, a) }
                    };

                    self.sensor_events.push(SensorEvent {
                        sensor_entity: sensor_e,
                        other_entity: other_e,
                        overlapping: true,
                        is_enter,
                    });
                }
            }
        }

        // Emit exit events for pairs that are no longer colliding
        for &key in &self.previous_collision_pairs {
            if !current_collision_pairs.contains(&key) {
                self.collision_events.push(CollisionEvent {
                    entity_a: key.0,
                    entity_b: key.1,
                    normal: Vec3::ZERO,
                    depth: 0.0,
                    point: Vec3::ZERO,
                    is_enter: false,
                });
            }
        }
        for &key in &self.previous_sensor_pairs {
            if !current_sensor_pairs.contains(&key) {
                let (sensor_e, other_e) = {
                    let a_is_sensor = world
                        .get_by_index::<Collider>(key.0)
                        .is_some_and(|c| c.is_sensor);
                    if a_is_sensor {
                        (key.0, key.1)
                    } else {
                        (key.1, key.0)
                    }
                };
                self.sensor_events.push(SensorEvent {
                    sensor_entity: sensor_e,
                    other_entity: other_e,
                    overlapping: false,
                    is_enter: false,
                });
            }
        }

        self.previous_collision_pairs = current_collision_pairs;
        self.previous_sensor_pairs = current_sensor_pairs;
    }

    /// Resolve detected collisions with warm-starting from cached contacts.
    ///
    /// Collisions are grouped into independent "islands" via union-find on
    /// shared entities. Islands are resolved in parallel; within each island
    /// constraints are solved sequentially since they share bodies.
    fn resolve_collisions_with_warm_start(&mut self, world: &mut World) {
        let dt = self.delta_time / self.sub_steps as f32;

        if self.collisions.is_empty() {
            // Decay stale contact cache entries
            self.contact_cache.clear();
            return;
        }

        // Build/update contact manifolds from current collisions
        let mut manifolds: Vec<ContactManifold> = Vec::new();
        let mut seen_keys: HashSet<PairKey> = HashSet::new();

        for &(idx_a, idx_b, ref info) in &self.collisions {
            let key = PairKey::new(idx_a, idx_b);
            seen_keys.insert(key);

            let restitution_a = world
                .get_by_index::<Collider>(idx_a)
                .map_or(0.3, |c| c.restitution);
            let restitution_b = world
                .get_by_index::<Collider>(idx_b)
                .map_or(0.3, |c| c.restitution);
            let friction_a = world
                .get_by_index::<Collider>(idx_a)
                .map_or(0.5, |c| c.friction);
            let friction_b = world
                .get_by_index::<Collider>(idx_b)
                .map_or(0.5, |c| c.friction);

            // Try to reuse cached manifold for warm-starting
            let mut manifold = if let Some(cached) = self.contact_cache.remove(&key) {
                cached
            } else {
                ContactManifold::new(idx_a, idx_b)
            };

            manifold.restitution = (restitution_a + restitution_b) * 0.5;
            manifold.friction = (friction_a + friction_b) * 0.5;

            // Update contact points (keep accumulated impulses for warm start)
            let contact = ContactPoint::new(info.point, info.normal, info.depth);
            if manifold.contacts.is_empty() {
                manifold.add_contact(contact);
            } else {
                // Update the first contact point position/normal/depth,
                // preserve accumulated impulses
                manifold.contacts[0].position = contact.position;
                manifold.contacts[0].normal = contact.normal;
                manifold.contacts[0].depth = contact.depth;
            }

            manifolds.push(manifold);
        }

        // Prune stale cache entries
        self.contact_cache.retain(|key, _| seen_keys.contains(key));

        // Union-Find: group collisions that share entities into islands
        let mut uf = UnionFind::new();
        for &(idx_a, idx_b, _) in &self.collisions {
            uf.union(idx_a as usize, idx_b as usize);
        }

        // Group manifold indices by island root
        let mut islands: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, manifold) in manifolds.iter().enumerate() {
            let root = uf.find(manifold.body_a as usize);
            islands.entry(root).or_default().push(i);
        }

        // Resolve each island in parallel using the contact solver.
        //
        // SAFETY INVARIANTS:
        //   1. Islands are disjoint sets of entities — the Union-Find guarantees
        //      that no two islands share an entity index.
        //   2. Each parallel task therefore writes to a distinct subset of
        //      RigidBody/Transform components, so there is no data race on
        //      component storage.
        //   3. The AtomicPtr is only *loaded* (never stored-to) inside the
        //      parallel closure; all loads return the same pointer to the
        //      original `world` borrow, which lives for the duration of
        //      `par_iter`.
        //   4. Within a single island, manifolds are processed sequentially,
        //      so shared-body writes within an island are ordered.
        //
        // If the island decomposition ever changes to allow entity overlap
        // between islands, this block MUST be reworked (e.g. split the World
        // into per-island sub-slices or use a lock).
        let solver = &self.contact_solver;
        let world_ptr = AtomicPtr::new(world as *mut World);

        islands.par_iter().for_each(|(_, manifold_indices)| {
            // SAFETY: see invariants above — all tasks receive the same pointer
            // and access disjoint entity sets.
            let world_ref = unsafe { &mut *world_ptr.load(std::sync::atomic::Ordering::Relaxed) };
            for &mi in manifold_indices {
                let manifold = &manifolds[mi];
                let idx_a = manifold.body_a;
                let idx_b = manifold.body_b;

                let is_dynamic_a = world_ref
                    .get_by_index::<RigidBody>(idx_a)
                    .is_some_and(|b| b.body_type == BodyType::Dynamic);
                let is_dynamic_b = world_ref
                    .get_by_index::<RigidBody>(idx_b)
                    .is_some_and(|b| b.body_type == BodyType::Dynamic);

                let inv_mass_a = if is_dynamic_a {
                    world_ref
                        .get_by_index::<RigidBody>(idx_a)
                        .map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 1.0 })
                } else {
                    0.0
                };
                let inv_mass_b = if is_dynamic_b {
                    world_ref
                        .get_by_index::<RigidBody>(idx_b)
                        .map_or(1.0, |b| if b.mass > 0.0 { 1.0 / b.mass } else { 1.0 })
                } else {
                    0.0
                };

                let mut vel_a = world_ref
                    .get_by_index::<RigidBody>(idx_a)
                    .map_or(Vec3::ZERO, |b| b.linear_velocity);
                let mut vel_b = world_ref
                    .get_by_index::<RigidBody>(idx_b)
                    .map_or(Vec3::ZERO, |b| b.linear_velocity);

                // Use the contact solver with warm-starting
                let mut manifold_clone = manifold.clone();
                solver.solve_manifold(
                    &mut manifold_clone,
                    &mut vel_a,
                    &mut vel_b,
                    inv_mass_a,
                    inv_mass_b,
                    dt,
                );

                // Apply velocity corrections
                if is_dynamic_a && let Some(body) = world_ref.get_by_index_mut::<RigidBody>(idx_a) {
                    body.linear_velocity = vel_a;
                }
                if is_dynamic_b && let Some(body) = world_ref.get_by_index_mut::<RigidBody>(idx_b) {
                    body.linear_velocity = vel_b;
                }

                // Position correction (Baumgarte stabilization)
                if let Some(contact) = manifold_clone.contacts.first() {
                    let slop = 0.005;
                    let percent = 0.4;
                    let correction = (contact.depth - slop).max(0.0)
                        / (inv_mass_a + inv_mass_b).max(f32::EPSILON)
                        * percent
                        * contact.normal;

                    if is_dynamic_a
                        && let Some(transform) = world_ref.get_by_index_mut::<Transform>(idx_a)
                    {
                        transform.position -= correction * inv_mass_a;
                    }
                    if is_dynamic_b
                        && let Some(transform) = world_ref.get_by_index_mut::<Transform>(idx_b)
                    {
                        transform.position += correction * inv_mass_b;
                    }
                }
            }
        });

        // Store updated manifolds in contact cache for next frame warm-starting
        for manifold in manifolds {
            let key = PairKey::new(manifold.body_a, manifold.body_b);
            self.contact_cache.insert(key, manifold);
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

        let floor = world.spawn();
        world.add_component(floor, Transform::from_xyz(0.0, -0.5, 0.0));
        world.add_component(floor, RigidBody::new_static());
        world.add_component(floor, Collider::cuboid(50.0, 0.5, 50.0));

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

        for _ in 0..10 {
            pw.step(&mut world);
        }

        let body = world.get_by_index::<RigidBody>(sphere.index()).unwrap();
        assert!(
            body.linear_velocity.y > 0.0,
            "Sphere should bounce, got velocity.y = {}",
            body.linear_velocity.y
        );
    }

    #[test]
    fn test_broadphase_integration() {
        let mut pw = PhysicsWorld::new();
        pw.set_broadphase_cell_size(5.0);
        assert_eq!(pw.broadphase.cell_size(), 5.0);
    }

    #[test]
    fn test_collision_enter_event() {
        let mut world = World::new();

        let a = world.spawn();
        world.add_component(a, Transform::from_xyz(0.0, 0.0, 0.0));
        world.add_component(a, RigidBody::new_dynamic());
        world.add_component(a, Collider::sphere(1.0));

        let b = world.spawn();
        world.add_component(b, Transform::from_xyz(1.5, 0.0, 0.0));
        world.add_component(b, RigidBody::new_static());
        world.add_component(b, Collider::sphere(1.0));

        let mut pw = PhysicsWorld::new();
        pw.sub_steps = 1;

        // First frame: should be a collision enter
        pw.step(&mut world);

        let enter_events: Vec<_> = pw.collision_events.iter().filter(|e| e.is_enter).collect();
        assert!(
            !enter_events.is_empty(),
            "Should have at least one collision enter event"
        );
    }

    #[test]
    fn test_layer_mask_filtering_in_world() {
        let mut world = World::new();

        let a = world.spawn();
        world.add_component(a, Transform::from_xyz(0.0, 0.0, 0.0));
        world.add_component(a, RigidBody::new_dynamic());
        let mut col_a = Collider::sphere(1.0);
        col_a.collision_layers = 0x01;
        col_a.collision_mask = 0x01;
        world.add_component(a, col_a);

        let b = world.spawn();
        world.add_component(b, Transform::from_xyz(0.5, 0.0, 0.0));
        world.add_component(b, RigidBody::new_static());
        let mut col_b = Collider::sphere(1.0);
        col_b.collision_layers = 0x02;
        col_b.collision_mask = 0x02;
        world.add_component(b, col_b);

        let mut pw = PhysicsWorld::new();
        pw.sub_steps = 1;

        pw.step(&mut world);

        assert!(
            pw.collisions.is_empty(),
            "Layer mismatch should prevent collision"
        );
    }
}
