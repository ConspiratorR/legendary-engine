/// Descriptor for a GPU texture resource in the render graph.
///
/// Textures can be marked as `transient` to be allocated per-frame during
/// `compile()` and dropped after graph execution (e.g., depth buffers).
#[derive(Debug, Clone)]
pub struct TextureDesc {
    pub label: Option<String>,
    pub size: wgpu::Extent3d,
    pub mip_levels: u32,
    pub sample_count: u32,
    pub dimension: wgpu::TextureDimension,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub transient: bool,
}

impl TextureDesc {
    /// Create a 2D texture descriptor with the given dimensions, format, and usage.
    pub fn new_2d(
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_levels: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            transient: false,
        }
    }

    /// Set a debug label for this texture.
    pub fn named(mut self, name: &str) -> Self {
        self.label = Some(name.to_string());
        self
    }

    /// Mark this texture as transient (allocated per-frame, dropped after execution).
    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }
}

/// Handle to a texture resource in the render graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);

/// Handle to a buffer resource in the render graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u32);

/// Internal node representing a texture resource in the render graph.
pub(crate) struct TextureNode {
    pub desc: TextureDesc,
    pub texture: Option<wgpu::Texture>,
    pub view: Option<wgpu::TextureView>,
    pub import: bool,
}

impl TextureNode {
    /// Create a new texture node (unallocated until graph compilation).
    pub fn new(desc: TextureDesc) -> Self {
        Self {
            desc,
            texture: None,
            view: None,
            import: false,
        }
    }

    /// Create a texture node that owns an externally-created GPU texture and view.
    pub fn imported(texture: wgpu::Texture, view: wgpu::TextureView) -> Self {
        Self {
            desc: TextureDesc::new_2d(
                texture.width(),
                texture.height(),
                texture.format(),
                texture.usage(),
            ),
            texture: Some(texture),
            view: Some(view),
            import: true,
        }
    }

    /// Create a texture node with only a view (e.g., for swapchain surfaces).
    pub fn imported_view(view: wgpu::TextureView) -> Self {
        Self {
            desc: TextureDesc::new_2d(
                0,
                0,
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            texture: None,
            view: Some(view),
            import: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_desc_2d() {
        let desc = TextureDesc::new_2d(
            640,
            480,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        assert_eq!(desc.size.width, 640);
        assert_eq!(desc.size.height, 480);
        assert_eq!(desc.format, wgpu::TextureFormat::Bgra8UnormSrgb);
    }

    #[test]
    fn test_texture_handle_distinct() {
        let h1 = TextureHandle(0);
        let h2 = TextureHandle(1);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_texture_desc_default_transient_false() {
        let desc = TextureDesc::new_2d(
            100,
            100,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING,
        );
        assert!(!desc.transient);
        assert!(desc.label.is_none());
        assert_eq!(desc.mip_levels, 1);
        assert_eq!(desc.sample_count, 1);
        assert_eq!(desc.dimension, wgpu::TextureDimension::D2);
    }

    #[test]
    fn test_texture_desc_chained_builders() {
        let desc = TextureDesc::new_2d(
            256,
            256,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .named("shadow_map")
        .transient();
        assert_eq!(desc.label.as_deref(), Some("shadow_map"));
        assert!(desc.transient);
    }

    #[test]
    fn test_texture_handle_hash() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(TextureHandle(1), "a");
        map.insert(TextureHandle(2), "b");
        assert_eq!(map.get(&TextureHandle(1)), Some(&"a"));
        assert_eq!(map.get(&TextureHandle(2)), Some(&"b"));
        assert_eq!(map.get(&TextureHandle(3)), None);
    }

    #[test]
    fn test_buffer_handle_hash() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(BufferHandle(0), "vertex");
        map.insert(BufferHandle(1), "index");
        assert_eq!(map.get(&BufferHandle(0)), Some(&"vertex"));
    }
}
