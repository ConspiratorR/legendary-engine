use super::{FontAtlas, FontFamily, RasterizedGlyph, TextError};
use engine_render::font::FontLoader;

/// Adapter that implements [`FontAtlas`] using engine-render's [`FontLoader`].
///
/// Bridges the engine-ui text system's font atlas abstraction with the
/// render crate's fontdue-based rasterizer. CPU-side only — no GPU resources
/// are needed for the trait methods.
pub struct FontAtlasAdapter {
    loader: FontLoader,
}

impl FontAtlasAdapter {
    /// Create a new empty adapter with no fonts loaded.
    pub fn new() -> Self {
        Self {
            loader: FontLoader::new(),
        }
    }

    /// Load a font from raw TTF/OTF bytes.
    pub fn load_font(
        &mut self,
        name: &str,
        data: &[u8],
    ) -> Result<(), engine_render::font::FontError> {
        self.loader.load_font(name, data)
    }
}

impl Default for FontAtlasAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl FontAtlas for FontAtlasAdapter {
    fn rasterize(
        &self,
        family: &FontFamily,
        ch: char,
        font_size: f32,
    ) -> Result<RasterizedGlyph, TextError> {
        for font_name in family.iter() {
            if self.loader.has_font(font_name) {
                let bitmap = self
                    .loader
                    .rasterize(font_name, ch, font_size)
                    .map_err(|_| TextError::RasterizationFailed(ch))?;
                return Ok(RasterizedGlyph {
                    ch,
                    width: bitmap.width,
                    height: bitmap.height,
                    advance: bitmap.advance,
                    bearing_x: bitmap.bearing_x,
                    bearing_y: bitmap.bearing_y,
                    pixels: bitmap.pixels,
                });
            }
        }
        Err(TextError::FontNotFound(family.primary().to_string()))
    }

    fn advance_width(&self, family: &FontFamily, ch: char, font_size: f32) -> f32 {
        for font_name in family.iter() {
            if self.loader.has_font(font_name) {
                return self
                    .loader
                    .metrics(font_name, ch, font_size)
                    .map(|m| m.advance)
                    .unwrap_or(font_size * 0.6);
            }
        }
        font_size * 0.6
    }

    fn line_height(&self, _family: &FontFamily, font_size: f32) -> f32 {
        font_size * 1.2
    }

    fn has_glyph(&self, family: &FontFamily, ch: char) -> bool {
        family.iter().any(|name| self.loader.has_glyph(name, ch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_default_is_empty() {
        let adapter = FontAtlasAdapter::new();
        let family = FontFamily::new("test");
        assert!(!adapter.has_glyph(&family, 'A'));
    }

    #[test]
    fn test_adapter_rasterize_missing_font() {
        let adapter = FontAtlasAdapter::new();
        let family = FontFamily::new("nonexistent");
        let result = adapter.rasterize(&family, 'A', 32.0);
        assert!(matches!(result, Err(TextError::FontNotFound(_))));
    }

    #[test]
    fn test_adapter_advance_width_fallback() {
        let adapter = FontAtlasAdapter::new();
        let family = FontFamily::new("nonexistent");
        let width = adapter.advance_width(&family, 'A', 20.0);
        assert!((width - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_adapter_line_height() {
        let adapter = FontAtlasAdapter::new();
        let family = FontFamily::new("test");
        let lh = adapter.line_height(&family, 20.0);
        assert!((lh - 24.0).abs() < f32::EPSILON);
    }
}
