//! ECS render plugins that automatically render Camera/Sprite components.
//!
//! [`RenderPlugin2D`] sets up the 2D sprite rendering pipeline and
//! prepares all GPU resources needed for automatic rendering.
//!
//! # Usage
//!
//! ```rust,ignore
//! use engine_render::plugin::RenderPlugin2D;
//!
//! // In run_default or custom event loop:
//! let mut plugin = RenderPlugin2D::new(window);
//! // plugin creates Renderer internally
//! // call plugin.take_renderer() to get the Renderer for App
//! ```

use crate::font::TextPainter;
use crate::pipeline::sprite::SpritePipeline;
use crate::renderer::Renderer;
use crate::shape::{ShapePainter, ShapePipeline};
use crate::texture_bridge::TextureBridge;
use std::sync::Arc;
use winit::window::Window;

/// 2D rendering plugin.
///
/// Creates a [`Renderer`], [`TextureBridge`], and [`TextPainter`],
/// inserts them as ECS resources.
/// After calling [`build`](Self::build), use [`take_renderer`](Self::take_renderer)
/// to extract the Renderer and set it on the [`App`](engine_core::app::App).
pub struct RenderPlugin2D {
    window: Arc<Window>,
    renderer: Option<Renderer>,
}

impl RenderPlugin2D {
    /// Create a new 2D render plugin for the given window.
    pub fn new(window: Arc<Window>) -> Self {
        Self {
            window,
            renderer: None,
        }
    }

    /// Build the plugin: create Renderer, TextureBridge, TextPainter, and Registry.
    ///
    /// Resources are inserted into the ECS world. The Renderer is stored
    /// internally — call [`take_renderer`](Self::take_renderer) to extract it.
    pub fn build(&mut self, world: &mut engine_ecs::world::World) {
        let renderer = Renderer::new(self.window.clone()).expect("Failed to create renderer");

        let texture_layout_a = SpritePipeline::create_texture_layout(&renderer.device);
        let bridge = TextureBridge::new(&renderer.device, &renderer.queue, texture_layout_a);
        world.insert_resource(bridge);

        let texture_layout_b = SpritePipeline::create_texture_layout(&renderer.device);
        let text_painter = TextPainter::new(&renderer.device, &renderer.queue, texture_layout_b);
        world.insert_resource(text_painter);

        let shape_pipeline = ShapePipeline::new(&renderer.device, wgpu::TextureFormat::Rgba16Float);
        let shape_painter = ShapePainter::new();
        world.insert_resource(shape_pipeline);
        world.insert_resource(shape_painter);

        self.renderer = Some(renderer);
    }

    /// Extract the Renderer from the plugin.
    ///
    /// Must be called after [`build`](Self::build). Returns `None` if already taken.
    pub fn take_renderer(&mut self) -> Option<Renderer> {
        self.renderer.take()
    }
}
