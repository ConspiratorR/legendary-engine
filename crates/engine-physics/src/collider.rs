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
    fn get_bounding_sphere(&self) -> f32 {
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
