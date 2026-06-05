//! Render pass types and context for the render graph.

use crate::graph::texture::TextureHandle;

/// Type alias for the execute function of a render pass.
pub type ExecuteFn<'a> = Box<dyn FnOnce(&mut PassContext<'_>) + Send + 'a>;

/// Color attachment configuration for a render pass.
pub struct ColorAttachment {
    /// The texture to render to.
    pub resource: TextureHandle,
    /// Load operation (clear or load existing content).
    pub load_op: wgpu::LoadOp<wgpu::Color>,
    /// Store operation (store or discard).
    pub store_op: wgpu::StoreOp,
}

/// Depth/stencil attachment configuration for a render pass.
pub struct DepthStencilAttachment {
    /// The depth texture to render to.
    pub resource: TextureHandle,
    /// Depth load operation.
    pub depth_load_op: wgpu::LoadOp<f32>,
    /// Depth store operation.
    pub depth_store_op: wgpu::StoreOp,
}

/// Descriptor for creating a render pass in the graph.
pub struct RenderPassDesc<'a> {
    /// Optional debug label for the pass.
    pub label: Option<String>,
    /// Color attachments for this pass.
    pub color_attachments: Vec<ColorAttachment>,
    /// Optional depth/stencil attachment.
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
    /// The function to execute when this pass runs.
    pub execute: ExecuteFn<'a>,
}

pub(crate) struct PassMetadata {
    pub label: Option<String>,
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
}

pub(crate) struct RenderPassNode {
    pub meta: PassMetadata,
}

impl RenderPassNode {
    pub fn new(meta: PassMetadata) -> Self {
        Self { meta }
    }
}

/// Context provided to render pass execute functions.
///
/// Contains the render pass, resource views, and device/queue references
/// needed to record GPU commands.
pub struct PassContext<'a> {
    /// The active render pass.
    pub pass: wgpu::RenderPass<'a>,
    /// Access to graph-managed resources (textures and buffers).
    pub resources: &'a RenderGraphResources<'a>,
    /// The wgpu device for creating GPU resources.
    pub device: &'a wgpu::Device,
    /// The wgpu queue for submitting commands.
    pub queue: &'a wgpu::Queue,
}

/// Resource views available to render passes.
pub struct RenderGraphResources<'a> {
    /// Texture views indexed by texture handle.
    pub textures: Vec<Option<&'a wgpu::TextureView>>,
    /// Buffer references indexed by buffer handle.
    pub buffers: Vec<Option<&'a wgpu::Buffer>>,
}
