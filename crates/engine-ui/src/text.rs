//! Text rendering system with i18n, font fallback, and rich text support.
//!
//! Provides [`FontFamily`] for fallback chains, [`RichText`] for styled spans,
//! [`I18nStore`] for locale-aware translations, and [`TextRenderer`] for
//! cached text layout and draw command generation.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during text operations.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TextError {
    /// A font in the family could not be found or loaded.
    #[error("font not found: {0}")]
    FontNotFound(String),

    /// The requested locale is not loaded.
    #[error("locale not loaded: {0}")]
    LocaleNotLoaded(String),

    /// A translation key was not found in the current locale.
    #[error("translation key not found: {0}")]
    KeyNotFound(String),

    /// Glyph rasterization failed.
    #[error("rasterization failed for glyph '{0}'")]
    RasterizationFailed(char),
}

// ---------------------------------------------------------------------------
// Simple 2D vector (self-contained to avoid cross-crate dependency for tests)
// ---------------------------------------------------------------------------

/// A 2D vector used for text measurement results.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

// ---------------------------------------------------------------------------
// Text alignment
// ---------------------------------------------------------------------------

/// Horizontal text alignment within a layout box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

// ---------------------------------------------------------------------------
// Font family with fallback chain
// ---------------------------------------------------------------------------

/// A font family consisting of a primary font name and an ordered fallback
/// chain. When rendering, the system tries the primary font first, then walks
/// the fallback chain until a font that contains the required glyph is found.
///
/// # Example
/// ```
/// use engine_ui::text::FontFamily;
/// let family = FontFamily::new("Segoe UI")
///     .with_fallback("Microsoft YaHei")
///     .with_fallback("Noto Sans CJK");
/// assert_eq!(family.primary(), "Segoe UI");
/// assert_eq!(family.fallbacks().len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FontFamily {
    primary: String,
    fallbacks: Vec<String>,
}

impl FontFamily {
    /// Create a new font family with the given primary font name.
    pub fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            fallbacks: Vec::new(),
        }
    }

    /// Append a fallback font to the chain. Returns `self` for chaining.
    pub fn with_fallback(mut self, name: impl Into<String>) -> Self {
        self.fallbacks.push(name.into());
        self
    }

    /// The primary font name.
    pub fn primary(&self) -> &str {
        &self.primary
    }

    /// The ordered fallback font names.
    pub fn fallbacks(&self) -> &[String] {
        &self.fallbacks
    }

    /// Iterate over all font names starting with the primary.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.primary.as_str()).chain(self.fallbacks.iter().map(|s| s.as_str()))
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        Self::new("Segoe UI").with_fallback("Microsoft YaHei")
    }
}

// ---------------------------------------------------------------------------
// Font atlas trait
// ---------------------------------------------------------------------------

/// A rasterized glyph bitmap.
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// The character this glyph represents.
    pub ch: char,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Horizontal advance in pixels.
    pub advance: f32,
    /// X-bearing (offset from cursor to left edge of bitmap).
    pub bearing_x: f32,
    /// Y-bearing (offset from baseline to top edge of bitmap).
    pub bearing_y: f32,
    /// Raw RGBA8 pixel data, row-major, top-left origin.
    pub pixels: Vec<u8>,
}

/// Abstraction for glyph rasterization.
///
/// Implementors load font data and produce [`RasterizedGlyph`] bitmaps on
/// demand. The actual rasterizer (e.g. `rustybuzz` + `ab_glyph`, or `fontdue`)
/// is chosen by the concrete implementation and can be swapped at runtime.
pub trait FontAtlas: Send + Sync {
    /// Rasterize a single glyph from the given font family at the specified
    /// pixel size. Returns `None` if no font in the family contains the glyph.
    fn rasterize(
        &self,
        family: &FontFamily,
        ch: char,
        font_size: f32,
    ) -> Result<RasterizedGlyph, TextError>;

    /// Measure the horizontal advance width of a character.
    fn advance_width(&self, family: &FontFamily, ch: char, font_size: f32) -> f32;

    /// The line height (ascent + descent + line gap) for the given font size.
    fn line_height(&self, family: &FontFamily, font_size: f32) -> f32;

    /// Check whether any font in the family contains the given glyph.
    fn has_glyph(&self, family: &FontFamily, ch: char) -> bool;
}

// ---------------------------------------------------------------------------
// Text style
// ---------------------------------------------------------------------------

/// Styling properties applied to a run of text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// The font family to use.
    pub font_family: FontFamily,
    /// Font size in points.
    pub font_size: f32,
    /// Text color as RGBA bytes.
    pub color: [u8; 4],
    /// Bold weight.
    pub bold: bool,
    /// Italic style.
    pub italic: bool,
    /// Underline decoration.
    pub underline: bool,
    /// Strikethrough decoration.
    pub strikethrough: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: FontFamily::default(),
            font_size: 16.0,
            color: [255, 255, 255, 255],
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

impl TextStyle {
    /// Create a builder for this style.
    pub fn builder() -> TextStyleBuilder {
        TextStyleBuilder::default()
    }
}

/// Builder for [`TextStyle`].
#[derive(Debug, Default)]
pub struct TextStyleBuilder {
    style: TextStyle,
}

impl TextStyleBuilder {
    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.style.font_family = family;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.style.font_size = size;
        self
    }

    pub fn color(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.style.color = [r, g, b, a];
        self
    }

    pub fn bold(mut self, bold: bool) -> Self {
        self.style.bold = bold;
        self
    }

    pub fn italic(mut self, italic: bool) -> Self {
        self.style.italic = italic;
        self
    }

    pub fn underline(mut self, underline: bool) -> Self {
        self.style.underline = underline;
        self
    }

    pub fn strikethrough(mut self, strikethrough: bool) -> Self {
        self.style.strikethrough = strikethrough;
        self
    }

    pub fn build(self) -> TextStyle {
        self.style
    }
}

// ---------------------------------------------------------------------------
// Rich text (span-based)
// ---------------------------------------------------------------------------

/// A span of text with its own style, used as a building block for [`RichText`].
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    /// The text content of this span.
    pub text: String,
    /// The style applied to this span. `None` means inherit from context.
    pub style: Option<TextStyle>,
}

/// A rich text object composed of multiple styled spans.
///
/// # Example
/// ```
/// use engine_ui::text::{RichText, TextStyle};
///
/// let rich = RichText::new()
///     .span("Hello ")
///     .styled_span("world", TextStyle::builder().bold(true).build())
///     .build();
/// assert_eq!(rich.spans().len(), 2);
/// ```
#[derive(Debug, Clone, Default)]
pub struct RichText {
    spans: Vec<TextSpan>,
}

impl RichText {
    /// Create an empty rich text builder.
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Append an unstyled span.
    pub fn span(mut self, text: impl Into<String>) -> Self {
        self.spans.push(TextSpan {
            text: text.into(),
            style: None,
        });
        self
    }

    /// Append a span with an explicit style override.
    pub fn styled_span(mut self, text: impl Into<String>, style: TextStyle) -> Self {
        self.spans.push(TextSpan {
            text: text.into(),
            style: Some(style),
        });
        self
    }

    /// Consume the builder and return the list of spans.
    pub fn build(self) -> Self {
        self
    }

    /// The spans that compose this rich text.
    pub fn spans(&self) -> &[TextSpan] {
        &self.spans
    }

    /// Concatenate all span text into a single plain string.
    pub fn plain_text(&self) -> String {
        let mut s = String::new();
        for span in &self.spans {
            s.push_str(&span.text);
        }
        s
    }

    /// Total character count across all spans.
    pub fn char_count(&self) -> usize {
        self.spans.iter().map(|s| s.text.chars().count()).sum()
    }
}

// ---------------------------------------------------------------------------
// Glyph info (per-character layout data)
// ---------------------------------------------------------------------------

/// Positional information for a single laid-out glyph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphInfo {
    /// The character.
    pub ch: char,
    /// X position of the glyph's left bearing edge.
    pub x: f32,
    /// Y position of the glyph's baseline.
    pub y: f32,
    /// Horizontal advance to the next glyph.
    pub advance: f32,
    /// Index of the line this glyph belongs to (0-based).
    pub line: u32,
    /// Index of the span this glyph belongs to.
    pub span_index: usize,
}

// ---------------------------------------------------------------------------
// Text layout
// ---------------------------------------------------------------------------

/// The result of shaping and laying out text: glyph positions, line breaks,
/// and overall metrics.
#[derive(Debug, Clone)]
pub struct TextLayout {
    /// Per-glyph positional data, in visual order.
    pub glyphs: Vec<GlyphInfo>,
    /// Total measured size of the laid-out text (width, height).
    pub size: Vec2,
    /// Number of lines in the layout.
    pub line_count: u32,
    /// The alignment used during layout.
    pub alignment: TextAlign,
}

impl TextLayout {
    /// Create an empty layout.
    pub fn empty() -> Self {
        Self {
            glyphs: Vec::new(),
            size: Vec2::ZERO,
            line_count: 0,
            alignment: TextAlign::default(),
        }
    }

    /// Whether the layout contains no glyphs.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

// ---------------------------------------------------------------------------
// I18n store
// ---------------------------------------------------------------------------

/// A key-value translation store supporting multiple locales and nested keys.
///
/// Keys use dot-separated segments (e.g. `"menu.file.open"`). The store
/// resolves keys by walking the hierarchy.
///
/// # Example
/// ```
/// use engine_ui::text::I18nStore;
///
/// let mut store = I18nStore::new();
/// store.load_locale("en", &[
///     ("menu.file.open", "Open"),
///     ("menu.file.save", "Save"),
/// ]);
/// store.load_locale("zh", &[
///     ("menu.file.open", "打开"),
///     ("menu.file.save", "保存"),
/// ]);
///
/// store.set_locale("en");
/// assert_eq!(store.translate("menu.file.open"), Ok("Open"));
///
/// store.set_locale("zh");
/// assert_eq!(store.translate("menu.file.open"), Ok("打开"));
/// ```
#[derive(Debug, Default)]
pub struct I18nStore {
    /// locale -> (key -> value)
    translations: HashMap<String, HashMap<String, String>>,
    /// The currently active locale.
    current_locale: String,
}

impl I18nStore {
    /// Create an empty store with no locales loaded.
    pub fn new() -> Self {
        Self {
            translations: HashMap::new(),
            current_locale: String::new(),
        }
    }

    /// Load a set of key-value pairs for the given locale. If the locale
    /// already exists, new keys are merged and existing keys are overwritten.
    pub fn load_locale(&mut self, locale: &str, entries: &[(&str, &str)]) {
        let map = self.translations.entry(locale.to_string()).or_default();
        for &(key, value) in entries {
            map.insert(key.to_string(), value.to_string());
        }
    }

    /// Switch the active locale. Returns [`TextError::LocaleNotLoaded`] if the
    /// locale has not been loaded.
    pub fn set_locale(&mut self, locale: &str) -> Result<(), TextError> {
        if !self.translations.contains_key(locale) {
            return Err(TextError::LocaleNotLoaded(locale.to_string()));
        }
        self.current_locale = locale.to_string();
        Ok(())
    }

    /// The currently active locale tag, or empty string if none is set.
    pub fn current_locale(&self) -> &str {
        &self.current_locale
    }

    /// List all loaded locale tags.
    pub fn locales(&self) -> Vec<&str> {
        self.translations.keys().map(|s| s.as_str()).collect()
    }

    /// Translate a key in the current locale. The key may use dot-separated
    /// segments (e.g. `"menu.file.open"`).
    pub fn translate(&self, key: &str) -> Result<&str, TextError> {
        let map = self
            .translations
            .get(&self.current_locale)
            .ok_or_else(|| TextError::LocaleNotLoaded(self.current_locale.clone()))?;

        // Try exact key first (fast path).
        if let Some(value) = map.get(key) {
            return Ok(value.as_str());
        }

        // Try nested resolution: "a.b.c" -> look up "a" -> sub-key "b" -> sub-key "c".
        // Since we store flat keys, we just try the exact key. If not found, error.
        Err(TextError::KeyNotFound(key.to_string()))
    }

    /// Translate a key and perform placeholder substitution. Placeholders in
    /// the translation value use `{name}` syntax.
    ///
    /// # Example
    /// ```
    /// use engine_ui::text::I18nStore;
    ///
    /// let mut store = I18nStore::new();
    /// store.load_locale("en", &[("greeting", "Hello, {name}!")]);
    /// store.set_locale("en").unwrap();
    ///
    /// let result = store.translate_fmt("greeting", &[("name", "World")]);
    /// assert_eq!(result, Ok("Hello, World!".to_string()));
    /// ```
    pub fn translate_fmt(&self, key: &str, args: &[(&str, &str)]) -> Result<String, TextError> {
        let template = self.translate(key)?;
        let mut result = template.to_string();
        for &(name, value) in args {
            result = result.replace(&format!("{{{name}}}"), value);
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Draw command
// ---------------------------------------------------------------------------

/// A command produced by the text renderer, describing what to draw.
#[derive(Debug, Clone)]
pub enum DrawCommand {
    /// Draw a glyph bitmap at the given position.
    Glyph {
        /// Position in points (x, y) of the glyph's top-left corner.
        position: [f32; 2],
        /// The rasterized glyph data.
        glyph: RasterizedGlyph,
        /// RGBA color.
        color: [u8; 4],
    },
    /// Draw an underline or strikethrough decoration.
    Decoration {
        /// Start position (x, y).
        start: [f32; 2],
        /// End position (x, y).
        end: [f32; 2],
        /// Thickness in points.
        thickness: f32,
        /// RGBA color.
        color: [u8; 4],
    },
}

// ---------------------------------------------------------------------------
// Text renderer
// ---------------------------------------------------------------------------

/// Cached text layout engine that manages font atlases and produces draw
/// commands for rendering text.
///
/// The renderer caches [`TextLayout`] results keyed by `(text_hash, style_hash)`
/// to avoid re-shaping text every frame. Call [`layout`](Self::layout) to get
/// or compute a layout, and [`draw_commands`](Self::draw_commands) to produce
/// GPU-ready draw instructions.
pub struct TextRenderer {
    /// The font atlas used for glyph rasterization.
    atlas: Box<dyn FontAtlas>,
    /// Layout cache: (text, style_hash) -> TextLayout.
    layout_cache: HashMap<(u64, u64), TextLayout>,
}

impl TextRenderer {
    /// Create a new text renderer with the given font atlas.
    pub fn new(atlas: Box<dyn FontAtlas>) -> Self {
        Self {
            atlas,
            layout_cache: HashMap::new(),
        }
    }

    /// Compute a hash of a text string for cache keying.
    fn hash_text(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Compute a hash of a text style for cache keying.
    fn hash_style(style: &TextStyle) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        style.font_family.primary().hash(&mut hasher);
        for fb in style.font_family.fallbacks() {
            fb.hash(&mut hasher);
        }
        style.font_size.to_bits().hash(&mut hasher);
        style.bold.hash(&mut hasher);
        style.italic.hash(&mut hasher);
        style.underline.hash(&mut hasher);
        style.strikethrough.hash(&mut hasher);
        hasher.finish()
    }

    /// Lay out a plain string with the given style and alignment. Results are
    /// cached so repeated calls with the same inputs are fast.
    pub fn layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        alignment: TextAlign,
        max_width: Option<f32>,
    ) -> TextLayout {
        let text_hash = Self::hash_text(text);
        let style_hash = Self::hash_style(style);
        // Mix alignment into the hash.
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        style_hash.hash(&mut h);
        alignment.hash(&mut h);
        if let Some(mw) = max_width {
            mw.to_bits().hash(&mut h);
        }
        let key = (text_hash, h.finish());

        if let Some(cached) = self.layout_cache.get(&key) {
            return cached.clone();
        }

        let layout = self.compute_layout(text, style, alignment, max_width);
        self.layout_cache.insert(key, layout.clone());
        layout
    }

    /// Core layout computation (no caching).
    fn compute_layout(
        &self,
        text: &str,
        style: &TextStyle,
        alignment: TextAlign,
        max_width: Option<f32>,
    ) -> TextLayout {
        if text.is_empty() {
            return TextLayout::empty();
        }

        let family = &style.font_family;
        let font_size = style.font_size;
        let line_h = self.atlas.line_height(family, font_size);

        // If word wrapping is requested, wrap first.
        let lines: Vec<String> = if let Some(mw) = max_width {
            wrap_text_impl(&*self.atlas, text, mw, style)
        } else {
            vec![text.to_string()]
        };

        let mut glyphs = Vec::new();
        let mut max_x: f32 = 0.0;
        let mut y: f32 = 0.0;

        for (line_idx, line) in lines.iter().enumerate() {
            // Measure line width for alignment.
            let line_width: f32 = line
                .chars()
                .map(|c| self.atlas.advance_width(family, c, font_size))
                .sum();

            let x_offset = match alignment {
                TextAlign::Left => 0.0,
                TextAlign::Center => {
                    if let Some(mw) = max_width {
                        (mw - line_width) / 2.0
                    } else {
                        0.0
                    }
                }
                TextAlign::Right => {
                    if let Some(mw) = max_width {
                        mw - line_width
                    } else {
                        0.0
                    }
                }
            };

            let mut x = x_offset;
            for ch in line.chars() {
                let advance = self.atlas.advance_width(family, ch, font_size);
                glyphs.push(GlyphInfo {
                    ch,
                    x,
                    y,
                    advance,
                    line: line_idx as u32,
                    span_index: 0,
                });
                x += advance;
            }
            max_x = max_x.max(x);
            y += line_h;
        }

        TextLayout {
            glyphs,
            size: Vec2::new(max_x, y),
            line_count: lines.len() as u32,
            alignment,
        }
    }

    /// Generate draw commands for a laid-out text at the given origin.
    pub fn draw_commands(
        &self,
        layout: &TextLayout,
        style: &TextStyle,
        origin: [f32; 2],
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::with_capacity(layout.glyphs.len());
        let family = &style.font_family;
        let font_size = style.font_size;

        for glyph in &layout.glyphs {
            match self.atlas.rasterize(family, glyph.ch, font_size) {
                Ok(rasterized) => {
                    let x = origin[0] + glyph.x + rasterized.bearing_x;
                    let y = origin[1] + glyph.y - rasterized.bearing_y;
                    commands.push(DrawCommand::Glyph {
                        position: [x, y],
                        glyph: rasterized,
                        color: style.color,
                    });
                }
                Err(_) => {
                    // Skip glyphs that cannot be rasterized (e.g. spaces).
                }
            }

            // Decoration: underline
            if style.underline {
                let deco_y = origin[1] + glyph.y + font_size * 0.1;
                commands.push(DrawCommand::Decoration {
                    start: [origin[0] + glyph.x, deco_y],
                    end: [origin[0] + glyph.x + glyph.advance, deco_y],
                    thickness: (font_size * 0.05).max(1.0),
                    color: style.color,
                });
            }

            // Decoration: strikethrough
            if style.strikethrough {
                let deco_y = origin[1] + glyph.y - font_size * 0.3;
                commands.push(DrawCommand::Decoration {
                    start: [origin[0] + glyph.x, deco_y],
                    end: [origin[0] + glyph.x + glyph.advance, deco_y],
                    thickness: (font_size * 0.05).max(1.0),
                    color: style.color,
                });
            }
        }

        commands
    }

    /// Clear the layout cache. Call when fonts or styles change globally.
    pub fn clear_cache(&mut self) {
        self.layout_cache.clear();
    }

    /// Number of cached layouts.
    pub fn cache_size(&self) -> usize {
        self.layout_cache.len()
    }
}

// ---------------------------------------------------------------------------
// Free functions: measure_text, wrap_text
// ---------------------------------------------------------------------------

/// Measure the size of a plain text string in points, using the given style.
///
/// This is a convenience wrapper that creates a temporary layout. For repeated
/// measurements, prefer using a [`TextRenderer`] directly.
pub fn measure_text(atlas: &dyn FontAtlas, text: &str, style: &TextStyle) -> Vec2 {
    if text.is_empty() {
        return Vec2::ZERO;
    }

    let family = &style.font_family;
    let font_size = style.font_size;
    let line_h = atlas.line_height(family, font_size);

    let mut max_width: f32 = 0.0;
    let mut current_width: f32 = 0.0;
    let mut line_count: u32 = 1;

    for ch in text.chars() {
        if ch == '\n' {
            max_width = max_width.max(current_width);
            current_width = 0.0;
            line_count += 1;
        } else {
            current_width += atlas.advance_width(family, ch, font_size);
        }
    }
    max_width = max_width.max(current_width);

    Vec2::new(max_width, line_h * line_count as f32)
}

/// Wrap plain text into multiple lines that fit within `max_width` points.
///
/// Breaking occurs at word boundaries (spaces) and explicit newlines.
/// If a single word exceeds `max_width`, it is placed on its own line.
pub fn wrap_text(text: &str, max_width: f32, style: &TextStyle) -> Vec<String> {
    // We need a FontAtlas to measure; use a simple approximation based on
    // font_size since we don't have an atlas in this free function context.
    // For accurate wrapping, use TextRenderer::layout with max_width.
    wrap_text_approx(text, max_width, style.font_size)
}

/// Approximate word-wrap using an average character width heuristic.
/// For precise wrapping, use `TextRenderer::layout` with a real atlas.
fn wrap_text_approx(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    // Average character width heuristic: ~0.55 * font_size for Latin scripts.
    let avg_char_w = font_size * 0.55;
    let max_chars = ((max_width / avg_char_w).floor() as usize).max(1);

    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let words: Vec<&str> = paragraph.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line.push_str(word);
            } else if current_line.len() + 1 + word.len() <= max_chars {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Internal word-wrap using a real atlas for measurement.
fn wrap_text_impl(
    atlas: &dyn FontAtlas,
    text: &str,
    max_width: f32,
    style: &TextStyle,
) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let family = &style.font_family;
    let font_size = style.font_size;
    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let words: Vec<&str> = paragraph.split_whitespace().collect();
        let mut current_line = String::new();
        let mut current_width: f32 = 0.0;

        for word in words {
            let word_width: f32 = word
                .chars()
                .map(|c| atlas.advance_width(family, c, font_size))
                .sum();
            let space_width = atlas.advance_width(family, ' ', font_size);

            if current_line.is_empty() {
                current_line.push_str(word);
                current_width = word_width;
            } else if current_width + space_width + word_width <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
                current_width += space_width + word_width;
            } else {
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_width;
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Minimal FontAtlas for testing ----------------------------------------

    struct TestAtlas;

    impl FontAtlas for TestAtlas {
        fn rasterize(
            &self,
            _family: &FontFamily,
            ch: char,
            font_size: f32,
        ) -> Result<RasterizedGlyph, TextError> {
            if ch == ' ' {
                return Err(TextError::RasterizationFailed(ch));
            }
            let w = (font_size * 0.6) as u32;
            let h = font_size as u32;
            Ok(RasterizedGlyph {
                ch,
                width: w.max(1),
                height: h.max(1),
                advance: font_size * 0.6,
                bearing_x: 0.0,
                bearing_y: font_size * 0.8,
                pixels: vec![255; (w.max(1) * h.max(1) * 4) as usize],
            })
        }

        fn advance_width(&self, _family: &FontFamily, ch: char, font_size: f32) -> f32 {
            if ch == '\n' { 0.0 } else { font_size * 0.6 }
        }

        fn line_height(&self, _family: &FontFamily, font_size: f32) -> f32 {
            font_size * 1.2
        }

        fn has_glyph(&self, _family: &FontFamily, ch: char) -> bool {
            ch != '\u{FFFF}'
        }
    }

    fn test_style() -> TextStyle {
        TextStyle {
            font_family: FontFamily::new("TestFont"),
            font_size: 20.0,
            color: [255, 255, 255, 255],
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }

    // -- FontFamily tests ----------------------------------------------------

    #[test]
    fn test_font_family_fallback_chain() {
        let family = FontFamily::new("Segoe UI")
            .with_fallback("Microsoft YaHei")
            .with_fallback("Noto Sans CJK");
        assert_eq!(family.primary(), "Segoe UI");
        assert_eq!(family.fallbacks().len(), 2);
        assert_eq!(family.fallbacks()[0], "Microsoft YaHei");
        assert_eq!(family.fallbacks()[1], "Noto Sans CJK");

        let all: Vec<&str> = family.iter().collect();
        assert_eq!(all, vec!["Segoe UI", "Microsoft YaHei", "Noto Sans CJK"]);
    }

    #[test]
    fn test_font_family_default() {
        let family = FontFamily::default();
        assert_eq!(family.primary(), "Segoe UI");
        assert!(!family.fallbacks().is_empty());
    }

    // -- RichText tests -------------------------------------------------------

    #[test]
    fn test_rich_text_builder() {
        let rich = RichText::new()
            .span("Hello ")
            .styled_span("bold", TextStyle::builder().bold(true).build())
            .span(" world")
            .build();

        assert_eq!(rich.spans().len(), 3);
        assert_eq!(rich.plain_text(), "Hello bold world");
        assert_eq!(rich.char_count(), 16);
    }

    #[test]
    fn test_rich_text_empty() {
        let rich = RichText::new();
        assert!(rich.spans().is_empty());
        assert_eq!(rich.plain_text(), "");
        assert_eq!(rich.char_count(), 0);
    }

    // -- I18nStore tests -------------------------------------------------------

    #[test]
    fn test_i18n_basic_translation() {
        let mut store = I18nStore::new();
        store.load_locale("en", &[("hello", "Hello"), ("bye", "Goodbye")]);
        store.load_locale("zh", &[("hello", "你好"), ("bye", "再见")]);

        store.set_locale("en").unwrap();
        assert_eq!(store.translate("hello"), Ok("Hello"));
        assert_eq!(store.translate("bye"), Ok("Goodbye"));

        store.set_locale("zh").unwrap();
        assert_eq!(store.translate("hello"), Ok("你好"));
    }

    #[test]
    fn test_i18n_nested_keys() {
        let mut store = I18nStore::new();
        store.load_locale(
            "en",
            &[
                ("menu.file.open", "Open"),
                ("menu.file.save", "Save"),
                ("menu.edit.copy", "Copy"),
            ],
        );

        store.set_locale("en").unwrap();
        assert_eq!(store.translate("menu.file.open"), Ok("Open"));
        assert_eq!(store.translate("menu.edit.copy"), Ok("Copy"));
    }

    #[test]
    fn test_i18n_translate_fmt() {
        let mut store = I18nStore::new();
        store.load_locale(
            "en",
            &[
                ("greeting", "Hello, {name}!"),
                ("count", "You have {n} items"),
            ],
        );
        store.set_locale("en").unwrap();

        assert_eq!(
            store.translate_fmt("greeting", &[("name", "World")]),
            Ok("Hello, World!".to_string())
        );
        assert_eq!(
            store.translate_fmt("count", &[("n", "5")]),
            Ok("You have 5 items".to_string())
        );
    }

    #[test]
    fn test_i18n_missing_locale_error() {
        let mut store = I18nStore::new();
        assert!(store.set_locale("fr").is_err());
    }

    #[test]
    fn test_i18n_missing_key_error() {
        let mut store = I18nStore::new();
        store.load_locale("en", &[("hello", "Hello")]);
        store.set_locale("en").unwrap();
        assert!(store.translate("nonexistent").is_err());
    }

    #[test]
    fn test_i18n_locale_list() {
        let mut store = I18nStore::new();
        store.load_locale("en", &[]);
        store.load_locale("zh", &[]);
        store.load_locale("ja", &[]);
        let mut locales = store.locales();
        locales.sort();
        assert_eq!(locales, vec!["en", "ja", "zh"]);
    }

    // -- Text measurement tests -----------------------------------------------

    #[test]
    fn test_measure_text_empty() {
        let atlas = TestAtlas;
        let style = test_style();
        assert_eq!(measure_text(&atlas, "", &style), Vec2::ZERO);
    }

    #[test]
    fn test_measure_text_single_line() {
        let atlas = TestAtlas;
        let style = test_style(); // font_size = 20, advance = 12.0 per char
        let size = measure_text(&atlas, "Hello", &style);
        // 5 chars * 12.0 = 60.0 width, 20 * 1.2 = 24.0 height
        assert!((size.x - 60.0).abs() < f32::EPSILON);
        assert!((size.y - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_measure_text_multiline() {
        let atlas = TestAtlas;
        let style = test_style();
        let size = measure_text(&atlas, "Hi\nWorld", &style);
        // "World" = 5 chars * 12.0 = 60.0 (longer line)
        assert!((size.x - 60.0).abs() < f32::EPSILON);
        // 2 lines * 24.0 = 48.0
        assert!((size.y - 48.0).abs() < f32::EPSILON);
    }

    // -- Word wrap tests ------------------------------------------------------

    #[test]
    fn test_wrap_text_short_line() {
        let style = test_style();
        let lines = wrap_text("Hello World", 500.0, &style);
        assert_eq!(lines, vec!["Hello World"]);
    }

    #[test]
    fn test_wrap_text_forces_break() {
        // font_size=20, avg_char_w = 11.0, max_width=30 => max_chars ~= 2
        let style = test_style();
        let lines = wrap_text("Hello World", 30.0, &style);
        assert!(lines.len() > 1, "should wrap into multiple lines");
        // Each word should appear exactly once across all lines
        let combined = lines.join(" ");
        assert!(combined.contains("Hello"));
        assert!(combined.contains("World"));
    }

    #[test]
    fn test_wrap_text_preserves_newlines() {
        let style = test_style();
        let lines = wrap_text("Line1\nLine2", 500.0, &style);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Line1");
        assert_eq!(lines[1], "Line2");
    }

    #[test]
    fn test_wrap_text_empty() {
        let style = test_style();
        let lines = wrap_text("", 100.0, &style);
        assert_eq!(lines, vec![""]);
    }

    // -- TextRenderer tests ---------------------------------------------------

    #[test]
    fn test_renderer_layout_caching() {
        let mut renderer = TextRenderer::new(Box::new(TestAtlas));
        let style = test_style();

        let l1 = renderer.layout("Hello", &style, TextAlign::Left, None);
        assert_eq!(renderer.cache_size(), 1);

        let l2 = renderer.layout("Hello", &style, TextAlign::Left, None);
        assert_eq!(renderer.cache_size(), 1);
        assert_eq!(l1.size.x, l2.size.x);

        // Different text should create a new cache entry.
        let _l3 = renderer.layout("World", &style, TextAlign::Left, None);
        assert_eq!(renderer.cache_size(), 2);
    }

    #[test]
    fn test_renderer_draw_commands() {
        let renderer = TextRenderer::new(Box::new(TestAtlas));
        let style = test_style();
        let layout = TextLayout {
            glyphs: vec![GlyphInfo {
                ch: 'H',
                x: 0.0,
                y: 0.0,
                advance: 12.0,
                line: 0,
                span_index: 0,
            }],
            size: Vec2::new(12.0, 24.0),
            line_count: 1,
            alignment: TextAlign::Left,
        };

        let cmds = renderer.draw_commands(&layout, &style, [10.0, 20.0]);
        assert!(!cmds.is_empty());
        // Should have at least one Glyph command.
        assert!(cmds.iter().any(|c| matches!(c, DrawCommand::Glyph { .. })));
    }

    #[test]
    fn test_renderer_clear_cache() {
        let mut renderer = TextRenderer::new(Box::new(TestAtlas));
        let style = test_style();
        renderer.layout("Hello", &style, TextAlign::Left, None);
        assert_eq!(renderer.cache_size(), 1);
        renderer.clear_cache();
        assert_eq!(renderer.cache_size(), 0);
    }

    #[test]
    fn test_renderer_alignment_affects_layout() {
        let mut renderer = TextRenderer::new(Box::new(TestAtlas));
        let style = test_style();

        let left = renderer.layout("Hi", &style, TextAlign::Left, Some(200.0));
        let center = renderer.layout("Hi", &style, TextAlign::Center, Some(200.0));
        let right = renderer.layout("Hi", &style, TextAlign::Right, Some(200.0));

        // The first glyph's x position should differ by alignment.
        let left_x = left.glyphs[0].x;
        let center_x = center.glyphs[0].x;
        let right_x = right.glyphs[0].x;

        assert!(left_x < center_x, "left should be left of center");
        assert!(center_x < right_x, "center should be left of right");
    }

    #[test]
    fn test_text_layout_empty() {
        let layout = TextLayout::empty();
        assert!(layout.is_empty());
        assert_eq!(layout.line_count, 0);
        assert_eq!(layout.size, Vec2::ZERO);
    }

    #[test]
    fn test_draw_commands_underline_strikethrough() {
        let renderer = TextRenderer::new(Box::new(TestAtlas));
        let style = TextStyle {
            underline: true,
            strikethrough: true,
            ..test_style()
        };
        let layout = TextLayout {
            glyphs: vec![GlyphInfo {
                ch: 'A',
                x: 0.0,
                y: 0.0,
                advance: 12.0,
                line: 0,
                span_index: 0,
            }],
            size: Vec2::new(12.0, 24.0),
            line_count: 1,
            alignment: TextAlign::Left,
        };

        let cmds = renderer.draw_commands(&layout, &style, [0.0, 0.0]);
        let deco_count = cmds
            .iter()
            .filter(|c| matches!(c, DrawCommand::Decoration { .. }))
            .count();
        // One underline + one strikethrough per glyph.
        assert_eq!(deco_count, 2);
    }

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::builder()
            .font_size(24.0)
            .color(255, 0, 0, 255)
            .bold(true)
            .build();

        assert_eq!(style.font_size, 24.0);
        assert_eq!(style.color, [255, 0, 0, 255]);
        assert!(style.bold);
        assert!(!style.italic);
    }

    #[test]
    fn test_rich_text_styled_span_style() {
        let custom_style = TextStyle::builder()
            .font_size(32.0)
            .color(0, 255, 0, 255)
            .build();

        let rich = RichText::new()
            .styled_span("green", custom_style.clone())
            .build();

        assert_eq!(rich.spans()[0].style.as_ref().unwrap().font_size, 32.0);
        assert_eq!(
            rich.spans()[0].style.as_ref().unwrap().color,
            [0, 255, 0, 255]
        );
    }
}
