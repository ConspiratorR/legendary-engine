use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct VariableWatcher {
    pub variables: BTreeMap<String, WatchedVariable>,
    pub expanded: std::collections::HashSet<String>,
    pub filter: String,
}

#[derive(Debug, Clone)]
pub struct WatchedVariable {
    pub name: String,
    pub value: String,
    pub var_type: VarType,
    pub is_editable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    Nil,
    Bool,
    Number,
    String,
    Table,
    Function,
    Userdata,
    Other,
}

impl VarType {
    pub fn label(&self) -> &'static str {
        match self {
            VarType::Nil => "nil",
            VarType::Bool => "bool",
            VarType::Number => "number",
            VarType::String => "string",
            VarType::Table => "table",
            VarType::Function => "function",
            VarType::Userdata => "userdata",
            VarType::Other => "other",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            VarType::Nil => egui::Color32::from_gray(100),
            VarType::Bool => egui::Color32::from_rgb(204, 120, 220),
            VarType::Number => egui::Color32::from_rgb(209, 154, 102),
            VarType::String => egui::Color32::from_rgb(152, 195, 121),
            VarType::Table => egui::Color32::from_rgb(86, 182, 194),
            VarType::Function => egui::Color32::from_rgb(97, 175, 239),
            VarType::Userdata => egui::Color32::from_rgb(198, 120, 82),
            VarType::Other => egui::Color32::from_gray(152),
        }
    }
}

impl Default for VariableWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl VariableWatcher {
    pub fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
            expanded: std::collections::HashSet::new(),
            filter: String::new(),
        }
    }

    pub fn set_variable(&mut self, name: &str, value: &str, var_type: VarType) {
        self.variables.insert(
            name.to_string(),
            WatchedVariable {
                name: name.to_string(),
                value: value.to_string(),
                var_type,
                is_editable: matches!(var_type, VarType::Bool | VarType::Number | VarType::String),
            },
        );
    }

    pub fn remove_variable(&mut self, name: &str) {
        self.variables.remove(name);
    }

    pub fn clear(&mut self) {
        self.variables.clear();
        self.expanded.clear();
    }

    pub fn filtered_variables(&self) -> Vec<&WatchedVariable> {
        if self.filter.is_empty() {
            self.variables.values().collect()
        } else {
            let q = self.filter.to_lowercase();
            self.variables
                .values()
                .filter(|v| v.name.to_lowercase().contains(&q))
                .collect()
        }
    }

    pub fn toggle_expand(&mut self, name: &str) {
        let key = name.to_string();
        if self.expanded.contains(&key) {
            self.expanded.remove(&key);
        } else {
            self.expanded.insert(key);
        }
    }

    pub fn is_expanded(&self, name: &str) -> bool {
        self.expanded.contains(name)
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut w = VariableWatcher::new();
        w.set_variable("x", "42", VarType::Number);
        assert_eq!(w.variable_count(), 1);
        assert!(w.variables.contains_key("x"));
    }

    #[test]
    fn test_filter() {
        let mut w = VariableWatcher::new();
        w.set_variable("player_health", "100", VarType::Number);
        w.set_variable("enemy_count", "5", VarType::Number);
        w.filter = "player".to_string();
        assert_eq!(w.filtered_variables().len(), 1);
    }
}
