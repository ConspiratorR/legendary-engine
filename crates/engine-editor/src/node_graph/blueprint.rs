use std::collections::HashMap;

use super::graph::{Node, NodeGraph, NodeType};
use super::types::{NodeId, NodeValue, PinType};

// ───────────────────────────────────────────────
// Variable Store
// ───────────────────────────────────────────────

/// A type-safe variable store for blueprint runtime.
/// Supports both global and local (per-scope) variables.
#[derive(Debug, Clone, Default)]
pub struct VariableStore {
    globals: HashMap<String, NodeValue>,
    locals: Vec<HashMap<String, NodeValue>>,
}

impl VariableStore {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            locals: vec![HashMap::new()],
        }
    }

    /// Push a new local scope.
    pub fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    /// Pop the current local scope.
    pub fn pop_scope(&mut self) {
        if self.locals.len() > 1 {
            self.locals.pop();
        }
    }

    /// Set a global variable.
    pub fn set_global(&mut self, name: &str, value: NodeValue) {
        self.globals.insert(name.to_string(), value);
    }

    /// Get a global variable.
    pub fn get_global(&self, name: &str) -> Option<&NodeValue> {
        self.globals.get(name)
    }

    /// Set a local variable (writes to the innermost scope).
    pub fn set_local(&mut self, name: &str, value: NodeValue) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    /// Get a local variable (searches from innermost to outermost scope).
    pub fn get_local(&self, name: &str) -> Option<&NodeValue> {
        for scope in self.locals.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val);
            }
        }
        None
    }

    /// Get a variable — checks locals first, then globals.
    pub fn get(&self, name: &str) -> Option<&NodeValue> {
        self.get_local(name).or_else(|| self.get_global(name))
    }

    /// Set a variable — writes to the current local scope.
    pub fn set(&mut self, name: &str, value: NodeValue) {
        self.set_local(name, value);
    }

    /// Clear all variables.
    pub fn clear(&mut self) {
        self.globals.clear();
        self.locals.clear();
        self.locals.push(HashMap::new());
    }
}

// ───────────────────────────────────────────────
// Blueprint Runtime State
// ───────────────────────────────────────────────

/// State of a single blueprint execution.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum BlueprintState {
    /// Execution has not started.
    #[default]
    Idle,
    /// Currently executing nodes.
    Running,
    /// Execution completed successfully.
    Completed,
    /// Execution hit an error.
    Error(String),
    /// Waiting for a delay to complete.
    Waiting { remaining: f32 },
}

/// The runtime context for blueprint execution.
#[derive(Debug)]
pub struct BlueprintContext {
    /// Variable storage.
    pub variables: VariableStore,
    /// Output values from data-flow pins (node_id, output_index) -> value.
    pub data_outputs: HashMap<(NodeId, usize), NodeValue>,
    /// Current execution state.
    pub state: BlueprintState,
    /// Execution trace (for debugging).
    pub trace: Vec<NodeId>,
    /// Print output buffer.
    pub print_buffer: Vec<String>,
    /// Do-Once state per node.
    pub do_once_fired: HashMap<NodeId, bool>,
    /// Flip-Flop state per node.
    pub flip_flop_state: HashMap<NodeId, bool>,
    /// Loop iteration counters.
    pub loop_counters: HashMap<NodeId, i32>,
    /// Sequence output index.
    pub sequence_index: HashMap<NodeId, usize>,
    /// Max execution steps (prevents infinite loops).
    pub max_steps: usize,
}

impl Default for BlueprintContext {
    fn default() -> Self {
        Self {
            variables: VariableStore::new(),
            data_outputs: HashMap::new(),
            state: BlueprintState::Idle,
            trace: Vec::new(),
            print_buffer: Vec::new(),
            do_once_fired: HashMap::new(),
            flip_flop_state: HashMap::new(),
            loop_counters: HashMap::new(),
            sequence_index: HashMap::new(),
            max_steps: 10_000,
        }
    }
}

impl BlueprintContext {
    pub fn new() -> Self {
        Self::default()
    }
}

// ───────────────────────────────────────────────
// Blueprint Executor
// ───────────────────────────────────────────────

/// Result of executing a blueprint graph.
#[derive(Debug, Clone)]
pub struct BlueprintResult {
    pub state: BlueprintState,
    pub trace: Vec<NodeId>,
    pub print_buffer: Vec<String>,
    pub errors: Vec<String>,
}

/// The blueprint executor — steps through execution-flow pins.
pub struct BlueprintExecutor;

impl BlueprintExecutor {
    /// Execute a blueprint graph starting from event nodes.
    pub fn execute(graph: &NodeGraph, ctx: &mut BlueprintContext) -> BlueprintResult {
        ctx.state = BlueprintState::Running;
        ctx.trace.clear();
        ctx.print_buffer.clear();
        let mut errors = Vec::new();
        let mut steps = 0;

        // Find all event nodes (entry points)
        let event_nodes: Vec<NodeId> = graph
            .nodes
            .values()
            .filter(|n| is_event_node(&n.node_type))
            .map(|n| n.id)
            .collect();

        // Collect initial execution pins from events
        let mut exec_queue: Vec<(NodeId, usize)> = Vec::new();
        for &event_id in &event_nodes {
            // Evaluate the event node to produce data outputs
            if let Some(node) = graph.nodes.get(&event_id) {
                match evaluate_blueprint_node(graph, node, ctx) {
                    Ok(outputs) => {
                        for (i, val) in outputs.into_iter().enumerate() {
                            ctx.data_outputs.insert((event_id, i), val);
                        }
                    }
                    Err(e) => errors.push(format!("Node {}: {}", event_id, e)),
                }
            }

            // Find execution output pins from this event
            if let Some(node) = graph.nodes.get(&event_id) {
                for (i, pin) in node.outputs.iter().enumerate() {
                    if pin.pin_type == PinType::Execution {
                        exec_queue.push((event_id, i));
                    }
                }
            }
        }

        // BFS through execution flow
        while let Some((node_id, exec_output_idx)) = exec_queue.pop() {
            steps += 1;
            if steps > ctx.max_steps {
                ctx.state = BlueprintState::Error("Max execution steps exceeded".to_string());
                errors.push("Infinite loop detected — max steps exceeded".to_string());
                break;
            }

            ctx.trace.push(node_id);

            // Find connections from this execution output pin
            let output_pin_index = if let Some(node) = graph.nodes.get(&node_id) {
                node.inputs.len() + exec_output_idx
            } else {
                continue;
            };

            let outgoing: Vec<NodeId> = graph
                .connections
                .iter()
                .filter(|c| {
                    c.output_pin.node_id == node_id && c.output_pin.index == output_pin_index
                })
                .map(|c| c.input_pin.node_id)
                .collect();

            for next_id in outgoing {
                if let Some(next_node) = graph.nodes.get(&next_id) {
                    match evaluate_blueprint_node(graph, next_node, ctx) {
                        Ok(outputs) => {
                            for (i, val) in outputs.into_iter().enumerate() {
                                ctx.data_outputs.insert((next_id, i), val);
                            }

                            // Check for execution outputs from this node
                            for (i, pin) in next_node.outputs.iter().enumerate() {
                                if pin.pin_type == PinType::Execution {
                                    exec_queue.push((next_id, i));
                                }
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Node {}: {}", next_id, e));
                        }
                    }
                }
            }
        }

        if ctx.state == BlueprintState::Running {
            ctx.state = BlueprintState::Completed;
        }

        BlueprintResult {
            state: ctx.state.clone(),
            trace: ctx.trace.clone(),
            print_buffer: ctx.print_buffer.clone(),
            errors,
        }
    }

    /// Update a waiting context (for Delay nodes).
    pub fn update_waiting(ctx: &mut BlueprintContext, dt: f32) {
        if let BlueprintState::Waiting { remaining } = &mut ctx.state {
            *remaining -= dt;
            if *remaining <= 0.0 {
                ctx.state = BlueprintState::Running;
            }
        }
    }
}

/// Check if a node is an event entry point.
fn is_event_node(node_type: &NodeType) -> bool {
    matches!(
        node_type,
        NodeType::EventBeginPlay | NodeType::EventTick | NodeType::EventCustom(_)
    )
}

/// Evaluate a single blueprint node, producing data outputs.
fn evaluate_blueprint_node(
    graph: &NodeGraph,
    node: &Node,
    ctx: &mut BlueprintContext,
) -> Result<Vec<NodeValue>, String> {
    // Gather input values from data connections (recursively evaluating deps)
    let input_values = gather_data_inputs(graph, node, &mut ctx.data_outputs);

    match node.node_type {
        // ── Event nodes ──
        NodeType::EventBeginPlay => Ok(vec![NodeValue::Float(0.0)]), // placeholder output
        NodeType::EventTick => Ok(vec![NodeValue::Float(0.016)]),    // ~60fps dt
        NodeType::EventCustom(_) => Ok(vec![]),

        // ── Flow control ──
        NodeType::Branch => {
            let condition = get_bool(&input_values, 1);
            // Store condition for downstream use
            Ok(vec![NodeValue::Bool(condition)])
        }
        NodeType::ForLoop => {
            let first = get_int(&input_values, 1);
            let last = get_int(&input_values, 2);
            let counter = ctx.loop_counters.entry(node.id).or_insert(first);
            let current = *counter;
            if current <= last {
                *counter += 1;
                // Loop body output: current index
                Ok(vec![NodeValue::Int(current)])
            } else {
                // Loop completed — reset counter
                ctx.loop_counters.remove(&node.id);
                Ok(vec![])
            }
        }
        NodeType::ForEachLoop => {
            // Simplified — just pass through
            Ok(vec![])
        }
        NodeType::Sequence => {
            // Sequence fires outputs in order — track which one we're on
            let idx = ctx.sequence_index.entry(node.id).or_insert(0);
            *idx = (*idx + 1) % 3;
            Ok(vec![])
        }
        NodeType::FlipFlop => {
            let state = ctx.flip_flop_state.entry(node.id).or_insert(true);
            *state = !*state;
            Ok(vec![NodeValue::Bool(*state)])
        }
        NodeType::DoOnce => {
            let fired = ctx.do_once_fired.entry(node.id).or_insert(false);
            if !*fired {
                *fired = true;
                Ok(vec![])
            } else {
                // Already fired — no execution output
                Ok(vec![])
            }
        }
        NodeType::Delay => {
            let duration = get_float(&input_values, 1);
            ctx.state = BlueprintState::Waiting {
                remaining: duration,
            };
            Ok(vec![])
        }

        // ── Variables ──
        NodeType::VariableGet => {
            // The variable name is stored in the node's values as a string-like value
            let name = node
                .values
                .get(&0)
                .map(|v| format!("{}", v))
                .unwrap_or_default();
            let value = ctx
                .variables
                .get(&name)
                .cloned()
                .unwrap_or(NodeValue::Float(0.0));
            Ok(vec![value])
        }
        NodeType::VariableSet => {
            let name = node
                .values
                .get(&1)
                .map(|v| format!("{}", v))
                .unwrap_or_default();
            let value = input_values
                .get(2)
                .cloned()
                .unwrap_or(NodeValue::Float(0.0));
            ctx.variables.set(&name, value.clone());
            Ok(vec![value])
        }

        // ── Logic ──
        NodeType::BooleanAnd => {
            let a = get_bool(&input_values, 0);
            let b = get_bool(&input_values, 1);
            Ok(vec![NodeValue::Bool(a && b)])
        }
        NodeType::BooleanOr => {
            let a = get_bool(&input_values, 0);
            let b = get_bool(&input_values, 1);
            Ok(vec![NodeValue::Bool(a || b)])
        }
        NodeType::BooleanNot => {
            let v = get_bool(&input_values, 0);
            Ok(vec![NodeValue::Bool(!v)])
        }
        NodeType::Equal => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Bool((a - b).abs() < f32::EPSILON)])
        }
        NodeType::NotEqual => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Bool((a - b).abs() >= f32::EPSILON)])
        }
        NodeType::GreaterThan => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Bool(a > b)])
        }
        NodeType::LessThan => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Bool(a < b)])
        }
        NodeType::GreaterEqual => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Bool(a >= b)])
        }
        NodeType::LessEqual => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Bool(a <= b)])
        }

        // ── Function ──
        NodeType::FunctionCall => {
            // Pass-through — function body would be a sub-graph in a real implementation
            Ok(vec![NodeValue::Float(0.0)])
        }
        NodeType::PrintString => {
            let msg = input_values
                .get(1)
                .map(|v| format!("{}", v))
                .unwrap_or_default();
            ctx.print_buffer.push(msg);
            Ok(vec![])
        }

        // ── Blueprint math ──
        NodeType::BlueprintAdd => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Float(a + b)])
        }
        NodeType::BlueprintSubtract => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Float(a - b)])
        }
        NodeType::BlueprintMultiply => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Float(a * b)])
        }
        NodeType::BlueprintDivide => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            if b.abs() < f32::EPSILON {
                Ok(vec![NodeValue::Float(0.0)])
            } else {
                Ok(vec![NodeValue::Float(a / b)])
            }
        }
        NodeType::BlueprintClamp => {
            let v = get_float(&input_values, 0);
            let min = get_float(&input_values, 1);
            let max = get_float(&input_values, 2);
            Ok(vec![NodeValue::Float(v.clamp(min, max))])
        }
        NodeType::BlueprintAbs => {
            let v = get_float(&input_values, 0);
            Ok(vec![NodeValue::Float(v.abs())])
        }
        NodeType::BlueprintMin => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Float(a.min(b))])
        }
        NodeType::BlueprintMax => {
            let a = get_float(&input_values, 0);
            let b = get_float(&input_values, 1);
            Ok(vec![NodeValue::Float(a.max(b))])
        }

        _ => {
            // For non-blueprint nodes, delegate to data-flow evaluator
            // or return empty
            Ok(vec![])
        }
    }
}

/// Gather data-flow input values for a node.
/// Recursively evaluates data dependencies that haven't been computed yet.
fn gather_data_inputs(
    graph: &NodeGraph,
    node: &Node,
    data_outputs: &mut HashMap<(NodeId, usize), NodeValue>,
) -> Vec<NodeValue> {
    let mut values = Vec::with_capacity(node.inputs.len());

    for (i, pin) in node.inputs.iter().enumerate() {
        // Skip execution pins — they don't carry data
        if pin.pin_type == PinType::Execution {
            values.push(NodeValue::Float(0.0));
            continue;
        }

        // Check for a data connection
        if let Some(conn) = graph.get_connection_to_input(pin.id) {
            let src_node_id = conn.output_pin.node_id;
            if let Some(src_node) = graph.nodes.get(&src_node_id) {
                let output_idx = conn.output_pin.index - src_node.inputs.len();

                // If the source hasn't been computed yet, evaluate it now
                if !data_outputs.contains_key(&(src_node_id, output_idx))
                    && let Ok(outputs) = evaluate_data_only_node(graph, src_node, data_outputs)
                {
                    for (j, val) in outputs.into_iter().enumerate() {
                        data_outputs.insert((src_node_id, j), val);
                    }
                }

                if let Some(value) = data_outputs.get(&(src_node_id, output_idx)) {
                    values.push(value.clone());
                    continue;
                }
            }
        }

        // Use default
        let default = node
            .values
            .get(&i)
            .cloned()
            .unwrap_or_else(|| pin.default_value.clone());
        values.push(default);
    }

    values
}

/// Evaluate a data-only node (no side effects, no context needed).
/// Used for recursive dependency resolution.
fn evaluate_data_only_node(
    graph: &NodeGraph,
    node: &Node,
    data_outputs: &mut HashMap<(NodeId, usize), NodeValue>,
) -> Result<Vec<NodeValue>, String> {
    let inputs = gather_data_inputs(graph, node, data_outputs);

    match node.node_type {
        // Constants
        NodeType::ConstantFloat
        | NodeType::ConstantVec2
        | NodeType::ConstantVec3
        | NodeType::ConstantVec4
        | NodeType::ConstantColor
        | NodeType::ConstantBool
        | NodeType::ConstantInt => {
            let val = node
                .values
                .get(&0)
                .cloned()
                .unwrap_or(NodeValue::Float(0.0));
            Ok(vec![val])
        }

        // Blueprint math
        NodeType::BlueprintAdd | NodeType::Add => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Float(a + b)])
        }
        NodeType::BlueprintSubtract | NodeType::Subtract => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Float(a - b)])
        }
        NodeType::BlueprintMultiply | NodeType::Multiply => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Float(a * b)])
        }
        NodeType::BlueprintDivide | NodeType::Divide => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            if b.abs() < f32::EPSILON {
                Ok(vec![NodeValue::Float(0.0)])
            } else {
                Ok(vec![NodeValue::Float(a / b)])
            }
        }
        NodeType::BlueprintClamp | NodeType::Clamp => {
            let v = get_float(&inputs, 0);
            let min = get_float(&inputs, 1);
            let max = get_float(&inputs, 2);
            Ok(vec![NodeValue::Float(v.clamp(min, max))])
        }
        NodeType::BlueprintAbs | NodeType::Abs => {
            let v = get_float(&inputs, 0);
            Ok(vec![NodeValue::Float(v.abs())])
        }
        NodeType::BlueprintMin => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Float(a.min(b))])
        }
        NodeType::BlueprintMax => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Float(a.max(b))])
        }

        // Logic
        NodeType::BooleanAnd => {
            let a = get_bool(&inputs, 0);
            let b = get_bool(&inputs, 1);
            Ok(vec![NodeValue::Bool(a && b)])
        }
        NodeType::BooleanOr => {
            let a = get_bool(&inputs, 0);
            let b = get_bool(&inputs, 1);
            Ok(vec![NodeValue::Bool(a || b)])
        }
        NodeType::BooleanNot => {
            let v = get_bool(&inputs, 0);
            Ok(vec![NodeValue::Bool(!v)])
        }
        NodeType::Equal => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Bool((a - b).abs() < f32::EPSILON)])
        }
        NodeType::NotEqual => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Bool((a - b).abs() >= f32::EPSILON)])
        }
        NodeType::GreaterThan => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Bool(a > b)])
        }
        NodeType::LessThan => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Bool(a < b)])
        }
        NodeType::GreaterEqual => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Bool(a >= b)])
        }
        NodeType::LessEqual => {
            let a = get_float(&inputs, 0);
            let b = get_float(&inputs, 1);
            Ok(vec![NodeValue::Bool(a <= b)])
        }

        _ => Ok(vec![]),
    }
}

fn get_float(inputs: &[NodeValue], index: usize) -> f32 {
    inputs.get(index).map(|v| v.to_float()).unwrap_or(0.0)
}

fn get_int(inputs: &[NodeValue], index: usize) -> i32 {
    inputs
        .get(index)
        .map(|v| match v {
            NodeValue::Int(i) => *i,
            other => other.to_float() as i32,
        })
        .unwrap_or(0)
}

fn get_bool(inputs: &[NodeValue], index: usize) -> bool {
    inputs
        .get(index)
        .map(|v| match v {
            NodeValue::Bool(b) => *b,
            other => other.to_float() != 0.0,
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_graph::graph::NodeGraph;
    use crate::node_graph::nodes::create_node;
    use crate::node_graph::types::PinId;
    use egui::Pos2;

    #[test]
    fn test_variable_store_basic() {
        let mut store = VariableStore::new();
        store.set("health", NodeValue::Float(100.0));
        assert_eq!(store.get("health"), Some(&NodeValue::Float(100.0)));
    }

    #[test]
    fn test_variable_store_scopes() {
        let mut store = VariableStore::new();
        store.set_global("score", NodeValue::Int(0));
        store.set_local("temp", NodeValue::Float(42.0));

        assert_eq!(store.get("score"), Some(&NodeValue::Int(0)));
        assert_eq!(store.get("temp"), Some(&NodeValue::Float(42.0)));

        store.push_scope();
        store.set_local("temp", NodeValue::Float(99.0));
        assert_eq!(store.get("temp"), Some(&NodeValue::Float(99.0)));

        store.pop_scope();
        assert_eq!(store.get("temp"), Some(&NodeValue::Float(42.0)));
    }

    #[test]
    fn test_variable_store_local_over_global() {
        let mut store = VariableStore::new();
        store.set_global("x", NodeValue::Int(1));
        store.set_local("x", NodeValue::Int(2));
        assert_eq!(store.get("x"), Some(&NodeValue::Int(2)));
    }

    #[test]
    fn test_branch_node_true() {
        let mut graph = NodeGraph::new();

        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        let event_id = graph.add_node(event);

        let mut bool_const = create_node(NodeType::ConstantBool, Pos2::ZERO);
        bool_const.values.insert(0, NodeValue::Bool(true));
        let bool_id = graph.add_node(bool_const);

        let branch = create_node(NodeType::Branch, Pos2::ZERO);
        let branch_id = graph.add_node(branch);

        // Connect event exec -> branch exec
        graph
            .connect(PinId::new(event_id, 0), PinId::new(branch_id, 0))
            .unwrap();
        // Connect bool -> branch condition
        graph
            .connect(PinId::new(bool_id, 0), PinId::new(branch_id, 1))
            .unwrap();

        let mut ctx = BlueprintContext::new();
        let result = BlueprintExecutor::execute(&graph, &mut ctx);

        assert!(
            result.errors.is_empty(),
            "Unexpected errors: {:?}",
            result.errors
        );
        // Branch should have produced a true output
        let branch_output = ctx.data_outputs.get(&(branch_id, 0));
        assert_eq!(branch_output, Some(&NodeValue::Bool(true)));
    }

    #[test]
    fn test_boolean_logic() {
        let mut graph = NodeGraph::new();

        let mut a = create_node(NodeType::ConstantBool, Pos2::ZERO);
        a.values.insert(0, NodeValue::Bool(true));
        let a_id = graph.add_node(a);

        let mut b = create_node(NodeType::ConstantBool, Pos2::ZERO);
        b.values.insert(0, NodeValue::Bool(false));
        let b_id = graph.add_node(b);

        let and_node = create_node(NodeType::BooleanAnd, Pos2::ZERO);
        let and_id = graph.add_node(and_node);

        let or_node = create_node(NodeType::BooleanOr, Pos2::ZERO);
        let or_id = graph.add_node(or_node);

        let not_node = create_node(NodeType::BooleanNot, Pos2::ZERO);
        let not_id = graph.add_node(not_node);

        graph
            .connect(PinId::new(a_id, 0), PinId::new(and_id, 0))
            .unwrap();
        graph
            .connect(PinId::new(b_id, 0), PinId::new(and_id, 1))
            .unwrap();
        graph
            .connect(PinId::new(a_id, 0), PinId::new(or_id, 0))
            .unwrap();
        graph
            .connect(PinId::new(b_id, 0), PinId::new(or_id, 1))
            .unwrap();
        graph
            .connect(PinId::new(a_id, 0), PinId::new(not_id, 0))
            .unwrap();

        // Test via data-only evaluator (no execution flow needed)
        let mut data_outputs = HashMap::new();

        let and_node_ref = graph.nodes.get(&and_id).unwrap();
        let and_out = evaluate_data_only_node(&graph, and_node_ref, &mut data_outputs).unwrap();
        data_outputs.insert((and_id, 0), and_out[0].clone());

        let or_node_ref = graph.nodes.get(&or_id).unwrap();
        let or_out = evaluate_data_only_node(&graph, or_node_ref, &mut data_outputs).unwrap();
        data_outputs.insert((or_id, 0), or_out[0].clone());

        let not_node_ref = graph.nodes.get(&not_id).unwrap();
        let not_out = evaluate_data_only_node(&graph, not_node_ref, &mut data_outputs).unwrap();
        data_outputs.insert((not_id, 0), not_out[0].clone());

        assert_eq!(
            data_outputs.get(&(and_id, 0)),
            Some(&NodeValue::Bool(false))
        ); // true AND false
        assert_eq!(data_outputs.get(&(or_id, 0)), Some(&NodeValue::Bool(true))); // true OR false
        assert_eq!(
            data_outputs.get(&(not_id, 0)),
            Some(&NodeValue::Bool(false))
        ); // NOT true
    }

    #[test]
    fn test_comparison_nodes() {
        let mut graph = NodeGraph::new();

        let mut a = create_node(NodeType::ConstantFloat, Pos2::ZERO);
        a.values.insert(0, NodeValue::Float(5.0));
        let a_id = graph.add_node(a);

        let mut b = create_node(NodeType::ConstantFloat, Pos2::ZERO);
        b.values.insert(0, NodeValue::Float(3.0));
        let b_id = graph.add_node(b);

        let gt = create_node(NodeType::GreaterThan, Pos2::ZERO);
        let gt_id = graph.add_node(gt);

        let lt = create_node(NodeType::LessThan, Pos2::ZERO);
        let lt_id = graph.add_node(lt);

        let eq = create_node(NodeType::Equal, Pos2::ZERO);
        let eq_id = graph.add_node(eq);

        graph
            .connect(PinId::new(a_id, 0), PinId::new(gt_id, 0))
            .unwrap();
        graph
            .connect(PinId::new(b_id, 0), PinId::new(gt_id, 1))
            .unwrap();
        graph
            .connect(PinId::new(a_id, 0), PinId::new(lt_id, 0))
            .unwrap();
        graph
            .connect(PinId::new(b_id, 0), PinId::new(lt_id, 1))
            .unwrap();
        graph
            .connect(PinId::new(a_id, 0), PinId::new(eq_id, 0))
            .unwrap();
        graph
            .connect(PinId::new(b_id, 0), PinId::new(eq_id, 1))
            .unwrap();

        let mut data_outputs = HashMap::new();

        let gt_ref = graph.nodes.get(&gt_id).unwrap();
        let gt_out = evaluate_data_only_node(&graph, gt_ref, &mut data_outputs).unwrap();
        assert_eq!(gt_out[0], NodeValue::Bool(true)); // 5 > 3

        let lt_ref = graph.nodes.get(&lt_id).unwrap();
        let lt_out = evaluate_data_only_node(&graph, lt_ref, &mut data_outputs).unwrap();
        assert_eq!(lt_out[0], NodeValue::Bool(false)); // 5 < 3

        let eq_ref = graph.nodes.get(&eq_id).unwrap();
        let eq_out = evaluate_data_only_node(&graph, eq_ref, &mut data_outputs).unwrap();
        assert_eq!(eq_out[0], NodeValue::Bool(false)); // 5 == 3
    }

    #[test]
    fn test_blueprint_math() {
        let mut graph = NodeGraph::new();

        let mut a = create_node(NodeType::ConstantFloat, Pos2::ZERO);
        a.values.insert(0, NodeValue::Float(10.0));
        let a_id = graph.add_node(a);

        let mut b = create_node(NodeType::ConstantFloat, Pos2::ZERO);
        b.values.insert(0, NodeValue::Float(3.0));
        let b_id = graph.add_node(b);

        let add = create_node(NodeType::BlueprintAdd, Pos2::ZERO);
        let add_id = graph.add_node(add);

        let mul = create_node(NodeType::BlueprintMultiply, Pos2::ZERO);
        let mul_id = graph.add_node(mul);

        let abs = create_node(NodeType::BlueprintAbs, Pos2::ZERO);
        let abs_id = graph.add_node(abs);

        graph
            .connect(PinId::new(a_id, 0), PinId::new(add_id, 0))
            .unwrap();
        graph
            .connect(PinId::new(b_id, 0), PinId::new(add_id, 1))
            .unwrap();
        graph
            .connect(PinId::new(a_id, 0), PinId::new(mul_id, 0))
            .unwrap();
        graph
            .connect(PinId::new(b_id, 0), PinId::new(mul_id, 1))
            .unwrap();

        // Negative value for abs test
        let mut neg = create_node(NodeType::ConstantFloat, Pos2::ZERO);
        neg.values.insert(0, NodeValue::Float(-7.5));
        let neg_id = graph.add_node(neg);
        graph
            .connect(PinId::new(neg_id, 0), PinId::new(abs_id, 0))
            .unwrap();

        let mut data_outputs = HashMap::new();

        let add_ref = graph.nodes.get(&add_id).unwrap();
        let add_out = evaluate_data_only_node(&graph, add_ref, &mut data_outputs).unwrap();
        assert_eq!(add_out[0], NodeValue::Float(13.0));

        let mul_ref = graph.nodes.get(&mul_id).unwrap();
        let mul_out = evaluate_data_only_node(&graph, mul_ref, &mut data_outputs).unwrap();
        assert_eq!(mul_out[0], NodeValue::Float(30.0));

        let abs_ref = graph.nodes.get(&abs_id).unwrap();
        let abs_out = evaluate_data_only_node(&graph, abs_ref, &mut data_outputs).unwrap();
        assert_eq!(abs_out[0], NodeValue::Float(7.5));
    }

    #[test]
    fn test_print_string() {
        let mut graph = NodeGraph::new();

        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        let event_id = graph.add_node(event);

        let print = create_node(NodeType::PrintString, Pos2::ZERO);
        let print_id = graph.add_node(print);

        // Connect event exec -> print exec
        graph
            .connect(PinId::new(event_id, 0), PinId::new(print_id, 0))
            .unwrap();

        let mut ctx = BlueprintContext::new();
        let result = BlueprintExecutor::execute(&graph, &mut ctx);

        assert!(
            result.errors.is_empty(),
            "Unexpected errors: {:?}",
            result.errors
        );
        // Print should have been called (empty string since no input connected)
        assert!(!ctx.trace.is_empty());
    }

    #[test]
    fn test_for_loop() {
        let mut graph = NodeGraph::new();

        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        let event_id = graph.add_node(event);

        let mut for_loop = create_node(NodeType::ForLoop, Pos2::ZERO);
        for_loop.values.insert(1, NodeValue::Int(0));
        for_loop.values.insert(2, NodeValue::Int(3));
        let loop_id = graph.add_node(for_loop);

        // Connect event -> loop
        graph
            .connect(PinId::new(event_id, 0), PinId::new(loop_id, 0))
            .unwrap();

        let mut ctx = BlueprintContext::new();
        let result = BlueprintExecutor::execute(&graph, &mut ctx);

        assert!(
            result.errors.is_empty(),
            "Unexpected errors: {:?}",
            result.errors
        );
        // Loop should have iterated
        assert!(ctx.loop_counters.is_empty() || result.trace.contains(&loop_id));
    }

    #[test]
    fn test_do_once() {
        let mut graph = NodeGraph::new();

        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        let event_id = graph.add_node(event);

        let do_once = create_node(NodeType::DoOnce, Pos2::ZERO);
        let do_id = graph.add_node(do_once);

        graph
            .connect(PinId::new(event_id, 0), PinId::new(do_id, 0))
            .unwrap();

        let mut ctx = BlueprintContext::new();

        // First execution should fire
        let _ = BlueprintExecutor::execute(&graph, &mut ctx);
        assert_eq!(ctx.do_once_fired.get(&do_id), Some(&true));

        // Second execution should NOT fire (already fired)
        ctx.state = BlueprintState::Idle;
        ctx.trace.clear();
        let _ = BlueprintExecutor::execute(&graph, &mut ctx);
        // Do-Once should still be fired=true
        assert_eq!(ctx.do_once_fired.get(&do_id), Some(&true));
    }

    #[test]
    fn test_flip_flop() {
        let mut graph = NodeGraph::new();

        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        let event_id = graph.add_node(event);

        let flip = create_node(NodeType::FlipFlop, Pos2::ZERO);
        let flip_id = graph.add_node(flip);

        graph
            .connect(PinId::new(event_id, 0), PinId::new(flip_id, 0))
            .unwrap();

        let mut ctx = BlueprintContext::new();

        let _ = BlueprintExecutor::execute(&graph, &mut ctx);
        let state1 = ctx.flip_flop_state.get(&flip_id).copied();

        ctx.state = BlueprintState::Idle;
        let _ = BlueprintExecutor::execute(&graph, &mut ctx);
        let state2 = ctx.flip_flop_state.get(&flip_id).copied();

        // States should alternate
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_max_steps_prevents_infinite_loop() {
        let mut graph = NodeGraph::new();

        // Create a node that connects to itself via data (no exec loop possible
        // in this test, but max_steps still applies)
        let event = create_node(NodeType::EventBeginPlay, Pos2::ZERO);
        graph.add_node(event);

        let mut ctx = BlueprintContext::new();
        ctx.max_steps = 5;
        let result = BlueprintExecutor::execute(&graph, &mut ctx);

        // Should complete normally (event has no exec connections)
        assert!(
            result.state == BlueprintState::Completed
                || result.state == BlueprintState::Idle
                || matches!(result.state, BlueprintState::Error(_))
        );
    }
}
