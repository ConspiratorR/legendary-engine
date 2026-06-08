use super::context::UiContext;

impl UiContext<'_> {
    pub fn slider(&mut self, label: &str, value: &mut f32, min: f32, max: f32) {
        let _id = self.next_id();
        let line_height = self.style.font_size * 1.2;
        let slider_h = 8.0;
        let slider_w = 150.0;
        let handle_r = 6.0;

        let label_text = format!("{}: {:.1}", label, value);
        let _label_w = self.estimate_text_width(&label_text);

        let x = self.cursor[0];
        let y = self.cursor[1] + line_height * 0.5 - slider_h * 0.5;

        self.shape_painter
            .rect([x, y], [slider_w, slider_h], self.style.input_bg_color);

        let t = (*value - min) / (max - min);
        let handle_x = x + t * slider_w;

        self.shape_painter.circle(
            [handle_x, y + slider_h * 0.5],
            handle_r,
            self.style.accent_color,
        );

        let text_x = x + slider_w + self.style.spacing;
        let text_y = self.cursor[1] + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            &label_text,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [text_x, text_y],
        );

        let track_hovered = self.mouse_in_rect(
            x - handle_r,
            y - handle_r,
            slider_w + handle_r * 2.0,
            slider_h + handle_r * 2.0,
        );
        if track_hovered && self.mouse_down {
            let t = ((self.mouse_pos[0] - x) / slider_w).clamp(0.0, 1.0);
            *value = min + t * (max - min);
        }

        let total_h = line_height + self.style.padding[0] + self.style.padding[2];
        self.advance(total_h);
    }
}
