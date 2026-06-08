use engine_math::{Mat4, Vec2, Vec3};

use super::atlas::GlyphAtlas;
use super::error::FontError;
use super::loader::FontLoader;
use crate::sprite::SpriteDraw;

/// Converts text strings into SpriteDraw arrays for rendering via SpriteBatch.
pub struct TextPainter {
    loader: FontLoader,
    atlas: GlyphAtlas,
}

impl TextPainter {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: wgpu::BindGroupLayout,
    ) -> Self {
        Self {
            loader: FontLoader::new(),
            atlas: GlyphAtlas::new(device, queue, texture_layout),
        }
    }

    pub fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), FontError> {
        self.loader.load_font(name, data)
    }

    /// Convert text into SpriteDraw commands.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        font_name: &str,
        size: f32,
        color: [f32; 4],
        position: [f32; 2],
    ) -> Result<Vec<SpriteDraw>, FontError> {
        let mut sprites = Vec::with_capacity(text.len());
        let mut cursor_x = position[0];
        let mut baseline_y = position[1];

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = position[0];
                baseline_y += size;
                continue;
            }

            let entry = self.atlas.get_or_rasterize(
                device,
                queue,
                &self.loader,
                font_name,
                ch,
                size as u32,
            )?;

            if entry.width > 0 && entry.height > 0 {
                let glyph_x = cursor_x + entry.bearing_x;
                let glyph_y = baseline_y - entry.bearing_y;

                let half_w = entry.width as f32 * 0.5;
                let half_h = entry.height as f32 * 0.5;

                let world_matrix =
                    Mat4::from_translation(Vec3::new(glyph_x + half_w, glyph_y + half_h, 0.0));

                sprites.push(SpriteDraw {
                    world_matrix,
                    color,
                    size: Vec2::new(entry.width as f32, entry.height as f32),
                    texture_id: entry.atlas_index as u64 + 1,
                    flip_x: false,
                    flip_y: false,
                    depth: 0.0,
                    uv_region: entry.uv,
                });
            }

            cursor_x += entry.advance;
        }

        Ok(sprites)
    }

    pub fn loader(&self) -> &FontLoader {
        &self.loader
    }

    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_painter_draw_empty_string() {
        let loader = FontLoader::new();
        assert!(!loader.has_font("default"));
    }

    #[test]
    fn test_text_painter_load_font_missing_data() {
        let mut loader = FontLoader::new();
        let result = loader.load_font("bad", &[0, 1, 2]);
        assert!(result.is_err());
    }
}
