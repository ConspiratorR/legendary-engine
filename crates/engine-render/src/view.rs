use engine_math::Mat4;

pub struct Camera {
    pub projection: Projection,
    pub view: Mat4,
}

pub enum Projection {
    Perspective {
        fov_y: f32,
        near: f32,
        far: f32,
    },
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
}

impl Camera {
    pub fn perspective(fov_y: f32, _aspect: f32, near: f32, far: f32) -> Self {
        Self {
            projection: Projection::Perspective { fov_y, near, far },
            view: Mat4::IDENTITY,
        }
    }

    pub fn view_projection_matrix(&self, aspect: f32) -> Mat4 {
        let proj_matrix = match self.projection {
            Projection::Perspective { fov_y, near, far } => {
                Mat4::perspective_rh(fov_y, aspect, near, far)
            }
            Projection::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => Mat4::orthographic_rh(left, right, bottom, top, near, far),
        };
        proj_matrix * self.view
    }
}

pub struct View {
    pub camera: Camera,
    pub viewport: (u32, u32),
}

impl View {
    pub fn new(camera: Camera, width: u32, height: u32) -> Self {
        Self {
            camera,
            viewport: (width, height),
        }
    }

    pub fn aspect(&self) -> f32 {
        self.viewport.0 as f32 / self.viewport.1.max(1) as f32
    }
}
