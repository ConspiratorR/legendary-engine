# Camera System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement multi-camera rendering with frustum culling (2D AABB + 3D sphere/AABB) and render-to-texture support, using Camera as an ECS component.

**Architecture:** Camera becomes an ECS component with priority, viewport, and render target. Frustum planes are extracted from the view-projection matrix for culling. The Renderer iterates cameras sorted by priority, culls sprites per camera, sets viewport/scissor, and submits per-camera render passes. Render-to-texture is supported via `RenderTarget::Texture`.

**Tech Stack:** Rust, wgpu, glam (Vec3/Vec4/Mat4), engine-ecs (World, Query), engine-render (Renderer, SpriteBatch, TextureStore)

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/engine-render/src/camera.rs` | Create | Camera, Viewport, RenderTarget, Color |
| `crates/engine-render/src/frustum.rs` | Create | Frustum plane extraction, AABB/sphere tests |
| `crates/engine-render/src/view.rs` | Modify | Remove old Camera/Projection/View, re-export from camera.rs |
| `crates/engine-render/src/renderer.rs` | Modify | `render_frame()` multi-camera + RTT, viewport/scissor |
| `crates/engine-render/src/texture_store.rs` | Modify | `create_render_target()`, render target texture management |
| `crates/engine-render/src/lib.rs` | Modify | Export `camera` and `frustum` modules |
| `crates/engine-core/examples/sprite_demo.rs` | Modify | Use new Camera component, multi-camera demo |

---

### Task 1: Camera Component Types

**Files:**
- Create: `crates/engine-render/src/camera.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: Create camera.rs with core types**

```rust
// crates/engine-render/src/camera.rs

use engine_math::{Mat4, Vec3};

/// RGBA color.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

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
#[derive(Debug, Clone)]
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

impl Projection {
    pub fn perspective(fov_y: f32, near: f32, far: f32) -> Self {
        Self::Perspective { fov_y, near, far }
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self::Orthographic { left, right, bottom, top, near, far }
    }

    pub fn matrix(&self, aspect: f32) -> Mat4 {
        match *self {
            Self::Perspective { fov_y, near, far } => {
                Mat4::perspective_rh(fov_y, aspect, near, far)
            }
            Self::Orthographic { left, right, bottom, top, near, far } => {
                Mat4::orthographic_rh(left, right, bottom, top, near, far)
            }
        }
    }
}

/// Viewport region for a camera.
#[derive(Debug, Clone)]
pub enum Viewport {
    /// Absolute pixel coordinates.
    Absolute { x: u32, y: u32, width: u32, height: u32 },
    /// Normalized coordinates (0.0-1.0) relative to render target size.
    Relative { x: f32, y: f32, width: f32, height: f32 },
}

impl Viewport {
    /// Convert to absolute pixel coordinates given the render target size.
    pub fn to_absolute(&self, target_width: u32, target_height: u32) -> (u32, u32, u32, u32) {
        match *self {
            Self::Absolute { x, y, width, height } => (x, y, width, height),
            Self::Relative { x, y, width, height } => {
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
#[derive(Debug, Clone)]
pub enum RenderTarget {
    /// Render to the screen (swapchain).
    Screen,
    /// Render to a texture (for picture-in-picture, post-processing, etc.).
    /// The u64 is a texture store key.
    Texture(u64),
}

/// Camera component — attach to any ECS entity.
#[derive(Debug, Clone)]
pub struct Camera {
    pub projection: Projection,
    pub view: Mat4,
    pub priority: i32,
    pub viewport: Viewport,
    pub render_target: RenderTarget,
    pub is_active: bool,
    pub clear_color: Option<Color>,
}

impl Camera {
    pub fn new(projection: Projection) -> Self {
        Self {
            projection,
            view: Mat4::IDENTITY,
            priority: 0,
            viewport: Viewport::Relative { x: 0.0, y: 0.0, width: 1.0, height: 1.0 },
            render_target: RenderTarget::Screen,
            is_active: true,
            clear_color: Some(Color::BLACK),
        }
    }

    pub fn perspective(fov_y: f32, near: f32, far: f32) -> Self {
        Self::new(Projection::perspective(fov_y, near, far))
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Self::new(Projection::orthographic(left, right, bottom, top, -1.0, 1.0))
    }

    /// Compute the combined view-projection matrix.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection.matrix(aspect) * self.view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_relative_to_absolute() {
        let vp = Viewport::Relative { x: 0.5, y: 0.0, width: 0.5, height: 1.0 };
        let (x, y, w, h) = vp.to_absolute(800, 600);
        assert_eq!(x, 400);
        assert_eq!(y, 0);
        assert_eq!(w, 400);
        assert_eq!(h, 600);
    }

    #[test]
    fn test_viewport_absolute_passthrough() {
        let vp = Viewport::Absolute { x: 10, y: 20, width: 300, height: 200 };
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
        // Should produce a valid matrix (not all zeros)
        assert_ne!(vp, Mat4::ZERO);
    }
}
```

- [ ] **Step 2: Update lib.rs to export camera module**

Add to `crates/engine-render/src/lib.rs`:

```rust
pub mod camera;
```

- [ ] **Step 3: Run tests to verify**

Run: `cargo test -p engine-render`
Expected: All camera tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/camera.rs crates/engine-render/src/lib.rs
git commit -m "feat(render): add Camera, Projection, Viewport, RenderTarget components"
```

---

### Task 2: Frustum Culling

**Files:**
- Create: `crates/engine-render/src/frustum.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: Create frustum.rs with Frustum struct and tests**

```rust
// crates/engine-render/src/frustum.rs

use engine_math::{Mat4, Vec3, Vec4};

/// A view frustum defined by 6 planes.
///
/// Planes: 0=left, 1=right, 2=bottom, 3=top, 4=near, 5=far.
/// Each plane is (a, b, c, d) where ax + by + cz + d = 0.
/// The normal points inward (positive half-space is inside).
#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [Vec4; 6],
}

impl Frustum {
    /// Extract frustum planes from a combined view-projection matrix.
    ///
    /// Uses the Gribb-Hartmann method.
    pub fn from_view_projection(vp: &Mat4) -> Self {
        let row0 = vp.row(0);
        let row1 = vp.row(1);
        let row2 = vp.row(2);
        let row3 = vp.row(3);

        // Left:   row3 + row0
        // Right:  row3 - row0
        // Bottom: row3 + row1
        // Top:    row3 - row1
        // Near:   row3 + row2  (for RH: row2 already points inward)
        // Far:    row3 - row2
        let planes = [
            normalize_plane(row3 + row0), // left
            normalize_plane(row3 - row0), // right
            normalize_plane(row3 + row1), // bottom
            normalize_plane(row3 - row1), // top
            normalize_plane(row3 + row2), // near
            normalize_plane(row3 - row2), // far
        ];
        Self { planes }
    }

    /// Test if an AABB (axis-aligned bounding box) intersects the frustum.
    /// Returns true if the AABB is at least partially inside the frustum.
    pub fn test_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            // Find the p-vertex (corner most in the direction of the normal)
            let p = Vec3::new(
                if plane.x >= 0.0 { max.x } else { min.x },
                if plane.y >= 0.0 { max.y } else { min.y },
                if plane.z >= 0.0 { max.z } else { min.z },
            );
            // If p-vertex is outside, the whole AABB is outside
            if plane.x * p.x + plane.y * p.y + plane.z * p.z + plane.w < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test if a sphere intersects the frustum.
    /// Returns true if the sphere is at least partially inside the frustum.
    pub fn test_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            let dist = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
            if dist < -radius {
                return false;
            }
        }
        true
    }
}

/// Normalize a plane so that the normal has unit length.
fn normalize_plane(p: Vec4) -> Vec4 {
    let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
    if len < 1e-10 {
        return p;
    }
    Vec4::new(p.x / len, p.y / len, p.z / len, p.w / len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_frustum() -> Frustum {
        // Identity VP means the frustum is the canonical [-1,1] cube
        Frustum::from_view_projection(&Mat4::IDENTITY)
    }

    #[test]
    fn test_frustum_from_identity_has_six_planes() {
        let f = identity_frustum();
        assert_eq!(f.planes.len(), 6);
    }

    #[test]
    fn test_aabb_inside_frustum() {
        let f = identity_frustum();
        // AABB fully inside [-1,1] canonical volume
        assert!(f.test_aabb(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)));
    }

    #[test]
    fn test_aabb_outside_frustum() {
        let f = identity_frustum();
        // AABB fully outside (right of right plane)
        assert!(!f.test_aabb(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)));
    }

    #[test]
    fn test_aabb_partially_inside() {
        let f = identity_frustum();
        // AABB straddling the right plane
        assert!(f.test_aabb(Vec3::new(0.5, -0.5, -0.5), Vec3::new(1.5, 0.5, 0.5)));
    }

    #[test]
    fn test_aabb_fully_outside_left() {
        let f = identity_frustum();
        assert!(!f.test_aabb(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(-2.0, 1.0, 1.0)));
    }

    #[test]
    fn test_sphere_inside() {
        let f = identity_frustum();
        assert!(f.test_sphere(Vec3::ZERO, 0.5));
    }

    #[test]
    fn test_sphere_outside() {
        let f = identity_frustum();
        assert!(!f.test_sphere(Vec3::new(3.0, 0.0, 0.0), 0.5));
    }

    #[test]
    fn test_sphere_intersecting() {
        let f = identity_frustum();
        // Sphere center outside but radius reaches in
        assert!(f.test_sphere(Vec3::new(1.2, 0.0, 0.0), 0.5));
    }

    #[test]
    fn test_sphere_fully_outside() {
        let f = identity_frustum();
        assert!(!f.test_sphere(Vec3::new(0.0, 0.0, 3.0), 0.5));
    }

    #[test]
    fn test_orthographic_frustum() {
        let proj = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
        let view = Mat4::IDENTITY;
        let vp = proj * view;
        let f = Frustum::from_view_projection(&vp);
        // Point at center should be inside
        assert!(f.test_sphere(Vec3::new(400.0, 300.0, 0.0), 1.0));
        // Point far outside should be outside
        assert!(!f.test_sphere(Vec3::new(1000.0, 300.0, 0.0), 1.0));
    }
}
```

- [ ] **Step 2: Update lib.rs to export frustum module**

Add to `crates/engine-render/src/lib.rs`:

```rust
pub mod frustum;
```

- [ ] **Step 3: Run frustum tests**

Run: `cargo test -p engine-render`
Expected: All frustum tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/frustum.rs crates/engine-render/src/lib.rs
git commit -m "feat(render): add Frustum with AABB and sphere culling tests"
```

---

### Task 3: Refactor view.rs — Remove Old Camera

**Files:**
- Modify: `crates/engine-render/src/view.rs`

- [ ] **Step 1: Replace view.rs contents**

The old `Camera`, `Projection`, and `View` types are replaced by the new `camera.rs` types. `view.rs` becomes a thin re-export or is removed entirely.

Replace `crates/engine-render/src/view.rs` with:

```rust
// Re-export camera types for backward compatibility.
pub use crate::camera::{Camera, Projection, Viewport, RenderTarget, Color};
```

- [ ] **Step 2: Update renderer.rs imports**

In `crates/engine-render/src/renderer.rs`, the `Camera` and `Mat4` types used in `present()` need updating. The `view.rs` re-export means existing `use crate::view::Camera` still works. But `present()` currently takes `&Mat4` directly — this will change in Task 5.

No code changes yet — just verify it compiles.

- [ ] **Step 3: Build to verify no breakage**

Run: `cargo build -p engine-render`
Expected: SUCCESS (re-exports preserve API)

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/view.rs
git commit -m "refactor(render): replace old Camera/View with re-exports from camera.rs"
```

---

### Task 4: Extend TextureStore for Render Targets

**Files:**
- Modify: `crates/engine-render/src/texture_store.rs`

- [ ] **Step 1: Add render target creation to TextureStore**

Read `crates/engine-render/src/texture_store.rs` first, then add these methods:

```rust
// Add to TextureStore impl

/// Create a render target texture and register it.
/// Returns a texture store key (u64) that can be used with RenderTarget::Texture(key).
pub fn create_render_target(
    &mut self,
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: Option<&str>,
) -> u64 {
    let key = self.next_key();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    self.textures.insert(key, StoredTexture { texture, view });
    key
}

/// Get the texture view for a render target.
pub fn get_render_target_view(&self, key: u64) -> Option<&wgpu::TextureView> {
    self.textures.get(&key).map(|t| &t.view)
}
```

Note: The exact implementation depends on the existing TextureStore structure. Adapt field names and key management to match the current code. If TextureStore uses `HashMap<u64, ...>`, follow that pattern. If it uses a `Vec`, adapt accordingly.

- [ ] **Step 2: Build to verify**

Run: `cargo build -p engine-render`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/texture_store.rs
git commit -m "feat(render): add render target creation to TextureStore"
```

---

### Task 5: Renderer Multi-Camera render_frame()

**Files:**
- Modify: `crates/engine-render/src/renderer.rs`

- [ ] **Step 1: Add render_frame method to Renderer**

This is the main integration point. Add a new method alongside the existing `present()` (keep `present()` for backward compatibility initially):

```rust
/// Render a frame with multiple cameras.
///
/// Cameras are sorted by priority. Each camera gets its own viewport/scissor
/// and render pass. Sprites are culled per camera using frustum culling.
pub fn render_frame(
    &mut self,
    cameras: &[&crate::camera::Camera],
    all_sprites: &[crate::sprite::SpriteDraw],
) -> Result<(), wgpu::SurfaceError> {
    use crate::camera::{Camera, RenderTarget};
    use crate::frustum::Frustum;

    // Sort cameras by priority
    let mut sorted: Vec<&Camera> = cameras.iter().copied().collect();
    sorted.sort_by_key(|c| c.priority);

    let output = self.surface.get_current_texture()?;
    let swapchain_view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("main_encoder"),
        });

    let surface_width = self.config.width;
    let surface_height = self.config.height;

    for camera in &sorted {
        if !camera.is_active {
            continue;
        }

        // Compute aspect and view-projection matrix
        let (vx, vy, vw, vh) = camera.viewport.to_absolute(surface_width, surface_height);
        let aspect = vw as f32 / vh.max(1) as f32;
        let vp_matrix = camera.view_projection(aspect);

        // Frustum cull sprites
        let frustum = Frustum::from_view_projection(&vp_matrix);
        let visible: Vec<crate::sprite::SpriteDraw> = all_sprites
            .iter()
            .filter(|s| {
                // Sprite AABB: world position ± size/2
                let pos = s.world_matrix.transform_point3(Vec3::ZERO);
                let half = Vec3::new(s.size.x * 0.5, s.size.y * 0.5, 0.1);
                frustum.test_aabb(pos - half, pos + half)
            })
            .cloned()
            .collect();

        // Collect batches from visible sprites
        let mut batches = crate::sprite::collect_batches(&visible);
        for batch in &mut batches {
            batch.upload(&self.device);
        }

        // Upload camera uniform
        let matrix_data = vp_matrix.to_cols_array();
        self.queue
            .write_buffer(&self.camera_uniform, 0, bytemuck::cast_slice(&matrix_data));

        // Determine render target view
        let target_view = match camera.render_target {
            RenderTarget::Screen => &swapchain_view,
            RenderTarget::Texture(key) => {
                self.texture_store
                    .get_render_target_view(key)
                    .expect("render target texture not found")
            }
        };

        // Build batch references
        let camera_bg = &self.camera_bind_group;
        let pipeline = &self.sprite_pipeline.pipeline;
        let batch_refs: Vec<(
            &wgpu::BindGroup,
            &Option<wgpu::Buffer>,
            &Option<wgpu::Buffer>,
            u32,
        )> = batches
            .iter()
            .map(|b| {
                let bg = self.texture_store.get_bind_group(b.texture_id);
                (bg, &b.vertex_buffer, &b.index_buffer, b.index_count)
            })
            .collect();

        // Render pass
        let clear = camera.clear_color.unwrap_or(crate::camera::Color::BLACK);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("camera_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_viewport(vx as f32, vy as f32, vw as f32, vh as f32, 0.0, 1.0);
            pass.set_scissor_rect(vx, vy, vw, vh);

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera_bg, &[]);
            for (bind_group, vb, ib, index_count) in &batch_refs {
                pass.set_bind_group(1, *bind_group, &[]);
                if let (Some(vb), Some(ib)) = (vb, ib) {
                    pass.set_vertex_buffer(0, vb.slice(..));
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..*index_count, 0, 0..1);
                }
            }
        }
    }

    self.queue.submit([encoder.finish()]);
    output.present();
    Ok(())
}
```

- [ ] **Step 2: Add necessary imports to renderer.rs**

Ensure these imports exist at the top of `renderer.rs`:

```rust
use engine_math::Vec3;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p engine-render`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "feat(render): add render_frame() with multi-camera, frustum culling, viewport/scissor"
```

---

### Task 6: Update sprite_demo with Multi-Camera

**Files:**
- Modify: `crates/engine-core/examples/sprite_demo.rs`

- [ ] **Step 1: Rewrite sprite_demo to use Camera component**

```rust
use engine_math::{Mat4, Vec2, Vec3};
use engine_render::camera::{Camera, Color, Viewport};
use engine_render::renderer::Renderer;
use engine_render::sprite::SpriteDraw;
use engine_window::{window::WindowConfig, window::create_window};
use log::info;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Sprite Demo — Multi-Camera");

    let event_loop = EventLoop::new().unwrap();
    let window = std::sync::Arc::new(create_window(
        &WindowConfig {
            title: "Sprite Demo — Multi-Camera".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        },
        &event_loop,
    ));

    let mut renderer = Renderer::new(window);

    let texture_id = renderer
        .texture_store
        .load(
            &renderer.device,
            &renderer.queue,
            &renderer.sprite_pipeline.texture_bind_group_layout,
            "assets/test.png",
        )
        .unwrap_or_else(|e| {
            info!("Could not load texture: {} — using fallback", e);
            0
        });

    // Define scene sprites (in a real app, these come from ECS queries)
    let all_sprites = vec![
        SpriteDraw {
            world_matrix: Mat4::from_translation(Vec3::new(200.0, 300.0, 0.0)),
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            texture_id,
            flip_x: false,
            flip_y: false,
        },
        SpriteDraw {
            world_matrix: Mat4::from_translation(Vec3::new(600.0, 300.0, 0.0)),
            color: [0.0, 1.0, 0.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            texture_id,
            flip_x: false,
            flip_y: false,
        },
    ];

    // Main camera — full screen
    let mut main_camera = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
    main_camera.priority = 0;
    main_camera.clear_color = Some(Color::new(0.1, 0.1, 0.1, 1.0));

    // Top-right mini camera (picture-in-picture)
    let mut mini_camera = Camera::orthographic(0.0, 400.0, 300.0, 0.0);
    mini_camera.priority = 1;
    mini_camera.viewport = Viewport::Relative {
        x: 0.6,
        y: 0.0,
        width: 0.4,
        height: 0.4,
    };
    mini_camera.clear_color = Some(Color::new(0.2, 0.2, 0.3, 1.0));

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match &event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    renderer.resize(size.width, size.height);
                }
                _ => {}
            }

            if let Event::AboutToWait = event {
                let cameras: Vec<&Camera> = vec![&main_camera, &mini_camera];
                let _ = renderer.render_frame(&cameras, &all_sprites);
            }
        })
        .unwrap();
}
```

- [ ] **Step 2: Build the demo**

Run: `cargo build --example sprite_demo -p engine-core`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/examples/sprite_demo.rs
git commit -m "feat: update sprite_demo with multi-camera rendering"
```

---

### Task 7: Cleanup — Deprecate Old present()

**Files:**
- Modify: `crates/engine-render/src/renderer.rs`

- [ ] **Step 1: Mark old present() as deprecated**

Add deprecation attribute to the old `present()` method:

```rust
#[deprecated(note = "Use render_frame() for multi-camera support")]
pub fn present(...) { ... }
```

- [ ] **Step 2: Run clippy and fmt**

Run: `cargo clippy -p engine-render && cargo fmt --check`
Expected: No new warnings

- [ ] **Step 3: Run all tests**

Run: `cargo test -p engine-render`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "chore(render): deprecate old present() in favor of render_frame()"
```

---

### Task 8: Final Verification

- [ ] **Step 1: Full build**

Run: `cargo build`
Expected: SUCCESS

- [ ] **Step 2: Full test suite**

Run: `cargo test`
Expected: All tests PASS (engine-asset tests now pass with tempfile dep)

- [ ] **Step 3: Clippy + fmt**

Run: `cargo clippy && cargo fmt --check`
Expected: No new warnings, formatting clean

- [ ] **Step 4: Commit any fixes**

If any issues found, fix and commit.
