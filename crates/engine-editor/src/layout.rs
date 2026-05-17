use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::{Gui, GuiSkin};
use crate::state::{EditorState, ToolType};

pub fn frame(state: &mut EditorState, ctx: &egui::Context, skin: &GuiSkin) {
    let screen_rect = ctx.screen_rect();
    let h_scale = screen_rect.height() / 1080.0;
    let w_scale = screen_rect.width() / 1920.0;

    egui::Area::new(egui::Id::new("editor"))
        .interactable(true)
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            let screen = ui.ctx().screen_rect();
            let menu_h = 32.0 * h_scale;
            let toolbar_h = 44.0 * h_scale;
            let status_h = 24.0 * h_scale;
            let bottom_h = (screen.height() * 180.0 / 1080.0).clamp(120.0, 400.0);

            let menu_rect = Rect::from_min_size(screen.left_top(), Vec2::new(screen.width(), menu_h));
            let toolbar_rect = Rect::from_min_size(
                Pos2::new(screen.left(), menu_rect.bottom()),
                Vec2::new(screen.width(), toolbar_h),
            );
            let status_rect = Rect::from_min_size(
                Pos2::new(screen.left(), screen.bottom() - status_h),
                Vec2::new(screen.width(), status_h),
            );
            let bottom_rect = Rect::from_min_size(
                Pos2::new(screen.left(), status_rect.top() - bottom_h),
                Vec2::new(screen.width(), bottom_h),
            );
            let main_rect = Rect::from_min_size(
                Pos2::new(screen.left(), toolbar_rect.bottom()),
                Vec2::new(screen.width(), bottom_rect.top() - toolbar_rect.bottom()),
            );

            let left_w = (main_rect.width() * 260.0 / 1920.0).clamp(180.0, 400.0);
            let right_w = (main_rect.width() * 300.0 / 1920.0).clamp(200.0, 500.0);

            let hierarchy_rect = Rect::from_min_size(
                main_rect.left_top(),
                Vec2::new(if state.show_left_panel { left_w } else { 0.0 }, main_rect.height()),
            );
            let inspector_rect = Rect::from_min_size(
                Pos2::new(main_rect.right() - (if state.show_right_panel { right_w } else { 0.0 }), main_rect.top()),
                Vec2::new(if state.show_right_panel { right_w } else { 0.0 }, main_rect.height()),
            );
            let viewport_rect = Rect::from_min_size(
                Pos2::new(hierarchy_rect.right(), main_rect.top()),
                Vec2::new(inspector_rect.left() - hierarchy_rect.right(), main_rect.height()),
            );

            let mut gui = Gui::new(ui, skin);
            draw_menu_bar(state, &mut gui, menu_rect, w_scale, h_scale);
            draw_toolbar(state, &mut gui, toolbar_rect, w_scale, h_scale);
            if state.show_left_panel {
                crate::hierarchy::draw(state, &mut gui, hierarchy_rect);
            }
            crate::viewport::draw(state, &mut gui, viewport_rect);
            if state.show_right_panel {
                crate::inspector::draw(state, &mut gui, inspector_rect);
            }
            draw_bottom_panel(state, ui, bottom_rect, h_scale, w_scale);
            draw_status_bar(state, &mut gui, status_rect, h_scale, w_scale);
        });
}

fn draw_menu_bar(state: &mut EditorState, gui: &mut Gui, rect: Rect, w_scale: f32, h_scale: f32) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    let items = &["文件", "编辑", "视图", "场景", "资源", "构建", "窗口", "帮助"];
    let font_sz = 13.0 * h_scale;
    let char_w = 8.0 * w_scale;
    let item_pad = 12.0 * w_scale;
    let rounding = 4.0 * h_scale;
    let mut x = rect.left() + 8.0 * w_scale;
    for (i, item) in items.iter().enumerate() {
        let text_w = item.len() as f32 * char_w;
        let item_rect = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(text_w + item_pad * 2.0, rect.height()));
        let id = egui::Id::new("mm").with(i as u64);
        let response = gui.ui.interact(item_rect, id, egui::Sense::click());
        if response.hovered() || state.active_menu == Some(i) {
            painter.add(Shape::rect_filled(item_rect, Rounding::same(rounding), Color32::from_rgb(30, 30, 34)));
        }
        painter.text(
            egui::pos2(x + item_pad, rect.center().y),
            egui::Align2::LEFT_CENTER,
            *item,
            FontId::proportional(font_sz),
            if response.hovered() { Color32::from_rgb(232, 232, 236) } else { Color32::from_gray(152) },
        );
        if response.clicked() {
            state.active_menu = Some(i);
        }
        x += text_w + item_pad * 2.0 + 4.0 * w_scale;
    }

    painter.text(
        egui::pos2(rect.right() - 12.0 * w_scale, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        "MyGame",
        FontId::proportional(font_sz),
        Color32::from_gray(152),
    );
}

fn draw_separator(painter: &egui::Painter, pos: f32, top: f32, bottom: f32, h_scale: f32) {
    let m = 8.0 * h_scale;
    painter.add(Shape::line(
        vec![Pos2::new(pos, top + m), Pos2::new(pos, bottom - m)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));
}

fn draw_toolbar(state: &mut EditorState, gui: &mut Gui, rect: Rect, w_scale: f32, h_scale: f32) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    let btn_size = 32.0 * h_scale;
    let gap = 4.0 * w_scale;
    let pad = 12.0 * w_scale;
    let mut x = rect.left() + pad;
    let cy = rect.top() + (rect.height() - btn_size) / 2.0;

    let tools = &["↖", "↔", "⟳", "⤢"];
    let tool_types = [ToolType::Select, ToolType::Translate, ToolType::Rotate, ToolType::Scale];
    for (i, tool) in tools.iter().enumerate() {
        let btn_rect = Rect::from_min_size(Pos2::new(x + i as f32 * (btn_size + gap), cy), Vec2::new(btn_size, btn_size));
        if gui.tool_button(btn_rect, tool, state.active_tool == tool_types[i]) {
            state.active_tool = tool_types[i];
        }
    }
    x += 4.0 * (btn_size + gap) + pad;
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;

    for (i, icon) in ["📁", "🔍"].iter().enumerate() {
        let btn_rect = Rect::from_min_size(Pos2::new(x + i as f32 * (btn_size + gap), cy), Vec2::new(btn_size, btn_size));
        if gui.tool_button(btn_rect, icon, false) {
            if i == 0 { state.show_left_panel = !state.show_left_panel; }
            if i == 1 { state.show_right_panel = !state.show_right_panel; }
        }
    }
    x += 2.0 * (btn_size + gap) + pad;
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;

    let modes = &["3D", "T", "F", "R"];
    for (i, mode) in modes.iter().enumerate() {
        let btn_rect = Rect::from_min_size(Pos2::new(x + i as f32 * (btn_size + gap), cy), Vec2::new(btn_size, btn_size));
        gui.tool_button(btn_rect, mode, state.active_viewport_tab == i);
    }
    x += 4.0 * (btn_size + gap) + pad;
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;

    for icon in ["▶", "⏸", "⏹"].iter() {
        let btn_rect = Rect::from_min_size(Pos2::new(x, cy), Vec2::new(btn_size, btn_size));
        gui.tool_button(btn_rect, icon, false);
        x += btn_size + gap;
    }
    x += 8.0 * w_scale;
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;

    painter.text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("FPS: {}", state.fps),
        FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );
}

fn draw_bottom_panel(state: &mut EditorState, ui: &egui::Ui, rect: Rect, h_scale: f32, w_scale: f32) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.top()), Pos2::new(rect.right(), rect.top())],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    let tab_h = 32.0 * h_scale;
    let tab_bar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), tab_h));
    let tabs = &["日志", "性能", "音频", "网络"];
    let tab_font = 12.0 * h_scale;
    let char_w = 8.0 * w_scale;
    let mut tx = rect.left() + 8.0 * w_scale;
    for (i, label) in tabs.iter().enumerate() {
        let text_w = label.len() as f32 * char_w;
        let tab_rect = Rect::from_min_size(Pos2::new(tx, rect.top()), Vec2::new(text_w + 28.0 * w_scale, tab_h));
        let id = egui::Id::new("btm_tab").with(i as u64);
        let response = ui.interact(tab_rect, id, egui::Sense::click());
        if state.active_bottom_tab == i {
            let line_rect = Rect::from_min_size(Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0 * h_scale), Vec2::new(tab_rect.width(), 2.0 * h_scale));
            painter.add(Shape::rect_filled(line_rect, Rounding::ZERO, Color32::from_rgb(0, 212, 170)));
            painter.text(tab_rect.center(), egui::Align2::CENTER_CENTER, *label, FontId::proportional(tab_font), Color32::from_rgb(0, 212, 170));
        } else {
            painter.text(tab_rect.center(), egui::Align2::CENTER_CENTER, *label, FontId::proportional(tab_font), Color32::from_gray(90));
        }
        if response.clicked() {
            state.active_bottom_tab = i;
        }
        tx += text_w + 28.0 * w_scale;
    }

    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 12.0 * w_scale, tab_bar_rect.bottom()),
        Vec2::new(rect.width() - 24.0 * w_scale, rect.bottom() - tab_bar_rect.bottom()),
    );

    let log_font = 11.0 * h_scale;
    let log_step = 18.0 * h_scale;
    match state.active_bottom_tab {
        0 => {
            let logs = [
                ("10:23:15", "info", "编辑器已启动"),
                ("10:23:16", "info", "项目已加载: MyGame"),
                ("10:23:18", "info", "着色器编译完成 (12个)"),
                ("10:23:20", "warn", "缺少法线贴图: Materials/Wood"),
                ("10:23:22", "info", "场景保存成功"),
            ];
            let mut y = content_rect.top() + 8.0 * h_scale;
            for (time, level, msg) in &logs {
                let time_color = Color32::from_gray(90);
                let level_color = match *level {
                    "info" => Color32::from_gray(152),
                    "warn" => Color32::from_rgb(255, 184, 0),
                    _ => Color32::from_rgb(255, 71, 87),
                };
                painter.text(egui::pos2(content_rect.left(), y), egui::Align2::LEFT_CENTER, *time, FontId::proportional(log_font), time_color);
                painter.text(egui::pos2(content_rect.left() + 60.0 * w_scale, y), egui::Align2::LEFT_CENTER, *level, FontId::proportional(log_font), level_color);
                painter.text(egui::pos2(content_rect.left() + 110.0 * w_scale, y), egui::Align2::LEFT_CENTER, *msg, FontId::proportional(log_font), Color32::from_rgb(232, 232, 236));
                y += log_step;
            }
        }
        1 => {
            let perf_data = [
                "Draw Calls: 128",
                "Triangles: 45.2K",
                "Vertices: 22.8K",
                "GPU: 32ms",
                "Memory: 256MB / 2GB",
            ];
            let mut y = content_rect.top() + 8.0 * h_scale;
            for line in &perf_data {
                painter.text(egui::pos2(content_rect.left(), y), egui::Align2::LEFT_CENTER, *line, FontId::proportional(log_font), Color32::from_rgb(232, 232, 236));
                y += log_step;
            }
        }
        _ => {
            painter.text(content_rect.center(), egui::Align2::CENTER_CENTER, "-- 面板内容 --", FontId::proportional(log_font), Color32::from_gray(90));
        }
    }
}

fn draw_status_bar(state: &EditorState, gui: &mut Gui, rect: Rect, h_scale: f32, w_scale: f32) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(30, 30, 34)));
    painter.add(Shape::line(
        vec![Pos2::new(rect.left(), rect.top()), Pos2::new(rect.right(), rect.top())],
        Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
    ));

    let status_font = 11.0 * h_scale;
    let pad12 = 12.0 * w_scale;
    gui.status_item(
        Rect::from_min_size(Pos2::new(rect.left() + pad12, rect.top()), Vec2::new(60.0 * w_scale, rect.height())),
        "就绪",
        Color32::from_rgb(46, 213, 115),
    );

    painter.text(
        egui::pos2(rect.left() + 80.0 * w_scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("对象: {}", state.scene_tree.nodes.len()),
        FontId::proportional(status_font),
        Color32::from_gray(90),
    );

    painter.text(
        egui::pos2(rect.left() + 160.0 * w_scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "三角形: 45K",
        FontId::proportional(status_font),
        Color32::from_gray(90),
    );

    let view_modes = ["场景", "游戏", "物理"];
    let view_names = ["perspective", "top", "front", "right"];
    let view_mode = view_modes.get(state.active_viewport_tab).unwrap_or(&"场景");
    let view_name = view_names.first().unwrap_or(&"perspective");
    painter.text(
        egui::pos2(rect.right() - pad12, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{} 视图  |  {}", view_mode, view_name),
        FontId::proportional(status_font),
        Color32::from_gray(90),
    );
}
