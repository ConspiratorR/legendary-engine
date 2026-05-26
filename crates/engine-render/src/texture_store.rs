use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TextureLoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image decode error: {0}")]
    Decode(String),
    #[error("Invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
}

pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

pub struct TextureStore {
    textures: HashMap<u64, GpuTexture>,
    bind_groups: HashMap<u64, wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    fallback_id: u64,
    next_id: u64,
}

impl TextureStore {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture_store_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create fallback 2x2 magenta/black checkerboard
        let fallback_pixels: [u8; 16] = [
            255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
        ];
        let fallback_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback_texture"),
            size: wgpu::Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &fallback_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &fallback_pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 2),
                rows_per_image: Some(2),
            },
            wgpu::Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
        );
        let fallback_view = fallback_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let fallback_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fallback_bind_group"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let mut textures = HashMap::new();
        let mut bind_groups = HashMap::new();
        textures.insert(
            0,
            GpuTexture {
                texture: fallback_texture,
                view: fallback_view,
                width: 2,
                height: 2,
            },
        );
        bind_groups.insert(0, fallback_bind_group);

        Self {
            textures,
            bind_groups,
            sampler,
            fallback_id: 0,
            next_id: 1,
        }
    }

    pub fn load(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        path: &str,
    ) -> Result<u64, TextureLoadError> {
        let bytes = std::fs::read(path)?;
        let img =
            image::load_from_memory(&bytes).map_err(|e| TextureLoadError::Decode(e.to_string()))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        self.load_from_bytes(device, queue, texture_layout, &rgba, width, height)
    }

    pub fn load_from_bytes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<u64, TextureLoadError> {
        if width == 0 || height == 0 {
            return Err(TextureLoadError::InvalidDimensions { width, height });
        }
        let expected_len = (4 * width * height) as usize;
        if pixels.len() < expected_len {
            return Err(TextureLoadError::InvalidDimensions { width, height });
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let id = self.next_id;
        self.next_id += 1;
        self.textures.insert(
            id,
            GpuTexture {
                texture,
                view,
                width,
                height,
            },
        );
        self.bind_groups.insert(id, bind_group);
        Ok(id)
    }

    pub fn get_bind_group(&self, id: u64) -> &wgpu::BindGroup {
        self.bind_groups
            .get(&id)
            .unwrap_or_else(|| &self.bind_groups[&self.fallback_id])
    }

    pub fn get_size(&self, id: u64) -> (u32, u32) {
        self.textures
            .get(&id)
            .map(|t| (t.width, t.height))
            .unwrap_or((2, 2))
    }

    pub fn contains(&self, id: u64) -> bool {
        self.textures.contains_key(&id)
    }

    pub fn unload(&mut self, id: u64) {
        if id == self.fallback_id {
            return;
        }
        self.textures.remove(&id);
        self.bind_groups.remove(&id);
    }

    pub fn create_render_target(
        &mut self,
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        label: Option<&str>,
    ) -> u64 {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let id = self.next_id;
        self.next_id += 1;
        self.textures.insert(
            id,
            GpuTexture {
                texture,
                view,
                width,
                height,
            },
        );
        self.bind_groups.insert(id, bind_group);
        id
    }

    pub fn get_render_target_view(&self, key: u64) -> Option<&wgpu::TextureView> {
        self.textures.get(&key).map(|t| &t.view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap()
    }

    fn test_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_texture_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    #[test]
    fn test_fallback_exists() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let store = TextureStore::new(&device, &queue, &layout);
        assert!(store.contains(0));
        assert_eq!(store.get_size(0), (2, 2));
    }

    #[test]
    fn test_invalid_id_returns_fallback() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let store = TextureStore::new(&device, &queue, &layout);
        let bg = store.get_bind_group(999);
        let fallback_bg = store.get_bind_group(0);
        assert!(std::ptr::eq(bg, fallback_bg));
    }

    #[test]
    fn test_load_from_bytes() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let mut store = TextureStore::new(&device, &queue, &layout);
        let pixels = vec![255u8, 0, 0, 255]; // 1x1 red
        let id = store
            .load_from_bytes(&device, &queue, &layout, &pixels, 1, 1)
            .unwrap();
        assert_eq!(id, 1);
        assert!(store.contains(id));
        assert_eq!(store.get_size(id), (1, 1));
    }

    #[test]
    fn test_unload() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let mut store = TextureStore::new(&device, &queue, &layout);
        let pixels = vec![255u8, 0, 0, 255];
        let id = store
            .load_from_bytes(&device, &queue, &layout, &pixels, 1, 1)
            .unwrap();
        store.unload(id);
        assert!(!store.contains(id));
    }

    #[test]
    fn test_cannot_unload_fallback() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let mut store = TextureStore::new(&device, &queue, &layout);
        store.unload(0);
        assert!(store.contains(0));
    }
}
