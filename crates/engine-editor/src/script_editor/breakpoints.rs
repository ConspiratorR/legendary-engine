use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct BreakpointManager {
    /// Line numbers (1-indexed) with active breakpoints
    pub breakpoints: HashSet<usize>,
    /// Currently paused line (for visual indicator)
    pub paused_line: Option<usize>,
    /// Whether execution is currently paused
    pub is_paused: bool,
    /// Single-step mode active
    pub step_mode: StepMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    None,
    StepOver,
    StepInto,
    StepOut,
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self {
            breakpoints: HashSet::new(),
            paused_line: None,
            is_paused: false,
            step_mode: StepMode::None,
        }
    }
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle(&mut self, line: usize) {
        if self.breakpoints.contains(&line) {
            self.breakpoints.remove(&line);
        } else {
            self.breakpoints.insert(line);
        }
    }

    pub fn has_breakpoint(&self, line: usize) -> bool {
        self.breakpoints.contains(&line)
    }

    pub fn clear_all(&mut self) {
        self.breakpoints.clear();
        self.paused_line = None;
        self.is_paused = false;
        self.step_mode = StepMode::None;
    }

    pub fn pause_at(&mut self, line: usize) {
        self.paused_line = Some(line);
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.paused_line = None;
        self.is_paused = false;
        self.step_mode = StepMode::None;
    }

    pub fn should_break(&self, line: usize) -> bool {
        self.breakpoints.contains(&line)
    }

    pub fn set_step_mode(&mut self, mode: StepMode) {
        self.step_mode = mode;
        if mode != StepMode::None {
            self.is_paused = false;
        }
    }

    pub fn breakpoint_count(&self) -> usize {
        self.breakpoints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_breakpoint() {
        let mut bp = BreakpointManager::new();
        bp.toggle(5);
        assert!(bp.has_breakpoint(5));
        bp.toggle(5);
        assert!(!bp.has_breakpoint(5));
    }

    #[test]
    fn test_clear_all() {
        let mut bp = BreakpointManager::new();
        bp.toggle(1);
        bp.toggle(5);
        bp.pause_at(5);
        bp.clear_all();
        assert!(bp.breakpoints.is_empty());
        assert!(!bp.is_paused);
    }

    #[test]
    fn test_should_break() {
        let mut bp = BreakpointManager::new();
        bp.toggle(10);
        assert!(bp.should_break(10));
        assert!(!bp.should_break(11));
    }
}
