//! Rendering error types.

use thiserror::Error;

/// Errors that can occur in the rendering pipeline.
#[derive(Error, Debug)]
pub enum RenderError {
    #[error("GPU initialization failed: {0}")]
    GpuInitFailed(String),

    #[error("shader compilation failed: {0}")]
    ShaderCompilationFailed(String),

    #[error("pipeline creation failed: {0}")]
    PipelineCreationFailed(String),

    #[error("texture error: {0}")]
    TextureError(String),

    #[error("buffer error: {0}")]
    BufferError(String),

    #[error("surface error: {0}")]
    SurfaceError(String),

    #[error("render pass error: {0}")]
    RenderPassError(String),
}
