//! Axis-aligned bounding box (matches Unity's Bounds).

use engine_math::Vec3;

/// Axis-aligned bounding box (matches Unity's `Bounds`).
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub center: Vec3,
    pub extents: Vec3,
}

impl Bounds {
    pub fn new(center: Vec3, size: Vec3) -> Self {
        Self {
            center,
            extents: size * 0.5,
        }
    }

    pub fn size(&self) -> Vec3 {
        self.extents * 2.0
    }

    pub fn min(&self) -> Vec3 {
        self.center - self.extents
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.extents
    }

    pub fn Contains(&self, point: Vec3) -> bool {
        let min = self.min();
        let max = self.max();
        point.x >= min.x
            && point.x <= max.x
            && point.y >= min.y
            && point.y <= max.y
            && point.z >= min.z
            && point.z <= max.z
    }

    pub fn Intersects(&self, other: &Bounds) -> bool {
        let a_min = self.min();
        let a_max = self.max();
        let b_min = other.min();
        let b_max = other.max();
        a_min.x <= b_max.x
            && a_max.x >= b_min.x
            && a_min.y <= b_max.y
            && a_max.y >= b_min.y
            && a_min.z <= b_max.z
            && a_max.z >= b_min.z
    }

    pub fn Encapsulate(&mut self, point: Vec3) {
        let min = self.min();
        let max = self.max();
        let new_min = Vec3::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z));
        let new_max = Vec3::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z));
        self.center = (new_min + new_max) * 0.5;
        self.extents = (new_max - new_min) * 0.5;
    }

    pub fn EncapsulateBounds(&mut self, other: &Bounds) {
        self.Encapsulate(other.min());
        self.Encapsulate(other.max());
    }

    pub fn ClosestPoint(&self, point: Vec3) -> Vec3 {
        let min = self.min();
        let max = self.max();
        Vec3::new(
            point.x.clamp(min.x, max.x),
            point.y.clamp(min.y, max.y),
            point.z.clamp(min.z, max.z),
        )
    }

    pub fn SqrDistance(&self, point: Vec3) -> f32 {
        let closest = self.ClosestPoint(point);
        (closest - point).length_squared()
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            extents: Vec3::ZERO,
        }
    }
}
