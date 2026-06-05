//! # engine-render
//!
//! Rendering system for the RustEngine.
//!
//! A wgpu-based rendering pipeline featuring:
//! - Render graph for organizing render passes
//! - Sprite pipeline with batching
//! - 3D PBR rendering with deferred shading
//! - Shadow mapping (CSM)
//! - Environment mapping / IBL
//! - Camera system with ECS integration
//! - 2D/3D particle systems
//! - Tilemap support
//!
//! ## Architecture
//!
//! The rendering pipeline is organized as a render graph:
//!
//! ```text
//! [Camera] -> [Shadow Pass] -> [G-Buffer Pass] -> [Lighting Pass] -> [Post-Processing] -> [Output]
//! ```
//!
//! Each pass reads from and writes to GPU resources (textures, buffers)
//! managed by the resource manager.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use engine_render::camera::{Camera, Projection};
//! use engine_render::graph::RenderGraph;
//!
//! // Create a camera
//! let camera = Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
//!
//! // Create a render graph
//! let mut graph = RenderGraph::new();
//! ```

pub mod error;
pub use error::RenderError;

pub mod animation;
pub mod atmosphere;
pub mod bloom;
pub mod camera;
pub mod camera_system;
pub mod collect_system;
pub mod command_batch;
pub mod culling;
pub mod culling_system;
pub mod deferred;
pub mod frustum;
pub mod gpu_profiler;
pub mod graph;
pub mod ibl;
pub mod indirect;
pub mod instancing;
pub mod light;
pub mod lod;
pub mod mesh_bridge;
pub mod occlusion;
pub mod particle;
pub mod particle3d;
pub mod pipeline;
pub mod post_process;
pub mod renderer;
pub mod resource;
pub mod shadow;
pub mod sprite;
pub mod sprite_renderer;
pub mod ssao;
pub mod taa;
pub mod texture_bridge;
pub mod texture_store;
pub mod tilemap;
pub mod view;
