//! Top-level editor layout — composes all panels into the main window with
//! menu bar, toolbar, and dockable panel regions.

use crate::state::{EditorState, PlayState, ToolType};
use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_ui::{Gui, GuiSkin};
use std::path::PathBuf;

pub fn frame(
    state: &mut EditorState,
    ctx: &egui::Context,
    skin: &GuiSkin,
    renderer: &mut engine_render::renderer::Renderer,
    vp_renderer: &mut crate::viewport_renderer::ViewportRenderer,
    egui_state: &mut engine_ui::EguiState,
) {
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

            let menu_rect =
                Rect::from_min_size(screen.left_top(), Vec2::new(screen.width(), menu_h));
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
                Vec2::new(
                    if state.show_left_panel { left_w } else { 0.0 },
                    main_rect.height(),
                ),
            );
            let inspector_rect = Rect::from_min_size(
                Pos2::new(
                    main_rect.right() - (if state.show_right_panel { right_w } else { 0.0 }),
                    main_rect.top(),
                ),
                Vec2::new(
                    if state.show_right_panel { right_w } else { 0.0 },
                    main_rect.height(),
                ),
            );
            let viewport_rect = Rect::from_min_size(
                Pos2::new(hierarchy_rect.right(), main_rect.top()),
                Vec2::new(
                    inspector_rect.left() - hierarchy_rect.right(),
                    main_rect.height(),
                ),
            );

            {
                let mut gui = Gui::new(ui, skin);
                draw_menu_bar(state, &mut gui, menu_rect, w_scale, h_scale);
                draw_toolbar(state, &mut gui, toolbar_rect, w_scale, h_scale);
                if state.show_left_panel {
                    crate::hierarchy::draw(state, &mut gui, hierarchy_rect);
                }
                crate::viewport::draw(state, &mut gui, viewport_rect, renderer, vp_renderer, egui_state);
                if state.show_right_panel {
                    crate::inspector::draw(state, &mut gui, inspector_rect);
                }
            }
            draw_bottom_panel(state, ui, bottom_rect, h_scale, w_scale);
            if state.animation_editor.visible {
                let anim_h = (screen.height() * 300.0 / 1080.0).clamp(200.0, 500.0);
                let anim_rect = Rect::from_min_size(
                    Pos2::new(screen.left(), status_rect.top() - anim_h),
                    Vec2::new(screen.width(), anim_h),
                );
                crate::animation_editor::draw_animation_editor(state, ui, anim_rect);
            }
            if state.material_editor.visible {
                let mat_h = (screen.height() * 450.0 / 1080.0).clamp(300.0, 700.0);
                let mat_rect = Rect::from_min_size(
                    Pos2::new(screen.left(), status_rect.top() - mat_h),
                    Vec2::new(screen.width(), mat_h),
                );
                crate::material_editor::draw_material_editor(state, ui, mat_rect);
            }
            {
                let mut gui = Gui::new(ui, skin);
                draw_status_bar(state, &mut gui, status_rect, h_scale, w_scale);
            }
            // Real-time performance overlay
            state.performance_overlay.tick();
            state.performance_overlay.draw(ctx);
        });
}

fn draw_menu_bar(state: &mut EditorState, gui: &mut Gui, rect: Rect, w_scale: f32, h_scale: f32) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.bottom() - 1.0),
            Pos2::new(rect.right(), rect.bottom() - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let items = &[
        "文件", "编辑", "视图", "场景", "资源", "构建", "窗口", "帮助",
    ];
    let font_sz = 13.0 * h_scale;
    let char_w = 8.0 * w_scale;
    let item_pad = 12.0 * w_scale;
    let rounding = 4.0 * h_scale;
    let mut x = rect.left() + 8.0 * w_scale;

    // Track menu rects for dropdown rendering
    let mut menu_rects = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let text_w = item.len() as f32 * char_w;
        let item_rect = Rect::from_min_size(
            Pos2::new(x, rect.top()),
            Vec2::new(text_w + item_pad * 2.0, rect.height()),
        );
        menu_rects.push((i, item_rect));

        let id = egui::Id::new("mm").with(i as u64);
        let response = gui.ui.interact(item_rect, id, egui::Sense::click());

        // If a menu is active and we hover another, switch to that one
        if state.active_menu.is_some() && response.hovered() {
            state.active_menu = Some(i);
        } else if response.hovered() || state.active_menu == Some(i) {
            painter.add(Shape::rect_filled(
                item_rect,
                Rounding::same(rounding),
                Color32::from_rgb(30, 30, 34),
            ));
        }

        painter.text(
            egui::pos2(x + item_pad, rect.center().y),
            egui::Align2::LEFT_CENTER,
            *item,
            FontId::proportional(font_sz),
            if response.hovered() || state.active_menu == Some(i) {
                Color32::from_rgb(232, 232, 236)
            } else {
                Color32::from_gray(152)
            },
        );

        if response.clicked() {
            if state.active_menu == Some(i) {
                // Toggle off if clicking the same menu
                state.active_menu = None;
            } else {
                state.active_menu = Some(i);
            }
        }

        x += text_w + item_pad * 2.0 + 4.0 * w_scale;
    }

    // Draw dropdown menu if active
    if let Some(active_idx) = state.active_menu
        && let Some((_, menu_rect)) = menu_rects.iter().find(|(i, _)| *i == active_idx)
    {
        draw_dropdown_menu(state, gui, *menu_rect, w_scale, h_scale, active_idx);
    }

    // Check for clicks outside menus to close active menu
    if let Some(pos) = gui.ui.input(|i| i.pointer.latest_pos()) {
        let is_inside_any_menu = menu_rects.iter().any(|(_, r)| r.contains(pos));
        if !is_inside_any_menu && gui.ui.input(|i| i.pointer.primary_clicked()) {
            state.active_menu = None;
        }
    }

    painter.text(
        egui::pos2(rect.right() - 12.0 * w_scale, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        "MyGame",
        FontId::proportional(font_sz),
        Color32::from_gray(152),
    );
}

fn draw_dropdown_menu(
    state: &mut EditorState,
    gui: &mut Gui,
    menu_rect: Rect,
    w_scale: f32,
    h_scale: f32,
    menu_idx: usize,
) {
    let menu_items = match menu_idx {
        0 => vec!["新建项目", "打开项目", "保存", "另存为", "退出"], // 文件
        1 => vec!["撤销", "重做", "剪切", "复制", "粘贴"],           // 编辑
        2 => vec!["切换左侧面板", "切换右侧面板", "重置布局"],       // 视图
        3 => vec!["新建场景", "保存场景", "加载场景"],               // 场景
        4 => vec!["导入资源", "刷新资源"],                           // 资源
        5 => vec!["构建项目", "运行项目"],                           // 构建
        6 => vec![
            "控制台",
            "性能",
            "资源浏览器",
            "动画编辑器",
            "材质编辑器",
            "脚本编辑器",
            "性能叠加层",
        ], // 窗口
        7 => vec!["关于", "文档"],                                   // 帮助
        _ => vec![],
    };

    if menu_items.is_empty() {
        return;
    }

    let item_h = 32.0 * h_scale;
    let menu_w = 200.0 * w_scale;
    let padding = 8.0 * w_scale;

    let dropdown_rect = Rect::from_min_size(
        Pos2::new(menu_rect.left(), menu_rect.bottom()),
        Vec2::new(menu_w, item_h * menu_items.len() as f32 + padding * 2.0),
    );

    let painter = gui.ui.painter_at(dropdown_rect);

    // Draw menu background
    painter.add(Shape::rect_filled(
        dropdown_rect,
        Rounding::same(4.0 * h_scale),
        Color32::from_rgb(30, 30, 34),
    ));

    // Draw border
    painter.add(Shape::rect_stroke(
        dropdown_rect,
        Rounding::same(4.0 * h_scale),
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    // Draw menu items
    let font_sz = 13.0 * h_scale;
    let _char_w = 8.0 * w_scale;
    let item_pad = 16.0 * w_scale;
    let rounding = 4.0 * h_scale;

    let mut y = dropdown_rect.top() + padding;
    for (i, item) in menu_items.iter().enumerate() {
        let item_rect = Rect::from_min_size(
            Pos2::new(dropdown_rect.left(), y),
            Vec2::new(dropdown_rect.width(), item_h),
        );
        let id = egui::Id::new("dropdown_item").with(menu_idx).with(i as u64);
        let response = gui.ui.interact(item_rect, id, egui::Sense::click());

        if response.hovered() {
            painter.add(Shape::rect_filled(
                item_rect,
                Rounding::same(rounding),
                Color32::from_rgb(0, 110, 210),
            ));
        }

        painter.text(
            egui::pos2(dropdown_rect.left() + item_pad, item_rect.center().y),
            egui::Align2::LEFT_CENTER,
            *item,
            FontId::proportional(font_sz),
            if response.hovered() {
                Color32::from_rgb(232, 232, 236)
            } else {
                Color32::from_gray(180)
            },
        );

        if response.clicked() {
            // Handle menu item actions
            match menu_idx {
                0 => {
                    // 文件菜单
                    match i {
                        2 => {
                            // 保存
                            save_scene(state);
                        }
                        3 => {
                            // 另存为
                            save_scene_as(state);
                        }
                        4 => {
                            // 退出
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                }
                1 => {
                    // 编辑菜单
                    match i {
                        0 => state.undo(),    // 撤销
                        1 => state.redo(),    // 重做
                        2 => state.cut_selected(),   // 剪切
                        3 => state.copy_selected(),  // 复制
                        4 => state.paste(),          // 粘贴
                        _ => {}
                    }
                }
                2 => {
                    // 视图菜单
                    match i {
                        0 => state.show_left_panel = !state.show_left_panel,
                        1 => state.show_right_panel = !state.show_right_panel,
                        2 => {
                            // 重置布局
                            state.show_left_panel = true;
                            state.show_right_panel = true;
                            state.viewport_layout = crate::viewport_renderer::ViewportLayout::default();
                            state.status_message = Some("布局已重置".into());
                        }
                        _ => {}
                    }
                }
                3 => {
                    // 场景菜单
                    match i {
                        0 => {
                            // 新建场景
                            state.scene_manager.create_scene("Untitled".to_string());
                            state.status_message = Some("New scene created".to_string());
                        }
                        1 => {
                            // 保存场景
                            save_scene(state);
                        }
                        2 => {
                            // 加载场景
                            load_scene(state);
                        }
                        _ => {}
                    }
                }
                4 => {
                    // 资源菜单
                    match i {
                        0 => {
                            // 导入资源
                            state.status_message = Some("导入资源: 功能开发中".into());
                        }
                        1 => {
                            // 刷新资源
                            state.resource_browser.refresh();
                            state.status_message = Some("资源已刷新".into());
                        }
                        _ => {}
                    }
                }
                5 => {
                    // 构建菜单
                    match i {
                        0 => {
                            // 构建项目
                            state.build_project();
                        }
                        1 => {
                            // 运行项目
                            state.run_project();
                        }
                        _ => {}
                    }
                }
                6 => {
                    // 窗口菜单
                    match i {
                        0 => state.active_bottom_tab = 0, // 日志
                        1 => state.active_bottom_tab = 1, // 性能
                        2 => state.active_bottom_tab = 2, // 资源
                        3 => state.animation_editor.visible = !state.animation_editor.visible, // 动画编辑器
                        4 => state.material_editor.visible = !state.material_editor.visible, // 材质编辑器
                        5 => state.active_bottom_tab = 5, // 脚本编辑器
                        6 => {
                            // 性能叠加层
                            state.performance_overlay.config.visible =
                                !state.performance_overlay.config.visible;
                        }
                        _ => {}
                    }
                }
                7 => {
                    // 帮助菜单
                    match i {
                        0 => {
                            // 关于
                            state.status_message = Some("RustEngine v0.1.0 — 基于 Rust 的高性能游戏引擎".into());
                        }
                        1 => {
                            // 文档
                            state.status_message = Some("文档: docs/ 目录下包含完整教程".into());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            state.active_menu = None; // Close menu after action
        }

        y += item_h;
    }
}

fn draw_separator(painter: &egui::Painter, pos: f32, top: f32, bottom: f32, h_scale: f32) {
    let m = 8.0 * h_scale;
    painter.add(Shape::line(
        vec![Pos2::new(pos, top + m), Pos2::new(pos, bottom - m)],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
}

fn draw_toolbar(state: &mut EditorState, gui: &mut Gui, rect: Rect, w_scale: f32, h_scale: f32) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.bottom() - 1.0),
            Pos2::new(rect.right(), rect.bottom() - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let btn_size = 32.0 * h_scale;
    let gap = 4.0 * w_scale;
    let pad = 12.0 * w_scale;
    let mut x = rect.left() + pad;
    let cy = rect.top() + (rect.height() - btn_size) / 2.0;

    let tools = &["↖", "↔", "⟳", "⤢", "⛰"];
    let tool_types = [
        ToolType::Select,
        ToolType::Translate,
        ToolType::Rotate,
        ToolType::Scale,
        ToolType::Terrain,
    ];
    for (i, tool) in tools.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(x + i as f32 * (btn_size + gap), cy),
            Vec2::new(btn_size, btn_size),
        );
        if gui.tool_button(btn_rect, tool, state.active_tool == tool_types[i]) {
            state.active_tool = tool_types[i];
        }
    }
    x += 5.0 * (btn_size + gap) + pad;
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;

    for (i, icon) in ["📁", "🔍"].iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(x + i as f32 * (btn_size + gap), cy),
            Vec2::new(btn_size, btn_size),
        );
        if gui.tool_button(btn_rect, icon, false) {
            if i == 0 {
                state.show_left_panel = !state.show_left_panel;
            }
            if i == 1 {
                state.show_right_panel = !state.show_right_panel;
            }
        }
    }
    x += 2.0 * (btn_size + gap) + pad;
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;

    let modes = &["3D", "T", "F", "R"];
    for (i, mode) in modes.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(x + i as f32 * (btn_size + gap), cy),
            Vec2::new(btn_size, btn_size),
        );
        gui.tool_button(btn_rect, mode, state.active_viewport_tab == i);
    }
    x += 4.0 * (btn_size + gap) + pad;

    // Viewport layout buttons
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;
    let layouts = &["1", "⬌", "⬍", "⊞"];
    for (i, icon) in layouts.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(x + i as f32 * (btn_size + gap), cy),
            Vec2::new(btn_size, btn_size),
        );
        if gui.tool_button(btn_rect, icon, false) {
            use crate::viewport_renderer::{ViewportLayout, ViewportType};
            state.viewport_layout = match i {
                0 => ViewportLayout::Single(ViewportType::Perspective),
                1 => ViewportLayout::Horizontal(ViewportType::Perspective, ViewportType::Top),
                2 => ViewportLayout::Vertical(ViewportType::Perspective, ViewportType::Top),
                3 => ViewportLayout::Quad,
                _ => state.viewport_layout,
            };
        }
    }
    x += 4.0 * (btn_size + gap) + pad;

    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;

    // Scene management buttons
    let scene_icons = &["📄", "💾", "📂"];
    for (i, icon) in scene_icons.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(x + i as f32 * (btn_size + gap), cy),
            Vec2::new(btn_size, btn_size),
        );
        let is_modified = state.scene_manager.is_modified();
        let is_active = i == 1 && is_modified; // Highlight save button when modified
        if gui.tool_button(btn_rect, icon, is_active) {
            match i {
                0 => {
                    // New scene
                    state.scene_manager.create_scene("Untitled".to_string());
                    state.status_message = Some("新场景已创建".into());
                }
                1 => {
                    // Save scene
                    save_scene(state);
                }
                2 => {
                    // Load scene
                    load_scene(state);
                }
                _ => {}
            }
        }
    }
    x += 3.0 * (btn_size + gap) + pad;

    // Play/Pause/Stop buttons
    let play_icons = ["▶", "⏸", "⏹"];
    let play_states = [state.play_state == PlayState::Playing, state.play_state == PlayState::Paused, state.play_state != PlayState::Editing];
    for (i, icon) in play_icons.iter().enumerate() {
        let btn_rect = Rect::from_min_size(Pos2::new(x, cy), Vec2::new(btn_size, btn_size));
        if gui.tool_button(btn_rect, icon, play_states[i]) {
            match i {
                0 => { state.play(); }
                1 => { state.pause(); }
                2 => { state.stop(); }
                _ => {}
            }
        }
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

fn draw_bottom_panel(
    state: &mut EditorState,
    ui: &mut egui::Ui,
    rect: Rect,
    h_scale: f32,
    w_scale: f32,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.top()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let tab_h = 32.0 * h_scale;
    let tab_bar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), tab_h));
    let tabs = &["日志", "性能", "资源", "音频", "网络", "脚本"];
    let tab_font = 12.0 * h_scale;
    let char_w = 8.0 * w_scale;
    let mut tx = rect.left() + 8.0 * w_scale;
    for (i, label) in tabs.iter().enumerate() {
        let text_w = label.len() as f32 * char_w;
        let tab_rect = Rect::from_min_size(
            Pos2::new(tx, rect.top()),
            Vec2::new(text_w + 28.0 * w_scale, tab_h),
        );
        let id = egui::Id::new("btm_tab").with(i as u64);
        let response = ui.interact(tab_rect, id, egui::Sense::click());
        if state.active_bottom_tab == i {
            let line_rect = Rect::from_min_size(
                Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0 * h_scale),
                Vec2::new(tab_rect.width(), 2.0 * h_scale),
            );
            painter.add(Shape::rect_filled(
                line_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 212, 170),
            ));
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_rgb(0, 212, 170),
            );
        } else {
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_gray(90),
            );
        }
        if response.clicked() {
            state.active_bottom_tab = i;
        }
        tx += text_w + 28.0 * w_scale;
    }

    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), tab_bar_rect.bottom()),
        Vec2::new(rect.width(), rect.bottom() - tab_bar_rect.bottom()),
    );

    let log_font = 11.0 * h_scale;
    let log_step = 18.0 * h_scale;
    match state.active_bottom_tab {
        0 => {
            if state.log_messages.is_empty() {
                painter.text(
                    content_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "暂无日志",
                    FontId::proportional(log_font),
                    Color32::from_gray(60),
                );
            } else {
                let mut y = content_rect.top() + 8.0 * h_scale;
                for entry in state.log_messages.iter().rev() {
                    if y > content_rect.bottom() {
                        break;
                    }
                    let time_color = Color32::from_gray(90);
                    let level_color = match entry.level {
                        crate::state::LogLevel::Info => Color32::from_gray(152),
                        crate::state::LogLevel::Warn => Color32::from_rgb(255, 184, 0),
                        crate::state::LogLevel::Error => Color32::from_rgb(255, 71, 87),
                    };
                    let level_str = match entry.level {
                        crate::state::LogLevel::Info => "info",
                        crate::state::LogLevel::Warn => "warn",
                        crate::state::LogLevel::Error => "error",
                    };
                    painter.text(
                        egui::pos2(content_rect.left() + 12.0 * w_scale, y),
                        egui::Align2::LEFT_CENTER,
                        &entry.timestamp,
                        FontId::proportional(log_font),
                        time_color,
                    );
                    painter.text(
                        egui::pos2(content_rect.left() + 72.0 * w_scale, y),
                        egui::Align2::LEFT_CENTER,
                        level_str,
                        FontId::proportional(log_font),
                        level_color,
                    );
                    painter.text(
                        egui::pos2(content_rect.left() + 122.0 * w_scale, y),
                        egui::Align2::LEFT_CENTER,
                        &entry.message,
                        FontId::proportional(log_font),
                        Color32::from_rgb(232, 232, 236),
                    );
                    y += log_step;
                }
            }
        }
        1 => {
            crate::performance_profiler::draw(&mut state.performance_profiler, ui, content_rect);
        }
        2 => {
            let skin_default = engine_ui::GuiSkin::default();
            let mut gui = Gui::new(ui, &skin_default);
            crate::resource_browser::draw(state, &mut gui, content_rect);
        }
        5 => {
            crate::script_editor::draw_script_editor(state, ui, content_rect);
        }
        3 => {
            // 音频面板 — 显示混音器总线和音频状态
            let bus_font = FontId::proportional(log_font);
            let mut y = content_rect.top() + 8.0 * h_scale;
            let bus_names = ["Master", "SFX", "Music", "Ambient", "Voice", "UI"];
            let bus_colors = [
                Color32::from_rgb(0, 212, 170),
                Color32::from_rgb(255, 184, 0),
                Color32::from_rgb(77, 171, 247),
                Color32::from_rgb(46, 213, 115),
                Color32::from_rgb(255, 107, 107),
                Color32::from_rgb(180, 130, 255),
            ];
            painter.text(
                egui::pos2(content_rect.left() + 12.0 * w_scale, y),
                egui::Align2::LEFT_CENTER,
                "音频混音器",
                FontId::proportional(12.0 * h_scale),
                Color32::from_rgb(220, 220, 224),
            );
            y += 24.0 * h_scale;
            for (i, name) in bus_names.iter().enumerate() {
                let bar_w = content_rect.width() - 120.0 * w_scale;
                let bar_h = 12.0 * h_scale;
                // Bus name
                painter.text(
                    egui::pos2(content_rect.left() + 12.0 * w_scale, y + bar_h / 2.0),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    bus_font.clone(),
                    bus_colors[i],
                );
                // Volume bar background
                let bar_rect = Rect::from_min_size(
                    Pos2::new(content_rect.left() + 80.0 * w_scale, y),
                    Vec2::new(bar_w, bar_h),
                );
                painter.add(Shape::rect_filled(
                    bar_rect,
                    Rounding::same(3.0),
                    Color32::from_rgb(30, 30, 34),
                ));
                // Volume bar fill (random-looking for visual feedback)
                let fill = if i == 0 { 0.75 } else { 0.5 + (i as f32 * 0.08) };
                let fill_rect = Rect::from_min_size(
                    bar_rect.left_top(),
                    Vec2::new(bar_w * fill.min(1.0), bar_h),
                );
                painter.add(Shape::rect_filled(
                    fill_rect,
                    Rounding::same(3.0),
                    bus_colors[i],
                ));
                // Volume percentage
                painter.text(
                    egui::pos2(bar_rect.right() + 8.0 * w_scale, y + bar_h / 2.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{}%", (fill * 100.0) as i32),
                    bus_font.clone(),
                    Color32::from_gray(120),
                );
                y += bar_h + 6.0 * h_scale;
            }
            // Status info
            y += 12.0 * h_scale;
            painter.text(
                egui::pos2(content_rect.left() + 12.0 * w_scale, y),
                egui::Align2::LEFT_CENTER,
                "状态: 就绪 | 采样率: 44100 Hz | 声道: 2 (立体声)",
                bus_font,
                Color32::from_gray(90),
            );
        }
        4 => {
            // 网络面板 — 显示连接状态和网络统计
            let net_font = FontId::proportional(log_font);
            let mut y = content_rect.top() + 8.0 * h_scale;
            painter.text(
                egui::pos2(content_rect.left() + 12.0 * w_scale, y),
                egui::Align2::LEFT_CENTER,
                "网络状态",
                FontId::proportional(12.0 * h_scale),
                Color32::from_rgb(220, 220, 224),
            );
            y += 24.0 * h_scale;
            let stats = [
                ("连接状态", "未连接", Color32::from_gray(90)),
                ("协议", "TCP/UDP", Color32::from_rgb(77, 171, 247)),
                ("监听端口", "无", Color32::from_gray(90)),
                ("活跃连接", "0", Color32::from_gray(90)),
                ("发送速率", "0 KB/s", Color32::from_gray(90)),
                ("接收速率", "0 KB/s", Color32::from_gray(90)),
                ("延迟", "N/A", Color32::from_gray(90)),
                ("丢包率", "0%", Color32::from_gray(90)),
            ];
            for (label, value, color) in &stats {
                painter.text(
                    egui::pos2(content_rect.left() + 12.0 * w_scale, y),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    net_font.clone(),
                    Color32::from_gray(120),
                );
                painter.text(
                    egui::pos2(content_rect.left() + 120.0 * w_scale, y),
                    egui::Align2::LEFT_CENTER,
                    *value,
                    net_font.clone(),
                    *color,
                );
                y += 18.0 * h_scale;
            }
        }
        _ => {
            painter.text(
                content_rect.center(),
                egui::Align2::CENTER_CENTER,
                "-- 面板内容 --",
                FontId::proportional(log_font),
                Color32::from_gray(90),
            );
        }
    }
}

fn draw_status_bar(state: &EditorState, gui: &mut Gui, rect: Rect, h_scale: f32, w_scale: f32) {
    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(30, 30, 34),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.top()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let status_font = 11.0 * h_scale;
    let pad12 = 12.0 * w_scale;
    gui.status_item(
        Rect::from_min_size(
            Pos2::new(rect.left() + pad12, rect.top()),
            Vec2::new(60.0 * w_scale, rect.height()),
        ),
        "就绪",
        Color32::from_rgb(46, 213, 115),
    );

    // Scene file path and modification indicator
    let scene_path = state.scene_manager.scene_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "未保存".into());
    let is_modified = state.scene_manager.is_modified();
    let modified_indicator = if is_modified { " ●" } else { "" };
    let scene_display = format!("{}{}", scene_path, modified_indicator);

    painter.text(
        egui::pos2(rect.left() + 80.0 * w_scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &scene_display,
        FontId::proportional(status_font),
        if is_modified {
            Color32::from_rgb(255, 184, 0)
        } else {
            Color32::from_gray(120)
        },
    );

    let default_status = format!("对象: {}", state.scene_tree.nodes.len());
    let status_text = state.status_message.as_deref().unwrap_or(&default_status);

    painter.text(
        egui::pos2(rect.left() + 300.0 * w_scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        status_text,
        FontId::proportional(status_font),
        if state.status_message.is_some() {
            Color32::from_rgb(0, 212, 170)
        } else {
            Color32::from_gray(90)
        },
    );

    painter.text(
        egui::pos2(rect.left() + 420.0 * w_scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "三角形: 45K".to_string(),
        FontId::proportional(status_font),
        Color32::from_gray(90),
    );

    // Undo/redo status
    let undo_text = state
        .command_manager
        .undo_description()
        .map(|d| format!("撤销: {}", d))
        .unwrap_or_else(|| "无可撤销".into());
    let redo_text = state
        .command_manager
        .redo_description()
        .map(|d| format!("重做: {}", d))
        .unwrap_or_else(|| "无可重做".into());
    painter.text(
        egui::pos2(rect.left() + 300.0 * w_scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{} | {}", undo_text, redo_text),
        FontId::proportional(status_font),
        if state.command_manager.can_undo() {
            Color32::from_gray(120)
        } else {
            Color32::from_gray(60)
        },
    );

    let view_modes = ["场景", "游戏", "物理"];
    let view_names = ["perspective", "top", "front", "right"];
    let view_mode = view_modes.get(state.active_viewport_tab).unwrap_or(&"场景");
    let view_name = view_names.first().unwrap_or(&"perspective");
    let play_str = match state.play_state {
        crate::state::PlayState::Playing => " ▶ 运行中",
        crate::state::PlayState::Paused => " ⏸ 暂停",
        crate::state::PlayState::Editing => "",
    };
    painter.text(
        egui::pos2(rect.right() - pad12, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{} 视图  |  {}{}", view_mode, view_name, play_str),
        FontId::proportional(status_font),
        match state.play_state {
            crate::state::PlayState::Playing => Color32::from_rgb(46, 213, 115),
            crate::state::PlayState::Paused => Color32::from_rgb(255, 184, 0),
            _ => Color32::from_gray(90),
        },
    );
}

const DEFAULT_SCENE_FILE: &str = "scenes/untitled.json";

fn save_scene(state: &mut EditorState) {
    // Sync EditorState to scene before saving
    let scene = state.to_scene("Untitled");
    state.scene_manager.set_current_scene(scene);

    let path = state
        .scene_manager
        .scene_path()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SCENE_FILE));

    match state.scene_manager.save_scene(&path) {
        Ok(()) => {
            let entity_count = state.scene_manager.current_scene().map(|s| s.entities.len()).unwrap_or(0);
            state.log_info(&format!("场景已保存: {} ({} 个实体)", path.display(), entity_count));
            state.status_message = Some(format!("已保存: {}", path.display()));
        }
        Err(e) => {
            state.log_error(&format!("保存失败: {}", e));
            state.status_message = Some(format!("保存失败: {}", e));
        }
    }
}

fn save_scene_as(state: &mut EditorState) {
    // Sync EditorState to scene before saving
    let scene = state.to_scene("Untitled");
    state.scene_manager.set_current_scene(scene);

    // Open native file dialog
    let path = rfd::FileDialog::new()
        .set_title("另存为")
        .add_filter("场景文件", &["json", "scene"])
        .add_filter("所有文件", &["*"])
        .set_directory(".")
        .save_file();

    match path {
        Some(path) => {
            match state.scene_manager.save_scene(&path) {
                Ok(()) => {
                    state.status_message = Some(format!("已保存: {}", path.display()));
                    state.log_info(&format!("场景已保存到: {}", path.display()));
                }
                Err(e) => {
                    state.status_message = Some(format!("保存失败: {}", e));
                    state.log_error(&format!("保存失败: {}", e));
                }
            }
        }
        None => {
            state.status_message = Some("已取消保存".into());
        }
    }
}

fn load_scene(state: &mut EditorState) {
    // Open native file dialog
    let path = rfd::FileDialog::new()
        .set_title("加载场景")
        .add_filter("场景文件", &["json", "scene"])
        .add_filter("所有文件", &["*"])
        .set_directory(".")
        .pick_file();

    let path = match path {
        Some(p) => p,
        None => {
            state.status_message = Some("已取消加载".into());
            return;
        }
    };

    if !path.exists() {
        state.status_message = Some(format!("文件不存在: {}", path.display()));
        return;
    }

    match state.scene_manager.load_scene(&path) {
        Ok(()) => {
            // Sync loaded scene to EditorState
            if let Some(scene) = state.scene_manager.current_scene() {
                let scene_clone = scene.clone();
                let entity_count = scene_clone.entities.len();
                let version = scene_clone.version;
                state.load_from_scene(&scene_clone);
                state.log_info(&format!(
                    "场景已加载: {} (版本 {}, {} 个实体)",
                    path.display(),
                    version,
                    entity_count
                ));
            }
            let name = state
                .scene_manager
                .current_scene()
                .map(|s| s.name.clone())
                .unwrap_or_default();
            state.status_message = Some(format!("已加载: {}", name));
        }
        Err(e) => {
            state.log_error(&format!("加载失败: {}", e));
            state.status_message = Some(format!("加载失败: {}", e));
        }
    }
}
