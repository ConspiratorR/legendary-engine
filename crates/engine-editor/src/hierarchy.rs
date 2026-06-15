//! Scene hierarchy panel — displays the entity tree with drag-and-drop
//! selection and right-click context actions.

use crate::state::EditorState;
use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::Gui;

/// Available object types for creation.
const CREATE_TYPES: &[(&str, &str)] = &[
    ("空节点", "📄"),
    ("立方体", "📦"),
    ("球体", "🔮"),
    ("方向光", "☀"),
    ("点光源", "💡"),
    ("聚光灯", "🔦"),
];

/// Check if `target_id` is a descendant of `ancestor_id` in the scene tree.
fn is_descendant(tree: &crate::state::SceneTree, target_id: u64, ancestor_id: u64) -> bool {
    let mut current = Some(target_id);
    while let Some(id) = current {
        if id == ancestor_id {
            return true;
        }
        current = tree
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.parent);
    }
    false
}

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
    for (i, icon) in ["+", "✕", "📦"].iter().enumerate() {
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
                // "+" button: toggle creation menu
                state.show_create_menu = !state.show_create_menu;
            } else if i == 1 && !state.selected_nodes.is_empty() {
                // "✕" button: delete selected nodes
                let to_delete: Vec<u64> = state.selected_nodes.clone();
                for &node_id in &to_delete {
                    state.scene_tree.remove_node(node_id);
                }
                state.selected_nodes.clear();
            } else if i == 2 && !state.selected_nodes.is_empty() {
                // "📦" button: save selection as prefab
                let name = state.scene_tree.nodes.iter()
                    .find(|n| n.id == state.selected_nodes[0])
                    .map(|n| n.name.clone())
                    .unwrap_or_else(|| "Prefab".into());
                state.create_prefab_from_selection(&name);
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

    // Object creation menu
    if state.show_create_menu {
        let menu_x = rect.right() - 140.0 * w_scale;
        let menu_y = header_rect.bottom() + 2.0;
        let item_h = 28.0 * h_scale;
        let menu_w = 130.0 * w_scale;
        let menu_h = item_h * CREATE_TYPES.len() as f32;
        let menu_rect = Rect::from_min_size(
            Pos2::new(menu_x, menu_y),
            Vec2::new(menu_w, menu_h),
        );
        painter.add(Shape::rect_filled(
            menu_rect,
            Rounding::same(4.0 * h_scale),
            Color32::from_rgb(35, 35, 40),
        ));
        painter.rect_stroke(
            menu_rect,
            Rounding::same(4.0 * h_scale),
            Stroke::new(1.0_f32, Color32::from_rgb(55, 55, 60)),
        );

        for (j, (label, icon)) in CREATE_TYPES.iter().enumerate() {
            let item_y = menu_y + j as f32 * item_h;
            let item_rect = Rect::from_min_size(
                Pos2::new(menu_x, item_y),
                Vec2::new(menu_w, item_h),
            );
            let item_id = egui::Id::new("create_item").with(j as u64);
            let item_resp = gui.ui.interact(item_rect, item_id, egui::Sense::click());
            if item_resp.hovered() {
                painter.add(Shape::rect_filled(
                    item_rect,
                    Rounding::ZERO,
                    Color32::from_rgb(0, 80, 60),
                ));
            }
            painter.text(
                Pos2::new(menu_x + 8.0 * w_scale, item_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{} {}", icon, label),
                FontId::proportional(11.0 * h_scale),
                Color32::from_gray(200),
            );
            if item_resp.clicked() {
                let parent = state
                    .selected_nodes
                    .first()
                    .copied()
                    .or(state.scene_tree.root_ids.first().copied());
                let new_id = state.scene_tree.add_node(label, parent);
                // Record undo command
                let mut cm = std::mem::take(&mut state.command_manager);
                cm.execute(
                    Box::new(crate::commands::CreateNodeCommand::new(
                        label.to_string(),
                        parent,
                    )),
                    state,
                );
                state.command_manager = cm;
                // Initialize transform and component data based on type
                state
                    .node_transforms
                    .insert(new_id, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
                match j {
                    1 => {
                        // Cube
                        state
                            .node_render
                            .insert(new_id, ("Default".into(), "Cube".into(), true));
                        state
                            .node_materials
                            .insert(new_id, crate::state::MaterialData::default());
                    }
                    2 => {
                        // Sphere
                        state
                            .node_render
                            .insert(new_id, ("Default".into(), "Sphere".into(), true));
                        state.node_materials.insert(
                            new_id,
                            crate::state::MaterialData {
                                base_color: [0.2, 0.6, 1.0, 1.0],
                                metallic: 0.8,
                                roughness: 0.1,
                                ..Default::default()
                            },
                        );
                    }
                    3 => {
                        // Directional light
                        state
                            .node_lights
                            .insert(new_id, crate::state::LightData::default());
                    }
                    4 => {
                        // Point light
                        state.node_lights.insert(
                            new_id,
                            crate::state::LightData {
                                light_type: crate::state::LightType::Point,
                                color: [1.0, 1.0, 1.0],
                                intensity: 1.0,
                                range: 10.0,
                                ..Default::default()
                            },
                        );
                    }
                    5 => {
                        // Spot light
                        state.node_lights.insert(
                            new_id,
                            crate::state::LightData {
                                light_type: crate::state::LightType::Spot,
                                color: [1.0, 1.0, 1.0],
                                intensity: 1.0,
                                range: 15.0,
                                inner_angle: 15.0,
                                outer_angle: 30.0,
                                ..Default::default()
                            },
                        );
                    }
                    _ => {} // Empty node
                }
                state.show_create_menu = false;
                state.selected_nodes = vec![new_id];
            }
        }
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

    // Clear drag state if mouse released without dropping on a target
    if gui.ui.input(|i| i.pointer.any_released()) && state.drag_source.is_some() {
        state.drag_source = None;
        state.drag_hover_target = None;
    }

    // Reset hover target each frame (will be set by hovered nodes)
    if state.drag_source.is_some() {
        state.drag_hover_target = None;
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
    let response = gui
        .ui
        .interact(id_rect, id, egui::Sense::click_and_drag());

    let painter = gui.ui.painter_at(id_rect);
    let is_selected = state.selected_nodes.contains(&node_id);
    let is_search_match = search_results.contains(&node_id);
    let is_drag_hover = state.drag_hover_target == Some(node_id);
    let is_dragging = state.drag_source == Some(node_id);

    if is_selected {
        painter.add(Shape::rect_filled(
            id_rect,
            Rounding::same(rounding),
            Color32::from_rgba_premultiplied(0, 212, 170, 30),
        ));
    } else if is_drag_hover {
        // Drop target highlight
        painter.add(Shape::rect_filled(
            id_rect,
            Rounding::same(rounding),
            Color32::from_rgba_premultiplied(0, 212, 170, 60),
        ));
        painter.rect_stroke(
            id_rect,
            Rounding::same(rounding),
            Stroke::new(2.0_f32, Color32::from_rgb(0, 212, 170)),
        );
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

    // Drag source: dim the node being dragged
    if is_dragging {
        painter.add(Shape::rect_filled(
            id_rect,
            Rounding::same(rounding),
            Color32::from_rgba_premultiplied(0, 0, 0, 80),
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

    // Drag initiation
    if response.drag_started() {
        state.drag_source = Some(node_id);
    }

    // Drag hover detection
    if state.drag_source.is_some() && response.hovered() {
        state.drag_hover_target = Some(node_id);
    }

    // Drop: reparent dragged node to hovered node
    if response.hovered()
        && gui.ui.input(|i| i.pointer.any_released())
        && let Some(source_id) = state.drag_source
    {
        if source_id != node_id {
            // Check that target is not a descendant of source
            if !is_descendant(&state.scene_tree, node_id, source_id) {
                let old_parent = state
                    .scene_tree
                    .nodes
                    .iter()
                    .find(|n| n.id == source_id)
                    .and_then(|n| n.parent);
                state.scene_tree.reparent(source_id, Some(node_id));
                // Expand target to show the moved node
                if let Some(n) =
                    state.scene_tree.nodes.iter_mut().find(|n| n.id == node_id)
                {
                    n.expanded = true;
                }
                // Record command for undo (take command_manager out to avoid borrow conflict)
                let mut cm = std::mem::take(&mut state.command_manager);
                cm.execute(
                    Box::new(crate::commands::MoveEntityCommand::new(
                        source_id,
                        old_parent,
                        Some(node_id),
                    )),
                    state,
                );
                state.command_manager = cm;
            }
        }
        state.drag_source = None;
        state.drag_hover_target = None;
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
