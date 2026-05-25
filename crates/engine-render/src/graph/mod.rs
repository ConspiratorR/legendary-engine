pub mod buffer;
pub mod compile;
pub mod execute;
pub mod pass;
pub mod texture;

pub use buffer::BufferDesc;
pub use texture::{TextureDesc, TextureHandle};

use std::collections::HashMap;

pub struct RenderGraph {
    textures: Vec<Option<texture::TextureNode>>,
    buffers: Vec<Option<buffer::BufferNode>>,
    passes: Vec<pass::RenderPassNode>,
    texture_map: HashMap<String, TextureHandle>,
    buffer_map: HashMap<String, texture::BufferHandle>,
    compiled: bool,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            buffers: Vec::new(),
            passes: Vec::new(),
            texture_map: HashMap::new(),
            buffer_map: HashMap::new(),
            compiled: false,
        }
    }

    pub fn create_texture(&mut self, desc: TextureDesc) -> TextureHandle {
        let id = TextureHandle(self.textures.len() as u32);
        if let Some(ref name) = desc.label {
            self.texture_map.insert(name.clone(), id);
        }
        self.textures.push(Some(texture::TextureNode::new(desc)));
        id
    }

    pub fn create_buffer(&mut self, desc: BufferDesc) -> texture::BufferHandle {
        let id = texture::BufferHandle(self.buffers.len() as u32);
        if let Some(ref name) = desc.label {
            self.buffer_map.insert(name.clone(), id);
        }
        self.buffers.push(Some(buffer::BufferNode::new(desc)));
        id
    }

    pub fn import_texture(
        &mut self,
        name: &str,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
    ) -> TextureHandle {
        let id = TextureHandle(self.textures.len() as u32);
        self.texture_map.insert(name.to_string(), id);
        self.textures
            .push(Some(texture::TextureNode::imported(texture, view)));
        id
    }

    pub fn import_texture_view(&mut self, name: &str, view: wgpu::TextureView) -> TextureHandle {
        let id = TextureHandle(self.textures.len() as u32);
        self.texture_map.insert(name.to_string(), id);
        self.textures
            .push(Some(texture::TextureNode::imported_view(view)));
        id
    }

    pub fn add_render_pass<'a>(&mut self, desc: pass::RenderPassDesc<'a>) -> pass::ExecuteFn<'a> {
        let meta = pass::PassMetadata {
            label: desc.label,
            color_attachments: desc.color_attachments,
            depth_stencil_attachment: desc.depth_stencil_attachment,
        };
        self.passes.push(pass::RenderPassNode::new(meta));
        desc.execute
    }

    pub fn is_compiled(&self) -> bool {
        self.compiled
    }

    pub fn reset(&mut self) {
        self.textures
            .retain(|t| t.as_ref().is_some_and(|n| n.import));
        self.buffers
            .retain(|b| b.as_ref().is_some_and(|n| n.import));
        self.passes.clear();
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}
