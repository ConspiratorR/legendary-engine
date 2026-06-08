use super::error::FontError;
use super::loader::FontLoader;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

const ATLAS_SIZE: u32 = 1024;
const GLYPH_PADDING: u32 = 1;

/// Cache key for a rasterized glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_hash: u64,
    pub ch: char,
    pub size: u32,
}

/// Cached glyph placement and metrics.
#[derive(Debug, Clone)]
pub struct GlyphEntry {
    pub atlas_index: u32,
    pub uv: [f32; 4],
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

struct Shelf {
    y: u32,
    height: u32,
    cursor_x: u32,
}

struct ShelfPacker {
    shelves: Vec<Shelf>,
    next_y: u32,
}

impl ShelfPacker {
    fn new() -> Self {
        Self {
            shelves: Vec::new(),
            next_y: 0,
        }
    }

    fn try_pack(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        for shelf in &mut self.shelves {
            if height <= shelf.height {
                let x = shelf.cursor_x;
                if x + width + GLYPH_PADDING <= ATLAS_SIZE {
                    shelf.cursor_x += width + GLYPH_PADDING;
                    return Some((x, shelf.y));
                }
            }
        }

        let y = self.next_y;
        if y + height + GLYPH_PADDING > ATLAS_SIZE {
            return None;
        }

        self.shelves.push(Shelf {
            y,
            height,
            cursor_x: width + GLYPH_PADDING,
        });
        self.next_y += height + GLYPH_PADDING;
        Some((0, y))
    }
}

struct AtlasPage {
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    packer: ShelfPacker,
}

/// Dynamic texture atlas with shelf-based bin packing.
///
/// Manages multiple 1024x1024 RGBA8 texture pages. Glyphs are packed into
/// horizontal shelves within each page. New pages are created on demand.
pub struct GlyphAtlas {
    pages: Vec<AtlasPage>,
    cache: HashMap<GlyphKey, GlyphEntry>,
    sampler: wgpu::Sampler,
    texture_layout: wgpu::BindGroupLayout,
}

impl GlyphAtlas {
    /// Create a new atlas with one empty texture page.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: wgpu::BindGroupLayout,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("font_atlas_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let mut atlas = Self {
            pages: Vec::new(),
            cache: HashMap::new(),
            sampler,
            texture_layout,
        };

        atlas.create_page(device, queue);
        atlas
    }

    fn create_page(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        let index = self.pages.len() as u32;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("font_atlas_page_{index}")),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let clear_data = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &clear_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE * 4),
                rows_per_image: Some(ATLAS_SIZE),
            },
            wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("font_atlas_bind_group_{index}")),
            layout: &self.texture_layout,
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

        self.pages.push(AtlasPage {
            texture,
            view,
            bind_group,
            packer: ShelfPacker::new(),
        });

        index
    }

    /// Get a cached glyph or rasterize and pack it into the atlas.
    pub fn get_or_rasterize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        loader: &FontLoader,
        font_name: &str,
        ch: char,
        size: u32,
    ) -> Result<&GlyphEntry, FontError> {
        let font_hash = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            font_name.hash(&mut hasher);
            hasher.finish()
        };
        let key = GlyphKey {
            font_hash,
            ch,
            size,
        };

        if !self.cache.contains_key(&key) {
            let bitmap = loader.rasterize(font_name, ch, size as f32)?;

            let entry = if bitmap.width == 0 || bitmap.height == 0 {
                GlyphEntry {
                    atlas_index: 0,
                    uv: [0.0, 0.0, 0.0, 0.0],
                    width: 0,
                    height: 0,
                    advance: bitmap.advance,
                    bearing_x: bitmap.bearing_x,
                    bearing_y: bitmap.bearing_y,
                }
            } else {
                self.pack_glyph(device, queue, &bitmap)?
            };

            self.cache.insert(key, entry);
        }

        Ok(self.cache.get(&key).unwrap())
    }

    fn pack_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bitmap: &super::loader::GlyphBitmap,
    ) -> Result<GlyphEntry, FontError> {
        let w = bitmap.width;
        let h = bitmap.height;

        let mut packed_page: Option<u32> = None;
        let mut packed_pos: Option<(u32, u32)> = None;

        for (i, page) in self.pages.iter_mut().enumerate() {
            if let Some(pos) = page.packer.try_pack(w, h) {
                packed_page = Some(i as u32);
                packed_pos = Some(pos);
                break;
            }
        }

        let (page_index, (x, y)) = if let (Some(pi), Some(pos)) = (packed_page, packed_pos) {
            (pi, pos)
        } else {
            let pi = self.create_page(device, queue);
            let pos = self.pages[pi as usize]
                .packer
                .try_pack(w, h)
                .ok_or(FontError::AtlasFull(ch_placeholder()))?;
            (pi, pos)
        };

        let page = &self.pages[page_index as usize];

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &page.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &bitmap.pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let atlas_w = ATLAS_SIZE as f32;
        let atlas_h = ATLAS_SIZE as f32;
        let uv = [
            x as f32 / atlas_w,
            y as f32 / atlas_h,
            (x + w) as f32 / atlas_w,
            (y + h) as f32 / atlas_h,
        ];

        Ok(GlyphEntry {
            atlas_index: page_index,
            uv,
            width: w,
            height: h,
            advance: bitmap.advance,
            bearing_x: bitmap.bearing_x,
            bearing_y: bitmap.bearing_y,
        })
    }

    /// Get the bind group for a specific atlas texture page.
    pub fn bind_group(&self, atlas_index: u32) -> &wgpu::BindGroup {
        &self.pages[atlas_index as usize].bind_group
    }

    /// Number of texture pages in the atlas.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Number of cached glyph entries.
    pub fn cached_glyph_count(&self) -> usize {
        self.cache.len()
    }
}

fn ch_placeholder() -> char {
    '\0'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_key_hash_eq() {
        let a = GlyphKey {
            font_hash: 42,
            ch: 'A',
            size: 32,
        };
        let b = GlyphKey {
            font_hash: 42,
            ch: 'A',
            size: 32,
        };
        assert_eq!(a, b);

        let mut h1 = std::collections::hash_map::DefaultHasher::new();
        let mut h2 = std::collections::hash_map::DefaultHasher::new();
        a.hash(&mut h1);
        b.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_glyph_key_different_char() {
        let a = GlyphKey {
            font_hash: 42,
            ch: 'A',
            size: 32,
        };
        let b = GlyphKey {
            font_hash: 42,
            ch: 'B',
            size: 32,
        };
        assert_ne!(a, b);
    }
}
