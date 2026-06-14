//! Multi-viewport renderer for the editor.
//!
//! Provides split-screen and multi-viewport rendering capabilities
//! using wgpu render targets.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Available viewport types for different camera angles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportType {
    Perspective,
    Top,
    Front,
    Right,
}

/// Layout arrangement of viewports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportLayout {
    Single(ViewportType),
    Horizontal(ViewportType, ViewportType),
    Vertical(ViewportType, ViewportType),
    Quad,
}

impl Default for ViewportLayout {
    fn default() -> Self {
        ViewportLayout::Single(ViewportType::Perspective)
    }
}

/// Orthographic camera parameters for non-perspective viewports.
#[derive(Debug, Clone)]
pub struct OrthoCamera {
    pub target: [f32; 3],
    pub zoom: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for OrthoCamera {
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            zoom: 1.0,
            near: -1000.0,
            far: 1000.0,
        }
    }
}

/// Offscreen render target for a single viewport.
#[allow(dead_code)] // texture and egui_texture_id are kept for future egui integration
struct ViewportTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    egui_texture_id: Option<egui::TextureId>,
}

/// Manages offscreen render targets for multiple editor viewports.
pub struct ViewportRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    targets: HashMap<ViewportType, ViewportTarget>,
    current_layout: ViewportLayout,
}

/// Thread-safe shared viewport renderer.
pub type SharedViewportRenderer = Arc<Mutex<ViewportRenderer>>;

const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const TARGET_USAGE: wgpu::TextureUsages = wgpu::TextureUsages::RENDER_ATTACHMENT
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC);

impl ViewportRenderer {
    /// Creates a new viewport renderer.
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            targets: HashMap::new(),
            current_layout: ViewportLayout::default(),
        }
    }

    /// Sets the active viewport layout.
    pub fn set_layout(&mut self, layout: ViewportLayout) {
        self.current_layout = layout;
    }

    /// Returns the current viewport layout.
    pub fn layout(&self) -> ViewportLayout {
        self.current_layout
    }

    /// Returns the list of viewports that are active in the current layout.
    pub fn active_viewports(&self) -> Vec<ViewportType> {
        match self.current_layout {
            ViewportLayout::Single(vt) => vec![vt],
            ViewportLayout::Horizontal(a, b) => vec![a, b],
            ViewportLayout::Vertical(a, b) => vec![a, b],
            ViewportLayout::Quad => vec![
                ViewportType::Perspective,
                ViewportType::Top,
                ViewportType::Front,
                ViewportType::Right,
            ],
        }
    }

    /// Ensures a render target exists for the given viewport with the specified dimensions.
    /// Recreates the texture if the size has changed.
    pub fn ensure_target(&mut self, viewport: ViewportType, width: u32, height: u32) {
        let needs_recreate = match self.targets.get(&viewport) {
            Some(target) => target.width != width || target.height != height,
            None => true,
        };

        if needs_recreate && width > 0 && height > 0 {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Viewport_{:?}", viewport)),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TARGET_FORMAT,
                usage: TARGET_USAGE,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            self.targets.insert(
                viewport,
                ViewportTarget {
                    texture,
                    view,
                    width,
                    height,
                    egui_texture_id: None,
                },
            );
        }
    }

    /// Returns the texture view for a viewport, if it exists.
    pub fn target_view(&self, viewport: ViewportType) -> Option<&wgpu::TextureView> {
        self.targets.get(&viewport).map(|t| &t.view)
    }

    /// Returns the (width, height) of a viewport target, if it exists.
    pub fn target_size(&self, viewport: ViewportType) -> Option<(u32, u32)> {
        self.targets.get(&viewport).map(|t| (t.width, t.height))
    }

    /// Clears the render target for a viewport with the given color.
    pub fn clear_target(&self, viewport: ViewportType, color: wgpu::Color) {
        if let Some(target) = self.targets.get(&viewport) {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("Clear_{:?}", viewport)),
                });
            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("ClearPass_{:?}", viewport)),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn try_create_renderer() -> Option<ViewportRenderer> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: true,
        }))
        .or_else(|| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
        });
        let adapter = adapter?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .ok()?;

        Some(ViewportRenderer::new(Arc::new(device), Arc::new(queue)))
    }

    #[test]
    fn test_viewport_renderer_new() {
        let Some(renderer) = try_create_renderer() else {
            return;
        };
        assert_eq!(
            renderer.layout(),
            ViewportLayout::Single(ViewportType::Perspective)
        );
    }

    #[test]
    fn test_set_layout() {
        let Some(mut renderer) = try_create_renderer() else {
            return;
        };
        renderer.set_layout(ViewportLayout::Quad);
        assert_eq!(renderer.layout(), ViewportLayout::Quad);
    }

    #[test]
    fn test_active_viewports_single() {
        let Some(renderer) = try_create_renderer() else {
            return;
        };
        let viewports = renderer.active_viewports();
        assert_eq!(viewports, vec![ViewportType::Perspective]);
    }

    #[test]
    fn test_active_viewports_quad() {
        let Some(mut renderer) = try_create_renderer() else {
            return;
        };
        renderer.set_layout(ViewportLayout::Quad);
        let viewports = renderer.active_viewports();
        assert_eq!(viewports.len(), 4);
        assert!(viewports.contains(&ViewportType::Perspective));
        assert!(viewports.contains(&ViewportType::Top));
        assert!(viewports.contains(&ViewportType::Front));
        assert!(viewports.contains(&ViewportType::Right));
    }

    #[test]
    fn test_ensure_target() {
        let Some(mut renderer) = try_create_renderer() else {
            return;
        };

        renderer.ensure_target(ViewportType::Perspective, 800, 600);
        assert!(renderer.target_view(ViewportType::Perspective).is_some());
        assert_eq!(
            renderer.target_size(ViewportType::Perspective),
            Some((800, 600))
        );
    }
}
