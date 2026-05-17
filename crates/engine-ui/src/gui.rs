use egui::{Color32, Painter, Pos2, Rect, Rounding, Shape, Stroke};
use crate::skin::{ColorBlock, GuiSkin};

pub struct Gui<'a> {
    pub ui: &'a egui::Ui,
    pub skin: &'a GuiSkin,
}

impl<'a> Gui<'a> {
    pub fn new(ui: &'a egui::Ui, skin: &'a GuiSkin) -> Gui<'a> {
        Gui { ui, skin }
    }

    fn draw_background(painter: &Painter, block: &ColorBlock, rect: Rect, rounding: Rounding) {
        painter.add(Shape::rect_filled(rect, rounding, block.background));
        if let Some(border_color) = block.border {
            painter.add(Shape::rect_stroke(rect, rounding, Stroke::new(1.0, border_color)));
        }
    }

    fn draw_text(painter: &Painter, block: &ColorBlock, rect: Rect, text: &str, font_id: &egui::FontId) {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, font_id.clone(), block.text);
    }

    pub fn label(&mut self, rect: Rect, text: &str) {
        let painter = self.ui.painter_at(rect);
        let block = &self.skin.label.normal;
        Self::draw_background(&painter, block, rect, self.skin.label.border);
        Self::draw_text(&painter, block, rect, text, &self.skin.font);
    }

    pub fn button(&mut self, rect: Rect, text: &str) -> bool {
        let id = egui::Id::new("gui_btn").with(rect.min.x as u64).with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());

        let block = if response.clicked() { &self.skin.button.active }
                    else if response.hovered() { &self.skin.button.hover }
                    else { &self.skin.button.normal };

        let painter = self.ui.painter_at(rect);
        Self::draw_background(&painter, block, rect, self.skin.button.border);
        Self::draw_text(&painter, block, rect, text, &self.skin.font);
        response.clicked()
    }

    pub fn repeat_button(&mut self, rect: Rect, text: &str) -> bool {
        let id = egui::Id::new("gui_rpt").with(rect.min.x as u64).with(rect.min.y as u64);
        let response = self.ui.interact(rect, id, egui::Sense::click());

        let is_down = response.is_pointer_button_down_on();
        let block = if is_down { &self.skin.button.active }
                    else if response.hovered() { &self.skin.button.hover }
                    else { &self.skin.button.normal };

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
            vec![Pos2::new(rect.left(), center_y), Pos2::new(rect.right(), center_y)],
            Stroke::new(1.0, Color32::from_gray(100)),
        ));
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
}
