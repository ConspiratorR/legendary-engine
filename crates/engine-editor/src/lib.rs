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
pub mod hot_reload;
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
pub mod viewport_renderer;

mod plugin;
pub use plugin::EditorPlugin;

/// WASM entry point — initializes the editor for browser runtime.
///
/// Call this from a `#[wasm_bindgen(start)]` function or from JavaScript.
/// Uses `wasm_bindgen_futures::spawn_local` for async renderer initialization.
#[cfg(target_arch = "wasm32")]
pub async fn start_wasm() -> Result<(), Box<dyn std::error::Error>> {
    use engine_render::renderer::Renderer;
    use engine_window::{WindowConfig, create_window};
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::EventLoop;

    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"RustEngine Editor: initializing...".into());

    let event_loop = EventLoop::new()?;
    let window = std::sync::Arc::new(create_window(
        &WindowConfig {
            title: "RustEngine Editor (Web)".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        },
        &event_loop,
    )?);

    let renderer = Renderer::new_async(std::sync::Arc::clone(&window)).await?;
    let scale_factor = window.scale_factor() as f32;
    let egui_state = engine_ui::EguiState::new(&renderer.device, &renderer.config, scale_factor);
    let mut _gui_skin = engine_ui::GuiSkin::default();

    web_sys::console::log_1(&"RustEngine Editor: initialized".into());

    let mut _editor_state = crate::state::EditorState::new();
    let mut last_time = std::time::Instant::now();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::RedrawRequested => {
                    let now = std::time::Instant::now();
                    let _dt = (now - last_time).as_secs_f32();
                    last_time = now;

                    // TODO: render frame with egui + editor UI
                }
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                WindowEvent::Resized(_size) => {
                    // renderer.resize(size.width, size.height);
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;

    Ok(())
}
