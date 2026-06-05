//! # engine-editor
//!
//! Visual editor for the RustEngine.
//!
//! Features:
//! - Scene hierarchy panel
//! - Inspector panel
//! - Resource browser
//! - Viewport with gizmo controls
//! - Menu bar and toolbar
//! - Undo/redo system
//! - Scene serialization
//! - Animation editor
//! - Material editor
//! - Node graph editor
//! - Terrain editor
//! - Script editor
//!
//! ## Architecture
//!
//! The editor is built as a plugin for engine-core:
//!
//! ```text
//! EditorPlugin -> EditorState -> Panels (Hierarchy, Inspector, Browser, Viewport)
//! ```
//!
//! Each panel is a separate module that communicates via the editor state.
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_editor::EditorPlugin;
//! ```
//!
//! Wire [`EditorPlugin`] into an [`engine_core::app::AppBuilder`] to get the
//! full editor UI rendered via egui.

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
