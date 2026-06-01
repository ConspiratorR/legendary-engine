#[derive(Debug, Clone)]
pub struct BufferDesc {
    pub label: Option<String>,
    pub size: u64,
    pub usage: wgpu::BufferUsages,
    pub transient: bool,
}

impl BufferDesc {
    pub fn new(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            label: None,
            size,
            usage,
            transient: false,
        }
    }

    pub fn named(mut self, name: &str) -> Self {
        self.label = Some(name.to_string());
        self
    }

    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }
}

pub(crate) struct BufferNode {
    #[allow(dead_code)]
    pub desc: BufferDesc,
    pub buffer: Option<wgpu::Buffer>,
    /// 非拥有引用，指向外部缓冲区。调用者需确保缓冲区在图执行期间有效。
    pub ref_ptr: Option<*const wgpu::Buffer>,
    pub import: bool,
}

impl BufferNode {
    pub fn new(desc: BufferDesc) -> Self {
        Self {
            desc,
            buffer: None,
            ref_ptr: None,
            import: false,
        }
    }

    #[allow(dead_code)]
    pub fn imported(buffer: wgpu::Buffer) -> Self {
        Self {
            desc: BufferDesc::new(buffer.size(), buffer.usage()),
            buffer: Some(buffer),
            ref_ptr: None,
            import: true,
        }
    }

    pub fn imported_ref(buffer: &wgpu::Buffer) -> Self {
        Self {
            desc: BufferDesc::new(buffer.size(), buffer.usage()),
            buffer: None,
            ref_ptr: Some(buffer as *const wgpu::Buffer),
            import: true,
        }
    }

    pub fn get_buffer(&self) -> Option<&wgpu::Buffer> {
        if let Some(ref buf) = self.buffer {
            Some(buf)
        } else if let Some(ptr) = self.ref_ptr {
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }
}
