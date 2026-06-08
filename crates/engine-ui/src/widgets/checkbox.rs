use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    pub fn checkbox(&mut self, label: &str, checked: &mut bool) {
        let _id = self.next_id();
        let box_size = self.style.font_size;
        let line_height = self.style.font_size * 1.2;
        let x = self.cursor[0];
        let y = self.cursor[1] + (line_height - box_size) * 0.5;

        self.shape_painter
            .rect([x, y], [box_size, box_size], self.style.input_bg_color);

        if self.style.border_width > 0.0 {
            self.shape_painter.rect_stroked(
                [x, y],
                [box_size, box_size],
                Color::TRANSPARENT,
                self.style.border_color,
                self.style.border_width,
            );
        }

        if *checked {
            let inset = box_size * 0.2;
            self.shape_painter.rect(
                [x + inset, y + inset],
                [box_size - inset * 2.0, box_size - inset * 2.0],
                self.style.accent_color,
            );
        }

        let text_x = x + box_size + self.style.spacing;
        let text_y = self.cursor[1] + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            label,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        let total_w = box_size + self.style.spacing + self.estimate_text_width(label);
        let hovered = self.mouse_in_rect(self.cursor[0], self.cursor[1], total_w, line_height);
        if hovered && self.mouse_clicked {
            *checked = !*checked;
        }

        self.advance(line_height + self.style.padding[0] + self.style.padding[2]);
    }
}
