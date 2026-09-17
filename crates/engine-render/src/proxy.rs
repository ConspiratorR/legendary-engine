//! ECS-side lightweight proxies written by the Unity identity bridge.

use engine_math::{Quat, Vec3};

/// World-space pose of a linked Unity object (P2 render/physics bridge).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformProxy {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for TransformProxy {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}
