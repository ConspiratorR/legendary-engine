use engine_math::Vec3;

/// A 2D-friendly transform with position, Euler rotation, and scale.
///
/// This is the core transform type used by the engine. For 3D scene-graph
/// transforms, see [`engine_scene::transform::Transform`].
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// World or local position.
    pub position: Vec3,
    /// Euler rotation in radians (x, y, z).
    pub rotation: Vec3,
    /// Scale factors per axis.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    /// Create a new transform (identity).
    pub fn new() -> Self {
        Self::identity()
    }

    /// Create an identity transform (zero position, zero rotation, unit scale).
    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }

    /// Create a transform at the given position.
    pub fn from_position(pos: Vec3) -> Self {
        Self {
            position: pos,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }

    /// Create a transform at `(x, y, 0)`.
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self::from_position(Vec3::new(x, y, 0.0))
    }

    /// Create a transform at `(x, y, z)`.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_position(Vec3::new(x, y, z))
    }

    /// Set the rotation (builder pattern).
    pub fn with_rotation(mut self, rotation: Vec3) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the scale (builder pattern).
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Translate by a delta vector.
    pub fn translate(&mut self, delta: Vec3) {
        self.position += delta;
    }

    /// Translate by `(x, y)` on the XY plane.
    pub fn translate_xy(&mut self, x: f32, y: f32) {
        self.position.x += x;
        self.position.y += y;
    }

    /// Rotate by a delta Euler vector.
    pub fn rotate(&mut self, delta: Vec3) {
        self.rotation += delta;
    }

    /// Multiply scale by a factor vector.
    pub fn scale_by(&mut self, factor: Vec3) {
        self.scale *= factor;
    }

    /// Orient the transform to face a target position (2D: sets `rotation.z`).
    pub fn look_at(&mut self, target: Vec3) {
        let direction = target - self.position;
        if direction.length_squared() > 0.0001 {
            self.rotation.z = direction.y.atan2(direction.x);
        }
    }

    /// Return the forward direction vector (based on `rotation.z`).
    pub fn forward(&self) -> Vec3 {
        Vec3::new(self.rotation.z.cos(), self.rotation.z.sin(), 0.0)
    }

    /// Return the right direction vector (based on `rotation.z`).
    pub fn right(&self) -> Vec3 {
        Vec3::new(-self.rotation.z.sin(), self.rotation.z.cos(), 0.0)
    }

    /// Compute world-space position, optionally relative to a parent transform.
    pub fn world_position(&self, parent: Option<&Transform>) -> Vec3 {
        match parent {
            Some(parent) => parent.world_position(None) + self.position,
            None => self.position,
        }
    }
}

impl std::ops::Add for Transform {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            position: self.position + other.position,
            rotation: self.rotation + other.rotation,
            scale: self.scale * other.scale,
        }
    }
}
