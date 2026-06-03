//! In-engine editor for scene authoring and debugging.
//!
//! Provides viewport, inspector, hierarchy panel, gizmos, resource browser,
//! animation editor, and scene serialization. Wire in with
//! [`EditorPlugin`].

pub mod animation_editor;
pub mod camera;
pub mod commands;
pub mod gizmo;
pub mod hierarchy;
pub mod inspector;
pub mod layout;
pub mod material_editor;
pub mod node_graph;
pub mod performance_overlay;
pub mod performance_profiler;
pub mod resource_browser;
pub mod scene_bridge;
pub mod scene_serializer;
pub mod script_editor;
pub mod shortcuts;
pub mod state;
pub mod terrain_panel;
pub mod viewport;

mod plugin;
pub use plugin::EditorPlugin;
