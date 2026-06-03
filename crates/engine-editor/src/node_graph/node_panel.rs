use egui::{Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use std::collections::HashSet;

use super::graph::{NodeCategory, NodeType};
use super::nodes::builtin_node_types;

/// Node panel for browsing and selecting nodes to add to the graph.
#[derive(Debug, Clone)]
pub struct NodePanel {
    /// Current search filter text.
    pub search_text: String,
    /// Set of favorited node type names.
    pub favorites: HashSet<String>,
    /// Which categories are expanded.
    pub expanded_categories: HashSet<String>,
    /// Whether the panel is visible.
    pub visible: bool,
    /// Node type being dragged from the panel.
    pub dragging: Option<NodeType>,
}

impl Default for NodePanel {
    fn default() -> Self {
        let mut expanded_categories = HashSet::new();
        expanded_categories.insert("Blueprint".to_string());
        expanded_categories.insert("Input".to_string());

        Self {
            search_text: String::new(),
            favorites: HashSet::new(),
            expanded_categories,
            visible: true,
            dragging: None,
        }
    }
}

impl NodePanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draw the node panel UI.
    pub fn draw(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);

        // Background
        painter.add(Shape::rect_filled(
            rect,
            Rounding::ZERO,
            Color32::from_rgb(22, 22, 25),
        ));

        // Left border
        painter.add(Shape::line(
            vec![
                Pos2::new(rect.left(), rect.top()),
                Pos2::new(rect.left(), rect.bottom()),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
        ));

        let h_scale = ui.ctx().screen_rect().height() / 1080.0;
        let w_scale = ui.ctx().screen_rect().width() / 1920.0;
        let pad = 8.0 * w_scale;

        // Search bar
        let search_h = 32.0 * h_scale;
        let search_rect = Rect::from_min_size(
            Pos2::new(rect.left() + pad, rect.top() + pad),
            Vec2::new(rect.width() - pad * 2.0, search_h),
        );

        // Search background
        painter.add(Shape::rect_filled(
            search_rect,
            Rounding::same(4.0),
            Color32::from_rgb(30, 30, 34),
        ));

        // Search input
        let search_id = egui::Id::new("node_panel_search");
        let mut search_text = self.search_text.clone();
        let search_response = ui.put(
            search_rect,
            egui::TextEdit::singleline(&mut search_text)
                .id(search_id)
                .font(egui::FontId::proportional(12.0 * h_scale))
                .text_color(Color32::from_gray(200))
                .frame(false)
                .hint_text("Search nodes..."),
        );
        if search_response.changed() {
            self.search_text = search_text;
        }

        // Content area
        let content_top = rect.top() + pad + search_h + pad;
        let content_rect = Rect::from_min_size(
            Pos2::new(rect.left(), content_top),
            Vec2::new(rect.width(), rect.bottom() - content_top),
        );

        // Collect and filter nodes
        let all_types = builtin_node_types();
        let filter = self.search_text.to_lowercase();

        // Group by category
        let mut categories: Vec<(NodeCategory, Vec<&NodeType>)> = Vec::new();
        for node_type in &all_types {
            let category = node_type.category();
            if !filter.is_empty()
                && !node_type.display_name().to_lowercase().contains(&filter)
                && !category.display_name().to_lowercase().contains(&filter)
            {
                continue;
            }
            if let Some(entry) = categories.iter_mut().find(|(c, _)| *c == category) {
                entry.1.push(node_type);
            } else {
                categories.push((category, vec![node_type]));
            }
        }

        // Sort: favorites first, then alphabetically
        categories.sort_by(|a, b| {
            let a_name = a.0.display_name();
            let b_name = b.0.display_name();
            if a_name == "Blueprint" {
                std::cmp::Ordering::Less
            } else if b_name == "Blueprint" {
                std::cmp::Ordering::Greater
            } else {
                a_name.cmp(b_name)
            }
        });

        // Scroll area for node list
        let mut scroll_rect = content_rect;
        scroll_rect.set_bottom(rect.bottom() - 4.0);

        // Draw categories
        let mut y = content_rect.top();
        let row_h = 24.0 * h_scale;
        let category_h = 22.0 * h_scale;

        for (category, nodes) in &categories {
            let cat_name = category.display_name();
            let is_expanded = self.expanded_categories.contains(cat_name);

            // Category header
            let header_rect = Rect::from_min_size(
                Pos2::new(content_rect.left() + pad, y),
                Vec2::new(content_rect.width() - pad * 2.0, category_h),
            );

            // Draw category header
            let arrow = if is_expanded { "▼" } else { "▶" };
            let header_text = format!("{} {}", arrow, cat_name);
            let header_color = category_color(category);

            let header_response = ui.put(
                header_rect,
                egui::Label::new(
                    egui::RichText::new(&header_text)
                        .font(egui::FontId::proportional(11.0 * h_scale))
                        .color(header_color),
                ),
            );

            if header_response.clicked() {
                if is_expanded {
                    self.expanded_categories.remove(cat_name);
                } else {
                    self.expanded_categories.insert(cat_name.to_string());
                }
            }

            y += category_h;

            // Draw nodes if expanded
            if is_expanded {
                for node_type in nodes {
                    if y > scroll_rect.bottom() {
                        break;
                    }

                    let node_name = node_type.display_name();
                    let is_favorite = self.favorites.contains(node_name);

                    let node_rect = Rect::from_min_size(
                        Pos2::new(content_rect.left() + pad * 2.0, y),
                        Vec2::new(content_rect.width() - pad * 4.0, row_h),
                    );

                    // Node item background
                    let item_response = ui.allocate_rect(node_rect, egui::Sense::click());
                    if item_response.hovered() {
                        painter.add(Shape::rect_filled(
                            node_rect,
                            Rounding::same(3.0),
                            Color32::from_rgb(40, 40, 48),
                        ));
                    }

                    // Favorite indicator
                    let fav_text = if is_favorite { "★ " } else { "  " };
                    let fav_color = if is_favorite {
                        Color32::from_rgb(255, 200, 50)
                    } else {
                        Color32::from_gray(60)
                    };
                    painter.text(
                        Pos2::new(node_rect.left() + 4.0, node_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        fav_text,
                        egui::FontId::proportional(10.0 * h_scale),
                        fav_color,
                    );

                    // Node name
                    painter.text(
                        Pos2::new(node_rect.left() + 20.0, node_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        node_name,
                        egui::FontId::proportional(11.0 * h_scale),
                        Color32::from_gray(180),
                    );

                    // Color indicator for category
                    let indicator_rect = Rect::from_min_size(
                        Pos2::new(node_rect.right() - 8.0, node_rect.center().y - 4.0),
                        Vec2::new(6.0, 8.0),
                    );
                    painter.add(Shape::rect_filled(
                        indicator_rect,
                        Rounding::same(2.0),
                        header_color,
                    ));

                    // Handle click for drag initiation
                    if item_response.drag_started() {
                        self.dragging = Some((*node_type).clone());
                    }

                    // Double-click to toggle favorite
                    if item_response.double_clicked() {
                        if is_favorite {
                            self.favorites.remove(node_name);
                        } else {
                            self.favorites.insert(node_name.to_string());
                        }
                    }

                    y += row_h;
                }
            }

            y += 2.0; // spacing between categories
        }
    }

    /// Get the node type being dragged (if any) and clear the state.
    pub fn take_dragging(&mut self) -> Option<NodeType> {
        self.dragging.take()
    }
}

fn category_color(category: &NodeCategory) -> Color32 {
    match category {
        NodeCategory::Input => Color32::from_rgb(46, 100, 150),
        NodeCategory::Math => Color32::from_rgb(80, 80, 120),
        NodeCategory::Texture => Color32::from_rgb(120, 80, 40),
        NodeCategory::Color => Color32::from_rgb(120, 100, 40),
        NodeCategory::Vector => Color32::from_rgb(80, 120, 80),
        NodeCategory::Output => Color32::from_rgb(150, 50, 50),
        NodeCategory::Blueprint => Color32::from_rgb(60, 130, 60),
        NodeCategory::Custom(_) => Color32::from_rgb(80, 80, 80),
    }
}

use egui::Shape;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_panel_default() {
        let panel = NodePanel::new();
        assert!(panel.visible);
        assert!(panel.search_text.is_empty());
        assert!(panel.favorites.is_empty());
        assert!(panel.expanded_categories.contains("Blueprint"));
        assert!(panel.expanded_categories.contains("Input"));
    }

    #[test]
    fn test_node_panel_search_filter() {
        let panel = NodePanel::new();
        assert_eq!(panel.search_text, "");
    }

    #[test]
    fn test_node_panel_favorites() {
        let mut panel = NodePanel::new();
        panel.favorites.insert("Add".to_string());
        assert!(panel.favorites.contains("Add"));
        assert!(!panel.favorites.contains("Subtract"));
    }

    #[test]
    fn test_node_panel_toggle_favorite() {
        let mut panel = NodePanel::new();
        let name = "Add";
        assert!(!panel.favorites.contains(name));
        panel.favorites.insert(name.to_string());
        assert!(panel.favorites.contains(name));
        panel.favorites.remove(name);
        assert!(!panel.favorites.contains(name));
    }

    #[test]
    fn test_category_colors() {
        assert_eq!(
            category_color(&NodeCategory::Blueprint),
            Color32::from_rgb(60, 130, 60)
        );
        assert_eq!(
            category_color(&NodeCategory::Input),
            Color32::from_rgb(46, 100, 150)
        );
    }

    #[test]
    fn test_node_panel_dragging() {
        let mut panel = NodePanel::new();
        assert!(panel.take_dragging().is_none());
        panel.dragging = Some(NodeType::Add);
        assert!(panel.take_dragging().is_some());
        assert!(panel.take_dragging().is_none());
    }
}
