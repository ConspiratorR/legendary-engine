use std::ptr;

/// 持久映射的 GPU 缓冲
/// 实现零拷贝上传，CPU 直接写入 GPU 内存
pub struct PersistentBuffer {
    buffer: wgpu::Buffer,
    size: usize,
    mapped_ptr: *mut u8,
}

unsafe impl Send for PersistentBuffer {}
unsafe impl Sync for PersistentBuffer {}

impl PersistentBuffer {
    /// 创建持久映射缓冲
    ///
    /// # Safety
    /// 返回的缓冲区的映射指针必须在缓冲区有效期内有效
    pub fn new(device: &wgpu::Device, size: usize, label: Option<&str>) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: size as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let mapped_ptr = buffer.slice(..).get_mapped_range_mut().as_mut_ptr();

        Self {
            buffer,
            size,
            mapped_ptr,
        }
    }

    /// 获取缓冲区引用
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// 获取缓冲区大小
    pub fn size(&self) -> usize {
        self.size
    }

    /// 写入数据到缓冲区指定偏移
    ///
    /// # Safety
    /// 调用者必须确保 offset + data.len() <= self.size
    pub unsafe fn write(&self, offset: usize, data: &[u8]) {
        debug_assert!(
            offset + data.len() <= self.size,
            "Write out of bounds: offset={}, len={}, size={}",
            offset,
            data.len(),
            self.size
        );

        unsafe {
            let dst = self.mapped_ptr.add(offset);
            ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }
    }

    /// 取消映射（在 GPU 使用前调用）
    pub fn unmap(&self) {
        self.buffer.unmap();
    }
}
