//! High-performance sprite renderer with batched rendering and indirect draw calls.
//!
//! The sprite renderer uses persistent GPU buffers and indirect drawing to
//! efficiently render large numbers of sprites with minimal CPU overhead.

use std::sync::Arc;

use crate::indirect::DrawIndexedIndirectArgs;
use crate::pipeline::sprite::SpritePipeline;
use crate::sprite::SpriteBatch;

/// Draw offset information for a single batch.
///
/// Contains the byte offset into the indirect draw buffer where this batch's
/// draw command is located.
pub struct BatchDrawInfo {
    /// Byte offset of the indirect draw command in the indirect buffer.
    pub indirect_offset: u64,
}

/// A persistent GPU buffer wrapper.
///
/// Wraps a wgpu buffer with its size for convenient access.
pub struct PersistentBuffer {
    buffer: wgpu::Buffer,
    size: usize,
}

impl PersistentBuffer {
    /// Create a new persistent buffer with the given size and optional label.
    pub fn new(device: &wgpu::Device, size: usize, label: Option<&str>) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: size as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { buffer, size }
    }

    /// Get a reference to the underlying wgpu buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get the buffer size in bytes.
    pub fn size(&self) -> usize {
        self.size
    }
}

/// High-performance sprite renderer using persistent buffers and indirect drawing.
///
/// Uses double-buffered instance data and indirect draw commands to minimize
/// CPU overhead when rendering large numbers of sprites. Sprites are batched
/// by texture to reduce bind group switches.
#[allow(dead_code)]
pub struct SpriteRenderer {
    /// Double-buffered instance buffers for frame pipelining.
    instance_buffers: [PersistentBuffer; 2],
    /// Indirect draw command buffer.
    indirect_buffer: PersistentBuffer,
    /// Current frame index (0 or 1) for double buffering.
    current_frame: usize,
    /// Maximum sprite capacity.
    sprite_capacity: usize,
    /// Shared sprite pipeline.
    pipeline: Arc<SpritePipeline>,
    /// Vertex buffer (reused across frames).
    vertex_buffer: PersistentBuffer,
    /// Index buffer (reused across frames).
    index_buffer: PersistentBuffer,
}

impl SpriteRenderer {
    /// Create a new sprite renderer with the given capacity.
    ///
    /// # Arguments
    /// * `device` - wgpu device for buffer creation
    /// * `pipeline` - Shared sprite pipeline
    /// * `sprite_capacity` - Maximum number of sprites that can be rendered per frame
    pub fn new(
        device: &wgpu::Device,
        pipeline: Arc<SpritePipeline>,
        sprite_capacity: usize,
    ) -> Self {
        // Calculate buffer sizes
        // Per sprite: 4 vertices * 36 bytes + 6 indices * 2 bytes + 1 instance * 64 bytes
        let vertex_size = sprite_capacity * 4 * 36;
        let index_size = sprite_capacity * 6 * 2;
        let instance_size = sprite_capacity * 64;
        let indirect_size = sprite_capacity * std::mem::size_of::<DrawIndexedIndirectArgs>();

        let vertex_buffer =
            PersistentBuffer::new(device, vertex_size, Some("sprite_vertex_buffer"));

        let index_buffer = PersistentBuffer::new(device, index_size, Some("sprite_index_buffer"));

        let instance_buffers = [
            PersistentBuffer::new(device, instance_size, Some("sprite_instance_buffer_0")),
            PersistentBuffer::new(device, instance_size, Some("sprite_instance_buffer_1")),
        ];

        let indirect_buffer =
            PersistentBuffer::new(device, indirect_size, Some("sprite_indirect_buffer"));

        Self {
            instance_buffers,
            indirect_buffer,
            current_frame: 0,
            sprite_capacity,
            pipeline,
            vertex_buffer,
            index_buffer,
        }
    }

    /// Begin a new frame, switching the double-buffered instance buffer.
    pub fn begin_frame(&mut self) {
        self.current_frame = 1 - self.current_frame;
    }

    /// Get the current frame's instance buffer.
    pub fn current_instance_buffer(&self) -> &PersistentBuffer {
        &self.instance_buffers[self.current_frame]
    }

    /// Get the current frame index (0 or 1).
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }

    /// Upload a single batch's data to the GPU buffers.
    pub fn upload_batch(
        &self,
        queue: &wgpu::Queue,
        batch: &SpriteBatch,
        vertex_offset: usize,
        instance_offset: usize,
        indirect_offset: usize,
    ) {
        let vertex_data = bytemuck::cast_slice(&batch.vertices);
        queue.write_buffer(
            self.vertex_buffer.buffer(),
            vertex_offset as u64,
            vertex_data,
        );

        let index_data = bytemuck::cast_slice(&batch.indices);
        queue.write_buffer(
            self.index_buffer.buffer(),
            (vertex_offset / 36 * 6 * 2) as u64,
            index_data,
        );

        let instance_data = bytemuck::cast_slice(&batch.instance_data);
        queue.write_buffer(
            self.current_instance_buffer().buffer(),
            instance_offset as u64,
            instance_data,
        );

        let indirect_data = bytemuck::bytes_of(&batch.indirect_cmd);
        queue.write_buffer(
            self.indirect_buffer.buffer(),
            indirect_offset as u64,
            indirect_data,
        );
    }

    /// Upload all batches in a single consolidated operation.
    ///
    /// Merges all vertex, index, instance, and indirect data into contiguous
    /// buffers using only 4 `write_buffer` calls instead of 4N calls.
    ///
    /// Returns draw offset information for each batch.
    pub fn upload_batches(
        &self,
        queue: &wgpu::Queue,
        batches: &[SpriteBatch],
    ) -> Vec<BatchDrawInfo> {
        if batches.is_empty() {
            return Vec::new();
        }

        // Accumulate all data
        let total_vertices: usize = batches.iter().map(|b| b.vertices.len()).sum();
        let total_indices: usize = batches.iter().map(|b| b.indices.len()).sum();
        let total_instances: usize = batches.iter().map(|b| b.instance_data.len()).sum();

        let mut vertex_data = Vec::with_capacity(
            total_vertices * std::mem::size_of::<crate::pipeline::sprite::SpriteVertex>(),
        );
        let mut index_data = Vec::with_capacity(total_indices * std::mem::size_of::<u16>());
        let mut instance_data =
            Vec::with_capacity(total_instances * std::mem::size_of::<engine_math::Mat4>());
        let mut indirect_data =
            Vec::with_capacity(batches.len() * std::mem::size_of::<DrawIndexedIndirectArgs>());

        for batch in batches {
            vertex_data.extend_from_slice(bytemuck::cast_slice(&batch.vertices));
            index_data.extend_from_slice(bytemuck::cast_slice(&batch.indices));
            instance_data.extend_from_slice(bytemuck::cast_slice(&batch.instance_data));
            indirect_data.extend_from_slice(bytemuck::bytes_of(&batch.indirect_cmd));
        }

        // 4 write_buffer calls instead of 4N
        queue.write_buffer(self.vertex_buffer.buffer(), 0, &vertex_data);
        queue.write_buffer(self.index_buffer.buffer(), 0, &index_data);
        queue.write_buffer(self.current_instance_buffer().buffer(), 0, &instance_data);
        queue.write_buffer(self.indirect_buffer.buffer(), 0, &indirect_data);

        // Calculate indirect offsets for each batch
        let indirect_stride = std::mem::size_of::<DrawIndexedIndirectArgs>() as u64;
        (0..batches.len())
            .map(|i| BatchDrawInfo {
                indirect_offset: i as u64 * indirect_stride,
            })
            .collect()
    }

    /// Get buffer references needed for drawing.
    pub fn get_buffers(&self) -> SpriteRendererBuffers<'_> {
        SpriteRendererBuffers {
            vertex_buffer: self.vertex_buffer.buffer(),
            index_buffer: self.index_buffer.buffer(),
            instance_buffer: self.current_instance_buffer().buffer(),
            indirect_buffer: self.indirect_buffer.buffer(),
        }
    }

    /// Consume the renderer and return owned buffers for import into a render graph.
    pub fn into_buffers(self) -> SpriteRendererOwnedBuffers {
        SpriteRendererOwnedBuffers {
            vertex_buffer: self.vertex_buffer,
            index_buffer: self.index_buffer,
            instance_buffers: self.instance_buffers,
            indirect_buffer: self.indirect_buffer,
        }
    }

    /// Get buffer sizes in bytes: (vertex, index, instance, indirect).
    pub fn buffer_sizes(&self) -> (usize, usize, usize, usize) {
        (
            self.vertex_buffer.size(),
            self.index_buffer.size(),
            self.instance_buffers[0].size(),
            self.indirect_buffer.size(),
        )
    }
}

/// Buffer references needed for sprite drawing.
pub struct SpriteRendererBuffers<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub instance_buffer: &'a wgpu::Buffer,
    pub indirect_buffer: &'a wgpu::Buffer,
}

/// Owned buffer collection for import into a render graph.
pub struct SpriteRendererOwnedBuffers {
    pub vertex_buffer: PersistentBuffer,
    pub index_buffer: PersistentBuffer,
    pub instance_buffers: [PersistentBuffer; 2],
    pub indirect_buffer: PersistentBuffer,
}
