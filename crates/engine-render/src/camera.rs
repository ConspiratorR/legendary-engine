//! Camera system with projection, viewport, and render target management.
//!
//! Cameras define how the 3D scene is projected onto the 2D screen. They support
//! both perspective and orthographic projections, configurable viewports, and
//! can render to either the screen or off-screen textures.

use engine_math::Mat4;

/// RGBA color with floating-point components.
///
/// Colors are typically in linear space and converted to wgpu color format
/// for clear values and other GPU operations.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Black color (0, 0, 0, 1).
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// White color (1, 1, 1, 1).
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Transparent color (0, 0, 0, 0).
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Create a new color with the given RGBA components.
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Convert to wgpu color format.
    pub fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

/// Projection type for cameras.
///
/// Determines how 3D coordinates are mapped to 2D screen space.
#[derive(Debug, Clone)]
pub enum Projection {
    /// Perspective projection with field of view, near and far planes.
    Perspective {
        /// Vertical field of view in radians.
        fov_y: f32,
        /// Near clipping plane distance.
        near: f32,
        /// Far clipping plane distance.
        far: f32,
    },
    /// Orthographic projection with explicit bounds.
    Orthographic {
        /// Left boundary.
        left: f32,
        /// Right boundary.
        right: f32,
        /// Bottom boundary.
        bottom: f32,
        /// Top boundary.
        top: f32,
        /// Near clipping plane.
        near: f32,
        /// Far clipping plane.
        far: f32,
    },
}

impl Projection {
    /// Create a perspective projection.
    ///
    /// # Arguments
    /// * `fov_y` - Vertical field of view in radians
    /// * `near` - Near clipping plane distance
    /// * `far` - Far clipping plane distance
    pub fn perspective(fov_y: f32, near: f32, far: f32) -> Self {
        Self::Perspective { fov_y, near, far }
    }

    /// Create an orthographic projection.
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self::Orthographic {
            left,
            right,
            bottom,
            top,
            near,
            far,
        }
    }

    /// Compute the projection matrix for the given aspect ratio.
    pub fn matrix(&self, aspect: f32) -> Mat4 {
        match *self {
            Self::Perspective { fov_y, near, far } => {
                Mat4::perspective_rh(fov_y, aspect, near, far)
            }
            Self::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => Mat4::orthographic_rh(left, right, bottom, top, near, far),
        }
    }
}

/// Viewport region for a camera.
///
/// Defines the portion of the render target that the camera renders to.
/// This enables split-screen rendering and picture-in-picture effects.
#[derive(Debug, Clone)]
pub enum Viewport {
    /// Absolute pixel coordinates.
    Absolute {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    /// Normalized coordinates (0.0-1.0) relative to render target size.
    Relative {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

impl Viewport {
    /// Convert to absolute pixel coordinates given the render target size.
    pub fn to_absolute(&self, target_width: u32, target_height: u32) -> (u32, u32, u32, u32) {
        match *self {
            Self::Absolute {
                x,
                y,
                width,
                height,
            } => (x, y, width, height),
            Self::Relative {
                x,
                y,
                width,
                height,
            } => {
                let tw = target_width as f32;
                let th = target_height as f32;
                (
                    (x * tw) as u32,
                    (y * th) as u32,
                    (width * tw) as u32,
                    (height * th) as u32,
                )
            }
        }
    }
}

/// Render target for a camera.
///
/// Cameras can render to either the screen (swapchain) or to an off-screen
/// texture for effects like picture-in-picture or post-processing chains.
#[derive(Debug, Clone)]
pub enum RenderTarget {
    /// Render to the screen (swapchain).
    Screen,
    /// Render to a texture (for picture-in-picture, post-processing, etc.).
    /// The u64 is a texture store key.
    Texture(u64),
}

/// Camera component for rendering the scene.
///
/// Attach to any ECS entity to define a viewpoint from which the scene is rendered.
/// Cameras support priority-based ordering for multi-camera setups (e.g., split-screen,
/// UI overlay cameras).
///
/// # Example
///
/// ```rust
/// use engine_render::camera::{Camera, Projection, Viewport};
///
/// // Create a perspective camera
/// let mut camera = Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
/// camera.priority = 10;
///
/// // Compute view-projection matrix
/// let vp = camera.view_projection(16.0 / 9.0);
/// ```
#[derive(Debug, Clone)]
pub struct Camera {
    /// The projection type (perspective or orthographic).
    pub projection: Projection,
    /// The view matrix (world-to-camera transform).
    pub view: Mat4,
    /// Rendering priority (lower values render first).
    pub priority: i32,
    /// The viewport region within the render target.
    pub viewport: Viewport,
    /// The render target (screen or off-screen texture).
    pub render_target: RenderTarget,
    /// Whether this camera is actively rendering.
    pub is_active: bool,
    /// Optional clear color for this camera's render pass.
    pub clear_color: Option<Color>,
}

impl Camera {
    /// Create a new camera with the given projection.
    pub fn new(projection: Projection) -> Self {
        Self {
            projection,
            view: Mat4::IDENTITY,
            priority: 0,
            viewport: Viewport::Relative {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            render_target: RenderTarget::Screen,
            is_active: true,
            clear_color: Some(Color::BLACK),
        }
    }

    /// Create a perspective camera.
    ///
    /// # Arguments
    /// * `fov_y` - Vertical field of view in radians
    /// * `near` - Near clipping plane distance
    /// * `far` - Far clipping plane distance
    pub fn perspective(fov_y: f32, near: f32, far: f32) -> Self {
        Self::new(Projection::perspective(fov_y, near, far))
    }

    /// Create an orthographic camera with default near/far planes (-1.0 to 1.0).
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Self::new(Projection::orthographic(
            left, right, bottom, top, -1.0, 1.0,
        ))
    }

    /// Compute the combined view-projection matrix.
    ///
    /// This matrix transforms world coordinates to clip space.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection.matrix(aspect) * self.view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_relative_to_absolute() {
        let vp = Viewport::Relative {
            x: 0.5,
            y: 0.0,
            width: 0.5,
            height: 1.0,
        };
        let (x, y, w, h) = vp.to_absolute(800, 600);
        assert_eq!(x, 400);
        assert_eq!(y, 0);
        assert_eq!(w, 400);
        assert_eq!(h, 600);
    }

    #[test]
    fn test_viewport_absolute_passthrough() {
        let vp = Viewport::Absolute {
            x: 10,
            y: 20,
            width: 300,
            height: 200,
        };
        let (x, y, w, h) = vp.to_absolute(800, 600);
        assert_eq!(x, 10);
        assert_eq!(y, 20);
        assert_eq!(w, 300);
        assert_eq!(h, 200);
    }

    #[test]
    fn test_camera_priority_sort() {
        let mut cameras = vec![
            Camera::perspective(1.0, 0.1, 100.0),
            Camera::perspective(1.0, 0.1, 100.0),
            Camera::perspective(1.0, 0.1, 100.0),
        ];
        cameras[0].priority = 10;
        cameras[1].priority = 0;
        cameras[2].priority = 5;
        cameras.sort_by_key(|c| c.priority);
        assert_eq!(cameras[0].priority, 0);
        assert_eq!(cameras[1].priority, 5);
        assert_eq!(cameras[2].priority, 10);
    }

    #[test]
    fn test_color_to_wgpu() {
        let c = Color::new(0.5, 0.25, 0.1, 1.0);
        let w = c.to_wgpu();
        assert!((w.r - 0.5).abs() < 1e-6);
        assert!((w.g - 0.25).abs() < 1e-6);
        assert!((w.b - 0.1).abs() < 1e-6);
        assert!((w.a - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_camera_view_projection() {
        let cam = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
        let vp = cam.view_projection(800.0 / 600.0);
        assert_ne!(vp, Mat4::ZERO);
    }
}
