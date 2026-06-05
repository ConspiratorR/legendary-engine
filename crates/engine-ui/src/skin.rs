//! Skin and style data structures for the immediate-mode GUI.
//!
//! A [`GuiSkin`] defines the visual appearance of every widget type by
//! mapping each to a [`GuiStyle`]. Each style contains [`ColorBlock`]s for
//! the normal, hover, active, and focused states, plus border rounding,
//! margins, and font size.

use egui::{Color32, Margin, Rounding};

/// Visual theme for the GUI, defining styles for each widget type.
#[derive(Clone, Debug)]
pub struct GuiSkin {
    /// Style for labels.
    pub label: GuiStyle,
    /// Style for buttons.
    pub button: GuiStyle,
    /// Style for box containers.
    pub box_: GuiStyle,
    /// Style for text fields.
    pub text_field: GuiStyle,
    /// Style for toggles.
    pub toggle: GuiStyle,
    /// Style for windows.
    pub window: GuiStyle,
    /// Style for sliders.
    pub slider: GuiStyle,
    /// Style for toolbars.
    pub toolbar: GuiStyle,
    /// Style for selection grids.
    pub selection_grid: GuiStyle,
    /// The font used across all widgets.
    pub font: egui::FontId,
    /// Optional cursor icon override.
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

/// Style configuration for a single widget type.
#[derive(Clone, Debug)]
pub struct GuiStyle {
    /// Colors for the normal (idle) state.
    pub normal: ColorBlock,
    /// Colors for the hover state.
    pub hover: ColorBlock,
    /// Colors for the active (pressed) state.
    pub active: ColorBlock,
    /// Colors for the focused state.
    pub focused: ColorBlock,
    /// Corner rounding.
    pub border: Rounding,
    /// Inner margins.
    pub margins: Margin,
    /// Font size in points.
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

/// A set of colors for a single widget state (background, text, optional border).
#[derive(Clone, Debug)]
pub struct ColorBlock {
    /// Fill color.
    pub background: Color32,
    /// Text color.
    pub text: Color32,
    /// Optional border color.
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
