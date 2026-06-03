use serde::{Deserialize, Serialize};

use super::blueprint::{BlueprintContext, BlueprintExecutor, BlueprintResult, BlueprintState};
use super::graph::NodeGraph;
use super::renderer::NodeGraphState;
use super::types::NodeId;

/// A component that attaches a visual script (blueprint) to an ECS entity.
///
/// This allows blueprints to be mounted on entities and executed at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintComponent {
    /// The blueprint graph data.
    pub graph: NodeGraph,
    /// Display name for this blueprint.
    pub name: String,
    /// Whether this blueprint is enabled for execution.
    pub enabled: bool,
    /// The entry point event node ID (e.g., BeginPlay or Tick).
    pub entry_node: Option<NodeId>,
    /// Whether this blueprint has been initialized (BeginPlay fired).
    #[serde(skip)]
    pub initialized: bool,
    /// Current execution state.
    #[serde(skip)]
    pub state: BlueprintState,
}

impl BlueprintComponent {
    /// Create a new blueprint component with an empty graph.
    pub fn new(name: &str) -> Self {
        Self {
            graph: NodeGraph::new(),
            name: name.to_string(),
            enabled: true,
            entry_node: None,
            initialized: false,
            state: BlueprintState::Idle,
        }
    }

    /// Create a blueprint component from an existing graph.
    pub fn from_graph(name: &str, graph: NodeGraph) -> Self {
        Self {
            graph,
            name: name.to_string(),
            enabled: true,
            entry_node: None,
            initialized: false,
            state: BlueprintState::Idle,
        }
    }

    /// Create a blueprint component from a NodeGraphState.
    pub fn from_state(name: &str, state: &NodeGraphState) -> Self {
        Self {
            graph: state.graph.clone(),
            name: name.to_string(),
            enabled: true,
            entry_node: None,
            initialized: false,
            state: BlueprintState::Idle,
        }
    }

    /// Serialize the blueprint to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a blueprint from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Execute the blueprint's BeginPlay event.
    pub fn begin_play(&mut self) -> BlueprintResult {
        if !self.enabled {
            return BlueprintResult {
                state: BlueprintState::Completed,
                trace: Vec::new(),
                print_buffer: Vec::new(),
                errors: Vec::new(),
            };
        }

        let mut ctx = BlueprintContext::new();
        let result = BlueprintExecutor::execute(&self.graph, &mut ctx);
        self.state = result.state.clone();
        self.initialized = true;
        result
    }

    /// Execute the blueprint's Tick event.
    pub fn tick(&mut self, dt: f32) -> BlueprintResult {
        if !self.enabled || !self.initialized {
            return BlueprintResult {
                state: BlueprintState::Completed,
                trace: Vec::new(),
                print_buffer: Vec::new(),
                errors: Vec::new(),
            };
        }

        // Handle waiting state (delays)
        if let BlueprintState::Waiting { remaining } = &mut self.state {
            *remaining -= dt;
            if *remaining <= 0.0 {
                self.state = BlueprintState::Running;
            } else {
                return BlueprintResult {
                    state: self.state.clone(),
                    trace: Vec::new(),
                    print_buffer: Vec::new(),
                    errors: Vec::new(),
                };
            }
        }

        let mut ctx = BlueprintContext::new();
        let result = BlueprintExecutor::execute(&self.graph, &mut ctx);
        self.state = result.state.clone();
        result
    }

    /// Reset the blueprint state.
    pub fn reset(&mut self) {
        self.initialized = false;
        self.state = BlueprintState::Idle;
    }
}

impl Default for BlueprintComponent {
    fn default() -> Self {
        Self::new("Untitled Blueprint")
    }
}

/// Manager for handling multiple blueprint components in the editor.
#[derive(Debug, Clone, Default)]
pub struct BlueprintManager {
    /// All registered blueprints.
    pub blueprints: Vec<BlueprintComponent>,
    /// Index of the currently selected blueprint.
    pub selected: Option<usize>,
}

impl BlueprintManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new blueprint.
    pub fn add(&mut self, blueprint: BlueprintComponent) -> usize {
        let index = self.blueprints.len();
        self.blueprints.push(blueprint);
        index
    }

    /// Remove a blueprint by index.
    pub fn remove(&mut self, index: usize) -> Option<BlueprintComponent> {
        if index < self.blueprints.len() {
            if self.selected == Some(index) {
                self.selected = None;
            } else if let Some(sel) = self.selected
                && sel > index
            {
                self.selected = Some(sel - 1);
            }
            Some(self.blueprints.remove(index))
        } else {
            None
        }
    }

    /// Get a reference to the selected blueprint.
    pub fn selected_blueprint(&self) -> Option<&BlueprintComponent> {
        self.selected.and_then(|i| self.blueprints.get(i))
    }

    /// Get a mutable reference to the selected blueprint.
    pub fn selected_blueprint_mut(&mut self) -> Option<&mut BlueprintComponent> {
        self.selected.and_then(|i| self.blueprints.get_mut(i))
    }

    /// Tick all enabled blueprints.
    pub fn tick_all(&mut self, dt: f32) -> Vec<(usize, BlueprintResult)> {
        let mut results = Vec::new();
        for (i, bp) in self.blueprints.iter_mut().enumerate() {
            if bp.enabled {
                let result = bp.tick(dt);
                results.push((i, result));
            }
        }
        results
    }

    /// Initialize all blueprints (fire BeginPlay).
    pub fn init_all(&mut self) -> Vec<(usize, BlueprintResult)> {
        let mut results = Vec::new();
        for (i, bp) in self.blueprints.iter_mut().enumerate() {
            if bp.enabled && !bp.initialized {
                let result = bp.begin_play();
                results.push((i, result));
            }
        }
        results
    }

    /// Serialize all blueprints to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.blueprints)
    }

    /// Deserialize blueprints from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let blueprints: Vec<BlueprintComponent> = serde_json::from_str(json)?;
        Ok(Self {
            blueprints,
            selected: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_graph::graph::NodeType;
    use crate::node_graph::nodes::create_node;
    use egui::Pos2;

    #[test]
    fn test_blueprint_component_new() {
        let bp = BlueprintComponent::new("Test Blueprint");
        assert_eq!(bp.name, "Test Blueprint");
        assert!(bp.enabled);
        assert!(!bp.initialized);
        assert_eq!(bp.state, BlueprintState::Idle);
    }

    #[test]
    fn test_blueprint_component_default() {
        let bp = BlueprintComponent::default();
        assert_eq!(bp.name, "Untitled Blueprint");
    }

    #[test]
    fn test_blueprint_component_from_graph() {
        let graph = NodeGraph::new();
        let bp = BlueprintComponent::from_graph("MyBP", graph);
        assert_eq!(bp.name, "MyBP");
    }

    #[test]
    fn test_blueprint_component_json_roundtrip() {
        let bp = BlueprintComponent::new("Test");
        let json = bp.to_json().unwrap();
        let restored = BlueprintComponent::from_json(&json).unwrap();
        assert_eq!(restored.name, "Test");
    }

    #[test]
    fn test_blueprint_component_begin_play() {
        let mut graph = NodeGraph::new();
        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        graph.add_node(event);

        let mut bp = BlueprintComponent::from_graph("Test", graph);
        let result = bp.begin_play();
        assert!(bp.initialized);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_blueprint_component_disabled() {
        let mut bp = BlueprintComponent::new("Test");
        bp.enabled = false;
        let result = bp.begin_play();
        assert!(!bp.initialized);
        assert_eq!(result.state, BlueprintState::Completed);
    }

    #[test]
    fn test_blueprint_component_tick() {
        let mut graph = NodeGraph::new();
        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        graph.add_node(event);

        let mut bp = BlueprintComponent::from_graph("Test", graph);
        bp.begin_play();

        let result = bp.tick(0.016);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_blueprint_manager_add_remove() {
        let mut manager = BlueprintManager::new();
        let idx = manager.add(BlueprintComponent::new("BP1"));
        assert_eq!(idx, 0);
        manager.add(BlueprintComponent::new("BP2"));
        assert_eq!(manager.blueprints.len(), 2);

        let removed = manager.remove(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "BP1");
        assert_eq!(manager.blueprints.len(), 1);
    }

    #[test]
    fn test_blueprint_manager_select() {
        let mut manager = BlueprintManager::new();
        manager.add(BlueprintComponent::new("BP1"));
        manager.add(BlueprintComponent::new("BP2"));

        manager.selected = Some(0);
        assert!(manager.selected_blueprint().is_some());
        assert_eq!(manager.selected_blueprint().unwrap().name, "BP1");

        manager.selected = Some(1);
        assert_eq!(manager.selected_blueprint().unwrap().name, "BP2");

        manager.selected = None;
        assert!(manager.selected_blueprint().is_none());
    }

    #[test]
    fn test_blueprint_manager_tick_all() {
        let mut manager = BlueprintManager::new();

        let mut graph = NodeGraph::new();
        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        graph.add_node(event);

        let mut bp = BlueprintComponent::from_graph("BP1", graph);
        bp.begin_play();
        manager.add(bp);

        let mut bp2 = BlueprintComponent::new("BP2");
        bp2.enabled = false;
        manager.add(bp2);

        let results = manager.tick_all(0.016);
        assert_eq!(results.len(), 1); // Only enabled blueprint ticked
    }

    #[test]
    fn test_blueprint_manager_init_all() {
        let mut manager = BlueprintManager::new();

        let mut graph = NodeGraph::new();
        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        graph.add_node(event);

        manager.add(BlueprintComponent::from_graph("BP1", graph));
        manager.add(BlueprintComponent::new("BP2"));

        let results = manager.init_all();
        assert_eq!(results.len(), 2);
        assert!(manager.blueprints[0].initialized);
        assert!(manager.blueprints[1].initialized);
    }

    #[test]
    fn test_blueprint_manager_json_roundtrip() {
        let mut manager = BlueprintManager::new();
        manager.add(BlueprintComponent::new("BP1"));
        manager.add(BlueprintComponent::new("BP2"));

        let json = manager.to_json().unwrap();
        let restored = BlueprintManager::from_json(&json).unwrap();
        assert_eq!(restored.blueprints.len(), 2);
        assert_eq!(restored.blueprints[0].name, "BP1");
    }
}
