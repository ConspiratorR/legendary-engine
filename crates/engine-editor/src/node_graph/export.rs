use std::fmt::Write;

use super::evaluator::{self, EvalContext, EvalResult};
use super::graph::{NodeGraph, NodeType};
use super::types::{NodeId, NodeValue};

/// Material parameters extracted from the node graph.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaterialParams {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub ao: f32,
    pub normal: [f32; 3],
}

/// Extract material parameters from a graph by evaluating it.
pub fn extract_material_params(graph: &NodeGraph) -> MaterialParams {
    let result = evaluator::evaluate(graph, &EvalContext::default());

    let mut params = MaterialParams::default();

    // Find the MaterialOutput node
    let output_node = graph
        .nodes
        .values()
        .find(|n| n.node_type == NodeType::MaterialOutput);

    if let Some(node) = output_node {
        // Get values from connections or defaults
        for (i, pin) in node.inputs.iter().enumerate() {
            let value = get_input_value(graph, node.id, i, &result);

            match pin.name.as_str() {
                "Base Color" => {
                    let v4 = value.to_vec4();
                    params.base_color = v4;
                }
                "Metallic" => {
                    params.metallic = value.to_float();
                }
                "Roughness" => {
                    params.roughness = value.to_float();
                }
                "Normal" => {
                    let v4 = value.to_vec4();
                    params.normal = [v4[0], v4[1], v4[2]];
                }
                "Emissive" => {
                    let v4 = value.to_vec4();
                    params.emissive = [v4[0], v4[1], v4[2]];
                }
                "AO" => {
                    params.ao = value.to_float();
                }
                _ => {}
            }
        }
    }

    params
}

fn get_input_value(
    graph: &NodeGraph,
    node_id: NodeId,
    input_index: usize,
    result: &EvalResult,
) -> NodeValue {
    let node = &graph.nodes[&node_id];
    let pin = &node.inputs[input_index];

    // Check for connection
    if let Some(conn) = graph.get_connection_to_input(pin.id) {
        let src_node_id = conn.output_pin.node_id;
        let src_node = &graph.nodes[&src_node_id];
        let output_idx = conn.output_pin.index - src_node.inputs.len();

        if let Some(value) = result.outputs.get(&(src_node_id, output_idx)) {
            return value.clone();
        }
    }

    // Use default
    node.values
        .get(&input_index)
        .cloned()
        .unwrap_or_else(|| pin.default_value.clone())
}

/// Generate WGSL shader code from the node graph.
pub fn generate_wgsl(graph: &NodeGraph) -> String {
    let mut wgsl = String::new();

    // Header
    wgsl.push_str("// Auto-generated material shader\n");
    wgsl.push_str("// Generated from node graph\n\n");

    // Structs
    wgsl.push_str("struct MaterialParams {\n");
    wgsl.push_str("    base_color: vec4<f32>,\n");
    wgsl.push_str("    metallic: f32,\n");
    wgsl.push_str("    roughness: f32,\n");
    wgsl.push_str("    ao: f32,\n");
    wgsl.push_str("    emissive: vec3<f32>,\n");
    wgsl.push_str("    normal: vec3<f32>,\n");
    wgsl.push_str("};\n\n");

    // Helper functions
    wgsl.push_str("fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {\n");
    wgsl.push_str("    return a + (b - a) * t;\n");
    wgsl.push_str("}\n\n");

    wgsl.push_str("fn saturate_f32(x: f32) -> f32 {\n");
    wgsl.push_str("    return clamp(x, 0.0, 1.0);\n");
    wgsl.push_str("}\n\n");

    // Main material function
    wgsl.push_str("fn evaluate_material(uv: vec2<f32>, time: f32) -> MaterialParams {\n");
    wgsl.push_str("    var params: MaterialParams;\n");

    // Generate code for each node in evaluation order
    let result = evaluator::evaluate(graph, &EvalContext::default());

    for &node_id in &result.order {
        let node = match graph.nodes.get(&node_id) {
            Some(n) => n,
            None => continue,
        };

        let var_name = format!("n{}", node_id);

        match node.node_type {
            // Blueprint nodes don't produce WGSL — skip them
            NodeType::EventBeginPlay
            | NodeType::EventTick
            | NodeType::EventCustom(_)
            | NodeType::Branch
            | NodeType::ForLoop
            | NodeType::ForEachLoop
            | NodeType::Sequence
            | NodeType::FlipFlop
            | NodeType::DoOnce
            | NodeType::Delay
            | NodeType::VariableGet
            | NodeType::VariableSet
            | NodeType::BooleanAnd
            | NodeType::BooleanOr
            | NodeType::BooleanNot
            | NodeType::Equal
            | NodeType::NotEqual
            | NodeType::GreaterThan
            | NodeType::LessThan
            | NodeType::GreaterEqual
            | NodeType::LessEqual
            | NodeType::FunctionCall
            | NodeType::PrintString
            | NodeType::BlueprintAdd
            | NodeType::BlueprintSubtract
            | NodeType::BlueprintMultiply
            | NodeType::BlueprintDivide
            | NodeType::BlueprintClamp
            | NodeType::BlueprintAbs
            | NodeType::BlueprintMin
            | NodeType::BlueprintMax => continue,
            NodeType::ConstantFloat => {
                let val = node.values.get(&0).map(|v| v.to_float()).unwrap_or(0.0);
                let _ = writeln!(wgsl, "    let {} = {};", var_name, val);
            }
            NodeType::ConstantVec2 => {
                let val = node.values.get(&0).map(|v| v.to_vec4()).unwrap_or([0.0; 4]);
                let _ = writeln!(
                    wgsl,
                    "    let {} = vec2<f32>({}, {});",
                    var_name, val[0], val[1]
                );
            }
            NodeType::ConstantVec3 => {
                let val = node.values.get(&0).map(|v| v.to_vec4()).unwrap_or([0.0; 4]);
                let _ = writeln!(
                    wgsl,
                    "    let {} = vec3<f32>({}, {}, {});",
                    var_name, val[0], val[1], val[2]
                );
            }
            NodeType::ConstantVec4 | NodeType::ConstantColor => {
                let val = node.values.get(&0).map(|v| v.to_vec4()).unwrap_or([0.0; 4]);
                let _ = writeln!(
                    wgsl,
                    "    let {} = vec4<f32>({}, {}, {}, {});",
                    var_name, val[0], val[1], val[2], val[3]
                );
            }
            NodeType::ConstantBool => {
                let val = node.values.get(&0).map(|v| v.to_float()).unwrap_or(0.0);
                let _ = writeln!(
                    wgsl,
                    "    let {} = {};",
                    var_name,
                    if val > 0.5 { "true" } else { "false" }
                );
            }
            NodeType::ConstantInt => {
                let val = node.values.get(&0).map(|v| v.to_float()).unwrap_or(0.0) as i32;
                let _ = writeln!(wgsl, "    let {} = {};", var_name, val);
            }
            NodeType::UVCoordinate => {
                let _ = writeln!(wgsl, "    let {} = uv;", var_name);
            }
            NodeType::Time => {
                let _ = writeln!(wgsl, "    let {} = time;", var_name);
            }
            NodeType::Add => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let _ = writeln!(wgsl, "    let {} = {} + {};", var_name, a, b);
            }
            NodeType::Subtract => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let _ = writeln!(wgsl, "    let {} = {} - {};", var_name, a, b);
            }
            NodeType::Multiply => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let _ = writeln!(wgsl, "    let {} = {} * {};", var_name, a, b);
            }
            NodeType::Divide => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let _ = writeln!(wgsl, "    let {} = {} / {};", var_name, a, b);
            }
            NodeType::Sin => {
                let a = get_input_var(graph, node, 0, &result);
                let _ = writeln!(wgsl, "    let {} = sin({});", var_name, a);
            }
            NodeType::Cos => {
                let a = get_input_var(graph, node, 0, &result);
                let _ = writeln!(wgsl, "    let {} = cos({});", var_name, a);
            }
            NodeType::Abs => {
                let a = get_input_var(graph, node, 0, &result);
                let _ = writeln!(wgsl, "    let {} = abs({});", var_name, a);
            }
            NodeType::Clamp => {
                let v = get_input_var(graph, node, 0, &result);
                let min = get_input_var(graph, node, 1, &result);
                let max = get_input_var(graph, node, 2, &result);
                let _ = writeln!(
                    wgsl,
                    "    let {} = clamp({}, {}, {});",
                    var_name, v, min, max
                );
            }
            NodeType::Lerp => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let t = get_input_var(graph, node, 2, &result);
                let _ = writeln!(
                    wgsl,
                    "    let {} = lerp_f32({}, {}, {});",
                    var_name, a, b, t
                );
            }
            NodeType::Power => {
                let base = get_input_var(graph, node, 0, &result);
                let exp = get_input_var(graph, node, 1, &result);
                let _ = writeln!(wgsl, "    let {} = pow({}, {});", var_name, base, exp);
            }
            NodeType::Saturate => {
                let a = get_input_var(graph, node, 0, &result);
                let _ = writeln!(wgsl, "    let {} = saturate_f32({});", var_name, a);
            }
            NodeType::Negate => {
                let a = get_input_var(graph, node, 0, &result);
                let _ = writeln!(wgsl, "    let {} = -{};", var_name, a);
            }
            NodeType::DotProduct => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let _ = writeln!(wgsl, "    let {} = dot({}, {});", var_name, a, b);
            }
            NodeType::Normalize => {
                let a = get_input_var(graph, node, 0, &result);
                let _ = writeln!(wgsl, "    let {} = normalize({});", var_name, a);
            }
            NodeType::CrossProduct => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let _ = writeln!(wgsl, "    let {} = cross({}, {});", var_name, a, b);
            }
            NodeType::CombineRgb => {
                let r = get_input_var(graph, node, 0, &result);
                let g = get_input_var(graph, node, 1, &result);
                let b = get_input_var(graph, node, 2, &result);
                let _ = writeln!(
                    wgsl,
                    "    let {} = vec4<f32>({}, {}, {}, 1.0);",
                    var_name, r, g, b
                );
            }
            NodeType::SplitRgb => {
                let c = get_input_var(graph, node, 0, &result);
                let _ = writeln!(wgsl, "    let {}_r = {}.r;", var_name, c);
                let _ = writeln!(wgsl, "    let {}_g = {}.g;", var_name, c);
                let _ = writeln!(wgsl, "    let {}_b = {}.b;", var_name, c);
                let _ = writeln!(wgsl, "    let {}_a = {}.a;", var_name, c);
            }
            NodeType::TextureSample => {
                let _ = writeln!(
                    wgsl,
                    "    let {} = vec4<f32>(1.0, 1.0, 1.0, 1.0); // TODO: texture sample",
                    var_name
                );
            }
            NodeType::NormalMap => {
                let normal = get_input_var(graph, node, 0, &result);
                let strength = get_input_var(graph, node, 1, &result);
                let _ = writeln!(
                    wgsl,
                    "    let {} = normalize(vec3<f32>({}.xy * {}, max({}.z, 0.001)));",
                    var_name, normal, strength, normal
                );
            }
            NodeType::Flipbook => {
                let uv = get_input_var(graph, node, 0, &result);
                let cols = get_input_var(graph, node, 1, &result);
                let rows = get_input_var(graph, node, 2, &result);
                let index = get_input_var(graph, node, 3, &result);
                let _ = writeln!(
                    wgsl,
                    "    let {} = vec2<f32>(({}.x / {}) + (floor({} / {}) * (1.0 / {})), ({}.y / {}) + (floor({} / {}) * (1.0 / {})));",
                    var_name, uv, cols, index, cols, cols, uv, rows, index, rows, rows
                );
            }
            NodeType::Mix => {
                let a = get_input_var(graph, node, 0, &result);
                let b = get_input_var(graph, node, 1, &result);
                let t = get_input_var(graph, node, 2, &result);
                let _ = writeln!(wgsl, "    let {} = mix({}, {}, {});", var_name, a, b, t);
            }
            NodeType::Fresnel => {
                let normal = get_input_var(graph, node, 0, &result);
                let view_dir = get_input_var(graph, node, 1, &result);
                let power = get_input_var(graph, node, 2, &result);
                let _ = writeln!(
                    wgsl,
                    "    let {} = pow(1.0 - abs(dot({}, {})), {});",
                    var_name, normal, view_dir, power
                );
            }
            NodeType::MaterialOutput => {
                // Assign to output params
                let base_color = get_input_var(graph, node, 0, &result);
                let metallic = get_input_var(graph, node, 1, &result);
                let roughness = get_input_var(graph, node, 2, &result);
                let emissive = get_input_var(graph, node, 4, &result);
                let ao = get_input_var(graph, node, 5, &result);

                let _ = writeln!(wgsl, "    params.base_color = {};", base_color);
                let _ = writeln!(wgsl, "    params.metallic = {};", metallic);
                let _ = writeln!(wgsl, "    params.roughness = {};", roughness);
                let _ = writeln!(wgsl, "    params.emissive = {}.xyz;", emissive);
                let _ = writeln!(wgsl, "    params.ao = {};", ao);
            }
            NodeType::Custom(_) => {
                let _ = writeln!(wgsl, "    // Custom node: {}", node.name);
            }
        }
    }

    wgsl.push_str("\n    return params;\n");
    wgsl.push_str("}\n");

    wgsl
}

/// Convert MaterialParams to engine PbrMaterial.
///
/// This provides integration between the material graph system and the
/// engine's PbrMaterial component used for mesh rendering.
pub fn to_pbr_material(params: &MaterialParams) -> engine_render::resource::material::PbrMaterial {
    engine_render::resource::material::PbrMaterial {
        base_color: params.base_color,
        metallic: params.metallic,
        roughness: params.roughness,
        ao: params.ao,
        emissive: params.emissive,
        base_color_texture: None,
        normal_texture: None,
        metallic_roughness_texture: None,
    }
}

/// Extract PbrMaterial directly from a material graph.
///
/// Convenience function that evaluates the graph and converts to PbrMaterial.
pub fn extract_pbr_material(graph: &NodeGraph) -> engine_render::resource::material::PbrMaterial {
    let params = extract_material_params(graph);
    to_pbr_material(&params)
}

/// Get the variable name for a node's input, either from a connection or default.
fn get_input_var(
    graph: &NodeGraph,
    node: &super::graph::Node,
    input_index: usize,
    _result: &EvalResult,
) -> String {
    let pin = &node.inputs[input_index];

    if let Some(conn) = graph.get_connection_to_input(pin.id) {
        let src_node_id = conn.output_pin.node_id;
        let src_node = &graph.nodes[&src_node_id];

        if src_node.node_type == NodeType::SplitRgb {
            let output_idx = conn.output_pin.index - src_node.inputs.len();
            let suffix = match output_idx {
                0 => "_r",
                1 => "_g",
                2 => "_b",
                3 => "_a",
                _ => "",
            };
            return format!("n{}{}", src_node_id, suffix);
        }

        return format!("n{}", src_node_id);
    }

    // Default value
    let default = node
        .values
        .get(&input_index)
        .map(|v| v.to_float())
        .unwrap_or(0.0);
    format!("{}", default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_graph::graph::NodeGraph;
    use crate::node_graph::nodes::create_node;
    use crate::node_graph::types::PinId;
    use egui::Pos2;

    #[test]
    fn test_extract_material_params_default() {
        let graph = NodeGraph::new();
        let params = extract_material_params(&graph);
        // When no MaterialOutput node exists, returns default MaterialParams
        assert_eq!(params.base_color, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(params.metallic, 0.0);
        assert_eq!(params.roughness, 0.0);
    }

    #[test]
    fn test_extract_material_params_with_output() {
        let mut graph = NodeGraph::new();

        let output = create_node(NodeType::MaterialOutput, Pos2::ZERO);
        let _id = graph.add_node(output);

        let params = extract_material_params(&graph);
        assert_eq!(params.metallic, 0.0); // default
        assert_eq!(params.roughness, 0.5); // default from node
    }

    #[test]
    fn test_generate_wgsl_empty() {
        let graph = NodeGraph::new();
        let wgsl = generate_wgsl(&graph);
        assert!(wgsl.contains("struct MaterialParams"));
        assert!(wgsl.contains("fn evaluate_material"));
    }

    #[test]
    fn test_generate_wgsl_simple() {
        let mut graph = NodeGraph::new();

        let mut n1 = create_node(NodeType::ConstantFloat, Pos2::ZERO);
        n1.values.insert(0, NodeValue::Float(0.8));
        let id1 = graph.add_node(n1);

        let output = create_node(NodeType::MaterialOutput, Pos2::ZERO);
        let id_out = graph.add_node(output);

        // Connect float to metallic
        graph
            .connect(PinId::new(id1, 0), PinId::new(id_out, 1))
            .unwrap();

        let wgsl = generate_wgsl(&graph);
        assert!(wgsl.contains("0.8"));
        assert!(wgsl.contains("params.metallic"));
    }
}
