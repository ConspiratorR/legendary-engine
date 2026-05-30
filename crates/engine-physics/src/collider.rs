//! Collider component for collision detection.
use engine_math::Vec3;

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
                radius * radius + (height * 0.5) * (height * 0.5)
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
}

/// Collision info.
#[derive(Debug, Clone)]
pub struct CollisionInfo {
    pub other_entity: u64,
    pub normal: Vec3,
    pub depth: f32,
    pub point: Vec3,
}

/// Check collision between two AABB boxes.
pub fn check_box_box(pos1: Vec3, half1: Vec3, pos2: Vec3, half2: Vec3) -> Option<CollisionInfo> {
    let delta = pos2 - pos1;
    let overlap_x = half1.x + half2.x - delta.x.abs();
    let overlap_y = half1.y + half2.y - delta.y.abs();
    let overlap_z = half1.z + half2.z - delta.z.abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
        return None;
    }

    // Find axis of minimum penetration
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
        // Inside the box — find axis of minimum penetration
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

/// Physics layers for collision detection.
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

/// Check collision between two colliders given their positions.
pub fn check_collision(
    pos_a: Vec3,
    collider_a: &Collider,
    pos_b: Vec3,
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
        ) => check_box_box(a_pos, *h1, b_pos, *h2),
        (
            ColliderShape::Sphere { radius },
            ColliderShape::Box {
                half_extents: h, ..
            },
        ) => check_sphere_box(a_pos, *radius, b_pos, *h),
        (
            ColliderShape::Box {
                half_extents: h, ..
            },
            ColliderShape::Sphere { radius },
        ) => {
            // Flip normal direction
            check_sphere_box(b_pos, *radius, a_pos, *h).map(|mut info| {
                info.normal = -info.normal;
                info
            })
        }
        // For unsupported shapes, treat capsule/cylinder as sphere using bounding sphere
        _ => {
            let r_a = collider_a.shape.get_bounding_sphere();
            let r_b = collider_b.shape.get_bounding_sphere();
            check_sphere_sphere(a_pos, r_a, b_pos, r_b)
        }
    }
}
