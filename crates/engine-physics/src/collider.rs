//! Collider component for collision detection.

#![allow(clippy::too_many_arguments)]
use engine_math::{Quat, Vec3};

/// Shape of the collider.
#[derive(Debug, Clone)]
pub enum ColliderShape {
    /// Sphere shape with radius.
    Sphere { radius: f32 },
    /// Box shape with half-extents.
    Box { half_extents: Vec3 },
    /// Capsule shape.
    Capsule { radius: f32, height: f32 },
    /// Cylinder shape.
    Cylinder { radius: f32, height: f32 },
}

impl ColliderShape {
    pub fn get_bounding_sphere(&self) -> f32 {
        match self {
            ColliderShape::Sphere { radius } => *radius,
            ColliderShape::Box { half_extents } => half_extents.length(),
            ColliderShape::Capsule { radius, height } => radius + height * 0.5,
            ColliderShape::Cylinder { radius, height } => {
                (radius * radius + (height * 0.5) * (height * 0.5)).sqrt()
            }
        }
    }
}

/// Collider component.
#[derive(Debug, Clone)]
pub struct Collider {
    pub shape: ColliderShape,
    pub is_sensor: bool,
    pub friction: f32,
    pub restitution: f32,
    pub density: f32,
    pub offset: Vec3,
    /// Which collision layers this collider belongs to (bitmask).
    pub collision_layers: u32,
    /// Which collision layers this collider can collide with (bitmask).
    pub collision_mask: u32,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Box {
                half_extents: Vec3::new(0.5, 0.5, 0.5),
            },
            is_sensor: false,
            friction: 0.5,
            restitution: 0.3,
            density: 1.0,
            offset: Vec3::new(0.0, 0.0, 0.0),
            collision_layers: 0xFFFF_FFFF,
            collision_mask: 0xFFFF_FFFF,
        }
    }
}

impl Collider {
    pub fn sphere(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Sphere { radius },
            ..Default::default()
        }
    }

    pub fn cuboid(half_x: f32, half_y: f32, half_z: f32) -> Self {
        Self {
            shape: ColliderShape::Box {
                half_extents: Vec3::new(half_x, half_y, half_z),
            },
            ..Default::default()
        }
    }

    pub fn capsule(radius: f32, height: f32) -> Self {
        Self {
            shape: ColliderShape::Capsule { radius, height },
            ..Default::default()
        }
    }

    pub fn cylinder(radius: f32, height: f32) -> Self {
        Self {
            shape: ColliderShape::Cylinder { radius, height },
            ..Default::default()
        }
    }

    /// Check if this collider can collide with another based on layer masks.
    pub fn can_collide_with(&self, other: &Collider) -> bool {
        (self.collision_layers & other.collision_mask) != 0
            && (other.collision_layers & self.collision_mask) != 0
    }
}

/// Collision info.
#[derive(Debug, Clone)]
pub struct CollisionInfo {
    pub other_entity: u64,
    pub normal: Vec3,
    pub depth: f32,
    pub point: Vec3,
}

// ---------------------------------------------------------------------------
// AABB helpers (axis-aligned, kept for backward compatibility)
// ---------------------------------------------------------------------------

/// Check collision between two AABB boxes.
pub fn check_box_box(pos1: Vec3, half1: Vec3, pos2: Vec3, half2: Vec3) -> Option<CollisionInfo> {
    let delta = pos2 - pos1;
    let overlap_x = half1.x + half2.x - delta.x.abs();
    let overlap_y = half1.y + half2.y - delta.y.abs();
    let overlap_z = half1.z + half2.z - delta.z.abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
        return None;
    }

    let (normal, depth) = if overlap_x < overlap_y && overlap_x < overlap_z {
        (
            Vec3::new(if delta.x >= 0.0 { 1.0 } else { -1.0 }, 0.0, 0.0),
            overlap_x,
        )
    } else if overlap_y < overlap_z {
        (
            Vec3::new(0.0, if delta.y >= 0.0 { 1.0 } else { -1.0 }, 0.0),
            overlap_y,
        )
    } else {
        (
            Vec3::new(0.0, 0.0, if delta.z >= 0.0 { 1.0 } else { -1.0 }),
            overlap_z,
        )
    };

    let point = pos1
        + normal
            * (if normal.x != 0.0 {
                half1.x
            } else if normal.y != 0.0 {
                half1.y
            } else {
                half1.z
            });

    Some(CollisionInfo {
        other_entity: 0,
        normal,
        depth,
        point,
    })
}

/// Check collision between a sphere and an AABB box.
pub fn check_sphere_box(
    sphere_pos: Vec3,
    radius: f32,
    box_pos: Vec3,
    half_extents: Vec3,
) -> Option<CollisionInfo> {
    let delta = sphere_pos - box_pos;
    let closest = Vec3::new(
        delta.x.clamp(-half_extents.x, half_extents.x),
        delta.y.clamp(-half_extents.y, half_extents.y),
        delta.z.clamp(-half_extents.z, half_extents.z),
    );

    let diff = delta - closest;
    let dist_sq = diff.length_squared();

    if dist_sq >= radius * radius || dist_sq < f32::EPSILON {
        let overlap_x = half_extents.x + radius - delta.x.abs();
        let overlap_y = half_extents.y + radius - delta.y.abs();
        let overlap_z = half_extents.z + radius - delta.z.abs();

        if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
            return None;
        }

        let (normal, depth) = if overlap_x < overlap_y && overlap_x < overlap_z {
            (
                Vec3::new(if delta.x >= 0.0 { 1.0 } else { -1.0 }, 0.0, 0.0),
                overlap_x,
            )
        } else if overlap_y < overlap_z {
            (
                Vec3::new(0.0, if delta.y >= 0.0 { 1.0 } else { -1.0 }, 0.0),
                overlap_y,
            )
        } else {
            (
                Vec3::new(0.0, 0.0, if delta.z >= 0.0 { 1.0 } else { -1.0 }),
                overlap_z,
            )
        };

        return Some(CollisionInfo {
            other_entity: 0,
            normal,
            depth,
            point: sphere_pos - normal * radius,
        });
    }

    let distance = dist_sq.sqrt();
    let normal = diff / distance;
    let depth = radius - distance;
    let point = sphere_pos - normal * radius;

    Some(CollisionInfo {
        other_entity: 0,
        normal,
        depth,
        point,
    })
}

// ---------------------------------------------------------------------------
// Generic sphere-sphere (no rotation needed)
// ---------------------------------------------------------------------------

pub fn check_sphere_sphere(
    pos1: Vec3,
    radius1: f32,
    pos2: Vec3,
    radius2: f32,
) -> Option<CollisionInfo> {
    let delta = pos2 - pos1;
    let distance = delta.length();
    let combined_radius = radius1 + radius2;

    if distance < combined_radius && distance > 0.0 {
        let normal = delta / distance;
        let depth = combined_radius - distance;
        let point = pos1 + normal * radius1;
        Some(CollisionInfo {
            other_entity: 0,
            normal,
            depth,
            point,
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// OBB collision – Separating Axis Theorem
// ---------------------------------------------------------------------------

/// SAT-based OBB-OBB collision.
pub fn check_obb_obb(
    pos_a: Vec3,
    rot_a: Quat,
    half_a: Vec3,
    pos_b: Vec3,
    rot_b: Quat,
    half_b: Vec3,
) -> Option<CollisionInfo> {
    let axes_a = [rot_a * Vec3::X, rot_a * Vec3::Y, rot_a * Vec3::Z];
    let axes_b = [rot_b * Vec3::X, rot_b * Vec3::Y, rot_b * Vec3::Z];

    let mut test_axes: [Vec3; 15] = [Vec3::ZERO; 15];
    let mut axis_count = 0;

    // Face normals of A and B
    for &ax in &axes_a {
        test_axes[axis_count] = ax;
        axis_count += 1;
    }
    for &ax in &axes_b {
        test_axes[axis_count] = ax;
        axis_count += 1;
    }
    // Edge cross products
    for &ax_a in &axes_a {
        for &ax_b in &axes_b {
            let cross = ax_a.cross(ax_b);
            if cross.length_squared() > f32::EPSILON {
                test_axes[axis_count] = cross.normalize();
                axis_count += 1;
            }
        }
    }

    let center_offset = pos_b - pos_a;
    let mut min_overlap = f32::MAX;
    let mut best_axis = Vec3::ZERO;

    for &test_axis in test_axes.iter().take(axis_count) {
        let axis = test_axis.normalize();
        let proj = center_offset.dot(axis).abs();

        let mut radius_a = 0.0;
        let mut radius_b = 0.0;
        for j in 0..3 {
            radius_a += (axes_a[j].dot(axis)).abs() * half_a[j];
            radius_b += (axes_b[j].dot(axis)).abs() * half_b[j];
        }

        let overlap = radius_a + radius_b - proj;
        if overlap <= 0.0 {
            return None;
        }
        if overlap < min_overlap {
            min_overlap = overlap;
            best_axis = axis;
        }
    }

    if best_axis.dot(center_offset) < 0.0 {
        best_axis = -best_axis;
    }

    let point = pos_a
        + best_axis
            * (half_a.x * (axes_a[0].dot(best_axis)).abs()
                + half_a.y * (axes_a[1].dot(best_axis)).abs()
                + half_a.z * (axes_a[2].dot(best_axis)).abs());

    Some(CollisionInfo {
        other_entity: 0,
        normal: best_axis,
        depth: min_overlap,
        point,
    })
}

/// Sphere vs OBB collision.
pub fn check_sphere_obb(
    sphere_pos: Vec3,
    radius: f32,
    obb_pos: Vec3,
    obb_rot: Quat,
    obb_half: Vec3,
) -> Option<CollisionInfo> {
    let inv_rot = obb_rot.inverse();
    let local_sphere = inv_rot * (sphere_pos - obb_pos);

    let closest = Vec3::new(
        local_sphere.x.clamp(-obb_half.x, obb_half.x),
        local_sphere.y.clamp(-obb_half.y, obb_half.y),
        local_sphere.z.clamp(-obb_half.z, obb_half.z),
    );

    let diff = local_sphere - closest;
    let dist_sq = diff.length_squared();

    if dist_sq >= radius * radius {
        return None;
    }

    let (local_normal, depth, local_point) = if dist_sq < f32::EPSILON {
        let overlap_x = obb_half.x + radius - local_sphere.x.abs();
        let overlap_y = obb_half.y + radius - local_sphere.y.abs();
        let overlap_z = obb_half.z + radius - local_sphere.z.abs();

        if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
            return None;
        }

        let (n, d) = if overlap_x < overlap_y && overlap_x < overlap_z {
            (
                Vec3::new(if local_sphere.x >= 0.0 { 1.0 } else { -1.0 }, 0.0, 0.0),
                overlap_x,
            )
        } else if overlap_y < overlap_z {
            (
                Vec3::new(0.0, if local_sphere.y >= 0.0 { 1.0 } else { -1.0 }, 0.0),
                overlap_y,
            )
        } else {
            (
                Vec3::new(0.0, 0.0, if local_sphere.z >= 0.0 { 1.0 } else { -1.0 }),
                overlap_z,
            )
        };
        (n, d, closest)
    } else {
        let distance = dist_sq.sqrt();
        (diff / distance, radius - distance, closest)
    };

    let world_normal = obb_rot * local_normal;
    let world_point = obb_rot * local_point + obb_pos;

    Some(CollisionInfo {
        other_entity: 0,
        normal: world_normal,
        depth,
        point: world_point,
    })
}

// ---------------------------------------------------------------------------
// Capsule helpers and collision
// ---------------------------------------------------------------------------

/// Capsule segment endpoints in world space (axis is local Y).
fn capsule_segment(pos: Vec3, rot: Quat, height: f32) -> (Vec3, Vec3) {
    let half = height * 0.5;
    let a = pos + rot * Vec3::new(0.0, -half, 0.0);
    let b = pos + rot * Vec3::new(0.0, half, 0.0);
    (a, b)
}

/// Closest point on a line segment to a given point.
fn closest_point_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let ap = p - a;
    let t = (ap.dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    a + ab * t
}

/// Closest points between two line segments in 3D.
fn closest_points_segment_segment(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3, f32) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;

    let a = d1.length_squared();
    let e = d2.length_squared();
    let f = d2.dot(r);

    if a <= f32::EPSILON && e <= f32::EPSILON {
        return (p1, p2, 0.0);
    }

    let (mut s, mut t);
    if a <= f32::EPSILON {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= f32::EPSILON {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;

            if denom.abs() > f32::EPSILON {
                s = (b * f - c * e) / denom;
            } else {
                s = 0.0;
            }

            s = s.clamp(0.0, 1.0);
            t = (b * s + f) / e;

            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
        }
    }

    let cp1 = p1 + d1 * s;
    let cp2 = p2 + d2 * t;
    let dist = (cp2 - cp1).length();
    (cp1, cp2, dist)
}

/// Capsule-capsule collision.
pub fn check_capsule_capsule(
    pos_a: Vec3,
    rot_a: Quat,
    radius_a: f32,
    height_a: f32,
    pos_b: Vec3,
    rot_b: Quat,
    radius_b: f32,
    height_b: f32,
) -> Option<CollisionInfo> {
    let (a1, a2) = capsule_segment(pos_a, rot_a, height_a);
    let (b1, b2) = capsule_segment(pos_b, rot_b, height_b);

    let (cp_a, cp_b, dist) = closest_points_segment_segment(a1, a2, b1, b2);
    let combined = radius_a + radius_b;

    if dist >= combined {
        return None;
    }

    let (normal, depth) = if dist > f32::EPSILON {
        ((cp_b - cp_a) / dist, combined - dist)
    } else {
        (Vec3::Y, combined)
    };

    let point = cp_a + normal * radius_a;

    Some(CollisionInfo {
        other_entity: 0,
        normal,
        depth,
        point,
    })
}

/// Sphere-capsule collision.
pub fn check_sphere_capsule(
    sphere_pos: Vec3,
    sphere_radius: f32,
    cap_pos: Vec3,
    cap_rot: Quat,
    cap_radius: f32,
    cap_height: f32,
) -> Option<CollisionInfo> {
    let (c1, c2) = capsule_segment(cap_pos, cap_rot, cap_height);
    let closest = closest_point_on_segment(sphere_pos, c1, c2);

    let delta = sphere_pos - closest;
    let dist_sq = delta.length_squared();
    let combined = sphere_radius + cap_radius;

    if dist_sq >= combined * combined {
        return None;
    }

    if dist_sq < f32::EPSILON {
        return Some(CollisionInfo {
            other_entity: 0,
            normal: Vec3::Y,
            depth: combined,
            point: sphere_pos - Vec3::Y * sphere_radius,
        });
    }

    let dist = dist_sq.sqrt();
    let normal = delta / dist;
    let depth = combined - dist;
    let point = sphere_pos - normal * sphere_radius;

    Some(CollisionInfo {
        other_entity: 0,
        normal,
        depth,
        point,
    })
}

/// OBB-capsule collision via capsule segment sampling against OBB.
pub fn check_obb_capsule(
    obb_pos: Vec3,
    obb_rot: Quat,
    obb_half: Vec3,
    cap_pos: Vec3,
    cap_rot: Quat,
    cap_radius: f32,
    cap_height: f32,
) -> Option<CollisionInfo> {
    let (c1, c2) = capsule_segment(cap_pos, cap_rot, cap_height);
    let inv_obb = obb_rot.inverse();
    let local_c1 = inv_obb * (c1 - obb_pos);
    let local_c2 = inv_obb * (c2 - obb_pos);
    let local_dir = local_c2 - local_c1;

    let steps = 16;
    let mut min_dist_sq = f32::MAX;
    let mut best_local_p = Vec3::ZERO;
    let mut best_closest = Vec3::ZERO;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let local_p = local_c1 + local_dir * t;

        let closest = Vec3::new(
            local_p.x.clamp(-obb_half.x, obb_half.x),
            local_p.y.clamp(-obb_half.y, obb_half.y),
            local_p.z.clamp(-obb_half.z, obb_half.z),
        );

        let diff = local_p - closest;
        let dist_sq = diff.length_squared();

        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            best_local_p = local_p;
            best_closest = closest;
        }
    }

    let dist = min_dist_sq.sqrt();

    if dist >= cap_radius {
        return None;
    }

    let (local_normal, depth) = if min_dist_sq < f32::EPSILON {
        let overlap_x = obb_half.x + cap_radius - best_local_p.x.abs();
        let overlap_y = obb_half.y + cap_radius - best_local_p.y.abs();
        let overlap_z = obb_half.z + cap_radius - best_local_p.z.abs();

        if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
            return None;
        }

        let (n, d) = if overlap_x < overlap_y && overlap_x < overlap_z {
            (
                Vec3::new(if best_local_p.x >= 0.0 { 1.0 } else { -1.0 }, 0.0, 0.0),
                overlap_x,
            )
        } else if overlap_y < overlap_z {
            (
                Vec3::new(0.0, if best_local_p.y >= 0.0 { 1.0 } else { -1.0 }, 0.0),
                overlap_y,
            )
        } else {
            (
                Vec3::new(0.0, 0.0, if best_local_p.z >= 0.0 { 1.0 } else { -1.0 }),
                overlap_z,
            )
        };
        (n, d)
    } else {
        let diff = best_local_p - best_closest;
        let local_normal = diff / dist;
        (local_normal, cap_radius - dist)
    };

    let world_normal = obb_rot * local_normal;
    let world_point = obb_rot * best_closest + obb_pos;

    Some(CollisionInfo {
        other_entity: 0,
        normal: world_normal,
        depth,
        point: world_point,
    })
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Check collision between two colliders given their positions and rotations.
/// Use [`Quat::IDENTITY`] for shapes that don't require orientation (Sphere).
pub fn check_collision(
    pos_a: Vec3,
    rot_a: Quat,
    collider_a: &Collider,
    pos_b: Vec3,
    rot_b: Quat,
    collider_b: &Collider,
) -> Option<CollisionInfo> {
    let a_pos = pos_a + collider_a.offset;
    let b_pos = pos_b + collider_b.offset;

    match (&collider_a.shape, &collider_b.shape) {
        (ColliderShape::Sphere { radius: r1 }, ColliderShape::Sphere { radius: r2 }) => {
            check_sphere_sphere(a_pos, *r1, b_pos, *r2)
        }
        (
            ColliderShape::Box {
                half_extents: h1, ..
            },
            ColliderShape::Box {
                half_extents: h2, ..
            },
        ) => check_obb_obb(a_pos, rot_a, *h1, b_pos, rot_b, *h2),
        (
            ColliderShape::Sphere { radius },
            ColliderShape::Box {
                half_extents: h, ..
            },
        ) => check_sphere_obb(a_pos, *radius, b_pos, rot_b, *h),
        (
            ColliderShape::Box {
                half_extents: h, ..
            },
            ColliderShape::Sphere { radius },
        ) => check_sphere_obb(b_pos, *radius, a_pos, rot_a, *h).map(|mut info| {
            info.normal = -info.normal;
            info
        }),
        (
            ColliderShape::Capsule {
                radius: r1,
                height: h1,
            },
            ColliderShape::Capsule {
                radius: r2,
                height: h2,
            },
        ) => check_capsule_capsule(a_pos, rot_a, *r1, *h1, b_pos, rot_b, *r2, *h2),
        (
            ColliderShape::Sphere { radius },
            ColliderShape::Capsule {
                radius: cr,
                height: ch,
            },
        ) => check_sphere_capsule(a_pos, *radius, b_pos, rot_b, *cr, *ch),
        (
            ColliderShape::Capsule {
                radius: cr,
                height: ch,
            },
            ColliderShape::Sphere { radius },
        ) => check_sphere_capsule(b_pos, *radius, a_pos, rot_a, *cr, *ch).map(|mut info| {
            info.normal = -info.normal;
            info
        }),
        (
            ColliderShape::Box {
                half_extents: h, ..
            },
            ColliderShape::Capsule {
                radius: cr,
                height: ch,
            },
        ) => check_obb_capsule(a_pos, rot_a, *h, b_pos, rot_b, *cr, *ch),
        (
            ColliderShape::Capsule {
                radius: cr,
                height: ch,
            },
            ColliderShape::Box {
                half_extents: h, ..
            },
        ) => check_obb_capsule(b_pos, rot_b, *h, a_pos, rot_a, *cr, *ch).map(|mut info| {
            info.normal = -info.normal;
            info
        }),
        // Cylinder and other combinations fall back to bounding sphere
        _ => {
            let r_a = collider_a.shape.get_bounding_sphere();
            let r_b = collider_b.shape.get_bounding_sphere();
            check_sphere_sphere(a_pos, r_a, b_pos, r_b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::EulerRot;

    // -----------------------------------------------------------------------
    // OBB-OBB (SAT)
    // -----------------------------------------------------------------------

    #[test]
    fn test_obb_obb_overlap_centered() {
        let result = check_obb_obb(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(0.5, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::splat(1.0),
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
        assert!(info.normal.x != 0.0 || info.normal.y != 0.0 || info.normal.z != 0.0);
    }

    #[test]
    fn test_obb_obb_no_overlap() {
        let result = check_obb_obb(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(5.0, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::splat(1.0),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_obb_obb_edge_overlap() {
        let result = check_obb_obb(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(1.8, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::splat(1.0),
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    #[test]
    fn test_obb_obb_rotated_overlap() {
        let rot = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.5);
        let result = check_obb_obb(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(1.2, 1.2, 0.0),
            rot,
            Vec3::new(1.5, 0.5, 1.0),
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    #[test]
    fn test_obb_obb_rotated_no_overlap() {
        let rot = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.5);
        let result = check_obb_obb(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(5.0, 0.0, 0.0),
            rot,
            Vec3::splat(1.0),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_obb_obb_contact_normal_direction() {
        let result = check_obb_obb(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(1.5, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::splat(1.0),
        );
        assert!(result.is_some());
        let info = result.unwrap();
        // Normal should point from A to B (positive X)
        assert!(info.normal.x > 0.0);
    }

    // -----------------------------------------------------------------------
    // Sphere-OBB
    // -----------------------------------------------------------------------

    #[test]
    fn test_sphere_obb_overlap() {
        let result = check_sphere_obb(
            Vec3::new(1.2, 0.0, 0.0),
            0.5,
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    #[test]
    fn test_sphere_obb_no_overlap() {
        let result = check_sphere_obb(
            Vec3::new(3.0, 0.0, 0.0),
            0.5,
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_sphere_obb_inside() {
        let result = check_sphere_obb(
            Vec3::new(0.3, 0.2, 0.0),
            0.5,
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    #[test]
    fn test_sphere_obb_rotated() {
        let rot = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.3);
        let result = check_sphere_obb(
            Vec3::new(0.2, 0.8, 0.0),
            0.5,
            Vec3::ZERO,
            rot,
            Vec3::new(0.5, 0.5, 0.5),
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    // -----------------------------------------------------------------------
    // Capsule-Capsule
    // -----------------------------------------------------------------------

    #[test]
    fn test_capsule_capsule_overlap_parallel() {
        let result = check_capsule_capsule(
            Vec3::ZERO,
            Quat::IDENTITY,
            0.5,
            2.0,
            Vec3::new(0.6, 0.0, 0.0),
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
        assert!(info.normal.x > 0.0);
    }

    #[test]
    fn test_capsule_capsule_no_overlap_parallel() {
        let result = check_capsule_capsule(
            Vec3::ZERO,
            Quat::IDENTITY,
            0.5,
            2.0,
            Vec3::new(3.0, 0.0, 0.0),
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_capsule_capsule_overlap_angled() {
        let rot = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 1.57);
        let result = check_capsule_capsule(
            Vec3::ZERO,
            Quat::IDENTITY,
            0.5,
            2.0,
            Vec3::ZERO,
            rot,
            0.5,
            2.0,
        );
        // Both at same position with perpendicular axes
        assert!(result.is_some());
        assert!(result.unwrap().depth > 0.0);
    }

    #[test]
    fn test_capsule_capsule_end_to_end() {
        let result = check_capsule_capsule(
            Vec3::ZERO,
            Quat::IDENTITY,
            0.5,
            2.0,
            Vec3::new(0.0, 2.0, 0.0),
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        // End-to-end with hemispheres touching
        assert!(result.is_some());
    }

    // -----------------------------------------------------------------------
    // Sphere-Capsule
    // -----------------------------------------------------------------------

    #[test]
    fn test_sphere_capsule_overlap() {
        let result = check_sphere_capsule(
            Vec3::new(0.7, 0.0, 0.0),
            0.5,
            Vec3::ZERO,
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    #[test]
    fn test_sphere_capsule_no_overlap() {
        let result = check_sphere_capsule(
            Vec3::new(3.0, 0.0, 0.0),
            0.5,
            Vec3::ZERO,
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_sphere_capsule_at_end() {
        let result = check_sphere_capsule(
            Vec3::new(0.0, 1.8, 0.0),
            0.5,
            Vec3::ZERO,
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        assert!(result.is_some());
    }

    // -----------------------------------------------------------------------
    // OBB-Capsule
    // -----------------------------------------------------------------------

    #[test]
    fn test_obb_capsule_overlap() {
        let result = check_obb_capsule(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(0.5, 0.0, 0.0),
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.depth > 0.0);
    }

    #[test]
    fn test_obb_capsule_no_overlap() {
        let result = check_obb_capsule(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::splat(1.0),
            Vec3::new(5.0, 0.0, 0.0),
            Quat::IDENTITY,
            0.5,
            2.0,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_obb_capsule_rotated_obb() {
        let rot = Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 0.5);
        let result = check_obb_capsule(
            Vec3::ZERO,
            rot,
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::new(0.8, 0.8, 0.0),
            Quat::IDENTITY,
            0.3,
            1.0,
        );
        assert!(result.is_some());
    }

    // -----------------------------------------------------------------------
    // Dispatcher integration (via check_collision)
    // -----------------------------------------------------------------------

    fn ident_q() -> Quat {
        Quat::IDENTITY
    }

    #[test]
    fn test_dispatch_obb_obb() {
        let a = Collider::cuboid(1.0, 1.0, 1.0);
        let b = Collider::cuboid(1.0, 1.0, 1.0);
        let result = check_collision(
            Vec3::ZERO,
            ident_q(),
            &a,
            Vec3::new(0.5, 0.0, 0.0),
            ident_q(),
            &b,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_dispatch_sphere_obb() {
        let s = Collider::sphere(0.5);
        let b = Collider::cuboid(1.0, 1.0, 1.0);
        let result = check_collision(
            Vec3::new(1.2, 0.0, 0.0),
            ident_q(),
            &s,
            Vec3::ZERO,
            ident_q(),
            &b,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_dispatch_capsule_capsule() {
        let a = Collider::capsule(0.5, 2.0);
        let b = Collider::capsule(0.5, 2.0);
        let result = check_collision(
            Vec3::ZERO,
            ident_q(),
            &a,
            Vec3::new(0.5, 0.0, 0.0),
            ident_q(),
            &b,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_dispatch_sphere_capsule() {
        let s = Collider::sphere(0.5);
        let c = Collider::capsule(0.5, 2.0);
        let result = check_collision(
            Vec3::new(0.7, 0.0, 0.0),
            ident_q(),
            &s,
            Vec3::ZERO,
            ident_q(),
            &c,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_dispatch_obb_capsule() {
        let b = Collider::cuboid(1.0, 1.0, 1.0);
        let c = Collider::capsule(0.5, 2.0);
        let result = check_collision(
            Vec3::ZERO,
            ident_q(),
            &b,
            Vec3::new(0.5, 0.0, 0.0),
            ident_q(),
            &c,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_dispatch_no_collision() {
        let s = Collider::sphere(0.5);
        let b = Collider::cuboid(1.0, 1.0, 1.0);
        let result = check_collision(
            Vec3::new(5.0, 0.0, 0.0),
            ident_q(),
            &s,
            Vec3::ZERO,
            ident_q(),
            &b,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_dispatch_cylinder_fallback() {
        let cyl = Collider::cylinder(0.5, 1.0);
        let s = Collider::sphere(0.5);
        let result = check_collision(
            Vec3::ZERO,
            ident_q(),
            &cyl,
            Vec3::new(0.5, 0.0, 0.0),
            ident_q(),
            &s,
        );
        assert!(result.is_some());
    }
}
