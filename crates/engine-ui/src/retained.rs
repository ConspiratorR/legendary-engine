//! Retained-mode UI system with a widget tree, constraint-based layout,
//! style cascading, and event propagation.
//!
//! This module complements the immediate-mode [`crate::gui`] system by
//! providing a persistent widget tree that survives across frames.

use std::collections::HashMap;

use egui::{Pos2, Rect, Vec2};

use crate::skin::{ColorBlock, GuiSkin, GuiStyle};

// ---------------------------------------------------------------------------
// IDs and types
// ---------------------------------------------------------------------------

/// Unique identifier for a widget in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

/// The kind of widget — determines default sizing and behaviour.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    /// A generic container (no intrinsic content).
    Container,
    /// A text label.
    Label(String),
    /// A clickable button.
    Button(String),
    /// A text input field.
    TextField { text: String, placeholder: String },
    /// A scrollable list view.
    ListView {
        items: Vec<String>,
        selected: Option<usize>,
    },
    /// A tree view node.
    TreeNode { label: String, expanded: bool },
    /// A tab bar with selectable tabs.
    TabBar { tabs: Vec<String>, active: usize },
    /// A horizontal slider.
    Slider { value: f32, min: f32, max: f32 },
    /// A checkbox toggle.
    Checkbox { checked: bool, label: String },
}

// ---------------------------------------------------------------------------
// Layout types
// ---------------------------------------------------------------------------

/// Layout direction for arranging children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutType {
    /// Stack children horizontally.
    Horizontal,
    /// Stack children vertically.
    #[default]
    Vertical,
    /// Overlay children on top of each other (z-order).
    Stack,
    /// Arrange children in a grid with a fixed column count.
    Grid { columns: u32 },
}

/// Constraint-based sizing and spacing for a widget.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutConstraints {
    /// Minimum width in points.
    pub min_width: Option<f32>,
    /// Maximum width in points.
    pub max_width: Option<f32>,
    /// Minimum height in points.
    pub min_height: Option<f32>,
    /// Maximum height in points.
    pub max_height: Option<f32>,
    /// Stretch factor for distributing extra space among siblings.
    /// 0.0 means no stretch.
    pub stretch_x: f32,
    /// Vertical stretch factor.
    pub stretch_y: f32,
    /// Inner padding (space between border and content).
    pub padding: Padding,
    /// Outer margin (space between this widget and its siblings).
    pub margin: Padding,
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            stretch_x: 0.0,
            stretch_y: 0.0,
            padding: Padding::all(0.0),
            margin: Padding::all(0.0),
        }
    }
}

/// Padding / margin values for each side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Padding {
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };

    pub fn all(v: f32) -> Self {
        Self {
            left: v,
            right: v,
            top: v,
            bottom: v,
        }
    }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

// ---------------------------------------------------------------------------
// Event system
// ---------------------------------------------------------------------------

/// UI events that propagate through the widget tree.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Mouse moved over the widget.
    Hover { pos: Pos2 },
    /// Mouse click (press + release inside widget).
    Click { pos: Pos2 },
    /// Mouse button pressed inside widget.
    MouseDown { pos: Pos2 },
    /// Mouse button released.
    MouseUp { pos: Pos2 },
    /// Keyboard key pressed while widget has focus.
    KeyPress { key: String },
    /// Text input while widget has focus.
    TextInput { text: String },
    /// Widget received focus.
    FocusGained,
    /// Widget lost focus.
    FocusLost,
}

/// The result of handling an event.
#[derive(Debug, Clone, Default)]
pub struct EventResponse {
    /// Whether the event was consumed (stops propagation).
    pub consumed: bool,
    /// Whether the widget needs a re-layout.
    pub dirty: bool,
}

// ---------------------------------------------------------------------------
// Widget node
// ---------------------------------------------------------------------------

/// A single node in the retained-mode widget tree.
#[derive(Debug, Clone)]
pub struct Widget {
    /// Unique id.
    pub id: WidgetId,
    /// Parent widget id (`None` for root).
    pub parent: Option<WidgetId>,
    /// Child widget ids in order.
    pub children: Vec<WidgetId>,
    /// What kind of widget this is.
    pub kind: WidgetKind,
    /// Layout algorithm for children.
    pub layout: LayoutType,
    /// Sizing and spacing constraints.
    pub constraints: LayoutConstraints,
    /// Per-widget style overrides (merged with theme defaults).
    pub style_override: Option<GuiStyle>,
    /// Computed bounds after layout (set by the layout pass).
    pub bounds: Rect,
    /// Whether this widget is visible.
    pub visible: bool,
    /// Z-order within parent's children (higher = drawn on top).
    pub z_order: i32,
    /// Whether this widget needs re-layout.
    pub dirty: bool,
    /// Whether this widget needs re-draw.
    pub needs_draw: bool,
}

impl Widget {
    fn new(id: WidgetId, kind: WidgetKind) -> Self {
        Self {
            id,
            parent: None,
            children: Vec::new(),
            kind,
            layout: LayoutType::default(),
            constraints: LayoutConstraints::default(),
            style_override: None,
            bounds: Rect::from_min_size(Pos2::ZERO, Vec2::ZERO),
            visible: true,
            z_order: 0,
            dirty: true,
            needs_draw: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Widget tree
// ---------------------------------------------------------------------------

/// The main container for the retained-mode UI.
///
/// Holds all widgets in a flat map keyed by [`WidgetId`], with a designated
/// root widget. Provides methods for building, laying out, and querying the
/// tree.
pub struct UiTree {
    widgets: HashMap<WidgetId, Widget>,
    root: Option<WidgetId>,
    next_id: u64,
    focused: Option<WidgetId>,
}

impl UiTree {
    /// Create an empty tree with no root.
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            root: None,
            next_id: 1,
            focused: None,
        }
    }

    /// Allocate a fresh [`WidgetId`].
    fn alloc_id(&mut self) -> WidgetId {
        let id = WidgetId(self.next_id);
        self.next_id += 1;
        id
    }

    // -- construction -------------------------------------------------------

    /// Create a new widget and return its id.  Does **not** add it to any parent.
    pub fn create_widget(&mut self, kind: WidgetKind) -> WidgetId {
        let id = self.alloc_id();
        self.widgets.insert(id, Widget::new(id, kind));
        id
    }

    /// Add a child widget to a parent.  If the child already has a parent it
    /// is removed from the old parent first.
    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        // Remove from old parent.
        if let Some(old_parent) = self.widgets.get(&child).and_then(|c| c.parent)
            && let Some(old) = self.widgets.get_mut(&old_parent)
        {
            old.children.retain(|&id| id != child);
        }
        // Set new parent.
        if let Some(child_w) = self.widgets.get_mut(&child) {
            child_w.parent = Some(parent);
        }
        if let Some(parent_w) = self.widgets.get_mut(&parent) {
            parent_w.children.push(child);
        }
    }

    /// Remove a widget and all its descendants from the tree.
    pub fn remove_widget(&mut self, id: WidgetId) {
        // Collect descendants recursively.
        let descendants = self.collect_descendants(id);
        for desc in descendants {
            self.widgets.remove(&desc);
        }
        // Remove from parent's children list.
        if let Some(parent_id) = self.widgets.get(&id).and_then(|w| w.parent)
            && let Some(parent) = self.widgets.get_mut(&parent_id)
        {
            parent.children.retain(|&cid| cid != id);
        }
        self.widgets.remove(&id);
        if self.root == Some(id) {
            self.root = None;
        }
    }

    fn collect_descendants(&self, id: WidgetId) -> Vec<WidgetId> {
        let mut result = Vec::new();
        if let Some(w) = self.widgets.get(&id) {
            for &child in &w.children {
                result.push(child);
                result.extend(self.collect_descendants(child));
            }
        }
        result
    }

    /// Set the root widget.
    pub fn set_root(&mut self, id: WidgetId) {
        self.root = Some(id);
    }

    /// Get the root widget id.
    pub fn root(&self) -> Option<WidgetId> {
        self.root
    }

    // -- accessors ----------------------------------------------------------

    /// Borrow a widget by id.
    pub fn get(&self, id: WidgetId) -> Option<&Widget> {
        self.widgets.get(&id)
    }

    /// Mutably borrow a widget by id.
    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut Widget> {
        self.widgets.get_mut(&id)
    }

    /// Total number of widgets in the tree.
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// The currently focused widget, if any.
    pub fn focused(&self) -> Option<WidgetId> {
        self.focused
    }

    /// Set focus to a widget (or `None` to clear focus).
    pub fn set_focus(&mut self, id: Option<WidgetId>) {
        if self.focused != id {
            self.focused = id;
        }
    }

    // -- style resolution ---------------------------------------------------

    /// Resolve the effective style for a widget, merging its per-widget
    /// override with the given theme skin defaults.
    pub fn resolve_style(&self, id: WidgetId, skin: &GuiSkin) -> GuiStyle {
        let w = match self.widgets.get(&id) {
            Some(w) => w,
            None => return GuiStyle::default(),
        };
        let base = match &w.kind {
            WidgetKind::Label(_) => skin.label.clone(),
            WidgetKind::Button(_) => skin.button.clone(),
            WidgetKind::TextField { .. } => skin.text_field.clone(),
            WidgetKind::Slider { .. } => skin.slider.clone(),
            WidgetKind::Checkbox { .. } => skin.toggle.clone(),
            WidgetKind::TabBar { .. } => skin.toolbar.clone(),
            _ => GuiStyle::default(),
        };
        if let Some(ref override_) = w.style_override {
            merge_style(&base, override_)
        } else {
            base
        }
    }

    // -- layout -------------------------------------------------------------

    /// Run a full layout pass starting from the root, given the available
    /// screen size.
    pub fn layout(&mut self, available_size: Vec2) {
        let root = match self.root {
            Some(r) => r,
            None => return,
        };
        let root_margin = self
            .widgets
            .get(&root)
            .map(|w| w.constraints.margin)
            .unwrap_or(Padding::ZERO);
        let inner = Rect::from_min_size(
            Pos2::new(root_margin.left, root_margin.top),
            Vec2::new(
                (available_size.x - root_margin.horizontal()).max(0.0),
                (available_size.y - root_margin.vertical()).max(0.0),
            ),
        );
        self.layout_widget(root, inner);
    }

    fn layout_widget(&mut self, id: WidgetId, available: Rect) {
        let (layout_type, padding, children) = match self.widgets.get(&id) {
            Some(w) => (w.layout, w.constraints.padding, w.children.clone()),
            None => return,
        };

        let content_rect = Rect::from_min_size(
            Pos2::new(
                available.min.x + padding.left,
                available.min.y + padding.top,
            ),
            Vec2::new(
                (available.width() - padding.horizontal()).max(0.0),
                (available.height() - padding.vertical()).max(0.0),
            ),
        );

        if let Some(w) = self.widgets.get_mut(&id) {
            w.bounds = available;
            w.dirty = false;
        }

        match layout_type {
            LayoutType::Vertical => self.layout_vertical(id, content_rect, &children),
            LayoutType::Horizontal => self.layout_horizontal(id, content_rect, &children),
            LayoutType::Stack => self.layout_stack(id, content_rect, &children),
            LayoutType::Grid { columns } => self.layout_grid(id, content_rect, &children, columns),
        }
    }

    fn layout_vertical(&mut self, _parent: WidgetId, rect: Rect, children: &[WidgetId]) {
        let mut y = rect.min.y;
        let total_stretch: f32 = children
            .iter()
            .filter_map(|id| self.widgets.get(id))
            .filter(|w| w.visible)
            .map(|w| w.constraints.stretch_y)
            .sum();

        let fixed_height: f32 = children
            .iter()
            .filter_map(|id| self.widgets.get(id))
            .filter(|w| w.visible && w.constraints.stretch_y == 0.0)
            .map(|w| {
                let m = w.constraints.margin;
                w.constraints.min_height.unwrap_or(24.0) + m.vertical()
            })
            .sum();

        let extra = (rect.height() - fixed_height).max(0.0);

        for &child_id in children {
            let (visible, margin, stretch, min_h) = match self.widgets.get(&child_id) {
                Some(w) => (
                    w.visible,
                    w.constraints.margin,
                    w.constraints.stretch_y,
                    w.constraints.min_height.unwrap_or(24.0),
                ),
                None => continue,
            };
            if !visible {
                continue;
            }

            let h = if stretch > 0.0 && total_stretch > 0.0 {
                extra * stretch / total_stretch
            } else {
                min_h
            };

            let child_rect = Rect::from_min_size(
                Pos2::new(rect.min.x + margin.left, y + margin.top),
                Vec2::new(
                    (rect.width() - margin.horizontal()).max(0.0),
                    (h - margin.vertical()).max(0.0),
                ),
            );
            self.layout_widget(child_id, child_rect);
            y += h;
        }
    }

    fn layout_horizontal(&mut self, _parent: WidgetId, rect: Rect, children: &[WidgetId]) {
        let mut x = rect.min.x;
        let total_stretch: f32 = children
            .iter()
            .filter_map(|id| self.widgets.get(id))
            .filter(|w| w.visible)
            .map(|w| w.constraints.stretch_x)
            .sum();

        let fixed_width: f32 = children
            .iter()
            .filter_map(|id| self.widgets.get(id))
            .filter(|w| w.visible && w.constraints.stretch_x == 0.0)
            .map(|w| {
                let m = w.constraints.margin;
                w.constraints.min_width.unwrap_or(80.0) + m.horizontal()
            })
            .sum();

        let extra = (rect.width() - fixed_width).max(0.0);

        for &child_id in children {
            let (visible, margin, stretch, min_w) = match self.widgets.get(&child_id) {
                Some(w) => (
                    w.visible,
                    w.constraints.margin,
                    w.constraints.stretch_x,
                    w.constraints.min_width.unwrap_or(80.0),
                ),
                None => continue,
            };
            if !visible {
                continue;
            }

            let w = if stretch > 0.0 && total_stretch > 0.0 {
                extra * stretch / total_stretch
            } else {
                min_w
            };

            let child_rect = Rect::from_min_size(
                Pos2::new(x + margin.left, rect.min.y + margin.top),
                Vec2::new(
                    (w - margin.horizontal()).max(0.0),
                    (rect.height() - margin.vertical()).max(0.0),
                ),
            );
            self.layout_widget(child_id, child_rect);
            x += w;
        }
    }

    fn layout_stack(&mut self, _parent: WidgetId, rect: Rect, children: &[WidgetId]) {
        for &child_id in children {
            if let Some(w) = self.widgets.get(&child_id)
                && !w.visible
            {
                continue;
            }
            self.layout_widget(child_id, rect);
        }
    }

    fn layout_grid(&mut self, _parent: WidgetId, rect: Rect, children: &[WidgetId], columns: u32) {
        let cols = columns.max(1) as usize;
        let visible_children: Vec<WidgetId> = children
            .iter()
            .filter(|id| self.widgets.get(*id).is_some_and(|w| w.visible))
            .copied()
            .collect();

        let rows = visible_children.len().div_ceil(cols);
        if rows == 0 {
            return;
        }

        let cell_w = rect.width() / cols as f32;
        let cell_h = rect.height() / rows as f32;

        for (i, &child_id) in visible_children.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let child_rect = Rect::from_min_size(
                Pos2::new(
                    rect.min.x + col as f32 * cell_w,
                    rect.min.y + row as f32 * cell_h,
                ),
                Vec2::new(cell_w, cell_h),
            );
            self.layout_widget(child_id, child_rect);
        }
    }

    // -- hit testing --------------------------------------------------------

    /// Find the deepest (most-specific) widget that contains the given point.
    /// Only visible widgets are considered. Returns `None` if no widget
    /// contains the point.
    pub fn hit_test(&self, pos: Pos2) -> Option<WidgetId> {
        self.root.and_then(|r| self.hit_test_recursive(r, pos))
    }

    fn hit_test_recursive(&self, id: WidgetId, pos: Pos2) -> Option<WidgetId> {
        let w = self.widgets.get(&id)?;
        if !w.visible || !w.bounds.contains(pos) {
            return None;
        }
        // Check children in reverse z-order (highest z = checked first).
        let mut sorted_children: Vec<WidgetId> = w
            .children
            .iter()
            .filter(|cid| self.widgets.contains_key(*cid))
            .copied()
            .collect();
        sorted_children.sort_by_key(|cid| -self.widgets.get(cid).map(|c| c.z_order).unwrap_or(0));

        for child_id in sorted_children {
            if let Some(hit) = self.hit_test_recursive(child_id, pos) {
                return Some(hit);
            }
        }
        Some(id)
    }

    // -- event propagation --------------------------------------------------

    /// Dispatch an event to the target widget, bubbling up through ancestors.
    /// Returns the final [`EventResponse`].
    pub fn dispatch_event(&mut self, target: WidgetId, event: &UiEvent) -> EventResponse {
        let mut current = Some(target);
        let mut final_response = EventResponse::default();

        while let Some(id) = current {
            let response = self.handle_event(id, event);
            if response.dirty
                && let Some(w) = self.widgets.get_mut(&id)
            {
                w.dirty = true;
                w.needs_draw = true;
            }
            if response.consumed {
                return response;
            }
            final_response.dirty |= response.dirty;
            current = self.widgets.get(&id).and_then(|w| w.parent);
        }
        final_response
    }

    fn handle_event(&mut self, id: WidgetId, event: &UiEvent) -> EventResponse {
        let w = match self.widgets.get(&id) {
            Some(w) => w,
            None => return EventResponse::default(),
        };

        match event {
            UiEvent::Click { pos } => {
                if w.bounds.contains(*pos) {
                    match &w.kind {
                        WidgetKind::Button(_) => EventResponse {
                            consumed: true,
                            dirty: true,
                        },
                        WidgetKind::Checkbox { .. } => EventResponse {
                            consumed: true,
                            dirty: true,
                        },
                        _ => EventResponse {
                            consumed: false,
                            dirty: false,
                        },
                    }
                } else {
                    EventResponse::default()
                }
            }
            UiEvent::Hover { pos } => {
                if w.bounds.contains(*pos) {
                    EventResponse {
                        consumed: false,
                        dirty: false,
                    }
                } else {
                    EventResponse::default()
                }
            }
            UiEvent::FocusGained | UiEvent::FocusLost => EventResponse {
                consumed: false,
                dirty: true,
            },
            _ => EventResponse::default(),
        }
    }

    // -- dirty flags --------------------------------------------------------

    /// Mark a widget and all its ancestors as dirty (needs re-layout).
    pub fn mark_dirty(&mut self, id: WidgetId) {
        let mut current = Some(id);
        while let Some(cid) = current {
            if let Some(w) = self.widgets.get_mut(&cid) {
                w.dirty = true;
                w.needs_draw = true;
                current = w.parent;
            } else {
                break;
            }
        }
    }

    /// Check if any widget in the tree is dirty.
    pub fn has_dirty(&self) -> bool {
        self.widgets.values().any(|w| w.dirty)
    }

    /// Clear all dirty flags.
    pub fn clear_dirty(&mut self) {
        for w in self.widgets.values_mut() {
            w.dirty = false;
            w.needs_draw = false;
        }
    }
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Style merge helper
// ---------------------------------------------------------------------------

fn merge_style(base: &GuiStyle, override_: &GuiStyle) -> GuiStyle {
    let defaults = GuiStyle::default();
    GuiStyle {
        normal: merge_color_block(&base.normal, &override_.normal, &defaults.normal),
        hover: merge_color_block(&base.hover, &override_.hover, &defaults.hover),
        active: merge_color_block(&base.active, &override_.active, &defaults.active),
        focused: merge_color_block(&base.focused, &override_.focused, &defaults.focused),
        border: if override_.border != defaults.border {
            override_.border
        } else {
            base.border
        },
        margins: if override_.margins != defaults.margins {
            override_.margins
        } else {
            base.margins
        },
        font_size: if (override_.font_size - defaults.font_size).abs() > f32::EPSILON {
            override_.font_size
        } else {
            base.font_size
        },
    }
}

fn merge_color_block(
    base: &ColorBlock,
    override_: &ColorBlock,
    defaults: &ColorBlock,
) -> ColorBlock {
    ColorBlock {
        background: if override_.background != defaults.background {
            override_.background
        } else {
            base.background
        },
        text: if override_.text != defaults.text {
            override_.text
        } else {
            base.text
        },
        border: if override_.border != defaults.border {
            override_.border
        } else {
            base.border
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_tree() -> UiTree {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let child1 = tree.create_widget(WidgetKind::Label("Hello".into()));
        let child2 = tree.create_widget(WidgetKind::Button("Click".into()));
        tree.set_root(root);
        tree.add_child(root, child1);
        tree.add_child(root, child2);
        tree
    }

    #[test]
    fn test_create_and_access_widget() {
        let mut tree = UiTree::new();
        let id = tree.create_widget(WidgetKind::Label("test".into()));
        let w = tree.get(id).unwrap();
        assert_eq!(w.kind, WidgetKind::Label("test".into()));
        assert!(w.visible);
    }

    #[test]
    fn test_parent_child_relationship() {
        let tree = make_simple_tree();
        let root = tree.root().unwrap();
        let root_w = tree.get(root).unwrap();
        assert_eq!(root_w.children.len(), 2);

        let child1 = root_w.children[0];
        let child1_w = tree.get(child1).unwrap();
        assert_eq!(child1_w.parent, Some(root));
    }

    #[test]
    fn test_remove_widget() {
        let mut tree = make_simple_tree();
        let root = tree.root().unwrap();
        let child1 = tree.get(root).unwrap().children[0];
        tree.remove_widget(child1);
        assert!(tree.get(child1).is_none());
        assert_eq!(tree.get(root).unwrap().children.len(), 1);
    }

    #[test]
    fn test_reparent_widget() {
        let mut tree = UiTree::new();
        let a = tree.create_widget(WidgetKind::Container);
        let b = tree.create_widget(WidgetKind::Container);
        let child = tree.create_widget(WidgetKind::Label("x".into()));
        tree.set_root(a);
        tree.add_child(a, child);
        assert_eq!(tree.get(a).unwrap().children.len(), 1);

        tree.add_child(b, child);
        assert_eq!(tree.get(a).unwrap().children.len(), 0);
        assert_eq!(tree.get(b).unwrap().children.len(), 1);
        assert_eq!(tree.get(child).unwrap().parent, Some(b));
    }

    #[test]
    fn test_layout_vertical() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let c1 = tree.create_widget(WidgetKind::Label("A".into()));
        let c2 = tree.create_widget(WidgetKind::Label("B".into()));
        tree.set_root(root);
        tree.add_child(root, c1);
        tree.add_child(root, c2);
        tree.layout(Vec2::new(400.0, 300.0));

        let b1 = tree.get(c1).unwrap().bounds;
        let b2 = tree.get(c2).unwrap().bounds;
        assert!(b1.min.y < b2.min.y, "second child should be below first");
    }

    #[test]
    fn test_layout_horizontal() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        tree.get_mut(root).unwrap().layout = LayoutType::Horizontal;
        let c1 = tree.create_widget(WidgetKind::Label("A".into()));
        let c2 = tree.create_widget(WidgetKind::Label("B".into()));
        tree.set_root(root);
        tree.add_child(root, c1);
        tree.add_child(root, c2);
        tree.layout(Vec2::new(400.0, 300.0));

        let b1 = tree.get(c1).unwrap().bounds;
        let b2 = tree.get(c2).unwrap().bounds;
        assert!(
            b1.min.x < b2.min.x,
            "second child should be to the right of first"
        );
    }

    #[test]
    fn test_layout_grid() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        tree.get_mut(root).unwrap().layout = LayoutType::Grid { columns: 2 };
        let ids: Vec<WidgetId> = (0..4)
            .map(|_| tree.create_widget(WidgetKind::Label("cell".into())))
            .collect();
        tree.set_root(root);
        for &id in &ids {
            tree.add_child(root, id);
        }
        tree.layout(Vec2::new(200.0, 200.0));

        // Grid should produce 2x2 layout.
        let b0 = tree.get(ids[0]).unwrap().bounds;
        let b1 = tree.get(ids[1]).unwrap().bounds;
        let b2 = tree.get(ids[2]).unwrap().bounds;
        assert!(b0.min.x < b1.min.x, "col 0 before col 1");
        assert!(b0.min.y < b2.min.y, "row 0 before row 1");
    }

    #[test]
    fn test_hit_test_finds_deepest() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let child = tree.create_widget(WidgetKind::Button("OK".into()));
        tree.set_root(root);
        tree.add_child(root, child);
        tree.layout(Vec2::new(400.0, 300.0));

        let child_bounds = tree.get(child).unwrap().bounds;
        let hit = tree.hit_test(child_bounds.center());
        assert_eq!(hit, Some(child));
    }

    #[test]
    fn test_hit_test_outside_returns_none() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        tree.set_root(root);
        tree.layout(Vec2::new(400.0, 300.0));

        let hit = tree.hit_test(Pos2::new(500.0, 500.0));
        assert!(hit.is_none());
    }

    #[test]
    fn test_style_resolution_with_override() {
        let mut tree = UiTree::new();
        let id = tree.create_widget(WidgetKind::Button("X".into()));
        tree.get_mut(id).unwrap().style_override = Some(GuiStyle {
            font_size: 24.0,
            ..GuiStyle::default()
        });
        let skin = GuiSkin::default();
        let resolved = tree.resolve_style(id, &skin);
        assert!((resolved.font_size - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_style_resolution_without_override() {
        let mut tree = UiTree::new();
        let id = tree.create_widget(WidgetKind::Button("X".into()));
        let skin = GuiSkin::default();
        let resolved = tree.resolve_style(id, &skin);
        // Should match the skin's button style.
        assert_eq!(resolved.font_size, skin.button.font_size);
    }

    #[test]
    fn test_dirty_flag_propagation() {
        let mut tree = make_simple_tree();
        let root = tree.root().unwrap();
        let child = tree.get(root).unwrap().children[0];
        tree.clear_dirty();
        assert!(!tree.has_dirty());

        tree.mark_dirty(child);
        assert!(tree.has_dirty());
        let root_w = tree.get(root).unwrap();
        assert!(root_w.dirty, "dirty should bubble to parent");
    }

    #[test]
    fn test_event_dispatch_click_on_button() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let btn = tree.create_widget(WidgetKind::Button("OK".into()));
        tree.set_root(root);
        tree.add_child(root, btn);
        tree.layout(Vec2::new(400.0, 300.0));

        let btn_bounds = tree.get(btn).unwrap().bounds;
        let response = tree.dispatch_event(
            btn,
            &UiEvent::Click {
                pos: btn_bounds.center(),
            },
        );
        assert!(response.consumed, "button click should consume event");
        assert!(response.dirty, "button click should mark dirty");
    }

    #[test]
    fn test_visibility_hides_widget() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let child = tree.create_widget(WidgetKind::Label("hidden".into()));
        tree.set_root(root);
        tree.add_child(root, child);
        tree.get_mut(child).unwrap().visible = false;
        tree.layout(Vec2::new(400.0, 300.0));

        let hit = tree.hit_test(Pos2::new(200.0, 10.0));
        // The hidden child should not be hit.
        assert_ne!(hit, Some(child));
    }

    #[test]
    fn test_z_order_affects_hit_test() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        tree.get_mut(root).unwrap().layout = LayoutType::Stack;
        let bottom = tree.create_widget(WidgetKind::Button("bottom".into()));
        let top = tree.create_widget(WidgetKind::Button("top".into()));
        tree.get_mut(bottom).unwrap().z_order = 0;
        tree.get_mut(top).unwrap().z_order = 1;
        tree.set_root(root);
        tree.add_child(root, bottom);
        tree.add_child(root, top);
        tree.layout(Vec2::new(400.0, 300.0));

        let hit = tree.hit_test(Pos2::new(200.0, 150.0));
        assert_eq!(hit, Some(top), "higher z-order should win");
    }

    #[test]
    fn test_stretch_distribution() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let c1 = tree.create_widget(WidgetKind::Label("A".into()));
        let c2 = tree.create_widget(WidgetKind::Label("B".into()));
        tree.get_mut(c1).unwrap().constraints.stretch_y = 1.0;
        tree.get_mut(c2).unwrap().constraints.stretch_y = 2.0;
        tree.get_mut(c1).unwrap().constraints.min_height = Some(0.0);
        tree.get_mut(c2).unwrap().constraints.min_height = Some(0.0);
        tree.set_root(root);
        tree.add_child(root, c1);
        tree.add_child(root, c2);
        tree.layout(Vec2::new(400.0, 300.0));

        let h1 = tree.get(c1).unwrap().bounds.height();
        let h2 = tree.get(c2).unwrap().bounds.height();
        // c2 should get roughly 2x the height of c1.
        assert!(
            h2 > h1 * 1.5,
            "stretch 2.0 should give more space than stretch 1.0"
        );
    }

    #[test]
    fn test_padding_insets_content() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        tree.get_mut(root).unwrap().constraints.padding = Padding::all(20.0);
        let child = tree.create_widget(WidgetKind::Label("X".into()));
        tree.set_root(root);
        tree.add_child(root, child);
        tree.layout(Vec2::new(200.0, 200.0));

        let child_bounds = tree.get(child).unwrap().bounds;
        assert!(
            child_bounds.min.x >= 20.0,
            "child should be inset by padding"
        );
        assert!(child_bounds.min.y >= 20.0, "padding top");
    }

    #[test]
    fn test_tree_len_and_is_empty() {
        let mut tree = UiTree::new();
        assert!(tree.is_empty());
        let root = tree.create_widget(WidgetKind::Container);
        assert_eq!(tree.len(), 1);
        tree.remove_widget(root);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_widget_lifecycle_create_layout_destroy() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let btn = tree.create_widget(WidgetKind::Button("Play".into()));
        let label = tree.create_widget(WidgetKind::Label("Score".into()));
        tree.set_root(root);
        tree.add_child(root, btn);
        tree.add_child(root, label);
        assert_eq!(tree.len(), 3);

        tree.layout(Vec2::new(800.0, 600.0));
        let btn_bounds = tree.get(btn).unwrap().bounds;
        assert!(btn_bounds.width() > 0.0);
        assert!(btn_bounds.height() > 0.0);

        tree.remove_widget(btn);
        assert_eq!(tree.len(), 2);
        assert!(tree.get(btn).is_none());
    }

    #[test]
    fn test_event_click_on_label_not_consumed() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let label = tree.create_widget(WidgetKind::Label("text".into()));
        tree.set_root(root);
        tree.add_child(root, label);
        tree.layout(Vec2::new(400.0, 300.0));

        let label_bounds = tree.get(label).unwrap().bounds;
        let response = tree.dispatch_event(
            label,
            &UiEvent::Click {
                pos: label_bounds.center(),
            },
        );
        assert!(!response.consumed, "label click should not consume");
    }

    #[test]
    fn test_event_hover_propagates() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let child = tree.create_widget(WidgetKind::Button("OK".into()));
        tree.set_root(root);
        tree.add_child(root, child);
        tree.layout(Vec2::new(400.0, 300.0));

        let child_bounds = tree.get(child).unwrap().bounds;
        let response = tree.dispatch_event(
            child,
            &UiEvent::Hover {
                pos: child_bounds.center(),
            },
        );
        assert!(!response.consumed, "hover should not consume");
    }

    #[test]
    fn test_event_focus_marks_dirty() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let field = tree.create_widget(WidgetKind::TextField {
            text: String::new(),
            placeholder: "type".into(),
        });
        tree.set_root(root);
        tree.add_child(root, field);
        tree.clear_dirty();

        let response = tree.dispatch_event(field, &UiEvent::FocusGained);
        assert!(response.dirty, "focus should mark dirty");
    }

    #[test]
    fn test_event_focus_lost_marks_dirty() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let field = tree.create_widget(WidgetKind::TextField {
            text: String::new(),
            placeholder: "type".into(),
        });
        tree.set_root(root);
        tree.add_child(root, field);
        tree.clear_dirty();

        let response = tree.dispatch_event(field, &UiEvent::FocusLost);
        assert!(response.dirty, "focus lost should mark dirty");
    }

    #[test]
    fn test_layout_flex_stretch_ratio() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let c1 = tree.create_widget(WidgetKind::Label("A".into()));
        let c2 = tree.create_widget(WidgetKind::Label("B".into()));
        let c3 = tree.create_widget(WidgetKind::Label("C".into()));
        tree.get_mut(c1).unwrap().constraints.stretch_y = 1.0;
        tree.get_mut(c2).unwrap().constraints.stretch_y = 1.0;
        tree.get_mut(c3).unwrap().constraints.stretch_y = 1.0;
        tree.get_mut(c1).unwrap().constraints.min_height = Some(0.0);
        tree.get_mut(c2).unwrap().constraints.min_height = Some(0.0);
        tree.get_mut(c3).unwrap().constraints.min_height = Some(0.0);
        tree.set_root(root);
        tree.add_child(root, c1);
        tree.add_child(root, c2);
        tree.add_child(root, c3);
        tree.layout(Vec2::new(400.0, 300.0));

        let h1 = tree.get(c1).unwrap().bounds.height();
        let h2 = tree.get(c2).unwrap().bounds.height();
        let h3 = tree.get(c3).unwrap().bounds.height();
        assert!(
            (h1 - h2).abs() < 1.0,
            "equal stretch should give equal height"
        );
        assert!(
            (h2 - h3).abs() < 1.0,
            "equal stretch should give equal height"
        );
    }

    #[test]
    fn test_layout_constraints_stored() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let child = tree.create_widget(WidgetKind::Label("X".into()));
        tree.get_mut(child).unwrap().constraints.max_width = Some(50.0);
        tree.get_mut(child).unwrap().constraints.max_height = Some(30.0);
        tree.get_mut(child).unwrap().constraints.min_width = Some(10.0);
        tree.get_mut(child).unwrap().constraints.min_height = Some(5.0);
        tree.set_root(root);
        tree.add_child(root, child);
        tree.layout(Vec2::new(400.0, 300.0));

        let c = tree.get(child).unwrap();
        assert_eq!(c.constraints.max_width, Some(50.0));
        assert_eq!(c.constraints.max_height, Some(30.0));
        assert_eq!(c.constraints.min_width, Some(10.0));
        assert_eq!(c.constraints.min_height, Some(5.0));
    }

    #[test]
    fn test_descendants_removed_with_parent() {
        let mut tree = UiTree::new();
        let root = tree.create_widget(WidgetKind::Container);
        let parent = tree.create_widget(WidgetKind::Container);
        let child = tree.create_widget(WidgetKind::Label("deep".into()));
        tree.set_root(root);
        tree.add_child(root, parent);
        tree.add_child(parent, child);
        assert_eq!(tree.len(), 3);

        tree.remove_widget(parent);
        assert_eq!(tree.len(), 1);
        assert!(tree.get(parent).is_none());
        assert!(tree.get(child).is_none());
    }
}
