use super::context::UiContext;
use engine_render::shape::Color;

impl UiContext<'_> {
    #[allow(clippy::ptr_arg)]
    pub fn text_input(&mut self, label: &str, buffer: &mut String) {
        let id = self.next_id();
        let line_height = self.style.font_size * 1.2;
        let input_w = 200.0;
        let input_h = line_height + self.style.padding[0] + self.style.padding[2];

        let x = self.cursor[0];
        let y = self.cursor[1];

        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            label,
            &self.style.font_name,
            self.style.font_size * 0.8,
            self.style.text_color.to_array(),
            [x, y],
        );

        let input_y = y + line_height * 0.8 + 4.0;

        self.shape_painter
            .rect([x, input_y], [input_w, input_h], self.style.input_bg_color);

        let border_color = if self.focused_id == Some(id) {
            self.style.accent_color
        } else {
            self.style.border_color
        };
        self.shape_painter.rect_stroked(
            [x, input_y],
            [input_w, input_h],
            Color::TRANSPARENT,
            border_color,
            self.style.border_width,
        );

        let text_x = x + self.style.padding[3];
        let text_y = input_y + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            buffer,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        if self.focused_id == Some(id) {
            let cursor_x = text_x + self.estimate_text_width(buffer);
            self.shape_painter.rect(
                [cursor_x, text_y],
                [2.0, self.style.font_size],
                self.style.cursor_color,
            );
        }

        let hovered = self.mouse_in_rect(x, input_y, input_w, input_h);
        if hovered && self.mouse_clicked {
            self.focused_id = Some(id);
        } else if !hovered && self.mouse_clicked && self.focused_id == Some(id) {
            self.focused_id = None;
        }

        self.advance(input_h + line_height * 0.8 + 4.0 + self.style.spacing);
    }

    pub fn focused_input(&self) -> Option<u64> {
        self.focused_id
    }
}
