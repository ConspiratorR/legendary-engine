use crate::state::EditorState;
use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::Gui;

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.right() - 1.0, rect.top()),
            Pos2::new(rect.right() - 1.0, rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let header_h = 36.0 * h_scale;
    let header_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), header_h));
    painter.add(Shape::rect_filled(
        header_rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.text(
        egui::pos2(rect.left() + 12.0 * w_scale, header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "层级",
        FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );

    let btn_sz = 24.0 * h_scale;
    let spacing = 28.0 * w_scale;
    let rounding = 4.0 * h_scale;
    for (i, icon) in ["+", "✕"].iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(
                rect.right() - spacing - i as f32 * spacing,
                header_rect.top() + (header_h - btn_sz) / 2.0,
            ),
            Vec2::new(btn_sz, btn_sz),
        );
        let id = egui::Id::new("hdr_act").with(i as u64);
        let response = gui.ui.interact(btn_rect, id, egui::Sense::click());
        if response.hovered() {
            painter.add(Shape::rect_filled(
                btn_rect,
                Rounding::same(rounding),
                Color32::from_rgb(30, 30, 34),
            ));
        }
        if response.clicked() {
            if i == 0 {
                // "+" button: add node under selected or root
                let parent = state.selected_nodes.first().copied().or(state
                    .scene_tree
                    .root_ids
                    .first()
                    .copied());
                state.scene_tree.add_node("New Node", parent);
            } else if i == 1 && !state.selected_nodes.is_empty() {
                // "✕" button: delete selected nodes
                let to_delete: Vec<u64> = state.selected_nodes.clone();
                for &node_id in &to_delete {
                    state.scene_tree.remove_node(node_id);
                }
                state.selected_nodes.clear();
            }
        }
        painter.text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            *icon,
            FontId::proportional(14.0 * h_scale),
            Color32::from_gray(90),
        );
    }

    let line_y = header_rect.bottom() - 1.0;
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), line_y),
            Pos2::new(rect.right(), line_y),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    // Search bar (visual, with character-based search by clicking to cycle)
    let search_h = 28.0 * h_scale;
    let search_rect = Rect::from_min_size(
        Pos2::new(
            rect.left() + 8.0 * w_scale,
            header_rect.bottom() + 4.0 * h_scale,
        ),
        Vec2::new(rect.width() - 16.0 * w_scale, search_h),
    );

    painter.add(Shape::rect_filled(
        search_rect,
        Rounding::same(4.0 * h_scale),
        Color32::from_rgb(30, 30, 34),
    ));

    let search_text = if state.hierarchy_search.is_empty() {
        "🔍 搜索...".to_string()
    } else {
        format!("🔍 {}", state.hierarchy_search)
    };

    painter.text(
        egui::pos2(search_rect.left() + 8.0 * w_scale, search_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &search_text,
        FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );

    // Clear search button
    if !state.hierarchy_search.is_empty() {
        let clear_rect = Rect::from_min_size(
            Pos2::new(search_rect.right() - 32.0 * w_scale, search_rect.top()),
            Vec2::new(28.0 * w_scale, search_rect.height()),
        );
        let clear_id = egui::Id::new("clear_search");
        let clear_response = gui.ui.interact(clear_rect, clear_id, egui::Sense::click());
        if clear_response.hovered() {
            painter.add(Shape::rect_filled(
                clear_rect,
                Rounding::same(4.0 * h_scale),
                Color32::from_rgb(40, 40, 44),
            ));
        }
        painter.text(
            clear_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✕",
            FontId::proportional(12.0 * h_scale),
            Color32::from_gray(90),
        );
        if clear_response.clicked() {
            state.hierarchy_search.clear();
        }
    }

    let content_top = search_rect.bottom() + 4.0 * h_scale;
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), content_top),
        Vec2::new(rect.width(), rect.bottom() - content_top),
    );

    let mut y = content_rect.top() + 4.0 * h_scale;
    let item_h = 28.0 * h_scale;
    let left = rect.left() + 8.0 * w_scale;
    let right = rect.right() - 8.0 * w_scale;

    // Get search results if query is not empty
    let search_results = if !state.hierarchy_search.is_empty() {
        state.scene_tree.search(&state.hierarchy_search)
    } else {
        Vec::new()
    };

    let root_ids: Vec<u64> = state.scene_tree.root_ids.clone();
    for &root_id in &root_ids {
        y = draw_node(
            state,
            gui,
            root_id,
            0,
            &mut y,
            left,
            right,
            item_h,
            h_scale,
            &search_results,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_node(
    state: &mut EditorState,
    gui: &mut Gui,
    node_id: u64,
    depth: u32,
    y: &mut f32,
    left: f32,
    right: f32,
    item_h: f32,
    h_scale: f32,
    search_results: &[u64],
) -> f32 {
    let (node_name, node_icon, node_expanded, children) = {
        let node = match state.scene_tree.nodes.iter().find(|n| n.id == node_id) {
            Some(n) => n,
            None => return *y,
        };
        (
            node.name.clone(),
            node.icon.clone(),
            node.expanded,
            node.children.clone(),
        )
    };

    let indent_step = 16.0 * h_scale;
    let arrow_sz = 16.0 * h_scale;
    let rounding = 4.0 * h_scale;
    let icon_font = 14.0 * h_scale;
    let label_font = 13.0 * h_scale;
    let arrow_font = 10.0 * h_scale;
    let indent = left + depth as f32 * indent_step;

    let id_rect = Rect::from_min_size(Pos2::new(indent, *y), Vec2::new(right - indent, item_h));
    let id = egui::Id::new("tree").with(node_id);
    let response = gui.ui.interact(id_rect, id, egui::Sense::click());

    let painter = gui.ui.painter_at(id_rect);
    let is_selected = state.selected_nodes.contains(&node_id);
    let is_search_match = search_results.contains(&node_id);

    if is_selected {
        painter.add(Shape::rect_filled(
            id_rect,
            Rounding::same(rounding),
            Color32::from_rgba_premultiplied(0, 212, 170, 30),
        ));
    } else if is_search_match {
        painter.add(Shape::rect_filled(
            id_rect,
            Rounding::same(rounding),
            Color32::from_rgba_premultiplied(255, 184, 0, 20),
        ));
    } else if response.hovered() {
        painter.add(Shape::rect_filled(
            id_rect,
            Rounding::same(rounding),
            Color32::from_rgb(30, 30, 34),
        ));
    }

    let has_children = !children.is_empty();
    let arrow_rect = Rect::from_min_size(
        Pos2::new(indent, *y + (item_h - arrow_sz) / 2.0),
        Vec2::new(arrow_sz, arrow_sz),
    );
    if has_children {
        let arrow_id = egui::Id::new("arrow").with(node_id);
        let arrow_response = gui.ui.interact(arrow_rect, arrow_id, egui::Sense::click());
        if arrow_response.clicked()
            && let Some(n) = state.scene_tree.nodes.iter_mut().find(|n| n.id == node_id)
        {
            n.expanded = !n.expanded;
        }
        painter.text(
            arrow_rect.center(),
            egui::Align2::CENTER_CENTER,
            if node_expanded { "▾" } else { "▸" },
            FontId::proportional(arrow_font),
            Color32::from_gray(90),
        );
    }

    painter.text(
        egui::pos2(indent + 20.0 * h_scale, *y + item_h / 2.0),
        egui::Align2::LEFT_CENTER,
        &node_icon,
        FontId::proportional(icon_font),
        Color32::from_gray(200),
    );

    painter.text(
        egui::pos2(indent + 42.0 * h_scale, *y + item_h / 2.0),
        egui::Align2::LEFT_CENTER,
        &node_name,
        FontId::proportional(label_font),
        if is_search_match {
            Color32::from_rgb(255, 184, 0)
        } else {
            Color32::from_rgb(232, 232, 236)
        },
    );

    if response.clicked() {
        state.selected_nodes.clear();
        state.selected_nodes.push(node_id);
    }

    *y += item_h;

    if node_expanded || !search_results.is_empty() {
        for &child_id in &children {
            *y = draw_node(
                state,
                gui,
                child_id,
                depth + 1,
                y,
                left,
                right,
                item_h,
                h_scale,
                search_results,
            );
        }
    }

    *y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;
    use engine_ui::{Gui, GuiSkin};

    #[test]
    fn test_draw_hierarchy_no_panic() {
        let ctx = egui::Context::default();
        let skin = GuiSkin::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::Area::new(egui::Id::new("h_test")).show(ctx, |ui| {
                let mut state = EditorState::new();
                let mut gui = Gui::new(ui, &skin);
                let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(260.0, 600.0));
                draw(&mut state, &mut gui, rect);
            });
        });
    }
}
