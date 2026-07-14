//! GUIContent class (matches Unity's GUIContent).

/// Content for GUI elements (matches Unity's `GUIContent`).
#[derive(Debug, Clone)]
pub struct GUIContent {
    pub text: String,
    pub image: Option<String>,
    pub tooltip: String,
}

impl GUIContent {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            image: None,
            tooltip: String::new(),
        }
    }

    pub fn new_with_tooltip(text: &str, tooltip: &str) -> Self {
        Self {
            text: text.to_string(),
            image: None,
            tooltip: tooltip.to_string(),
        }
    }

    pub fn new_with_image(text: &str, image: &str) -> Self {
        Self {
            text: text.to_string(),
            image: Some(image.to_string()),
            tooltip: String::new(),
        }
    }

    pub fn none() -> Self {
        Self {
            text: String::new(),
            image: None,
            tooltip: String::new(),
        }
    }

    pub fn Text(&self) -> &str {
        &self.text
    }
    pub fn Tooltip(&self) -> &str {
        &self.tooltip
    }
}

impl Default for GUIContent {
    fn default() -> Self {
        Self::none()
    }
}

impl From<&str> for GUIContent {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for GUIContent {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}
