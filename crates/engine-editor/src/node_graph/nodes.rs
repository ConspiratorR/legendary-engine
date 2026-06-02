use egui::Pos2;

use super::graph::{Node, NodeType};
use super::types::{NodeValue, PinType};

/// Create a node of the given type at the specified position.
pub fn create_node(node_type: NodeType, position: Pos2) -> Node {
    match node_type {
        // ── Input nodes ──
        NodeType::ConstantFloat => create_constant_float(position),
        NodeType::ConstantVec2 => create_constant_vec2(position),
        NodeType::ConstantVec3 => create_constant_vec3(position),
        NodeType::ConstantVec4 => create_constant_vec4(position),
        NodeType::ConstantColor => create_constant_color(position),
        NodeType::ConstantBool => create_constant_bool(position),
        NodeType::ConstantInt => create_constant_int(position),
        NodeType::UVCoordinate => create_uv_coordinate(position),
        NodeType::Time => create_time(position),

        // ── Math nodes ──
        NodeType::Add => create_add(position),
        NodeType::Subtract => create_subtract(position),
        NodeType::Multiply => create_multiply(position),
        NodeType::Divide => create_divide(position),
        NodeType::Sin => create_sin(position),
        NodeType::Cos => create_cos(position),
        NodeType::Abs => create_abs(position),
        NodeType::Clamp => create_clamp(position),
        NodeType::Lerp => create_lerp(position),
        NodeType::Power => create_power(position),
        NodeType::Saturate => create_saturate(position),
        NodeType::Negate => create_negate(position),

        // ── Texture nodes ──
        NodeType::TextureSample => create_texture_sample(position),
        NodeType::NormalMap => create_normal_map(position),
        NodeType::Flipbook => create_flipbook(position),

        // ── Color nodes ──
        NodeType::CombineRgb => create_combine_rgb(position),
        NodeType::SplitRgb => create_split_rgb(position),
        NodeType::Mix => create_mix(position),

        // ── PBR helper nodes ──
        NodeType::Fresnel => create_fresnel(position),

        // ── Vector nodes ──
        NodeType::DotProduct => create_dot_product(position),
        NodeType::Normalize => create_normalize(position),
        NodeType::CrossProduct => create_cross_product(position),

        // ── Output ──
        NodeType::MaterialOutput => create_material_output(position),

        // ── Blueprint nodes ──
        NodeType::EventBeginPlay => create_event_begin_play(position),
        NodeType::EventTick => create_event_tick(position),
        NodeType::EventCustom(ref name) => create_event_custom(position, name),
        NodeType::Branch => create_branch(position),
        NodeType::ForLoop => create_for_loop(position),
        NodeType::ForEachLoop => create_for_each_loop(position),
        NodeType::Sequence => create_sequence(position),
        NodeType::FlipFlop => create_flip_flop(position),
        NodeType::DoOnce => create_do_once(position),
        NodeType::Delay => create_delay_node(position),
        NodeType::VariableGet => create_variable_get(position),
        NodeType::VariableSet => create_variable_set(position),
        NodeType::BooleanAnd => create_boolean_and(position),
        NodeType::BooleanOr => create_boolean_or(position),
        NodeType::BooleanNot => create_boolean_not(position),
        NodeType::Equal => create_equal(position),
        NodeType::NotEqual => create_not_equal(position),
        NodeType::GreaterThan => create_greater_than(position),
        NodeType::LessThan => create_less_than(position),
        NodeType::GreaterEqual => create_greater_equal(position),
        NodeType::LessEqual => create_less_equal(position),
        NodeType::FunctionCall => create_function_call(position),
        NodeType::PrintString => create_print_string(position),
        NodeType::BlueprintAdd => create_bp_add(position),
        NodeType::BlueprintSubtract => create_bp_subtract(position),
        NodeType::BlueprintMultiply => create_bp_multiply(position),
        NodeType::BlueprintDivide => create_bp_divide(position),
        NodeType::BlueprintClamp => create_bp_clamp(position),
        NodeType::BlueprintAbs => create_bp_abs(position),
        NodeType::BlueprintMin => create_bp_min(position),
        NodeType::BlueprintMax => create_bp_max(position),

        NodeType::Custom(ref name) => {
            let name_clone = name.clone();
            let mut node = Node::new(0, node_type, &name_clone, position);
            node.add_input("In", PinType::Float);
            node.add_output("Out", PinType::Float);
            node
        }
    }
}

/// Get all available built-in node types.
pub fn builtin_node_types() -> Vec<NodeType> {
    vec![
        // Input
        NodeType::ConstantFloat,
        NodeType::ConstantVec2,
        NodeType::ConstantVec3,
        NodeType::ConstantVec4,
        NodeType::ConstantColor,
        NodeType::ConstantBool,
        NodeType::ConstantInt,
        NodeType::UVCoordinate,
        NodeType::Time,
        // Math
        NodeType::Add,
        NodeType::Subtract,
        NodeType::Multiply,
        NodeType::Divide,
        NodeType::Sin,
        NodeType::Cos,
        NodeType::Abs,
        NodeType::Clamp,
        NodeType::Lerp,
        NodeType::Power,
        NodeType::Saturate,
        NodeType::Negate,
        // Texture
        NodeType::TextureSample,
        NodeType::NormalMap,
        NodeType::Flipbook,
        // Color
        NodeType::CombineRgb,
        NodeType::SplitRgb,
        NodeType::Mix,
        // PBR helper
        NodeType::Fresnel,
        // Vector
        NodeType::DotProduct,
        NodeType::Normalize,
        NodeType::CrossProduct,
        // Output
        NodeType::MaterialOutput,
        // Blueprint - Flow Control
        NodeType::EventBeginPlay,
        NodeType::EventTick,
        NodeType::Branch,
        NodeType::ForLoop,
        NodeType::Sequence,
        NodeType::FlipFlop,
        NodeType::DoOnce,
        NodeType::Delay,
        // Blueprint - Variable
        NodeType::VariableGet,
        NodeType::VariableSet,
        // Blueprint - Logic
        NodeType::BooleanAnd,
        NodeType::BooleanOr,
        NodeType::BooleanNot,
        NodeType::Equal,
        NodeType::NotEqual,
        NodeType::GreaterThan,
        NodeType::LessThan,
        NodeType::GreaterEqual,
        NodeType::LessEqual,
        // Blueprint - Function
        NodeType::FunctionCall,
        NodeType::PrintString,
        // Blueprint - Math
        NodeType::BlueprintAdd,
        NodeType::BlueprintSubtract,
        NodeType::BlueprintMultiply,
        NodeType::BlueprintDivide,
        NodeType::BlueprintClamp,
        NodeType::BlueprintAbs,
        NodeType::BlueprintMin,
        NodeType::BlueprintMax,
    ]
}

// ── Input node factories ──

fn create_constant_float(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ConstantFloat, "Float", position);
    node.add_output("Value", PinType::Float);
    node.values.insert(0, NodeValue::Float(0.0));
    node
}

fn create_constant_vec2(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ConstantVec2, "Vec2", position);
    node.add_output("Value", PinType::Vec2);
    node.values.insert(0, NodeValue::Vec2([0.0, 0.0]));
    node
}

fn create_constant_vec3(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ConstantVec3, "Vec3", position);
    node.add_output("Value", PinType::Vec3);
    node.values.insert(0, NodeValue::Vec3([0.0, 0.0, 0.0]));
    node
}

fn create_constant_vec4(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ConstantVec4, "Vec4", position);
    node.add_output("Value", PinType::Vec4);
    node.values.insert(0, NodeValue::Vec4([0.0, 0.0, 0.0, 0.0]));
    node
}

fn create_constant_color(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ConstantColor, "Color", position);
    node.add_output("Color", PinType::Color);
    node.values
        .insert(0, NodeValue::Color([1.0, 1.0, 1.0, 1.0]));
    node
}

fn create_constant_bool(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ConstantBool, "Bool", position);
    node.add_output("Value", PinType::Bool);
    node.values.insert(0, NodeValue::Bool(false));
    node
}

fn create_constant_int(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ConstantInt, "Int", position);
    node.add_output("Value", PinType::Int);
    node.values.insert(0, NodeValue::Int(0));
    node
}

fn create_uv_coordinate(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::UVCoordinate, "UV", position);
    node.add_output("UV", PinType::Vec2);
    node.add_output("U", PinType::Float);
    node.add_output("V", PinType::Float);
    node
}

fn create_time(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Time, "Time", position);
    node.add_output("Time", PinType::Float);
    node.add_output("Sin", PinType::Float);
    node.add_output("Cos", PinType::Float);
    node
}

// ── Math node factories ──

fn create_binary_op(position: Pos2, node_type: NodeType, name: &str) -> Node {
    let mut node = Node::new(0, node_type, name, position);
    node.add_input("A", PinType::Float);
    node.add_input("B", PinType::Float);
    node.add_output("Result", PinType::Float);
    node
}

fn create_add(position: Pos2) -> Node {
    create_binary_op(position, NodeType::Add, "Add")
}

fn create_subtract(position: Pos2) -> Node {
    create_binary_op(position, NodeType::Subtract, "Subtract")
}

fn create_multiply(position: Pos2) -> Node {
    create_binary_op(position, NodeType::Multiply, "Multiply")
}

fn create_divide(position: Pos2) -> Node {
    create_binary_op(position, NodeType::Divide, "Divide")
}

fn create_power(position: Pos2) -> Node {
    create_binary_op(position, NodeType::Power, "Power")
}

fn create_unary_op(position: Pos2, node_type: NodeType, name: &str) -> Node {
    let mut node = Node::new(0, node_type, name, position);
    node.add_input("Value", PinType::Float);
    node.add_output("Result", PinType::Float);
    node
}

fn create_sin(position: Pos2) -> Node {
    create_unary_op(position, NodeType::Sin, "Sine")
}

fn create_cos(position: Pos2) -> Node {
    create_unary_op(position, NodeType::Cos, "Cosine")
}

fn create_abs(position: Pos2) -> Node {
    create_unary_op(position, NodeType::Abs, "Absolute")
}

fn create_saturate(position: Pos2) -> Node {
    create_unary_op(position, NodeType::Saturate, "Saturate")
}

fn create_negate(position: Pos2) -> Node {
    create_unary_op(position, NodeType::Negate, "Negate")
}

fn create_clamp(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Clamp, "Clamp", position);
    node.add_input("Value", PinType::Float);
    node.add_input("Min", PinType::Float);
    node.add_input("Max", PinType::Float);
    node.add_output("Result", PinType::Float);
    node.values.insert(1, NodeValue::Float(0.0));
    node.values.insert(2, NodeValue::Float(1.0));
    node
}

fn create_lerp(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Lerp, "Lerp", position);
    node.add_input("A", PinType::Float);
    node.add_input("B", PinType::Float);
    node.add_input("Alpha", PinType::Float);
    node.add_output("Result", PinType::Float);
    node.values.insert(2, NodeValue::Float(0.5));
    node
}

// ── Texture node factories ──

fn create_texture_sample(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::TextureSample, "Texture Sample", position);
    node.add_input("UV", PinType::Vec2);
    node.add_output("Color", PinType::Vec4);
    node.add_output("R", PinType::Float);
    node.add_output("G", PinType::Float);
    node.add_output("B", PinType::Float);
    node.add_output("A", PinType::Float);
    node.values
        .insert(100, NodeValue::Vec4([0.0, 0.0, 0.0, 0.0])); // texture path placeholder
    node
}

fn create_normal_map(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::NormalMap, "Normal Map", position);
    node.add_input("Normal", PinType::Vec3);
    node.add_input("Strength", PinType::Float);
    node.add_output("Normal", PinType::Vec3);
    node.values.insert(1, NodeValue::Float(1.0));
    node
}

fn create_flipbook(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Flipbook, "Flipbook", position);
    node.add_input("UV", PinType::Vec2);
    node.add_input("Columns", PinType::Int);
    node.add_input("Rows", PinType::Int);
    node.add_input("Index", PinType::Int);
    node.add_output("UV", PinType::Vec2);
    node.values.insert(1, NodeValue::Int(4));
    node.values.insert(2, NodeValue::Int(4));
    node.values.insert(3, NodeValue::Int(0));
    node
}

// ── Color node factories ──

fn create_combine_rgb(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::CombineRgb, "Combine RGB", position);
    node.add_input("R", PinType::Float);
    node.add_input("G", PinType::Float);
    node.add_input("B", PinType::Float);
    node.add_output("Color", PinType::Color);
    node.values.insert(0, NodeValue::Float(0.0));
    node.values.insert(1, NodeValue::Float(0.0));
    node.values.insert(2, NodeValue::Float(0.0));
    node
}

fn create_split_rgb(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::SplitRgb, "Split RGB", position);
    node.add_input("Color", PinType::Color);
    node.add_output("R", PinType::Float);
    node.add_output("G", PinType::Float);
    node.add_output("B", PinType::Float);
    node.add_output("A", PinType::Float);
    node
}

fn create_mix(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Mix, "Mix", position);
    node.add_input("A", PinType::Color);
    node.add_input("B", PinType::Color);
    node.add_input("Factor", PinType::Float);
    node.add_output("Result", PinType::Color);
    node.values.insert(2, NodeValue::Float(0.5));
    node
}

fn create_fresnel(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Fresnel, "Fresnel", position);
    node.add_input("Normal", PinType::Vec3);
    node.add_input("ViewDir", PinType::Vec3);
    node.add_input("Power", PinType::Float);
    node.add_output("Result", PinType::Float);
    node.values.insert(2, NodeValue::Float(5.0));
    node
}

// ── Vector node factories ──

fn create_dot_product(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::DotProduct, "Dot Product", position);
    node.add_input("A", PinType::Vec3);
    node.add_input("B", PinType::Vec3);
    node.add_output("Result", PinType::Float);
    node
}

fn create_normalize(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Normalize, "Normalize", position);
    node.add_input("Vector", PinType::Vec3);
    node.add_output("Result", PinType::Vec3);
    node
}

fn create_cross_product(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::CrossProduct, "Cross Product", position);
    node.add_input("A", PinType::Vec3);
    node.add_input("B", PinType::Vec3);
    node.add_output("Result", PinType::Vec3);
    node
}

// ── Output node factories ──

fn create_material_output(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::MaterialOutput, "Material Output", position);
    node.add_input("Base Color", PinType::Color);
    node.add_input("Metallic", PinType::Float);
    node.add_input("Roughness", PinType::Float);
    node.add_input("Normal", PinType::Vec3);
    node.add_input("Emissive", PinType::Color);
    node.add_input("AO", PinType::Float);
    node.values.insert(1, NodeValue::Float(0.0));
    node.values.insert(2, NodeValue::Float(0.5));
    node.values.insert(5, NodeValue::Float(1.0));
    node
}

// ── Blueprint flow control factories ──

fn create_event_begin_play(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::EventBeginPlay, "BeginPlay", position);
    node.add_output("Exec", PinType::Execution);
    node
}

fn create_event_tick(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::EventTick, "Tick", position);
    node.add_output("Exec", PinType::Execution);
    node.add_output("Delta Time", PinType::Float);
    node
}

fn create_event_custom(position: Pos2, name: &str) -> Node {
    let mut node = Node::new(0, NodeType::EventCustom(name.to_string()), name, position);
    node.add_output("Exec", PinType::Execution);
    node
}

fn create_branch(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Branch, "Branch", position);
    node.add_input("Exec", PinType::Execution);
    node.add_input("Condition", PinType::Bool);
    node.add_output("True", PinType::Execution);
    node.add_output("False", PinType::Execution);
    node
}

fn create_for_loop(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ForLoop, "For Loop", position);
    node.add_input("Exec", PinType::Execution);
    node.add_input("First Index", PinType::Int);
    node.add_input("Last Index", PinType::Int);
    node.add_output("Loop Body", PinType::Execution);
    node.add_output("Index", PinType::Int);
    node.add_output("Completed", PinType::Execution);
    node.values.insert(1, NodeValue::Int(0));
    node.values.insert(2, NodeValue::Int(10));
    node
}

fn create_for_each_loop(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::ForEachLoop, "For Each Loop", position);
    node.add_input("Exec", PinType::Execution);
    node.add_input("Array", PinType::Wildcard);
    node.add_output("Loop Body", PinType::Execution);
    node.add_output("Element", PinType::Wildcard);
    node.add_output("Index", PinType::Int);
    node.add_output("Completed", PinType::Execution);
    node
}

fn create_sequence(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Sequence, "Sequence", position);
    node.add_input("Exec", PinType::Execution);
    node.add_output("Then 0", PinType::Execution);
    node.add_output("Then 1", PinType::Execution);
    node.add_output("Then 2", PinType::Execution);
    node
}

fn create_flip_flop(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::FlipFlop, "Flip Flop", position);
    node.add_input("Exec", PinType::Execution);
    node.add_output("A", PinType::Execution);
    node.add_output("B", PinType::Execution);
    node.add_output("Is A", PinType::Bool);
    node
}

fn create_do_once(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::DoOnce, "Do Once", position);
    node.add_input("Exec", PinType::Execution);
    node.add_input("Reset", PinType::Execution);
    node.add_output("Out", PinType::Execution);
    node
}

fn create_delay_node(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::Delay, "Delay", position);
    node.add_input("Exec", PinType::Execution);
    node.add_input("Duration", PinType::Float);
    node.add_output("Out", PinType::Execution);
    node.values.insert(1, NodeValue::Float(1.0));
    node
}

// ── Blueprint variable factories ──

fn create_variable_get(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::VariableGet, "Get Variable", position);
    node.add_input("Name", PinType::Wildcard);
    node.add_output("Value", PinType::Wildcard);
    node.values.insert(0, NodeValue::Vec4([0.0, 0.0, 0.0, 0.0])); // placeholder
    node
}

fn create_variable_set(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::VariableSet, "Set Variable", position);
    node.add_input("Exec", PinType::Execution);
    node.add_input("Name", PinType::Wildcard);
    node.add_input("Value", PinType::Wildcard);
    node.add_output("Exec", PinType::Execution);
    node.add_output("Value", PinType::Wildcard);
    node
}

// ── Blueprint logic factories ──

fn create_boolean_and(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::BooleanAnd, "AND", position);
    node.add_input("A", PinType::Bool);
    node.add_input("B", PinType::Bool);
    node.add_output("Result", PinType::Bool);
    node
}

fn create_boolean_or(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::BooleanOr, "OR", position);
    node.add_input("A", PinType::Bool);
    node.add_input("B", PinType::Bool);
    node.add_output("Result", PinType::Bool);
    node
}

fn create_boolean_not(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::BooleanNot, "NOT", position);
    node.add_input("Value", PinType::Bool);
    node.add_output("Result", PinType::Bool);
    node
}

fn create_comparison_node(position: Pos2, node_type: NodeType, name: &str) -> Node {
    let mut node = Node::new(0, node_type, name, position);
    node.add_input("A", PinType::Float);
    node.add_input("B", PinType::Float);
    node.add_output("Result", PinType::Bool);
    node
}

fn create_equal(position: Pos2) -> Node {
    create_comparison_node(position, NodeType::Equal, "Equal")
}

fn create_not_equal(position: Pos2) -> Node {
    create_comparison_node(position, NodeType::NotEqual, "Not Equal")
}

fn create_greater_than(position: Pos2) -> Node {
    create_comparison_node(position, NodeType::GreaterThan, "Greater Than")
}

fn create_less_than(position: Pos2) -> Node {
    create_comparison_node(position, NodeType::LessThan, "Less Than")
}

fn create_greater_equal(position: Pos2) -> Node {
    create_comparison_node(position, NodeType::GreaterEqual, "Greater Equal")
}

fn create_less_equal(position: Pos2) -> Node {
    create_comparison_node(position, NodeType::LessEqual, "Less Equal")
}

// ── Blueprint function factories ──

fn create_function_call(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::FunctionCall, "Function Call", position);
    node.add_input("Exec", PinType::Execution);
    node.add_output("Exec", PinType::Execution);
    node.add_output("Return", PinType::Wildcard);
    node
}

fn create_print_string(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::PrintString, "Print String", position);
    node.add_input("Exec", PinType::Execution);
    node.add_input("String", PinType::Wildcard);
    node.add_output("Exec", PinType::Execution);
    node
}

// ── Blueprint math factories ──

fn create_bp_binary_op(position: Pos2, node_type: NodeType, name: &str) -> Node {
    let mut node = Node::new(0, node_type, name, position);
    node.add_input("A", PinType::Float);
    node.add_input("B", PinType::Float);
    node.add_output("Result", PinType::Float);
    node
}

fn create_bp_add(position: Pos2) -> Node {
    create_bp_binary_op(position, NodeType::BlueprintAdd, "Add")
}

fn create_bp_subtract(position: Pos2) -> Node {
    create_bp_binary_op(position, NodeType::BlueprintSubtract, "Subtract")
}

fn create_bp_multiply(position: Pos2) -> Node {
    create_bp_binary_op(position, NodeType::BlueprintMultiply, "Multiply")
}

fn create_bp_divide(position: Pos2) -> Node {
    create_bp_binary_op(position, NodeType::BlueprintDivide, "Divide")
}

fn create_bp_clamp(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::BlueprintClamp, "Clamp", position);
    node.add_input("Value", PinType::Float);
    node.add_input("Min", PinType::Float);
    node.add_input("Max", PinType::Float);
    node.add_output("Result", PinType::Float);
    node.values.insert(1, NodeValue::Float(0.0));
    node.values.insert(2, NodeValue::Float(1.0));
    node
}

fn create_bp_abs(position: Pos2) -> Node {
    let mut node = Node::new(0, NodeType::BlueprintAbs, "Abs", position);
    node.add_input("Value", PinType::Float);
    node.add_output("Result", PinType::Float);
    node
}

fn create_bp_min(position: Pos2) -> Node {
    create_bp_binary_op(position, NodeType::BlueprintMin, "Min")
}

fn create_bp_max(position: Pos2) -> Node {
    create_bp_binary_op(position, NodeType::BlueprintMax, "Max")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_all_builtin_nodes() {
        let types = builtin_node_types();
        assert!(
            types.len() >= 25,
            "Expected at least 25 builtin node types, got {}",
            types.len()
        );

        for node_type in &types {
            let node = create_node(node_type.clone(), Pos2::new(0.0, 0.0));
            assert!(!node.name.is_empty(), "Node {:?} has empty name", node_type);
        }
    }

    #[test]
    fn test_constant_float_node() {
        let node = create_node(NodeType::ConstantFloat, Pos2::new(10.0, 20.0));
        assert_eq!(node.name, "Float");
        assert_eq!(node.outputs.len(), 1);
        assert_eq!(node.outputs[0].pin_type, PinType::Float);
        assert_eq!(node.values.get(&0), Some(&NodeValue::Float(0.0)));
    }

    #[test]
    fn test_add_node() {
        let node = create_node(NodeType::Add, Pos2::new(0.0, 0.0));
        assert_eq!(node.name, "Add");
        assert_eq!(node.inputs.len(), 2);
        assert_eq!(node.outputs.len(), 1);
        assert_eq!(node.inputs[0].pin_type, PinType::Float);
        assert_eq!(node.inputs[1].pin_type, PinType::Float);
        assert_eq!(node.outputs[0].pin_type, PinType::Float);
    }

    #[test]
    fn test_lerp_node() {
        let node = create_node(NodeType::Lerp, Pos2::new(0.0, 0.0));
        assert_eq!(node.name, "Lerp");
        assert_eq!(node.inputs.len(), 3);
        assert_eq!(node.outputs.len(), 1);
        assert_eq!(node.values.get(&2), Some(&NodeValue::Float(0.5)));
    }

    #[test]
    fn test_material_output_node() {
        let node = create_node(NodeType::MaterialOutput, Pos2::new(0.0, 0.0));
        assert_eq!(node.name, "Material Output");
        assert_eq!(node.inputs.len(), 6);
        assert_eq!(node.inputs[0].pin_type, PinType::Color);
        assert_eq!(node.inputs[1].pin_type, PinType::Float);
        assert_eq!(node.inputs[2].pin_type, PinType::Float);
        assert_eq!(node.inputs[3].pin_type, PinType::Vec3);
        assert_eq!(node.inputs[4].pin_type, PinType::Color);
        assert_eq!(node.inputs[5].pin_type, PinType::Float);
    }

    #[test]
    fn test_texture_sample_node() {
        let node = create_node(NodeType::TextureSample, Pos2::new(0.0, 0.0));
        assert_eq!(node.name, "Texture Sample");
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.outputs.len(), 5);
        assert_eq!(node.inputs[0].pin_type, PinType::Vec2);
        assert_eq!(node.outputs[0].pin_type, PinType::Vec4);
    }

    #[test]
    fn test_custom_node() {
        let node = create_node(NodeType::Custom("MyNode".to_string()), Pos2::new(0.0, 0.0));
        assert_eq!(node.name, "MyNode");
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.outputs.len(), 1);
    }

    #[test]
    fn test_builtin_node_types_count() {
        let types = builtin_node_types();
        assert!(types.len() >= 25, "Expected at least 25 builtin node types");
    }
}
