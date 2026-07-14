//! Visual material authoring with node-graph backend.
//!
//! TODO: Migrate from direct egui to IMGUI wrapper (engine_ui::imgui)
//! Unity Reference: https://docs.unity3d.com/ScriptReference/ShaderGraph.html

pub mod material_library;

use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

use crate::node_graph::export::{MaterialParams, extract_material_params};
use crate::node_graph::types::{NodeValue, PinType};
use crate::node_graph::{MaterialPreview, NodeGraph, NodeGraphRenderer, NodePanel, NodeType};
use material_library::MaterialLibrary;

/// State for the material editor panel.
#[derive(Debug, Clone)]
pub struct MaterialEditorState {
    pub visible: bool,
    pub renderer: NodeGraphRenderer,
    pub node_panel: NodePanel,
    pub preview: MaterialPreview,
    pub library: MaterialLibrary,
    pub material_name: String,
    pub show_properties: bool,
    pub selected_node_values: std::collections::HashMap<usize, NodeValue>,
}

impl Default for MaterialEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            renderer: NodeGraphRenderer::new(),
            node_panel: NodePanel::new(),
            preview: MaterialPreview::new(),
            library: MaterialLibrary::new(),
            material_name: "Untitled Material".into(),
            show_properties: true,
            selected_node_values: std::collections::HashMap::new(),
        }
    }
}

impl MaterialEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the material editor. Caller should set `editor_state.node_graph_state.graph` before calling.
    pub fn open(&mut self) {
        self.visible = true;
        self.renderer = NodeGraphRenderer::new();
        self.renderer.visible = true;
        self.preview.mark_dirty();
    }

    /// Load a preset from the library into the graph.
    pub fn load_preset(&mut self, params: &MaterialParams, graph: &mut NodeGraph) {
        graph.nodes.clear();
        graph.connections.clear();

        // Create a MaterialOutput node
        let output = crate::node_graph::nodes::create_node(
            NodeType::MaterialOutput,
            Pos2::new(400.0, 200.0),
        );
        let _out_id = graph.add_node(output);

        // Create constant nodes for each parameter and connect them
        let base_color =
            crate::node_graph::nodes::create_node(NodeType::ConstantColor, Pos2::new(50.0, 50.0));
        let mut bc_node = base_color;
        bc_node.values.insert(0, NodeValue::Vec4(params.base_color));
        bc_node.name = "Base Color".into();
        let bc_id = graph.add_node(bc_node);

        let metallic =
            crate::node_graph::nodes::create_node(NodeType::ConstantFloat, Pos2::new(50.0, 180.0));
        let mut met_node = metallic;
        met_node.values.insert(0, NodeValue::Float(params.metallic));
        met_node.name = "Metallic".into();
        let met_id = graph.add_node(met_node);

        let roughness =
            crate::node_graph::nodes::create_node(NodeType::ConstantFloat, Pos2::new(50.0, 280.0));
        let mut rough_node = roughness;
        rough_node
            .values
            .insert(0, NodeValue::Float(params.roughness));
        rough_node.name = "Roughness".into();
        let rough_id = graph.add_node(rough_node);

        let emissive =
            crate::node_graph::nodes::create_node(NodeType::ConstantVec3, Pos2::new(50.0, 380.0));
        let mut em_node = emissive;
        em_node.values.insert(
            0,
            NodeValue::Vec4([
                params.emissive[0],
                params.emissive[1],
                params.emissive[2],
                0.0,
            ]),
        );
        em_node.name = "Emissive".into();
        let em_id = graph.add_node(em_node);

        let ao =
            crate::node_graph::nodes::create_node(NodeType::ConstantFloat, Pos2::new(50.0, 480.0));
        let mut ao_node = ao;
        ao_node.values.insert(0, NodeValue::Float(params.ao));
        ao_node.name = "AO".into();
        let ao_id = graph.add_node(ao_node);

        // Find the output node id (it was the last added)
        let out_id = graph
            .nodes
            .values()
            .find(|n| n.node_type == NodeType::MaterialOutput)
            .map(|n| n.id);

        if let Some(oid) = out_id {
            // Connect: Base Color -> Base Color input (index 0)
            let _ = graph.connect(
                crate::node_graph::types::PinId::new(bc_id, 0),
                crate::node_graph::types::PinId::new(oid, 0),
            );
            // Metallic -> Metallic input (index 1)
            let _ = graph.connect(
                crate::node_graph::types::PinId::new(met_id, 0),
                crate::node_graph::types::PinId::new(oid, 1),
            );
            // Roughness -> Roughness input (index 2)
            let _ = graph.connect(
                crate::node_graph::types::PinId::new(rough_id, 0),
                crate::node_graph::types::PinId::new(oid, 2),
            );
            // Emissive -> Emissive input (index 4)
            let _ = graph.connect(
                crate::node_graph::types::PinId::new(em_id, 0),
                crate::node_graph::types::PinId::new(oid, 4),
            );
            // AO -> AO input (index 5)
            let _ = graph.connect(
                crate::node_graph::types::PinId::new(ao_id, 0),
                crate::node_graph::types::PinId::new(oid, 5),
            );
        }

        self.renderer.auto_layout(graph);
        self.preview.mark_dirty();
    }

    /// Apply the current graph to a material data entry.
    pub fn apply_to_material(graph: &NodeGraph, material: &mut crate::state::MaterialData) {
        let params = extract_material_params(graph);
        material.base_color = params.base_color;
        material.metallic = params.metallic;
        material.roughness = params.roughness;
        material.ao = params.ao;
        material.emissive = params.emissive;
    }

    /// Build a graph from existing material data.
    pub fn graph_from_material(material: &crate::state::MaterialData) -> NodeGraph {
        let params = MaterialParams {
            base_color: material.base_color,
            metallic: material.metallic,
            roughness: material.roughness,
            ao: material.ao,
            emissive: material.emissive,
            ..Default::default()
        };
        let mut graph = NodeGraph::new();
        let mut temp_editor = MaterialEditorState::new();
        temp_editor.load_preset(&params, &mut graph);
        graph
    }
}

/// Draw the material editor panel.
pub fn draw_material_editor(state: &mut crate::state::EditorState, ui: &mut egui::Ui, rect: Rect) {
    if !state.material_editor.visible {
        return;
    }

    let h_scale = ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = ui.ctx().screen_rect().width() / 1920.0;

    let painter = ui.painter_at(rect);

    // Panel background
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    // Top border
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.top()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let toolbar_h = 32.0 * h_scale;
    let node_panel_w = 200.0 * w_scale;
    let preview_h = 160.0 * h_scale;
    let props_w = if state.material_editor.show_properties {
        220.0 * w_scale
    } else {
        0.0
    };

    // Toolbar
    let toolbar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), toolbar_h));
    draw_material_toolbar(state, ui, toolbar_rect, w_scale, h_scale);

    let content_top = rect.top() + toolbar_h;
    let content_h = rect.height() - toolbar_h;

    // Layout: [NodePanel | GraphCanvas | Properties]
    // Bottom strip: [Preview]

    let graph_h = content_h - preview_h;

    let node_panel_rect = Rect::from_min_size(
        Pos2::new(rect.left(), content_top),
        Vec2::new(node_panel_w, graph_h),
    );

    let graph_rect = Rect::from_min_size(
        Pos2::new(rect.left() + node_panel_w, content_top),
        Vec2::new(rect.width() - node_panel_w - props_w, graph_h),
    );

    let preview_rect = Rect::from_min_size(
        Pos2::new(rect.left() + node_panel_w, content_top + graph_h),
        Vec2::new(rect.width() - node_panel_w - props_w, preview_h),
    );

    let props_rect = if state.material_editor.show_properties {
        Some(Rect::from_min_size(
            Pos2::new(rect.right() - props_w, content_top),
            Vec2::new(props_w, content_h),
        ))
    } else {
        None
    };

    // Draw node panel (left)
    state.material_editor.node_panel.draw(ui, node_panel_rect);

    // Handle node drag from panel to graph
    if let Some(node_type) = state.material_editor.node_panel.take_dragging() {
        // Add node at center of graph view
        let center_screen = graph_rect.center();
        let r = &state.material_editor.renderer;
        let center = Pos2::new(
            (center_screen.x - r.pan_offset.x) / r.zoom,
            (center_screen.y - r.pan_offset.y) / r.zoom,
        );
        let node = crate::node_graph::nodes::create_node(node_type, center);
        state.node_graph_state.graph.add_node(node);
        state.material_editor.preview.mark_dirty();
    }

    // Draw graph canvas (center)
    state
        .material_editor
        .renderer
        .draw(ui, graph_rect, &mut state.node_graph_state.graph);

    // Draw preview (bottom center)
    state
        .material_editor
        .preview
        .update(&state.node_graph_state.graph);
    state.material_editor.preview.draw(ui, preview_rect);

    // Draw properties panel (right)
    if let Some(props_r) = props_rect {
        draw_properties_panel(state, ui, props_r);
    }
}

fn draw_material_toolbar(
    state: &mut crate::state::EditorState,
    ui: &mut egui::Ui,
    rect: Rect,
    w_scale: f32,
    h_scale: f32,
) {
    let painter = ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(30, 30, 34),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.bottom() - 1.0),
            Pos2::new(rect.right(), rect.bottom() - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let btn_size = 24.0 * h_scale;
    let gap = 4.0 * w_scale;
    let pad = 8.0 * w_scale;
    let mut x = rect.left() + pad;
    let cy = rect.top() + (rect.height() - btn_size) / 2.0;

    // Material name
    painter.text(
        Pos2::new(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &state.material_editor.material_name,
        egui::FontId::proportional(12.0 * h_scale),
        Color32::from_rgb(200, 200, 200),
    );
    x += 140.0 * w_scale;

    // Separator
    painter.add(Shape::line(
        vec![
            Pos2::new(x, rect.top() + 4.0),
            Pos2::new(x, rect.bottom() - 4.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    x += pad;

    // Auto-layout button
    let layout_rect = Rect::from_min_size(Pos2::new(x, cy), Vec2::new(60.0 * w_scale, btn_size));
    let resp = ui.interact(
        layout_rect,
        egui::Id::new("mat_auto_layout"),
        egui::Sense::click(),
    );
    painter.add(Shape::rect_filled(
        layout_rect,
        Rounding::same(4.0),
        if resp.hovered() {
            Color32::from_rgb(40, 40, 48)
        } else {
            Color32::from_rgb(30, 30, 34)
        },
    ));
    painter.text(
        layout_rect.center(),
        egui::Align2::CENTER_CENTER,
        "布局",
        egui::FontId::proportional(10.0 * h_scale),
        Color32::from_gray(180),
    );
    if resp.clicked() {
        state
            .material_editor
            .renderer
            .auto_layout(&mut state.node_graph_state.graph);
    }
    x += 60.0 * w_scale + gap;

    // Properties toggle
    let props_rect = Rect::from_min_size(Pos2::new(x, cy), Vec2::new(60.0 * w_scale, btn_size));
    let resp = ui.interact(
        props_rect,
        egui::Id::new("mat_props_toggle"),
        egui::Sense::click(),
    );
    let props_on = state.material_editor.show_properties;
    painter.add(Shape::rect_filled(
        props_rect,
        Rounding::same(4.0),
        if props_on {
            Color32::from_rgb(0, 80, 140)
        } else if resp.hovered() {
            Color32::from_rgb(40, 40, 48)
        } else {
            Color32::from_rgb(30, 30, 34)
        },
    ));
    painter.text(
        props_rect.center(),
        egui::Align2::CENTER_CENTER,
        "属性",
        egui::FontId::proportional(10.0 * h_scale),
        if props_on {
            Color32::WHITE
        } else {
            Color32::from_gray(140)
        },
    );
    if resp.clicked() {
        state.material_editor.show_properties = !state.material_editor.show_properties;
    }
    x += 60.0 * w_scale + gap;

    // Library toggle
    let lib_rect = Rect::from_min_size(Pos2::new(x, cy), Vec2::new(60.0 * w_scale, btn_size));
    let resp = ui.interact(
        lib_rect,
        egui::Id::new("mat_lib_toggle"),
        egui::Sense::click(),
    );
    let lib_on = state.material_editor.library.visible;
    painter.add(Shape::rect_filled(
        lib_rect,
        Rounding::same(4.0),
        if lib_on {
            Color32::from_rgb(0, 80, 140)
        } else if resp.hovered() {
            Color32::from_rgb(40, 40, 48)
        } else {
            Color32::from_rgb(30, 30, 34)
        },
    ));
    painter.text(
        lib_rect.center(),
        egui::Align2::CENTER_CENTER,
        "材质库",
        egui::FontId::proportional(10.0 * h_scale),
        if lib_on {
            Color32::WHITE
        } else {
            Color32::from_gray(140)
        },
    );
    if resp.clicked() {
        state.material_editor.library.visible = !state.material_editor.library.visible;
    }
    x += 60.0 * w_scale + gap;

    // Separator
    painter.add(Shape::line(
        vec![
            Pos2::new(x, rect.top() + 4.0),
            Pos2::new(x, rect.bottom() - 4.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    x += pad;

    // Apply to selected object button
    let apply_rect = Rect::from_min_size(Pos2::new(x, cy), Vec2::new(80.0 * w_scale, btn_size));
    let resp = ui.interact(
        apply_rect,
        egui::Id::new("mat_apply_btn"),
        egui::Sense::click(),
    );
    painter.add(Shape::rect_filled(
        apply_rect,
        Rounding::same(4.0),
        if resp.hovered() {
            Color32::from_rgb(0, 120, 80)
        } else {
            Color32::from_rgb(0, 90, 60)
        },
    ));
    painter.text(
        apply_rect.center(),
        egui::Align2::CENTER_CENTER,
        "应用",
        egui::FontId::proportional(10.0 * h_scale),
        Color32::WHITE,
    );
    if resp.clicked() {
        // Apply graph material to selected node's material data
        if let Some(&sel_id) = state.selected_nodes.first()
            && let Some(mat) = state.node_materials.get_mut(&sel_id)
        {
            MaterialEditorState::apply_to_material(&state.node_graph_state.graph, mat);
            state.status_message = Some("材质已应用到选中对象".into());
        }
    }

    // Close button
    let close_rect = Rect::from_min_size(
        Pos2::new(rect.right() - btn_size - pad, cy),
        Vec2::new(btn_size, btn_size),
    );
    let resp = ui.interact(
        close_rect,
        egui::Id::new("mat_close_btn"),
        egui::Sense::click(),
    );
    painter.text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "✕",
        egui::FontId::proportional(12.0 * h_scale),
        Color32::from_gray(140),
    );
    if resp.clicked() {
        state.material_editor.visible = false;
    }
}

/// Draw the properties panel for the selected node.
fn draw_properties_panel(state: &mut crate::state::EditorState, ui: &mut egui::Ui, rect: Rect) {
    let h_scale = ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = ui.ctx().screen_rect().width() / 1920.0;
    let pad = 8.0 * w_scale;

    let painter = ui.painter_at(rect);

    // Background
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    // Left border
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.left(), rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    // Header
    let header_h = 28.0 * h_scale;
    let header_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), header_h));
    painter.add(Shape::rect_filled(
        header_rect,
        Rounding::ZERO,
        Color32::from_rgb(30, 30, 34),
    ));
    painter.text(
        Pos2::new(rect.left() + pad, header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "节点属性",
        egui::FontId::proportional(11.0 * h_scale),
        Color32::from_gray(160),
    );

    let content_top = rect.top() + header_h + pad;
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left() + pad, content_top),
        Vec2::new(rect.width() - pad * 2.0, rect.bottom() - content_top - pad),
    );

    // Find selected node
    let selected_id = state.material_editor.renderer.selected_node;
    let graph = &state.node_graph_state.graph;

    let Some(node_id) = selected_id else {
        painter.text(
            content_rect.center(),
            egui::Align2::CENTER_CENTER,
            "未选中节点",
            egui::FontId::proportional(11.0),
            Color32::from_gray(80),
        );
        return;
    };

    let Some(node) = graph.nodes.get(&node_id) else {
        return;
    };

    let mut y = content_rect.top();
    let row_h = 24.0 * h_scale;

    // Node name
    painter.text(
        Pos2::new(content_rect.left(), y),
        egui::Align2::LEFT_CENTER,
        &node.name,
        egui::FontId::proportional(12.0 * h_scale),
        Color32::WHITE,
    );
    y += row_h + 4.0;

    // Node type
    let type_name = node.node_type.display_name();
    painter.text(
        Pos2::new(content_rect.left(), y),
        egui::Align2::LEFT_CENTER,
        type_name,
        egui::FontId::proportional(10.0 * h_scale),
        Color32::from_gray(120),
    );
    y += row_h + 4.0;

    // Category
    let category = node.node_type.category();
    let cat_name = category.display_name();
    painter.text(
        Pos2::new(content_rect.left(), y),
        egui::Align2::LEFT_CENTER,
        cat_name,
        egui::FontId::proportional(10.0 * h_scale),
        Color32::from_gray(100),
    );
    y += row_h + 8.0;

    // Separator
    painter.add(Shape::line(
        vec![
            Pos2::new(content_rect.left(), y),
            Pos2::new(content_rect.right(), y),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));
    y += 8.0;

    // Editable values
    let graph = &mut state.node_graph_state.graph;
    if let Some(node) = graph.nodes.get_mut(&node_id) {
        for (idx, pin) in node.inputs.iter().enumerate() {
            if y + row_h > content_rect.bottom() {
                break;
            }

            let value = node
                .values
                .entry(idx)
                .or_insert_with(|| pin.default_value.clone());

            match pin.pin_type {
                PinType::Float => {
                    let label = &pin.name;

                    // Draw label
                    painter.text(
                        Pos2::new(content_rect.left(), y + row_h * 0.3),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(10.0 * h_scale),
                        Color32::from_gray(140),
                    );

                    // Draw slider
                    let slider_y = y + row_h * 0.5;
                    let slider_rect = Rect::from_min_size(
                        Pos2::new(content_rect.left(), slider_y),
                        Vec2::new(content_rect.width(), row_h * 0.5),
                    );
                    let _id = egui::Id::new("node_val").with(node_id).with(idx);
                    let mut slider_val = value.to_float();
                    let resp = ui.put(
                        slider_rect,
                        egui::Slider::new(&mut slider_val, 0.0..=1.0)
                            .show_value(true)
                            .fixed_decimals(2),
                    );
                    if resp.changed() {
                        *value = NodeValue::Float(slider_val);
                        state.material_editor.preview.mark_dirty();
                    }

                    y += row_h + 2.0;
                }
                PinType::Vec2 | PinType::Vec3 | PinType::Vec4 | PinType::Color => {
                    let v = value.to_vec4();
                    let label = &pin.name;

                    painter.text(
                        Pos2::new(content_rect.left(), y + row_h * 0.3),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(10.0 * h_scale),
                        Color32::from_gray(140),
                    );

                    let components = match pin.pin_type {
                        PinType::Vec2 => 2,
                        PinType::Vec3 => 3,
                        _ => 4,
                    };

                    let component_labels = ["X", "Y", "Z", "W"];
                    let slider_y = y + row_h * 0.5;
                    let comp_w = content_rect.width() / components as f32;

                    for ci in 0..components {
                        let comp_rect = Rect::from_min_size(
                            Pos2::new(content_rect.left() + ci as f32 * comp_w, slider_y),
                            Vec2::new(comp_w, row_h * 0.5),
                        );
                        let mut val = v[ci];
                        let _id = egui::Id::new("node_vec_val")
                            .with(node_id)
                            .with(idx)
                            .with(ci);
                        let resp = ui.put(
                            comp_rect,
                            egui::DragValue::new(&mut val)
                                .speed(0.01)
                                .prefix(format!("{}:", component_labels[ci])),
                        );
                        if resp.changed() {
                            let mut new_v = v;
                            new_v[ci] = val;
                            *value = NodeValue::Vec4(new_v);
                            state.material_editor.preview.mark_dirty();
                        }
                    }

                    y += row_h + 2.0;
                }
                PinType::Bool => {
                    let val = value.to_float() > 0.5;
                    let mut bool_val = val;
                    let checkbox_rect = Rect::from_min_size(
                        Pos2::new(content_rect.left(), y),
                        Vec2::new(content_rect.width(), row_h),
                    );
                    let resp = ui.put(checkbox_rect, egui::Checkbox::new(&mut bool_val, &pin.name));
                    if resp.changed() {
                        *value = NodeValue::Float(if bool_val { 1.0 } else { 0.0 });
                        state.material_editor.preview.mark_dirty();
                    }
                    y += row_h + 2.0;
                }
                PinType::Int => {
                    let mut val = value.to_float() as i32;
                    let label_rect = Rect::from_min_size(
                        Pos2::new(content_rect.left(), y),
                        Vec2::new(content_rect.width(), row_h),
                    );
                    let resp = ui.put(
                        label_rect,
                        egui::DragValue::new(&mut val)
                            .speed(1)
                            .prefix(format!("{}: ", pin.name)),
                    );
                    if resp.changed() {
                        *value = NodeValue::Float(val as f32);
                        state.material_editor.preview.mark_dirty();
                    }
                    y += row_h + 2.0;
                }
                _ => {
                    // Non-editable pin type
                    y += row_h + 2.0;
                }
            }
        }
    }

    // Library panel overlay (if visible)
    if state.material_editor.library.visible {
        let lib_w = 200.0 * w_scale;
        let lib_rect = Rect::from_min_size(
            Pos2::new(rect.right() - lib_w, rect.top() + header_h),
            Vec2::new(lib_w, rect.height() - header_h),
        );
        state.material_editor.library.draw(ui, lib_rect);

        // Handle preset selection
        if let Some(params) = state.material_editor.library.selected_params().cloned() {
            // Double-click to apply preset
            let _ = params; // Preset is available for application via load_preset
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_editor_state_default() {
        let state = MaterialEditorState::new();
        assert!(!state.visible);
        assert_eq!(state.material_name, "Untitled Material");
        assert!(state.show_properties);
    }

    #[test]
    fn test_graph_from_material() {
        let mat = crate::state::MaterialData {
            base_color: [0.5, 0.3, 0.1, 1.0],
            metallic: 0.7,
            roughness: 0.3,
            ao: 0.9,
            emissive: [0.1, 0.0, 0.0],
        };
        let graph = MaterialEditorState::graph_from_material(&mat);
        assert!(!graph.nodes.is_empty());
    }

    #[test]
    fn test_apply_to_material() {
        let mut graph = NodeGraph::new();
        let output = crate::node_graph::nodes::create_node(NodeType::MaterialOutput, Pos2::ZERO);
        graph.add_node(output);

        let mut mat = crate::state::MaterialData::default();
        MaterialEditorState::apply_to_material(&graph, &mut mat);
        // Default values should be applied
        assert_eq!(mat.metallic, 0.0);
    }
}
