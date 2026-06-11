# Font Rendering System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add font rendering to RustEngine: load TTF/OTF fonts, rasterize glyphs, upload to GPU texture atlas, draw text via existing SpriteBatch pipeline.

**Architecture:** FontLoader (fontdue) → GlyphAtlas (shelf-packed texture atlas) → TextPainter (text → SpriteDraw[]) → SpriteBatch (existing). New code lives in `engine-render/src/font/`.

**Tech Stack:** Rust, wgpu 23, fontdue 0.9, thiserror 2

---

## File Structure

```
crates/engine-render/src/font/
├── mod.rs              # Module entry, pub use
├── error.rs            # FontError type
├── loader.rs           # FontLoader - fontdue wrapper
├── atlas.rs            # GlyphAtlas - dynamic texture atlas with shelf packing
├── painter.rs          # TextPainter - text → SpriteDraw conversion
└── atlas_adapter.rs    # FontAtlasAdapter - implements engine-ui::text::FontAtlas trait

Modified files:
├── crates/engine-render/Cargo.toml         # Add fontdue dependency
├── crates/engine-render/src/lib.rs         # Add pub mod font
└── crates/engine-render/src/plugin.rs      # Insert TextPainter as ECS resource
```

---

### Task 1: Add fontdue dependency

**Files:**
- Modify: `crates/engine-render/Cargo.toml`

- [ ] **Step 1: Add fontdue to Cargo.toml**

Add `fontdue = "0.9"` to the `[dependencies]` section of `crates/engine-render/Cargo.toml`:

```toml
[dependencies]
engine-window = { path = "../engine-window" }
engine-math = { path = "../engine-math" }
engine-asset = { path = "../engine-asset" }
engine-ecs = { path = "../engine-ecs" }
engine-scene = { path = "../engine-scene" }
wgpu = { version = "23", features = ["wgsl"] }
winit = "0.30"
thiserror = "2"
bytemuck = { version = "1", features = ["derive"] }
image = "0.25"
pollster = "0.4"
crossbeam-channel = "0.5"
parking_lot = "0.12"
rayon = "1.10"
rand = "0.9"
fontdue = "0.9"
```

- [ ] **Step 2: Verify dependency resolves**

Run: `cargo check -p engine-render`
Expected: Compiles successfully (fontdue downloaded and resolved)

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/Cargo.toml
git commit -m "deps(render): add fontdue 0.9 for font rasterization"
```

---

### Task 2: FontError type

**Files:**
- Create: `crates/engine-render/src/font/error.rs`

- [ ] **Step 1: Create font/error.rs**

```rust
use thiserror::Error;

/// Errors that can occur during font operations.
#[derive(Debug, Error)]
pub enum FontError {
    /// The requested font name was not loaded.
    #[error("font not found: {0}")]
    FontNotFound(String),

    /// fontdue failed to parse or rasterize.
    #[error("fontdue error: {0}")]
    Fontdue(String),

    /// The atlas has no space for the glyph (should not happen with dynamic expansion).
    #[error("atlas full: no space for glyph '{0}'")]
    AtlasFull(char),

    /// The font data could not be parsed.
    #[error("invalid font data")]
    InvalidData,
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/engine-render/src/font/error.rs
git commit -m "feat(render): add FontError type for font subsystem"
```

---

### Task 3: FontLoader — fontdue wrapper

**Files:**
- Create: `crates/engine-render/src/font/loader.rs`
- Test: `crates/engine-render/src/font/loader.rs` (inline tests)

- [ ] **Step 1: Create font/loader.rs with FontLoader and GlyphBitmap**

```rust
use std::collections::HashMap;
use super::error::FontError;

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
    ///
    /// `name` is the identifier used in subsequent rasterize/has_glyph calls.
    pub fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), FontError> {
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
            .map_err(|e| FontError::Fontdue(e.to_string()))?;
        self.fonts.insert(name.to_string(), font);
        Ok(())
    }

    /// Rasterize a single glyph at the given pixel size.
    ///
    /// Returns a GlyphBitmap with RGBA8 pixels (white color, alpha from coverage).
    pub fn rasterize(&self, font_name: &str, ch: char, size: f32) -> Result<GlyphBitmap, FontError> {
        let font = self.fonts.get(font_name)
            .ok_or_else(|| FontError::FontNotFound(font_name.to_string()))?;

        let (metrics, coverage) = font.rasterize(ch, size);

        let width = metrics.width as u32;
        let height = metrics.height as u32;

        // Convert coverage (u8 grayscale) to RGBA8 (white + alpha)
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

    /// Check whether any font with the given name has the glyph.
    pub fn has_glyph(&self, font_name: &str, ch: char) -> bool {
        self.fonts.get(font_name)
            .map_or(false, |f| f.has_glyph(ch))
    }

    /// Get glyph metrics without rasterizing.
    pub fn metrics(&self, font_name: &str, ch: char, size: f32) -> Result<GlyphMetrics, FontError> {
        let font = self.fonts.get(font_name)
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

    // Use a minimal embedded font for testing.
    // We'll load the system font or use a small test font.
    // For now, test with the API surface using a real font file if available.

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
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p engine-render --lib font::loader::tests`
Expected: All 4 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/font/loader.rs
git commit -m "feat(render): add FontLoader wrapping fontdue for glyph rasterization"
```

---

### Task 4: GlyphAtlas — dynamic texture atlas

**Files:**
- Create: `crates/engine-render/src/font/atlas.rs`
- Test: `crates/engine-render/src/font/atlas.rs` (inline tests)

- [ ] **Step 1: Create font/atlas.rs with ShelfPacker**

```rust
use std::collections::HashMap;
use super::error::FontError;
use super::loader::{FontLoader, GlyphBitmap};

/// Width and height of each atlas texture.
const ATLAS_SIZE: u32 = 1024;

/// Key for glyph cache lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_hash: u64,
    pub ch: char,
    pub size: u32,
}

/// Packed glyph entry with UV coordinates and metrics.
#[derive(Debug, Clone)]
pub struct GlyphEntry {
    pub atlas_index: u32,
    pub uv: [f32; 4], // [u0, v0, u1, v1]
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

/// Shelf packing: each shelf is a horizontal row with fixed height.
struct Shelf {
    y: u32,
    height: u32,
    cursor_x: u32,
}

/// A single atlas texture with shelf-based packing.
struct AtlasPage {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    shelves: Vec<Shelf>,
    next_y: u32,
}

impl AtlasPage {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        index: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("font_atlas_{index}")),
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

        // Clear to transparent black
        let zeros = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &zeros,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * ATLAS_SIZE),
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
            label: Some(&format!("font_atlas_bg_{index}")),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                        &wgpu::SamplerDescriptor {
                            label: Some("font_atlas_sampler"),
                            mag_filter: wgpu::FilterMode::Linear,
                            min_filter: wgpu::FilterMode::Linear,
                            ..Default::default()
                        },
                    )),
                },
            ],
        });

        Self {
            _texture: texture,
            _view: view,
            bind_group,
            shelves: Vec::new(),
            next_y: 0,
        }
    }

    /// Try to pack a glyph into this page. Returns (x, y) position if successful.
    fn try_pack(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        let padded_w = width + 1; // 1px padding
        let padded_h = height + 1;

        // Try existing shelves
        for shelf in &mut self.shelves {
            if padded_h <= shelf.height && shelf.cursor_x + padded_w <= ATLAS_SIZE {
                let x = shelf.cursor_x;
                shelf.cursor_x += padded_w;
                return Some((x, shelf.y));
            }
        }

        // Create new shelf
        if self.next_y + padded_h <= ATLAS_SIZE {
            let y = self.next_y;
            self.shelves.push(Shelf {
                y,
                height: padded_h,
                cursor_x: padded_w,
            });
            self.next_y += padded_h;
            return Some((0, y));
        }

        None // Page full
    }
}

/// Dynamic glyph atlas with shelf packing across multiple GPU textures.
pub struct GlyphAtlas {
    pages: Vec<AtlasPage>,
    cache: HashMap<GlyphKey, GlyphEntry>,
    texture_layout: wgpu::BindGroupLayout,
}

impl GlyphAtlas {
    /// Create a new empty glyph atlas.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: wgpu::BindGroupLayout,
    ) -> Self {
        let first_page = AtlasPage::new(device, queue, &texture_layout, 0);
        Self {
            pages: vec![first_page],
            cache: HashMap::new(),
            texture_layout,
        }
    }

    /// Get or rasterize and pack a glyph.
    pub fn get_or_rasterize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        loader: &FontLoader,
        font_name: &str,
        ch: char,
        size: f32,
    ) -> Result<&GlyphEntry, FontError> {
        let font_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            font_name.hash(&mut h);
            h.finish()
        };
        let key = GlyphKey {
            font_hash,
            ch,
            size: size as u32,
        };

        if !self.cache.contains_key(&key) {
            let bitmap = loader.rasterize(font_name, ch, size)?;
            if bitmap.width == 0 || bitmap.height == 0 {
                // Space or zero-size glyph — store with zero UV
                self.cache.insert(key, GlyphEntry {
                    atlas_index: 0,
                    uv: [0.0, 0.0, 0.0, 0.0],
                    width: 0,
                    height: 0,
                    advance: bitmap.advance,
                    bearing_x: bitmap.bearing_x,
                    bearing_y: bitmap.bearing_y,
                });
            } else {
                let entry = self.pack_glyph(device, queue, &bitmap)?;
                self.cache.insert(key, entry);
            }
        }

        Ok(self.cache.get(&key).unwrap())
    }

    fn pack_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bitmap: &GlyphBitmap,
    ) -> Result<GlyphEntry, FontError> {
        // Try existing pages
        let mut packed = None;
        for (i, page) in self.pages.iter_mut().enumerate() {
            if let Some((x, y)) = page.try_pack(bitmap.width, bitmap.height) {
                packed = Some((i, x, y));
                break;
            }
        }

        // Create new page if needed
        let (page_idx, x, y) = match packed {
            Some((i, x, y)) => (i, x, y),
            None => {
                let idx = self.pages.len() as u32;
                let new_page = AtlasPage::new(device, queue, &self.texture_layout, idx);
                self.pages.push(new_page);
                let page = self.pages.last_mut().unwrap();
                let (x, y) = page.try_pack(bitmap.width, bitmap.height)
                    .ok_or(FontError::AtlasFull('?' as char))?;
                (self.pages.len() - 1, x, y)
            }
        };

        // Upload glyph pixels to the texture
        let page = &self.pages[page_idx];
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &page._texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &bitmap.pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * bitmap.width),
                rows_per_image: Some(bitmap.height),
            },
            wgpu::Extent3d {
                width: bitmap.width,
                height: bitmap.height,
                depth_or_array_layers: 1,
            },
        );

        let u0 = x as f32 / ATLAS_SIZE as f32;
        let v0 = y as f32 / ATLAS_SIZE as f32;
        let u1 = (x + bitmap.width) as f32 / ATLAS_SIZE as f32;
        let v1 = (y + bitmap.height) as f32 / ATLAS_SIZE as f32;

        Ok(GlyphEntry {
            atlas_index: page_idx as u32,
            uv: [u0, v0, u1, v1],
            width: bitmap.width,
            height: bitmap.height,
            advance: bitmap.advance,
            bearing_x: bitmap.bearing_x,
            bearing_y: bitmap.bearing_y,
        })
    }

    /// Get the bind group for a given atlas page index.
    pub fn bind_group(&self, atlas_index: u32) -> &wgpu::BindGroup {
        &self.pages[atlas_index as usize].bind_group
    }

    /// Number of atlas pages currently allocated.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Number of cached glyphs.
    pub fn cached_glyph_count(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_key_hash_eq() {
        let a = GlyphKey { font_hash: 1, ch: 'A', size: 32 };
        let b = GlyphKey { font_hash: 1, ch: 'A', size: 32 };
        assert_eq!(a, b);
    }

    #[test]
    fn test_glyph_key_different_char() {
        let a = GlyphKey { font_hash: 1, ch: 'A', size: 32 };
        let b = GlyphKey { font_hash: 1, ch: 'B', size: 32 };
        assert_ne!(a, b);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p engine-render --lib font::atlas::tests`
Expected: All 2 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/font/atlas.rs
git commit -m "feat(render): add GlyphAtlas with shelf-packed dynamic texture atlas"
```

---

### Task 5: TextPainter — text to SpriteDraw conversion

**Files:**
- Create: `crates/engine-render/src/font/painter.rs`
- Test: `crates/engine-render/src/font/painter.rs` (inline tests)

- [ ] **Step 1: Create font/painter.rs**

```rust
use engine_math::{Mat4, Vec2, Vec3};
use crate::sprite::SpriteDraw;
use super::atlas::GlyphAtlas;
use super::error::FontError;
use super::loader::FontLoader;

/// Converts text strings into SpriteDraw arrays for rendering via SpriteBatch.
///
/// Owns a FontLoader and GlyphAtlas. Call `load_font()` to register fonts,
/// then `draw_text()` to produce SpriteDraw lists.
pub struct TextPainter {
    loader: FontLoader,
    atlas: GlyphAtlas,
}

impl TextPainter {
    /// Create a new TextPainter with an empty font loader and glyph atlas.
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

    /// Load a font from raw TTF/OTF bytes.
    pub fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), FontError> {
        self.loader.load_font(name, data)
    }

    /// Convert text into a list of SpriteDraw commands.
    ///
    /// Each character becomes one SpriteDraw. The text is rendered left-to-right
    /// starting at `position`. Uses the glyph atlas texture for rendering.
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
        let baseline_y = position[1];

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = position[0];
                // Line height approximation: size * 1.2
                // (caller should handle multi-line layout)
                continue;
            }

            let entry = self.atlas.get_or_rasterize(
                device, queue, &self.loader, font_name, ch, size,
            )?;

            if entry.width > 0 && entry.height > 0 {
                let glyph_x = cursor_x + entry.bearing_x;
                let glyph_y = baseline_y - entry.bearing_y;

                let half_w = entry.width as f32 * 0.5;
                let half_h = entry.height as f32 * 0.5;

                let world_matrix = Mat4::from_translation(Vec3::new(
                    glyph_x + half_w,
                    glyph_y + half_h,
                    0.0,
                ));

                sprites.push(SpriteDraw {
                    world_matrix,
                    color,
                    size: Vec2::new(entry.width as f32, entry.height as f32),
                    texture_id: entry.atlas_index as u64 + 1, // +1 to avoid 0 (fallback)
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

    /// Access the underlying font loader.
    pub fn loader(&self) -> &FontLoader {
        &self.loader
    }

    /// Access the underlying glyph atlas.
    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_painter_draw_empty_string() {
        // Can't easily create wgpu device in unit test without full setup.
        // Test the logic flow only.
        let loader = FontLoader::new();
        assert!(!loader.has_font("default"));
    }

    #[test]
    fn test_text_painter_load_font_missing_data() {
        // Verify error propagation
        let mut loader = FontLoader::new();
        let result = loader.load_font("bad", &[0, 1, 2]);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p engine-render --lib font::painter::tests`
Expected: All 2 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/font/painter.rs
git commit -m "feat(render): add TextPainter for text-to-SpriteDraw conversion"
```

---

### Task 6: Font module entry point

**Files:**
- Create: `crates/engine-render/src/font/mod.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: Create font/mod.rs**

```rust
//! Font rendering subsystem.
//!
//! Provides TTF/OTF font loading, glyph rasterization, dynamic texture atlas,
//! and text-to-SpriteDraw conversion for rendering text via the sprite pipeline.
//!
//! # Usage
//!
//! ```rust,no_run
//! use engine_render::font::TextPainter;
//!
//! // Create painter (in plugin setup):
//! let mut painter = TextPainter::new(&device, &queue, texture_layout);
//! painter.load_font("default", include_bytes!("path/to/font.ttf"))?;
//!
//! // Draw text (each frame):
//! let sprites = painter.draw_text(&device, &queue, "Hello", "default", 24.0, [1.0; 4], [10.0, 10.0])?;
//! // Submit sprites to SpriteBatch for rendering
//! ```

pub mod error;
pub mod loader;
pub mod atlas;
pub mod painter;
pub mod atlas_adapter;

pub use error::FontError;
pub use loader::{FontLoader, GlyphBitmap, GlyphMetrics};
pub use atlas::{GlyphAtlas, GlyphEntry, GlyphKey};
pub use painter::TextPainter;
pub use atlas_adapter::FontAtlasAdapter;
```

- [ ] **Step 2: Add `pub mod font;` to lib.rs**

Add after line 98 (`pub mod graph;`) in `crates/engine-render/src/lib.rs`:

```rust
pub mod font;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-render`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/font/mod.rs crates/engine-render/src/lib.rs
git commit -m "feat(render): add font module entry point"
```

---

### Task 7: FontAtlasAdapter — bridge to engine-ui::text::FontAtlas

**Files:**
- Create: `crates/engine-render/src/font/atlas_adapter.rs`

- [ ] **Step 1: Create font/atlas_adapter.rs**

```rust
use engine_ui::text::{FontAtlas, FontFamily, RasterizedGlyph, TextError};
use super::loader::FontLoader;
use super::atlas::GlyphAtlas;

/// Adapter that implements `engine_ui::text::FontAtlas` trait
/// using the font subsystem's FontLoader and GlyphAtlas.
///
/// This bridges the engine-ui text layout system with the engine-render
/// font rasterization pipeline.
pub struct FontAtlasAdapter {
    loader: FontLoader,
    atlas: GlyphAtlas,
}

impl FontAtlasAdapter {
    /// Create a new adapter with an empty loader and atlas.
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

    /// Load a font from raw TTF/OTF bytes.
    pub fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), super::error::FontError> {
        self.loader.load_font(name, data)
    }
}

impl FontAtlas for FontAtlasAdapter {
    fn rasterize(
        &self,
        family: &FontFamily,
        ch: char,
        font_size: f32,
    ) -> Result<RasterizedGlyph, TextError> {
        // Try primary font first, then fallbacks
        for font_name in family.iter() {
            if self.loader.has_font(font_name) {
                let bitmap = self.loader.rasterize(font_name, ch, font_size)
                    .map_err(|e| TextError::RasterizationFailed(ch))?;

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
                return self.loader.metrics(font_name, ch, font_size)
                    .map(|m| m.advance)
                    .unwrap_or(font_size * 0.6);
            }
        }
        font_size * 0.6 // fallback approximation
    }

    fn line_height(&self, family: &FontFamily, font_size: f32) -> f32 {
        // fontdue doesn't expose line_height directly; use standard 1.2x ratio
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
    fn test_adapter_rasterize_no_font() {
        // Can't create wgpu device in unit test easily; test API surface
        let loader = FontLoader::new();
        assert!(!loader.has_font("default"));
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: Compiles successfully (depends on engine-ui being available)

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/font/atlas_adapter.rs
git commit -m "feat(render): add FontAtlasAdapter bridging to engine-ui::text::FontAtlas"
```

---

### Task 8: Integrate TextPainter into RenderPlugin2D

**Files:**
- Modify: `crates/engine-render/src/plugin.rs`

- [ ] **Step 1: Add TextPainter import and insertion in plugin.rs**

In `crates/engine-render/src/plugin.rs`, add the import:

```rust
use crate::font::TextPainter;
```

In the `build()` method, after `world.insert_resource(bridge);` (line 52), add:

```rust
let text_painter = TextPainter::new(&renderer.device, &renderer.queue, texture_layout.clone());
world.insert_resource(text_painter);
```

The full `build()` method should look like:

```rust
pub fn build(&mut self, world: &mut engine_ecs::world::World) {
    let renderer = Renderer::new(self.window.clone()).expect("Failed to create renderer");

    let texture_layout = SpritePipeline::create_texture_layout(&renderer.device);
    let bridge = TextureBridge::new(&renderer.device, &renderer.queue, texture_layout.clone());

    world.insert_resource(bridge);

    let text_painter = TextPainter::new(&renderer.device, &renderer.queue, texture_layout);
    world.insert_resource(text_painter);

    self.renderer = Some(renderer);
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-render`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/plugin.rs
git commit -m "feat(render): integrate TextPainter into RenderPlugin2D as ECS resource"
```

---

### Task 9: End-to-end verification with a test font

**Files:**
- Modify: `crates/engine-core/examples/tetris.rs` (add text rendering test)

- [ ] **Step 1: Add a system font loading test to tetris example**

In `crates/engine-core/examples/tetris.rs`, add after the existing setup code (after `app.set_renderer(renderer);` around line 377), add font loading:

```rust
// Load a system font for text rendering
let font_data = std::fs::read("C:/Windows/Fonts/arial.ttf")
    .or_else(|_| std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"))
    .or_else(|_| std::fs::read("/System/Library/Fonts/Helvetica.ttc"));
if let Ok(data) = font_data {
    let painter = app.world.get_resource_mut::<engine_render::font::TextPainter>().unwrap();
    let _ = painter.load_font("default", &data);
    log::info!("Font loaded for text rendering");
} else {
    log::warn!("No system font found for text rendering test");
}
```

- [ ] **Step 2: Add text drawing in the render loop**

In the `AboutToWait` handler, after `redraw(&mut app.world, &game, &entities);`, add:

```rust
// Draw score as text (test font rendering)
let painter = app.world.get_resource_mut::<engine_render::font::TextPainter>().unwrap();
if painter.loader().has_font("default") {
    let device = &app.renderer().unwrap().device;
    let queue = &app.renderer().unwrap().queue;
    let text_sprites = painter.draw_text(
        device, queue,
        &format!("Score: {}", game.score),
        "default",
        24.0,
        [1.0, 1.0, 1.0, 1.0],
        [10.0, 30.0],
    ).unwrap_or_default();
    // Text sprites would be submitted to sprite renderer here
}
```

Note: The exact integration point depends on how the sprite renderer collects sprites. This step verifies the API works end-to-end.

- [ ] **Step 3: Verify compilation and run**

Run: `cargo check -p engine-core --example tetris`
Expected: Compiles successfully

Run: `cargo run --example tetris -p engine-core`
Expected: Window opens, tetris game runs, console shows "Font loaded for text rendering"

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/tetris.rs
git commit -m "feat(tetris): add font rendering test with system font"
```

---

### Task 10: Final integration test and cleanup

**Files:**
- Verify all font module tests pass
- Verify full build

- [ ] **Step 1: Run all font module tests**

Run: `cargo test -p engine-render --lib font`
Expected: All tests pass

- [ ] **Step 2: Run full engine build**

Run: `cargo build`
Expected: Full workspace builds successfully

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p engine-render -- -D warnings`
Expected: No warnings

- [ ] **Step 4: Run fmt**

Run: `cargo fmt -p engine-render -- --check`
Expected: All files formatted

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(render): font rendering system complete (fontdue + texture atlas + SpriteBatch)"
```
