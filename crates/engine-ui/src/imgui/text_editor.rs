//! TextEditor class (matches Unity's TextEditor).

/// Text editor for IMGUI (matches Unity's `TextEditor`).
#[derive(Debug, Clone)]
pub struct TextEditor {
    /// The text content.
    pub text: String,
    /// Cursor position.
    pub cursor_index: usize,
    /// Selection start index.
    pub select_index: usize,
    /// Whether the editor is focused.
    pub focused: bool,
    /// Whether multiline.
    pub multiline: bool,
    /// Whether password mode.
    pub password: bool,
    /// Control ID.
    pub control_id: i32,
}

impl Default for TextEditor {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor_index: 0,
            select_index: 0,
            focused: false,
            multiline: false,
            password: false,
            control_id: 0,
        }
    }
}

impl TextEditor {
    /// Create a new text editor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy the selected text (matches `TextEditor.Copy`).
    pub fn Copy(&self) -> Option<String> {
        let start = self.cursor_index.min(self.select_index);
        let end = self.cursor_index.max(self.select_index);
        if start < end {
            Some(self.text[start..end].to_string())
        } else {
            None
        }
    }

    /// Cut the selected text (matches `TextEditor.Cut`).
    pub fn Cut(&mut self) -> Option<String> {
        let copied = self.Copy();
        if let Some(ref _text) = copied {
            let start = self.cursor_index.min(self.select_index);
            let end = self.cursor_index.max(self.select_index);
            self.text.drain(start..end);
            self.cursor_index = start;
            self.select_index = start;
        }
        copied
    }

    /// Paste text (matches `TextEditor.Paste`).
    pub fn Paste(&mut self, text: &str) {
        let start = self.cursor_index.min(self.select_index);
        let end = self.cursor_index.max(self.select_index);
        self.text.drain(start..end);
        self.text.insert_str(start, text);
        self.cursor_index = start + text.len();
        self.select_index = self.cursor_index;
    }

    /// Select all text (matches `TextEditor.SelectAll`).
    pub fn SelectAll(&mut self) {
        self.select_index = 0;
        self.cursor_index = self.text.len();
    }

    /// Move cursor to the beginning (matches `TextEditor.MoveToStart`).
    pub fn MoveToStart(&mut self) {
        self.cursor_index = 0;
        self.select_index = self.cursor_index;
    }

    /// Move cursor to the end (matches `TextEditor.MoveToEnd`).
    pub fn MoveToEnd(&mut self) {
        self.cursor_index = self.text.len();
        self.select_index = self.cursor_index;
    }

    /// Move cursor to the beginning of the line (matches `TextEditor.MoveToLineStart`).
    pub fn MoveToLineStart(&mut self) {
        let line_start = self.text[..self.cursor_index]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        self.cursor_index = line_start;
        self.select_index = self.cursor_index;
    }

    /// Move cursor to the end of the line (matches `TextEditor.MoveToLineEnd`).
    pub fn MoveToLineEnd(&mut self) {
        let line_end = self.text[self.cursor_index..]
            .find('\n')
            .map(|i| self.cursor_index + i)
            .unwrap_or(self.text.len());
        self.cursor_index = line_end;
        self.select_index = self.cursor_index;
    }

    /// Insert text at cursor (matches `TextEditor.Insert`).
    pub fn Insert(&mut self, text: &str) {
        let start = self.cursor_index.min(self.select_index);
        let end = self.cursor_index.max(self.select_index);
        self.text.drain(start..end);
        self.text.insert_str(start, text);
        self.cursor_index = start + text.len();
        self.select_index = self.cursor_index;
    }

    /// Delete character before cursor (matches `TextEditor.Delete`).
    pub fn Delete(&mut self) {
        if self.cursor_index < self.text.len() {
            self.text.remove(self.cursor_index);
        }
    }

    /// Backspace (matches `TextEditor.Backspace`).
    pub fn Backspace(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
            self.text.remove(self.cursor_index);
            self.select_index = self.cursor_index;
        }
    }

    /// Check if there is a selection.
    pub fn HasSelection(&self) -> bool {
        self.cursor_index != self.select_index
    }

    /// Get the selected text.
    pub fn SelectedText(&self) -> &str {
        let start = self.cursor_index.min(self.select_index);
        let end = self.cursor_index.max(self.select_index);
        &self.text[start..end]
    }

    /// Get the cursor position as (line, column).
    pub fn CursorPosition(&self) -> (usize, usize) {
        let line = self.text[..self.cursor_index].matches('\n').count();
        let col = self.text[..self.cursor_index]
            .rfind('\n')
            .map(|i| self.cursor_index - i - 1)
            .unwrap_or(self.cursor_index);
        (line, col)
    }
}
