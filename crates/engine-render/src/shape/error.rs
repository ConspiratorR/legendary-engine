use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShapeError {
    #[error("shader compilation failed: {0}")]
    ShaderCompilation(String),
    #[error("pipeline creation failed: {0}")]
    PipelineCreation(String),
}
