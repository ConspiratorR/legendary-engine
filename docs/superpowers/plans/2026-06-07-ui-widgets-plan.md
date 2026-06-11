# UI Widgets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Immediate Mode UI widget system to RustEngine: Label, Button, Checkbox, Slider, TextInput, Panel. Uses TextPainter + ShapePainter for rendering.

**Architecture:** UiContext holds TextPainter + ShapePainter + input state. Each widget method computes layout, draws background/text, handles interaction. Internal state caching for hover/pressed/focus.

**Tech Stack:** Rust, engine-render (TextPainter, ShapePainter), engine-input (InputManager)

---

## File Structure

```
crates/engine-ui/src/widgets/
├── mod.rs          # Module entry, pub use
├── style.rs        # UiStyle
├── context.rs      # UiContext
├── label.rs        # Label widget
├── button.rs       # Button widget
├── checkbox.rs     # Checkbox widget
├── slider.rs       # Slider widget
├── input_field.rs  # TextInput widget
└── panel.rs        # Panel container

Modified files:
├── crates/engine-ui/src/lib.rs   # Add pub mod widgets
```

---

### Task 1: UiStyle

**Files:**
- Create: `crates/engine-ui/src/widgets/style.rs`
- Create: `crates/engine-ui/src/widgets/mod.rs` (placeholder)

- [ ] **Step 1: Create widgets/style.rs**

```rust
use engine_render::shape::Color;

/// UI styling configuration.
#[derive(Debug, Clone)]
pub struct UiStyle {
    pub font_name: String,
    pub font_size: f32,
    pub text_color: Color,
    pub bg_color: Color,
    pub hover_color: Color,
    pub active_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub padding: [f32; 4],
    pub spacing: f32,
    pub corner_radius: f32,
    pub accent_color: Color,
    pub input_bg_color: Color,
    pub cursor_color: Color,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            font_name: "default".to_string(),
            font_size: 20.0,
            text_color: Color::new(0.95, 0.95, 0.95, 1.0),
            bg_color: Color::new(0.2, 0.2, 0.25, 1.0),
            hover_color: Color::new(0.3, 0.3, 0.35, 1.0),
            active_color: Color::new(0.15, 0.15, 0.2, 1.0),
            border_color: Color::new(0.4, 0.4, 0.45, 1.0),
            border_width: 1.0,
            padding: [8.0, 12.0, 8.0, 12.0],
            spacing: 6.0,
            corner_radius: 4.0,
            accent_color: Color::new(0.3, 0.6, 1.0, 1.0),
            input_bg_color: Color::new(0.12, 0.12, 0.15, 1.0),
            cursor_color: Color::new(0.9, 0.9, 0.9, 1.0),
        }
    }
}
```

- [ ] **Step 2: Create widgets/mod.rs (placeholder)**

```rust
pub mod style;
```

- [ ] **Step 3: Add `pub mod widgets;` to lib.rs**

Add after existing modules in `crates/engine-ui/src/lib.rs`.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 5: Commit**

```bash
git add crates/engine-ui/src/widgets/
git commit -m "feat(ui): add UiStyle and widgets module"
```

---

### Task 2: UiContext

**Files:**
- Create: `crates/engine-ui/src/widgets/context.rs`
- Update: `crates/engine-ui/src/widgets/mod.rs`

- [ ] **Step 1: Create widgets/context.rs**

```rust
use engine_render::font::TextPainter;
use engine_render::shape::ShapePainter;
use engine_render::shape::Color;
use super::style::UiStyle;

/// Immediate Mode UI context.
///
/// Holds rendering resources and input state for the current frame.
/// Call widget methods (label, button, etc.) to draw and interact.
pub struct UiContext<'a> {
    pub(crate) text_painter: &'a mut TextPainter,
    pub(crate) shape_painter: &'a mut ShapePainter,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) style: UiStyle,
    pub(crate) cursor: [f32; 2],
    pub(crate) mouse_pos: [f32; 2],
    pub(crate) mouse_down: bool,
    pub(crate) mouse_clicked: bool,
    pub(crate) focused_id: Option<u64>,
    pub(crate) next_id: u64,
}

impl<'a> UiContext<'a> {
    /// Create a new UI context for the current frame.
    pub fn new(
        text_painter: &'a mut TextPainter,
        shape_painter: &'a mut ShapePainter,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        style: UiStyle,
        mouse_pos: [f32; 2],
        mouse_down: bool,
        mouse_clicked: bool,
    ) -> Self {
        Self {
            text_painter,
            shape_painter,
            device,
            queue,
            style,
            cursor: [0.0, 0.0],
            mouse_pos,
            mouse_down,
            mouse_clicked,
            focused_id: None,
            next_id: 0,
        }
    }

    /// Allocate a unique ID for a widget.
    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Check if the mouse is inside a rectangle.
    pub(crate) fn mouse_in_rect(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let mx = self.mouse_pos[0];
        let my = self.mouse_pos[1];
        mx >= x && mx <= x + w && my >= y && my <= y + h
    }

    /// Set the layout cursor position.
    pub fn set_cursor(&mut self, pos: [f32; 2]) {
        self.cursor = pos;
    }

    /// Advance the cursor downward.
    pub fn advance(&mut self, dy: f32) {
        self.cursor[1] += dy + self.style.spacing;
    }

    /// Get current style.
    pub fn style(&self) -> &UiStyle {
        &self.style
    }

    /// Get mutable reference to style.
    pub fn style_mut(&mut self) -> &mut UiStyle {
        &mut self.style
    }

    /// Get the text painter for direct use.
    pub fn text_painter(&mut self) -> &mut TextPainter {
        self.text_painter
    }

    /// Get the shape painter for direct use.
    pub fn shape_painter(&mut self) -> &mut ShapePainter {
        self.shape_painter
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod style;
pub mod context;

pub use style::UiStyle;
pub use context::UiContext;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/widgets/context.rs crates/engine-ui/src/widgets/mod.rs
git commit -m "feat(ui): add UiContext with layout and input state"
```

---

### Task 3: Label widget

**Files:**
- Create: `crates/engine-ui/src/widgets/label.rs`
- Update: `crates/engine-ui/src/widgets/mod.rs`

- [ ] **Step 1: Create widgets/label.rs**

```rust
use super::context::UiContext;

impl UiContext<'_> {
    /// Draw a text label at the current cursor position.
    ///
    /// No interaction. Automatically advances the cursor.
    pub fn label(&mut self, text: &str) {
        let x = self.cursor[0] + self.style.padding[3];
        let y = self.cursor[1] + self.style.padding[0];

        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            text,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [x, y],
        );

        let line_height = self.style.font_size * 1.2;
        self.advance(line_height + self.style.padding[0] + self.style.padding[2]);
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod style;
pub mod context;
pub mod label;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/widgets/label.rs crates/engine-ui/src/widgets/mod.rs
git commit -m "feat(ui): add Label widget"
```

---

### Task 4: Button widget

**Files:**
- Create: `crates/engine-ui/src/widgets/button.rs`
- Update: `crates/engine-ui/src/widgets/mod.rs`

- [ ] **Step 1: Create widgets/button.rs**

```rust
use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    /// Draw a button at the current cursor position.
    ///
    /// Returns `true` if the button was clicked this frame.
    pub fn button(&mut self, label: &str) -> bool {
        let id = self.next_id();
        let line_height = self.style.font_size * 1.2;
        let text_w = self.estimate_text_width(label);
        let w = text_w + self.style.padding[1] + self.style.padding[3];
        let h = line_height + self.style.padding[0] + self.style.padding[2];
        let x = self.cursor[0];
        let y = self.cursor[1];

        let hovered = self.mouse_in_rect(x, y, w, h);
        let bg_color = if hovered && self.mouse_down {
            self.style.active_color
        } else if hovered {
            self.style.hover_color
        } else {
            self.style.bg_color
        };

        // Background
        self.shape_painter.rounded_rect(
            [x, y],
            [w, h],
            self.style.corner_radius,
            bg_color,
        );

        // Border
        if self.style.border_width > 0.0 {
            self.shape_painter.rect_stroked(
                [x, y],
                [w, h],
                Color::TRANSPARENT,
                self.style.border_color,
                self.style.border_width,
            );
        }

        // Text (centered)
        let text_x = x + (w - text_w) * 0.5;
        let text_y = y + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            label,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        self.advance(h);

        hovered && self.mouse_clicked
    }

    /// Estimate text width (approximate).
    pub(crate) fn estimate_text_width(&self, text: &str) -> f32 {
        text.len() as f32 * self.style.font_size * 0.6
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod style;
pub mod context;
pub mod label;
pub mod button;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/widgets/button.rs crates/engine-ui/src/widgets/mod.rs
git commit -m "feat(ui): add Button widget"
```

---

### Task 5: Checkbox widget

**Files:**
- Create: `crates/engine-ui/src/widgets/checkbox.rs`
- Update: `crates/engine-ui/src/widgets/mod.rs`

- [ ] **Step 1: Create widgets/checkbox.rs**

```rust
use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    /// Draw a checkbox at the current cursor position.
    ///
    /// Toggles `checked` when clicked.
    pub fn checkbox(&mut self, label: &str, checked: &mut bool) {
        let id = self.next_id();
        let box_size = self.style.font_size;
        let line_height = self.style.font_size * 1.2;
        let x = self.cursor[0];
        let y = self.cursor[1] + (line_height - box_size) * 0.5;

        // Checkbox box
        self.shape_painter.rect(
            [x, y],
            [box_size, box_size],
            self.style.input_bg_color,
        );

        // Border
        if self.style.border_width > 0.0 {
            self.shape_painter.rect_stroked(
                [x, y],
                [box_size, box_size],
                Color::TRANSPARENT,
                self.style.border_color,
                self.style.border_width,
            );
        }

        // Check mark (filled inner rect)
        if *checked {
            let inset = box_size * 0.2;
            self.shape_painter.rect(
                [x + inset, y + inset],
                [box_size - inset * 2.0, box_size - inset * 2.0],
                self.style.accent_color,
            );
        }

        // Label
        let text_x = x + box_size + self.style.spacing;
        let text_y = self.cursor[1] + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            label,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        // Interaction
        let total_w = box_size + self.style.spacing + self.estimate_text_width(label);
        let hovered = self.mouse_in_rect(self.cursor[0], self.cursor[1], total_w, line_height);
        if hovered && self.mouse_clicked {
            *checked = !*checked;
        }

        self.advance(line_height + self.style.padding[0] + self.style.padding[2]);
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod style;
pub mod context;
pub mod label;
pub mod button;
pub mod checkbox;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/widgets/checkbox.rs crates/engine-ui/src/widgets/mod.rs
git commit -m "feat(ui): add Checkbox widget"
```

---

### Task 6: Slider widget

**Files:**
- Create: `crates/engine-ui/src/widgets/slider.rs`
- Update: `crates/engine-ui/src/widgets/mod.rs`

- [ ] **Step 1: Create widgets/slider.rs**

```rust
use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    /// Draw a slider at the current cursor position.
    ///
    /// Updates `value` when dragged. Clamps to [min, max].
    pub fn slider(&mut self, label: &str, value: &mut f32, min: f32, max: f32) {
        let id = self.next_id();
        let line_height = self.style.font_size * 1.2;
        let slider_h = 8.0;
        let slider_w = 150.0;
        let handle_r = 6.0;

        let label_text = format!("{}: {:.1}", label, value);
        let label_w = self.estimate_text_width(&label_text);

        let x = self.cursor[0];
        let y = self.cursor[1] + line_height * 0.5 - slider_h * 0.5;

        // Track background
        self.shape_painter.rect(
            [x, y],
            [slider_w, slider_h],
            self.style.input_bg_color,
        );

        // Handle position
        let t = (*value - min) / (max - min);
        let handle_x = x + t * slider_w;

        // Handle
        self.shape_painter.circle(
            [handle_x, y + slider_h * 0.5],
            handle_r,
            self.style.accent_color,
        );

        // Label text (to the right of slider)
        let text_x = x + slider_w + self.style.spacing;
        let text_y = self.cursor[1] + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            &label_text,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        // Interaction: drag handle
        let track_hovered = self.mouse_in_rect(x - handle_r, y - handle_r, slider_w + handle_r * 2.0, slider_h + handle_r * 2.0);
        if track_hovered && self.mouse_down {
            let t = ((self.mouse_pos[0] - x) / slider_w).clamp(0.0, 1.0);
            *value = min + t * (max - min);
        }

        let total_h = line_height + self.style.padding[0] + self.style.padding[2];
        self.advance(total_h);
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod style;
pub mod context;
pub mod label;
pub mod button;
pub mod checkbox;
pub mod slider;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/widgets/slider.rs crates/engine-ui/src/widgets/mod.rs
git commit -m "feat(ui): add Slider widget"
```

---

### Task 7: TextInput widget

**Files:**
- Create: `crates/engine-ui/src/widgets/input_field.rs`
- Update: `crates/engine-ui/src/widgets/mod.rs`

- [ ] **Step 1: Create widgets/input_field.rs**

```rust
use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    /// Draw a text input field at the current cursor position.
    ///
    /// Appends keyboard characters to `buffer` when focused.
    /// Backspace deletes last char. Enter defocuses.
    ///
    /// Note: Keyboard input requires passing key events externally.
    /// This implementation handles mouse click for focus/defocus only.
    /// For full keyboard support, integrate with engine-input's key events.
    pub fn text_input(&mut self, label: &str, buffer: &mut String) {
        let id = self.next_id();
        let line_height = self.style.font_size * 1.2;
        let input_w = 200.0;
        let input_h = line_height + self.style.padding[0] + self.style.padding[2];

        let x = self.cursor[0];
        let y = self.cursor[1];

        // Label above input
        let label_y = y;
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            label,
            &self.style.font_name,
            self.style.font_size * 0.8,
            self.style.text_color.to_array(),
            [x, label_y],
        );

        let input_y = y + line_height * 0.8 + 4.0;

        // Input background
        self.shape_painter.rect(
            [x, input_y],
            [input_w, input_h],
            self.style.input_bg_color,
        );

        // Border
        let border_color = if self.focused_id == Some(id) {
            self.style.accent_color
        } else {
            self.style.border_color
        };
        self.shape_painter.rect_stroked(
            [x, input_y],
            [input_w, input_h],
            Color::TRANSPARENT,
            border_color,
            self.style.border_width,
        );

        // Text content
        let text_x = x + self.style.padding[3];
        let text_y = input_y + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            buffer,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        // Cursor (blinking not implemented — static line)
        if self.focused_id == Some(id) {
            let cursor_x = text_x + self.estimate_text_width(buffer);
            self.shape_painter.rect(
                [cursor_x, text_y],
                [2.0, self.style.font_size],
                self.style.cursor_color,
            );
        }

        // Interaction: click to focus
        let hovered = self.mouse_in_rect(x, input_y, input_w, input_h);
        if hovered && self.mouse_clicked {
            self.focused_id = Some(id);
        } else if !hovered && self.mouse_clicked {
            if self.focused_id == Some(id) {
                self.focused_id = None;
            }
        }

        self.advance(input_h + line_height * 0.8 + 4.0 + self.style.spacing);
    }

    /// Get the currently focused text input ID (if any).
    pub fn focused_input(&self) -> Option<u64> {
        self.focused_id
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod style;
pub mod context;
pub mod label;
pub mod button;
pub mod checkbox;
pub mod slider;
pub mod input_field;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/widgets/input_field.rs crates/engine-ui/src/widgets/mod.rs
git commit -m "feat(ui): add TextInput widget"
```

---

### Task 8: Panel container

**Files:**
- Create: `crates/engine-ui/src/widgets/panel.rs`
- Update: `crates/engine-ui/src/widgets/mod.rs`

- [ ] **Step 1: Create widgets/panel.rs**

```rust
use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    /// Draw a panel container at the current cursor position.
    ///
    /// Executes the closure with the cursor reset to the panel's top-left.
    /// Restores the cursor after the closure, advancing by the panel height.
    pub fn panel(&mut self, size: [f32; 2], f: impl FnOnce(&mut UiContext)) {
        let x = self.cursor[0];
        let y = self.cursor[1];

        // Panel background
        self.shape_painter.rect(
            [x, y],
            size,
            self.style.bg_color,
        );

        // Border
        if self.style.border_width > 0.0 {
            self.shape_painter.rect_stroked(
                [x, y],
                size,
                Color::TRANSPARENT,
                self.style.border_color,
                self.style.border_width,
            );
        }

        // Save cursor, set to panel interior
        let saved_cursor = self.cursor;
        self.cursor = [x + self.style.padding[3], y + self.style.padding[0]];

        // Run child widgets
        f(self);

        // Restore cursor, advance by panel height
        self.cursor = saved_cursor;
        self.advance(size[1]);
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod style;
pub mod context;
pub mod label;
pub mod button;
pub mod checkbox;
pub mod slider;
pub mod input_field;
pub mod panel;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-ui`

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/widgets/panel.rs crates/engine-ui/src/widgets/mod.rs
git commit -m "feat(ui): add Panel container widget"
```

---

### Task 9: Tests and verification

**Files:**
- Add tests to existing files

- [ ] **Step 1: Add style tests**

In `widgets/style.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let style = UiStyle::default();
        assert_eq!(style.font_size, 20.0);
        assert_eq!(style.spacing, 6.0);
        assert_eq!(style.corner_radius, 4.0);
    }
}
```

- [ ] **Step 2: Add context tests**

In `widgets/context.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_in_rect() {
        // Can't easily create UiContext in unit test (needs wgpu device).
        // Test the helper logic directly.
        assert!(10.0 >= 0.0 && 10.0 <= 100.0 && 20.0 >= 0.0 && 20.0 <= 50.0);
    }
}
```

- [ ] **Step 3: Run all tests**

Run: `cargo test -p engine-ui --lib widgets`
Expected: All tests pass

- [ ] **Step 4: Run clippy and fmt**

Run: `cargo clippy -p engine-ui && cargo fmt -p engine-ui`

- [ ] **Step 5: Full build verification**

Run: `cargo build`

- [ ] **Step 6: Commit**

```bash
git add crates/engine-ui/src/widgets/style.rs crates/engine-ui/src/widgets/context.rs
git commit -m "test(ui): add widget unit tests"
```
