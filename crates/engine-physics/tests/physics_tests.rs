use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_math::{Quat, Vec3};
use engine_physics::body::BodyType;
use engine_physics::ccd::{CcdBody, sweep_sphere_aabb, sweep_sphere_sphere};
use engine_physics::contact::{ContactManifold, ContactPoint, ContactSolver};
use engine_physics::joint::{Joint, JointSolver};
use engine_physics::{
    Collider, PhysicsWorld, RigidBody, check_box_box, check_collision, check_sphere_sphere,
};

// ===========================================================================
// Physics World creation
// ===========================================================================

#[test]
fn world_default_gravity() {
    let pw = PhysicsWorld::new();
    assert_eq!(pw.gravity, Vec3::new(0.0, -9.81, 0.0));
}

#[test]
fn world_default_sub_steps() {
    let pw = PhysicsWorld::new();
    assert_eq!(pw.sub_steps, 4);
}

#[test]
fn world_default_delta_time() {
    let pw = PhysicsWorld::new();
    assert!((pw.delta_time - 1.0 / 60.0).abs() < 1e-6);
}

#[test]
fn world_custom_gravity() {
    let mut pw = PhysicsWorld::new();
    pw.set_gravity(Vec3::new(0.0, -20.0, 0.0));
    assert_eq!(pw.gravity, Vec3::new(0.0, -20.0, 0.0));
}

#[test]
fn world_custom_broadphase_cell_size() {
    let mut pw = PhysicsWorld::new();
    pw.set_broadphase_cell_size(10.0);
    // Just verify it doesn't panic and the world still works
    let mut world = World::new();
    pw.step(&mut world);
}

// ===========================================================================
// Rigid body creation
// ===========================================================================

#[test]
fn dynamic_body_type() {
    let body = RigidBody::new_dynamic();
    assert_eq!(body.body_type, BodyType::Dynamic);
    assert!((body.mass - 1.0).abs() < 1e-6);
    assert_eq!(body.linear_velocity, Vec3::ZERO);
    assert_eq!(body.angular_velocity, Vec3::ZERO);
    assert!((body.gravity_scale - 1.0).abs() < 1e-6);
    assert!(!body.is_sleeping);
}

#[test]
fn static_body_type() {
    let body = RigidBody::new_static();
    assert_eq!(body.body_type, BodyType::Static);
}

#[test]
fn kinematic_body_type() {
    let body = RigidBody::new_kinematic();
    assert_eq!(body.body_type, BodyType::Kinematic);
}

#[test]
fn static_body_ignores_velocity() {
    let mut body = RigidBody::new_static();
    body.set_linear_velocity(Vec3::new(10.0, 0.0, 0.0));
    assert_eq!(body.linear_velocity, Vec3::ZERO);
}

#[test]
fn dynamic_body_accepts_velocity() {
    let mut body = RigidBody::new_dynamic();
    body.set_linear_velocity(Vec3::new(5.0, 3.0, 0.0));
    assert_eq!(body.linear_velocity, Vec3::new(5.0, 3.0, 0.0));
}

#[test]
fn kinematic_body_accepts_velocity() {
    let mut body = RigidBody::new_kinematic();
    body.set_linear_velocity(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(body.linear_velocity, Vec3::new(1.0, 2.0, 3.0));
}

// ===========================================================================
// Collider attachment
// ===========================================================================

#[test]
fn sphere_collider_shape() {
    let c = Collider::sphere(1.5);
    match &c.shape {
        engine_physics::collider::ColliderShape::Sphere { radius } => {
            assert!((radius - 1.5).abs() < 1e-6);
        }
        _ => panic!("Expected Sphere shape"),
    }
    assert!(!c.is_sensor);
}

#[test]
fn cuboid_collider_shape() {
    let c = Collider::cuboid(1.0, 2.0, 3.0);
    match &c.shape {
        engine_physics::collider::ColliderShape::Box { half_extents } => {
            assert!((half_extents.x - 1.0).abs() < 1e-6);
            assert!((half_extents.y - 2.0).abs() < 1e-6);
            assert!((half_extents.z - 3.0).abs() < 1e-6);
        }
        _ => panic!("Expected Box shape"),
    }
}

#[test]
fn capsule_collider_shape() {
    let c = Collider::capsule(0.5, 2.0);
    match &c.shape {
        engine_physics::collider::ColliderShape::Capsule { radius, height } => {
            assert!((radius - 0.5).abs() < 1e-6);
            assert!((height - 2.0).abs() < 1e-6);
        }
        _ => panic!("Expected Capsule shape"),
    }
}

#[test]
fn cylinder_collider_shape() {
    let c = Collider::cylinder(0.3, 1.5);
    match &c.shape {
        engine_physics::collider::ColliderShape::Cylinder { radius, height } => {
            assert!((radius - 0.3).abs() < 1e-6);
            assert!((height - 1.5).abs() < 1e-6);
        }
        _ => panic!("Expected Cylinder shape"),
    }
}

#[test]
fn collider_default_layer_masks() {
    let c = Collider::sphere(1.0);
    assert_eq!(c.collision_layers, 0xFFFF_FFFF);
    assert_eq!(c.collision_mask, 0xFFFF_FFFF);
}

#[test]
fn collider_layer_filtering() {
    let mut a = Collider::sphere(1.0);
    a.collision_layers = 0x01;
    a.collision_mask = 0x01;

    let mut b = Collider::sphere(1.0);
    b.collision_layers = 0x02;
    b.collision_mask = 0x02;

    assert!(!a.can_collide_with(&b));

    let mut c = Collider::sphere(1.0);
    c.collision_layers = 0x01;
    c.collision_mask = 0x01;

    assert!(a.can_collide_with(&c));
}

#[test]
fn collider_entity_spawning() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(1.0, 2.0, 3.0));
    world.add_component(e, RigidBody::new_dynamic());
    world.add_component(e, Collider::sphere(0.5));

    let col = world.get_by_index::<Collider>(e.index()).unwrap();
    let body = world.get_by_index::<RigidBody>(e.index()).unwrap();
    let t = world.get_by_index::<Transform>(e.index()).unwrap();

    assert_eq!(body.body_type, BodyType::Dynamic);
    assert!((t.position.x - 1.0).abs() < 1e-6);
    match &col.shape {
        engine_physics::collider::ColliderShape::Sphere { radius } => {
            assert!((radius - 0.5).abs() < 1e-6);
        }
        _ => panic!("Expected Sphere"),
    }
}

// ===========================================================================
// Sphere-sphere collision
// ===========================================================================

#[test]
fn sphere_sphere_overlapping() {
    let result = check_sphere_sphere(Vec3::ZERO, 1.0, Vec3::new(1.5, 0.0, 0.0), 1.0);
    assert!(result.is_some());
    let info = result.unwrap();
    assert!(info.depth > 0.0);
    assert!(info.normal.x > 0.0);
}

#[test]
fn sphere_sphere_separated() {
    let result = check_sphere_sphere(Vec3::ZERO, 0.5, Vec3::new(5.0, 0.0, 0.0), 0.5);
    assert!(result.is_none());
}

#[test]
fn sphere_sphere_touching() {
    // Exactly touching (distance == sum of radii) should not collide
    let result = check_sphere_sphere(Vec3::ZERO, 1.0, Vec3::new(2.0, 0.0, 0.0), 1.0);
    assert!(result.is_none());
}

#[test]
fn sphere_sphere_deep_overlap() {
    let result = check_sphere_sphere(Vec3::ZERO, 2.0, Vec3::new(0.5, 0.0, 0.0), 2.0);
    assert!(result.is_some());
    let info = result.unwrap();
    assert!(info.depth > 3.0);
}

#[test]
fn sphere_sphere_normal_direction() {
    let result = check_sphere_sphere(Vec3::ZERO, 1.0, Vec3::new(0.0, 1.5, 0.0), 1.0);
    assert!(result.is_some());
    let info = result.unwrap();
    assert!(info.normal.y > 0.0);
}

// ===========================================================================
// Force application
// ===========================================================================

#[test]
fn apply_force_to_dynamic_body() {
    let mut body = RigidBody::new_dynamic();
    body.mass = 2.0;
    body.apply_force(Vec3::new(10.0, 0.0, 0.0));
    // v = F/m = 10/2 = 5
    assert!((body.linear_velocity.x - 5.0).abs() < 1e-6);
}

#[test]
fn apply_force_to_static_body_does_nothing() {
    let mut body = RigidBody::new_static();
    body.apply_force(Vec3::new(100.0, 100.0, 100.0));
    assert_eq!(body.linear_velocity, Vec3::ZERO);
}

#[test]
fn apply_impulse_to_dynamic_body() {
    let mut body = RigidBody::new_dynamic();
    body.mass = 0.5;
    body.apply_impulse(Vec3::new(0.0, -5.0, 0.0));
    // v = impulse/mass = -5/0.5 = -10
    assert!((body.linear_velocity.y - (-10.0)).abs() < 1e-6);
}

#[test]
fn apply_force_zero_mass_does_nothing() {
    let mut body = RigidBody::new_dynamic();
    body.mass = 0.0;
    body.apply_force(Vec3::new(100.0, 0.0, 0.0));
    assert_eq!(body.linear_velocity, Vec3::ZERO);
}

// ===========================================================================
// Gravity
// ===========================================================================

#[test]
fn gravity_accelerates_dynamic_body() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(0.0, 100.0, 0.0));
    world.add_component(e, RigidBody::new_dynamic());
    world.add_component(e, Collider::sphere(0.5));

    let mut pw = PhysicsWorld::new();
    pw.sub_steps = 1;
    pw.delta_time = 1.0 / 60.0;

    pw.step(&mut world);

    let body = world.get_by_index::<RigidBody>(e.index()).unwrap();
    assert!(
        body.linear_velocity.y < 0.0,
        "Gravity should pull body down"
    );
}

#[test]
fn gravity_does_not_affect_static_body() {
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
fn gravity_does_not_affect_kinematic_body() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(0.0, 5.0, 0.0));
    world.add_component(e, RigidBody::new_kinematic());
    world.add_component(e, Collider::sphere(0.5));

    let mut pw = PhysicsWorld::new();
    pw.sub_steps = 1;
    pw.step(&mut world);

    let body = world.get_by_index::<RigidBody>(e.index()).unwrap();
    assert_eq!(body.linear_velocity, Vec3::ZERO);
}

#[test]
fn gravity_scale_multiplier() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Transform::from_xyz(0.0, 100.0, 0.0));
    let mut body = RigidBody::new_dynamic();
    body.gravity_scale = 2.0;
    world.add_component(e, body);
    world.add_component(e, Collider::sphere(0.5));

    let mut pw = PhysicsWorld::new();
    pw.sub_steps = 1;
    pw.delta_time = 1.0 / 60.0;

    pw.step(&mut world);

    let body = world.get_by_index::<RigidBody>(e.index()).unwrap();
    // Gravity vel = -9.81 * 2.0 * dt ≈ -0.327
    assert!(
        body.linear_velocity.y < -0.3,
        "Gravity scale 2x should double the effect, got {}",
        body.linear_velocity.y
    );
}

// ===========================================================================
// Sphere-box collision via dispatcher
// ===========================================================================

#[test]
fn sphere_box_collision_via_dispatcher() {
    let s = Collider::sphere(0.5);
    let b = Collider::cuboid(1.0, 1.0, 1.0);
    let result = check_collision(
        Vec3::new(1.2, 0.0, 0.0),
        Quat::IDENTITY,
        &s,
        Vec3::ZERO,
        Quat::IDENTITY,
        &b,
    );
    assert!(result.is_some());
}

#[test]
fn sphere_box_no_collision() {
    let s = Collider::sphere(0.5);
    let b = Collider::cuboid(1.0, 1.0, 1.0);
    let result = check_collision(
        Vec3::new(5.0, 0.0, 0.0),
        Quat::IDENTITY,
        &s,
        Vec3::ZERO,
        Quat::IDENTITY,
        &b,
    );
    assert!(result.is_none());
}

// ===========================================================================
// Box-box collision
// ===========================================================================

#[test]
fn box_box_overlapping() {
    let result = check_box_box(
        Vec3::ZERO,
        Vec3::splat(1.0),
        Vec3::new(1.5, 0.0, 0.0),
        Vec3::splat(1.0),
    );
    assert!(result.is_some());
    assert!(result.unwrap().depth > 0.0);
}

#[test]
fn box_box_separated() {
    let result = check_box_box(
        Vec3::ZERO,
        Vec3::splat(1.0),
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::splat(1.0),
    );
    assert!(result.is_none());
}

// ===========================================================================
// Capsule collisions via dispatcher
// ===========================================================================

#[test]
fn capsule_capsule_via_dispatcher() {
    let a = Collider::capsule(0.5, 2.0);
    let b = Collider::capsule(0.5, 2.0);
    let result = check_collision(
        Vec3::ZERO,
        Quat::IDENTITY,
        &a,
        Vec3::new(0.6, 0.0, 0.0),
        Quat::IDENTITY,
        &b,
    );
    assert!(result.is_some());
}

#[test]
fn sphere_capsule_via_dispatcher() {
    let s = Collider::sphere(0.5);
    let c = Collider::capsule(0.5, 2.0);
    let result = check_collision(
        Vec3::new(0.7, 0.0, 0.0),
        Quat::IDENTITY,
        &s,
        Vec3::ZERO,
        Quat::IDENTITY,
        &c,
    );
    assert!(result.is_some());
}

// ===========================================================================
// Cylinder fallback
// ===========================================================================

#[test]
fn cylinder_sphere_fallback_to_bounding_sphere() {
    let cyl = Collider::cylinder(0.5, 1.0);
    let s = Collider::sphere(0.5);
    let result = check_collision(
        Vec3::ZERO,
        Quat::IDENTITY,
        &cyl,
        Vec3::new(0.5, 0.0, 0.0),
        Quat::IDENTITY,
        &s,
    );
    assert!(result.is_some());
}

// ===========================================================================
// Collision resolution (integration)
// ===========================================================================

#[test]
fn sphere_bounces_off_floor() {
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
        "Sphere should bounce off floor, got velocity.y = {}",
        body.linear_velocity.y
    );
}

// ===========================================================================
// Collision events
// ===========================================================================

#[test]
fn collision_enter_event_emitted() {
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
    pw.step(&mut world);

    let enter_events: Vec<_> = pw.collision_events.iter().filter(|e| e.is_enter).collect();
    assert!(
        !enter_events.is_empty(),
        "Should have at least one collision enter event"
    );
}

#[test]
fn layer_mask_prevents_collision() {
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

// ===========================================================================
// Sensor events
// ===========================================================================

#[test]
fn sensor_overlap_detected() {
    let mut world = World::new();

    let a = world.spawn();
    world.add_component(a, Transform::from_xyz(0.0, 0.0, 0.0));
    world.add_component(a, RigidBody::new_dynamic());
    let mut col_a = Collider::sphere(1.0);
    col_a.is_sensor = true;
    world.add_component(a, col_a);

    let b = world.spawn();
    world.add_component(b, Transform::from_xyz(0.5, 0.0, 0.0));
    world.add_component(b, RigidBody::new_static());
    world.add_component(b, Collider::sphere(1.0));

    let mut pw = PhysicsWorld::new();
    pw.sub_steps = 1;
    pw.step(&mut world);

    let sensor_events: Vec<_> = pw.sensor_events.iter().filter(|e| e.is_enter).collect();
    assert!(
        !sensor_events.is_empty(),
        "Should have at least one sensor enter event"
    );
}

// ===========================================================================
// CCD (Continuous Collision Detection)
// ===========================================================================

#[test]
fn ccd_sweep_sphere_sphere_hit() {
    let result = sweep_sphere_sphere(
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        0.5,
        Vec3::new(5.0, 0.0, 0.0),
        1.0,
    );
    assert!(result.hit);
    assert!(result.toi > 0.0 && result.toi < 1.0);
}

#[test]
fn ccd_sweep_sphere_sphere_miss() {
    let result = sweep_sphere_sphere(
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        0.5,
        Vec3::new(0.0, 5.0, 0.0),
        1.0,
    );
    assert!(!result.hit);
}

#[test]
fn ccd_sweep_sphere_aabb_hit() {
    let result = sweep_sphere_aabb(
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        0.5,
        Vec3::new(4.0, -1.0, -1.0),
        Vec3::new(6.0, 1.0, 1.0),
    );
    assert!(result.hit);
    assert!(result.toi > 0.0 && result.toi < 1.0);
}

#[test]
fn ccd_sweep_sphere_aabb_miss() {
    let result = sweep_sphere_aabb(
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        0.5,
        Vec3::new(0.0, 5.0, 0.0),
        Vec3::new(1.0, 6.0, 1.0),
    );
    assert!(!result.hit);
}

#[test]
fn ccd_body_default_values() {
    let ccd = CcdBody::default();
    assert!(ccd.enabled);
    assert!((ccd.activation_threshold - 1.0).abs() < 1e-6);
}

// ===========================================================================
// Contact solver
// ===========================================================================

#[test]
fn contact_point_creation() {
    let cp = ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1);
    assert_eq!(cp.position, Vec3::ZERO);
    assert_eq!(cp.normal, Vec3::Y);
    assert!((cp.depth - 0.1).abs() < 1e-6);
}

#[test]
fn contact_manifold_add_and_count() {
    let mut m = ContactManifold::new(0, 1);
    m.add_contact(ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1));
    m.add_contact(ContactPoint::new(Vec3::X, Vec3::Y, 0.05));
    assert_eq!(m.contact_count(), 2);
}

#[test]
fn solver_pushes_bodies_apart() {
    let mut m = ContactManifold::new(0, 1);
    m.add_contact(ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1));

    let solver = ContactSolver::new();
    let mut vel_a = Vec3::ZERO;
    let mut vel_b = Vec3::new(0.0, -5.0, 0.0);

    solver.solve_manifold(&mut m, &mut vel_a, &mut vel_b, 1.0, 1.0, 1.0 / 60.0);

    assert!(vel_a.y <= 0.0);
    assert!(vel_b.y >= -5.0);
}

#[test]
fn solver_static_body_unchanged() {
    let mut m = ContactManifold::new(0, 1);
    m.add_contact(ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1));

    let solver = ContactSolver::new();
    let mut vel_a = Vec3::ZERO;
    let mut vel_b = Vec3::new(0.0, -5.0, 0.0);

    solver.solve_manifold(&mut m, &mut vel_a, &mut vel_b, 0.0, 1.0, 1.0 / 60.0);

    assert!(vel_a.length() < 1e-6);
    assert!(vel_b.y > -5.0);
}

// ===========================================================================
// Joints
// ===========================================================================

#[test]
fn ball_socket_joint_creation() {
    let j = Joint::ball_socket(0, 1, Vec3::ZERO, Vec3::ZERO);
    assert_eq!(j.joint_type, engine_physics::joint::JointType::BallSocket);
    assert!(j.enabled);
}

#[test]
fn hinge_joint_with_limits() {
    let j = Joint::hinge(0, 1, Vec3::ZERO, Vec3::ZERO, Vec3::Y).with_angle_limits(-1.0, 1.0);
    assert!((j.min_angle - (-1.0)).abs() < 1e-6);
    assert!((j.max_angle - 1.0).abs() < 1e-6);
}

#[test]
fn spring_joint_force_computation() {
    let mut solver = JointSolver::new();
    solver.add_joint(Joint::spring(0, 1, Vec3::ZERO, Vec3::ZERO, 5.0, 100.0, 0.0));

    let positions = vec![(0, Vec3::ZERO), (1, Vec3::new(10.0, 0.0, 0.0))];
    let velocities = vec![(0, Vec3::ZERO), (1, Vec3::ZERO)];

    let corrections = solver.solve_springs(&positions, &velocities);
    assert_eq!(corrections.len(), 2);
    assert!(
        corrections[0].1.x > 0.0,
        "Body 0 should be pulled toward body 1"
    );
    assert!(
        corrections[1].1.x < 0.0,
        "Body 1 should be pulled toward body 0"
    );
}

#[test]
fn disabled_joint_ignored() {
    let mut solver = JointSolver::new();
    let mut j = Joint::spring(0, 1, Vec3::ZERO, Vec3::ZERO, 5.0, 100.0, 0.0);
    j.enabled = false;
    solver.add_joint(j);

    let positions = vec![(0, Vec3::ZERO), (1, Vec3::new(10.0, 0.0, 0.0))];
    let velocities = vec![(0, Vec3::ZERO), (1, Vec3::ZERO)];

    let corrections = solver.solve_springs(&positions, &velocities);
    assert!(corrections.is_empty());
}

#[test]
fn joint_solver_remove_for_entity() {
    let mut solver = JointSolver::new();
    solver.add_joint(Joint::ball_socket(0, 1, Vec3::ZERO, Vec3::ZERO));
    solver.add_joint(Joint::hinge(0, 2, Vec3::ZERO, Vec3::ZERO, Vec3::Y));
    solver.add_joint(Joint::ball_socket(1, 2, Vec3::ZERO, Vec3::ZERO));

    solver.remove_joints_for_entity(0);
    assert_eq!(solver.joints.len(), 1);
    assert_eq!(solver.joints[0].entity_a, 1);
}

// ===========================================================================
// Physics 2D
// ===========================================================================

#[cfg(test)]
mod physics_2d_tests {
    use engine_math::Vec2;
    use engine_physics::physics_2d::{AABB2D, BodyType2D, Collider2D, PhysicsWorld2D, RigidBody2D};

    #[test]
    fn test_aabb_overlap() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let b = AABB2D::new(Vec2::new(1.0, 1.0), Vec2::new(3.0, 3.0));
        assert!(a.overlaps(&b));
    }

    #[test]
    fn test_aabb_no_overlap() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        let b = AABB2D::new(Vec2::new(2.0, 2.0), Vec2::new(3.0, 3.0));
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn test_aabb_intersection_x_axis() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let b = AABB2D::new(Vec2::new(1.0, 0.0), Vec2::new(3.0, 2.0));
        let (normal, pen) = a.intersection(&b).unwrap();
        assert!(pen > 0.0);
        assert!(normal.x.abs() > 0.0);
    }

    #[test]
    fn test_aabb_intersection_y_axis() {
        let a = AABB2D::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let b = AABB2D::new(Vec2::new(0.0, 1.0), Vec2::new(2.0, 3.0));
        let (normal, pen) = a.intersection(&b).unwrap();
        assert!(pen > 0.0);
        assert!(normal.y.abs() > 0.0);
    }

    #[test]
    fn test_collider2d_world_aabb() {
        let col = Collider2D::aabb(0.5, 0.5);
        let aabb = col.world_aabb(Vec2::new(1.0, 2.0));
        assert!((aabb.min.x - 0.5).abs() < 0.001);
        assert!((aabb.min.y - 1.5).abs() < 0.001);
        assert!((aabb.max.x - 1.5).abs() < 0.001);
        assert!((aabb.max.y - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_rigidbody2d_types() {
        let dynamic = RigidBody2D::new_dynamic();
        assert_eq!(dynamic.body_type, BodyType2D::Dynamic);
        assert_eq!(dynamic.gravity_scale, 1.0);

        let static_body = RigidBody2D::new_static();
        assert_eq!(static_body.body_type, BodyType2D::Static);
        assert_eq!(static_body.gravity_scale, 0.0);

        let kinematic = RigidBody2D::new_kinematic();
        assert_eq!(kinematic.body_type, BodyType2D::Kinematic);
    }

    #[test]
    fn test_physics_world_2d_gravity() {
        let mut world = engine_ecs::world::World::new();
        let entity = world.spawn();
        world.add_component(
            entity,
            engine_core::transform::Transform::from_xyz(0.0, 10.0, 0.0),
        );
        world.add_component(entity, RigidBody2D::new_dynamic());
        world.add_component(entity, Collider2D::aabb(0.5, 0.5));

        let mut physics = PhysicsWorld2D::new();
        physics.step(&mut world, 1.0 / 60.0);

        let transform = world
            .get_by_index::<engine_core::transform::Transform>(entity.index())
            .unwrap();
        assert!(transform.position.y < 10.0);
    }

    #[test]
    fn test_physics_world_2d_ground_detection() {
        let mut world = engine_ecs::world::World::new();

        let player = world.spawn();
        world.add_component(
            player,
            engine_core::transform::Transform::from_xyz(0.0, 0.6, 0.0),
        );
        world.add_component(player, RigidBody2D::new_dynamic());
        world.add_component(player, Collider2D::aabb(0.5, 0.5));

        let floor = world.spawn();
        world.add_component(
            floor,
            engine_core::transform::Transform::from_xyz(0.0, 0.0, 0.0),
        );
        world.add_component(floor, RigidBody2D::new_static());
        world.add_component(floor, Collider2D::aabb(50.0, 0.5));

        let mut physics = PhysicsWorld2D::new();
        for _ in 0..60 {
            physics.step(&mut world, 1.0 / 60.0);
        }

        let body = world.get_by_index::<RigidBody2D>(player.index()).unwrap();
        assert!(
            body.grounded,
            "Player should be grounded after falling onto floor"
        );
    }
}
