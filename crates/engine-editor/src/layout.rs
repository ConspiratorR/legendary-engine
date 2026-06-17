//! Top-level editor layout — composes all panels into the main window with
//! menu bar, toolbar, and dockable panel regions.

use crate::state::{EditorState, PlayState, ToolType};
use egui::{Color32, Rounding};
use engine_ui::{Gui, GuiSkin};

pub fn frame(
    state: &mut EditorState,
    ctx: &egui::Context,
    skin: &GuiSkin,
    renderer: &mut engine_render::renderer::Renderer,
    vp_renderer: &mut crate::viewport_renderer::ViewportRenderer,
    egui_state: &mut engine_ui::EguiState,
) {
    let screen = ctx.screen_rect();
    let sw = screen.width();
    let sh = screen.height();

    // Initialize panel sizes from screen ratio on first frame
    if state.left_panel_width <= 0.0 {
        state.left_panel_width = sw * 0.15;
    }
    if state.right_panel_width <= 0.0 {
        state.right_panel_width = sw * 0.16;
    }
    if state.bottom_panel_height <= 0.0 {
        state.bottom_panel_height = sh * 0.17;
    }

    let min_left = (sw * 0.08).max(120.0);
    let max_left = (sw * 0.35).min(600.0);
    let min_right = (sw * 0.10).max(150.0);
    let max_right = (sw * 0.40).min(700.0);
    let min_bottom = (sh * 0.06).max(50.0);
    let max_bottom = (sh * 0.50).min(600.0);

    // Top menu bar
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        draw_menu_bar(state, ui);
    });

    // Toolbar below menu
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        draw_toolbar(state, ui);
    });

    // Bottom panel (console/logs)
    let bottom_h = state.bottom_panel_height.clamp(min_bottom, max_bottom);
    let bottom_resp = egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .default_height(bottom_h)
        .min_height(min_bottom)
        .max_height(max_bottom)
        .show(ctx, |ui| {
            draw_bottom_panel(state, ui);
        });
    state.bottom_panel_height = bottom_resp.response.rect.height();

    // Status bar
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        draw_status_bar(state, ui);
    });

    // Left panel (hierarchy)
    if state.show_left_panel {
        let left_w = state.left_panel_width.clamp(min_left, max_left);
        let left_resp = egui::SidePanel::left("hierarchy")
            .resizable(true)
            .default_width(left_w)
            .width_range(min_left..=max_left)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let mut gui = Gui::new(ui, skin);
                crate::hierarchy::draw(state, &mut gui, rect);
            });
        state.left_panel_width = left_resp.response.rect.width();
    }

    // Right panel (inspector)
    if state.show_right_panel {
        let right_w = state.right_panel_width.clamp(min_right, max_right);
        let right_resp = egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(right_w)
            .width_range(min_right..=max_right)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let mut gui = Gui::new(ui, skin);
                crate::inspector::draw(state, &mut gui, rect);
            });
        state.right_panel_width = right_resp.response.rect.width();
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

fn draw_menu_bar(state: &mut EditorState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let menus = ["文件", "编辑", "场景", "视图", "资源", "帮助"];
        for (i, menu) in menus.iter().enumerate() {
            let btn = ui.button(*menu);
            if btn.clicked() {
                state.active_menu = if state.active_menu == Some(i) { None } else { Some(i) };
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
                egui::Area::new(egui::Id::new("menu_dropdown"))
                    .fixed_pos(btn.rect.left_bottom())
                    .show(ui.ctx(), |ui| {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(40, 40, 48))
                            .rounding(Rounding::same(2.0))
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                for (j, item) in items.iter().enumerate() {
                                    if ui.button(*item).clicked() {
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
                            });
                    });
            }
        }
    });
}

fn draw_toolbar(state: &mut EditorState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Play/Pause/Stop
        let play_text = match state.play_state {
            PlayState::Playing => "⏸ 暂停",
            _ => "▶ 播放",
        };
        if ui.button(play_text).clicked() {
            match state.play_state {
                PlayState::Editing => { state.play(); }
                PlayState::Playing => { state.pause(); }
                PlayState::Paused => { state.play(); }
            }
        }
        if ui.button("⏹ 停止").clicked() {
            state.stop();
        }

        ui.separator();

        // Tool buttons
        let tools = [("Q", "选择", ToolType::Select), ("W", "移动", ToolType::Translate), ("E", "旋转", ToolType::Rotate), ("R", "缩放", ToolType::Scale)];
        for (key, label, tool) in &tools {
            let text = format!("{}({})", label, key);
            let btn = ui.selectable_label(state.active_tool == *tool, text);
            if btn.clicked() {
                state.active_tool = *tool;
            }
        }

        ui.separator();

        let grid_btn = ui.selectable_label(state.show_grid, "网格");
        if grid_btn.clicked() {
            state.show_grid = !state.show_grid;
        }
        let debug_btn = ui.selectable_label(state.show_debug_overlay, "调试");
        if debug_btn.clicked() {
            state.show_debug_overlay = !state.show_debug_overlay;
        }
    });
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
    ui.horizontal(|ui| {
        let status_text = state.status_message.as_deref().unwrap_or("就绪");
        ui.label(status_text);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let play_text = match state.play_state {
                PlayState::Editing => "编辑模式",
                PlayState::Playing => "运行中",
                PlayState::Paused => "已暂停",
            };
            ui.colored_label(Color32::from_rgb(100, 200, 100), play_text);
        });
    });
}
