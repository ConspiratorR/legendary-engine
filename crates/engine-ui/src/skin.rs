use egui::{Color32, Margin, Rounding};

#[derive(Clone)]
pub struct GuiSkin {
    pub label: GuiStyle,
    pub button: GuiStyle,
    pub box_: GuiStyle,
    pub text_field: GuiStyle,
    pub toggle: GuiStyle,
    pub window: GuiStyle,
    pub slider: GuiStyle,
    pub toolbar: GuiStyle,
    pub selection_grid: GuiStyle,
    pub font: egui::FontId,
    pub cursor: Option<egui::CursorIcon>,
}

impl Default for GuiSkin {
    fn default() -> Self {
        Self {
            label: GuiStyle::default(),
            button: GuiStyle {
                normal: ColorBlock {
                    background: Color32::from_gray(80),
                    text: Color32::WHITE,
                    border: None,
                },
                hover: ColorBlock {
                    background: Color32::from_gray(100),
                    text: Color32::WHITE,
                    border: None,
                },
                active: ColorBlock {
                    background: Color32::from_gray(120),
                    text: Color32::WHITE,
                    border: None,
                },
                focused: ColorBlock {
                    background: Color32::from_gray(80),
                    text: Color32::WHITE,
                    border: None,
                },
                ..Default::default()
            },
            box_: GuiStyle {
                normal: ColorBlock {
                    background: Color32::from_gray(50),
                    text: Color32::WHITE,
                    border: Some(Color32::from_gray(80)),
                },
                ..Default::default()
            },
            text_field: GuiStyle {
                normal: ColorBlock {
                    background: Color32::from_gray(40),
                    text: Color32::WHITE,
                    border: Some(Color32::from_gray(100)),
                },
                focused: ColorBlock {
                    background: Color32::from_gray(45),
                    text: Color32::WHITE,
                    border: Some(Color32::from_rgb(60, 120, 200)),
                },
                ..Default::default()
            },
            toggle: GuiStyle::default(),
            window: GuiStyle {
                normal: ColorBlock {
                    background: Color32::from_gray(55),
                    text: Color32::WHITE,
                    border: Some(Color32::from_gray(90)),
                },
                ..Default::default()
            },
            slider: GuiStyle::default(),
            toolbar: GuiStyle {
                normal: ColorBlock {
                    background: Color32::from_gray(70),
                    text: Color32::from_gray(180),
                    border: None,
                },
                active: ColorBlock {
                    background: Color32::from_gray(110),
                    text: Color32::WHITE,
                    border: None,
                },
                ..Default::default()
            },
            selection_grid: GuiStyle {
                normal: ColorBlock {
                    background: Color32::from_gray(60),
                    text: Color32::from_gray(180),
                    border: Some(Color32::from_gray(80)),
                },
                active: ColorBlock {
                    background: Color32::from_gray(100),
                    text: Color32::WHITE,
                    border: Some(Color32::from_rgb(60, 120, 200)),
                },
                ..Default::default()
            },
            font: egui::FontId::proportional(14.0),
            cursor: None,
        }
    }
}

#[derive(Clone)]
pub struct GuiStyle {
    pub normal: ColorBlock,
    pub hover: ColorBlock,
    pub active: ColorBlock,
    pub focused: ColorBlock,
    pub border: Rounding,
    pub margins: Margin,
    pub font_size: f32,
}

impl Default for GuiStyle {
    fn default() -> Self {
        Self {
            normal: ColorBlock {
                background: Color32::from_gray(60),
                text: Color32::WHITE,
                border: None,
            },
            hover: ColorBlock {
                background: Color32::from_gray(75),
                text: Color32::WHITE,
                border: None,
            },
            active: ColorBlock {
                background: Color32::from_gray(90),
                text: Color32::WHITE,
                border: None,
            },
            focused: ColorBlock {
                background: Color32::from_gray(65),
                text: Color32::WHITE,
                border: None,
            },
            border: Rounding::same(2.0),
            margins: Margin::symmetric(4.0, 2.0),
            font_size: 14.0,
        }
    }
}

#[derive(Clone)]
pub struct ColorBlock {
    pub background: Color32,
    pub text: Color32,
    pub border: Option<Color32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skin_default_exists() {
        let skin = GuiSkin::default();
        assert_eq!(skin.button.normal.text, Color32::WHITE);
    }

    #[test]
    fn test_style_default_fields() {
        let style = GuiStyle::default();
        assert!(style.font_size > 0.0);
    }

    #[test]
    fn test_color_block_default() {
        let block = ColorBlock {
            background: Color32::BLACK,
            text: Color32::WHITE,
            border: None,
        };
        assert_eq!(block.text, Color32::WHITE);
    }
}
