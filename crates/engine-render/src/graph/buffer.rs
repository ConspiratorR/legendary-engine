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
    pub import: bool,
}

impl BufferNode {
    pub fn new(desc: BufferDesc) -> Self {
        Self {
            desc,
            buffer: None,
            import: false,
        }
    }

    #[allow(dead_code)]
    pub fn imported(buffer: wgpu::Buffer) -> Self {
        Self {
            desc: BufferDesc::new(buffer.size(), buffer.usage()),
            buffer: Some(buffer),
            import: true,
        }
    }
}
