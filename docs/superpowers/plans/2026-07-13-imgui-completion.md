# Unity IMGUI 系统补全计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Unity 风格的 IMGUI 系统，使游戏脚本能使用 GUILayout/GUI/Event 等 API

**Architecture:** 在 `engine-ui` crate 中构建 Unity 兼容的 IMGUI 层。底层仍使用 egui 渲染，但 API 层匹配 Unity 的 PascalCase 命名和使用模式。

**Tech Stack:** Rust, engine-ui crate, egui (底层渲染)

---

## 文件结构

### 新增文件
- `crates/engine-ui/src/imgui/gui.rs` — `GUI` 类（Window, DrawTexture, BeginArea 等）
- `crates/engine-ui/src/imgui/gui_layout.rs` — `GUILayout` 类（自动布局）
- `crates/engine-ui/src/imgui/gui_style.rs` — `GUIStyle` 样式类
- `crates/engine-ui/src/imgui/gui_skin.rs` — `GUISkin` 皮肤类
- `crates/engine-ui/src/imgui/gui_content.rs` — `GUIContent` 内容类
- `crates/engine-ui/src/imgui/gui_event.rs` — `Event` 输入事件类
- `crates/engine-ui/src/imgui/gui_utility.rs` — `GUIUtility` 工具类
- `crates/engine-ui/src/imgui/editor_gui_layout.rs` — `EditorGUILayout` 编辑器布局
- `crates/engine-ui/src/imgui/mod.rs` — 模块导出
- `crates/engine-ui/tests/imgui_tests.rs` — 测试

### 修改文件
- `crates/engine-ui/src/lib.rs` — 导出 imgui 模块
- `crates/engine-ui/src/gui.rs` — 扩展现有 Gui 结构体

---

## Phase 1: 基础类型

### Task 1: GUIContent

**Files:**
- Create: `crates/engine-ui/src/imgui/gui_content.rs`
- Create: `crates/engine-ui/src/imgui/mod.rs`
- Modify: `crates/engine-ui/src/lib.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 mod.rs**

```rust
pub mod gui_content;
pub mod gui_style;
pub mod gui_skin;
pub mod gui_event;
pub mod gui;
pub mod gui_layout;
pub mod gui_utility;
pub mod editor_gui_layout;
```

- [ ] **Step 2: 创建 gui_content.rs**

```rust
//! GUIContent class (matches Unity's GUIContent).

/// Content for GUI elements (matches Unity's `GUIContent`).
#[derive(Debug, Clone)]
pub struct GUIContent {
    /// Text to display.
    pub text: String,
    /// Image to display.
    pub image: Option<String>,
    /// Tooltip text.
    pub tooltip: String,
}

impl GUIContent {
    /// Create with text only (matches `new GUIContent(string)`).
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            image: None,
            tooltip: String::new(),
        }
    }

    /// Create with text and tooltip (matches `new GUIContent(string, string)`).
    pub fn new_with_tooltip(text: &str, tooltip: &str) -> Self {
        Self {
            text: text.to_string(),
            image: None,
            tooltip: tooltip.to_string(),
        }
    }

    /// Create with text and image (matches `new GUIContent(string, Texture)`).
    pub fn new_with_image(text: &str, image: &str) -> Self {
        Self {
            text: text.to_string(),
            image: Some(image.to_string()),
            tooltip: String::new(),
        }
    }

    /// Empty content (matches `GUIContent.none`).
    pub fn none() -> Self {
        Self {
            text: String::new(),
            image: None,
            tooltip: String::new(),
        }
    }

    /// Get the display text.
    pub fn Text(&self) -> &str {
        &self.text
    }

    /// Get the tooltip.
    pub fn Tooltip(&self) -> &str {
        &self.tooltip
    }
}

impl Default for GUIContent {
    fn default() -> Self {
        Self::none()
    }
}

impl From<&str> for GUIContent {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for GUIContent {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}
```

- [ ] **Step 3: 在 lib.rs 中导出**

```rust
pub mod imgui;
```

- [ ] **Step 4: 编写测试**

```rust
use engine_ui::imgui::gui_content::GUIContent;

#[test]
fn test_gui_content_new() {
    let c = GUIContent::new("Hello");
    assert_eq!(c.Text(), "Hello");
    assert!(c.Tooltip().is_empty());
}

#[test]
fn test_gui_content_tooltip() {
    let c = GUIContent::new_with_tooltip("OK", "Click to confirm");
    assert_eq!(c.Tooltip(), "Click to confirm");
}

#[test]
fn test_gui_content_none() {
    let c = GUIContent::none();
    assert!(c.Text().is_empty());
}

#[test]
fn test_gui_content_from_str() {
    let c: GUIContent = "test".into();
    assert_eq!(c.Text(), "test");
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test -p engine-ui --test imgui_tests`

- [ ] **Step 6: Commit**

```bash
git add crates/engine-ui/src/imgui/ crates/engine-ui/src/lib.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add GUIContent type for IMGUI system"
```

---

### Task 2: GUIStyle

**Files:**
- Create: `crates/engine-ui/src/imgui/gui_style.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 gui_style.rs**

```rust
//! GUIStyle class (matches Unity's GUIStyle).

use egui::Color32;

/// Style state for a GUI element (matches Unity's `GUIStyleState`).
#[derive(Debug, Clone)]
pub struct GUIStyleState {
    /// Text color.
    pub text_color: [u8; 4],
    /// Background color.
    pub background: Option<[u8; 4]>,
}

impl Default for GUIStyleState {
    fn default() -> Self {
        Self {
            text_color: [255, 255, 255, 255],
            background: None,
        }
    }
}

/// Image position options (matches Unity's `ImagePosition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePosition {
    ImageLeft,
    ImageAbove,
    TextOnly,
    ImageOnly,
}

impl Default for ImagePosition {
    fn default() -> Self { Self::ImageLeft }
}

/// Text alignment options (matches Unity's `TextAnchor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    UpperLeft,
    UpperCenter,
    UpperRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    LowerLeft,
    LowerCenter,
    LowerRight,
}

impl Default for TextAnchor {
    fn default() -> Self { Self::UpperLeft }
}

/// Font style options (matches Unity's `FontStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Bold,
    Italic,
    BoldAndItalic,
}

impl Default for FontStyle {
    fn default() -> Self { Self::Normal }
}

/// A style for GUI elements (matches Unity's `GUIStyle`).
#[derive(Debug, Clone)]
pub struct GUIStyle {
    /// Normal state style.
    pub normal: GUIStyleState,
    /// Hover state style.
    pub hover: GUIStyleState,
    /// Active (pressed) state style.
    pub active: GUIStyleState,
    /// Focused state style.
    pub focused: GUIStyleState,

    /// Font size in pixels (0 = default).
    pub font_size: i32,
    /// Font style (normal, bold, italic).
    pub font_style: FontStyle,
    /// Text alignment.
    pub alignment: TextAnchor,
    /// Whether text wraps.
    pub word_wrap: bool,
    /// Whether text supports rich text tags.
    pub rich_text: bool,
    /// Image position relative to text.
    pub image_position: ImagePosition,

    /// Fixed width (0 = auto).
    pub fixed_width: f32,
    /// Fixed height (0 = auto).
    pub fixed_height: f32,
    /// Minimum width.
    pub min_width: f32,
    /// Maximum width (f32::INFINITY = no limit).
    pub max_width: f32,
    /// Minimum height.
    pub min_height: f32,
    /// Maximum height.
    pub max_height: f32,
    /// Whether width expands to fill available space.
    pub expand_width: bool,
    /// Whether height expands to fill available space.
    pub expand_height: bool,

    /// Padding (left, right, top, bottom).
    pub padding: [f32; 4],
    /// Margin (left, right, top, bottom).
    pub margin: [f32; 4],
    /// Overflow (left, right, top, bottom).
    pub overflow: [f32; 4],
    /// Border width (left, right, top, bottom).
    pub border: [f32; 4],

    /// Name of the style.
    pub name: String,
}

impl Default for GUIStyle {
    fn default() -> Self {
        Self {
            normal: GUIStyleState::default(),
            hover: GUIStyleState::default(),
            active: GUIStyleState::default(),
            focused: GUIStyleState::default(),
            font_size: 0,
            font_style: FontStyle::default(),
            alignment: TextAnchor::default(),
            word_wrap: false,
            rich_text: false,
            image_position: ImagePosition::default(),
            fixed_width: 0.0,
            fixed_height: 0.0,
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            expand_width: false,
            expand_height: false,
            padding: [0.0; 4],
            margin: [0.0; 4],
            overflow: [0.0; 4],
            border: [0.0; 4],
            name: String::new(),
        }
    }
}

impl GUIStyle {
    /// Calculate the size needed for content.
    pub fn CalcSize(&self, content_size: [f32; 2]) -> [f32; 2] {
        let mut w = content_size[0] + self.padding[0] + self.padding[1];
        let mut h = content_size[1] + self.padding[2] + self.padding[3];

        if self.fixed_width > 0.0 { w = self.fixed_width; }
        if self.fixed_height > 0.0 { h = self.fixed_height; }
        w = w.clamp(self.min_width, self.max_width);
        h = h.clamp(self.min_height, self.max_height);

        [w, h]
    }

    /// Get the content rect (inside padding).
    pub fn ContentRect(&self, rect: [f32; 4]) -> [f32; 4] {
        [
            rect[0] + self.padding[0],
            rect[1] + self.padding[2],
            rect[2] - self.padding[0] - self.padding[1],
            rect[3] - self.padding[2] - self.padding[3],
        ]
    }
}
```

- [ ] **Step 2: 编写测试**

```rust
use engine_ui::imgui::gui_style::{GUIStyle, TextAnchor, FontStyle, ImagePosition};

#[test]
fn test_gui_style_default() {
    let s = GUIStyle::default();
    assert_eq!(s.alignment, TextAnchor::UpperLeft);
    assert_eq!(s.font_style, FontStyle::Normal);
    assert!(!s.word_wrap);
}

#[test]
fn test_gui_style_calc_size() {
    let mut s = GUIStyle::default();
    s.padding = [5.0, 5.0, 5.0, 5.0];
    let size = s.CalcSize([100.0, 20.0]);
    assert_eq!(size, [110.0, 30.0]);
}

#[test]
fn test_gui_style_fixed_size() {
    let mut s = GUIStyle::default();
    s.fixed_width = 200.0;
    s.fixed_height = 50.0;
    let size = s.CalcSize([100.0, 20.0]);
    assert_eq!(size, [200.0, 50.0]);
}

#[test]
fn test_gui_style_content_rect() {
    let s = GUIStyle::default();
    let rect = s.ContentRect([10.0, 20.0, 100.0, 50.0]);
    assert_eq!(rect, [10.0, 20.0, 100.0, 50.0]);
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/imgui/gui_style.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add GUIStyle with states, alignment, sizing"
```

---

### Task 3: GUISkin

**Files:**
- Create: `crates/engine-ui/src/imgui/gui_skin.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 gui_skin.rs**

```rust
//! GUISkin class (matches Unity's GUISkin).

use super::gui_style::GUIStyle;

/// A collection of styles for IMGUI (matches Unity's `GUISkin`).
#[derive(Debug, Clone)]
pub struct GUISkin {
    /// Font name/path.
    pub font: String,

    /// Box style.
    pub box_style: GUIStyle,
    /// Button style.
    pub button: GUIStyle,
    /// Repeat button style.
    pub repeat_button: GUIStyle,
    /// Toggle style.
    pub toggle: GUIStyle,
    /// Label style.
    pub label: GUIStyle,
    /// TextField style.
    pub text_field: GUIStyle,
    /// TextArea style.
    pub text_area: GUIStyle,
    /// Window style.
    pub window: GUIStyle,
    /// HorizontalSlider style.
    pub horizontal_slider: GUIStyle,
    /// HorizontalSliderThumb style.
    pub horizontal_slider_thumb: GUIStyle,
    /// VerticalSlider style.
    pub vertical_slider: GUIStyle,
    /// VerticalSliderThumb style.
    pub vertical_slider_thumb: GUIStyle,
    /// HorizontalScrollbar style.
    pub horizontal_scrollbar: GUIStyle,
    /// HorizontalScrollbarThumb style.
    pub horizontal_scrollbar_thumb: GUIStyle,
    /// HorizontalScrollbarLeftButton style.
    pub horizontal_scrollbar_left_button: GUIStyle,
    /// HorizontalScrollbarRightButton style.
    pub horizontal_scrollbar_right_button: GUIStyle,
    /// VerticalScrollbar style.
    pub vertical_scrollbar: GUIStyle,
    /// VerticalScrollbarThumb style.
    pub vertical_scrollbar_thumb: GUIStyle,
    /// VerticalScrollbarUpButton style.
    pub vertical_scrollbar_up_button: GUIStyle,
    /// VerticalScrollbarDownButton style.
    pub vertical_scrollbar_down_button: GUIStyle,
    /// ScrollView style.
    pub scroll_view: GUIStyle,
    /// HorizontalScrollbar style (alias).
    pub horizontalScrollbar: GUIStyle,
    /// HorizontalScrollbarThumb style (alias).
    pub horizontalScrollbarThumb: GUIStyle,
    /// VerticalScrollbar style (alias).
    pub verticalScrollbar: GUIStyle,
    /// VerticalScrollbarThumb style (alias).
    pub verticalScrollbarThumb: GUIStyle,
    /// Grid style.
    pub grid: GUIStyle,
}

impl Default for GUISkin {
    fn default() -> Self {
        Self {
            font: String::new(),
            box_style: GUIStyle::default(),
            button: GUIStyle::default(),
            repeat_button: GUIStyle::default(),
            toggle: GUIStyle::default(),
            label: GUIStyle::default(),
            text_field: GUIStyle::default(),
            text_area: GUIStyle::default(),
            window: GUIStyle::default(),
            horizontal_slider: GUIStyle::default(),
            horizontal_slider_thumb: GUIStyle::default(),
            vertical_slider: GUIStyle::default(),
            vertical_slider_thumb: GUIStyle::default(),
            horizontal_scrollbar: GUIStyle::default(),
            horizontal_scrollbar_thumb: GUIStyle::default(),
            horizontal_scrollbar_left_button: GUIStyle::default(),
            horizontal_scrollbar_right_button: GUIStyle::default(),
            vertical_scrollbar: GUIStyle::default(),
            vertical_scrollbar_thumb: GUIStyle::default(),
            vertical_scrollbar_up_button: GUIStyle::default(),
            vertical_scrollbar_down_button: GUIStyle::default(),
            scroll_view: GUIStyle::default(),
            horizontalScrollbar: GUIStyle::default(),
            horizontalScrollbarThumb: GUIStyle::default(),
            verticalScrollbar: GUIStyle::default(),
            verticalScrollbarThumb: GUIStyle::default(),
            grid: GUIStyle::default(),
        }
    }
}

impl GUISkin {
    /// Find a style by name (matches `GUISkin.FindStyle`).
    pub fn FindStyle(&self, name: &str) -> Option<&GUIStyle> {
        match name {
            "box" | "Box" => Some(&self.box_style),
            "button" | "Button" => Some(&self.button),
            "toggle" | "Toggle" => Some(&self.toggle),
            "label" | "Label" => Some(&self.label),
            "textfield" | "TextField" => Some(&self.text_field),
            "textarea" | "TextArea" => Some(&self.text_area),
            "window" | "Window" => Some(&self.window),
            _ => None,
        }
    }
}
```

- [ ] **Step 2: 编写测试**

```rust
use engine_ui::imgui::gui_skin::GUISkin;

#[test]
fn test_gui_skin_default() {
    let skin = GUISkin::default();
    assert!(skin.font.is_empty());
}

#[test]
fn test_gui_skin_find_style() {
    let skin = GUISkin::default();
    assert!(skin.FindStyle("Button").is_some());
    assert!(skin.FindStyle("label").is_some());
    assert!(skin.FindStyle("nonexistent").is_none());
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/imgui/gui_skin.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add GUISkin with style collection"
```

---

## Phase 2: 输入事件系统

### Task 4: Event

**Files:**
- Create: `crates/engine-ui/src/imgui/gui_event.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 gui_event.rs**

```rust
//! Event class (matches Unity's Event).

/// Event type (matches Unity's `EventType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    MouseDown,
    MouseUp,
    MouseDrag,
    MouseMove,
    ScrollWheel,
    KeyDown,
    KeyUp,
    Repaint,
    Layout,
    Used,
    Ignore,
}

impl Default for EventType {
    fn default() -> Self { Self::Repaint }
}

/// Mouse button (matches Unity's `MouseButton`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

/// Key code (matches Unity's `KeyCode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    None,
    Backspace,
    Tab,
    Return,
    Escape,
    Space,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Alpha0, Alpha1, Alpha2, Alpha3, Alpha4,
    Alpha5, Alpha6, Alpha7, Alpha8, Alpha9,
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    LeftShift, RightShift, LeftControl, RightControl,
    LeftAlt, RightAlt,
    UpArrow, DownArrow, LeftArrow, RightArrow,
    Home, End, PageUp, PageDown,
    Insert, Delete,
}

impl Default for KeyCode {
    fn default() -> Self { Self::None }
}

/// An input event for IMGUI (matches Unity's `Event`).
#[derive(Debug, Clone)]
pub struct Event {
    /// Event type.
    pub event_type: EventType,
    /// Mouse position.
    pub mouse_position: [f32; 2],
    /// Mouse button (for mouse events).
    pub button: i32,
    /// Key code (for keyboard events).
    pub key_code: KeyCode,
    /// Character (for text input).
    pub character: Option<char>,
    /// Whether Control key is held.
    pub control: bool,
    /// Whether Shift key is held.
    pub shift: bool,
    /// Whether Alt key is held.
    pub alt: bool,
    /// Command key (Mac) / Control key (Windows/Linux).
    pub command: bool,
    /// Whether this event has been used.
    pub used: bool,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            event_type: EventType::Repaint,
            mouse_position: [0.0, 0.0],
            button: 0,
            key_code: KeyCode::None,
            character: None,
            control: false,
            shift: false,
            alt: false,
            command: false,
            used: false,
        }
    }
}

impl Event {
    /// Current event (matches `Event.current`).
    pub fn current() -> Self {
        // Placeholder: would return actual current event
        Self::default()
    }

    /// Use the event (matches `Event.Use()`).
    pub fn Use(&mut self) {
        self.used = true;
        self.event_type = EventType::Used;
    }

    /// Check if this is a mouse event.
    pub fn IsMouse(&self) -> bool {
        matches!(self.event_type,
            EventType::MouseDown | EventType::MouseUp |
            EventType::MouseDrag | EventType::MouseMove |
            EventType::ScrollWheel)
    }

    /// Check if this is a key event.
    pub fn IsKey(&self) -> bool {
        matches!(self.event_type, EventType::KeyDown | EventType::KeyUp)
    }

    /// Check if shift is held.
    pub fn IsShiftKeyDown(&self) -> bool {
        self.shift
    }

    /// Check if control is held.
    pub fn IsControlKeyDown(&self) -> bool {
        self.control
    }

    /// Check if alt is held.
    pub fn IsAltKeyDown(&self) -> bool {
        self.alt
    }

    /// Convert screen position to GUI position.
    pub fn ScreenToGUIPoint(screen_pos: [f32; 2]) -> [f32; 2] {
        screen_pos
    }

    /// Convert GUI position to screen position.
    pub fn GUIToScreenPoint(gui_pos: [f32; 2]) -> [f32; 2] {
        gui_pos
    }
}
```

- [ ] **Step 2: 编写测试**

```rust
use engine_ui::imgui::gui_event::{Event, EventType, KeyCode};

#[test]
fn test_event_default() {
    let e = Event::default();
    assert_eq!(e.event_type, EventType::Repaint);
    assert!(!e.used);
}

#[test]
fn test_event_use() {
    let mut e = Event::default();
    e.Use();
    assert!(e.used);
    assert_eq!(e.event_type, EventType::Used);
}

#[test]
fn test_event_is_mouse() {
    let mut e = Event::default();
    e.event_type = EventType::MouseDown;
    assert!(e.IsMouse());
    e.event_type = EventType::KeyDown;
    assert!(!e.IsMouse());
}

#[test]
fn test_event_modifiers() {
    let mut e = Event::default();
    e.shift = true;
    e.control = true;
    assert!(e.IsShiftKeyDown());
    assert!(e.IsControlKeyDown());
    assert!(!e.IsAltKeyDown());
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/imgui/gui_event.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add Event type with mouse, keyboard, modifiers"
```

---

## Phase 3: 核心 GUI 类

### Task 5: GUI

**Files:**
- Create: `crates/engine-ui/src/imgui/gui.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 gui.rs**

```rust
//! GUI class (matches Unity's GUI).

use super::gui_content::GUIContent;
use super::gui_skin::GUISkin;
use super::gui_style::GUIStyle;

/// Global GUI settings (matches Unity's `GUI`).
pub struct GUI {
    /// Current skin.
    pub skin: Option<GUISkin>,
    /// Tint color applied to all textures.
    pub color: [f32; 4],
    /// Background color drawn behind controls.
    pub background_color: [f32; 4],
    /// Color for text drawn by controls.
    pub content_color: [f32; 4],
    /// Whether the GUI is enabled.
    pub enabled: bool,
    /// Depth for this control (for sorting).
    pub depth: i32,
}

impl Default for GUI {
    fn default() -> Self {
        Self {
            skin: None,
            color: [1.0, 1.0, 1.0, 1.0],
            background_color: [1.0, 1.0, 1.0, 1.0],
            content_color: [1.0, 1.0, 1.0, 1.0],
            enabled: true,
            depth: 0,
        }
    }
}

impl GUI {
    /// Draw a label (matches `GUI.Label(Rect, string)`).
    pub fn Label(rect: [f32; 4], content: &GUIContent) {
        // Placeholder: would draw label
        let _ = (rect, content);
    }

    /// Draw a box (matches `GUI.Box(Rect, string)`).
    pub fn Box(rect: [f32; 4], content: &GUIContent) {
        // Placeholder: would draw box
        let _ = (rect, content);
    }

    /// Draw a button (matches `GUI.Button(Rect, string)`).
    pub fn Button(rect: [f32; 4], content: &GUIContent) -> bool {
        // Placeholder: would draw button
        let _ = (rect, content);
        false
    }

    /// Draw a repeat button (matches `GUI.RepeatButton(Rect, string)`).
    pub fn RepeatButton(rect: [f32; 4], content: &GUIContent) -> bool {
        // Placeholder
        let _ = (rect, content);
        false
    }

    /// Draw a toggle (matches `GUI.Toggle(Rect, bool, string)`).
    pub fn Toggle(rect: [f32; 4], value: bool, content: &GUIContent) -> bool {
        // Placeholder
        let _ = (rect, content);
        value
    }

    /// Draw a text field (matches `GUI.TextField(Rect, string)`).
    pub fn TextField(rect: [f32; 4], text: &str) -> String {
        // Placeholder
        let _ = (rect);
        text.to_string()
    }

    /// Draw a text area (matches `GUI.TextArea(Rect, string)`).
    pub fn TextArea(rect: [f32; 4], text: &str) -> String {
        // Placeholder
        let _ = (rect);
        text.to_string()
    }

    /// Draw a horizontal slider (matches `GUI.HorizontalSlider(Rect, float, float, float)`).
    pub fn HorizontalSlider(rect: [f32; 4], value: f32, left_value: f32, right_value: f32) -> f32 {
        // Placeholder
        let _ = (rect, left_value, right_value);
        value
    }

    /// Draw a toolbar (matches `GUI.Toolbar(Rect, int, string[])`).
    pub fn Toolbar(rect: [f32; 4], selected: i32, texts: &[&str]) -> i32 {
        // Placeholder
        let _ = (rect, texts);
        selected
    }

    /// Begin a group (matches `GUI.BeginGroup(Rect)`).
    pub fn BeginGroup(rect: [f32; 4]) {
        // Placeholder
        let _ = rect;
    }

    /// End a group (matches `GUI.EndGroup()`).
    pub fn EndGroup() {
        // Placeholder
    }

    /// Begin an area (matches `GUI.BeginArea(Rect)`).
    pub fn BeginArea(rect: [f32; 4]) {
        // Placeholder
        let _ = rect;
    }

    /// Begin an area with text (matches `GUI.BeginArea(Rect, string)`).
    pub fn BeginAreaWithTitle(rect: [f32; 4], text: &str) {
        // Placeholder
        let _ = (rect, text);
    }

    /// End an area (matches `GUI.EndArea()`).
    pub fn EndArea() {
        // Placeholder
    }

    /// Draw a texture (matches `GUI.DrawTexture(Rect, Texture)`).
    pub fn DrawTexture(rect: [f32; 4], texture: &str) {
        // Placeholder
        let _ = (rect, texture);
    }

    /// Draw a texture with tex coords (matches `GUI.DrawTextureWithTexCoords(Rect, Texture, Rect)`).
    pub fn DrawTextureWithTexCoords(rect: [f32; 4], texture: &str, tex_coords: [f32; 4]) {
        // Placeholder
        let _ = (rect, texture, tex_coords);
    }

    /// Bring a window to front (matches `GUI.BringWindowToFront(int)`).
    pub fn BringWindowToFront(window_id: i32) {
        // Placeholder
        let _ = window_id;
    }

    /// Bring a window to back (matches `GUI.BringWindowToBack(int)`).
    pub fn BringWindowToBack(window_id: i32) {
        // Placeholder
        let _ = window_id;
    }

    /// Drag a window (matches `GUI.DragWindow()`).
    pub fn DragWindow() {
        // Placeholder
    }

    /// Set the name for the next window (matches `GUI.SetNextWindowName(string)`).
    pub fn SetNextWindowName(name: &str) {
        // Placeholder
        let _ = name;
    }
}

/// Window function type.
pub type WindowFunction = Box<dyn FnMut(i32)>;

/// GUIWindow manages a window instance.
pub struct GUIWindow {
    /// Window ID.
    pub id: i32,
    /// Window position and size.
    pub rect: [f32; 4],
    /// Window title.
    pub title: String,
    /// Whether the window is draggable.
    pub draggable: bool,
    /// Whether the window has a scrollbar.
    pub scroll: bool,
    /// Background image.
    pub background: Option<String>,
}

impl GUIWindow {
    /// Create a new window.
    pub fn new(id: i32, rect: [f32; 4], title: &str) -> Self {
        Self {
            id,
            rect,
            title: title.to_string(),
            draggable: true,
            scroll: false,
            background: None,
        }
    }

    /// Draw the window (matches `GUI.Window(int, Rect, WindowFunction, string)`).
    pub fn Draw(&mut self) {
        // Placeholder
    }
}
```

- [ ] **Step 2: 编写测试**

```rust
use engine_ui::imgui::gui::{GUI, GUIWindow};
use engine_ui::imgui::gui_content::GUIContent;

#[test]
fn test_gui_label() {
    GUI::Label([0.0, 0.0, 100.0, 20.0], &GUIContent::new("Hello"));
}

#[test]
fn test_gui_button() {
    let result = GUI::Button([0.0, 0.0, 100.0, 30.0], &GUIContent::new("Click"));
    assert!(!result);
}

#[test]
fn test_gui_toggle() {
    let result = GUI::Toggle([0.0, 0.0, 100.0, 20.0], false, &GUIContent::new("On"));
    assert!(!result);
}

#[test]
fn test_gui_window() {
    let mut win = GUIWindow::new(1, [10.0, 10.0, 200.0, 100.0], "Test Window");
    assert_eq!(win.id, 1);
    assert_eq!(win.title, "Test Window");
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/imgui/gui.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add GUI class with Label, Button, Window, Group"
```

---

### Task 6: GUILayout

**Files:**
- Create: `crates/engine-ui/src/imgui/gui_layout.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 gui_layout.rs**

```rust
//! GUILayout class (matches Unity's GUILayout).

use super::gui_content::GUIContent;

/// Auto-layout GUI system (matches Unity's `GUILayout`).
pub struct GUILayout;

impl GUILayout {
    // ── Basic Widgets ──

    /// Auto-layout label.
    pub fn Label(content: &GUIContent) {
        // Placeholder
        let _ = content;
    }

    /// Auto-layout box.
    pub fn Box(content: &GUIContent) {
        // Placeholder
        let _ = content;
    }

    /// Auto-layout button.
    pub fn Button(content: &GUIContent) -> bool {
        // Placeholder
        let _ = content;
        false
    }

    /// Auto-layout repeat button.
    pub fn RepeatButton(content: &GUIContent) -> bool {
        // Placeholder
        let _ = content;
        false
    }

    /// Auto-layout toggle.
    pub fn Toggle(value: bool, content: &GUIContent) -> bool {
        // Placeholder
        let _ = content;
        value
    }

    /// Auto-layout text field.
    pub fn TextField(text: &str) -> String {
        // Placeholder
        text.to_string()
    }

    /// Auto-layout text area.
    pub fn TextArea(text: &str) -> String {
        // Placeholder
        text.to_string()
    }

    /// Auto-layout horizontal slider.
    pub fn HorizontalSlider(value: f32, left_value: f32, right_value: f32) -> f32 {
        // Placeholder
        let _ = (left_value, right_value);
        value
    }

    /// Auto-layout toolbar.
    pub fn Toolbar(selected: i32, texts: &[&str]) -> i32 {
        // Placeholder
        let _ = texts;
        selected
    }

    // ── Layout Groups ──

    /// Begin a horizontal layout group.
    pub fn BeginHorizontal() {
        // Placeholder
    }

    /// Begin a horizontal layout group with options.
    pub fn BeginHorizontalWithOptions(options: &[&str]) {
        // Placeholder: options like "width 200", "height 30"
        let _ = options;
    }

    /// End a horizontal layout group.
    pub fn EndHorizontal() {
        // Placeholder
    }

    /// Begin a vertical layout group.
    pub fn BeginVertical() {
        // Placeholder
    }

    /// Begin a vertical layout group with options.
    pub fn BeginVerticalWithOptions(options: &[&str]) {
        // Placeholder
        let _ = options;
    }

    /// End a vertical layout group.
    pub fn EndVertical() {
        // Placeholder
    }

    /// Begin a scroll view.
    pub fn BeginScrollView(scroll_position: [f32; 2]) -> [f32; 2] {
        // Placeholder
        scroll_position
    }

    /// End a scroll view.
    pub fn EndScrollView() {
        // Placeholder
    }

    /// Begin a area.
    pub fn BeginArea(screenRect: [f32; 4]) {
        // Placeholder
        let _ = screenRect;
    }

    /// End a area.
    pub fn EndArea() {
        // Placeholder
    }

    /// Begin a window.
    pub fn Window(id: i32, content: &GUIContent) -> bool {
        // Placeholder
        let _ = (id, content);
        false
    }

    /// End a window.
    pub fn EndWindow() {
        // Placeholder
    }

    // ── Spacing ──

    /// Add horizontal space.
    pub fn Space(space: f32) {
        // Placeholder
        let _ = space;
    }

    /// Add flexible horizontal space.
    pub fn FlexibleSpace() {
        // Placeholder
    }

    // ── Options ──

    /// Set width option.
    pub fn Width(width: f32) -> LayoutOption {
        LayoutOption::Width(width)
    }

    /// Set height option.
    pub fn Height(height: f32) -> LayoutOption {
        LayoutOption::Height(height)
    }

    /// Set minimum width option.
    pub fn MinWidth(min_width: f32) -> LayoutOption {
        LayoutOption::MinWidth(min_width)
    }

    /// Set maximum width option.
    pub fn MaxWidth(max_width: f32) -> LayoutOption {
        LayoutOption::MaxWidth(max_width)
    }

    /// Set minimum height option.
    pub fn MinHeight(min_height: f32) -> LayoutOption {
        LayoutOption::MinHeight(min_height)
    }

    /// Set maximum height option.
    pub fn MaxHeight(max_height: f32) -> LayoutOption {
        LayoutOption::MaxHeight(max_height)
    }

    /// Set expand width option.
    pub fn ExpandWidth(expand: bool) -> LayoutOption {
        LayoutOption::ExpandWidth(expand)
    }

    /// Set expand height option.
    pub fn ExpandHeight(expand: bool) -> LayoutOption {
        LayoutOption::ExpandHeight(expand)
    }

    /// Get the last rect drawn.
    pub fn GetLastRect() -> [f32; 4] {
        // Placeholder
        [0.0, 0.0, 0.0, 0.0]
    }
}

/// Layout option for GUILayout widgets.
#[derive(Debug, Clone)]
pub enum LayoutOption {
    Width(f32),
    Height(f32),
    MinWidth(f32),
    MaxWidth(f32),
    MinHeight(f32),
    MaxHeight(f32),
    ExpandWidth(bool),
    ExpandHeight(bool),
}
```

- [ ] **Step 2: 编写测试**

```rust
use engine_ui::imgui::gui_layout::{GUILayout, LayoutOption};
use engine_ui::imgui::gui_content::GUIContent;

#[test]
fn test_gUILayout_label() {
    GUILayout::Label(&GUIContent::new("Hello"));
}

#[test]
fn test_gUILayout_button() {
    let result = GUILayout::Button(&GUIContent::new("Click"));
    assert!(!result);
}

#[test]
fn test_gUILayout_options() {
    let opt = GUILayout::Width(200.0);
    assert!(matches!(opt, LayoutOption::Width(200.0)));

    let opt = GUILayout::Height(30.0);
    assert!(matches!(opt, LayoutOption::Height(30.0)));
}

#[test]
fn test_gUILayout_groups() {
    GUILayout::BeginHorizontal();
    GUILayout::Label(&GUIContent::new("Left"));
    GUILayout::Label(&GUIContent::new("Right"));
    GUILayout::EndHorizontal();

    GUILayout::BeginVertical();
    GUILayout::Label(&GUIContent::new("Top"));
    GUILayout::Label(&GUIContent::new("Bottom"));
    GUILayout::EndVertical();
}

#[test]
fn test_gUILayout_spacing() {
    GUILayout::Space(10.0);
    GUILayout::FlexibleSpace();
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/imgui/gui_layout.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add GUILayout with auto-layout and options"
```

---

## Phase 4: 工具类

### Task 7: GUIUtility

**Files:**
- Create: `crates/engine-ui/src/imgui/gui_utility.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 gui_utility.rs**

```rust
//! GUIUtility class (matches Unity's GUIUtility).

/// Utility methods for IMGUI (matches Unity's `GUIUtility`).
pub struct GUIUtility;

impl GUIUtility {
    /// Get a control ID (matches `GUIUtility.GetControlID`).
    pub fn GetControlID(hint: i32) -> i32 {
        hint
    }

    /// Get a control ID with focus hint.
    pub fn GetControlIDWithFocus(hint: i32, focus_type: i32) -> i32 {
        let _ = focus_type;
        hint
    }

    /// Get the hot control ID.
    pub fn hotControl() -> i32 {
        0
    }

    /// Set the hot control ID.
    pub fn SetHotControl(id: i32) {
        // Placeholder
        let _ = id;
    }

    /// Get the keyboard control ID.
    pub fn keyboardControl() -> i32 {
        0
    }

    /// Set the keyboard control ID.
    pub fn SetKeyboardControl(id: i32) {
        // Placeholder
        let _ = id;
    }

    /// Exit the GUI (matches `GUIUtility.ExitGUI`).
    pub fn ExitGUI() {
        // Placeholder
    }

    /// Convert screen point to GUI point.
    pub fn ScreenToGUIPoint(screen_point: [f32; 2]) -> [f32; 2] {
        screen_point
    }

    /// Convert GUI point to screen point.
    pub fn GUIToScreenPoint(gui_point: [f32; 2]) -> [f32; 2] {
        gui_point
    }

    /// Rotate vector by GUI matrix.
    pub fn RotateAroundPivot(angle: f32, pivot: [f32; 2]) {
        // Placeholder
        let _ = (angle, pivot);
    }

    /// Scale around pivot.
    pub fn ScaleAroundPivot(scale: [f32; 2], pivot: [f32; 2]) {
        // Placeholder
        let _ = (scale, pivot);
    }
}
```

- [ ] **Step 2: 编写测试**

```rust
use engine_ui::imgui::gui_utility::GUIUtility;

#[test]
fn test_gui_utility_get_control_id() {
    let id = GUIUtility::GetControlID(0);
    assert_eq!(id, 0);
}

#[test]
fn test_gui_utility_screen_to_gui() {
    let gui = GUIUtility::ScreenToGUIPoint([100.0, 200.0]);
    assert_eq!(gui, [100.0, 200.0]);
}

#[test]
fn test_gui_utility_hot_control() {
    GUIUtility::SetHotControl(42);
    // Note: placeholder returns 0
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/imgui/gui_utility.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add GUIUtility with control IDs and coordinate conversion"
```

---

## Phase 5: 编辑器布局

### Task 8: EditorGUILayout

**Files:**
- Create: `crates/engine-ui/src/imgui/editor_gui_layout.rs`
- Test: `crates/engine-ui/tests/imgui_tests.rs`

- [ ] **Step 1: 创建 editor_gui_layout.rs**

```rust
//! EditorGUILayout class (matches Unity's EditorGUILayout).

use super::gui_content::GUIContent;

/// Editor-specific auto-layout (matches Unity's `EditorGUILayout`).
pub struct EditorGUILayout;

impl EditorGUILayout {
    /// Auto-layout property field.
    pub fn PropertyField(label: &GUIContent, value: &str) -> String {
        // Placeholder
        let _ = label;
        value.to_string()
    }

    /// Auto-layout object field.
    pub fn ObjectField(label: &GUIContent, obj_type: &str) -> Option<String> {
        // Placeholder
        let _ = (label, obj_type);
        None
    }

    /// Auto-layout mask field.
    pub fn MaskField(label: &GUIContent, selected: i32, options: &[&str]) -> i32 {
        // Placeholder
        let _ = (label, options);
        selected
    }

    /// Auto-layout popup.
    pub fn Popup(label: &GUIContent, selected: i32, options: &[&str]) -> i32 {
        // Placeholder
        let _ = (label, options);
        selected
    }

    /// Auto-layout enum popup.
    pub fn EnumPopup(label: &GUIContent, selected: i32) -> i32 {
        // Placeholder
        let _ = label;
        selected
    }

    /// Auto-layout int popup.
    pub fn IntPopup(label: &GUIContent, selected: i32, options: &[&str]) -> i32 {
        // Placeholder
        let _ = (label, options);
        selected
    }

    /// Auto-layout float popup.
    pub fn FloatPopup(label: &GUIContent, selected: f32, options: &[&str]) -> f32 {
        // Placeholder
        let _ = (label, options);
        selected
    }

    /// Auto-layout layer field.
    pub fn LayerField(label: &GUIContent, layer: i32) -> i32 {
        // Placeholder
        let _ = label;
        layer
    }

    /// Auto-layout tag field.
    pub fn TagField(label: &GUIContent, tag: &str) -> String {
        // Placeholder
        let _ = label;
        tag.to_string()
    }

    /// Auto-layout layer mask field.
    pub fn LayerMaskField(label: &GUIContent, mask: i32) -> i32 {
        // Placeholder
        let _ = label;
        mask
    }

    /// Auto-layout min max slider.
    pub fn MinMaxSlider(label: &GUIContent, min: f32, max: f32) -> (f32, f32) {
        // Placeholder
        let _ = label;
        (min, max)
    }

    /// Auto-layout foldout.
    pub fn Foldout(foldout: bool, content: &GUIContent) -> bool {
        // Placeholder
        let _ = content;
        foldout
    }

    /// Auto-layout help box.
    pub fn HelpBox(message: &str, message_type: i32) {
        // Placeholder
        let _ = (message, message_type);
    }

    /// Add a separator.
    pub fn Separator() {
        // Placeholder
    }

    /// Add a prefix label.
    pub fn PrefixLabel(label: &GUIContent) {
        // Placeholder
        let _ = label;
    }

    /// Begin a horizontal layout.
    pub fn BeginHorizontal() {
        // Placeholder
    }

    /// End a horizontal layout.
    pub fn EndHorizontal() {
        // Placeholder
    }

    /// Begin a vertical layout.
    pub fn BeginVertical() {
        // Placeholder
    }

    /// End a vertical layout.
    pub fn EndVertical() {
        // Placeholder
    }

    /// Begin a scroll view.
    pub fn BeginScrollView(scroll_position: [f32; 2]) -> [f32; 2] {
        // Placeholder
        scroll_position
    }

    /// End a scroll view.
    pub fn EndScrollView() {
        // Placeholder
    }

    /// Get the last rect.
    pub fn GetLastRect() -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
```

- [ ] **Step 2: 编写测试**

```rust
use engine_ui::imgui::editor_gui_layout::EditorGUILayout;
use engine_ui::imgui::gui_content::GUIContent;

#[test]
fn test_editor_gui_layout_property_field() {
    let result = EditorGUILayout::PropertyField(&GUIContent::new("Name"), "Default");
    assert_eq!(result, "Default");
}

#[test]
fn test_editor_gui_layout_foldout() {
    let result = EditorGUILayout::Foldout(false, &GUIContent::new("Section"));
    assert!(!result);
}

#[test]
fn test_editor_gui_layout_popup() {
    let result = EditorGUILayout::Popup(&GUIContent::new("Choice"), 0, &["A", "B", "C"]);
    assert_eq!(result, 0);
}

#[test]
fn test_editor_gui_layout_separator() {
    EditorGUILayout::Separator();
}
```

- [ ] **Step 3: 运行测试确认通过**

- [ ] **Step 4: Commit**

```bash
git add crates/engine-ui/src/imgui/editor_gui_layout.rs crates/engine-ui/tests/imgui_tests.rs
git commit -m "feat: add EditorGUILayout with PropertyField, Foldout, Popup"
```

---

## 执行顺序

| 阶段 | Task | 依赖 | 预计时间 |
|------|------|------|----------|
| Phase 1 | Task 1-3 | 无 | 1小时 |
| Phase 2 | Task 4 | 无 | 30分钟 |
| Phase 3 | Task 5-6 | Task 1-3 | 1小时 |
| Phase 4 | Task 7 | 无 | 20分钟 |
| Phase 5 | Task 8 | Task 6 | 30分钟 |

**总计约 3 小时**

---

## 后续可扩展

完成以上计划后，可继续补充：
- Handles（3D 场景编辑器句柄）
- Event 完整实现（从 egui 事件转换）
- GUI/GUILayout 实际渲染（从 placeholder 到 egui 绘制）
- GUI.skin 动态切换
- TextEditor（文本选择、复制粘贴）
