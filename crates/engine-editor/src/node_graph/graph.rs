use egui::Pos2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{Connection, NodeId, NodeValue, PinDirection, PinId, PinType};

/// Serializable 2D position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GraphPos {
    pub x: f32,
    pub y: f32,
}

impl GraphPos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn to_pos2(self) -> Pos2 {
        Pos2::new(self.x, self.y)
    }

    pub fn from_pos2(pos: Pos2) -> Self {
        Self { x: pos.x, y: pos.y }
    }
}

impl Default for GraphPos {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Definition of a single pin on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: PinId,
    pub name: String,
    pub pin_type: PinType,
    pub direction: PinDirection,
    pub default_value: NodeValue,
}

impl Pin {
    pub fn new(
        node_id: NodeId,
        index: usize,
        name: &str,
        pin_type: PinType,
        direction: PinDirection,
    ) -> Self {
        let default_value = NodeValue::default_for_type(pin_type);
        Self {
            id: PinId::new(node_id, index),
            name: name.to_string(),
            pin_type,
            direction,
            default_value,
        }
    }

    pub fn with_default(mut self, value: NodeValue) -> Self {
        self.default_value = value;
        self
    }
}

/// Category of a node for organization in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeCategory {
    Input,
    Math,
    Texture,
    Color,
    Vector,
    Output,
    Custom(String),
}

impl NodeCategory {
    pub fn display_name(&self) -> &str {
        match self {
            NodeCategory::Input => "Input",
            NodeCategory::Math => "Math",
            NodeCategory::Texture => "Texture",
            NodeCategory::Color => "Color",
            NodeCategory::Vector => "Vector",
            NodeCategory::Output => "Output",
            NodeCategory::Custom(name) => name,
        }
    }
}

/// Type identifier for a node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    // Input nodes
    ConstantFloat,
    ConstantVec2,
    ConstantVec3,
    ConstantVec4,
    ConstantColor,
    ConstantBool,
    ConstantInt,
    UVCoordinate,
    Time,

    // Math nodes
    Add,
    Subtract,
    Multiply,
    Divide,
    Sin,
    Cos,
    Abs,
    Clamp,
    Lerp,
    Power,
    Saturate,
    Negate,

    // Texture nodes
    TextureSample,

    // Color nodes
    CombineRgb,
    SplitRgb,

    // Vector nodes
    DotProduct,
    Normalize,
    CrossProduct,

    // Output
    MaterialOutput,

    Custom(String),
}

impl NodeType {
    pub fn display_name(&self) -> &str {
        match self {
            NodeType::ConstantFloat => "Float Constant",
            NodeType::ConstantVec2 => "Vec2 Constant",
            NodeType::ConstantVec3 => "Vec3 Constant",
            NodeType::ConstantVec4 => "Vec4 Constant",
            NodeType::ConstantColor => "Color Constant",
            NodeType::ConstantBool => "Bool Constant",
            NodeType::ConstantInt => "Int Constant",
            NodeType::UVCoordinate => "UV Coordinate",
            NodeType::Time => "Time",
            NodeType::Add => "Add",
            NodeType::Subtract => "Subtract",
            NodeType::Multiply => "Multiply",
            NodeType::Divide => "Divide",
            NodeType::Sin => "Sine",
            NodeType::Cos => "Cosine",
            NodeType::Abs => "Absolute",
            NodeType::Clamp => "Clamp",
            NodeType::Lerp => "Lerp",
            NodeType::Power => "Power",
            NodeType::Saturate => "Saturate",
            NodeType::Negate => "Negate",
            NodeType::TextureSample => "Texture Sample",
            NodeType::CombineRgb => "Combine RGB",
            NodeType::SplitRgb => "Split RGB",
            NodeType::DotProduct => "Dot Product",
            NodeType::Normalize => "Normalize",
            NodeType::CrossProduct => "Cross Product",
            NodeType::MaterialOutput => "Material Output",
            NodeType::Custom(name) => name,
        }
    }

    pub fn category(&self) -> NodeCategory {
        match self {
            NodeType::ConstantFloat
            | NodeType::ConstantVec2
            | NodeType::ConstantVec3
            | NodeType::ConstantVec4
            | NodeType::ConstantColor
            | NodeType::ConstantBool
            | NodeType::ConstantInt
            | NodeType::UVCoordinate
            | NodeType::Time => NodeCategory::Input,

            NodeType::Add
            | NodeType::Subtract
            | NodeType::Multiply
            | NodeType::Divide
            | NodeType::Sin
            | NodeType::Cos
            | NodeType::Abs
            | NodeType::Clamp
            | NodeType::Lerp
            | NodeType::Power
            | NodeType::Saturate
            | NodeType::Negate => NodeCategory::Math,

            NodeType::TextureSample => NodeCategory::Texture,

            NodeType::CombineRgb | NodeType::SplitRgb => NodeCategory::Color,

            NodeType::DotProduct | NodeType::Normalize | NodeType::CrossProduct => {
                NodeCategory::Vector
            }

            NodeType::MaterialOutput => NodeCategory::Output,
            NodeType::Custom(_) => NodeCategory::Custom("Custom".to_string()),
        }
    }
}

/// A node in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub node_type: NodeType,
    pub name: String,
    #[serde(default)]
    pub position: GraphPos,
    pub inputs: Vec<Pin>,
    pub outputs: Vec<Pin>,
    pub values: HashMap<usize, NodeValue>,
}

impl Node {
    pub fn new(id: NodeId, node_type: NodeType, name: &str, position: Pos2) -> Self {
        Self {
            id,
            node_type,
            name: name.to_string(),
            position: GraphPos::from_pos2(position),
            inputs: Vec::new(),
            outputs: Vec::new(),
            values: HashMap::new(),
        }
    }

    pub fn add_input(&mut self, name: &str, pin_type: PinType) -> usize {
        let index = self.inputs.len();
        self.inputs
            .push(Pin::new(self.id, index, name, pin_type, PinDirection::Input));
        index
    }

    pub fn add_output(&mut self, name: &str, pin_type: PinType) -> usize {
        let index = self.outputs.len();
        self.outputs.push(Pin::new(
            self.id,
            index + self.inputs.len(),
            name,
            pin_type,
            PinDirection::Output,
        ));
        index
    }

    pub fn add_input_with_default(
        &mut self,
        name: &str,
        pin_type: PinType,
        default: NodeValue,
    ) -> usize {
        let index = self.inputs.len();
        self.inputs.push(
            Pin::new(self.id, index, name, pin_type, PinDirection::Input).with_default(default),
        );
        index
    }

    pub fn width(&self) -> f32 {
        let name_len = self.name.len() as f32;
        let max_pin_len = self
            .inputs
            .iter()
            .chain(self.outputs.iter())
            .map(|p| p.name.len() as f32)
            .fold(0.0_f32, f32::max);
        (name_len * 8.0 + 40.0).max(max_pin_len * 7.0 + 80.0).max(140.0)
    }

    pub fn height(&self) -> f32 {
        let pin_rows = self.inputs.len().max(self.outputs.len());
        32.0 + pin_rows as f32 * 24.0 + 8.0
    }
}

/// The complete node graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGraph {
    pub nodes: HashMap<NodeId, Node>,
    pub connections: Vec<Connection>,
    next_id: NodeId,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a node to the graph and return its ID.
    pub fn add_node(&mut self, mut node: Node) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        node.id = id;

        // Update pin IDs with the actual node ID
        for (i, pin) in node.inputs.iter_mut().enumerate() {
            pin.id = PinId::new(id, i);
        }
        for (i, pin) in node.outputs.iter_mut().enumerate() {
            pin.id = PinId::new(id, node.inputs.len() + i);
        }

        self.nodes.insert(id, node);
        id
    }

    /// Remove a node and all its connections.
    pub fn remove_node(&mut self, node_id: NodeId) -> bool {
        if self.nodes.remove(&node_id).is_some() {
            self.connections
                .retain(|c| c.output_pin.node_id != node_id && c.input_pin.node_id != node_id);
            true
        } else {
            false
        }
    }

    /// Connect an output pin to an input pin.
    pub fn connect(&mut self, output: PinId, input: PinId) -> Result<(), ConnectError> {
        // Validate that both nodes exist
        let output_node = self
            .nodes
            .get(&output.node_id)
            .ok_or(ConnectError::NodeNotFound(output.node_id))?;
        let input_node = self
            .nodes
            .get(&input.node_id)
            .ok_or(ConnectError::NodeNotFound(input.node_id))?;

        // Validate pin indices
        let output_pin = output_node
            .outputs
            .iter()
            .find(|p| p.id.index == output.index)
            .ok_or(ConnectError::PinNotFound(output))?;
        let input_pin = input_node
            .inputs
            .iter()
            .find(|p| p.id.index == input.index)
            .ok_or(ConnectError::PinNotFound(input))?;

        // Type compatibility check
        if !output_pin.pin_type.is_compatible_with(&input_pin.pin_type) {
            return Err(ConnectError::TypeMismatch {
                from: output_pin.pin_type,
                to: input_pin.pin_type,
            });
        }

        // Prevent self-connections
        if output.node_id == input.node_id {
            return Err(ConnectError::SelfConnection);
        }

        // Remove any existing connection to this input pin
        self.connections
            .retain(|c| c.input_pin != input);

        // Check for cycles before adding
        if self.would_create_cycle(output.node_id, input.node_id) {
            return Err(ConnectError::CycleDetected);
        }

        self.connections.push(Connection {
            output_pin: output,
            input_pin: input,
        });

        Ok(())
    }

    /// Disconnect a specific connection.
    pub fn disconnect(&mut self, output: PinId, input: PinId) -> bool {
        let len_before = self.connections.len();
        self.connections
            .retain(|c| !(c.output_pin == output && c.input_pin == input));
        self.connections.len() < len_before
    }

    /// Remove all connections to/from a specific pin.
    pub fn disconnect_all(&mut self, pin: PinId) {
        self.connections
            .retain(|c| c.output_pin != pin && c.input_pin != pin);
    }

    /// Check if connecting two nodes would create a cycle.
    fn would_create_cycle(&self, from: NodeId, to: NodeId) -> bool {
        // BFS from 'to' to see if we can reach 'from'
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(to);
        visited.insert(to);

        while let Some(current) = queue.pop_front() {
            if current == from {
                return true;
            }
            for conn in &self.connections {
                if conn.output_pin.node_id == current {
                    let next = conn.input_pin.node_id;
                    if !visited.contains(&next) {
                        visited.insert(next);
                        queue.push_back(next);
                    }
                }
            }
        }
        false
    }

    /// Get all input connections for a node.
    pub fn get_input_connections(&self, node_id: NodeId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.input_pin.node_id == node_id)
            .collect()
    }

    /// Get all output connections for a node.
    pub fn get_output_connections(&self, node_id: NodeId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.output_pin.node_id == node_id)
            .collect()
    }

    /// Find connection to a specific input pin.
    pub fn get_connection_to_input(&self, input: PinId) -> Option<&Connection> {
        self.connections.iter().find(|c| c.input_pin == input)
    }

    /// Move a node to a new position.
    pub fn move_node(&mut self, node_id: NodeId, position: Pos2) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.position = GraphPos::from_pos2(position);
        }
    }

    /// Get all nodes in a category.
    pub fn get_nodes_by_type(&self, node_type: &NodeType) -> Vec<&Node> {
        self.nodes
            .values()
            .filter(|n| &n.node_type == node_type)
            .collect()
    }

    /// Serialize the graph to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a graph from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for NodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when connecting pins.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectError {
    NodeNotFound(NodeId),
    PinNotFound(PinId),
    TypeMismatch { from: PinType, to: PinType },
    SelfConnection,
    CycleDetected,
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectError::NodeNotFound(id) => write!(f, "Node {} not found", id),
            ConnectError::PinNotFound(id) => {
                write!(f, "Pin ({}, {}) not found", id.node_id, id.index)
            }
            ConnectError::TypeMismatch { from, to } => {
                write!(
                    f,
                    "Cannot connect {} to {}",
                    from.display_name(),
                    to.display_name()
                )
            }
            ConnectError::SelfConnection => write!(f, "Cannot connect a node to itself"),
            ConnectError::CycleDetected => write!(f, "Connection would create a cycle"),
        }
    }
}

impl std::error::Error for ConnectError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_graph() -> (NodeGraph, NodeId, NodeId) {
        let mut graph = NodeGraph::new();

        let mut node1 = Node::new(0, NodeType::ConstantFloat, "Float", Pos2::new(0.0, 0.0));
        node1.add_output("Value", PinType::Float);

        let mut node2 = Node::new(0, NodeType::Add, "Add", Pos2::new(200.0, 0.0));
        node2.add_input("A", PinType::Float);
        node2.add_input("B", PinType::Float);
        node2.add_output("Result", PinType::Float);

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        (graph, id1, id2)
    }

    #[test]
    fn test_add_node() {
        let mut graph = NodeGraph::new();
        let node = Node::new(0, NodeType::ConstantFloat, "Float", Pos2::new(0.0, 0.0));
        let id = graph.add_node(node);
        assert!(graph.nodes.contains_key(&id));
    }

    #[test]
    fn test_remove_node() {
        let (mut graph, id1, _id2) = make_test_graph();
        assert!(graph.remove_node(id1));
        assert!(!graph.nodes.contains_key(&id1));
    }

    #[test]
    fn test_connect_success() {
        let (mut graph, id1, id2) = make_test_graph();
        let output = PinId::new(id1, 0);
        let input = PinId::new(id2, 0);
        assert!(graph.connect(output, input).is_ok());
        assert_eq!(graph.connections.len(), 1);
    }

    #[test]
    fn test_connect_type_mismatch() {
        let mut graph = NodeGraph::new();

        let mut node1 = Node::new(0, NodeType::ConstantFloat, "Float", Pos2::new(0.0, 0.0));
        node1.add_output("Value", PinType::Float);

        let mut node2 = Node::new(0, NodeType::ConstantVec3, "Vec3", Pos2::new(200.0, 0.0));
        node2.add_input("Value", PinType::Vec3);

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        let output = PinId::new(id1, 0);
        let input = PinId::new(id2, 0);
        assert_eq!(
            graph.connect(output, input),
            Err(ConnectError::TypeMismatch {
                from: PinType::Float,
                to: PinType::Vec3,
            })
        );
    }

    #[test]
    fn test_connect_self_connection() {
        let mut graph = NodeGraph::new();
        let mut node = Node::new(0, NodeType::Add, "Add", Pos2::new(0.0, 0.0));
        node.add_input("A", PinType::Float);
        node.add_output("Result", PinType::Float);
        let id = graph.add_node(node);

        let output = PinId::new(id, 1); // output index is 1 (after input)
        let input = PinId::new(id, 0);
        assert_eq!(graph.connect(output, input), Err(ConnectError::SelfConnection));
    }

    #[test]
    fn test_connect_cycle_detection() {
        let mut graph = NodeGraph::new();

        let mut n1 = Node::new(0, NodeType::Add, "Add1", Pos2::new(0.0, 0.0));
        n1.add_input("A", PinType::Float);
        n1.add_output("Result", PinType::Float);

        let mut n2 = Node::new(0, NodeType::Add, "Add2", Pos2::new(200.0, 0.0));
        n2.add_input("A", PinType::Float);
        n2.add_output("Result", PinType::Float);

        let id1 = graph.add_node(n1);
        let id2 = graph.add_node(n2);

        // Connect id1 -> id2
        let out1 = PinId::new(id1, 1); // output
        let in2 = PinId::new(id2, 0);
        assert!(graph.connect(out1, in2).is_ok());

        // Try to connect id2 -> id1 (would create cycle)
        let out2 = PinId::new(id2, 1);
        let in1 = PinId::new(id1, 0);
        assert_eq!(graph.connect(out2, in1), Err(ConnectError::CycleDetected));
    }

    #[test]
    fn test_disconnect() {
        let (mut graph, id1, id2) = make_test_graph();
        let output = PinId::new(id1, 0);
        let input = PinId::new(id2, 0);
        graph.connect(output, input).unwrap();
        assert!(graph.disconnect(output, input));
        assert_eq!(graph.connections.len(), 0);
    }

    #[test]
    fn test_reconnect_replaces_existing() {
        let (mut graph, id1, id2) = make_test_graph();

        // Add another float output node
        let mut node3 = Node::new(0, NodeType::ConstantFloat, "Float2", Pos2::new(0.0, 100.0));
        node3.add_output("Value", PinType::Float);
        let id3 = graph.add_node(node3);

        let out1 = PinId::new(id1, 0);
        let out3 = PinId::new(id3, 0);
        let in2 = PinId::new(id2, 0);

        graph.connect(out1, in2).unwrap();
        assert_eq!(graph.connections.len(), 1);

        // Reconnect with different source
        graph.connect(out3, in2).unwrap();
        assert_eq!(graph.connections.len(), 1);
        assert_eq!(graph.connections[0].output_pin.node_id, id3);
    }

    #[test]
    fn test_get_input_connections() {
        let (mut graph, id1, id2) = make_test_graph();
        let output = PinId::new(id1, 0);
        let input = PinId::new(id2, 0);
        graph.connect(output, input).unwrap();

        let conns = graph.get_input_connections(id2);
        assert_eq!(conns.len(), 1);
    }

    #[test]
    fn test_node_dimensions() {
        let node = Node::new(1, NodeType::Add, "Add", Pos2::new(0.0, 0.0));
        assert!(node.width() >= 140.0);
        assert!(node.height() >= 32.0);
    }

    #[test]
    fn test_json_roundtrip() {
        let (mut graph, id1, id2) = make_test_graph();
        let output = PinId::new(id1, 0);
        let input = PinId::new(id2, 0);
        graph.connect(output, input).unwrap();

        let json = graph.to_json().unwrap();
        let restored = NodeGraph::from_json(&json).unwrap();
        assert_eq!(restored.nodes.len(), 2);
        assert_eq!(restored.connections.len(), 1);
    }

    #[test]
    fn test_move_node() {
        let mut graph = NodeGraph::new();
        let node = Node::new(0, NodeType::ConstantFloat, "Float", Pos2::new(0.0, 0.0));
        let id = graph.add_node(node);
        graph.move_node(id, Pos2::new(100.0, 200.0));
        assert_eq!(graph.nodes[&id].position, GraphPos::new(100.0, 200.0));
    }
}
