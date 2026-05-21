use crate::graph::texture::TextureHandle;

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

pub struct RenderPassDesc {
    pub label: Option<String>,
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
    pub execute: Box<dyn FnOnce(&mut PassContext<'_>) + Send>,
}

pub struct RenderPassNode {
    pub desc: RenderPassDesc,
}

impl RenderPassNode {
    pub fn new(desc: RenderPassDesc) -> Self {
        Self { desc }
    }
}

pub struct PassContext<'a> {
    pub pass: wgpu::RenderPass<'a>,
    pub resources: &'a RenderGraphResources<'a>,
}

pub struct RenderGraphResources<'a> {
    pub textures: Vec<Option<&'a wgpu::TextureView>>,
    pub buffers: Vec<Option<&'a wgpu::Buffer>>,
}
