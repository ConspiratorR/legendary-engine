use std::sync::Arc;

use crate::indirect::DrawIndexedIndirectArgs;
use crate::pipeline::sprite::SpritePipeline;
use crate::sprite::SpriteBatch;

/// 每个批次的绘制偏移信息
pub struct BatchDrawInfo {
    /// 间接绘制命令在间接缓冲区中的字节偏移
    pub indirect_offset: u64,
}

/// GPU 缓冲包装
pub struct PersistentBuffer {
    buffer: wgpu::Buffer,
    size: usize,
}

impl PersistentBuffer {
    /// 创建缓冲区
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

    /// 获取缓冲区引用
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// 获取缓冲区大小
    pub fn size(&self) -> usize {
        self.size
    }
}

/// 高性能精灵渲染器
/// 使用持久映射缓冲 + 间接绘制
#[allow(dead_code)]
pub struct SpriteRenderer {
    /// 双缓冲实例缓冲
    instance_buffers: [PersistentBuffer; 2],
    /// 间接绘制命令缓冲
    indirect_buffer: PersistentBuffer,
    /// 当前帧索引 (0 or 1)
    current_frame: usize,
    /// 精灵容量上限
    sprite_capacity: usize,
    /// 复用现有管线
    pipeline: Arc<SpritePipeline>,
    /// 顶点缓冲（复用）
    vertex_buffer: PersistentBuffer,
    /// 索引缓冲（复用）
    index_buffer: PersistentBuffer,
}

impl SpriteRenderer {
    pub fn new(
        device: &wgpu::Device,
        pipeline: Arc<SpritePipeline>,
        sprite_capacity: usize,
    ) -> Self {
        // 计算缓冲区大小
        // 每个精灵: 4 顶点 * 36 字节 + 6 索引 * 2 字节 + 1 实例 * 64 字节
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

    /// 开始新帧，切换双缓冲
    pub fn begin_frame(&mut self) {
        self.current_frame = 1 - self.current_frame;
    }

    /// 获取当前帧的实例缓冲
    pub fn current_instance_buffer(&self) -> &PersistentBuffer {
        &self.instance_buffers[self.current_frame]
    }

    /// 获取当前帧的实例缓冲区索引 (0 or 1)
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }

    /// 上传批次数据到缓冲
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

    /// 批量上传所有批次数据，合并为 4 次 write_buffer 调用
    /// 返回每个批次的绘制偏移信息，用于间接绘制
    pub fn upload_batches(
        &self,
        queue: &wgpu::Queue,
        batches: &[SpriteBatch],
    ) -> Vec<BatchDrawInfo> {
        if batches.is_empty() {
            return Vec::new();
        }

        // 累积所有顶点数据
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

        // 4 次 write_buffer 调用替代 4N 次
        queue.write_buffer(self.vertex_buffer.buffer(), 0, &vertex_data);
        queue.write_buffer(self.index_buffer.buffer(), 0, &index_data);
        queue.write_buffer(self.current_instance_buffer().buffer(), 0, &instance_data);
        queue.write_buffer(self.indirect_buffer.buffer(), 0, &indirect_data);

        // 计算每个批次的间接偏移
        let indirect_stride = std::mem::size_of::<DrawIndexedIndirectArgs>() as u64;
        (0..batches.len())
            .map(|i| BatchDrawInfo {
                indirect_offset: i as u64 * indirect_stride,
            })
            .collect()
    }

    /// 获取绘制所需的缓冲区引用
    pub fn get_buffers(&self) -> SpriteRendererBuffers<'_> {
        SpriteRendererBuffers {
            vertex_buffer: self.vertex_buffer.buffer(),
            index_buffer: self.index_buffer.buffer(),
            instance_buffer: self.current_instance_buffer().buffer(),
            indirect_buffer: self.indirect_buffer.buffer(),
        }
    }

    /// 释放所有缓冲区所有权，用于导入 Render Graph
    pub fn into_buffers(self) -> SpriteRendererOwnedBuffers {
        SpriteRendererOwnedBuffers {
            vertex_buffer: self.vertex_buffer,
            index_buffer: self.index_buffer,
            instance_buffers: self.instance_buffers,
            indirect_buffer: self.indirect_buffer,
        }
    }

    /// 获取各缓冲区大小（字节）
    pub fn buffer_sizes(&self) -> (usize, usize, usize, usize) {
        (
            self.vertex_buffer.size(),
            self.index_buffer.size(),
            self.instance_buffers[0].size(),
            self.indirect_buffer.size(),
        )
    }
}

/// 绘制所需的缓冲区引用集合
pub struct SpriteRendererBuffers<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub instance_buffer: &'a wgpu::Buffer,
    pub indirect_buffer: &'a wgpu::Buffer,
}

/// 拥有所有权的缓冲区集合，用于导入 Render Graph
pub struct SpriteRendererOwnedBuffers {
    pub vertex_buffer: PersistentBuffer,
    pub index_buffer: PersistentBuffer,
    pub instance_buffers: [PersistentBuffer; 2],
    pub indirect_buffer: PersistentBuffer,
}
