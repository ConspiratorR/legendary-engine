//! Rendering pipeline with wgpu.
//!
//! This crate provides:
//! - **Renderer**: wgpu device/queue/surface management
//! - **Render Graph**: declarative pass dependency system
//! - **Sprite Pipeline**: 2D sprite batching and rendering
//! - **PBR Pipeline**: physically-based 3D rendering with lighting
//! - **Camera System**: projection, view matrices, frustum culling
//! - **Shadow Mapping**: cascaded shadow maps (CSM)
//! - **Post-Processing**: tonemapping, bloom, SSAO
//! - **Particles**: 2D and 3D particle systems
//! - **Animation**: sprite sheet and skeletal animation
//! - **Tilemap**: tile-based level rendering

pub mod animation;
pub mod atmosphere;
pub mod bloom;
pub mod camera;
pub mod camera_system;
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
