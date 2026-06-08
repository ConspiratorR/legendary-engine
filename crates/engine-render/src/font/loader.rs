use super::error::FontError;
use std::collections::HashMap;

/// Rasterized glyph bitmap in RGBA8 format.
pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    /// RGBA8 pixel data, row-major, top-left origin.
    /// White pixels with alpha from fontdue coverage.
    pub pixels: Vec<u8>,
}

/// Glyph metrics without bitmap data.
pub struct GlyphMetrics {
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub width: u32,
    pub height: u32,
}

/// Wraps fontdue for font loading and glyph rasterization.
pub struct FontLoader {
    fonts: HashMap<String, fontdue::Font>,
}

impl FontLoader {
    /// Create a new empty font loader.
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }

    /// Load a font from raw TTF/OTF bytes.
    pub fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), FontError> {
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
            .map_err(|e| FontError::Fontdue(e.to_string()))?;
        self.fonts.insert(name.to_string(), font);
        Ok(())
    }

    /// Rasterize a single glyph at the given pixel size.
    pub fn rasterize(
        &self,
        font_name: &str,
        ch: char,
        size: f32,
    ) -> Result<GlyphBitmap, FontError> {
        let font = self
            .fonts
            .get(font_name)
            .ok_or_else(|| FontError::FontNotFound(font_name.to_string()))?;

        let (metrics, coverage) = font.rasterize(ch, size);

        let width = metrics.width as u32;
        let height = metrics.height as u32;

        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for &alpha in &coverage {
            pixels.push(255); // R
            pixels.push(255); // G
            pixels.push(255); // B
            pixels.push(alpha); // A
        }

        Ok(GlyphBitmap {
            width,
            height,
            advance: metrics.advance_width,
            bearing_x: metrics.xmin as f32,
            bearing_y: metrics.ymin as f32,
            pixels,
        })
    }

    /// Check whether the font has the glyph.
    pub fn has_glyph(&self, font_name: &str, ch: char) -> bool {
        self.fonts.get(font_name).is_some_and(|f| f.has_glyph(ch))
    }

    /// Get glyph metrics without rasterizing.
    pub fn metrics(&self, font_name: &str, ch: char, size: f32) -> Result<GlyphMetrics, FontError> {
        let font = self
            .fonts
            .get(font_name)
            .ok_or_else(|| FontError::FontNotFound(font_name.to_string()))?;

        let (metrics, _coverage) = font.rasterize(ch, size);

        Ok(GlyphMetrics {
            advance: metrics.advance_width,
            bearing_x: metrics.xmin as f32,
            bearing_y: metrics.ymin as f32,
            width: metrics.width as u32,
            height: metrics.height as u32,
        })
    }

    /// Check if a font with the given name is loaded.
    pub fn has_font(&self, font_name: &str) -> bool {
        self.fonts.contains_key(font_name)
    }
}

impl Default for FontLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_loader_new_is_empty() {
        let loader = FontLoader::new();
        assert!(!loader.has_font("default"));
    }

    #[test]
    fn test_font_loader_load_invalid_data() {
        let mut loader = FontLoader::new();
        let result = loader.load_font("bad", &[0, 1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_font_loader_rasterize_missing_font() {
        let loader = FontLoader::new();
        let result = loader.rasterize("nonexistent", 'A', 32.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_font_loader_metrics_missing_font() {
        let loader = FontLoader::new();
        let result = loader.metrics("nonexistent", 'A', 32.0);
        assert!(result.is_err());
    }
}
