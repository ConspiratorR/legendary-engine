//! Terrain system error types.

use thiserror::Error;

/// Errors that can occur in the terrain system.
#[derive(Error, Debug)]
pub enum TerrainError {
    #[error("invalid heightmap: {0}")]
    InvalidHeightmap(String),

    #[error("invalid layer: {0}")]
    InvalidLayer(String),

    #[error("terrain generation failed: {0}")]
    GenerationFailed(String),

    #[error("sculpt error: {0}")]
    SculptError(String),
}
