//! Top-level editor layout — composes all panels into the main window with
//! menu bar, toolbar, and dockable panel regions.

use crate::state::{EditorState, PlayState, ToolType};
use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::{Gui, GuiSkin};

pub fn frame(
    state: &mut EditorState,
    ctx: &egui::Context,
    skin: &GuiSkin,
    renderer: &mut engine_render::renderer::Renderer,
    vp_renderer: &mut crate::viewport_renderer::ViewportRenderer,
    egui_state: &mut engine_ui::EguiState,
) {
    // Top menu bar
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        let rect = ui.max_rect();
        let mut gui = Gui::new(ui, skin);
        draw_menu_bar(state, &mut gui, rect);
    });

    // Toolbar below menu
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        let rect = ui.max_rect();
        let mut gui = Gui::new(ui, skin);
        draw_toolbar(state, &mut gui, rect);
    });

    // Bottom panel (console/logs)
    egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .default_height(180.0)
        .show(ctx, |ui| {
            draw_bottom_panel(state, ui);
        });

    // Status bar
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        draw_status_bar(state, ui);
    });

    // Left panel (hierarchy)
    if state.show_left_panel {
        egui::SidePanel::left("hierarchy")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let mut gui = Gui::new(ui, skin);
                crate::hierarchy::draw(state, &mut gui, rect);
            });
    }

    // Right panel (inspector)
    if state.show_right_panel {
        egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let mut gui = Gui::new(ui, skin);
                crate::inspector::draw(state, &mut gui, rect);
            });
    }

    // Central viewport
    egui::CentralPanel::default().show(ctx, |ui| {
        let rect = ui.max_rect();
        let mut gui = Gui::new(ui, skin);
        crate::viewport::draw(
            state,
            &mut gui,
            rect,
            renderer,
            vp_renderer,
            egui_state,
        );
    });
}

fn draw_menu_bar(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(30, 30, 36)));

    let menus = ["文件", "编辑", "场景", "视图", "资源", "帮助"];
    let mut x = rect.left() + 8.0 * w_scale;
    let font = FontId::proportional(13.0 * h_scale);

    for (i, menu) in menus.iter().enumerate() {
        let text_w = painter.ctx().fonts(|f| f.layout_no_wrap(menu.to_string(), font.clone(), Color32::WHITE)).size().x;
        let item_rect = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(text_w + 16.0 * w_scale, rect.height()));

        if gui.button(item_rect, menu) {
            state.active_menu = Some(i);
        }

        if state.active_menu == Some(i) {
            let items = match i {
                0 => vec!["新建场景", "打开场景", "保存", "另存为", "退出"],
                1 => vec!["撤销", "重做", "剪切", "复制", "粘贴"],
                2 => vec!["创建空节点", "创建立方体", "创建球体", "创建光源"],
                3 => vec!["层级面板", "检视面板", "资源浏览器"],
                4 => vec!["导入资源", "加载模型", "加载预制件", "刷新资源"],
                5 => vec!["关于"],
                _ => vec![],
            };

            let item_h = 24.0 * h_scale;
            let dropdown_w = 140.0 * w_scale;
            let dropdown_rect = Rect::from_min_size(
                Pos2::new(item_rect.left(), item_rect.bottom()),
                Vec2::new(dropdown_w, items.len() as f32 * item_h),
            );

            let dp = gui.ui.painter_at(dropdown_rect);
            dp.add(Shape::rect_filled(dropdown_rect, Rounding::same(2.0), Color32::from_rgb(40, 40, 48)));

            for (j, item) in items.iter().enumerate() {
                let item_rect = Rect::from_min_size(
                    Pos2::new(dropdown_rect.left(), dropdown_rect.top() + j as f32 * item_h),
                    Vec2::new(dropdown_w, item_h),
                );
                if gui.button(item_rect, item) {
                    // Handle menu actions
                    match (i, j) {
                        (0, 0) => state.scene_manager.create_scene("新场景".to_string()),
                        (0, 2) => { let _ = state.scene_manager.save_current_scene(); },
                        (3, 0) => state.show_left_panel = !state.show_left_panel,
                        (3, 1) => state.show_right_panel = !state.show_right_panel,
                        _ => {}
                    }
                    state.active_menu = None;
                }
            }
        }

        x += text_w + 20.0 * w_scale;
    }
}

fn draw_toolbar(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(35, 35, 42)));

    let btn_w = 32.0 * w_scale;
    let btn_h = 28.0 * h_scale;
    let gap = 4.0 * w_scale;
    let mut x = rect.left() + 8.0 * w_scale;
    let y = rect.center().y - btn_h * 0.5;

    // Play/Pause/Stop
    let play_text = match state.play_state {
        PlayState::Playing => "⏸",
        _ => "▶",
    };
    let play_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(btn_w, btn_h));
    if gui.button(play_rect, play_text) {
        match state.play_state {
            PlayState::Editing => { state.play(); }
            PlayState::Playing => { state.pause(); }
            PlayState::Paused => { state.play(); }
        }
    }
    x += btn_w + gap;

    let stop_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(btn_w, btn_h));
    if gui.button(stop_rect, "⏹") {
        state.stop();
    }
    x += btn_w + gap * 3.0;

    // Tool buttons
    let tools = [("Q", ToolType::Select), ("W", ToolType::Translate), ("E", ToolType::Rotate), ("R", ToolType::Scale)];
    for (label, tool) in &tools {
        let btn_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(btn_w, btn_h));
        let is_active = state.active_tool == *tool;
        if is_active {
            painter.add(Shape::rect_filled(btn_rect, Rounding::same(2.0), Color32::from_rgb(60, 120, 200)));
        }
        if gui.button(btn_rect, label) {
            state.active_tool = *tool;
        }
        x += btn_w + gap;
    }

    x += gap * 3.0;

    // Grid toggle
    let grid_text = if state.show_grid { "网格✓" } else { "网格" };
    let grid_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(btn_w * 1.3, btn_h));
    if gui.button(grid_rect, grid_text) {
        state.show_grid = !state.show_grid;
    }
    x += btn_w * 1.3 + gap;

    // Debug overlay toggle
    let debug_text = if state.show_debug_overlay { "调试✓" } else { "调试" };
    let debug_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(btn_w * 1.3, btn_h));
    if gui.button(debug_rect, debug_text) {
        state.show_debug_overlay = !state.show_debug_overlay;
    }
}

fn draw_bottom_panel(state: &mut EditorState, ui: &mut egui::Ui) {
    let tabs = ["控制台", "资源浏览器"];
    ui.horizontal(|ui| {
        for (i, tab) in tabs.iter().enumerate() {
            let selected = state.active_bottom_tab == i;
            let btn = ui.selectable_label(selected, *tab);
            if btn.clicked() {
                state.active_bottom_tab = i;
            }
        }
    });

    ui.separator();

    match state.active_bottom_tab {
        0 => {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for entry in &state.log_messages {
                    let color = match entry.level {
                        crate::state::LogLevel::Info => Color32::from_rgb(200, 200, 200),
                        crate::state::LogLevel::Warn => Color32::from_rgb(255, 200, 50),
                        crate::state::LogLevel::Error => Color32::from_rgb(255, 80, 80),
                    };
                    ui.colored_label(color, &entry.message);
                }
            });
        }
        1 => {
            let rect = ui.max_rect();
            let skin = GuiSkin::default();
            let mut gui = Gui::new(ui, &skin);
            crate::resource_browser::draw(state, &mut gui, rect);
        }
        _ => {}
    }
}

fn draw_status_bar(state: &mut EditorState, ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    let h_scale = ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = ui.ctx().screen_rect().width() / 1920.0;
    let painter = ui.painter();
    painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(25, 25, 30)));

    let font = FontId::proportional(11.0 * h_scale);
    let y = rect.center().y;

    let status_text = state.status_message.as_deref().unwrap_or("就绪");
    painter.text(Pos2::new(rect.left() + 8.0 * w_scale, y), egui::Align2::LEFT_CENTER, status_text, font.clone(), Color32::from_rgb(160, 160, 160));

    let play_text = match state.play_state {
        PlayState::Editing => "编辑模式",
        PlayState::Playing => "运行中",
        PlayState::Paused => "已暂停",
    };
    painter.text(Pos2::new(rect.right() - 120.0 * w_scale, y), egui::Align2::LEFT_CENTER, play_text, font.clone(), Color32::from_rgb(100, 200, 100));
}
