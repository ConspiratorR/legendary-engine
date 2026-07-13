//! Scene hierarchy panel — displays the entity tree with drag-and-drop
//! selection and right-click context actions.

use crate::state::{ContextMenuState, EditorState};
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

/// Scene hierarchy panel — manages the entity tree display and interaction.
#[derive(Debug, Clone)]
pub struct HierarchyPanel {
    /// Currently selected node IDs.
    pub selected: Vec<u64>,
    /// Nodes currently being dragged.
    pub drag_source: Option<u64>,
    /// Node being hovered during drag.
    pub drag_hover_target: Option<u64>,
    /// Search filter text.
    pub search_text: String,
}

impl Default for HierarchyPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl HierarchyPanel {
    /// Creates a new hierarchy panel with empty selection.
    pub fn new() -> Self {
        Self {
            selected: Vec::new(),
            drag_source: None,
            drag_hover_target: None,
            search_text: String::new(),
        }
    }

    /// Get the currently selected node IDs.
    pub fn selected(&self) -> &[u64] {
        &self.selected
    }

    /// Set the selected node IDs.
    pub fn set_selected(&mut self, ids: Vec<u64>) {
        self.selected = ids;
    }

    /// Toggle the expanded state of a node in the scene tree.
    pub fn toggle_expanded(&self, tree: &mut crate::state::SceneTree, node_id: u64) {
        if let Some(node) = tree.nodes.iter_mut().find(|n| n.id == node_id) {
            node.expanded = !node.expanded;
        }
    }

    /// Render the entire hierarchy panel.
    pub fn render(&mut self, state: &mut EditorState, gui: &mut Gui, rect: Rect) {
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
                    state.show_create_menu = !state.show_create_menu;
                } else if i == 1 && !self.selected.is_empty() {
                    let to_delete: Vec<u64> = self.selected.clone();
                    for &node_id in &to_delete {
                        state.scene_tree.remove_node(node_id);
                    }
                    self.selected.clear();
                } else if i == 2 && !self.selected.is_empty() {
                    let name = state
                        .scene_tree
                        .nodes
                        .iter()
                        .find(|n| n.id == self.selected[0])
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

        if state.show_create_menu {
            let menu_x = rect.right() - 140.0 * w_scale;
            let menu_y = header_rect.bottom() + 2.0;
            let item_h = 28.0 * h_scale;
            let menu_w = 130.0 * w_scale;
            let menu_h = item_h * CREATE_TYPES.len() as f32;
            let menu_rect =
                Rect::from_min_size(Pos2::new(menu_x, menu_y), Vec2::new(menu_w, menu_h));
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
                let item_rect =
                    Rect::from_min_size(Pos2::new(menu_x, item_y), Vec2::new(menu_w, item_h));
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
                    let parent = self.selected.first().copied().or(state
                        .scene_tree
                        .root_ids
                        .first()
                        .copied());
                    let mut cm = std::mem::take(&mut state.command_manager);
                    cm.execute(
                        Box::new(crate::commands::CreateNodeCommand::new(
                            label.to_string(),
                            parent,
                        )),
                        state,
                    );
                    state.command_manager = cm;
                    let new_id = state.selected_nodes.first().copied().unwrap_or(0);
                    match j {
                        1 => {
                            state
                                .node_render
                                .insert(new_id, ("Default".into(), "Cube".into(), true));
                            state
                                .node_materials
                                .insert(new_id, crate::state::MaterialData::default());
                        }
                        2 => {
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
                            state
                                .node_lights
                                .insert(new_id, crate::state::LightData::default());
                        }
                        4 => {
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
                        _ => {}
                    }
                    state.show_create_menu = false;
                    self.selected = vec![new_id];
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

        let search_text = if self.search_text.is_empty() {
            "🔍 搜索...".to_string()
        } else {
            format!("🔍 {}", self.search_text)
        };

        painter.text(
            egui::pos2(search_rect.left() + 8.0 * w_scale, search_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &search_text,
            FontId::proportional(12.0 * h_scale),
            Color32::from_gray(90),
        );

        if !self.search_text.is_empty() {
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
                self.search_text.clear();
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

        let search_results = if !self.search_text.is_empty() {
            state.scene_tree.search(&self.search_text)
        } else {
            Vec::new()
        };

        let root_ids: Vec<u64> = state.scene_tree.root_ids.clone();
        for &root_id in &root_ids {
            self.render_gameobject(
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

        if gui.ui.input(|i| i.pointer.any_released()) && self.drag_source.is_some() {
            self.drag_source = None;
            self.drag_hover_target = None;
        }

        if self.drag_source.is_some() {
            self.drag_hover_target = None;
        }

        if state.context_menu.is_some() {
            draw_context_menu(state, self, gui, rect, h_scale, w_scale);
        }
    }

    /// Render a single game object (node) in the hierarchy tree.
    #[allow(clippy::too_many_arguments)]
    pub fn render_gameobject(
        &mut self,
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
    ) {
        let (node_name, node_icon, node_expanded, children) = {
            let node = match state.scene_tree.nodes.iter().find(|n| n.id == node_id) {
                Some(n) => n,
                None => return,
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
        let response = gui.ui.interact(id_rect, id, egui::Sense::click_and_drag());

        let painter = gui.ui.painter_at(id_rect);
        let is_selected = self.selected.contains(&node_id);
        let is_search_match = search_results.contains(&node_id);
        let is_drag_hover = self.drag_hover_target == Some(node_id);
        let is_dragging = self.drag_source == Some(node_id);

        if is_selected {
            painter.add(Shape::rect_filled(
                id_rect,
                Rounding::same(rounding),
                Color32::from_rgba_premultiplied(0, 212, 170, 30),
            ));
        } else if is_drag_hover {
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
            self.selected.clear();
            self.selected.push(node_id);
        }

        if response.secondary_clicked() {
            self.selected.clear();
            self.selected.push(node_id);
            let mouse_pos = gui.ui.input(|i| i.pointer.interact_pos());
            state.context_menu = Some(ContextMenuState {
                position: mouse_pos.unwrap_or(egui::pos2(id_rect.center().x, id_rect.center().y)),
                node_id,
                renaming: false,
                rename_buffer: String::new(),
            });
        }

        if response.drag_started() {
            self.drag_source = Some(node_id);
        }

        if self.drag_source.is_some() && response.hovered() {
            self.drag_hover_target = Some(node_id);
        }

        if response.hovered()
            && gui.ui.input(|i| i.pointer.any_released())
            && let Some(source_id) = self.drag_source
        {
            if source_id != node_id && !is_descendant(&state.scene_tree, node_id, source_id) {
                let old_parent = state
                    .scene_tree
                    .nodes
                    .iter()
                    .find(|n| n.id == source_id)
                    .and_then(|n| n.parent);
                state.scene_tree.reparent(source_id, Some(node_id));
                if let Some(n) = state.scene_tree.nodes.iter_mut().find(|n| n.id == node_id) {
                    n.expanded = true;
                }
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
            self.drag_source = None;
            self.drag_hover_target = None;
        }

        *y += item_h;

        if node_expanded || !search_results.is_empty() {
            for &child_id in &children {
                self.render_gameobject(
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
    }
}

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

/// Draw the context menu for the hierarchy panel.
fn draw_context_menu(
    state: &mut EditorState,
    panel: &mut HierarchyPanel,
    gui: &mut Gui,
    _rect: Rect,
    h_scale: f32,
    w_scale: f32,
) {
    let Some(menu) = &state.context_menu else {
        return;
    };

    let node_id = menu.node_id;
    let menu_pos = menu.position;

    let items: Vec<(&str, bool)> = if menu.renaming {
        vec![]
    } else {
        vec![
            ("重命名", true),
            ("复制", true),
            ("粘贴", !state.clipboard.is_empty()),
            ("剪切", true),
            ("删除", true),
            ("复制节点", true),
            ("聚焦对象", true),
            ("─", true),
            ("创建子节点", true),
            ("创建立方体", true),
            ("创建球体", true),
            ("创建光源", true),
        ]
    };

    let item_h = 28.0 * h_scale;
    let menu_w = 160.0 * w_scale;
    let menu_h = if menu.renaming {
        40.0 * h_scale
    } else {
        item_h * items.len() as f32
    };

    let screen = gui.ui.ctx().screen_rect();
    let mut menu_x = menu_pos.x;
    let mut menu_y = menu_pos.y;
    if menu_x + menu_w > screen.right() {
        menu_x = screen.right() - menu_w - 4.0;
    }
    if menu_y + menu_h > screen.bottom() {
        menu_y = screen.bottom() - menu_h - 4.0;
    }

    let menu_rect = Rect::from_min_size(Pos2::new(menu_x, menu_y), Vec2::new(menu_w, menu_h));

    let bg_response = gui.ui.interact(
        gui.ui.ctx().screen_rect(),
        egui::Id::new("ctx_menu_bg"),
        egui::Sense::click(),
    );
    if bg_response.clicked()
        && !menu_rect.contains(bg_response.interact_pointer_pos().unwrap_or_default())
    {
        state.context_menu = None;
        return;
    }

    let painter = gui.ui.painter_at(menu_rect);
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

    if menu.renaming {
        let input_rect = Rect::from_min_size(
            Pos2::new(menu_x + 8.0 * w_scale, menu_y + 6.0 * h_scale),
            Vec2::new(menu_w - 16.0 * w_scale, 28.0 * h_scale),
        );
        painter.add(Shape::rect_filled(
            input_rect,
            Rounding::same(4.0),
            Color32::from_rgb(30, 30, 34),
        ));
        painter.rect_stroke(
            input_rect,
            Rounding::same(4.0),
            Stroke::new(1.0_f32, Color32::from_rgb(0, 212, 170)),
        );

        let rename_id = egui::Id::new("ctx_rename");
        let response = gui.ui.interact(input_rect, rename_id, egui::Sense::click());

        if state
            .context_menu
            .as_ref()
            .is_some_and(|m| m.rename_buffer.is_empty())
        {
            let name = state
                .scene_tree
                .nodes
                .iter()
                .find(|n| n.id == node_id)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            if let Some(menu) = &mut state.context_menu {
                menu.rename_buffer = name;
            }
        }

        if response.clicked() {
            gui.ui.ctx().memory_mut(|m| m.request_focus(rename_id));
        }

        let has_focus = gui.ui.ctx().memory(|m| m.has_focus(rename_id));
        let mut close_menu = false;
        let mut apply_rename = false;

        if has_focus {
            let events: Vec<egui::Event> = gui.ui.ctx().input(|i| i.events.clone());
            for event in &events {
                match event {
                    egui::Event::Text(text) => {
                        if let Some(menu) = &mut state.context_menu {
                            menu.rename_buffer.push_str(text);
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } => {
                        if let Some(menu) = &mut state.context_menu {
                            menu.rename_buffer.pop();
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::Enter,
                        pressed: true,
                        ..
                    } => {
                        apply_rename = true;
                    }
                    egui::Event::Key {
                        key: egui::Key::Escape,
                        pressed: true,
                        ..
                    } => {
                        close_menu = true;
                    }
                    _ => {}
                }
            }
        }

        if apply_rename {
            if let Some(menu) = &state.context_menu
                && !menu.rename_buffer.is_empty()
            {
                state
                    .scene_tree
                    .rename(node_id, &menu.rename_buffer.clone());
            }
            close_menu = true;
        }

        if close_menu {
            state.context_menu = None;
            return;
        }

        let display_text = state
            .context_menu
            .as_ref()
            .map(|m| m.rename_buffer.clone())
            .unwrap_or_default();
        painter.text(
            input_rect.center(),
            egui::Align2::CENTER_CENTER,
            &display_text,
            FontId::proportional(12.0 * h_scale),
            Color32::from_rgb(232, 232, 236),
        );

        if has_focus {
            let cursor_x = input_rect.left()
                + 8.0 * w_scale
                + painter
                    .layout(
                        display_text,
                        FontId::proportional(12.0 * h_scale),
                        Color32::WHITE,
                        input_rect.width() - 16.0 * w_scale,
                    )
                    .size()
                    .x;
            painter.add(Shape::line(
                vec![
                    Pos2::new(cursor_x, input_rect.top() + 4.0),
                    Pos2::new(cursor_x, input_rect.bottom() - 4.0),
                ],
                Stroke::new(1.0_f32, Color32::from_rgb(0, 212, 170)),
            ));
        }
        return;
    }

    for (j, (label, enabled)) in items.iter().enumerate() {
        if *label == "─" {
            let sep_y = menu_y + j as f32 * item_h + item_h / 2.0;
            painter.add(Shape::line(
                vec![
                    Pos2::new(menu_x + 8.0, sep_y),
                    Pos2::new(menu_x + menu_w - 8.0, sep_y),
                ],
                Stroke::new(1.0_f32, Color32::from_rgb(55, 55, 60)),
            ));
            continue;
        }

        let item_y = menu_y + j as f32 * item_h;
        let item_rect = Rect::from_min_size(Pos2::new(menu_x, item_y), Vec2::new(menu_w, item_h));
        let item_id = egui::Id::new("ctx_item").with(j as u64);
        let item_resp = gui.ui.interact(item_rect, item_id, egui::Sense::click());

        if item_resp.hovered() && *enabled {
            painter.add(Shape::rect_filled(
                item_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 80, 60),
            ));
        }

        let text_color = if *enabled {
            Color32::from_gray(200)
        } else {
            Color32::from_gray(60)
        };
        painter.text(
            Pos2::new(menu_x + 12.0 * w_scale, item_rect.center().y),
            egui::Align2::LEFT_CENTER,
            *label,
            FontId::proportional(12.0 * h_scale),
            text_color,
        );

        if item_resp.clicked() && *enabled {
            match *label {
                "重命名" => {
                    if let Some(menu) = &mut state.context_menu {
                        menu.renaming = true;
                        menu.rename_buffer = state
                            .scene_tree
                            .nodes
                            .iter()
                            .find(|n| n.id == node_id)
                            .map(|n| n.name.clone())
                            .unwrap_or_default();
                    }
                }
                "复制" => {
                    state.copy_selected();
                }
                "粘贴" => {
                    state.paste();
                }
                "剪切" => {
                    state.cut_selected();
                }
                "删除" => {
                    state.delete_selected();
                }
                "复制节点" => {
                    state.duplicate_selected();
                }
                "聚焦对象" => {
                    state.focus_on_selection();
                }
                "创建子节点" => {
                    let new_id = state.scene_tree.add_node("新节点", Some(node_id));
                    state
                        .node_transforms
                        .insert(new_id, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
                    panel.selected = vec![new_id];
                }
                "创建立方体" => {
                    let new_id = state.scene_tree.add_node("立方体", Some(node_id));
                    state
                        .node_transforms
                        .insert(new_id, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
                    state
                        .node_render
                        .insert(new_id, ("Default".into(), "Cube".into(), true));
                    state
                        .node_materials
                        .insert(new_id, crate::state::MaterialData::default());
                    panel.selected = vec![new_id];
                }
                "创建球体" => {
                    let new_id = state.scene_tree.add_node("球体", Some(node_id));
                    state
                        .node_transforms
                        .insert(new_id, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
                    state
                        .node_render
                        .insert(new_id, ("Default".into(), "Sphere".into(), true));
                    state
                        .node_materials
                        .insert(new_id, crate::state::MaterialData::default());
                    panel.selected = vec![new_id];
                }
                "创建光源" => {
                    let new_id = state.scene_tree.add_node("光源", Some(node_id));
                    state
                        .node_lights
                        .insert(new_id, crate::state::LightData::default());
                    panel.selected = vec![new_id];
                }
                _ => {}
            }
            state.context_menu = None;
        }
    }
}

/// Draw the hierarchy panel using the legacy function interface.
pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let mut panel = std::mem::take(&mut state.hierarchy_panel);
    panel.selected = state.selected_nodes.clone();
    panel.render(state, gui, rect);
    state.selected_nodes = panel.selected.clone();
    state.hierarchy_panel = panel;
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_ui::{Gui, GuiSkin};

    #[test]
    fn test_hierarchy_panel_new() {
        let panel = HierarchyPanel::new();
        assert!(panel.selected.is_empty());
        assert!(panel.drag_source.is_none());
        assert!(panel.drag_hover_target.is_none());
        assert!(panel.search_text.is_empty());
    }

    #[test]
    fn test_selected_and_set_selected() {
        let mut panel = HierarchyPanel::new();
        assert!(panel.selected().is_empty());

        panel.set_selected(vec![1, 2, 3]);
        assert_eq!(panel.selected(), &[1, 2, 3]);
    }

    #[test]
    fn test_toggle_expanded() {
        let mut tree = crate::state::SceneTree::new();
        let node = tree.nodes.iter().find(|n| n.id == 2).unwrap();
        assert!(!node.expanded);

        let panel = HierarchyPanel::new();
        panel.toggle_expanded(&mut tree, 2);

        let node = tree.nodes.iter().find(|n| n.id == 2).unwrap();
        assert!(node.expanded);
    }

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
