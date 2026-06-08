use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    pub fn button(&mut self, label: &str) -> bool {
        let _id = self.next_id();
        let line_height = self.style.font_size * 1.2;
        let text_w = self.estimate_text_width(label);
        let w = text_w + self.style.padding[1] + self.style.padding[3];
        let h = line_height + self.style.padding[0] + self.style.padding[2];
        let x = self.cursor[0];
        let y = self.cursor[1];

        let hovered = self.mouse_in_rect(x, y, w, h);
        let bg_color = if hovered && self.mouse_down {
            self.style.active_color
        } else if hovered {
            self.style.hover_color
        } else {
            self.style.bg_color
        };

        self.shape_painter
            .rounded_rect([x, y], [w, h], self.style.corner_radius, bg_color);

        if self.style.border_width > 0.0 {
            self.shape_painter.rect_stroked(
                [x, y],
                [w, h],
                Color::TRANSPARENT,
                self.style.border_color,
                self.style.border_width,
            );
        }

        let text_x = x + (w - text_w) * 0.5;
        let text_y = y + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            label,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        self.advance(h);
        hovered && self.mouse_clicked
    }
}
