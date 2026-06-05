use crate::skin::GuiSkin;
use egui::{Align2, Color32, Id, LayerId, Order, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

/// Scoped layout helper for building GUIs with horizontal/vertical containers.
pub struct GuiLayout<'a> {
    /// The egui context.
    pub ctx: &'a egui::Context,
    /// The skin for widget rendering.
    pub skin: &'a GuiSkin,
}

/// A horizontal layout scope that auto-advances the cursor horizontally.
pub struct HorizontalScope<'a> {
    ctx: &'a egui::Context,
    skin: &'a GuiSkin,
    start: Pos2,
}

fn layer_id() -> LayerId {
    LayerId::new(Order::Foreground, Id::new("gui_layout"))
}

fn hovered(ctx: &egui::Context, rect: Rect) -> bool {
    ctx.pointer_interact_pos().is_some_and(|p| rect.contains(p))
}

fn clicked(ctx: &egui::Context, rect: Rect) -> bool {
    ctx.input(|i| {
        i.pointer.any_click() && i.pointer.press_origin().is_some_and(|p| rect.contains(p))
    })
}

fn painter_at(ctx: &egui::Context, rect: Rect) -> egui::Painter {
    egui::Painter::new(ctx.clone(), layer_id(), rect)
}

impl HorizontalScope<'_> {
    /// Draw a clickable button. Returns `true` when clicked.
    pub fn button(&mut self, text: &str) -> bool {
        let rect = Rect::from_min_size(self.start, Vec2::new(text.len() as f32 * 8.0 + 12.0, 22.0));
        let over = hovered(self.ctx, rect);
        let hit = over && clicked(self.ctx, rect);
        let block = if hit {
            &self.skin.button.active
        } else if over {
            &self.skin.button.hover
        } else {
            &self.skin.button.normal
        };
        let painter = painter_at(self.ctx, rect);
        crate::gui::Gui::draw_background(&painter, block, rect, self.skin.button.border);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            self.skin.font.clone(),
            block.text,
        );
        self.start.x = rect.right() + 4.0;
        hit
    }

    /// Draw a text label and advance the cursor.
    pub fn label(&mut self, text: &str) {
        let rect = Rect::from_min_size(self.start, Vec2::new(text.len() as f32 * 8.0 + 4.0, 22.0));
        let painter = painter_at(self.ctx, rect);
        let block = &self.skin.label.normal;
        crate::gui::Gui::draw_background(&painter, block, rect, self.skin.label.border);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            self.skin.font.clone(),
            block.text,
        );
        self.start.x = rect.right() + 4.0;
    }

    /// Advance the cursor by a fixed width.
    pub fn space(&mut self, width: f32) {
        self.start.x += width;
    }

    /// Advance the cursor by a small flexible spacing amount.
    pub fn flexible_space(&mut self) {
        self.start.x += 8.0;
    }

    /// Draw a read-only text field and advance the cursor horizontally.
    pub fn text_field(&mut self, text: &str, width: f32) {
        let rect = Rect::from_min_size(self.start, Vec2::new(width, 22.0));
        let block = &self.skin.text_field.normal;
        let painter = painter_at(self.ctx, rect);
        crate::gui::Gui::draw_background(&painter, block, rect, self.skin.text_field.border);
        painter.text(
            Pos2::new(rect.left() + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            self.skin.font.clone(),
            block.text,
        );
        self.start.x = rect.right() + 4.0;
    }

    /// Draw a toggle checkbox and advance the cursor horizontally.
    pub fn toggle(&mut self, value: &mut bool, text: &str) {
        let rect = Rect::from_min_size(self.start, Vec2::new(18.0 + text.len() as f32 * 8.0, 22.0));
        let over = hovered(self.ctx, rect);
        let hit = over && clicked(self.ctx, rect);
        let painter = painter_at(self.ctx, rect);
        let check_size = rect.height();
        let check_rect = Rect::from_min_size(rect.left_top(), Vec2::new(check_size, check_size));
        crate::gui::Gui::draw_background(
            &painter,
            &crate::skin::ColorBlock {
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
                Align2::CENTER_CENTER,
                "\u{2713}",
                self.skin.font.clone(),
                Color32::WHITE,
            );
        }
        if hit {
            *value = !*value;
        }
        self.start.x = rect.right() + 4.0;
    }
}

pub struct VerticalScope<'a> {
    ctx: &'a egui::Context,
    skin: &'a GuiSkin,
    start: Pos2,
}

impl VerticalScope<'_> {
    /// Draw a clickable button and advance the cursor vertically. Returns `true` when clicked.
    pub fn button(&mut self, text: &str) -> bool {
        let rect = Rect::from_min_size(self.start, Vec2::new(120.0, 22.0));
        let over = hovered(self.ctx, rect);
        let hit = over && clicked(self.ctx, rect);
        let block = if hit {
            &self.skin.button.active
        } else if over {
            &self.skin.button.hover
        } else {
            &self.skin.button.normal
        };
        let painter = painter_at(self.ctx, rect);
        crate::gui::Gui::draw_background(&painter, block, rect, self.skin.button.border);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            self.skin.font.clone(),
            block.text,
        );
        self.start.y = rect.bottom() + 2.0;
        hit
    }

    /// Draw a text label and advance the cursor vertically.
    pub fn label(&mut self, text: &str) {
        let rect = Rect::from_min_size(self.start, Vec2::new(120.0, 22.0));
        let painter = painter_at(self.ctx, rect);
        let block = &self.skin.label.normal;
        crate::gui::Gui::draw_background(&painter, block, rect, self.skin.label.border);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            self.skin.font.clone(),
            block.text,
        );
        self.start.y = rect.bottom() + 2.0;
    }

    /// Advance the cursor vertically by a fixed height.
    pub fn space(&mut self, height: f32) {
        self.start.y += height;
    }

    /// Advance the cursor vertically by a small flexible spacing amount.
    pub fn flexible_space(&mut self) {
        self.start.y += 4.0;
    }

    /// Draw a box container and advance the cursor vertically.
    pub fn box_(&mut self, text: &str, width: f32, height: f32) {
        let rect = Rect::from_min_size(self.start, Vec2::new(width, height));
        let painter = painter_at(self.ctx, rect);
        let block = &self.skin.box_.normal;
        crate::gui::Gui::draw_background(&painter, block, rect, self.skin.box_.border);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            self.skin.font.clone(),
            block.text,
        );
        self.start.y = rect.bottom() + 2.0;
    }

    /// Draw a read-only text field and advance the cursor vertically.
    pub fn text_field(&mut self, text: &str, width: f32) {
        let rect = Rect::from_min_size(self.start, Vec2::new(width, 22.0));
        let block = &self.skin.text_field.normal;
        let painter = painter_at(self.ctx, rect);
        crate::gui::Gui::draw_background(&painter, block, rect, self.skin.text_field.border);
        painter.text(
            Pos2::new(rect.left() + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            self.skin.font.clone(),
            block.text,
        );
        self.start.y = rect.bottom() + 2.0;
    }

    /// Draw a toggle checkbox and advance the cursor vertically.
    pub fn toggle(&mut self, value: &mut bool, text: &str) {
        let rect = Rect::from_min_size(self.start, Vec2::new(18.0 + text.len() as f32 * 8.0, 22.0));
        let over = hovered(self.ctx, rect);
        let hit = over && clicked(self.ctx, rect);
        let painter = painter_at(self.ctx, rect);
        let check_size = rect.height();
        let check_rect = Rect::from_min_size(rect.left_top(), Vec2::new(check_size, check_size));
        crate::gui::Gui::draw_background(
            &painter,
            &crate::skin::ColorBlock {
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
                Align2::CENTER_CENTER,
                "\u{2713}",
                self.skin.font.clone(),
                Color32::WHITE,
            );
        }
        if hit {
            *value = !*value;
        }
        self.start.y = rect.bottom() + 2.0;
    }

    /// Draw a draggable slider and advance the cursor vertically.
    pub fn slider(&mut self, value: &mut f32, min: f32, max: f32, width: f32) {
        let rect = Rect::from_min_size(self.start, Vec2::new(width, 22.0));
        let over = hovered(self.ctx, rect);
        let grabbed = over && self.ctx.input(|i| i.pointer.any_down());
        let drag_delta = self.ctx.input(|i| i.pointer.delta());
        let painter = painter_at(self.ctx, rect);
        let range = max - min;
        if range.abs() >= f32::EPSILON {
            let t = ((*value - min) / range).clamp(0.0, 1.0);
            crate::gui::Gui::draw_background(
                &painter,
                &self.skin.slider.normal,
                rect,
                Rounding::same(2.0),
            );
            let fill_rect =
                Rect::from_min_size(rect.left_top(), Vec2::new(rect.width() * t, rect.height()));
            painter.add(Shape::rect_filled(
                fill_rect,
                Rounding::same(2.0),
                Color32::from_rgb(60, 120, 200),
            ));
            let thumb_x = rect.left() + t * rect.width();
            let thumb_rect = Rect::from_center_size(
                Pos2::new(thumb_x, rect.center().y),
                Vec2::new(6.0, rect.height() + 4.0),
            );
            painter.add(Shape::rect_filled(
                thumb_rect,
                Rounding::same(3.0),
                Color32::WHITE,
            ));
            if grabbed {
                let new_t = t + drag_delta.x / rect.width();
                *value = (min + new_t * range).clamp(min, max);
            }
        }
        painter.text(
            Pos2::new(rect.center().x, rect.center().y),
            Align2::CENTER_CENTER,
            format!("{:.2}", *value),
            self.skin.font.clone(),
            Color32::WHITE,
        );
        self.start.y = rect.bottom() + 2.0;
    }

    /// Draw a horizontal separator and advance the cursor vertically.
    pub fn separator(&mut self) {
        let rect = Rect::from_min_size(self.start, Vec2::new(120.0, 4.0));
        let painter = painter_at(self.ctx, rect);
        let center_y = rect.center().y;
        painter.add(Shape::line(
            vec![
                Pos2::new(rect.left(), center_y),
                Pos2::new(rect.right(), center_y),
            ],
            Stroke::new(1.0_f32, Color32::from_gray(100)),
        ));
        self.start.y = rect.bottom();
    }

    pub fn horizontal(&mut self, f: impl FnOnce(&mut HorizontalScope)) {
        let mut h = HorizontalScope {
            ctx: self.ctx,
            skin: self.skin,
            start: Pos2::new(self.start.x, self.start.y),
        };
        f(&mut h);
        self.start.y = h.start.y + 22.0 + 2.0;
    }
}

impl<'a> GuiLayout<'a> {
    /// Create a new layout helper from an egui context and skin.
    pub fn new(ctx: &'a egui::Context, skin: &'a GuiSkin) -> GuiLayout<'a> {
        GuiLayout { ctx, skin }
    }

    /// Open a horizontal layout scope.
    pub fn horizontal(&mut self, f: impl FnOnce(&mut HorizontalScope)) {
        let mut scope = HorizontalScope {
            ctx: self.ctx,
            skin: self.skin,
            start: self.ctx.screen_rect().left_top(),
        };
        f(&mut scope);
    }

    /// Open a vertical layout scope.
    pub fn vertical(&mut self, f: impl FnOnce(&mut VerticalScope)) {
        let mut scope = VerticalScope {
            ctx: self.ctx,
            skin: self.skin,
            start: self.ctx.screen_rect().left_top(),
        };
        f(&mut scope);
    }

    /// Open a scrollable vertical view within the given rectangle.
    pub fn scroll_view(
        &mut self,
        rect: Rect,
        scroll: &mut Vec2,
        f: impl FnOnce(&mut VerticalScope),
    ) {
        let offset = *scroll;
        let mut vs = VerticalScope {
            ctx: self.ctx,
            skin: self.skin,
            start: rect.left_top() - offset,
        };
        f(&mut vs);
    }

    /// Draw a window with a title bar and open a vertical scope for its content.
    pub fn window(&mut self, title: &str, rect: &mut Rect, f: impl FnOnce(&mut VerticalScope)) {
        let painter = painter_at(self.ctx, *rect);
        crate::gui::Gui::draw_background(
            &painter,
            &self.skin.window.normal,
            *rect,
            self.skin.window.border,
        );
        let title_bar = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), 20.0));
        painter.text(
            title_bar.center(),
            Align2::CENTER_CENTER,
            title,
            self.skin.font.clone(),
            self.skin.window.normal.text,
        );

        let client_rect = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + 20.0),
            Vec2::new(rect.width(), rect.height() - 20.0),
        );
        let mut vs = VerticalScope {
            ctx: self.ctx,
            skin: self.skin,
            start: client_rect.left_top(),
        };
        f(&mut vs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::GuiSkin;

    fn run_layout(mut f: impl FnMut(&mut GuiLayout)) {
        let ctx = egui::Context::default();
        let skin = GuiSkin::default();
        let f_ref = &mut f;
        let _ = ctx.run(egui::RawInput::default(), move |ctx| {
            let mut layout = GuiLayout::new(ctx, &skin);
            f_ref(&mut layout);
        });
    }

    #[test]
    fn test_gui_layout_constructs() {
        let ctx = egui::Context::default();
        let skin = GuiSkin::default();
        let _layout = GuiLayout::new(&ctx, &skin);
    }

    #[test]
    fn test_horizontal_scope_advances_cursor() {
        run_layout(|layout| {
            layout.horizontal(|h| {
                let start_x = h.start.x;
                h.label("Hi");
                assert!(h.start.x > start_x, "cursor should advance after label");
            });
        });
    }

    #[test]
    fn test_vertical_scope_advances_cursor() {
        run_layout(|layout| {
            layout.vertical(|v| {
                let start_y = v.start.y;
                v.label("Hi");
                assert!(v.start.y > start_y, "cursor should advance after label");
            });
        });
    }

    #[test]
    fn test_window_creates_vertical_scope() {
        run_layout(|layout| {
            let mut rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(200.0, 300.0));
            layout.window("Test", &mut rect, |_v| {});
        });
    }
}
