//! Inspector panel — shows and edits properties of the selected entity,
//! including transform, light parameters, material assignments, and tags.

use crate::material_editor::MaterialEditorState;
use crate::state::{EditorState, LightType};
use egui::{Color32, FontId, Rounding, Stroke};
use engine_ui::Gui;

const SECTION_SPACING: f32 = 8.0;
const ROW_SPACING: f32 = 4.0;

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: egui::Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    // Background
    let painter = gui.ui.painter_at(rect);
    painter.add(egui::Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(egui::Shape::line(
        vec![
            egui::Pos2::new(rect.left(), rect.top()),
            egui::Pos2::new(rect.left(), rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    // Search bar
    let search_h = 28.0 * h_scale;
    let pad = 8.0 * w_scale;
    let search_rect = egui::Rect::from_min_size(
        egui::Pos2::new(rect.left() + pad, rect.top() + 6.0 * h_scale),
        egui::Vec2::new(rect.width() - pad * 2.0, search_h),
    );
    draw_search_bar(gui, state, search_rect, h_scale, w_scale);

    // Content area with scroll
    let content_top = search_rect.bottom() + 6.0 * h_scale;
    let content_rect = egui::Rect::from_min_size(
        egui::Pos2::new(rect.left(), content_top),
        egui::Vec2::new(rect.width(), rect.bottom() - content_top),
    );

    let selected_id = state.selected_nodes.first().copied();

    let mut content_ui = gui.ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content_rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(&mut content_ui, |ui| {
            let mut gui = Gui::new(ui, gui.skin);
            if let Some(id) = selected_id {
                draw_entity_inspector(&mut gui, state, id, h_scale, w_scale);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        egui::RichText::new("未选中对象")
                            .color(Color32::from_gray(90))
                            .size(12.0),
                    );
                });
            }
        });

    // Terrain panel overlay
    if state.active_tool == crate::state::ToolType::Terrain {
        egui::Area::new(egui::Id::new("terrain_panel"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 50.0))
            .show(gui.ui.ctx(), |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(30, 30, 35))
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_min_width(250.0);
                        let mut terrain = engine_terrain::components::Terrain::new(
                            128,
                            64,
                            engine_math::Vec2::new(100.0, 100.0),
                            50.0,
                        );
                        let mut texture_layers =
                            engine_terrain::components::TerrainTextureLayers::default();
                        let mut vegetation_data =
                            engine_terrain::components::VegetationData::default();
                        state.terrain_panel.draw(
                            ui,
                            &mut terrain,
                            &mut texture_layers,
                            &mut vegetation_data,
                        );
                    });
            });
    }
}

fn draw_search_bar(
    gui: &mut Gui,
    state: &mut EditorState,
    rect: egui::Rect,
    _h_scale: f32,
    w_scale: f32,
) {
    let painter = gui.ui.painter_at(rect);
    painter.add(egui::Shape::rect_filled(
        rect,
        Rounding::same(6.0),
        Color32::from_rgb(30, 30, 34),
    ));

    let search_id = egui::Id::new("inspector_search");
    let response = gui
        .ui
        .interact(rect, search_id, egui::Sense::click());
    if response.clicked() {
        gui.ui.ctx().memory_mut(|m| m.request_focus(search_id));
    }
    let has_focus = gui.ui.ctx().memory(|m| m.has_focus(search_id));

    if has_focus {
        gui.ui.ctx().input(|i| {
            for event in &i.events {
                if let egui::Event::Text(text) = event {
                    state.inspector_search.push_str(text);
                }
                if let egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                } = event
                {
                    state.inspector_search.pop();
                }
                if let egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } = event
                {
                    state.inspector_search.clear();
                    gui.ui.ctx().memory_mut(|m| m.surrender_focus(search_id));
                }
            }
        });
    }

    let display_text = if state.inspector_search.is_empty() && !has_focus {
        "搜索属性...".to_string()
    } else {
        state.inspector_search.clone()
    };
    let text_color = if has_focus {
        Color32::from_rgb(220, 220, 224)
    } else if state.inspector_search.is_empty() {
        Color32::from_gray(90)
    } else {
        Color32::from_rgb(0, 212, 170)
    };

    painter.text(
        egui::pos2(rect.left() + 12.0 * w_scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        display_text,
        egui::FontId::proportional(12.0),
        text_color,
    );

    // Clear button
    if !state.inspector_search.is_empty() {
        let clear_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.right() - 28.0 * w_scale, rect.top()),
            egui::Vec2::new(24.0 * w_scale, rect.height()),
        );
        let clear_id = egui::Id::new("inspector_clear_search");
        let clear_resp = gui.ui.interact(clear_rect, clear_id, egui::Sense::click());
        if clear_resp.hovered() {
            painter.add(egui::Shape::rect_filled(
                clear_rect,
                Rounding::same(4.0),
                Color32::from_rgb(40, 40, 44),
            ));
        }
        painter.text(
            clear_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✕",
            FontId::proportional(12.0),
            Color32::from_gray(90),
        );
        if clear_resp.clicked() {
            state.inspector_search.clear();
        }
    }
}

fn section_matches(section_name: &str, search: &str) -> bool {
    search.is_empty() || section_name.to_lowercase().contains(search)
}

fn draw_entity_inspector(
    gui: &mut Gui,
    state: &mut EditorState,
    id: u64,
    _h_scale: f32,
    _w_scale: f32,
) {
    let search_lower = state.inspector_search.to_lowercase();

    // Node name header
    let name = state
        .scene_tree
        .nodes
        .iter()
        .find(|n| n.id == id)
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "—".into());
    gui.ui.add_space(4.0);
    gui.ui.label(
        egui::RichText::new(format!("{} [{}]", name, id))
            .color(Color32::from_rgb(220, 220, 224))
            .size(13.0),
    );
    gui.ui.add_space(4.0);

    // ── Transform ──
    if section_matches(
        "变换 位置 旋转 缩放 transform position rotation scale",
        &search_lower,
    ) && let Some(t) = state.node_transforms.get_mut(&id)
    {
        let old_transform = *t;

        section_separator(gui);
        section_header(gui, "变换");

        let (mut px, mut py, mut pz) = (t[0], t[1], t[2]);
        vec3_row(gui, "位置", &mut px, &mut py, &mut pz);
        t[0] = px;
        t[1] = py;
        t[2] = pz;

        let (mut rx, mut ry, mut rz) = (t[3], t[4], t[5]);
        vec3_row(gui, "旋转", &mut rx, &mut ry, &mut rz);
        t[3] = rx;
        t[4] = ry;
        t[5] = rz;

        let (mut sx, mut sy, mut sz) = (t[6], t[7], t[8]);
        vec3_row(gui, "缩放", &mut sx, &mut sy, &mut sz);
        t[6] = sx;
        t[7] = sy;
        t[8] = sz;

        // Undo tracking
        let new_transform = *t;
        if new_transform != old_transform {
            if state.pending_transform_edit.is_none() {
                state.pending_transform_edit = Some((id, old_transform));
            }
        } else if let Some((pending_id, pending_old)) = state.pending_transform_edit.take()
            && pending_id == id
        {
            let mut cm = std::mem::take(&mut state.command_manager);
            cm.execute(
                Box::new(crate::commands::TransformEntityCommand::new(
                    id,
                    pending_old,
                    new_transform,
                )),
                state,
            );
            state.command_manager = cm;
        }
    }

    // ── Material (PBR) ──
    if section_matches("材质 pbr material 基础颜色 金属度 粗糙度", &search_lower)
        && let Some(mat) = state.node_materials.get_mut(&id)
    {
        section_separator(gui);
        section_header(gui, "材质 (PBR)");

        let (mut r, mut g, mut b) = (mat.base_color[0], mat.base_color[1], mat.base_color[2]);
        vec3_row(gui, "基础颜色", &mut r, &mut g, &mut b);
        mat.base_color[0] = r;
        mat.base_color[1] = g;
        mat.base_color[2] = b;

        slider_row(gui, "金属度", &mut mat.metallic, 0.0, 1.0);
        slider_row(gui, "粗糙度", &mut mat.roughness, 0.0, 1.0);
        slider_row(gui, "环境光遮蔽", &mut mat.ao, 0.0, 1.0);

        gui.ui.add_space(ROW_SPACING);
        if gui
            .ui
            .button("编辑材质图")
            .clicked()
        {
            let graph = MaterialEditorState::graph_from_material(mat);
            state.node_graph_state.graph = graph;
            state.material_editor.open();
            state.material_editor.material_name = format!("材质 #{}", id);
        }
    }

    // ── Render ──
    if section_matches("渲染 render 材质 网格 阴影 shadow mesh", &search_lower) {
        if let Some((mat, mesh, shadow)) = state.node_render.get_mut(&id) {
            section_separator(gui);
            section_header(gui, "渲染");

            read_only_row(gui, "材质", mat);

            // Mesh combo
            gui.ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("网格")
                        .color(Color32::from_gray(152))
                        .size(12.0),
                );
                egui::ComboBox::from_id_salt(egui::Id::new("mesh_combo").with(id))
                    .selected_text(mesh.as_str())
                    .show_ui(ui, |ui| {
                        for mt in ["Cube", "Sphere", "Plane", "Cylinder"] {
                            ui.selectable_value(mesh, mt.to_string(), mt);
                        }
                    });
            });
            gui.ui.add_space(ROW_SPACING);

            checkbox_row(gui, "投射阴影", shadow);
        }
    }

    // ── Light ──
    if section_matches(
        "光照 光 light 方向光 点光源 聚光灯 directional point spot",
        &search_lower,
    ) && let Some(light) = state.node_lights.get_mut(&id)
    {
        section_separator(gui);

        let type_label = match light.light_type {
            LightType::Directional => "光照 (方向光)",
            LightType::Point => "光照 (点光源)",
            LightType::Spot => "光照 (聚光灯)",
        };
        section_header(gui, type_label);

        checkbox_row(gui, "启用", &mut light.enabled);

        let (mut lr, mut lg, mut lb) = (light.color[0], light.color[1], light.color[2]);
        vec3_row(gui, "颜色", &mut lr, &mut lg, &mut lb);
        light.color[0] = lr;
        light.color[1] = lg;
        light.color[2] = lb;

        slider_row(gui, "强度", &mut light.intensity, 0.0, 10.0);

        if light.light_type != LightType::Directional {
            slider_row(gui, "范围", &mut light.range, 0.0, 100.0);
        }

        if light.light_type != LightType::Point {
            let (mut dx, mut dy, mut dz) =
                (light.direction[0], light.direction[1], light.direction[2]);
            vec3_row(gui, "方向", &mut dx, &mut dy, &mut dz);
            light.direction[0] = dx;
            light.direction[1] = dy;
            light.direction[2] = dz;
        }

        if light.light_type == LightType::Spot {
            slider_row(gui, "内角 (°)", &mut light.inner_angle, 0.0, 90.0);
            slider_row(gui, "外角 (°)", &mut light.outer_angle, 0.0, 90.0);
        }
    }

    // ── Physics ──
    if section_matches("物理 physics 刚体 碰撞 rigidbody collider", &search_lower) {
        if let Some((body, col)) = state.node_physics.get_mut(&id) {
            section_separator(gui);
            section_header(gui, "物理");

            read_only_row(gui, "刚体", body);
            read_only_row(gui, "碰撞", col);
        }
    }

    // ── Sprite ──
    if section_matches("精灵 sprite 纹理 翻转", &search_lower)
        && let Some(sprite) = state.node_sprites.get_mut(&id)
    {
        section_separator(gui);
        section_header(gui, "精灵");

        text_input_row(gui, "纹理", &mut sprite.texture);
        slider_row(gui, "宽度", &mut sprite.size[0], 0.0, 100.0);
        slider_row(gui, "高度", &mut sprite.size[1], 0.0, 100.0);

        let (mut r, mut g, mut b) = (sprite.color[0], sprite.color[1], sprite.color[2]);
        vec3_row(gui, "颜色", &mut r, &mut g, &mut b);
        sprite.color[0] = r;
        sprite.color[1] = g;
        sprite.color[2] = b;

        checkbox_row(gui, "水平翻转", &mut sprite.flip_x);
        checkbox_row(gui, "垂直翻转", &mut sprite.flip_y);
    }

    // ── Particle ──
    if section_matches("粒子 particle 发射器 粒子系统", &search_lower)
        && let Some(particle) = state.node_particles.get_mut(&id)
    {
        section_separator(gui);
        section_header(gui, "粒子系统");

        text_input_row(gui, "发射器", &mut particle.emitter_type);
        slider_row(gui, "发射速率", &mut particle.rate, 0.0, 100.0);
        slider_row(gui, "生命周期", &mut particle.lifetime, 0.1, 10.0);
        slider_row(gui, "速度", &mut particle.speed, 0.0, 50.0);
        slider_row(gui, "起始大小", &mut particle.size_start, 0.0, 10.0);
        slider_row(gui, "结束大小", &mut particle.size_end, 0.0, 10.0);
    }

    // ── Audio ──
    if section_matches("音频 audio 声音 音量", &search_lower)
        && let Some(audio) = state.node_audio.get_mut(&id)
    {
        section_separator(gui);
        section_header(gui, "音频");

        text_input_row(gui, "音频源", &mut audio.source);
        slider_row(gui, "音量", &mut audio.volume, 0.0, 1.0);
        checkbox_row(gui, "循环", &mut audio.looping);
        checkbox_row(gui, "空间音频", &mut audio.spatial);
    }

    // ── Script ──
    if section_matches("脚本 script lua wasm", &search_lower)
        && let Some(script) = state.node_scripts.get_mut(&id)
    {
        section_separator(gui);
        section_header(gui, "脚本");

        text_input_row(gui, "脚本路径", &mut script.script_path);
        checkbox_row(gui, "启用", &mut script.enabled);
    }

    // ── Tags ──
    if section_matches("标签 tags tag", &search_lower)
        && let Some(tags) = state.node_tags.get_mut(&id)
    {
        section_separator(gui);
        section_header(gui, "标签");

        let tag_str = tags.join(", ");
        let mut tag_edit = tag_str.clone();
        text_input_row(gui, "标签", &mut tag_edit);
        if tag_edit != tag_str {
            *tags = tag_edit
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    // ── Action buttons ──
    gui.ui.add_space(SECTION_SPACING);
    section_separator(gui);
    gui.ui.add_space(SECTION_SPACING);

    gui.ui.horizontal(|ui| {
        let half_w = (ui.available_width() - 4.0) / 2.0;

        let add_btn = ui.allocate_ui_with_layout(
            egui::Vec2::new(half_w, 28.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(half_w);
                ui.selectable_label(false, "+ 添加组件")
            },
        );
        if add_btn.inner.clicked() {
            state.show_add_component_menu = !state.show_add_component_menu;
            state.show_remove_component_menu = false;
        }

        let rm_btn = ui.allocate_ui_with_layout(
            egui::Vec2::new(half_w, 28.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(half_w);
                ui.selectable_label(false, "- 移除组件")
            },
        );
        if rm_btn.inner.clicked() {
            state.show_remove_component_menu = !state.show_remove_component_menu;
            state.show_add_component_menu = false;
        }
    });

    // Add Component dropdown
    if state.show_add_component_menu {
        let component_types = [
            ("材质 (PBR)", "material"),
            ("光照", "light"),
            ("精灵", "sprite"),
            ("粒子系统", "particle"),
            ("音频", "audio"),
            ("脚本", "script"),
            ("物理", "physics"),
            ("标签", "tags"),
        ];
        egui::Frame::default()
            .fill(Color32::from_rgb(35, 35, 40))
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::same(4.0))
            .show(gui.ui, |ui| {
                for (label, comp_type) in &component_types {
                    if ui
                        .selectable_label(false, *label)
                        .clicked()
                    {
                        add_component_to_node(state, id, comp_type);
                        state.show_add_component_menu = false;
                    }
                }
            });
    }

    // Remove Component dropdown
    if state.show_remove_component_menu {
        let existing_components: Vec<(&str, bool)> = vec![
            ("材质 (PBR)", state.node_materials.contains_key(&id)),
            ("渲染", state.node_render.contains_key(&id)),
            ("光照", state.node_lights.contains_key(&id)),
            ("物理", state.node_physics.contains_key(&id)),
            ("精灵", state.node_sprites.contains_key(&id)),
            ("粒子系统", state.node_particles.contains_key(&id)),
            ("音频", state.node_audio.contains_key(&id)),
            ("脚本", state.node_scripts.contains_key(&id)),
            ("标签", state.node_tags.contains_key(&id)),
        ];
        let comp_types = [
            "material", "render", "light", "physics", "sprite", "particle", "audio", "script",
            "tags",
        ];

        egui::Frame::default()
            .fill(Color32::from_rgb(50, 25, 25))
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::same(4.0))
            .show(gui.ui, |ui| {
                for ((label, exists), comp_type) in
                    existing_components.iter().zip(comp_types.iter())
                {
                    let text = if *exists {
                        format!("× {}", label)
                    } else {
                        format!("  {}", label)
                    };
                    let resp = ui.add_enabled(
                        *exists,
                        egui::SelectableLabel::new(false, text),
                    );
                    if *exists && resp.clicked() {
                        remove_component(state, id, comp_type);
                        state.log_info(&format!("已移除 {} 组件", label));
                        state.show_remove_component_menu = false;
                    }
                }
            });
    }
}

// ── Widget helpers ──

fn section_separator(gui: &mut Gui) {
    gui.ui.add_space(SECTION_SPACING);
    let rect = gui.ui.available_rect_before_wrap();
    let painter = gui.ui.painter_at(rect);
    let y = rect.top();
    painter.add(egui::Shape::line(
        vec![
            egui::Pos2::new(rect.left(), y),
            egui::Pos2::new(rect.right(), y),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    gui.ui.add_space(SECTION_SPACING);
}

fn section_header(gui: &mut Gui, label: &str) {
    gui.ui.label(
        egui::RichText::new(label)
            .color(Color32::from_gray(90))
            .size(11.0),
    );
    gui.ui.add_space(ROW_SPACING);
}

fn vec3_row(gui: &mut Gui, label: &str, x: &mut f32, y: &mut f32, z: &mut f32) {
    let axis_colors = [
        Color32::from_rgb(255, 107, 107),
        Color32::from_rgb(46, 213, 115),
        Color32::from_rgb(77, 171, 247),
    ];

    gui.ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .color(Color32::from_gray(152))
                .size(12.0),
        );

        for (val, (axis_label, acolor)) in [
            (x, ("X", axis_colors[0])),
            (y, ("Y", axis_colors[1])),
            (z, ("Z", axis_colors[2])),
        ] {
            let mut buf = format!("{:.1}", *val);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(axis_label)
                        .color(acolor)
                        .size(10.0),
                );
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut buf)
                        .desired_width(48.0)
                        .margin(egui::Margin::symmetric(4.0, 2.0)),
                );
                if resp.lost_focus() {
                    if let Ok(v) = buf.trim().parse::<f32>() {
                        *val = v;
                    }
                }
            });
        }
    });
    gui.ui.add_space(ROW_SPACING);
}

fn slider_row(gui: &mut Gui, label: &str, value: &mut f32, min: f32, max: f32) {
    gui.ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .color(Color32::from_gray(152))
                .size(12.0),
        );
        ui.add(egui::Slider::new(value, min..=max).show_value(true));
    });
    gui.ui.add_space(ROW_SPACING);
}

fn checkbox_row(gui: &mut Gui, label: &str, checked: &mut bool) {
    gui.ui.horizontal(|ui| {
        ui.checkbox(checked, label);
    });
    gui.ui.add_space(ROW_SPACING);
}

fn read_only_row(gui: &mut Gui, label: &str, value: &str) {
    gui.ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .color(Color32::from_gray(152))
                .size(12.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut value.to_string())
                .desired_width(ui.available_width())
                .interactive(false),
        );
    });
    gui.ui.add_space(ROW_SPACING);
}

fn text_input_row(gui: &mut Gui, label: &str, value: &mut String) {
    gui.ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .color(Color32::from_gray(152))
                .size(12.0),
        );
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(ui.available_width()),
        );
    });
    gui.ui.add_space(ROW_SPACING);
}

fn remove_component(state: &mut EditorState, id: u64, comp_type: &str) {
    match comp_type {
        "material" => {
            state.node_materials.remove(&id);
        }
        "render" => {
            state.node_render.remove(&id);
        }
        "light" => {
            state.node_lights.remove(&id);
        }
        "physics" => {
            state.node_physics.remove(&id);
        }
        "sprite" => {
            state.node_sprites.remove(&id);
        }
        "particle" => {
            state.node_particles.remove(&id);
        }
        "audio" => {
            state.node_audio.remove(&id);
        }
        "script" => {
            state.node_scripts.remove(&id);
        }
        "tags" => {
            state.node_tags.remove(&id);
        }
        _ => {}
    }
}

fn add_component_to_node(state: &mut EditorState, node_id: u64, comp_type: &str) {
    match comp_type {
        "material" => {
            state.node_materials.entry(node_id).or_default();
            state.log_info(&format!("已添加材质组件到节点 {}", node_id));
        }
        "light" => {
            state.node_lights.entry(node_id).or_default();
            state.log_info(&format!("已添加光照组件到节点 {}", node_id));
        }
        "sprite" => {
            state.node_sprites.entry(node_id).or_default();
            state.log_info(&format!("已添加精灵组件到节点 {}", node_id));
        }
        "particle" => {
            state.node_particles.entry(node_id).or_default();
            state.log_info(&format!("已添加粒子组件到节点 {}", node_id));
        }
        "audio" => {
            state.node_audio.entry(node_id).or_default();
            state.log_info(&format!("已添加音频组件到节点 {}", node_id));
        }
        "script" => {
            state.node_scripts.entry(node_id).or_default();
            state.log_info(&format!("已添加脚本组件到节点 {}", node_id));
        }
        "physics" => {
            state
                .node_physics
                .entry(node_id)
                .or_insert_with(|| ("Static".into(), "Box".into()));
            state.log_info(&format!("已添加物理组件到节点 {}", node_id));
        }
        "tags" => {
            state.node_tags.entry(node_id).or_default();
            state.log_info(&format!("已添加标签组件到节点 {}", node_id));
        }
        _ => {}
    }
}
