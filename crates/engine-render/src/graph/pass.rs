use crate::graph::texture::TextureHandle;

pub type ExecuteFn<'a> = Box<dyn FnOnce(&mut PassContext<'_>) + Send + 'a>;

pub struct ColorAttachment {
    pub resource: TextureHandle,
    pub load_op: wgpu::LoadOp<wgpu::Color>,
    pub store_op: wgpu::StoreOp,
}

pub struct DepthStencilAttachment {
    pub resource: TextureHandle,
    pub depth_load_op: wgpu::LoadOp<f32>,
    pub depth_store_op: wgpu::StoreOp,
}

pub struct RenderPassDesc<'a> {
    pub label: Option<String>,
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
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

pub struct PassContext<'a> {
    pub pass: wgpu::RenderPass<'a>,
    pub resources: &'a RenderGraphResources<'a>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

pub struct RenderGraphResources<'a> {
    pub textures: Vec<Option<&'a wgpu::TextureView>>,
    pub buffers: Vec<Option<&'a wgpu::Buffer>>,
}
