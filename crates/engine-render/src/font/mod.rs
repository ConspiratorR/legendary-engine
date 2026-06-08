//! Font rendering subsystem.
//!
//! Provides TTF/OTF font loading, glyph rasterization, dynamic texture atlas,
//! and text-to-SpriteDraw conversion for rendering text via the sprite pipeline.

pub mod atlas;
pub mod error;
pub mod loader;
pub mod painter;

pub use atlas::{GlyphAtlas, GlyphEntry, GlyphKey};
pub use error::FontError;
pub use loader::{FontLoader, GlyphBitmap, GlyphMetrics};
pub use painter::TextPainter;
