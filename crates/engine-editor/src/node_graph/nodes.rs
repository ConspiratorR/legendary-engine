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

        // ── Color nodes ──
        NodeType::CombineRgb => create_combine_rgb(position),
        NodeType::SplitRgb => create_split_rgb(position),

        // ── Vector nodes ──
        NodeType::DotProduct => create_dot_product(position),
        NodeType::Normalize => create_normalize(position),
        NodeType::CrossProduct => create_cross_product(position),

        // ── Output ──
        NodeType::MaterialOutput => create_material_output(position),

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
        // Color
        NodeType::CombineRgb,
        NodeType::SplitRgb,
        // Vector
        NodeType::DotProduct,
        NodeType::Normalize,
        NodeType::CrossProduct,
        // Output
        NodeType::MaterialOutput,
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
