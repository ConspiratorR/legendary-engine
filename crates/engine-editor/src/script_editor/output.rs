use egui::Color32;

#[derive(Debug, Clone)]
pub struct ScriptOutput {
    pub entries: Vec<OutputEntry>,
    pub max_entries: usize,
    pub auto_scroll: bool,
    pub filter_level: OutputLevel,
    pub search: String,
}

#[derive(Debug, Clone)]
pub struct OutputEntry {
    pub text: String,
    pub level: OutputLevel,
    pub source: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl OutputLevel {
    pub fn label(&self) -> &'static str {
        match self {
            OutputLevel::Debug => "DEBUG",
            OutputLevel::Info => "INFO",
            OutputLevel::Warning => "WARN",
            OutputLevel::Error => "ERROR",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            OutputLevel::Debug => Color32::from_gray(100),
            OutputLevel::Info => Color32::from_rgb(152, 195, 121),
            OutputLevel::Warning => Color32::from_rgb(229, 192, 123),
            OutputLevel::Error => Color32::from_rgb(224, 108, 117),
        }
    }
}

impl Default for ScriptOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptOutput {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 5000,
            auto_scroll: true,
            filter_level: OutputLevel::Debug,
            search: String::new(),
        }
    }

    pub fn log(&mut self, text: &str, level: OutputLevel, source: &str) {
        let now = chrono_or_fallback();
        self.entries.push(OutputEntry {
            text: text.to_string(),
            level,
            source: source.to_string(),
            timestamp: now,
        });
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    pub fn info(&mut self, text: &str, source: &str) {
        self.log(text, OutputLevel::Info, source);
    }

    pub fn warn(&mut self, text: &str, source: &str) {
        self.log(text, OutputLevel::Warning, source);
    }

    pub fn error(&mut self, text: &str, source: &str) {
        self.log(text, OutputLevel::Error, source);
    }

    pub fn debug(&mut self, text: &str, source: &str) {
        self.log(text, OutputLevel::Debug, source);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn filtered_entries(&self) -> Vec<&OutputEntry> {
        self.entries
            .iter()
            .filter(|e| e.level >= self.filter_level)
            .filter(|e| {
                if self.search.is_empty() {
                    true
                } else {
                    let q = self.search.to_lowercase();
                    e.text.to_lowercase().contains(&q) || e.source.to_lowercase().contains(&q)
                }
            })
            .collect()
    }

    pub fn error_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.level == OutputLevel::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.level == OutputLevel::Warning)
            .count()
    }
}

fn chrono_or_fallback() -> String {
    // Simple timestamp without chrono dependency
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let hours = (secs / 3600) % 24;
            let minutes = (secs / 60) % 60;
            let seconds = secs % 60;
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        })
        .unwrap_or_else(|_| "??:??:??".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entries() {
        let mut output = ScriptOutput::new();
        output.info("hello", "test.lua");
        output.error("bad", "test.lua");
        assert_eq!(output.entries.len(), 2);
        assert_eq!(output.error_count(), 1);
    }

    #[test]
    fn test_filter_level() {
        let mut output = ScriptOutput::new();
        output.debug("d", "s");
        output.info("i", "s");
        output.error("e", "s");
        output.filter_level = OutputLevel::Info;
        let filtered = output.filtered_entries();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_max_entries() {
        let mut output = ScriptOutput::new();
        output.max_entries = 3;
        for i in 0..5 {
            output.info(&format!("msg {}", i), "s");
        }
        assert_eq!(output.entries.len(), 3);
    }
}
