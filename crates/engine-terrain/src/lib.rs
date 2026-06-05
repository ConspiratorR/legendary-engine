//! Terrain system for the RustEngine.
//!
//! Provides heightmap-based terrain with:
//! - Chunked mesh generation from heightmap data
//! - Sculpting brushes (raise, lower, smooth, flatten)
//! - Splat map texture painting (up to 4 layers)
//! - Vegetation system with density maps and LOD
//! - Editor panel integration
//!
//! # Quick Start
//!
//! ```rust
//! use engine_terrain::components::{Terrain, SplatMap, TerrainTextureLayers};
//! use engine_math::Vec2;
//!
//! // Create a 129×129 terrain covering 100×100 world units
//! let mut terrain = Terrain::new(128, 64, Vec2::new(100.0, 100.0), 50.0);
//!
//! // Read/write heights (raw value, multiplied by height_scale on read)
//! terrain.set_height(64, 64, 1.0);
//! let h = terrain.get_height(64, 64); // == 50.0
//!
//! // Paint texture layers via a splat map
//! let mut splat = SplatMap::new(terrain.resolution);
//! splat.paint(64, 64, 1, 1.0); // shift weight toward layer 1
//!
//! // Add texture layers
//! let mut layers = TerrainTextureLayers::default();
//! layers.add_layer("Grass".to_string());
//! layers.add_layer("Rock".to_string());
//! ```

pub mod error;
pub use error::TerrainError;

pub mod brush;
pub mod components;
pub mod mesh_gen;
pub mod paint;
pub mod plugin;
pub mod raycast;
pub mod vegetation;
