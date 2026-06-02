//! Immediate-mode UI system built on `egui`.
//!
//! Provides [`Gui`] for skinned widgets, [`GuiLayout`] for scoped
//! horizontal/vertical layouts, and [`EguiState`] for egui integration
//! with wgpu. Use [`EguiPlugin`] or
//! [`ImGuiPlugin`] to wire into the engine.

pub mod animation;
pub mod gui;
pub mod imgui_plugin;
pub mod integration;
pub mod layout;
pub mod plugin;
pub mod retained;
pub mod skin;
pub mod text;
pub mod theme;

pub use gui::Gui;
pub use imgui_plugin::ImGuiPlugin;
pub use integration::EguiState;
pub use layout::GuiLayout;
pub use plugin::EguiPlugin;
pub use retained::UiTree;
pub use skin::GuiSkin;
pub use theme::{Theme, ThemeManager};
