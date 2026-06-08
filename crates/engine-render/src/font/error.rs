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
