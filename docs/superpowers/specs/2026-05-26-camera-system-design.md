# Camera System Design Spec

## Overview

Enhance the camera system with multi-camera support, frustum culling (2D AABB + 3D sphere/AABB), and render-to-texture capabilities. Camera becomes an ECS component that can be attached to any entity.

## Motivation

The current camera system is minimal: a single `Camera` struct with projection/view matrices and one uniform buffer in the Renderer. This blocks:
- Split-screen multiplayer (2P/4P local)
- Picture-in-picture / minimap overlays
- Editor + game dual viewport
- Efficient culling of off-screen sprites

## Architecture

### Approach: Camera as ECS Component + Render Queue

Camera is an ECS component with priority, viewport rect, and render target. Each frame, the system collects all Camera components, sorts by priority, and each camera produces a filtered render queue via frustum culling. The Renderer iterates cameras, sets viewport/scissor, and submits per-camera render passes.

### Camera Component

```rust
// engine-render/src/camera.rs (new file)

pub struct Camera {
    pub projection: Projection,       // existing, kept
    pub view: Mat4,                   // existing, kept
    pub priority: i32,                // render order (lower = first)
    pub viewport: Viewport,           // viewport region
    pub render_target: RenderTarget,  // screen or texture
    pub is_active: bool,              // whether to render
    pub clear_color: Option<Color>,   // per-camera clear color
}

pub enum Viewport {
    Absolute { x: u32, y: u32, width: u32, height: u32 },
    Relative { x: f32, y: f32, width: f32, height: f32 }, // 0.0-1.0
}

pub enum RenderTarget {
    Screen,
    Texture(Handle),
}
```

Cameras attach to entities via `world.add_component(entity, camera)`. The camera inherits the entity's Transform for its view matrix.

### Frustum Culling

```rust
// engine-render/src/frustum.rs (new file)

pub struct Frustum {
    pub planes: [Vec4; 6], // left, right, bottom, top, near, far
}

impl Frustum {
    pub fn from_view_projection(vp: &Mat4) -> Self;
    pub fn contains_aabb(&self, min: Vec3, max: Vec3) -> bool;
    pub fn contains_sphere(&self, center: Vec3, radius: f32) -> bool;
}
```

Culling flow:
1. Extract 6 frustum planes from `projection * view` matrix (Gribb-Hartmann method)
2. For sprites: test `world_position ± size/2` AABB against frustum
3. For 3D objects: test AABB or bounding sphere
4. Invisible objects excluded from render queue before `collect_batches()`

Culling runs on CPU, before batch collection. GPU-side rendering unchanged.

### Multi-Camera Rendering

```rust
// renderer.rs new method

impl Renderer {
    pub fn render_frame(
        &mut self,
        cameras: &[&Camera],
        all_sprites: &[SpriteDraw],
    ) -> Result<(), wgpu::SurfaceError>;
}
```

Per-camera flow:
1. Sort cameras by priority
2. For each camera:
   a. Compute frustum from camera matrices
   b. Cull sprites → visible set
   c. `collect_batches(visible_set)`
   d. Set viewport/scissor rect
   e. If `RenderTarget::Texture`, switch to RTT pass
   f. Submit render

### Render-to-Texture

When `RenderTarget::Texture(handle)`:
- Create `wgpu::Texture` + `wgpu::TextureView` at configured resolution
- TextureStore manages render target textures alongside regular textures
- The render pass targets the texture view instead of the swapchain
- For picture-in-picture: render secondary camera to texture, then draw that texture as a Sprite in the primary camera's pass

## File Changes

| File | Action | Content |
|------|--------|---------|
| `engine-render/src/camera.rs` | Create | Camera, Viewport, RenderTarget components |
| `engine-render/src/frustum.rs` | Create | Frustum extraction, AABB/sphere tests |
| `engine-render/src/view.rs` | Refactor | Remove old Camera, simplify View |
| `engine-render/src/renderer.rs` | Modify | `render_frame()` multi-camera + RTT |
| `engine-render/src/texture_store.rs` | Extend | Render target texture management |
| `engine-render/src/lib.rs` | Update | Export new modules |
| `examples/sprite_demo.rs` | Update | Use new Camera component |

## ECS Integration

Camera is a plain ECS component. Systems query Camera components each frame, collect visible sprites, and call `renderer.render_frame()`. No special ECS hooks needed — standard component add/query pattern.

## Testing

- Frustum AABB culling unit tests (inside/outside/partial)
- Frustum sphere culling unit tests
- Camera priority sort tests
- Viewport relative→absolute conversion tests
- Multi-camera rendering integration (sprite_demo with split-screen)
