//! Immediate-mode UI system built on `egui`.
//!
//! This crate provides two complementary UI paradigms:
//!
//! - **Immediate-mode** via [`Gui`] (skinned widgets drawn each frame) and
//!   [`GuiLayout`] (scoped horizontal/vertical containers).
//! - **Retained-mode** via [`UiTree`] (persistent widget tree with constraint-based
//!   layout, style cascading, and event propagation).
//!
//! Additional subsystems include [`ThemeManager`] for runtime theme switching,
//! an i18n-aware [`text::TextRenderer`], [`animation::Tween`] / gesture support,
//! and [`EguiState`] for wgpu integration.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use engine_ui::{Gui, GuiSkin, GuiLayout, UiTree};
//! use engine_ui::retained::{WidgetKind, LayoutType};
//! use engine_ui::theme::{Theme, ThemeManager};
//!
//! // --- Immediate-mode label + button ---
//! let skin = GuiSkin::default();
//! // Inside an egui::Ui closure:
//! // let mut gui = Gui::new(ui, &skin);
//! // gui.label(rect, "Hello");
//! // if gui.button(rect, "Click me") { /* handle click */ }
//!
//! // --- Layout scopes ---
//! let ctx = egui::Context::default();
//! let mut layout = GuiLayout::new(&ctx, &skin);
//! layout.vertical(|v| {
//!     v.label("Item 1");
//!     v.label("Item 2");
//!     if v.button("OK") { /* ... */ }
//! });
//!
//! // --- Retained-mode widget tree ---
//! let mut tree = UiTree::new();
//! let root = tree.create_widget(WidgetKind::Container);
//! let btn = tree.create_widget(WidgetKind::Button("Play".into()));
//! tree.set_root(root);
//! tree.add_child(root, btn);
//! tree.layout(egui::vec2(800.0, 600.0));
//!
//! // --- Theme switching ---
//! let mut themes = ThemeManager::new();
//! themes.set_active_theme(Theme::Light, 0.3); // 300ms cross-fade
//! ```
//!
//! # Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`gui`] | Immediate-mode skinned widgets |
//! | [`layout`] | Horizontal / vertical layout scopes |
//! | [`retained`] | Retained-mode widget tree |
//! | [`theme`] | Theme management and style cascading |
//! | [`text`] | Text rendering, i18n, rich text |
//! | [`animation`] | Easing, tweening, gesture recognition |
//! | [`skin`] | Skin / style data structures |
//! | [`integration`] | egui ↔ wgpu bridge |
//! | [`plugin`] | Engine plugin for egui |
//! | [`imgui_plugin`] | Lightweight skin-only plugin |
//! | [`error`] | Error types |

pub mod error;
pub use error::UiError;

pub mod animation;
pub mod gui;
pub mod imgui;
pub mod imgui_plugin;
pub mod integration;
pub mod layout;
pub mod plugin;
pub mod retained;
pub mod skin;
pub mod text;
pub mod theme;
pub mod widgets;

pub use gui::Gui;
pub use imgui_plugin::ImGuiPlugin;
pub use integration::EguiState;
pub use layout::GuiLayout;
pub use plugin::EguiPlugin;
pub use retained::UiTree;
pub use skin::GuiSkin;
pub use theme::{Theme, ThemeManager};
