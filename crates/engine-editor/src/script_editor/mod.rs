pub mod breakpoints;
pub mod output;
pub mod syntax_highlight;
pub mod watcher;

use breakpoints::{BreakpointManager, StepMode};
use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Rounding, ScrollArea, Shape, Stroke, Vec2};
use output::{OutputLevel, ScriptOutput};
use std::path::PathBuf;
use syntax_highlight::{ScriptLanguage, detect_language, highlight};
use watcher::{VarType, VariableWatcher};

use crate::state::EditorState;

#[derive(Debug, Clone)]
pub struct OpenScript {
    pub path: PathBuf,
    pub name: String,
    pub content: String,
    pub language: ScriptLanguage,
    pub modified: bool,
    pub cursor_line: usize,
}

#[derive(Debug, Clone)]
pub struct ScriptEditorState {
    pub open_scripts: Vec<OpenScript>,
    pub active_script: usize,
    pub breakpoints: BreakpointManager,
    pub watcher: VariableWatcher,
    pub output: ScriptOutput,
    pub show_watcher: bool,
    pub show_output: bool,
    pub editor_sub_tab: EditorSubTab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorSubTab {
    Code,
    Output,
    Watcher,
}

impl Default for ScriptEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEditorState {
    pub fn new() -> Self {
        let mut state = Self {
            open_scripts: Vec::new(),
            active_script: 0,
            breakpoints: BreakpointManager::new(),
            watcher: VariableWatcher::new(),
            output: ScriptOutput::new(),
            show_watcher: true,
            show_output: true,
            editor_sub_tab: EditorSubTab::Code,
        };
        state.open_script(
            "movement.lua",
            "-- Player movement system\nlocal speed = 5.0\nlocal jump_force = 10.0\n\nfunction update(dt)\n    local entities = world:entities(\"Transform\", \"Velocity\")\n    for _, entity in ipairs(entities) do\n        local transform = world:get(entity, \"Transform\")\n        local velocity = world:get(entity, \"Velocity\")\n        if transform and velocity then\n            transform.position.x = transform.position.x + velocity.x * dt\n            transform.position.y = transform.position.y + velocity.y * dt\n        end\n    end\nend\n\nprint(\"Movement system loaded\")\n",
        );
        state.output.info("编辑器已启动", "system");
        state.output.info("脚本系统就绪", "engine-script");
        state
    }

    pub fn open_script(&mut self, name: &str, content: &str) {
        let path = PathBuf::from(name);
        let language = detect_language(name);
        if let Some(idx) = self.open_scripts.iter().position(|s| s.name == name) {
            self.active_script = idx;
            return;
        }
        self.open_scripts.push(OpenScript {
            path,
            name: name.to_string(),
            content: content.to_string(),
            language,
            modified: false,
            cursor_line: 0,
        });
        self.active_script = self.open_scripts.len() - 1;
    }

    pub fn close_script(&mut self, idx: usize) {
        if idx < self.open_scripts.len() {
            self.open_scripts.remove(idx);
            if self.active_script >= self.open_scripts.len() && !self.open_scripts.is_empty() {
                self.active_script = self.open_scripts.len() - 1;
            }
        }
    }

    pub fn active_script(&self) -> Option<&OpenScript> {
        self.open_scripts.get(self.active_script)
    }

    pub fn active_script_mut(&mut self) -> Option<&mut OpenScript> {
        self.open_scripts.get_mut(self.active_script)
    }
}

pub fn draw_script_editor(state: &mut EditorState, ui: &mut egui::Ui, rect: Rect) {
    let h_scale = rect.height() / 300.0;
    let w_scale = rect.width() / 1920.0;
    let font_sz = 12.0 * h_scale.min(1.5);
    let small_font = 11.0 * h_scale.min(1.5);

    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));

    let tab_h = 28.0 * h_scale.min(1.5);
    let tab_bar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), tab_h));
    draw_sub_tabs(ui, state, tab_bar_rect, font_sz, w_scale, h_scale);

    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), tab_bar_rect.bottom()),
        Vec2::new(rect.width(), rect.bottom() - tab_bar_rect.bottom()),
    );

    match state.script_editor.editor_sub_tab {
        EditorSubTab::Code => draw_code_area(
            ui,
            state,
            content_rect,
            font_sz,
            small_font,
            w_scale,
            h_scale,
        ),
        EditorSubTab::Output => draw_output_panel(
            ui,
            state,
            content_rect,
            font_sz,
            small_font,
            w_scale,
            h_scale,
        ),
        EditorSubTab::Watcher => draw_watcher_panel(
            ui,
            state,
            content_rect,
            font_sz,
            small_font,
            w_scale,
            h_scale,
        ),
    }
}

fn draw_sub_tabs(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    rect: Rect,
    font_sz: f32,
    w_scale: f32,
    _h_scale: f32,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(28, 28, 32),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.bottom() - 1.0),
            Pos2::new(rect.right(), rect.bottom() - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let mut x = rect.left() + 8.0 * w_scale;
    let tab_pad = 12.0 * w_scale;
    let char_w = 7.0 * w_scale;

    for i in 0..state.script_editor.open_scripts.len() {
        let script = &state.script_editor.open_scripts[i];
        let label = if script.modified {
            format!("{}*", script.name)
        } else {
            script.name.clone()
        };
        let text_w = label.len() as f32 * char_w;
        let tab_rect = Rect::from_min_size(
            Pos2::new(x, rect.top()),
            Vec2::new(text_w + tab_pad * 2.0 + 16.0 * w_scale, rect.height()),
        );

        let id = Id::new("script_tab").with(i as u64);
        let response = ui.interact(tab_rect, id, egui::Sense::click());

        let is_active = state.script_editor.active_script == i;
        if is_active {
            painter.add(Shape::rect_filled(
                Rect::from_min_size(
                    tab_rect.left_top(),
                    Vec2::new(tab_rect.width(), tab_rect.height() - 2.0),
                ),
                Rounding::same(4.0),
                Color32::from_rgb(30, 30, 34),
            ));
            let line_rect = Rect::from_min_size(
                Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0),
                Vec2::new(tab_rect.width(), 2.0),
            );
            painter.add(Shape::rect_filled(
                line_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 212, 170),
            ));
        }

        let text_color = if is_active {
            Color32::from_rgb(232, 232, 236)
        } else {
            Color32::from_gray(120)
        };

        painter.text(
            Pos2::new(x + tab_pad, rect.center().y),
            Align2::LEFT_CENTER,
            &label,
            FontId::proportional(font_sz),
            text_color,
        );

        // Close button
        let close_x = x + text_w + tab_pad * 2.0;
        let close_rect = Rect::from_center_size(
            Pos2::new(close_x + 6.0 * w_scale, rect.center().y),
            Vec2::splat(14.0 * w_scale),
        );
        let close_id = Id::new("script_close").with(i as u64);
        let close_resp = ui.interact(close_rect, close_id, egui::Sense::click());
        painter.text(
            close_rect.center(),
            Align2::CENTER_CENTER,
            "x",
            FontId::proportional(font_sz * 0.8),
            if close_resp.hovered() {
                Color32::from_rgb(224, 108, 117)
            } else {
                Color32::from_gray(80)
            },
        );

        if response.clicked() {
            state.script_editor.active_script = i;
        }
        if close_resp.clicked() {
            state.script_editor.close_script(i);
            if state.script_editor.active_script >= state.script_editor.open_scripts.len()
                && !state.script_editor.open_scripts.is_empty()
            {
                state.script_editor.active_script = state.script_editor.open_scripts.len() - 1;
            }
        }

        x += text_w + tab_pad * 2.0 + 20.0 * w_scale;
    }

    // Sub-tab buttons on the right
    let sub_tabs = &[
        (EditorSubTab::Code, "代码"),
        (EditorSubTab::Output, "输出"),
        (EditorSubTab::Watcher, "监视"),
    ];
    let sub_tab_w = 50.0 * w_scale;
    let mut sx = rect.right() - sub_tabs.len() as f32 * sub_tab_w;

    // Breakpoint status
    let bp_count = state.script_editor.breakpoints.breakpoint_count();
    let status_text = if state.script_editor.breakpoints.is_paused {
        "已暂停".to_string()
    } else if bp_count > 0 {
        format!("断点: {}", bp_count)
    } else {
        String::new()
    };
    if !status_text.is_empty() {
        let status_color = if state.script_editor.breakpoints.is_paused {
            Color32::from_rgb(229, 192, 123)
        } else {
            Color32::from_gray(100)
        };
        painter.text(
            Pos2::new(sx - 80.0 * w_scale, rect.center().y),
            Align2::LEFT_CENTER,
            &status_text,
            FontId::proportional(font_sz * 0.85),
            status_color,
        );
    }

    for (tab, label) in sub_tabs {
        let tab_rect = Rect::from_min_size(
            Pos2::new(sx, rect.top()),
            Vec2::new(sub_tab_w, rect.height()),
        );
        let id = Id::new("script_sub_tab").with(*tab as u64);
        let response = ui.interact(tab_rect, id, egui::Sense::click());

        let is_active = state.script_editor.editor_sub_tab == *tab;
        painter.text(
            tab_rect.center(),
            Align2::CENTER_CENTER,
            *label,
            FontId::proportional(font_sz),
            if is_active {
                Color32::from_rgb(0, 212, 170)
            } else {
                Color32::from_gray(100)
            },
        );

        if response.clicked() {
            state.script_editor.editor_sub_tab = *tab;
        }
        sx += sub_tab_w;
    }
}

fn draw_code_area(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    rect: Rect,
    font_sz: f32,
    _small_font: f32,
    w_scale: f32,
    h_scale: f32,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(30, 30, 34),
    ));

    if state.script_editor.open_scripts.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "-- 没有打开的脚本文件 --",
            FontId::proportional(font_sz),
            Color32::from_gray(80),
        );
        return;
    }

    // Toolbar
    let toolbar_h = 28.0 * h_scale.min(1.5);
    let toolbar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), toolbar_h));
    painter.add(Shape::rect_filled(
        toolbar_rect,
        Rounding::ZERO,
        Color32::from_rgb(26, 26, 30),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), toolbar_rect.bottom()),
            Pos2::new(rect.right(), toolbar_rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    // Debug controls
    let btn_size = 20.0 * h_scale.min(1.5);
    let gap = 4.0 * w_scale;
    let mut bx = toolbar_rect.left() + 8.0 * w_scale;
    let by = toolbar_rect.center().y - btn_size / 2.0;

    let debug_buttons = ["▶", "⏸", "⏹", "⏭", "↓"];

    for icon in &debug_buttons {
        let btn_rect = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(btn_size, btn_size));
        let id = Id::new("debug_btn").with(*icon);
        let response = ui.interact(btn_rect, id, egui::Sense::click());
        if response.hovered() {
            painter.add(Shape::rect_filled(
                btn_rect,
                Rounding::same(3.0),
                Color32::from_rgb(40, 40, 46),
            ));
        }
        painter.text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            *icon,
            FontId::proportional(font_sz * 0.85),
            if response.hovered() {
                Color32::from_rgb(232, 232, 236)
            } else {
                Color32::from_gray(150)
            },
        );

        if response.clicked() {
            match *icon {
                "▶" => {
                    state.script_editor.breakpoints.resume();
                    state
                        .script_editor
                        .output
                        .info("脚本执行已恢复", "debugger");
                }
                "⏸" => {
                    state.script_editor.breakpoints.pause_at(1);
                    state.script_editor.output.info("执行已暂停", "debugger");
                }
                "⏹" => {
                    state.script_editor.breakpoints.resume();
                    state.script_editor.output.info("执行已停止", "debugger");
                }
                "⏭" => {
                    state
                        .script_editor
                        .breakpoints
                        .set_step_mode(StepMode::StepOver);
                    state.script_editor.output.debug("单步执行", "debugger");
                }
                "↓" => {
                    state
                        .script_editor
                        .breakpoints
                        .set_step_mode(StepMode::StepInto);
                    state.script_editor.output.debug("步入", "debugger");
                }
                _ => {}
            }
        }
        bx += btn_size + gap;
    }

    // Separator
    bx += 4.0 * w_scale;
    painter.add(Shape::line(
        vec![
            Pos2::new(bx, toolbar_rect.top() + 4.0),
            Pos2::new(bx, toolbar_rect.bottom() - 4.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    bx += 8.0 * w_scale;

    // Language indicator
    if let Some(script) = state.script_editor.active_script() {
        let lang_label = match script.language {
            ScriptLanguage::Lua => "Lua",
            ScriptLanguage::Wasm => "WASM",
        };
        painter.text(
            Pos2::new(bx, toolbar_rect.center().y),
            Align2::LEFT_CENTER,
            lang_label,
            FontId::proportional(font_sz * 0.8),
            Color32::from_rgb(86, 182, 194),
        );
    }

    // File path
    if let Some(script) = state.script_editor.active_script() {
        let path_text = script.path.display().to_string();
        painter.text(
            Pos2::new(
                toolbar_rect.right() - 8.0 * w_scale,
                toolbar_rect.center().y,
            ),
            Align2::RIGHT_CENTER,
            &path_text,
            FontId::proportional(font_sz * 0.8),
            Color32::from_gray(80),
        );
    }

    // Code editor area
    let editor_rect = Rect::from_min_size(
        Pos2::new(rect.left(), toolbar_rect.bottom()),
        Vec2::new(rect.width(), rect.bottom() - toolbar_rect.bottom()),
    );

    let gutter_w = 50.0 * w_scale;
    let line_h = font_sz * 1.6;
    let font_id = FontId::monospace(font_sz);

    // Get active script data
    let (content, language, paused_line) = {
        let script = &state.script_editor.open_scripts[state.script_editor.active_script];
        let paused = state.script_editor.breakpoints.paused_line;
        (script.content.clone(), script.language, paused)
    };

    // Gutter background
    let gutter_rect = Rect::from_min_size(
        editor_rect.left_top(),
        Vec2::new(gutter_w, editor_rect.height()),
    );
    painter.add(Shape::rect_filled(
        gutter_rect,
        Rounding::ZERO,
        Color32::from_rgb(26, 26, 30),
    ));

    // Line numbers and breakpoint indicators
    let total_lines = content.lines().count().max(1);
    let scroll_id = Id::new("code_scroll");
    ScrollArea::vertical()
        .id_salt(scroll_id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(
                editor_rect.width() - gutter_w - 8.0,
                total_lines as f32 * line_h,
            ));

            let offset_y = ui.cursor().top();

            for (line_idx, _line_text) in content.lines().enumerate() {
                let line_num = line_idx + 1;
                let y = offset_y + line_idx as f32 * line_h;

                // Breakpoint dot
                if state.script_editor.breakpoints.has_breakpoint(line_num) {
                    let dot_center =
                        Pos2::new(gutter_rect.left() + gutter_w * 0.5, y + line_h * 0.5);
                    painter.circle_filled(
                        dot_center,
                        4.0 * h_scale.min(1.0),
                        Color32::from_rgb(224, 108, 117),
                    );
                }

                // Paused line highlight
                if paused_line == Some(line_num) {
                    let hl_rect = Rect::from_min_size(
                        Pos2::new(gutter_w, y),
                        Vec2::new(editor_rect.width() - gutter_w, line_h),
                    );
                    painter.add(Shape::rect_filled(
                        hl_rect,
                        Rounding::ZERO,
                        Color32::from_rgb(42, 45, 56),
                    ));
                    painter.text(
                        Pos2::new(gutter_rect.left() + 8.0 * w_scale, y + line_h * 0.5),
                        Align2::LEFT_CENTER,
                        "▶",
                        FontId::proportional(font_sz * 0.7),
                        Color32::from_rgb(229, 192, 123),
                    );
                }

                // Line number
                let num_text = format!("{:>4}", line_num);
                painter.text(
                    Pos2::new(gutter_rect.right() - 6.0 * w_scale, y + line_h * 0.5),
                    Align2::RIGHT_CENTER,
                    &num_text,
                    FontId::monospace(font_sz * 0.85),
                    Color32::from_gray(60),
                );

                // Clickable gutter for breakpoints
                let gutter_click_rect = Rect::from_min_size(
                    Pos2::new(gutter_rect.left(), y),
                    Vec2::new(gutter_w, line_h),
                );
                let bp_id = Id::new("bp_gutter").with(line_idx as u64);
                let bp_resp = ui.interact(gutter_click_rect, bp_id, egui::Sense::click());
                if bp_resp.clicked() {
                    state.script_editor.breakpoints.toggle(line_num);
                    if state.script_editor.breakpoints.has_breakpoint(line_num) {
                        state
                            .script_editor
                            .output
                            .debug(&format!("断点已设置: 第 {} 行", line_num), "debugger");
                    } else {
                        state
                            .script_editor
                            .output
                            .debug(&format!("断点已清除: 第 {} 行", line_num), "debugger");
                    }
                }
            }

            // Syntax highlighted code rendered as a label
            let highlighted = highlight(&content, language, font_id);
            ui.add(egui::Label::new(highlighted).extend());
        });
}

fn draw_output_panel(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    rect: Rect,
    _font_sz: f32,
    small_font: f32,
    w_scale: f32,
    _h_scale: f32,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(30, 30, 34),
    ));

    // Toolbar
    let toolbar_h = 26.0;
    let toolbar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), toolbar_h));
    painter.add(Shape::rect_filled(
        toolbar_rect,
        Rounding::ZERO,
        Color32::from_rgb(26, 26, 30),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), toolbar_rect.bottom()),
            Pos2::new(rect.right(), toolbar_rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    // Level filter buttons
    let levels = [
        (OutputLevel::Debug, "全部"),
        (OutputLevel::Info, "信息"),
        (OutputLevel::Warning, "警告"),
        (OutputLevel::Error, "错误"),
    ];
    let mut fx = toolbar_rect.left() + 8.0 * w_scale;
    for (level, label) in &levels {
        let text_w = label.len() as f32 * 7.0 * w_scale;
        let btn_rect = Rect::from_min_size(
            Pos2::new(fx, toolbar_rect.top()),
            Vec2::new(text_w + 16.0 * w_scale, toolbar_rect.height()),
        );
        let id = Id::new("output_filter").with(*level as u64);
        let response = ui.interact(btn_rect, id, egui::Sense::click());

        let is_active = state.script_editor.output.filter_level == *level;
        painter.text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            *label,
            FontId::proportional(small_font),
            if is_active {
                level.color()
            } else {
                Color32::from_gray(80)
            },
        );

        if response.clicked() {
            state.script_editor.output.filter_level = *level;
        }
        fx += text_w + 20.0 * w_scale;
    }

    // Clear button
    let clear_rect = Rect::from_min_size(
        Pos2::new(toolbar_rect.right() - 50.0 * w_scale, toolbar_rect.top()),
        Vec2::new(44.0 * w_scale, toolbar_rect.height()),
    );
    let clear_id = Id::new("output_clear");
    let clear_resp = ui.interact(clear_rect, clear_id, egui::Sense::click());
    painter.text(
        clear_rect.center(),
        Align2::CENTER_CENTER,
        "清除",
        FontId::proportional(small_font),
        if clear_resp.hovered() {
            Color32::from_rgb(224, 108, 117)
        } else {
            Color32::from_gray(100)
        },
    );
    if clear_resp.clicked() {
        state.script_editor.output.clear();
    }

    // Error/warning count
    let err_count = state.script_editor.output.error_count();
    let warn_count = state.script_editor.output.warning_count();
    if err_count > 0 || warn_count > 0 {
        let mut parts = Vec::new();
        if err_count > 0 {
            parts.push(format!("✗ {}", err_count));
        }
        if warn_count > 0 {
            parts.push(format!("⚠ {}", warn_count));
        }
        let count_text = parts.join("  ");
        painter.text(
            Pos2::new(clear_rect.left() - 100.0 * w_scale, toolbar_rect.center().y),
            Align2::RIGHT_CENTER,
            &count_text,
            FontId::proportional(small_font * 0.9),
            Color32::from_gray(100),
        );
    }

    // Output entries
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), toolbar_rect.bottom()),
        Vec2::new(rect.width(), rect.bottom() - toolbar_rect.bottom()),
    );

    let entries = state.script_editor.output.filtered_entries();

    ScrollArea::vertical()
        .id_salt("output_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(content_rect.width(), f32::INFINITY));

            for entry in &entries {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&entry.timestamp)
                            .font(FontId::monospace(small_font))
                            .color(Color32::from_gray(60)),
                    );
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new(entry.level.label())
                            .font(FontId::monospace(small_font))
                            .color(entry.level.color()),
                    );
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new(format!("[{}]", entry.source))
                            .font(FontId::monospace(small_font * 0.9))
                            .color(Color32::from_gray(70)),
                    );
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new(&entry.text)
                            .font(FontId::monospace(small_font))
                            .color(Color32::from_rgb(212, 212, 216)),
                    );
                });
            }
        });
}

fn draw_watcher_panel(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    rect: Rect,
    _font_sz: f32,
    small_font: f32,
    w_scale: f32,
    _h_scale: f32,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(30, 30, 34),
    ));

    // Toolbar
    let toolbar_h = 26.0;
    let toolbar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), toolbar_h));
    painter.add(Shape::rect_filled(
        toolbar_rect,
        Rounding::ZERO,
        Color32::from_rgb(26, 26, 30),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), toolbar_rect.bottom()),
            Pos2::new(rect.right(), toolbar_rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    // Variable count
    let count = state.script_editor.watcher.variable_count();
    painter.text(
        Pos2::new(toolbar_rect.left() + 8.0 * w_scale, toolbar_rect.center().y),
        Align2::LEFT_CENTER,
        format!("变量: {}", count),
        FontId::proportional(small_font),
        Color32::from_gray(100),
    );

    // Demo variables if empty
    if state.script_editor.watcher.variables.is_empty() {
        state
            .script_editor
            .watcher
            .set_variable("speed", "5.0", VarType::Number);
        state
            .script_editor
            .watcher
            .set_variable("jump_force", "10.0", VarType::Number);
        state
            .script_editor
            .watcher
            .set_variable("player_name", "\"Hero\"", VarType::String);
        state
            .script_editor
            .watcher
            .set_variable("is_grounded", "true", VarType::Bool);
        state
            .script_editor
            .watcher
            .set_variable("transform", "{...}", VarType::Table);
        state
            .script_editor
            .watcher
            .set_variable("on_update", "function", VarType::Function);
    }

    // Column headers
    let header_h = 22.0;
    let header_rect = Rect::from_min_size(
        Pos2::new(rect.left(), toolbar_rect.bottom()),
        Vec2::new(rect.width(), header_h),
    );
    painter.add(Shape::rect_filled(
        header_rect,
        Rounding::ZERO,
        Color32::from_rgb(24, 24, 28),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), header_rect.bottom()),
            Pos2::new(rect.right(), header_rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let col_w = rect.width() / 4.0;
    let headers = ["名称", "类型", "值", "操作"];
    for (i, header) in headers.iter().enumerate() {
        painter.text(
            Pos2::new(
                rect.left() + col_w * i as f32 + 8.0 * w_scale,
                header_rect.center().y,
            ),
            Align2::LEFT_CENTER,
            *header,
            FontId::proportional(small_font * 0.9),
            Color32::from_gray(70),
        );
    }

    // Variable list
    let list_rect = Rect::from_min_size(
        Pos2::new(rect.left(), header_rect.bottom()),
        Vec2::new(rect.width(), rect.bottom() - header_rect.bottom()),
    );

    let vars = state.script_editor.watcher.filtered_variables();

    ScrollArea::vertical()
        .id_salt("watcher_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(list_rect.width(), f32::INFINITY));

            for var in &vars {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&var.name)
                            .font(FontId::monospace(small_font))
                            .color(Color32::from_rgb(86, 182, 194)),
                    );

                    ui.add_space(col_w - var.name.len() as f32 * 6.0 - 16.0);

                    ui.label(
                        egui::RichText::new(var.var_type.label())
                            .font(FontId::monospace(small_font * 0.9))
                            .color(var.var_type.color()),
                    );

                    ui.add_space(16.0);

                    ui.label(
                        egui::RichText::new(&var.value)
                            .font(FontId::monospace(small_font))
                            .color(Color32::from_rgb(212, 212, 216)),
                    );
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_editor_state_new() {
        let state = ScriptEditorState::new();
        assert!(!state.open_scripts.is_empty());
        assert_eq!(state.active_script, 0);
    }

    #[test]
    fn test_open_close_script() {
        let mut state = ScriptEditorState::new();
        state.open_script("test.lua", "print('hello')");
        assert_eq!(state.open_scripts.len(), 2);
        state.close_script(1);
        assert_eq!(state.open_scripts.len(), 1);
    }

    #[test]
    fn test_open_duplicate_script() {
        let mut state = ScriptEditorState::new();
        state.open_script("test.lua", "v1");
        state.open_script("test.lua", "v2");
        assert_eq!(state.open_scripts.len(), 2);
        assert_eq!(state.open_scripts[1].content, "v1");
    }
}
