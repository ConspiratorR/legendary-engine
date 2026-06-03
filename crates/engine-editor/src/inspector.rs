use crate::material_editor::MaterialEditorState;
use crate::state::{EditorState, LightType};
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
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
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.left(), rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let search_h = 36.0 * h_scale;
    let pad8 = 8.0 * w_scale;
    painter.add(Shape::rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left() + pad8, rect.top() + 8.0 * h_scale),
            Vec2::new(rect.width() - pad8 * 2.0, search_h),
        ),
        Rounding::same(6.0 * h_scale),
        Color32::from_rgb(30, 30, 34),
    ));
    painter.text(
        egui::pos2(
            rect.left() + 20.0 * w_scale,
            rect.top() + (8.0 * h_scale + search_h / 2.0),
        ),
        egui::Align2::LEFT_CENTER,
        "🔍 搜索属性...",
        egui::FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );

    let content_top = rect.top() + (8.0 * h_scale + search_h + 8.0 * h_scale);
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 12.0 * w_scale, content_top),
        Vec2::new(rect.width() - 24.0 * w_scale, rect.bottom() - content_top),
    );

    let selected_id = state.selected_nodes.first().copied();

    if let Some(id) = selected_id {
        let name = state
            .scene_tree
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "—".into());
        draw_transform_section(gui, content_rect, state, id, &name);
    } else {
        let painter = gui.ui.painter_at(content_rect);
        painter.text(
            content_rect.center(),
            egui::Align2::CENTER_CENTER,
            "未选中对象",
            egui::FontId::proportional(12.0),
            Color32::from_gray(90),
        );
    }

    // Terrain panel (shown when Terrain tool is active)
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

fn separator(painter: &egui::Painter, x: f32, y: f32, w: f32) -> f32 {
    painter.add(Shape::line(
        vec![Pos2::new(x, y), Pos2::new(x + w, y)],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    y + 8.0
}

fn section_header(painter: &egui::Painter, x: f32, y: f32, label: &str) -> f32 {
    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(11.0),
        Color32::from_gray(90),
    );
    y + 18.0
}

fn draw_transform_section(gui: &mut Gui, rect: Rect, state: &mut EditorState, id: u64, name: &str) {
    let painter = gui.ui.painter_at(rect);
    let row_h = 26.0;
    let x = rect.left();
    let w = rect.width();
    let mut y = rect.top();

    // Node name
    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(13.0),
        Color32::WHITE,
    );
    y += 24.0;

    // ── Transform ──
    if let Some(t) = state.node_transforms.get_mut(&id) {
        y = separator(&painter, x, y, w);
        y = section_header(&painter, x, y, "变换");

        let pr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut px, mut py, mut pz) = (t[0], t[1], t[2]);
        gui.vec3_input(pr, "位置", &mut px, &mut py, &mut pz);
        t[0] = px;
        t[1] = py;
        t[2] = pz;
        y += row_h + 6.0;

        let rr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut rx, mut ry, mut rz) = (t[3], t[4], t[5]);
        gui.vec3_input(rr, "旋转", &mut rx, &mut ry, &mut rz);
        t[3] = rx;
        t[4] = ry;
        t[5] = rz;
        y += row_h + 6.0;

        let sr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut sx, mut sy, mut sz) = (t[6], t[7], t[8]);
        gui.vec3_input(sr, "缩放", &mut sx, &mut sy, &mut sz);
        t[6] = sx;
        t[7] = sy;
        t[8] = sz;
        y += 16.0;
    }

    // ── Material (PBR) ──
    if let Some(mat) = state.node_materials.get_mut(&id) {
        y = separator(&painter, x, y, w);
        y = section_header(&painter, x, y, "材质 (PBR)");

        // Base color
        let cr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut r, mut g, mut b) = (mat.base_color[0], mat.base_color[1], mat.base_color[2]);
        gui.vec3_input(cr, "基础颜色", &mut r, &mut g, &mut b);
        mat.base_color[0] = r;
        mat.base_color[1] = g;
        mat.base_color[2] = b;
        y += row_h + 6.0;

        // Metallic slider
        let mr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(mr, "金属度", &mut mat.metallic, 0.0, 1.0);
        y += row_h + 6.0;

        // Roughness slider
        let rr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(rr, "粗糙度", &mut mat.roughness, 0.0, 1.0);
        y += row_h + 6.0;

        // AO slider
        let ar = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(ar, "环境光遮蔽", &mut mat.ao, 0.0, 1.0);
        y += row_h + 6.0;

        // Edit Material Graph button
        let btn_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 28.0));
        let btn_painter = gui.ui.painter_at(btn_rect);
        let btn_id = egui::Id::new("edit_mat_graph_btn").with(id);
        let btn_resp = gui.ui.interact(btn_rect, btn_id, egui::Sense::click());
        btn_painter.add(Shape::rect_filled(
            btn_rect,
            Rounding::same(4.0),
            if btn_resp.hovered() {
                Color32::from_rgb(0, 120, 180)
            } else {
                Color32::from_rgb(0, 90, 140)
            },
        ));
        btn_painter.text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "编辑材质图",
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );
        if btn_resp.clicked() {
            // Build a graph from the current material data and open the editor
            let graph = MaterialEditorState::graph_from_material(mat);
            state.node_graph_state.graph = graph;
            state.material_editor.open();
            state.material_editor.material_name = format!("材质 #{}", id);
        }
        y += 36.0;
    }

    // ── Render ──
    y = separator(&painter, x, y, w);
    y = section_header(&painter, x, y, "渲染");

    if let Some((mat, mesh, shadow)) = state.node_render.get_mut(&id) {
        let mr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(mr, "材质", mat);
        y += row_h + 6.0;

        let mer = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(mer, "网格", mesh);
        y += row_h + 6.0;

        let sr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(sr, "投射阴影", shadow);
        y += 16.0;
    }

    // ── Light ──
    if let Some(light) = state.node_lights.get_mut(&id) {
        y = separator(&painter, x, y, w);

        let type_label = match light.light_type {
            LightType::Directional => "光照 (方向光)",
            LightType::Point => "光照 (点光源)",
            LightType::Spot => "光照 (聚光灯)",
        };
        y = section_header(&painter, x, y, type_label);

        // Enabled
        let er = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(er, "启用", &mut light.enabled);
        y += row_h + 6.0;

        // Color
        let clr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut lr, mut lg, mut lb) = (light.color[0], light.color[1], light.color[2]);
        gui.vec3_input(clr, "颜色", &mut lr, &mut lg, &mut lb);
        light.color[0] = lr;
        light.color[1] = lg;
        light.color[2] = lb;
        y += row_h + 6.0;

        // Intensity
        let ir = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(ir, "强度", &mut light.intensity, 0.0, 10.0);
        y += row_h + 6.0;

        // Range (for point/spot)
        if light.light_type != LightType::Directional {
            let rr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
            gui.slider_f32(rr, "范围", &mut light.range, 0.0, 100.0);
            y += row_h + 6.0;
        }

        // Direction (for directional/spot)
        if light.light_type != LightType::Point {
            let dr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
            let (mut dx, mut dy, mut dz) =
                (light.direction[0], light.direction[1], light.direction[2]);
            gui.vec3_input(dr, "方向", &mut dx, &mut dy, &mut dz);
            light.direction[0] = dx;
            light.direction[1] = dy;
            light.direction[2] = dz;
            y += row_h + 6.0;
        }

        // Spot cone angles
        if light.light_type == LightType::Spot {
            let ir2 = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
            gui.slider_f32(ir2, "内角 (°)", &mut light.inner_angle, 0.0, 90.0);
            y += row_h + 6.0;

            let or = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
            gui.slider_f32(or, "外角 (°)", &mut light.outer_angle, 0.0, 90.0);
            y += row_h + 6.0;
        }

        y += 8.0;
    }

    // ── Physics ──
    y = separator(&painter, x, y, w);
    y = section_header(&painter, x, y, "物理");

    if let Some((body, col)) = state.node_physics.get_mut(&id) {
        let br = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(br, "刚体", body);
        y += row_h + 6.0;

        let cr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.input_labeled(cr, "碰撞", col);
    }
}
