use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_math::Vec3;
use engine_physics::body::RigidBody;
use engine_physics::joint::*;

#[test]
fn hinge_joint_limits_rotation() {
    let mut world = World::new();

    let a = world.spawn();
    world.add_component(a, Transform::from_xyz(0.0, 0.0, 0.0));
    world.add_component(a, RigidBody::new_static());

    let b = world.spawn();
    world.add_component(b, Transform::from_xyz(2.0, 0.0, 0.0));
    let mut body_b = RigidBody::new_dynamic();
    body_b.mass = 1.0;
    body_b.angular_velocity = Vec3::new(0.0, 10.0, 0.0);
    world.add_component(b, body_b);

    let mut solver = JointSolver::new();
    let joint = Joint::hinge(a.index(), b.index(), Vec3::ZERO, Vec3::ZERO, Vec3::Y)
        .with_angle_limits(-0.5, 0.5);
    solver.add_joint(joint);

    solver.solve_constraints(&mut world, 1.0 / 60.0);

    let body_b = world.get_by_index::<RigidBody>(b.index()).unwrap();
    let ang_along_axis = body_b.angular_velocity.dot(Vec3::Y);
    assert!(
        ang_along_axis <= 0.5 + 0.1,
        "Hinge constraint should limit angle velocity around axis, got {}",
        ang_along_axis
    );
}

#[test]
fn ball_socket_limits_distance() {
    let mut world = World::new();

    let a = world.spawn();
    world.add_component(a, Transform::from_xyz(0.0, 0.0, 0.0));
    world.add_component(a, RigidBody::new_dynamic());

    let b = world.spawn();
    world.add_component(b, Transform::from_xyz(10.0, 0.0, 0.0));
    let mut body_b = RigidBody::new_dynamic();
    body_b.mass = 1.0;
    world.add_component(b, body_b);

    let max_dist = 5.0;
    let mut solver = JointSolver::new();
    let joint = Joint::ball_socket(a.index(), b.index(), Vec3::ZERO, Vec3::ZERO)
        .with_max_distance(max_dist);
    solver.add_joint(joint);

    solver.solve_constraints(&mut world, 1.0 / 60.0);

    let pos_a = world
        .get_by_index::<Transform>(a.index())
        .unwrap()
        .position();
    let pos_b = world
        .get_by_index::<Transform>(b.index())
        .unwrap()
        .position();
    let dist = (pos_b - pos_a).length();
    assert!(
        dist <= max_dist + 0.5,
        "Ball socket should constrain distance to {}, got {}",
        max_dist,
        dist
    );
}

#[test]
fn spring_joint_still_works() {
    let mut solver = JointSolver::new();
    solver.add_joint(Joint::spring(0, 1, Vec3::ZERO, Vec3::ZERO, 5.0, 100.0, 0.0));

    let positions = vec![(0, Vec3::ZERO), (1, Vec3::new(10.0, 0.0, 0.0))];
    let velocities = vec![(0, Vec3::ZERO), (1, Vec3::ZERO)];

    let corrections = solver.solve_springs(&positions, &velocities);
    assert_eq!(corrections.len(), 2);
    assert!(corrections[0].1.x > 0.0, "Body 0 pulled toward body 1");
    assert!(corrections[1].1.x < 0.0, "Body 1 pulled toward body 0");
}
