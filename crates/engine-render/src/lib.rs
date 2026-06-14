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
//! ### Render Graph Lifecycle
//!
//! 1. **Create** — Allocate texture/buffer handles via [`graph::RenderGraph::create_texture`]
//!    and [`graph::RenderGraph::create_buffer`].
//! 2. **Import** — Inject external GPU resources with [`graph::RenderGraph::import_texture`]
//!    or [`graph::RenderGraph::import_buffer`].
//! 3. **Register passes** — Add render passes with [`graph::RenderGraph::add_render_pass`],
//!    specifying color/depth attachments by handle.
//! 4. **Compile** — Call [`graph::RenderGraph::compile`] to allocate transient GPU resources
//!    and resolve attachment indices.
//! 5. **Execute** — Run compiled passes via [`graph::RenderGraph::execute`] in topological
//!    order.
//! 6. **Reset** — Clear passes and non-imported resources with [`graph::RenderGraph::reset`]
//!    for reuse next frame.
//!
//! ### GPU Resource Lifecycle
//!
//! GPU resources follow a create-upload-use pattern:
//!
//! - **Textures**: Loaded via [`texture_bridge::TextureBridge`] which bridges the asset
//!   system's `Handle<Texture>` to the GPU. Textures load asynchronously and are uploaded
//!   on `flush()`. The [`texture_store::TextureStore`] manages GPU texture lifetime and
//!   bind group allocation.
//! - **Buffers**: Created directly via wgpu with descriptors from [`graph::BufferDesc`].
//!   Transient buffers are allocated per-frame in `compile()` and dropped after execution.
//! - **Materials**: Defined as CPU-side structs in [`resource::material`] and uploaded as
//!   uniform buffers each frame.
//!
//! ### System Organization
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`camera`] / [`camera_system`] | Camera types, ECS integration, priority sorting |
//! | [`frustum`] / [`culling`] / [`culling_system`] | View frustum culling, LOD selection |
//! | [`sprite`] / [`sprite_renderer`] | 2D sprite batching and rendering |
//! | [`instancing`] | GPU instancing for meshes sharing materials |
//! | [`deferred`] | G-Buffer creation and deferred shading pass |
//! | [`shadow`] | Cascaded shadow maps |
//! | [`ibl`] | Image-based lighting (diffuse/specular) |
//! | [`post_process`] | Bloom, tonemapping, SSAO, TAA, SSR |
//! | [`particle`] / [`particle3d`] | 2D and 3D particle systems |
//! | [`tilemap`] | Tile-based 2D map rendering |
//! | [`light`] | Point, spot, and directional light management |
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
pub mod font;
pub mod frustum;
pub mod gpu_profiler;
pub mod graph;
pub mod ibl;
pub mod indirect;
pub mod instancing;
pub mod light;
pub mod line3d;
pub mod lod;
pub mod mesh_bridge;
pub mod occlusion;
pub mod particle;
pub mod particle3d;
pub mod pipeline;
pub mod plugin;
pub mod post_process;
pub mod renderer;
pub mod resource;
pub mod shadow;
pub mod shape;
pub mod sprite;
pub mod sprite_renderer;
pub mod ssao;
pub mod taa;
pub mod texture_bridge;
pub mod texture_store;
pub mod tilemap;
pub mod view;
