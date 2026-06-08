use super::context::UiContext;

impl UiContext<'_> {
    pub fn label(&mut self, text: &str) {
        let x = self.cursor[0] + self.style.padding[3];
        let y = self.cursor[1] + self.style.padding[0];
        let _ = self.text_painter.draw_text(
            self.device,
            self.queue,
            text,
            &self.style.font_name,
            self.style.font_size,
            self.style.text_color.to_array(),
            [x, y],
        );
        let line_height = self.style.font_size * 1.2;
        self.advance(line_height + self.style.padding[0] + self.style.padding[2]);
    }
}
