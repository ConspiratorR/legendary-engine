//! Node graph editor — visual scripting with blueprint components, data-flow
//! evaluation, export, and a drag-and-drop renderer.
//!
//! TODO: Migrate from direct egui to IMGUI wrapper (engine_ui::imgui)
//! Unity Reference: https://docs.unity3d.com/ScriptReference/VisualScripting.html

pub mod blueprint;
pub mod blueprint_component;
pub mod evaluator;
pub mod export;
pub mod graph;
pub mod node_panel;
pub mod nodes;
pub mod preview;
pub mod renderer;
pub mod types;

pub use blueprint::{
    BlueprintContext, BlueprintExecutor, BlueprintResult, BlueprintState, VariableStore,
};
pub use blueprint_component::{BlueprintComponent, BlueprintManager};
pub use evaluator::{EvalContext, EvalResult, evaluate, topological_sort};
pub use export::{
    MaterialParams, extract_material_params, extract_pbr_material, generate_wgsl, to_pbr_material,
};
pub use graph::{ConnectError, Node, NodeCategory, NodeGraph, NodeType, Pin};
pub use node_panel::NodePanel;
pub use nodes::{builtin_node_types, create_node};
pub use preview::{MaterialPreview, quick_preview_params};
pub use renderer::{NodeGraphRenderer, NodeGraphState};
pub use types::{Connection, NodeId, NodeValue, PinDirection, PinId, PinType};
