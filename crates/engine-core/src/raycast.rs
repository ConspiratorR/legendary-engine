//! Ray casting types (matches Unity's Ray, RaycastHit, Physics).

use engine_math::Vec3;

/// A ray in 3D space (matches Unity's `Ray`).
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    pub fn GetPoint(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

/// Result of a raycast hit (matches Unity's `RaycastHit`).
#[derive(Debug, Clone)]
pub struct RaycastHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub collider: Option<crate::gameobject::GameObjectHandle>,
    pub rigidbody: Option<crate::gameobject::GameObjectHandle>,
    pub transform: Option<crate::gameobject::GameObjectHandle>,
    pub triangle_index: Option<u32>,
    pub texture_coord: Option<engine_math::Vec2>,
}

/// Layer mask for physics queries (matches Unity's `LayerMask`).
#[derive(Debug, Clone, Copy, Default)]
pub struct LayerMask(pub i32);

impl LayerMask {
    pub fn NameToLayer(name: &str) -> i32 {
        match name {
            "Default" => 0,
            "TransparentFX" => 1,
            "Ignore Raycast" => 2,
            "Water" => 4,
            "UI" => 5,
            _ => -1,
        }
    }

    pub fn LayerToName(layer: i32) -> &'static str {
        match layer {
            0 => "Default",
            1 => "TransparentFX",
            2 => "Ignore Raycast",
            4 => "Water",
            5 => "UI",
            _ => "",
        }
    }

    pub fn GetMask(names: &[&str]) -> i32 {
        let mut mask = 0i32;
        for name in names {
            let layer = Self::NameToLayer(name);
            if layer >= 0 {
                mask |= 1 << layer;
            }
        }
        mask
    }
}

impl std::ops::BitOr for LayerMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        LayerMask(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for LayerMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        LayerMask(self.0 & rhs.0)
    }
}
