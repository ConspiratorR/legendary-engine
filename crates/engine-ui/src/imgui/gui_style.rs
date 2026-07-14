//! GUIStyle class (matches Unity's GUIStyle).

/// Style state for a GUI element (matches Unity's `GUIStyleState`).
#[derive(Debug, Clone)]
pub struct GUIStyleState {
    pub text_color: [u8; 4],
    pub background: Option<[u8; 4]>,
}

impl Default for GUIStyleState {
    fn default() -> Self {
        Self {
            text_color: [255, 255, 255, 255],
            background: None,
        }
    }
}

/// Image position options (matches Unity's `ImagePosition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePosition {
    ImageLeft,
    ImageAbove,
    TextOnly,
    ImageOnly,
}
impl Default for ImagePosition {
    fn default() -> Self {
        Self::ImageLeft
    }
}

/// Text alignment options (matches Unity's `TextAnchor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    UpperLeft,
    UpperCenter,
    UpperRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    LowerLeft,
    LowerCenter,
    LowerRight,
}
impl Default for TextAnchor {
    fn default() -> Self {
        Self::UpperLeft
    }
}

/// Font style options (matches Unity's `FontStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Bold,
    Italic,
    BoldAndItalic,
}
impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

/// A style for GUI elements (matches Unity's `GUIStyle`).
#[derive(Debug, Clone)]
pub struct GUIStyle {
    pub normal: GUIStyleState,
    pub hover: GUIStyleState,
    pub active: GUIStyleState,
    pub focused: GUIStyleState,
    pub font_size: i32,
    pub font_style: FontStyle,
    pub alignment: TextAnchor,
    pub word_wrap: bool,
    pub rich_text: bool,
    pub image_position: ImagePosition,
    pub fixed_width: f32,
    pub fixed_height: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub expand_width: bool,
    pub expand_height: bool,
    pub padding: [f32; 4],
    pub margin: [f32; 4],
    pub overflow: [f32; 4],
    pub border: [f32; 4],
    pub name: String,
}

impl Default for GUIStyle {
    fn default() -> Self {
        Self {
            normal: GUIStyleState::default(),
            hover: GUIStyleState::default(),
            active: GUIStyleState::default(),
            focused: GUIStyleState::default(),
            font_size: 0,
            font_style: FontStyle::default(),
            alignment: TextAnchor::default(),
            word_wrap: false,
            rich_text: false,
            image_position: ImagePosition::default(),
            fixed_width: 0.0,
            fixed_height: 0.0,
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            expand_width: false,
            expand_height: false,
            padding: [0.0; 4],
            margin: [0.0; 4],
            overflow: [0.0; 4],
            border: [0.0; 4],
            name: String::new(),
        }
    }
}

impl GUIStyle {
    pub fn CalcSize(&self, content_size: [f32; 2]) -> [f32; 2] {
        let mut w = content_size[0] + self.padding[0] + self.padding[1];
        let mut h = content_size[1] + self.padding[2] + self.padding[3];
        if self.fixed_width > 0.0 {
            w = self.fixed_width;
        }
        if self.fixed_height > 0.0 {
            h = self.fixed_height;
        }
        w = w.clamp(self.min_width, self.max_width);
        h = h.clamp(self.min_height, self.max_height);
        [w, h]
    }

    pub fn ContentRect(&self, rect: [f32; 4]) -> [f32; 4] {
        [
            rect[0] + self.padding[0],
            rect[1] + self.padding[2],
            rect[2] - self.padding[0] - self.padding[1],
            rect[3] - self.padding[2] - self.padding[3],
        ]
    }
}
