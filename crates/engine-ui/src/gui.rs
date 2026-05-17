use crate::skin::{ColorBlock, GuiSkin};
use egui::{Color32, Painter, Pos2, Rect, Rounding, Shape, Stroke};

pub struct Gui<'a> {
    pub ui: &'a egui::Ui,
    pub skin: &'a GuiSkin,
}

impl<'a> Gui<'a> {
    pub fn new(ui: &'a egui::Ui, skin: &'a GuiSkin) -> Gui<'a> {
        Gui { ui, skin }
    }

    pub fn draw_background(painter: &Painter, block: &ColorBlock, rect: Rect, rounding: Rounding) {
        painter.add(Shape::rect_filled(rect, rounding, block.background));
        if let Some(border_color) = block.border {
            painter.add(Shape::rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0, border_color),
            ));
        }
    }

    fn draw_text(
        painter: &Painter,
        block: &ColorBlock,
        rect: Rect,
        text: &str,
        font_id: &egui::FontId,
    ) {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            font_id.clone(),
            block.text,
        );
    }

    pub fn label(&mut self, rect: Rect, text: &str) {
        let painter = self.ui.painter_at(rect);
        let block = &self.skin.label.normal;
        Self::draw_background(&painter, block, rect, self.skin.label.border);
        Self::draw_text(&painter, block, rect, text, &self.skin.font);
    }

    pub fn button(&mut self, rect: Rect, text: &str) -> bool {
        let id = egui::Id::new("gui_btn")
            .with(rect.min.x as u64)
            .with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());

        let block = if response.clicked() {
            &self.skin.button.active
        } else if response.hovered() {
            &self.skin.button.hover
        } else {
            &self.skin.button.normal
        };

        let painter = self.ui.painter_at(rect);
        Self::draw_background(&painter, block, rect, self.skin.button.border);
        Self::draw_text(&painter, block, rect, text, &self.skin.font);
        response.clicked()
    }

    pub fn repeat_button(&mut self, rect: Rect, text: &str) -> bool {
        let id = egui::Id::new("gui_rpt")
            .with(rect.min.x as u64)
            .with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());

        let is_down = response.is_pointer_button_down_on();
        let block = if is_down {
            &self.skin.button.active
        } else if response.hovered() {
            &self.skin.button.hover
        } else {
            &self.skin.button.normal
        };

        let painter = self.ui.painter_at(rect);
        Self::draw_background(&painter, block, rect, self.skin.button.border);
        Self::draw_text(&painter, block, rect, text, &self.skin.font);
        is_down
    }

    pub fn box_(&mut self, rect: Rect, text: &str) {
        let painter = self.ui.painter_at(rect);
        let block = &self.skin.box_.normal;
        Self::draw_background(&painter, block, rect, self.skin.box_.border);
        Self::draw_text(&painter, block, rect, text, &self.skin.font);
    }

    pub fn separator(&mut self, rect: Rect) {
        let painter = self.ui.painter_at(rect);
        let center_y = rect.center().y;
        painter.add(Shape::line(
            vec![
                Pos2::new(rect.left(), center_y),
                Pos2::new(rect.right(), center_y),
            ],
            Stroke::new(1.0, Color32::from_gray(100)),
        ));
    }

    pub fn text_field(&mut self, rect: Rect, text: &mut String, id_salt: &str) {
        let widget_id = egui::Id::new(id_salt).with("field");
        let response = self.ui.interact(rect, widget_id, egui::Sense::click());

        let block = if response.has_focus() {
            &self.skin.text_field.focused
        } else if response.hovered() {
            &self.skin.text_field.hover
        } else {
            &self.skin.text_field.normal
        };

        let painter = self.ui.painter_at(rect);
        Self::draw_background(&painter, block, rect, self.skin.text_field.border);
        painter.text(egui::pos2(rect.left() + 4.0, rect.center().y), egui::Align2::LEFT_CENTER, text.as_str(), self.skin.font.clone(), block.text);

        if response.has_focus() {
            let mut chars_modified = false;
            self.ui.ctx().input(|i| {
                for event in &i.events {
                    if let egui::Event::Text(c) = event {
                        text.push_str(c);
                        chars_modified = true;
                    }
                }
            });
            if chars_modified {
                self.ui.ctx().request_repaint();
            }
        }

        if response.clicked() {
            self.ui.ctx().memory_mut(|mem| mem.request_focus(widget_id));
        }
    }

    pub fn toggle(&mut self, rect: Rect, value: &mut bool, label: &str) {
        let id = egui::Id::new("gui_tog")
            .with(rect.min.x as u64)
            .with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());
        let block = &self.skin.toggle.normal;

        let painter = self.ui.painter_at(rect);

        let check_size = rect.height();
        let check_rect = Rect::from_min_size(rect.left_top(), egui::vec2(check_size, check_size));
        Self::draw_background(
            &painter,
            &ColorBlock {
                background: if *value {
                    Color32::from_rgb(60, 120, 200)
                } else {
                    Color32::from_gray(40)
                },
                text: Color32::WHITE,
                border: Some(Color32::from_gray(100)),
            },
            check_rect,
            Rounding::same(3.0),
        );

        if *value {
            painter.text(
                check_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✓",
                self.skin.font.clone(),
                Color32::WHITE,
            );
        }

        let label_rect = Rect::from_min_max(
            egui::pos2(check_rect.right() + 4.0, rect.top()),
            rect.right_bottom(),
        );
        Self::draw_text(&painter, block, label_rect, label, &self.skin.font);

        if response.clicked() {
            *value = !*value;
        }
    }

    pub fn slider(&mut self, rect: Rect, value: &mut f32, min: f32, max: f32) {
        let id = egui::Id::new("gui_sld")
            .with(rect.min.x as u64)
            .with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click_and_drag());

        let painter = self.ui.painter_at(rect);

        let range = max - min;
        if range.abs() < f32::EPSILON {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, "—", self.skin.font.clone(), Color32::GRAY);
            return;
        }

        let t = ((*value - min) / range).clamp(0.0, 1.0);

        // Background track
        Self::draw_background(&painter, &self.skin.slider.normal, rect, Rounding::same(2.0));

        // Filled portion
        let fill_rect = Rect::from_min_size(rect.left_top(), egui::vec2(rect.width() * t, rect.height()));
        painter.add(Shape::rect_filled(
            fill_rect, Rounding::same(2.0), Color32::from_rgb(60, 120, 200),
        ));

        // Thumb
        let thumb_x = rect.left() + t * rect.width();
        let thumb_rect = Rect::from_center_size(
            egui::pos2(thumb_x, rect.center().y), egui::vec2(6.0, rect.height() + 4.0),
        );
        painter.add(Shape::rect_filled(thumb_rect, Rounding::same(3.0), Color32::WHITE));

        // Drag handling
        if response.dragged() {
            let delta = response.drag_delta();
            let new_t = t + delta.x / rect.width();
            *value = (min + new_t * range).clamp(min, max);
        }

        // Value label
        painter.text(
            egui::pos2(rect.center().x, rect.center().y),
            egui::Align2::CENTER_CENTER,
            format!("{:.2}", *value),
            self.skin.font.clone(),
            Color32::WHITE,
        );
    }

    pub fn toolbar(&mut self, rect: Rect, selected: &mut usize, texts: &[&str]) {
        let painter = self.ui.painter_at(rect);
        let n = texts.len();
        if n == 0 {
            return;
        }
        let btn_w = rect.width() / n as f32;

        for (i, text) in texts.iter().enumerate() {
            let btn_rect = Rect::from_min_size(
                egui::pos2(rect.left() + i as f32 * btn_w, rect.top()),
                egui::vec2(btn_w, rect.height()),
            );

            let id = egui::Id::new("gui_tb")
                .with(i as u64)
                .with(rect.min.y as u64);
            let response = self.ui.interact(btn_rect, id, egui::Sense::click());

            let block = if *selected == i {
                &self.skin.toolbar.active
            } else if response.hovered() {
                &self.skin.toolbar.hover
            } else {
                &self.skin.toolbar.normal
            };

            Self::draw_background(&painter, block, btn_rect, self.skin.toolbar.border);
            painter.text(
                btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                *text,
                self.skin.font.clone(),
                block.text,
            );

            if response.clicked() {
                *selected = i;
            }
        }
    }

    pub fn separator_h(&mut self, rect: Rect) {
        let painter = self.ui.painter_at(rect);
        let y = rect.center().y;
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0, Color32::from_gray(60)),
        ));
    }

    pub fn colored_label(&mut self, rect: Rect, text: &str, color: Color32) {
        let painter = self.ui.painter_at(rect);
        painter.text(
            egui::pos2(rect.left(), rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            self.skin.font.clone(),
            color,
        );
    }

    pub fn status_item(&mut self, rect: Rect, text: &str, dot_color: Color32) {
        let painter = self.ui.painter_at(rect);
        let dot_r = 4.0;
        let dot_center = egui::pos2(rect.left() + dot_r + 2.0, rect.center().y);
        painter.add(Shape::circle_filled(dot_center, dot_r, dot_color));
        painter.text(
            egui::pos2(dot_center.x + dot_r + 6.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            self.skin.font.clone(),
            self.skin.label.normal.text,
        );
    }

    pub fn panel_header(&mut self, rect: Rect, title: &str) -> Rect {
        let painter = self.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
        painter.text(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(12.0),
            Color32::from_gray(90),
        );
        let line_y = rect.bottom() - 1.0;
        painter.add(Shape::line(
            vec![Pos2::new(rect.left(), line_y), Pos2::new(rect.right(), line_y)],
            Stroke::new(1.0, Color32::from_rgb(45, 45, 53)),
        ));
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom()),
            egui::vec2(rect.width(), 0.0),
        )
    }

    pub fn checkbox(&mut self, rect: Rect, label: &str, checked: &mut bool) {
        let id = egui::Id::new("gui_chk").with(rect.min.x as u64).with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());

        let box_size = rect.height() - 4.0;
        let box_rect = Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, rect.top() + 2.0),
            egui::vec2(box_size, box_size),
        );

        let painter = self.ui.painter_at(rect);
        let bg = if *checked { Color32::from_rgb(0, 212, 170) } else { Color32::from_gray(40) };
        painter.add(Shape::rect_filled(box_rect, Rounding::same(3.0), bg));
        painter.add(Shape::rect_stroke(box_rect, Rounding::same(3.0), Stroke::new(1.0, Color32::from_gray(100))));

        if *checked {
            painter.text(box_rect.center(), egui::Align2::CENTER_CENTER, "✓", self.skin.font.clone(), Color32::from_rgb(13, 13, 15));
        }

        painter.text(
            egui::pos2(box_rect.right() + 6.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            self.skin.font.clone(),
            self.skin.label.normal.text,
        );

        if response.clicked() {
            *checked = !*checked;
        }
    }

    pub fn selection_grid(
        &mut self,
        rect: Rect,
        selected: &mut usize,
        texts: &[&str],
        cols: usize,
    ) {
        let n = texts.len();
        if n == 0 {
            return;
        }
        let cols = cols.max(1);
        let rows = n.div_ceil(cols);
        let cell_w = rect.width() / cols as f32;
        let cell_h = rect.height() / rows as f32;

        for (i, text) in texts.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let cell_rect = Rect::from_min_size(
                egui::pos2(
                    rect.left() + col as f32 * cell_w,
                    rect.top() + row as f32 * cell_h,
                ),
                egui::vec2(cell_w, cell_h),
            );

            let id = egui::Id::new("gui_sg").with(i as u64);
            let response = self.ui.interact(cell_rect, id, egui::Sense::click());

            let block = if *selected == i {
                &self.skin.selection_grid.active
            } else if response.hovered() {
                &self.skin.selection_grid.hover
            } else {
                &self.skin.selection_grid.normal
            };

            let painter = self.ui.painter_at(cell_rect);
            Self::draw_background(
                &painter,
                block,
                cell_rect,
                self.skin.selection_grid.border,
            );
            painter.text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                *text,
                self.skin.font.clone(),
                block.text,
            );

            if response.clicked() {
                *selected = i;
            }
        }
    }

    pub fn menu_bar(&mut self, rect: Rect, items: &[&str]) -> Option<usize> {
        if items.is_empty() {
            return None;
        }
        let painter = self.ui.painter_at(rect);
        painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));

        let n = items.len() as f32;
        let item_w = rect.width() / n;

        for (i, item) in items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                egui::pos2(rect.left() + i as f32 * item_w, rect.top()),
                egui::vec2(item_w, rect.height()),
            );

            let id = egui::Id::new("gui_menu").with(i as u64);
            let response = self.ui.interact(item_rect, id, egui::Sense::click());

            if response.hovered() {
                painter.add(Shape::rect_filled(item_rect, Rounding::ZERO, Color32::from_rgb(30, 30, 34)));
            }

            let text_color = if response.hovered() {
                Color32::from_rgb(232, 232, 236)
            } else {
                Color32::from_gray(152)
            };
            painter.text(
                item_rect.center(),
                egui::Align2::CENTER_CENTER,
                *item,
                egui::FontId::proportional(13.0),
                text_color,
            );

            if response.clicked() {
                return Some(i);
            }
        }
        None
    }

    pub fn tool_button(&mut self, rect: Rect, label: &str, active: bool) -> bool {
        let id = egui::Id::new("gui_tbtn").with(rect.min.x as u64).with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());

        let painter = self.ui.painter_at(rect);

        if active {
            painter.add(Shape::rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(0, 212, 170)));
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, self.skin.font.clone(), Color32::from_rgb(13, 13, 15));
        } else if response.hovered() {
            painter.add(Shape::rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(30, 30, 34)));
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, self.skin.font.clone(), Color32::from_gray(152));
        } else {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, self.skin.font.clone(), Color32::from_gray(152));
        }

        response.clicked()
    }

    pub fn tab(&mut self, rect: Rect, label: &str, active: bool) -> bool {
        let id = egui::Id::new("gui_tab").with(rect.min.x as u64).with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());

        let painter = self.ui.painter_at(rect);

        if active {
            painter.add(Shape::rect_filled(rect, Rounding::ZERO, Color32::from_rgb(22, 22, 25)));
            let line_rect = Rect::from_min_size(
                egui::pos2(rect.left(), rect.bottom() - 2.0),
                egui::vec2(rect.width(), 2.0),
            );
            painter.add(Shape::rect_filled(line_rect, Rounding::ZERO, Color32::from_rgb(0, 212, 170)));
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), Color32::from_rgb(0, 212, 170));
        } else {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), Color32::from_gray(90));
        }

        response.clicked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::GuiSkin;
    use egui::Pos2;

    fn run_in_ui(mut f: impl FnMut(&mut Gui)) {
        let ctx = egui::Context::default();
        let skin = GuiSkin::default();
        let f_ref = &mut f;
        let _ = ctx.run(egui::RawInput::default(), move |ctx| {
            egui::Area::new(egui::Id::new("test_area")).show(ctx, |ui| {
                let mut gui = Gui::new(ui, &skin);
                f_ref(&mut gui);
            });
        });
    }

    #[test]
    fn test_gui_constructs_and_labels() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 20.0));
            gui.label(rect, "Hello");
        });
    }

    #[test]
    fn test_button_returns_false_without_click() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 40.0), egui::vec2(100.0, 20.0));
            let clicked = gui.button(rect, "Click");
            assert!(!clicked, "Button should not be clicked without input");
        });
    }

    #[test]
    fn test_box_draws_without_panic() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 70.0), egui::vec2(100.0, 20.0));
            gui.box_(rect, "Boxed");
        });
    }

    #[test]
    fn test_separator_draws_without_panic() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 100.0), egui::vec2(100.0, 4.0));
            gui.separator(rect);
        });
    }

    #[test]
    fn test_repeat_button_returns_false_without_input() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 110.0), egui::vec2(100.0, 20.0));
            let down = gui.repeat_button(rect, "Hold");
            assert!(!down, "Repeat button should not be down without input");
        });
    }

    #[test]
    fn test_text_field_ignores_input_without_focus() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 160.0), egui::vec2(200.0, 22.0));
            let mut text = String::from("hello");
            gui.text_field(rect, &mut text, "test1");
            assert_eq!(text, "hello", "text should not change without focus/input");
        });
    }

    #[test]
    fn test_toggle_default_not_checked() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 190.0), egui::vec2(150.0, 22.0));
            let mut val = false;
            gui.toggle(rect, &mut val, "Option");
            assert!(!val, "toggle should remain false without click");
        });
    }

    #[test]
    fn test_toggle_checked_state() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 220.0), egui::vec2(150.0, 22.0));
            let mut val = true;
            gui.toggle(rect, &mut val, "Option");
            assert!(val, "toggle should remain true when initialized true");
        });
    }

    #[test]
    fn test_slider_default_value() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 250.0), egui::vec2(200.0, 22.0));
            let mut val = 0.5;
            gui.slider(rect, &mut val, 0.0, 1.0);
            assert!(
                (val - 0.5).abs() < f32::EPSILON,
                "slider value should remain unchanged without drag"
            );
        });
    }

    #[test]
    fn test_toolbar_empty_no_panic() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 280.0), egui::vec2(300.0, 24.0));
            let mut sel = 0;
            gui.toolbar(rect, &mut sel, &[]);
            assert_eq!(sel, 0);
        });
    }

    #[test]
    fn test_toolbar_initial_selection() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 310.0), egui::vec2(300.0, 24.0));
            let mut sel = 1;
            gui.toolbar(rect, &mut sel, &["A", "B", "C"]);
            assert_eq!(sel, 1, "selection unchanged without click");
        });
    }

    #[test]
    fn test_selection_grid_empty_no_panic() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 340.0), egui::vec2(200.0, 100.0));
            let mut sel = 0;
            gui.selection_grid(rect, &mut sel, &[], 2);
            assert_eq!(sel, 0);
        });
    }

    #[test]
    fn test_selection_grid_single_row() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 450.0), egui::vec2(200.0, 30.0));
            let mut sel = 0;
            gui.selection_grid(rect, &mut sel, &["X", "Y", "Z"], 3);
            assert_eq!(sel, 0);
        });
    }

    #[test]
    fn test_separator_h_draws_without_panic() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 4.0));
            gui.separator_h(rect);
        });
    }

    #[test]
    fn test_colored_label_draws_without_panic() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(100.0, 20.0));
            gui.colored_label(rect, "Hello", Color32::RED);
        });
    }

    #[test]
    fn test_checkbox_default_not_checked() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 30.0), egui::vec2(150.0, 22.0));
            let mut checked = false;
            gui.checkbox(rect, "Shadow", &mut checked);
            assert!(!checked, "checkbox should remain false without click");
        });
    }

    #[test]
    fn test_checkbox_checked_state() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 60.0), egui::vec2(150.0, 22.0));
            let mut checked = true;
            gui.checkbox(rect, "Shadow", &mut checked);
            assert!(checked, "checkbox should remain true when initialized true");
        });
    }

    #[test]
    fn test_panel_header_returns_content_rect() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 90.0), egui::vec2(200.0, 36.0));
            let content_rect = gui.panel_header(rect, "层级");
            assert!(content_rect.left() >= rect.left());
            assert!(content_rect.top() >= rect.bottom());
        });
    }

    #[test]
    fn test_menu_bar_returns_none_without_click() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 120.0), egui::vec2(400.0, 32.0));
            let result = gui.menu_bar(rect, &["文件", "编辑", "视图"]);
            assert!(result.is_none(), "menu_bar should return None without click");
        });
    }

    #[test]
    fn test_tool_button_returns_false_without_click() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 160.0), egui::vec2(32.0, 32.0));
            let clicked = gui.tool_button(rect, "↖", false);
            assert!(!clicked);
        });
    }

    #[test]
    fn test_tool_button_active_state() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(50.0, 160.0), egui::vec2(32.0, 32.0));
            let clicked = gui.tool_button(rect, "↔", true);
            assert!(!clicked, "active tool_button should not auto-click");
        });
    }

    #[test]
    fn test_tab_returns_false_without_click() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 200.0), egui::vec2(60.0, 32.0));
            let clicked = gui.tab(rect, "场景", false);
            assert!(!clicked);
        });
    }

    #[test]
    fn test_tab_active_does_not_auto_click() {
        run_in_ui(|gui| {
            let rect = Rect::from_min_size(Pos2::new(80.0, 200.0), egui::vec2(60.0, 32.0));
            let clicked = gui.tab(rect, "游戏", true);
            assert!(!clicked);
        });
    }
}
