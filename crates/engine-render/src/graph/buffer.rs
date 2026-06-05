/// Descriptor for a GPU buffer resource in the render graph.
///
/// Buffers can be marked as `transient` to be allocated per-frame during
/// `compile()` and dropped after graph execution, reducing persistent memory usage.
#[derive(Debug, Clone)]
pub struct BufferDesc {
    pub label: Option<String>,
    pub size: u64,
    pub usage: wgpu::BufferUsages,
    pub transient: bool,
}

impl BufferDesc {
    /// Create a new buffer descriptor with the given size and usage flags.
    pub fn new(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            label: None,
            size,
            usage,
            transient: false,
        }
    }

    /// Set a debug label for this buffer.
    pub fn named(mut self, name: &str) -> Self {
        self.label = Some(name.to_string());
        self
    }

    /// Mark this buffer as transient (allocated per-frame, dropped after execution).
    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }
}

/// Internal node representing a buffer resource in the render graph.
///
/// Can own a GPU buffer, hold a non-owning reference to an external buffer,
/// or remain unallocated until graph compilation.
pub(crate) struct BufferNode {
    #[allow(dead_code)]
    pub desc: BufferDesc,
    pub buffer: Option<wgpu::Buffer>,
    /// 非拥有引用，指向外部缓冲区。调用者需确保缓冲区在图执行期间有效。
    pub ref_ptr: Option<*const wgpu::Buffer>,
    pub import: bool,
}

impl BufferNode {
    /// Create a new buffer node (unallocated until graph compilation).
    pub fn new(desc: BufferDesc) -> Self {
        Self {
            desc,
            buffer: None,
            ref_ptr: None,
            import: false,
        }
    }

    /// Create a buffer node that owns an externally-created GPU buffer.
    #[allow(dead_code)]
    pub fn imported(buffer: wgpu::Buffer) -> Self {
        Self {
            desc: BufferDesc::new(buffer.size(), buffer.usage()),
            buffer: Some(buffer),
            ref_ptr: None,
            import: true,
        }
    }

    /// Create a buffer node with a non-owning reference to an external buffer.
    ///
    /// The caller must ensure the referenced buffer outlives graph execution.
    pub fn imported_ref(buffer: &wgpu::Buffer) -> Self {
        Self {
            desc: BufferDesc::new(buffer.size(), buffer.usage()),
            buffer: None,
            ref_ptr: Some(buffer as *const wgpu::Buffer),
            import: true,
        }
    }

    /// Get a reference to the underlying GPU buffer, if allocated or imported.
    pub fn get_buffer(&self) -> Option<&wgpu::Buffer> {
        if let Some(ref buf) = self.buffer {
            Some(buf)
        } else if let Some(ptr) = self.ref_ptr {
            // SAFETY: ref_ptr is set only via import_buffer_ref, which borrows an
            // external Buffer. The caller guarantees the Buffer outlives this graph node.
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_desc_new() {
        let desc = BufferDesc::new(1024, wgpu::BufferUsages::VERTEX);
        assert_eq!(desc.size, 1024);
        assert_eq!(desc.usage, wgpu::BufferUsages::VERTEX);
        assert!(desc.label.is_none());
        assert!(!desc.transient);
    }

    #[test]
    fn test_buffer_desc_named() {
        let desc = BufferDesc::new(256, wgpu::BufferUsages::UNIFORM).named("my_buffer");
        assert_eq!(desc.label.as_deref(), Some("my_buffer"));
    }

    #[test]
    fn test_buffer_desc_transient() {
        let desc = BufferDesc::new(512, wgpu::BufferUsages::STORAGE).transient();
        assert!(desc.transient);
    }

    #[test]
    fn test_buffer_desc_chained_builders() {
        let desc = BufferDesc::new(64, wgpu::BufferUsages::INDEX)
            .named("indices")
            .transient();
        assert_eq!(desc.label.as_deref(), Some("indices"));
        assert!(desc.transient);
    }

    #[test]
    fn test_buffer_node_new() {
        let desc = BufferDesc::new(128, wgpu::BufferUsages::VERTEX);
        let node = BufferNode::new(desc);
        assert!(node.buffer.is_none());
        assert!(node.ref_ptr.is_none());
        assert!(!node.import);
        assert_eq!(node.desc.size, 128);
    }

    #[test]
    fn test_buffer_node_get_buffer_none() {
        let node = BufferNode::new(BufferDesc::new(64, wgpu::BufferUsages::VERTEX));
        assert!(node.get_buffer().is_none());
    }
}
