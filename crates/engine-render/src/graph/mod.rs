//! Render graph for organizing render passes and resource dependencies.
//!
//! The render graph allows declarative specification of render passes and their
//! resource dependencies. Passes are automatically ordered based on their
//! read/write dependencies on textures and buffers.

pub mod buffer;
pub mod compile;
pub mod execute;
pub mod pass;
pub mod texture;

pub use buffer::BufferDesc;
pub use texture::{BufferHandle, TextureDesc, TextureHandle};

use std::collections::HashMap;

/// A declarative render graph that manages render passes and GPU resources.
///
/// Passes are added to the graph with their resource dependencies (textures and
/// buffers). The graph can then be compiled to determine execution order and
/// executed to run all passes in the correct sequence.
///
/// # Example
///
/// ```rust
/// use engine_render::graph::{RenderGraph, TextureDesc, BufferDesc};
///
/// let mut graph = RenderGraph::new();
/// let texture = graph.create_texture(
///     TextureDesc::new_2d(
///         1920,
///         1080,
///         wgpu::TextureFormat::Rgba8Unorm,
///         wgpu::TextureUsages::RENDER_ATTACHMENT,
///     ).named("hdr_buffer")
/// );
/// ```
pub struct RenderGraph {
    textures: Vec<Option<texture::TextureNode>>,
    buffers: Vec<Option<buffer::BufferNode>>,
    passes: Vec<pass::RenderPassNode>,
    texture_map: HashMap<String, TextureHandle>,
    buffer_map: HashMap<String, texture::BufferHandle>,
    compiled: bool,
}

impl RenderGraph {
    /// Create a new empty render graph.
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

    /// Create a new texture resource in the graph.
    ///
    /// Returns a handle that can be used to reference this texture in render passes.
    pub fn create_texture(&mut self, desc: TextureDesc) -> TextureHandle {
        let id = TextureHandle(self.textures.len() as u32);
        if let Some(ref name) = desc.label {
            self.texture_map.insert(name.clone(), id);
        }
        self.textures.push(Some(texture::TextureNode::new(desc)));
        id
    }

    /// Create a new buffer resource in the graph.
    ///
    /// Returns a handle that can be used to reference this buffer in render passes.
    pub fn create_buffer(&mut self, desc: BufferDesc) -> texture::BufferHandle {
        let id = texture::BufferHandle(self.buffers.len() as u32);
        if let Some(ref name) = desc.label {
            self.buffer_map.insert(name.clone(), id);
        }
        self.buffers.push(Some(buffer::BufferNode::new(desc)));
        id
    }

    /// Import an external texture into the graph.
    ///
    /// The graph takes ownership of the texture. Use this when the texture was
    /// created outside the graph but needs to be used in graph passes.
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

    /// Import an external texture view into the graph.
    ///
    /// The graph does not take ownership of the underlying texture, only the view.
    pub fn import_texture_view(&mut self, name: &str, view: wgpu::TextureView) -> TextureHandle {
        let id = TextureHandle(self.textures.len() as u32);
        self.texture_map.insert(name.to_string(), id);
        self.textures
            .push(Some(texture::TextureNode::imported_view(view)));
        id
    }

    /// Import an external buffer into the graph.
    ///
    /// The graph takes ownership of the buffer.
    pub fn import_buffer(&mut self, name: &str, buffer: wgpu::Buffer) -> texture::BufferHandle {
        let id = texture::BufferHandle(self.buffers.len() as u32);
        self.buffer_map.insert(name.to_string(), id);
        self.buffers
            .push(Some(buffer::BufferNode::imported(buffer)));
        id
    }

    /// Import an external buffer reference into the graph (without transferring ownership).
    ///
    /// The caller must ensure the buffer remains valid for the duration of graph execution.
    pub fn import_buffer_ref(
        &mut self,
        name: &str,
        buffer: &wgpu::Buffer,
    ) -> texture::BufferHandle {
        let id = texture::BufferHandle(self.buffers.len() as u32);
        self.buffer_map.insert(name.to_string(), id);
        self.buffers
            .push(Some(buffer::BufferNode::imported_ref(buffer)));
        id
    }

    /// Add a render pass to the graph.
    ///
    /// Returns the execute function that will be called when the pass is executed.
    pub fn add_render_pass<'a>(&mut self, desc: pass::RenderPassDesc<'a>) -> pass::ExecuteFn<'a> {
        let meta = pass::PassMetadata {
            label: desc.label,
            color_attachments: desc.color_attachments,
            depth_stencil_attachment: desc.depth_stencil_attachment,
        };
        self.passes.push(pass::RenderPassNode::new(meta));
        desc.execute
    }

    /// Check if the graph has been compiled.
    pub fn is_compiled(&self) -> bool {
        self.compiled
    }

    /// Get references to all buffers managed by the graph (in registration order).
    pub fn get_buffers(&self) -> Vec<Option<&wgpu::Buffer>> {
        self.buffers
            .iter()
            .map(|b| b.as_ref().and_then(|n| n.get_buffer()))
            .collect()
    }

    /// Reset the graph, removing all passes and non-imported resources.
    ///
    /// Imported textures and buffers are preserved. Passes are cleared and the
    /// graph is marked as not compiled.
    pub fn reset(&mut self) {
        self.textures
            .retain(|t| t.as_ref().is_some_and(|n| n.import));
        self.buffers
            .retain(|b| b.as_ref().is_some_and(|n| n.import));
        self.passes.clear();
        self.compiled = false;
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}
