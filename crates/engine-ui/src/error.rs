//! UI system error types.

use thiserror::Error;

/// Errors that can occur in the UI system.
#[derive(Error, Debug)]
pub enum UiError {
    #[error("widget not found: {0}")]
    WidgetNotFound(String),

    #[error("layout error: {0}")]
    LayoutError(String),

    #[error("theme error: {0}")]
    ThemeError(String),

    #[error("font error: {0}")]
    FontError(String),
}
