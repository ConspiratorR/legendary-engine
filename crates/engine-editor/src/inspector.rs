//! Property inspector for selected entity.
//!
//! Unity Reference: https://docs.unity3d.com/ScriptReference/Editor.html
//! Uses IMGUI wrapper (engine_ui::imgui) for Unity-style layout.

use crate::material_editor::MaterialEditorState;
use crate::state::{EditorState, LightType};
use egui::{Color32, FontId, Rounding, Stroke};
use engine_ui::Gui;

const SECTION_SPACING: f32 = 8.0;
const ROW_SPACING: f32 = 4.0;

/// Inspector panel — manages property display and editing for selected entities.
#[derive(Debug, Clone)]
pub struct InspectorPanel {
    /// Currently selected node IDs.
    pub selected: Vec<u64>,
    /// Search filter text.
    pub search_text: String,
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl InspectorPanel {
    /// Creates a new inspector panel with empty selection.
    pub fn new() -> Self {
        Self {
            selected: Vec::new(),
            search_text: String::new(),
        }
    }

    /// Get the currently selected node IDs.
    pub fn selected(&self) -> &[u64] {
        &self.selected
    }

    /// Set the selected node IDs.
    pub fn set_selected(&mut self, ids: Vec<u64>) {
        self.selected = ids;
    }

    /// Render the entire inspector panel.
    pub fn render(&mut self, state: &mut EditorState, gui: &mut Gui, rect: egui::Rect) {
        let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
        let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

        // Background
        let painter = gui.ui.painter_at(rect);
        painter.add(egui::Shape::rect_filled(
            rect,
            Rounding::ZERO,
            Color32::from_rgb(22, 22, 25),
        ));
        painter.add(egui::Shape::line(
            vec![
                egui::Pos2::new(rect.left(), rect.top()),
                egui::Pos2::new(rect.left(), rect.bottom()),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
        ));

        // Search bar
        let search_h = 28.0 * h_scale;
        let pad = 8.0 * w_scale;
        let search_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.left() + pad, rect.top() + 6.0 * h_scale),
            egui::Vec2::new(rect.width() - pad * 2.0, search_h),
        );
        self.draw_search_bar(gui, search_rect, h_scale, w_scale);

        // Content area with scroll
        let content_top = search_rect.bottom() + 6.0 * h_scale;
        let content_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.left(), content_top),
            egui::Vec2::new(rect.width(), rect.bottom() - content_top),
        );

        let selected_id = self.selected.first().copied();

        let mut content_ui = gui.ui.new_child(
            egui::UiBuilder::new()
                .max_rect(content_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut content_ui, |ui| {
                let mut gui = Gui::new(ui, gui.skin);
                if let Some(id) = selected_id {
                    self.draw_entity_inspector(&mut gui, state, id, h_scale, w_scale);
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new("未选中对象")
                                .color(Color32::from_gray(90))
                                .size(12.0),
                        );
                    });
                }
            });

        // Terrain panel overlay
        if state.active_tool == crate::state::ToolType::Terrain {
            egui::Area::new(egui::Id::new("terrain_panel"))
                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 50.0))
                .show(gui.ui.ctx(), |ui| {
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(30, 30, 35))
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.set_min_width(250.0);
                            let mut terrain = engine_terrain::components::Terrain::new(
                                128,
                                64,
                                engine_math::Vec2::new(100.0, 100.0),
                                50.0,
                            );
                            let mut texture_layers =
                                engine_terrain::components::TerrainTextureLayers::default();
                            let mut vegetation_data =
                                engine_terrain::components::VegetationData::default();
                            state.terrain_panel.draw(
                                ui,
                                &mut terrain,
                                &mut texture_layers,
                                &mut vegetation_data,
                            );
                        });
                });
        }
    }

    /// Render a specific component section for the given entity.
    pub fn render_component(
        &mut self,
        gui: &mut Gui,
        _state: &mut EditorState,
        id: u64,
        component: &str,
    ) {
        let search_lower = component.to_lowercase();
        self.draw_component_section(gui, _state, id, &search_lower, 1.0, 1.0);
    }

    fn draw_search_bar(&mut self, gui: &mut Gui, rect: egui::Rect, _h_scale: f32, w_scale: f32) {
        let painter = gui.ui.painter_at(rect);
        painter.add(egui::Shape::rect_filled(
            rect,
            Rounding::same(6.0),
            Color32::from_rgb(30, 30, 34),
        ));

        let search_id = egui::Id::new("inspector_search");
        let response = gui.ui.interact(rect, search_id, egui::Sense::click());
        if response.clicked() {
            gui.ui.ctx().memory_mut(|m| m.request_focus(search_id));
        }
        let has_focus = gui.ui.ctx().memory(|m| m.has_focus(search_id));

        if has_focus {
            gui.ui.ctx().input(|i| {
                for event in &i.events {
                    if let egui::Event::Text(text) = event {
                        self.search_text.push_str(text);
                    }
                    if let egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } = event
                    {
                        self.search_text.pop();
                    }
                    if let egui::Event::Key {
                        key: egui::Key::Escape,
                        pressed: true,
                        ..
                    } = event
                    {
                        self.search_text.clear();
                        gui.ui.ctx().memory_mut(|m| m.surrender_focus(search_id));
                    }
                }
            });
        }

        let display_text = if self.search_text.is_empty() && !has_focus {
            "搜索属性...".to_string()
        } else {
            self.search_text.clone()
        };
        let text_color = if has_focus {
            Color32::from_rgb(220, 220, 224)
        } else if self.search_text.is_empty() {
            Color32::from_gray(90)
        } else {
            Color32::from_rgb(0, 212, 170)
        };

        painter.text(
            egui::pos2(rect.left() + 12.0 * w_scale, rect.center().y),
            egui::Align2::LEFT_CENTER,
            display_text,
            egui::FontId::proportional(12.0),
            text_color,
        );

        // Clear button
        if !self.search_text.is_empty() {
            let clear_rect = egui::Rect::from_min_size(
                egui::Pos2::new(rect.right() - 28.0 * w_scale, rect.top()),
                egui::Vec2::new(24.0 * w_scale, rect.height()),
            );
            let clear_id = egui::Id::new("inspector_clear_search");
            let clear_resp = gui.ui.interact(clear_rect, clear_id, egui::Sense::click());
            if clear_resp.hovered() {
                painter.add(egui::Shape::rect_filled(
                    clear_rect,
                    Rounding::same(4.0),
                    Color32::from_rgb(40, 40, 44),
                ));
            }
            painter.text(
                clear_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                FontId::proportional(12.0),
                Color32::from_gray(90),
            );
            if clear_resp.clicked() {
                self.search_text.clear();
            }
        }
    }

    fn section_matches(section_name: &str, search: &str) -> bool {
        search.is_empty() || section_name.to_lowercase().contains(search)
    }

    fn draw_entity_inspector(
        &mut self,
        gui: &mut Gui,
        state: &mut EditorState,
        id: u64,
        _h_scale: f32,
        _w_scale: f32,
    ) {
        let search_lower = self.search_text.to_lowercase();

        // Node name header
        let name = state
            .scene_tree
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "—".into());
        gui.ui.add_space(4.0);
        gui.ui.label(
            egui::RichText::new(format!("{} [{}]", name, id))
                .color(Color32::from_rgb(220, 220, 224))
                .size(13.0),
        );
        gui.ui.add_space(4.0);

        self.draw_component_section(gui, state, id, &search_lower, _h_scale, _w_scale);
    }

    fn draw_component_section(
        &mut self,
        gui: &mut Gui,
        state: &mut EditorState,
        id: u64,
        search_lower: &str,
        _h_scale: f32,
        _w_scale: f32,
    ) {
        // ── Transform ──
        if Self::section_matches(
            "变换 位置 旋转 缩放 transform position rotation scale",
            search_lower,
        ) {
            // Use Unity-style World API to access Transform
            if let Some(handle) = state.GetHandle(id) {
                if let Some(t) = state.world.GetTransformMut(handle) {
                    let old_pos = t.LocalPosition();
                    let old_rot = t.LocalRotation();
                    let old_scale = t.LocalScale();

                    Self::section_separator(gui);
                    Self::section_header(gui, "变换");

                    let (mut px, mut py, mut pz) = (
                        old_pos.x, old_pos.y, old_pos.z,
                    );
                    Self::vec3_row(gui, "位置", &mut px, &mut py, &mut pz);
                    t.SetLocalPosition(engine_math::Vec3::new(px, py, pz));

                    let (mut rx, mut ry, mut rz) = (
                        old_rot.to_euler(engine_math::EulerRot::XYZ).0.to_degrees(),
                        old_rot.to_euler(engine_math::EulerRot::XYZ).1.to_degrees(),
                        old_rot.to_euler(engine_math::EulerRot::XYZ).2.to_degrees(),
                    );
                    Self::vec3_row(gui, "旋转", &mut rx, &mut ry, &mut rz);
                    t.SetLocalRotation(engine_math::Quat::from_euler(
                        engine_math::EulerRot::XYZ,
                        rx.to_radians(),
                        ry.to_radians(),
                        rz.to_radians(),
                    ));

                    let (mut sx, mut sy, mut sz) = (
                        old_scale.x, old_scale.y, old_scale.z,
                    );
                    Self::vec3_row(gui, "缩放", &mut sx, &mut sy, &mut sz);
                    t.SetLocalScale(engine_math::Vec3::new(sx, sy, sz));
                }
            }
        }

        // ── Material (PBR) ──
        if Self::section_matches("材质 pbr material 基础颜色 金属度 粗糙度", search_lower) {
            // Try World API first, fallback to legacy HashMap
            let has_material_in_world = state.GetHandle(id)
                .map(|h| state.world.HasComponent::<engine_core::components::Material>(h))
                .unwrap_or(false);
            let has_material_in_map = state.node_materials.contains_key(&id);

            if has_material_in_world || has_material_in_map {
                Self::section_separator(gui);
                Self::section_header(gui, "材质 (PBR)");

                if let Some(handle) = state.GetHandle(id) {
                    if let Some(mat) = state.world.GetComponentMut::<engine_core::components::Material>(handle) {
                        let (mut r, mut g, mut b) = (
                            mat.base_color[0], mat.base_color[1], mat.base_color[2],
                        );
                        Self::vec3_row(gui, "基础颜色", &mut r, &mut g, &mut b);
                        mat.base_color[0] = r;
                        mat.base_color[1] = g;
                        mat.base_color[2] = b;

                        // Map smoothness to roughness (roughness ≈ 1 - smoothness)
                        let mut roughness = 1.0 - mat.smoothness;
                        Self::slider_row(gui, "金属度", &mut mat.metallic, 0.0, 1.0);
                        Self::slider_row(gui, "粗糙度", &mut roughness, 0.0, 1.0);
                        mat.smoothness = 1.0 - roughness;

                        gui.ui.add_space(ROW_SPACING);
                        if gui.ui.button("编辑材质图").clicked() {
                            // Sync to legacy MaterialData for editor graph
                            let legacy_mat = crate::state::MaterialData {
                                base_color: mat.base_color,
                                metallic: mat.metallic,
                                roughness,
                                ao: 1.0,
                                emissive: mat.emission_color,
                            };
                            let graph = MaterialEditorState::graph_from_material(&legacy_mat);
                            state.node_graph_state.graph = graph;
                            state.material_editor.open();
                            state.material_editor.material_name = format!("材质 #{}", id);
                        }

                        // Sync to legacy HashMap for backward compatibility (viewport)
                        if let Some(legacy_mat) = state.node_materials.get_mut(&id) {
                            legacy_mat.base_color = mat.base_color;
                            legacy_mat.metallic = mat.metallic;
                            legacy_mat.roughness = roughness;
                        }
                    }
                } else if let Some(mat) = state.node_materials.get_mut(&id) {
                    // Fallback: read from legacy HashMap
                    let (mut r, mut g, mut b) = (mat.base_color[0], mat.base_color[1], mat.base_color[2]);
                    Self::vec3_row(gui, "基础颜色", &mut r, &mut g, &mut b);
                    mat.base_color[0] = r;
                    mat.base_color[1] = g;
                    mat.base_color[2] = b;

                    Self::slider_row(gui, "金属度", &mut mat.metallic, 0.0, 1.0);
                    Self::slider_row(gui, "粗糙度", &mut mat.roughness, 0.0, 1.0);
                    Self::slider_row(gui, "环境光遮蔽", &mut mat.ao, 0.0, 1.0);

                    gui.ui.add_space(ROW_SPACING);
                    if gui.ui.button("编辑材质图").clicked() {
                        let graph = MaterialEditorState::graph_from_material(mat);
                        state.node_graph_state.graph = graph;
                        state.material_editor.open();
                        state.material_editor.material_name = format!("材质 #{}", id);
                    }
                }
            }
        }

        // ── Render (MeshRenderer) ──
        if Self::section_matches("渲染 render 材质 网格 阴影 shadow mesh", search_lower) {
            // Try World API first, fallback to legacy HashMap
            let has_mesh_in_world = state.GetHandle(id)
                .map(|h| state.world.HasComponent::<engine_core::components::MeshRenderer>(h))
                .unwrap_or(false);
            let has_mesh_in_map = state.node_render.contains_key(&id);

            if has_mesh_in_world || has_mesh_in_map {
                Self::section_separator(gui);
                Self::section_header(gui, "渲染");

                if let Some(handle) = state.GetHandle(id) {
                    if let Some(renderer) = state.world.GetComponentMut::<engine_core::components::MeshRenderer>(handle) {
                        Self::read_only_row(gui, "材质", &renderer.material);

                        // Mesh combo
                        gui.ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("网格")
                                    .color(Color32::from_gray(152))
                                    .size(12.0),
                            );
                            egui::ComboBox::from_id_salt(egui::Id::new("mesh_combo").with(id))
                                .selected_text(renderer.mesh.as_str())
                                .show_ui(ui, |ui| {
                                    for mt in ["Cube", "Sphere", "Plane", "Cylinder"] {
                                        ui.selectable_value(&mut renderer.mesh, mt.to_string(), mt);
                                    }
                                });
                        });
                        gui.ui.add_space(ROW_SPACING);

                        Self::checkbox_row(gui, "投射阴影", &mut renderer.cast_shadows);
                        Self::checkbox_row(gui, "接收阴影", &mut renderer.receive_shadows);

                        // Sync to legacy HashMap for backward compatibility (viewport)
                        if let Some((mat, mesh, shadow)) = state.node_render.get_mut(&id) {
                            *mat = renderer.material.clone();
                            *mesh = renderer.mesh.clone();
                            *shadow = renderer.cast_shadows;
                        }
                    }
                } else if let Some((mat, mesh, shadow)) = state.node_render.get_mut(&id) {
                    // Fallback: read from legacy HashMap
                    Self::read_only_row(gui, "材质", mat);

                    // Mesh combo
                    gui.ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("网格")
                                .color(Color32::from_gray(152))
                                .size(12.0),
                        );
                        egui::ComboBox::from_id_salt(egui::Id::new("mesh_combo").with(id))
                            .selected_text(mesh.as_str())
                            .show_ui(ui, |ui| {
                                for mt in ["Cube", "Sphere", "Plane", "Cylinder"] {
                                    ui.selectable_value(mesh, mt.to_string(), mt);
                                }
                            });
                    });
                    gui.ui.add_space(ROW_SPACING);

                    Self::checkbox_row(gui, "投射阴影", shadow);
                }
            }
        }

        // ── Light ──
        if Self::section_matches(
            "光照 光 light 方向光 点光源 聚光灯 directional point spot",
            search_lower,
        ) {
            // Try World API first, fallback to legacy HashMap
            let has_light_in_world = state.GetHandle(id)
                .map(|h| state.world.HasComponent::<engine_core::components::Light>(h))
                .unwrap_or(false);
            let has_light_in_map = state.node_lights.contains_key(&id);

            if has_light_in_world || has_light_in_map {
                Self::section_separator(gui);

                if let Some(handle) = state.GetHandle(id) {
                    if let Some(light) = state.world.GetComponentMut::<engine_core::components::Light>(handle) {
                        let type_label = match light.light_type {
                            engine_core::components::LightType::Directional => "光照 (方向光)",
                            engine_core::components::LightType::Point => "光照 (点光源)",
                            engine_core::components::LightType::Spot => "光照 (聚光灯)",
                        };
                        Self::section_header(gui, type_label);

                        let (mut lr, mut lg, mut lb) = (light.color[0], light.color[1], light.color[2]);
                        Self::vec3_row(gui, "颜色", &mut lr, &mut lg, &mut lb);
                        light.color[0] = lr;
                        light.color[1] = lg;
                        light.color[2] = lb;

                        Self::slider_row(gui, "强度", &mut light.intensity, 0.0, 10.0);

                        if light.light_type != engine_core::components::LightType::Directional {
                            Self::slider_row(gui, "范围", &mut light.range, 0.0, 100.0);
                        }

                        if light.light_type != engine_core::components::LightType::Point {
                            Self::slider_row(gui, "内角 (°)", &mut light.inner_angle, 0.0, 90.0);
                            Self::slider_row(gui, "外角 (°)", &mut light.outer_angle, 0.0, 90.0);
                        }

                        Self::checkbox_row(gui, "投射阴影", &mut light.shadows);

                        // Sync to legacy HashMap for backward compatibility (viewport)
                        if let Some(legacy_light) = state.node_lights.get_mut(&id) {
                            legacy_light.color = light.color;
                            legacy_light.intensity = light.intensity;
                            legacy_light.range = light.range;
                            legacy_light.inner_angle = light.inner_angle;
                            legacy_light.outer_angle = light.outer_angle;
                        }
                    }
                } else if let Some(light) = state.node_lights.get_mut(&id) {
                    // Fallback: read from legacy HashMap
                    let type_label = match light.light_type {
                        LightType::Directional => "光照 (方向光)",
                        LightType::Point => "光照 (点光源)",
                        LightType::Spot => "光照 (聚光灯)",
                    };
                    Self::section_header(gui, type_label);

                    Self::checkbox_row(gui, "启用", &mut light.enabled);

                    let (mut lr, mut lg, mut lb) = (light.color[0], light.color[1], light.color[2]);
                    Self::vec3_row(gui, "颜色", &mut lr, &mut lg, &mut lb);
                    light.color[0] = lr;
                    light.color[1] = lg;
                    light.color[2] = lb;

                    Self::slider_row(gui, "强度", &mut light.intensity, 0.0, 10.0);

                    if light.light_type != LightType::Directional {
                        Self::slider_row(gui, "范围", &mut light.range, 0.0, 100.0);
                    }

                    if light.light_type != LightType::Point {
                        let (mut dx, mut dy, mut dz) =
                            (light.direction[0], light.direction[1], light.direction[2]);
                        Self::vec3_row(gui, "方向", &mut dx, &mut dy, &mut dz);
                        light.direction[0] = dx;
                        light.direction[1] = dy;
                        light.direction[2] = dz;
                    }

                    if light.light_type == LightType::Spot {
                        Self::slider_row(gui, "内角 (°)", &mut light.inner_angle, 0.0, 90.0);
                        Self::slider_row(gui, "外角 (°)", &mut light.outer_angle, 0.0, 90.0);
                    }
                }
            }
        }

        // ── Physics ──
        if Self::section_matches("物理 physics 刚体 碰撞 rigidbody collider", search_lower) {
            if let Some(handle) = state.GetHandle(id) {
                if state.world.HasComponent::<engine_core::components::Rigidbody>(handle) {
                    Self::section_separator(gui);
                    Self::section_header(gui, "物理");

                    // Query collider info before mutable borrow
                    let collider_name = if state.world.HasComponent::<engine_core::components::BoxCollider>(handle) {
                        "BoxCollider".to_string()
                    } else if state.world.HasComponent::<engine_core::components::SphereCollider>(handle) {
                        "SphereCollider".to_string()
                    } else if state.world.HasComponent::<engine_core::components::CapsuleCollider>(handle) {
                        "CapsuleCollider".to_string()
                    } else {
                        "无".to_string()
                    };

                    if let Some(rb) = state.world.GetComponentMut::<engine_core::components::Rigidbody>(handle) {
                        Self::slider_row(gui, "质量", &mut rb.mass, 0.01, 100.0);
                        Self::slider_row(gui, "阻力", &mut rb.drag, 0.0, 10.0);
                        Self::slider_row(gui, "角阻力", &mut rb.angular_drag, 0.0, 10.0);
                        Self::checkbox_row(gui, "重力", &mut rb.use_gravity);
                        Self::checkbox_row(gui, "运动学", &mut rb.is_kinematic);

                        Self::read_only_row(gui, "碰撞体", &collider_name);
                    }
                }
            }
        }

        // ── Sprite ──
        if Self::section_matches("精灵 sprite 纹理 翻转", search_lower) {
            if let Some(handle) = state.GetHandle(id) {
                if let Some(sprite) = state.world.GetComponentMut::<engine_core::components::SpriteRenderer>(handle) {
                    Self::section_separator(gui);
                    Self::section_header(gui, "精灵");

                    Self::text_input_row(gui, "纹理", &mut sprite.sprite);

                    let (mut r, mut g, mut b) = (sprite.color[0], sprite.color[1], sprite.color[2]);
                    Self::vec3_row(gui, "颜色", &mut r, &mut g, &mut b);
                    sprite.color[0] = r;
                    sprite.color[1] = g;
                    sprite.color[2] = b;

                    Self::checkbox_row(gui, "水平翻转", &mut sprite.flip_x);
                    Self::checkbox_row(gui, "垂直翻转", &mut sprite.flip_y);
                }
            }
        }

        // ── Particle ──
        if Self::section_matches("粒子 particle 发射器 粒子系统", search_lower) {
            if let Some(handle) = state.GetHandle(id) {
                if let Some(ps) = state.world.GetComponentMut::<engine_core::components::ParticleSystem>(handle) {
                    Self::section_separator(gui);
                    Self::section_header(gui, "粒子系统");

                    Self::slider_row(gui, "发射速率", &mut ps.rate, 0.0, 100.0);
                    Self::slider_row(gui, "生命周期", &mut ps.lifetime, 0.1, 10.0);
                    Self::slider_row(gui, "起始速度", &mut ps.start_speed, 0.0, 50.0);
                    Self::slider_row(gui, "起始大小", &mut ps.start_size, 0.0, 10.0);
                    Self::slider_row(gui, "结束大小", &mut ps.end_size, 0.0, 10.0);
                    Self::slider_row(gui, "重力修改", &mut ps.gravity_modifier, 0.0, 10.0);
                    Self::slider_row(gui, "模拟速度", &mut ps.simulation_speed, 0.0, 10.0);
                }
            }
        }

        // ── Audio ──
        if Self::section_matches("音频 audio 声音 音量", search_lower) {
            if let Some(handle) = state.GetHandle(id) {
                if let Some(audio) = state.world.GetComponentMut::<engine_core::components::AudioSource>(handle) {
                    Self::section_separator(gui);
                    Self::section_header(gui, "音频");

                    Self::text_input_row(gui, "音频源", &mut audio.clip);
                    Self::slider_row(gui, "音量", &mut audio.volume, 0.0, 1.0);
                    Self::slider_row(gui, "音调", &mut audio.pitch, 0.0, 3.0);
                    Self::checkbox_row(gui, "循环", &mut audio.loop_playing);
                    let mut spatial = audio.spatial_blend > 0.5;
                    Self::checkbox_row(gui, "空间音频", &mut spatial);
                    audio.spatial_blend = if spatial { 1.0 } else { 0.0 };
                }
            }
        }

        // ── Script ──
        if Self::section_matches("脚本 script lua wasm", search_lower) {
            if let Some(handle) = state.GetHandle(id) {
                if let Some(script) = state.world.GetComponentMut::<engine_core::components::ScriptBehaviour>(handle) {
                    Self::section_separator(gui);
                    Self::section_header(gui, "脚本");

                    Self::text_input_row(gui, "脚本路径", &mut script.script_path);
                    Self::checkbox_row(gui, "启用", &mut script.enabled);
                }
            }
        }

        // ── Tags ──
        if Self::section_matches("标签 tags tag", search_lower) {
            if let Some(handle) = state.GetHandle(id) {
                if let Some(tag_comp) = state.world.GetComponentMut::<engine_core::components::Tag>(handle) {
                    Self::section_separator(gui);
                    Self::section_header(gui, "标签");

                    let tag_str = tag_comp.tags.join(", ");
                    let mut tag_edit = tag_str.clone();
                    Self::text_input_row(gui, "标签", &mut tag_edit);
                    if tag_edit != tag_str {
                        tag_comp.tags = tag_edit
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                }
            }
        }

        // ── Action buttons ──
        gui.ui.add_space(SECTION_SPACING);
        Self::section_separator(gui);
        gui.ui.add_space(SECTION_SPACING);

        gui.ui.horizontal(|ui| {
            let half_w = (ui.available_width() - 4.0) / 2.0;

            let add_btn = ui.allocate_ui_with_layout(
                egui::Vec2::new(half_w, 28.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(half_w);
                    ui.selectable_label(false, "+ 添加组件")
                },
            );
            if add_btn.inner.clicked() {
                state.show_add_component_menu = !state.show_add_component_menu;
                state.show_remove_component_menu = false;
            }

            let rm_btn = ui.allocate_ui_with_layout(
                egui::Vec2::new(half_w, 28.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(half_w);
                    ui.selectable_label(false, "- 移除组件")
                },
            );
            if rm_btn.inner.clicked() {
                state.show_remove_component_menu = !state.show_remove_component_menu;
                state.show_add_component_menu = false;
            }
        });

        // Add Component dropdown
        if state.show_add_component_menu {
            let component_types = [
                ("材质 (PBR)", "material"),
                ("光照", "light"),
                ("精灵", "sprite"),
                ("粒子系统", "particle"),
                ("音频", "audio"),
                ("脚本", "script"),
                ("物理", "physics"),
                ("标签", "tags"),
            ];
            egui::Frame::default()
                .fill(Color32::from_rgb(35, 35, 40))
                .rounding(Rounding::same(4.0))
                .inner_margin(egui::Margin::same(4.0))
                .show(gui.ui, |ui| {
                    for (label, comp_type) in &component_types {
                        if ui.selectable_label(false, *label).clicked() {
                            Self::add_component_to_node(state, id, comp_type);
                            state.show_add_component_menu = false;
                        }
                    }
                });
        }

        // Remove Component dropdown
        if state.show_remove_component_menu {
            // Check World API for component existence
            let wh = state.GetHandle(id);
            let existing_components: Vec<(&str, bool)> = vec![
                ("材质 (PBR)", wh.map(|h| state.world.HasComponent::<engine_core::components::Material>(h)).unwrap_or(false)),
                ("渲染", wh.map(|h| state.world.HasComponent::<engine_core::components::MeshRenderer>(h)).unwrap_or(false)),
                ("光照", wh.map(|h| state.world.HasComponent::<engine_core::components::Light>(h)).unwrap_or(false)),
                ("物理", wh.map(|h| state.world.HasComponent::<engine_core::components::Rigidbody>(h)
                    || state.world.HasComponent::<engine_core::components::BoxCollider>(h)
                    || state.world.HasComponent::<engine_core::components::SphereCollider>(h)
                    || state.world.HasComponent::<engine_core::components::CapsuleCollider>(h)).unwrap_or(false)),
                ("精灵", wh.map(|h| state.world.HasComponent::<engine_core::components::SpriteRenderer>(h)).unwrap_or(false)),
                ("粒子系统", wh.map(|h| state.world.HasComponent::<engine_core::components::ParticleSystem>(h)).unwrap_or(false)),
                ("音频", wh.map(|h| state.world.HasComponent::<engine_core::components::AudioSource>(h)).unwrap_or(false)),
                ("脚本", wh.map(|h| state.world.HasComponent::<engine_core::components::ScriptBehaviour>(h)).unwrap_or(false)),
                ("标签", wh.map(|h| state.world.HasComponent::<engine_core::components::Tag>(h)).unwrap_or(false)),
            ];
            let comp_types = [
                "material", "render", "light", "physics", "sprite", "particle", "audio", "script",
                "tags",
            ];

            egui::Frame::default()
                .fill(Color32::from_rgb(50, 25, 25))
                .rounding(Rounding::same(4.0))
                .inner_margin(egui::Margin::same(4.0))
                .show(gui.ui, |ui| {
                    for ((label, exists), comp_type) in
                        existing_components.iter().zip(comp_types.iter())
                    {
                        let text = if *exists {
                            format!("× {}", label)
                        } else {
                            format!("  {}", label)
                        };
                        let resp = ui.add_enabled(*exists, egui::SelectableLabel::new(false, text));
                        if *exists && resp.clicked() {
                            Self::remove_component(state, id, comp_type);
                            state.log_info(&format!("已移除 {} 组件", label));
                            state.show_remove_component_menu = false;
                        }
                    }
                });
        }
    }

    // ── Widget helpers ──

    fn section_separator(gui: &mut Gui) {
        gui.ui.add_space(SECTION_SPACING);
        let rect = gui.ui.available_rect_before_wrap();
        let painter = gui.ui.painter_at(rect);
        let y = rect.top();
        painter.add(egui::Shape::line(
            vec![
                egui::Pos2::new(rect.left(), y),
                egui::Pos2::new(rect.right(), y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
        ));
        gui.ui.add_space(SECTION_SPACING);
    }

    fn section_header(gui: &mut Gui, label: &str) {
        gui.ui.label(
            egui::RichText::new(label)
                .color(Color32::from_gray(90))
                .size(11.0),
        );
        gui.ui.add_space(ROW_SPACING);
    }

    fn vec3_row(gui: &mut Gui, label: &str, x: &mut f32, y: &mut f32, z: &mut f32) {
        let row_h = 28.0;
        let row_rect = gui.ui.allocate_ui_with_layout(
            egui::vec2(gui.ui.available_width(), row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |_| {},
        );
        gui.vec3_input(row_rect.response.rect, label, x, y, z);
        gui.ui.add_space(ROW_SPACING);
    }

    fn slider_row(gui: &mut Gui, label: &str, value: &mut f32, min: f32, max: f32) {
        let row_h = 22.0;
        let row_rect = gui.ui.allocate_ui_with_layout(
            egui::vec2(gui.ui.available_width(), row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |_| {},
        );
        gui.slider_f32(row_rect.response.rect, label, value, min, max);
        gui.ui.add_space(ROW_SPACING);
    }

    fn checkbox_row(gui: &mut Gui, label: &str, checked: &mut bool) {
        let row_h = 22.0;
        let row_rect = gui.ui.allocate_ui_with_layout(
            egui::vec2(gui.ui.available_width(), row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |_| {},
        );
        gui.checkbox(row_rect.response.rect, label, checked);
        gui.ui.add_space(ROW_SPACING);
    }

    fn read_only_row(gui: &mut Gui, label: &str, value: &str) {
        let row_h = 22.0;
        let row_rect = gui.ui.allocate_ui_with_layout(
            egui::vec2(gui.ui.available_width(), row_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |_| {},
        );
        gui.input_labeled(row_rect.response.rect, label, value);
        gui.ui.add_space(ROW_SPACING);
    }

    fn text_input_row(gui: &mut Gui, label: &str, value: &mut String) {
        gui.ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(label)
                    .color(Color32::from_gray(152))
                    .size(12.0),
            );
            ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width()));
        });
        gui.ui.add_space(ROW_SPACING);
    }

    fn remove_component(state: &mut EditorState, id: u64, comp_type: &str) {
        if let Some(handle) = state.GetHandle(id) {
            match comp_type {
                "material" => {
                    state.world.RemoveComponent::<engine_core::components::Material>(handle);
                }
                "render" => {
                    state.world.RemoveComponent::<engine_core::components::MeshRenderer>(handle);
                }
                "light" => {
                    state.world.RemoveComponent::<engine_core::components::Light>(handle);
                }
                "physics" => {
                    state.world.RemoveComponent::<engine_core::components::Rigidbody>(handle);
                    state.world.RemoveComponent::<engine_core::components::BoxCollider>(handle);
                    state.world.RemoveComponent::<engine_core::components::SphereCollider>(handle);
                    state.world.RemoveComponent::<engine_core::components::CapsuleCollider>(handle);
                }
                "sprite" => {
                    state.world.RemoveComponent::<engine_core::components::SpriteRenderer>(handle);
                }
                "particle" => {
                    state.world.RemoveComponent::<engine_core::components::ParticleSystem>(handle);
                }
                "audio" => {
                    state.world.RemoveComponent::<engine_core::components::AudioSource>(handle);
                }
                "script" => {
                    state.world.RemoveComponent::<engine_core::components::ScriptBehaviour>(handle);
                }
                "tags" => {
                    state.world.RemoveComponent::<engine_core::components::Tag>(handle);
                }
                _ => {}
            }
        }
    }

    fn add_component_to_node(state: &mut EditorState, node_id: u64, comp_type: &str) {
        // Add to World API if handle exists
        if let Some(handle) = state.GetHandle(node_id) {
            match comp_type {
                "material" => {
                    if !state.world.HasComponent::<engine_core::components::Material>(handle) {
                        state.world.AddComponent(handle, engine_core::components::Material::default());
                    }
                }
                "render" => {
                    if !state.world.HasComponent::<engine_core::components::MeshRenderer>(handle) {
                        state.world.AddComponent(handle, engine_core::components::MeshRenderer::default());
                    }
                }
                "light" => {
                    if !state.world.HasComponent::<engine_core::components::Light>(handle) {
                        state.world.AddComponent(handle, engine_core::components::Light::default());
                    }
                }
                "physics" => {
                    if !state.world.HasComponent::<engine_core::components::Rigidbody>(handle) {
                        state.world.AddComponent(handle, engine_core::components::Rigidbody::default());
                    }
                    if !state.world.HasComponent::<engine_core::components::BoxCollider>(handle) {
                        state.world.AddComponent(handle, engine_core::components::BoxCollider::default());
                    }
                }
                "sprite" => {
                    if !state.world.HasComponent::<engine_core::components::SpriteRenderer>(handle) {
                        state.world.AddComponent(handle, engine_core::components::SpriteRenderer::default());
                    }
                }
                "particle" => {
                    if !state.world.HasComponent::<engine_core::components::ParticleSystem>(handle) {
                        state.world.AddComponent(handle, engine_core::components::ParticleSystem::default());
                    }
                }
                "audio" => {
                    if !state.world.HasComponent::<engine_core::components::AudioSource>(handle) {
                        state.world.AddComponent(handle, engine_core::components::AudioSource::default());
                    }
                }
                "script" => {
                    if !state.world.HasComponent::<engine_core::components::ScriptBehaviour>(handle) {
                        state.world.AddComponent(handle, engine_core::components::ScriptBehaviour::default());
                    }
                }
                "tags" => {
                    if !state.world.HasComponent::<engine_core::components::Tag>(handle) {
                        state.world.AddComponent(handle, engine_core::components::Tag::default());
                    }
                }
                _ => {}
            }
        }
    }
}

/// Backward-compatible draw function using the panel.
pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: egui::Rect) {
    let mut panel = InspectorPanel::new();
    panel.selected = state.selected_nodes.clone();
    panel.search_text = state.inspector_search.clone();
    panel.render(state, gui, rect);
    state.inspector_search = panel.search_text;
}
