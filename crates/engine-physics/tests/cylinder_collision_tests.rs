use engine_math::{Quat, Vec3};
use engine_physics::collider::{Collider, check_collision};

fn ident_q() -> Quat {
    Quat::IDENTITY
}

// ---------------------------------------------------------------------------
// Cylinder-Sphere (direct function)
// ---------------------------------------------------------------------------

#[test]
fn cylinder_sphere_side_collision() {
    // Cylinder (r=0.5, h=2.0) at origin; sphere (r=0.5) touching side
    let result = engine_physics::collider::check_cylinder_sphere(
        Vec3::ZERO,
        0.5,                      // cylinder radius
        2.0,                      // cylinder height
        Vec3::new(0.8, 0.0, 0.0), // sphere center, offset in X
        0.5,                      // sphere radius
    );
    assert!(
        result.is_some(),
        "sphere touching cylinder side should collide"
    );
    let info = result.unwrap();
    assert!(info.depth > 0.0, "penetration depth must be positive");
    // Normal should point roughly in +X (away from cylinder axis toward sphere)
    assert!(
        info.normal.x > 0.5,
        "normal should point toward sphere, got {:?}",
        info.normal
    );
}

#[test]
fn cylinder_sphere_top_collision() {
    // Cylinder (r=0.5, h=2.0) at origin; sphere resting on top cap
    let result = engine_physics::collider::check_cylinder_sphere(
        Vec3::ZERO,
        0.5,
        2.0,
        Vec3::new(0.0, 1.3, 0.0), // sphere center above top cap (h/2=1.0)
        0.5,
    );
    assert!(result.is_some(), "sphere on top cap should collide");
    let info = result.unwrap();
    assert!(info.depth > 0.0);
    // Normal should point in +Y
    assert!(
        info.normal.y > 0.5,
        "normal should point up for top cap collision, got {:?}",
        info.normal
    );
}

#[test]
fn cylinder_sphere_edge_collision() {
    // Cylinder (r=0.5, h=2.0) at origin; sphere near top edge
    let result = engine_physics::collider::check_cylinder_sphere(
        Vec3::ZERO,
        0.5,
        2.0,
        Vec3::new(0.4, 1.2, 0.0), // sphere near top-right edge
        0.3,
    );
    assert!(result.is_some(), "sphere near cylinder edge should collide");
    let info = result.unwrap();
    assert!(info.depth > 0.0);
}

// ---------------------------------------------------------------------------
// Cylinder-AABB (direct function)
// ---------------------------------------------------------------------------

#[test]
fn cylinder_aabb_overlap() {
    // Cylinder at origin (r=0.5, h=2.0), AABB at (0.3, 0, 0) half-ext (0.5, 0.5, 0.5)
    let result = engine_physics::collider::check_cylinder_aabb(
        Vec3::ZERO,
        0.5,
        2.0,
        Vec3::new(0.3, 0.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
    );
    assert!(
        result.is_some(),
        "overlapping cylinder and AABB should collide"
    );
    let info = result.unwrap();
    assert!(info.depth > 0.0);
}

#[test]
fn cylinder_aabb_separated_x() {
    // Cylinder at origin, AABB far in X
    let result = engine_physics::collider::check_cylinder_aabb(
        Vec3::ZERO,
        0.5,
        2.0,
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
    );
    assert!(result.is_none(), "separated in X should not collide");
}

#[test]
fn cylinder_aabb_separated_y() {
    // Cylinder at origin, AABB far in Y
    let result = engine_physics::collider::check_cylinder_aabb(
        Vec3::ZERO,
        0.5,
        2.0,
        Vec3::new(0.0, 5.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
    );
    assert!(result.is_none(), "separated in Y should not collide");
}

// ---------------------------------------------------------------------------
// No-collision case
// ---------------------------------------------------------------------------

#[test]
fn cylinder_no_collision() {
    // Cylinder and sphere far apart
    let result = engine_physics::collider::check_cylinder_sphere(
        Vec3::ZERO,
        0.5,
        2.0,
        Vec3::new(10.0, 0.0, 0.0),
        0.5,
    );
    assert!(result.is_none(), "far apart shapes should not collide");
}

// ---------------------------------------------------------------------------
// Dispatcher integration (via check_collision)
// ---------------------------------------------------------------------------

#[test]
fn dispatch_cylinder_sphere() {
    let cyl = Collider::cylinder(0.5, 2.0);
    let s = Collider::sphere(0.5);
    let result = check_collision(
        Vec3::ZERO,
        ident_q(),
        &cyl,
        Vec3::new(0.8, 0.0, 0.0),
        ident_q(),
        &s,
    );
    assert!(result.is_some(), "dispatch cylinder-sphere should collide");
}

#[test]
fn dispatch_sphere_cylinder() {
    let s = Collider::sphere(0.5);
    let cyl = Collider::cylinder(0.5, 2.0);
    let result = check_collision(
        Vec3::new(0.8, 0.0, 0.0),
        ident_q(),
        &s,
        Vec3::ZERO,
        ident_q(),
        &cyl,
    );
    assert!(result.is_some(), "dispatch sphere-cylinder should collide");
    let info = result.unwrap();
    // Convention: normal points from A toward B
    // A=sphere at (0.8,0,0), B=cylinder at origin → normal = (-1,0,0)
    assert!(
        info.normal.x < -0.5,
        "normal should point from A(sphere) toward B(cylinder) (-X), got {:?}",
        info.normal
    );
}

#[test]
fn dispatch_cylinder_box() {
    let cyl = Collider::cylinder(0.5, 2.0);
    let b = Collider::cuboid(0.5, 0.5, 0.5);
    let result = check_collision(
        Vec3::ZERO,
        ident_q(),
        &cyl,
        Vec3::new(0.3, 0.0, 0.0),
        ident_q(),
        &b,
    );
    assert!(result.is_some(), "dispatch cylinder-box should collide");
}

#[test]
fn dispatch_box_cylinder() {
    let b = Collider::cuboid(0.5, 0.5, 0.5);
    let cyl = Collider::cylinder(0.5, 2.0);
    let result = check_collision(
        Vec3::new(0.3, 0.0, 0.0),
        ident_q(),
        &b,
        Vec3::ZERO,
        ident_q(),
        &cyl,
    );
    assert!(result.is_some(), "dispatch box-cylinder should collide");
}

#[test]
fn dispatch_cylinder_no_collision() {
    let cyl = Collider::cylinder(0.5, 2.0);
    let s = Collider::sphere(0.5);
    let result = check_collision(
        Vec3::ZERO,
        ident_q(),
        &cyl,
        Vec3::new(10.0, 0.0, 0.0),
        ident_q(),
        &s,
    );
    assert!(result.is_none(), "dispatch: far apart should not collide");
}
