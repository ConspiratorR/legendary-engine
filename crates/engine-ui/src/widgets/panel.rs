use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    pub fn panel(&mut self, size: [f32; 2], f: impl FnOnce(&mut UiContext)) {
        let x = self.cursor[0];
        let y = self.cursor[1];

        self.shape_painter.rect([x, y], size, self.style.bg_color);

        if self.style.border_width > 0.0 {
            self.shape_painter.rect_stroked(
                [x, y],
                size,
                Color::TRANSPARENT,
                self.style.border_color,
                self.style.border_width,
            );
        }

        let saved_cursor = self.cursor;
        self.cursor = [x + self.style.padding[3], y + self.style.padding[0]];

        f(self);

        self.cursor = saved_cursor;
        self.advance(size[1]);
    }
}
