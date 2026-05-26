use engine_math::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    pub fn new() -> Self {
        Self::identity()
    }

    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }

    pub fn from_position(pos: Vec3) -> Self {
        Self {
            position: pos,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }

    pub fn from_xy(x: f32, y: f32) -> Self {
        Self::from_position(Vec3::new(x, y, 0.0))
    }

    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_position(Vec3::new(x, y, z))
    }

    pub fn with_rotation(mut self, rotation: Vec3) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn translate(&mut self, delta: Vec3) {
        self.position += delta;
    }

    pub fn translate_xy(&mut self, x: f32, y: f32) {
        self.position.x += x;
        self.position.y += y;
    }

    pub fn rotate(&mut self, delta: Vec3) {
        self.rotation += delta;
    }

    pub fn scale_by(&mut self, factor: Vec3) {
        self.scale *= factor;
    }

    pub fn look_at(&mut self, target: Vec3) {
        let direction = target - self.position;
        if direction.length_squared() > 0.0001 {
            self.rotation.z = direction.y.atan2(direction.x);
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(self.rotation.z.cos(), self.rotation.z.sin(), 0.0)
    }

    pub fn right(&self) -> Vec3 {
        Vec3::new(-self.rotation.z.sin(), self.rotation.z.cos(), 0.0)
    }

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
