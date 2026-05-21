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
    pub fn new_2d(width: u32, height: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> Self {
        Self {
            label: None,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_levels: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u32);

pub(crate) struct TextureNode {
    pub desc: TextureDesc,
    pub texture: Option<wgpu::Texture>,
    pub view: Option<wgpu::TextureView>,
    pub import: bool,
}

impl TextureNode {
    pub fn new(desc: TextureDesc) -> Self {
        Self { desc, texture: None, view: None, import: false }
    }

    pub fn imported(texture: wgpu::Texture, view: wgpu::TextureView) -> Self {
        Self {
            desc: TextureDesc::new_2d(texture.width(), texture.height(), texture.format(), texture.usage()),
            texture: Some(texture),
            view: Some(view),
            import: true,
        }
    }

    pub fn imported_view(view: wgpu::TextureView) -> Self {
        Self {
            desc: TextureDesc::new_2d(0, 0, wgpu::TextureFormat::Bgra8UnormSrgb, wgpu::TextureUsages::RENDER_ATTACHMENT),
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
        let desc = TextureDesc::new_2d(640, 480, wgpu::TextureFormat::Bgra8UnormSrgb, wgpu::TextureUsages::RENDER_ATTACHMENT);
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
}
