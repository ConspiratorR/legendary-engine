use engine_render::shape::Color;

#[derive(Debug, Clone)]
pub struct UiStyle {
    pub font_name: String,
    pub font_size: f32,
    pub text_color: Color,
    pub bg_color: Color,
    pub hover_color: Color,
    pub active_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub padding: [f32; 4],
    pub spacing: f32,
    pub corner_radius: f32,
    pub accent_color: Color,
    pub input_bg_color: Color,
    pub cursor_color: Color,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            font_name: "default".to_string(),
            font_size: 20.0,
            text_color: Color::new(0.95, 0.95, 0.95, 1.0),
            bg_color: Color::new(0.2, 0.2, 0.25, 1.0),
            hover_color: Color::new(0.3, 0.3, 0.35, 1.0),
            active_color: Color::new(0.15, 0.15, 0.2, 1.0),
            border_color: Color::new(0.4, 0.4, 0.45, 1.0),
            border_width: 1.0,
            padding: [8.0, 12.0, 8.0, 12.0],
            spacing: 6.0,
            corner_radius: 4.0,
            accent_color: Color::new(0.3, 0.6, 1.0, 1.0),
            input_bg_color: Color::new(0.12, 0.12, 0.15, 1.0),
            cursor_color: Color::new(0.9, 0.9, 0.9, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let style = UiStyle::default();
        assert_eq!(style.font_size, 20.0);
        assert_eq!(style.spacing, 6.0);
    }
}
