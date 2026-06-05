//! # engine-terrain
//!
//! Heightmap-based terrain system for the RustEngine.
//!
//! ## Architecture
//!
//! The terrain is split into a grid of **chunks**, each backed by its own GPU mesh.
//! A single [`Terrain`] component holds the full heightmap; chunk entities
//! ([`TerrainChunk`]) are parented to it and lazily rebuild their meshes when
//! marked dirty.
//!
//! ### Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`components`] | Core data types: `Terrain`, `TerrainChunk`, `SplatMap`, `BrushSettings`, `VegetationData` |
//! | [`brush`] | Sculpting brushes (raise, lower, smooth, flatten) that modify the heightmap |
//! | [`paint`] | Splat-map texture painting with up to 4 blend layers |
//! | [`mesh_gen`] | Chunk mesh generation (vertices, indices, normals) |
//! | [`raycast`] | Screen-to-world raycasting and height sampling |
//! | [`vegetation`] | Vegetation placement with density maps, slope/height filters, and LOD |
//! | [`plugin`] | ECS plugin that wires mesh rebuild and vegetation systems |
//!
//! ## Quick Start
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
