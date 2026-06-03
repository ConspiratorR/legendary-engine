//! Terrain system for the RustEngine.
//!
//! Provides heightmap-based terrain with:
//! - Chunked mesh generation from heightmap data
//! - Sculpting brushes (raise, lower, smooth, flatten)
//! - Splat map texture painting (up to 4 layers)
//! - Vegetation system with density maps and LOD
//! - Editor panel integration

pub mod brush;
pub mod components;
pub mod mesh_gen;
pub mod paint;
pub mod plugin;
pub mod raycast;
pub mod vegetation;
