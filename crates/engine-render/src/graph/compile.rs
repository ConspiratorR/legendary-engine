use crate::graph::{RenderGraph, TextureHandle};

#[derive(Debug)]
pub enum CompileError {
    NoSwapchainAttachment,
    TextureNotFound(TextureHandle),
}

pub struct CompiledColorAttachment {
    pub view_index: usize,
    pub load: wgpu::LoadOp<wgpu::Color>,
    pub store: wgpu::StoreOp,
}

pub struct CompiledDepthStencilAttachment {
    pub view_index: usize,
    pub depth_load: wgpu::LoadOp<f32>,
    pub depth_store: wgpu::StoreOp,
}

pub struct CompiledPass {
    pub label: Option<String>,
    pub color_attachments: Vec<CompiledColorAttachment>,
    pub depth_stencil_attachment: Option<CompiledDepthStencilAttachment>,
}

pub struct CompiledGraph {
    pub passes: Vec<CompiledPass>,
    pub transient_textures: Vec<wgpu::Texture>,
    pub transient_buffers: Vec<wgpu::Buffer>,
}

impl RenderGraph {
    pub fn compile(&mut self, device: &wgpu::Device) -> Result<CompiledGraph, CompileError> {
        let mut transient_textures = Vec::new();
        let texture_count = self.textures.len();
        for i in 0..texture_count {
            if let Some(ref mut node) = self.textures[i] {
                if node.import {
                    continue;
                }
                let device_texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: node.desc.label.as_deref(),
                    size: node.desc.size,
                    mip_level_count: node.desc.mip_levels,
                    sample_count: node.desc.sample_count,
                    dimension: node.desc.dimension,
                    format: node.desc.format,
                    usage: node.desc.usage,
                    view_formats: &[],
                });
                let view = device_texture.create_view(&wgpu::TextureViewDescriptor::default());
                if node.desc.transient {
                    transient_textures.push(device_texture);
                } else {
                    node.texture = Some(device_texture);
                }
                node.view = Some(view);
            }
        }

        let mut transient_buffers = Vec::new();
        let buffer_count = self.buffers.len();
        for i in 0..buffer_count {
            if let Some(ref mut node) = self.buffers[i] {
                if node.import {
                    continue;
                }
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: node.desc.label.as_deref(),
                    size: node.desc.size,
                    usage: node.desc.usage,
                    mapped_at_creation: false,
                });
                if node.desc.transient {
                    transient_buffers.push(buffer);
                } else {
                    node.buffer = Some(buffer);
                }
            }
        }

        // Resolve view indices for each pass
        let mut compiled_passes = Vec::new();
        for pass in &self.passes {
            let color_attachments = pass
                .meta
                .color_attachments
                .iter()
                .map(|ca| {
                    let idx = ca.resource.0 as usize;
                    if idx >= self.textures.len() || self.textures[idx].is_none() {
                        panic!("Color attachment texture handle {} not found", idx);
                    }
                    CompiledColorAttachment {
                        view_index: idx,
                        load: ca.load_op,
                        store: ca.store_op,
                    }
                })
                .collect();

            let depth_stencil = pass.meta.depth_stencil_attachment.as_ref().map(|ds| {
                let idx = ds.resource.0 as usize;
                if idx >= self.textures.len() || self.textures[idx].is_none() {
                    panic!("Depth attachment texture handle {} not found", idx);
                }
                CompiledDepthStencilAttachment {
                    view_index: idx,
                    depth_load: ds.depth_load_op,
                    depth_store: ds.depth_store_op,
                }
            });

            compiled_passes.push(CompiledPass {
                label: pass.meta.label.clone(),
                color_attachments,
                depth_stencil_attachment: depth_stencil,
            });
        }

        self.compiled = true;
        Ok(CompiledGraph {
            passes: compiled_passes,
            transient_textures,
            transient_buffers,
        })
    }
}
