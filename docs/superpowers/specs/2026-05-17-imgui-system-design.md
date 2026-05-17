# IMGUI System Design — RustEngine

## Overview

Build a Unity-style IMGUI system for the engine editor, using egui as the rendering backend. The system provides both fixed-layout (`GUI`) and auto-layout (`GUILayout`) patterns, a full skinning system, and a complete control set for constructing editor tools.

## Relationship to Existing UI Architecture

- **egui** (Phase 2) — remains as the rendering backend. EguiState, EguiPlugin unchanged. Our IMGUI draws through egui's `Context` (shapes, text, textures) and renders via `egui_wgpu::Renderer`.
- **IMGUI** (this project) — a new API layer in `engine-ui`. Provides Unity-style APIs but renders through egui.
- **UGUI** (future) — runtime game UI system (HUD, menus, inventory). Planned for a subsequent release.

## Architecture

```
engine-ui/src/
├── lib.rs                   ← re-exports
├── integration.rs           ← existing EguiState (unchanged)
├── plugin.rs                ← existing EguiPlugin (unchanged)
├── gui/
│   ├── mod.rs               ← pub use, top-level Gui struct
│   ├── skin.rs              ← GuiSkin, GuiStyle, ColorBlock
│   ├── controls.rs          ← Button, Label, Box, TextField, Toggle, etc.
│   └── window.rs            ← draggable/resizable Window
└── layout/
    ├── mod.rs               ← pub use, top-level GuiLayout struct
    ├── horizontal.rs        ← HorizontalScope
    ├── vertical.rs          ← VerticalScope
    └── scroll.rs            ← ScrollScope
```

## GuiSkin System

Per-control styling with state-based color blocks:

```rust
pub struct GuiSkin {
    pub label: GuiStyle,
    pub button: GuiStyle,
    pub box_: GuiStyle,
    pub text_field: GuiStyle,
    pub toggle: GuiStyle,
    pub window: GuiStyle,
    pub slider: GuiStyle,
    pub toolbar: GuiStyle,
    pub selection_grid: GuiStyle,
    pub font: egui::FontId,
    pub cursor: Option<egui::CursorIcon>,
}

pub struct GuiStyle {
    pub normal: ColorBlock,
    pub hover: ColorBlock,
    pub active: ColorBlock,
    pub focused: ColorBlock,
    pub border: egui::style::Rounding,
    pub margins: egui::Margin,
    pub font_size: f32,
}

pub struct ColorBlock {
    pub background: egui::Color32,
    pub text: egui::Color32,
    pub border: Option<egui::Color32>,
}
```

Default skin provided as `GuiSkin::default()`. Users replace individual styles after construction.

## Fixed Layout API (Gui)

Inspired by `UnityEngine.GUI`. User specifies pixel rectangles:

```rust
pub struct Gui<'a> {
    skin: &'a GuiSkin,
    ctx: &'a egui::Context,
}

impl Gui<'_> {
    pub fn label(&mut self, rect: Rect, text: &str) {}
    pub fn button(&mut self, rect: Rect, text: &str) -> bool {}
    pub fn repeat_button(&mut self, rect: Rect, text: &str) -> bool {}
    pub fn box_(&mut self, rect: Rect, text: &str) {}
    pub fn text_field(&mut self, rect: Rect, text: &mut String) {}
    pub fn text_area(&mut self, rect: Rect, text: &mut String) {}
    pub fn toggle(&mut self, rect: Rect, value: &mut bool, text: &str) {}
    pub fn toolbar(&mut self, rect: Rect, selected: &mut usize, texts: &[&str]) {}
    pub fn selection_grid(&mut self, rect: Rect, selected: &mut usize, texts: &[&str], cols: usize) -> bool {}
    pub fn slider(&mut self, rect: Rect, value: &mut f32, min: f32, max: f32) {}
    pub fn separator(&mut self, rect: Rect) {}
    pub fn draw_texture(&mut self, rect: Rect, texture: &egui::TextureId, scale_mode: ScaleMode) {}
}
```

## Auto Layout API (GuiLayout)

Inspired by `UnityEngine.GUILayout`. Automatic positioning via stack-based layout contexts:

### Primary API (closure-based, preferred)

```rust
pub struct GuiLayout<'a> {
    skin: &'a GuiSkin,
    ctx: &'a egui::Context,
}

impl GuiLayout<'_> {
    pub fn horizontal<R>(&mut self, f: impl FnOnce(&mut HorizontalScope)) {}
    pub fn vertical<R>(&mut self, f: impl FnOnce(&mut VerticalScope)) {}
    pub fn scroll_view<R>(&mut self, scroll: &mut egui::Vec2, f: impl FnOnce(&mut ScrollScope)) {}

    // Top-level controls (outside any layout group)
    pub fn label(&mut self, text: &str) {}
    pub fn button(&mut self, text: &str) -> bool {}
    // ... etc
}
```

### Scope types

```rust
pub struct HorizontalScope<'a> { .. }
pub struct VerticalScope<'a> { .. }
pub struct ScrollScope<'a> { .. }
// Each scope exposes the same control methods as GuiLayout
```

### Secondary API (Begin/End pairs with drop guards)

```rust
pub fn begin_horizontal(&mut self) -> HorizontalScope<'_> {}
// HorizontalScope::drop() calls end_horizontal
pub fn begin_vertical(&mut self) -> VerticalScope<'_> {}
pub fn begin_scroll_view(&mut self, scroll: &mut egui::Vec2) -> ScrollScope<'_> {}
```

## Window

```rust
impl GuiLayout<'_> {
    pub fn window<R>(&mut self, title: &str, rect: &mut egui::Rect, f: impl FnOnce(&mut WindowScope<'_>)) {}
}
```

WindowScope provides:
- Draggable title bar
- Resizable borders
- Clipping to content area
- Close button (returns false to close)

## Event System

egui internally handles event routing (mouse hit-testing, keyboard focus). Our IMGUI `button(rect, ...)` does:

1. Call `ctx.allocate_rect(rect, egui::Sense::click())` for interaction ID
2. Read `response.clicked()` or `response.hovered()` for state
3. Paint background rect using `ctx.painter().rect(...)` with skin colors based on state
4. Paint text centered in rect

No custom event loop needed — egui's frame lifecycle handles everything.

## Controls — Detailed Behavior

| Control | Interaction | Return Value |
|---------|-------------|-------------|
| Label | None | — |
| Button | Click (down+up inside) | `bool` (true on click) |
| RepeatButton | Held down | `bool` (true each repeat frame) |
| Box | None (visual only) | — |
| TextField | Keyboard focus + text input | `&mut String` (modified in place) |
| TextArea | Same, multiline | `&mut String` |
| Toggle | Click to flip | `&mut bool` |
| Toolbar | Click button in row | `&mut usize` |
| SelectionGrid | Click cell in grid | `&mut usize` |
| Slider | Horizontal drag | `&mut f32` |
| Separator | None | — |

## Usage Example

```rust
fn draw_editor(ctx: &egui::Context, skin: &GuiSkin) {
    let mut gui = Gui::new(ctx, skin);

    // Fixed layout toolbar
    gui.toolbar(Rect::new(0.0, 0.0, screen_w, 20.0), &mut tool_sel, &["Scene", "Game"]);

    // Auto layout inspector panel
    let mut layout = GuiLayout::new(ctx, skin);
    layout.window("Inspector", &mut inspector_rect, |win| {
        win.vertical(|v| {
            v.label("Position:");
            v.horizontal(|h| {
                h.label("X:");
                h.text_field(&mut pos.x);
            });
            // ...
        });
    });
}
```

## Testing

Each control gets unit tests for:
- State transitions (hover → click → release)
- Layout calculations (auto-layout width/height distribution)
- Skin color application (normal/hover/active)

Tests use egui's test harness (`egui::Context` in test mode).

## Integration

EguiPlugin unchanged. Users call our IMGUI API inside post-update hooks (same place egui calls go now):

```rust
app.add_post_update_hook(Box::new(|app: &mut App| {
    let egui_state = app.resources.get_mut::<EguiState>().unwrap();
    let skin = app.resources.get::<GuiSkin>().unwrap();
    let mut gui = Gui::new(egui_state.ctx(), skin);
    if gui.button(Rect::new(10.0, 10.0, 100.0, 30.0), "Create") {
        // ...
    }
}));
```

## Future Roadmap

- **UGUI** — runtime game UI system (next major release)
- **Editor Framework** — built on top of this IMGUI system (immediate next step after this crate)
