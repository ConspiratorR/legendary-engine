use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use serde::{Deserialize, Serialize};

use super::graph::{Node, NodeGraph};
use super::types::{Connection, NodeId, PinId, PinType};

/// State for the node graph renderer.
#[derive(Debug, Clone)]
pub struct NodeGraphRenderer {
    /// Pan offset of the graph view.
    pub pan_offset: Vec2,
    /// Zoom level (1.0 = 100%).
    pub zoom: f32,
    /// Currently selected node.
    pub selected_node: Option<NodeId>,
    /// Drag state for moving nodes.
    pub drag_state: DragState,
    /// Connection being drawn (from output pin).
    pub pending_connection: Option<PinId>,
    /// Mouse position during connection drawing.
    pub pending_connection_pos: Option<Pos2>,
    /// Context menu state.
    pub context_menu: Option<ContextMenuState>,
    /// Whether the graph panel is visible.
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub enum DragState {
    None,
    MovingNode { node_id: NodeId, offset: Vec2 },
    Panning { last_pos: Pos2 },
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub position: Pos2,
    pub filter: String,
}

impl Default for NodeGraphRenderer {
    fn default() -> Self {
        Self {
            pan_offset: Vec2::ZERO,
            zoom: 1.0,
            selected_node: None,
            drag_state: DragState::None,
            pending_connection: None,
            pending_connection_pos: None,
            context_menu: None,
            visible: false,
        }
    }
}

impl NodeGraphRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert graph coordinates to screen coordinates.
    fn graph_to_screen(&self, graph_pos: Pos2) -> Pos2 {
        Pos2::new(
            graph_pos.x * self.zoom + self.pan_offset.x,
            graph_pos.y * self.zoom + self.pan_offset.y,
        )
    }

    /// Convert screen coordinates to graph coordinates.
    fn screen_to_graph(&self, screen_pos: Pos2) -> Pos2 {
        Pos2::new(
            (screen_pos.x - self.pan_offset.x) / self.zoom,
            (screen_pos.y - self.pan_offset.y) / self.zoom,
        )
    }

    /// Draw the entire node graph.
    pub fn draw(&mut self, ui: &mut egui::Ui, rect: Rect, graph: &mut NodeGraph) {
        let painter = ui.painter_at(rect);

        // Background
        painter.add(Shape::rect_filled(
            rect,
            Rounding::ZERO,
            Color32::from_rgb(30, 30, 34),
        ));

        // Draw grid
        self.draw_grid(&painter, rect);

        // Draw connections
        self.draw_connections(&painter, rect, graph);

        // Draw pending connection
        if let (Some(from_pin), Some(mouse_pos)) =
            (self.pending_connection, self.pending_connection_pos)
        {
            self.draw_pending_connection(&painter, from_pin, mouse_pos, graph);
        }

        // Draw nodes
        let node_ids: Vec<NodeId> = graph.nodes.keys().copied().collect();
        for node_id in &node_ids {
            let is_selected = self.selected_node == Some(*node_id);
            if let Some(node) = graph.nodes.get(node_id) {
                self.draw_node(&painter, rect, node, is_selected);
            }
        }

        // Handle interactions
        self.handle_interactions(ui, rect, graph);
    }

    /// Draw the background grid.
    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let grid_size = 20.0 * self.zoom;
        if grid_size < 4.0 {
            return; // Too zoomed out to show grid
        }

        let grid_color = Color32::from_rgb(38, 38, 42);
        let major_grid_color = Color32::from_rgb(45, 45, 53);

        let start_x = (self.pan_offset.x % grid_size) - grid_size;
        let start_y = (self.pan_offset.y % grid_size) - grid_size;

        let mut x = start_x;
        while x < rect.width() + grid_size {
            let color = if ((x - self.pan_offset.x) / grid_size).round() as i32 % 5 == 0 {
                major_grid_color
            } else {
                grid_color
            };
            painter.add(Shape::line(
                vec![
                    Pos2::new(rect.left() + x, rect.top()),
                    Pos2::new(rect.left() + x, rect.bottom()),
                ],
                Stroke::new(1.0_f32, color),
            ));
            x += grid_size;
        }

        let mut y = start_y;
        while y < rect.height() + grid_size {
            let color = if ((y - self.pan_offset.y) / grid_size).round() as i32 % 5 == 0 {
                major_grid_color
            } else {
                grid_color
            };
            painter.add(Shape::line(
                vec![
                    Pos2::new(rect.left(), rect.top() + y),
                    Pos2::new(rect.right(), rect.top() + y),
                ],
                Stroke::new(1.0_f32, color),
            ));
            y += grid_size;
        }
    }

    /// Draw a single node.
    fn draw_node(&self, painter: &egui::Painter, _rect: Rect, node: &Node, is_selected: bool) {
        let screen_pos = self.graph_to_screen(node.position.to_pos2());
        let w = node.width() * self.zoom;
        let h = node.height() * self.zoom;
        let node_rect = Rect::from_min_size(screen_pos, Vec2::new(w, h));

        // Node background
        let bg_color = if is_selected {
            Color32::from_rgb(40, 45, 55)
        } else {
            Color32::from_rgb(35, 35, 40)
        };
        let rounding = Rounding::same(6.0 * self.zoom);
        painter.add(Shape::rect_filled(node_rect, rounding, bg_color));

        // Node border
        let border_color = if is_selected {
            Color32::from_rgb(0, 212, 170)
        } else {
            Color32::from_rgb(55, 55, 65)
        };
        painter.add(Shape::rect_stroke(
            node_rect,
            rounding,
            Stroke::new(1.5 * self.zoom, border_color),
        ));

        // Title bar
        let title_h = 28.0 * self.zoom;
        let title_rect = Rect::from_min_size(screen_pos, Vec2::new(w, title_h));
        let title_color = match node.node_type.category() {
            super::graph::NodeCategory::Input => Color32::from_rgb(46, 100, 150),
            super::graph::NodeCategory::Math => Color32::from_rgb(80, 80, 120),
            super::graph::NodeCategory::Texture => Color32::from_rgb(120, 80, 40),
            super::graph::NodeCategory::Color => Color32::from_rgb(120, 100, 40),
            super::graph::NodeCategory::Vector => Color32::from_rgb(80, 120, 80),
            super::graph::NodeCategory::Output => Color32::from_rgb(150, 50, 50),
            super::graph::NodeCategory::Custom(_) => Color32::from_rgb(80, 80, 80),
        };
        painter.add(Shape::rect_filled(
            title_rect,
            Rounding::same(6.0 * self.zoom),
            title_color,
        ));
        // Clip the bottom corners of the title
        painter.add(Shape::rect_filled(
            Rect::from_min_size(
                Pos2::new(title_rect.left(), title_rect.bottom() - 6.0 * self.zoom),
                Vec2::new(title_rect.width(), 6.0 * self.zoom),
            ),
            Rounding::ZERO,
            title_color,
        ));

        // Title text
        let font_size = 12.0 * self.zoom;
        painter.text(
            Pos2::new(screen_pos.x + 8.0 * self.zoom, title_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &node.name,
            egui::FontId::proportional(font_size),
            Color32::WHITE,
        );

        // Draw pins
        let pin_start_y = screen_pos.y + title_h + 4.0 * self.zoom;
        let pin_h = 20.0 * self.zoom;

        // Input pins (left side)
        for (i, pin) in node.inputs.iter().enumerate() {
            let y = pin_start_y + i as f32 * pin_h;
            let pin_pos = Pos2::new(screen_pos.x, y + pin_h / 2.0);

            // Pin circle
            let pin_r = 5.0 * self.zoom;
            painter.add(Shape::circle_filled(pin_pos, pin_r, pin.pin_type.color()));

            // Pin label
            painter.text(
                Pos2::new(screen_pos.x + 14.0 * self.zoom, y + pin_h / 2.0),
                egui::Align2::LEFT_CENTER,
                &pin.name,
                egui::FontId::proportional(10.0 * self.zoom),
                Color32::from_gray(180),
            );
        }

        // Output pins (right side)
        for (i, pin) in node.outputs.iter().enumerate() {
            let y = pin_start_y + i as f32 * pin_h;
            let pin_pos = Pos2::new(screen_pos.x + w, y + pin_h / 2.0);

            // Pin circle
            let pin_r = 5.0 * self.zoom;
            painter.add(Shape::circle_filled(pin_pos, pin_r, pin.pin_type.color()));

            // Pin label (right-aligned)
            painter.text(
                Pos2::new(screen_pos.x + w - 14.0 * self.zoom, y + pin_h / 2.0),
                egui::Align2::RIGHT_CENTER,
                &pin.name,
                egui::FontId::proportional(10.0 * self.zoom),
                Color32::from_gray(180),
            );
        }
    }

    /// Draw all connections.
    fn draw_connections(&self, painter: &egui::Painter, _rect: Rect, graph: &NodeGraph) {
        for conn in &graph.connections {
            self.draw_connection(painter, conn, graph);
        }
    }

    /// Draw a single connection as a bezier curve.
    fn draw_connection(&self, painter: &egui::Painter, conn: &Connection, graph: &NodeGraph) {
        let from_node = match graph.nodes.get(&conn.output_pin.node_id) {
            Some(n) => n,
            None => return,
        };
        let to_node = match graph.nodes.get(&conn.input_pin.node_id) {
            Some(n) => n,
            None => return,
        };

        let from_screen = self.graph_to_screen(from_node.position.to_pos2());
        let to_screen = self.graph_to_screen(to_node.position.to_pos2());

        let from_w = from_node.width() * self.zoom;
        let title_h = 28.0 * self.zoom;
        let pin_h = 20.0 * self.zoom;

        // Output pin position
        let output_idx = conn.output_pin.index - from_node.inputs.len();
        let from_pos = Pos2::new(
            from_screen.x + from_w,
            from_screen.y + title_h + 4.0 * self.zoom + output_idx as f32 * pin_h + pin_h / 2.0,
        );

        // Input pin position
        let to_pos = Pos2::new(
            to_screen.x,
            to_screen.y
                + title_h
                + 4.0 * self.zoom
                + conn.input_pin.index as f32 * pin_h
                + pin_h / 2.0,
        );

        // Bezier curve
        let dx = (to_pos.x - from_pos.x).abs() * 0.5;
        let p0 = from_pos;
        let p1 = Pos2::new(from_pos.x + dx, from_pos.y);
        let p2 = Pos2::new(to_pos.x - dx, to_pos.y);
        let p3 = to_pos;

        let color = PinType::Float.color(); // Use output pin color
        let stroke = Stroke::new(2.0 * self.zoom, color);

        // Draw bezier using line segments
        let steps = 20;
        let mut points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = cubic_bezier(p0.x, p1.x, p2.x, p3.x, t);
            let y = cubic_bezier(p0.y, p1.y, p2.y, p3.y, t);
            points.push(Pos2::new(x, y));
        }
        painter.add(Shape::line(points, stroke));
    }

    /// Draw a pending connection being dragged.
    fn draw_pending_connection(
        &self,
        painter: &egui::Painter,
        from_pin: PinId,
        mouse_pos: Pos2,
        graph: &NodeGraph,
    ) {
        let from_node = match graph.nodes.get(&from_pin.node_id) {
            Some(n) => n,
            None => return,
        };

        let from_screen = self.graph_to_screen(from_node.position.to_pos2());
        let from_w = from_node.width() * self.zoom;
        let title_h = 28.0 * self.zoom;
        let pin_h = 20.0 * self.zoom;

        let pin_idx = from_pin.index - from_node.inputs.len();
        let from_pos = Pos2::new(
            from_screen.x + from_w,
            from_screen.y + title_h + 4.0 * self.zoom + pin_idx as f32 * pin_h + pin_h / 2.0,
        );

        let dx = (mouse_pos.x - from_pos.x).abs() * 0.5;
        let p0 = from_pos;
        let p1 = Pos2::new(from_pos.x + dx, from_pos.y);
        let p2 = Pos2::new(mouse_pos.x - dx, mouse_pos.y);
        let p3 = mouse_pos;

        let color = Color32::from_rgb(255, 255, 100);
        let stroke = Stroke::new(2.0 * self.zoom, color);

        let steps = 20;
        let mut points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = cubic_bezier(p0.x, p1.x, p2.x, p3.x, t);
            let y = cubic_bezier(p0.y, p1.y, p2.y, p3.y, t);
            points.push(Pos2::new(x, y));
        }
        painter.add(Shape::line(points, stroke));
    }

    /// Handle all interactions (click, drag, connect, context menu).
    fn handle_interactions(&mut self, ui: &mut egui::Ui, rect: Rect, graph: &mut NodeGraph) {
        let response = ui.interact(
            rect,
            egui::Id::new("node_graph_canvas"),
            egui::Sense::click_and_drag(),
        );

        let pointer_pos = ui.input(|i| i.pointer.latest_pos());
        let Some(pointer_pos) = pointer_pos else {
            return;
        };

        if !rect.contains(pointer_pos) {
            return;
        }

        let graph_pos = self.screen_to_graph(pointer_pos);

        // Handle zoom
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_delta.abs() > 0.0 {
            let old_zoom = self.zoom;
            self.zoom = (self.zoom * (1.0 + scroll_delta * 0.001)).clamp(0.1, 3.0);

            // Zoom towards mouse position
            let zoom_ratio = self.zoom / old_zoom;
            self.pan_offset.x = pointer_pos.x - (pointer_pos.x - self.pan_offset.x) * zoom_ratio;
            self.pan_offset.y = pointer_pos.y - (pointer_pos.y - self.pan_offset.y) * zoom_ratio;
        }

        // Handle panning with middle mouse button
        if response.dragged_by(egui::PointerButton::Middle) {
            match &self.drag_state {
                DragState::None => {
                    self.drag_state = DragState::Panning {
                        last_pos: pointer_pos,
                    };
                }
                DragState::Panning { last_pos } => {
                    let delta = pointer_pos - *last_pos;
                    self.pan_offset += delta;
                    self.drag_state = DragState::Panning {
                        last_pos: pointer_pos,
                    };
                }
                _ => {}
            }
        } else if response.dragged_by(egui::PointerButton::Primary) {
            // Check if dragging a node
            match &self.drag_state {
                DragState::None => {
                    // Check if clicking on a pin
                    if let Some(pin_id) = self.find_pin_at(graph_pos, graph) {
                        // Output pins have index >= inputs.len()
                        if let Some(node) = graph.nodes.get(&pin_id.node_id)
                            && pin_id.index >= node.inputs.len()
                        {
                            self.pending_connection = Some(pin_id);
                        }
                    } else if let Some(node_id) = self.find_node_at(graph_pos, graph) {
                        let node = &graph.nodes[&node_id];
                        let node_pos = node.position.to_pos2();
                        let offset = graph_pos - node_pos;
                        self.drag_state = DragState::MovingNode { node_id, offset };
                        self.selected_node = Some(node_id);
                    }
                }
                DragState::MovingNode { node_id, offset } => {
                    let new_pos = graph_pos - *offset;
                    graph.move_node(*node_id, new_pos);
                }
                _ => {}
            }
        } else {
            // Mouse released
            if let Some(from_pin) = self.pending_connection {
                // Check if we're over an input pin
                if let Some(to_pin) = self.find_input_pin_at(graph_pos, graph) {
                    let _ = graph.connect(from_pin, to_pin);
                }
                self.pending_connection = None;
                self.pending_connection_pos = None;
            }
            self.drag_state = DragState::None;
        }

        // Update pending connection mouse position
        if self.pending_connection.is_some() {
            self.pending_connection_pos = Some(pointer_pos);
        }

        // Handle right-click context menu
        if response.secondary_clicked() {
            self.context_menu = Some(ContextMenuState {
                position: graph_pos,
                filter: String::new(),
            });
        }

        // Handle delete key
        if ui.input(|i| i.key_pressed(egui::Key::Delete))
            && let Some(node_id) = self.selected_node
        {
            graph.remove_node(node_id);
            self.selected_node = None;
        }
    }

    /// Find a node at the given graph position.
    fn find_node_at(&self, graph_pos: Pos2, graph: &NodeGraph) -> Option<NodeId> {
        // Search in reverse order (top-most node first)
        for (id, node) in &graph.nodes {
            let node_rect = Rect::from_min_size(
                node.position.to_pos2(),
                Vec2::new(node.width(), node.height()),
            );
            if node_rect.contains(graph_pos) {
                return Some(*id);
            }
        }
        None
    }

    /// Find a pin at the given graph position.
    fn find_pin_at(&self, graph_pos: Pos2, graph: &NodeGraph) -> Option<PinId> {
        for node in graph.nodes.values() {
            let screen_pos = self.graph_to_screen(node.position.to_pos2());
            let title_h = 28.0 * self.zoom;
            let pin_h = 20.0 * self.zoom;
            let pin_r = 8.0 * self.zoom;

            // Check output pins
            for (i, pin) in node.outputs.iter().enumerate() {
                let y = screen_pos.y + title_h + 4.0 * self.zoom + i as f32 * pin_h + pin_h / 2.0;
                let pin_pos = Pos2::new(screen_pos.x + node.width() * self.zoom, y);
                let dist = (graph_pos - pin_pos).length();
                if dist < pin_r {
                    return Some(pin.id);
                }
            }
        }
        None
    }

    /// Find an input pin at the given graph position.
    fn find_input_pin_at(&self, graph_pos: Pos2, graph: &NodeGraph) -> Option<PinId> {
        for node in graph.nodes.values() {
            let screen_pos = self.graph_to_screen(node.position.to_pos2());
            let title_h = 28.0 * self.zoom;
            let pin_h = 20.0 * self.zoom;
            let pin_r = 8.0 * self.zoom;

            // Check input pins
            for (i, pin) in node.inputs.iter().enumerate() {
                let y = screen_pos.y + title_h + 4.0 * self.zoom + i as f32 * pin_h + pin_h / 2.0;
                let pin_pos = Pos2::new(screen_pos.x, y);
                let dist = (graph_pos - pin_pos).length();
                if dist < pin_r {
                    return Some(pin.id);
                }
            }
        }
        None
    }
}

/// Cubic bezier interpolation.
fn cubic_bezier(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    mt3 * p0 + 3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3 * p3
}

/// Serializable graph state for the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGraphState {
    pub graph: NodeGraph,
    pub pan_offset: [f32; 2],
    pub zoom: f32,
    pub selected_node: Option<NodeId>,
}

impl Default for NodeGraphState {
    fn default() -> Self {
        Self {
            graph: NodeGraph::new(),
            pan_offset: [0.0, 0.0],
            zoom: 1.0,
            selected_node: None,
        }
    }
}

impl NodeGraphState {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_default() {
        let renderer = NodeGraphRenderer::new();
        assert_eq!(renderer.zoom, 1.0);
        assert_eq!(renderer.pan_offset, Vec2::ZERO);
        assert!(renderer.selected_node.is_none());
    }

    #[test]
    fn test_graph_to_screen_roundtrip() {
        let mut renderer = NodeGraphRenderer::new();
        renderer.pan_offset = Vec2::new(100.0, 50.0);
        renderer.zoom = 2.0;

        let graph_pos = Pos2::new(50.0, 30.0);
        let screen_pos = renderer.graph_to_screen(graph_pos);
        let back = renderer.screen_to_graph(screen_pos);

        assert!((back.x - graph_pos.x).abs() < 0.01);
        assert!((back.y - graph_pos.y).abs() < 0.01);
    }

    #[test]
    fn test_cubic_bezier() {
        let start = 0.0;
        let end = 100.0;
        let mid = cubic_bezier(start, start + 50.0, end - 50.0, end, 0.5);
        assert!(
            (mid - 50.0).abs() < 1.0,
            "midpoint should be ~50, got {}",
            mid
        );
    }

    #[test]
    fn test_node_graph_state_json() {
        let state = NodeGraphState::default();
        let json = state.to_json().unwrap();
        let restored = NodeGraphState::from_json(&json).unwrap();
        assert_eq!(restored.zoom, 1.0);
    }
}
