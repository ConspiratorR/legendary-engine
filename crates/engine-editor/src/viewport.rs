//! 3D viewport with tabs (Scene/Game/Physics), toolbar, camera controls.
//!
//! Unity Reference: https://docs.unity3d.com/ScriptReference/SceneView.html
//!
//! # IMGUI Architecture
//!
//! This module follows Unity's IMGUI (Immediate Mode GUI) pattern:
//! - Layout is computed every frame from current state
//! - Widgets are drawn immediately and return interaction results
//! - No retained widget tree — the UI is rebuilt each frame
//!
//! Key IMGUI patterns used here:
//! - `gui.tab()` — tab selector (Unity: GUILayout.Toolbar)
//! - `gui.tool_button()` — toggle button (Unity: GUILayout.Toolbar)
//! - `gui.ui.painter_at(rect)` — low-level drawing (Unity: Handles.DrawRectangle)
//! - `gui.ui.interact(rect, id, Sense::click())` — input capture (Unity: Event.current.mousePosition)
//!
//! The `Gui` wrapper from `engine_ui` provides skinned IMGUI widgets.
//! Direct `painter_at`/`interact` calls handle viewport-specific rendering
//! that goes beyond standard widget layouts.

use crate::state::EditorState;
use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_math::{Mat4, Vec3};
use engine_ui::Gui;

// --- IMGUI Helper: Orthographic projection matrix ---
// Equivalent to Unity's Matrix4x4.Ortho for 2D/orthographic viewport rendering.
fn ortho_projection(aspect: f32) -> Mat4 {
    let size = 10.0;
    Mat4::orthographic_rh(-size * aspect, size * aspect, -size, size, -1000.0, 1000.0)
}

fn view_matrix_for(target: [f32; 3], up: [f32; 3], position: [f32; 3]) -> Mat4 {
    Mat4::look_at_rh(
        Vec3::from_array(position),
        Vec3::from_array(target),
        Vec3::from_array(up),
    )
}

// --- IMGUI: Viewport Header ---
// Draws the tab bar (Scene/Game/Physics) and toolbar buttons.
// This is pure IMGUI: layout is computed from state, widgets are drawn immediately.
// Equivalent to Unity's OnGUI() method in a custom EditorWindow.
fn draw_viewport_header(
    state: &mut EditorState,
    gui: &mut Gui,
    rect: Rect,
    header_h: f32,
    w_scale: f32,
    h_scale: f32,
) {
    let painter = gui.ui.painter_at(Rect::from_min_size(
        rect.left_top(),
        Vec2::new(rect.width(), header_h),
    ));
    painter.add(Shape::rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), header_h)),
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), header_h - 1.0),
            Pos2::new(rect.right(), header_h - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let char_w = 8.0 * w_scale;
    let tab_pad = 12.0 * w_scale;
    let tab_font = 12.0 * h_scale;
    let tab_gap = 16.0 * w_scale;
    // IMGUI Tab Bar: iterate tabs, compute rect, check interaction
    // Unity equivalent: GUILayout.Toolbar with GUILayoutOption width
    let mut tx = rect.left() + 12.0 * w_scale;
    let tabs = &["场景", "游戏", "物理"];
    for (i, label) in tabs.iter().enumerate() {
        let text_w = label.len() as f32 * char_w;
        let tab_rect = Rect::from_min_size(
            Pos2::new(tx, rect.top()),
            Vec2::new(text_w + tab_pad * 2.0, header_h),
        );
        let id = egui::Id::new("vp_tab").with(i as u64);
        let response = gui.ui.interact(tab_rect, id, egui::Sense::click());
        if state.active_viewport_tab == i {
            let line_rect = Rect::from_min_size(
                Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0 * h_scale),
                Vec2::new(tab_rect.width(), 2.0 * h_scale),
            );
            painter.add(Shape::rect_filled(
                line_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 212, 170),
            ));
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_rgb(0, 212, 170),
            );
        } else {
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_gray(90),
            );
        }
        if response.clicked() {
            state.active_viewport_tab = i;
        }
        tx += text_w + tab_pad * 2.0 + tab_gap;
    }

    // IMGUI Tool Buttons: right-aligned toolbar with toggle state
    // Unity equivalent: GUILayout.Toolbar with active state management
    let tool_btn = 24.0 * h_scale;
    let tool_gap = 4.0 * w_scale;
    let tool_font = 12.0 * h_scale;
    let tool_icons = &["📐", "#", "⌖", "🐛"];
    let tool_active = [false, state.show_grid, false, state.show_debug_overlay];
    let rounding = 4.0 * h_scale;
    let mut tool_x =
        rect.right() - 12.0 * w_scale - tool_icons.len() as f32 * (tool_btn + tool_gap);
    for (i, icon) in tool_icons.iter().enumerate() {
        let tool_rect = Rect::from_min_size(
            Pos2::new(tool_x, rect.top() + (header_h - tool_btn) / 2.0),
            Vec2::new(tool_btn, tool_btn),
        );
        let id = egui::Id::new("vp_tool").with(tool_x as u64);
        let response = gui.ui.interact(tool_rect, id, egui::Sense::click());
        let bg_color = if tool_active[i] {
            Color32::from_rgb(0, 80, 60)
        } else if response.hovered() {
            Color32::from_rgb(30, 30, 34)
        } else {
            Color32::TRANSPARENT
        };
        if bg_color != Color32::TRANSPARENT {
            painter.add(Shape::rect_filled(
                tool_rect,
                Rounding::same(rounding),
                bg_color,
            ));
        }
        let text_color = if tool_active[i] {
            Color32::from_rgb(0, 212, 170)
        } else {
            Color32::from_gray(90)
        };
        painter.text(
            tool_rect.center(),
            egui::Align2::CENTER_CENTER,
            *icon,
            FontId::proportional(tool_font),
            text_color,
        );
        if response.clicked() {
            match i {
                1 => state.show_grid = !state.show_grid,
                3 => state.show_debug_overlay = !state.show_debug_overlay,
                _ => {}
            }
        }
        tool_x += tool_btn + tool_gap;
    }
}

// --- IMGUI: Main Viewport Entry Point ---
// Called every frame to draw the 3D viewport panel.
// Handles layout splitting (Single/Horizontal/Vertical/Quad) and delegates
// to draw_single_viewport for each viewport region.
pub fn draw(
    state: &mut EditorState,
    gui: &mut Gui,
    rect: Rect,
    renderer: &mut engine_render::renderer::Renderer,
    vp_renderer: &mut crate::viewport_renderer::ViewportRenderer,
    egui_state: &mut engine_ui::EguiState,
) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    let header_h = 32.0 * h_scale;
    draw_viewport_header(state, gui, rect, header_h, w_scale, h_scale);

    let canvas_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + header_h),
        Vec2::new(rect.width(), rect.height() - header_h),
    );

    use crate::viewport_renderer::ViewportLayout;
    use crate::viewport_renderer::ViewportType;
    match state.viewport_layout {
        ViewportLayout::Single(vt) => {
            draw_single_viewport(
                state,
                gui,
                canvas_rect,
                vt,
                h_scale,
                w_scale,
                renderer,
                vp_renderer,
                egui_state,
            );
        }
        ViewportLayout::Horizontal(a, b) => {
            let half_w = canvas_rect.width() / 2.0;
            let left_rect = Rect::from_min_size(
                canvas_rect.left_top(),
                Vec2::new(half_w, canvas_rect.height()),
            );
            let right_rect = Rect::from_min_size(
                Pos2::new(canvas_rect.left() + half_w, canvas_rect.top()),
                Vec2::new(half_w, canvas_rect.height()),
            );
            draw_single_viewport(
                state,
                gui,
                left_rect,
                a,
                h_scale,
                w_scale,
                renderer,
                vp_renderer,
                egui_state,
            );
            draw_single_viewport(
                state,
                gui,
                right_rect,
                b,
                h_scale,
                w_scale,
                renderer,
                vp_renderer,
                egui_state,
            );
        }
        ViewportLayout::Vertical(a, b) => {
            let half_h = canvas_rect.height() / 2.0;
            let top_rect = Rect::from_min_size(
                canvas_rect.left_top(),
                Vec2::new(canvas_rect.width(), half_h),
            );
            let bottom_rect = Rect::from_min_size(
                Pos2::new(canvas_rect.left(), canvas_rect.top() + half_h),
                Vec2::new(canvas_rect.width(), half_h),
            );
            draw_single_viewport(
                state,
                gui,
                top_rect,
                a,
                h_scale,
                w_scale,
                renderer,
                vp_renderer,
                egui_state,
            );
            draw_single_viewport(
                state,
                gui,
                bottom_rect,
                b,
                h_scale,
                w_scale,
                renderer,
                vp_renderer,
                egui_state,
            );
        }
        ViewportLayout::Quad => {
            let half_w = canvas_rect.width() / 2.0;
            let half_h = canvas_rect.height() / 2.0;
            let rects = [
                Rect::from_min_size(canvas_rect.left_top(), Vec2::new(half_w, half_h)),
                Rect::from_min_size(
                    Pos2::new(canvas_rect.left() + half_w, canvas_rect.top()),
                    Vec2::new(half_w, half_h),
                ),
                Rect::from_min_size(
                    Pos2::new(canvas_rect.left(), canvas_rect.top() + half_h),
                    Vec2::new(half_w, half_h),
                ),
                Rect::from_min_size(
                    Pos2::new(canvas_rect.left() + half_w, canvas_rect.top() + half_h),
                    Vec2::new(half_w, half_h),
                ),
            ];
            let types = [
                ViewportType::Perspective,
                ViewportType::Top,
                ViewportType::Front,
                ViewportType::Right,
            ];
            for (i, rect) in rects.iter().enumerate() {
                draw_single_viewport(
                    state,
                    gui,
                    *rect,
                    types[i],
                    h_scale,
                    w_scale,
                    renderer,
                    vp_renderer,
                    egui_state,
                );
            }
        }
    }
}

// --- IMGUI: Single Viewport Render ---
// Renders one viewport region: 3D scene, overlays, axis labels, and HUD.
// This is the core IMGUI render loop for a single viewport panel.
// Unity equivalent: OnGUI() in a SceneView with Handles.DrawCamera.
#[allow(clippy::too_many_arguments)]
fn draw_single_viewport(
    state: &mut EditorState,
    gui: &mut Gui,
    canvas_rect: Rect,
    viewport_type: crate::viewport_renderer::ViewportType,
    h_scale: f32,
    w_scale: f32,
    renderer: &mut engine_render::renderer::Renderer,
    vp_renderer: &mut crate::viewport_renderer::ViewportRenderer,
    egui_state: &mut engine_ui::EguiState,
) {
    let painter = gui.ui.painter_at(canvas_rect);

    // Render 3D scene to offscreen viewport texture
    let vp_w = canvas_rect.width().max(1.0) as u32;
    let vp_h = canvas_rect.height().max(1.0) as u32;
    let aspect = vp_w as f32 / vp_h.max(1) as f32;

    vp_renderer.ensure_target(viewport_type, vp_w, vp_h);
    if let Some(target_view) = vp_renderer.target_view(viewport_type) {
        // Select camera based on active viewport tab
        let camera = if state.active_viewport_tab == 1
            && state.play_state != crate::state::PlayState::Editing
        {
            &state.game_camera
        } else {
            &state.camera
        };

        // Compute VP matrix based on viewport type
        let camera_vp = match viewport_type {
            crate::viewport_renderer::ViewportType::Perspective => {
                camera.projection_matrix(aspect) * camera.view_matrix()
            }
            crate::viewport_renderer::ViewportType::Top => {
                let proj = ortho_projection(aspect);
                let view = view_matrix_for([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 10.0, 0.001]);
                proj * view
            }
            crate::viewport_renderer::ViewportType::Front => {
                let proj = ortho_projection(aspect);
                let view = view_matrix_for([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 10.0]);
                proj * view
            }
            crate::viewport_renderer::ViewportType::Right => {
                let proj = ortho_projection(aspect);
                let view = view_matrix_for([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [10.0, 0.0, 0.001]);
                proj * view
            }
        };

        // Override camera_vp in build_scene using a temporary camera clone
        let mut editor_scene_data =
            state.build_scene(&renderer.device, &renderer.queue, aspect, camera);
        editor_scene_data.camera_vp = camera_vp;
        let scene = engine_render::renderer::Scene3d {
            mesh_store: &editor_scene_data.mesh_store,
            material_store: &editor_scene_data.material_store,
            lighting_uniform: &editor_scene_data.lighting,
            camera_vp: &editor_scene_data.camera_vp,
            camera_pos: &editor_scene_data.camera_pos,
            light_direction: &editor_scene_data.light_direction,
            batches: &editor_scene_data.batches,
            scene_aabb_min: editor_scene_data.scene_aabb_min,
            scene_aabb_max: editor_scene_data.scene_aabb_max,
            shadow_config: editor_scene_data.shadow_config,
        };
        let clear_color = Some(wgpu::Color {
            r: state.sky_color[0] as f64,
            g: state.sky_color[1] as f64,
            b: state.sky_color[2] as f64,
            a: 1.0,
        });
        renderer.render_frame_3d_to_target(target_view, vp_w, vp_h, &scene, clear_color);

        // Build 3D overlay batch (grid, gizmo, selection highlights)
        let mut overlay = engine_render::line3d::Line3dBatch::new();

        // Ground grid
        if state.show_grid {
            overlay.grid_xz([0.0, 0.0, 0.0], 50.0, 25, [0.25, 0.25, 0.35, 0.5]);
            // Axis lines (thicker via brighter color)
            overlay.line(
                [-25.0, 0.001, 0.0],
                [25.0, 0.001, 0.0],
                [1.0, 0.3, 0.3, 0.8],
            );
            overlay.line(
                [0.0, 0.001, -25.0],
                [0.0, 0.001, 25.0],
                [0.3, 0.6, 1.0, 0.8],
            );
        }

        // Selection highlight: draw wireframe for selected nodes
        let select_color = [1.0, 0.6, 0.0, 1.0];
        for &node_id in &state.selected_nodes {
            if let Some(t) = state.node_transforms.get(&node_id) {
                let pos = [t[0], t[1], t[2]];
                overlay.selection_sphere(pos, 0.8, select_color);
            }
        }

        // Transform gizmo at first selected node
        if let Some(&first_id) = state.selected_nodes.first()
            && let Some(t) = state.node_transforms.get(&first_id)
        {
            let pos = [t[0], t[1], t[2]];
            let gizmo_scale = state.camera.distance * 0.15;
            match state.active_tool {
                crate::state::ToolType::Translate => {
                    overlay.translate_gizmo(pos, gizmo_scale);
                }
                crate::state::ToolType::Rotate => {
                    overlay.rotate_gizmo(pos, gizmo_scale);
                }
                crate::state::ToolType::Scale => {
                    overlay.scale_gizmo(pos, gizmo_scale);
                }
                _ => {}
            }
        }

        // Debug overlay: wireframe AABBs and velocity vectors
        // Always show in Physics viewport tab, or when debug overlay is toggled
        if state.show_debug_overlay || state.active_viewport_tab == 2 {
            let debug_color = [0.0, 1.0, 1.0, 0.6]; // cyan
            let dynamic_color = [1.0, 1.0, 0.0, 0.8]; // yellow for dynamic
            for node in &state.scene_tree.nodes {
                if node.parent.is_none() {
                    continue;
                }
                if let Some(t) = state.node_transforms.get(&node.id) {
                    let pos = [t[0], t[1], t[2]];
                    let is_dynamic = state.GetHandle(node.id)
                        .map(|h| {
                            state.world.HasComponent::<engine_core::components::Rigidbody>(h)
                                && state.world.GetComponent::<engine_core::components::Rigidbody>(h)
                                    .map(|rb| !rb.is_kinematic && rb.mass > 0.0)
                                    .unwrap_or(false)
                        })
                        .unwrap_or(false);
                    let c = if is_dynamic {
                        dynamic_color
                    } else {
                        debug_color
                    };
                    // Wireframe unit cube around object
                    overlay.aabb(
                        [pos[0] - 0.5, pos[1] - 0.5, pos[2] - 0.5],
                        [pos[0] + 0.5, pos[1] + 0.5, pos[2] + 0.5],
                        c,
                    );
                    // Velocity vector for dynamic objects
                    if is_dynamic {
                        overlay.line(pos, [pos[0], pos[1] - 2.0, pos[2]], [1.0, 0.3, 0.0, 1.0]);
                    }
                    // Show collider type label position in Physics tab
                    if state.active_viewport_tab == 2 {
                        if let Some(handle) = state.GetHandle(node.id) {
                            let collider_type = if state.world.HasComponent::<engine_core::components::BoxCollider>(handle) {
                                "Box"
                            } else if state.world.HasComponent::<engine_core::components::SphereCollider>(handle) {
                                "Sphere"
                            } else if state.world.HasComponent::<engine_core::components::CapsuleCollider>(handle) {
                                "Capsule"
                            } else {
                                ""
                            };
                            if !collider_type.is_empty() {
                                let collider_color = match collider_type {
                                    "Box" => [0.0, 0.8, 1.0, 0.8],
                                    "Sphere" => [1.0, 0.5, 0.0, 0.8],
                                    "Capsule" => [0.5, 1.0, 0.5, 0.8],
                                    _ => [0.5, 0.5, 0.5, 0.6],
                                };
                                match collider_type {
                                    "Sphere" => {
                                        overlay.selection_sphere(pos, 0.6, collider_color);
                                    }
                                    _ => {
                                        overlay.aabb(
                                            [pos[0] - 0.5, pos[1] - 0.5, pos[2] - 0.5],
                                            [pos[0] + 0.5, pos[1] + 0.5, pos[2] + 0.5],
                                            collider_color,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Update line camera and render overlays
        vp_renderer.update_line_camera(&renderer.queue, &editor_scene_data.camera_vp);
        let target_view = vp_renderer.target_view(viewport_type).unwrap();
        vp_renderer.render_overlays(target_view, &overlay, renderer);

        // IMGUI: Register render texture and display in viewport
        // Unity equivalent: GUI.DrawTexture(rect, renderTexture)
        let tex_id = if let Some(id) = vp_renderer.egui_texture_id(viewport_type) {
            id
        } else {
            let target_view = vp_renderer.target_view(viewport_type).unwrap();
            let id = egui_state.register_native_texture(&renderer.device, target_view);
            vp_renderer.set_egui_texture_id(viewport_type, id);
            id
        };
        let img_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.left(), canvas_rect.top() + 32.0 * h_scale),
            Vec2::new(canvas_rect.width(), canvas_rect.height() - 32.0 * h_scale),
        );
        egui::widgets::Image::new(egui::load::SizedTexture::new(
            tex_id,
            egui::vec2(img_rect.width(), img_rect.height()),
        ))
        .paint_at(gui.ui, img_rect);
    }

    // IMGUI: Axis label indicators (X/Y/Z HUD overlay)
    // Unity equivalent: Handles.DrawGizmo with axis labels
    let axes = [
        ("X", Color32::from_rgb(255, 107, 107)),
        ("Y", Color32::from_rgb(46, 213, 115)),
        ("Z", Color32::from_rgb(77, 171, 247)),
    ];
    for (i, (label, color)) in axes.iter().enumerate() {
        painter.text(
            egui::pos2(
                canvas_rect.left() + 20.0 * w_scale,
                canvas_rect.top() + 50.0 * h_scale + i as f32 * 14.0 * h_scale,
            ),
            egui::Align2::LEFT_CENTER,
            *label,
            FontId::proportional(10.0 * h_scale),
            *color,
        );
    }

    draw_transform_overlay(state, &painter, canvas_rect, h_scale, w_scale);

    // Viewport info overlay (top-right corner)
    let info_font = FontId::proportional(10.0 * h_scale);
    let info_x = canvas_rect.right() - 10.0 * w_scale;
    let info_color = Color32::from_gray(160);

    // Object count
    let obj_count = state
        .scene_tree
        .nodes
        .iter()
        .filter(|n| n.parent.is_some())
        .count();
    painter.text(
        egui::pos2(info_x, canvas_rect.top() + 50.0 * h_scale),
        egui::Align2::RIGHT_CENTER,
        format!("对象: {}", obj_count),
        info_font.clone(),
        info_color,
    );

    // Selected object info
    if let Some(&sel_id) = state.selected_nodes.first() {
        if let Some(node) = state.scene_tree.nodes.iter().find(|n| n.id == sel_id) {
            painter.text(
                egui::pos2(info_x, canvas_rect.top() + 64.0 * h_scale),
                egui::Align2::RIGHT_CENTER,
                format!("选中: {} ({})", node.name, sel_id),
                info_font.clone(),
                Color32::from_rgb(0, 212, 170),
            );
        }
        if let Some(t) = state.node_transforms.get(&sel_id) {
            painter.text(
                egui::pos2(info_x, canvas_rect.top() + 78.0 * h_scale),
                egui::Align2::RIGHT_CENTER,
                format!("位置: {:.1}, {:.1}, {:.1}", t[0], t[1], t[2]),
                info_font.clone(),
                info_color,
            );
        }
    }

    // Camera info
    painter.text(
        egui::pos2(info_x, canvas_rect.top() + 92.0 * h_scale),
        egui::Align2::RIGHT_CENTER,
        format!("距离: {:.1}", state.camera.distance),
        info_font,
        info_color,
    );

    handle_camera_input(state, gui, canvas_rect);
}

// --- IMGUI: Transform Overlay ---
// Draws the position/rotation/scale readout at the bottom of the viewport.
// Unity equivalent: Handles.BeginGUI() / Handles.EndGUI() with GUI.Label.
fn draw_transform_overlay(
    state: &EditorState,
    painter: &egui::Painter,
    canvas_rect: Rect,
    h_scale: f32,
    w_scale: f32,
) {
    let transform_bar_h = 28.0 * h_scale;
    let transform_w = 200.0 * w_scale;
    let transform_rect = Rect::from_min_size(
        Pos2::new(
            canvas_rect.left() + 20.0 * w_scale,
            canvas_rect.bottom() - 44.0 * h_scale,
        ),
        Vec2::new(transform_w, transform_bar_h),
    );
    painter.add(Shape::rect_filled(
        transform_rect,
        Rounding::same(6.0 * h_scale),
        Color32::from_rgba_premultiplied(22, 22, 25, 230),
    ));

    let sel_trans = state
        .selected_nodes
        .first()
        .and_then(|id| state.node_transforms.get(id).copied())
        .unwrap_or([0.0; 9]);

    let transform_axes = [
        (
            "X",
            format!("{:.1}", sel_trans[0]),
            Color32::from_rgb(255, 107, 107),
        ),
        (
            "Y",
            format!("{:.1}", sel_trans[1]),
            Color32::from_rgb(46, 213, 115),
        ),
        (
            "Z",
            format!("{:.1}", sel_trans[2]),
            Color32::from_rgb(77, 171, 247),
        ),
    ];
    for (i, (label, val, color)) in transform_axes.iter().enumerate() {
        painter.text(
            egui::pos2(
                transform_rect.left() + 12.0 * w_scale + i as f32 * 60.0 * w_scale,
                transform_rect.center().y,
            ),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", label, val),
            FontId::proportional(11.0 * h_scale),
            *color,
        );
    }
}

// --- IMGUI: Camera Input Handling ---
// Processes mouse/keyboard input for camera orbit, pan, zoom, and gizmo manipulation.
// This is pure IMGUI input handling: check Event.current each frame.
// Unity equivalent: HandleUtility.AddDefaultControl + SceneView camera control.
fn handle_camera_input(state: &mut EditorState, gui: &mut Gui, canvas_rect: Rect) {
    let ctx = gui.ui.ctx();

    if !canvas_rect.contains(ctx.pointer_interact_pos().unwrap_or(Pos2::ZERO)) {
        return;
    }

    // Disable camera controls in Game tab during play mode
    if state.active_viewport_tab == 1 && state.play_state != crate::state::PlayState::Editing {
        return;
    }

    // Terrain sculpting mode
    if state.active_tool == crate::state::ToolType::Terrain {
        if ctx.input(|i| i.pointer.primary_down()) {
            if let Some(pos) = ctx.pointer_interact_pos() {
                state.terrain_sculpt_active = true;
                state.terrain_sculpt_screen_pos =
                    Some((pos.x - canvas_rect.left(), pos.y - canvas_rect.top()));
            }
        } else {
            state.terrain_sculpt_active = false;
            state.terrain_sculpt_screen_pos = None;
        }

        // Still allow camera orbit with right-click
        let canvas_id = egui::Id::new("terrain_viewport");
        let response = gui
            .ui
            .interact(canvas_rect, canvas_id, egui::Sense::click_and_drag());
        if response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta();
            state.camera.orbit(delta.x, -delta.y);
        }
        let scroll = ctx.input(|i| i.raw_scroll_delta);
        if scroll.y != 0.0 {
            state.camera.zoom(scroll.y / 120.0);
        }
        return;
    }

    let canvas_id = egui::Id::new("viewport_canvas");
    let canvas_response = gui
        .ui
        .interact(canvas_rect, canvas_id, egui::Sense::click_and_drag());

    // IMGUI: Gizmo drag (left-click to translate/rotate/scale)
    // Unity equivalent: Handles.PositionHandle / RotationHandle / ScaleHandle
    if (state.active_tool == crate::state::ToolType::Translate
        || state.active_tool == crate::state::ToolType::Rotate
        || state.active_tool == crate::state::ToolType::Scale)
        && !state.selected_nodes.is_empty()
    {
        let primary_down = ctx.input(|i| i.pointer.primary_down());
        let pointer_pos = ctx.pointer_interact_pos().unwrap_or(Pos2::ZERO);

        if primary_down && canvas_rect.contains(pointer_pos) {
            if state.gizmo_drag_axis.is_none() {
                // Start gizmo drag
                state.gizmo_drag_axis = Some(0);
                state.gizmo_drag_start_screen = Some((pointer_pos.x, pointer_pos.y));
                // Use Unity-style World API to get start transform
                let first_id = state.selected_nodes[0];
                if let Some(handle) = state.GetHandle(first_id) {
                    if let Some(t) = state.world.GetTransform(handle) {
                        let pos = t.Position();
                        let rot = t.Rotation();
                        let scale = t.LossyScale();
                        state.gizmo_drag_start_pos = Some([
                            pos.x, pos.y, pos.z,
                            rot.to_euler(engine_math::EulerRot::XYZ).0.to_degrees(),
                            rot.to_euler(engine_math::EulerRot::XYZ).1.to_degrees(),
                            rot.to_euler(engine_math::EulerRot::XYZ).2.to_degrees(),
                            scale.x, scale.y, scale.z,
                        ]);
                    }
                }
            } else if let (Some((sx, sy)), Some(start_pos)) =
                (state.gizmo_drag_start_screen, state.gizmo_drag_start_pos)
            {
                let dx = pointer_pos.x - sx;
                let dy = pointer_pos.y - sy;
                let sensitivity = state.camera.distance * 0.003;

                // Use Unity-style World API for transform manipulation
                let first_id = state.selected_nodes[0];
                if let Some(handle) = state.GetHandle(first_id) {
                    if let Some(t) = state.world.GetTransformMut(handle) {
                        match state.active_tool {
                            crate::state::ToolType::Translate => {
                                let world_dx = dx * sensitivity;
                                let world_dz = dy * sensitivity;
                                t.SetPosition(engine_math::Vec3::new(
                                    start_pos[0] + world_dx,
                                    start_pos[1],
                                    start_pos[2] + world_dz,
                                ));
                            }
                            crate::state::ToolType::Rotate => {
                                let rot_sensitivity = 0.01;
                                let new_euler = engine_math::Vec3::new(
                                    start_pos[3] + dy * rot_sensitivity,
                                    start_pos[4] + dx * rot_sensitivity,
                                    start_pos[5],
                                );
                                t.SetRotation(engine_math::Quat::from_euler(
                                    engine_math::EulerRot::XYZ,
                                    new_euler.x.to_radians(),
                                    new_euler.y.to_radians(),
                                    new_euler.z.to_radians(),
                                ));
                            }
                            crate::state::ToolType::Scale => {
                                let scale_sensitivity = 0.005;
                                let scale_factor = 1.0 + dy * scale_sensitivity;
                                t.SetLocalScale(engine_math::Vec3::new(
                                    (start_pos[6] * scale_factor).max(0.01),
                                    (start_pos[7] * scale_factor).max(0.01),
                                    (start_pos[8] * scale_factor).max(0.01),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else if state.gizmo_drag_axis.is_some() {
            // Release gizmo drag
            state.gizmo_drag_axis = None;
            state.gizmo_drag_start_screen = None;
            state.gizmo_drag_start_pos = None;
        }
    }

    // IMGUI: Object selection (left-click with distance check)
    // Unity equivalent: HandleUtility.PickGameObject in OnSceneGUI
    if canvas_response.clicked()
        && state.active_tool != crate::state::ToolType::Terrain
        && state.gizmo_drag_axis.is_none()
        && let Some(click_pos) = ctx.pointer_interact_pos()
    {
        let rel_x = click_pos.x - canvas_rect.left();
        let rel_y = click_pos.y - canvas_rect.top();
        let canvas_w = canvas_rect.width();
        let canvas_h = canvas_rect.height();

        // Project each object to screen space and find nearest to click
        // Use Unity-style World API for transform access
        let aspect = canvas_w / canvas_h.max(1.0);
        let vp = state.camera.projection_matrix(aspect) * state.camera.view_matrix();

        let mut best_id = None;
        let mut best_dist_sq = f32::MAX;

        // Iterate through all node_to_handle mappings
        for (&node_id, &handle) in &state.node_to_handle {
            if let Some(t) = state.world.GetTransform(handle) {
                let world_pos = t.Position();
                let clip = vp * world_pos.extend(1.0);
                if clip.w <= 0.001 {
                    continue;
                }
                let ndc = clip.truncate() / clip.w;
                let screen_x = canvas_w * 0.5 * (ndc.x + 1.0);
                let screen_y = canvas_h * 0.5 * (1.0 - ndc.y);
                let dx = screen_x - rel_x;
                let dy = screen_y - rel_y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_id = Some(node_id);
                }
            }
        }

        // Select if within reasonable distance (25 pixels)
        if best_dist_sq < 625.0 {
            if let Some(id) = best_id {
                let ctrl = ctx.input(|i| i.modifiers.ctrl);
                if ctrl {
                    // Toggle selection
                    if let Some(pos) = state.selected_nodes.iter().position(|&n| n == id) {
                        state.selected_nodes.remove(pos);
                    } else {
                        state.selected_nodes.push(id);
                    }
                } else {
                    state.selected_nodes = vec![id];
                }
            }
        } else if !ctx.input(|i| i.modifiers.ctrl) {
            state.selected_nodes.clear();
        }
    }

    // IMGUI: Camera orbit (right-click drag)
    // Unity equivalent: SceneView.CameraSettings orbit control
    if canvas_response.dragged_by(egui::PointerButton::Secondary) {
        let delta = canvas_response.drag_delta();
        state.camera.orbit(delta.x, -delta.y);
    }

    // IMGUI: Camera pan (middle-click drag)
    // Unity equivalent: SceneView.CameraSettings pan control
    if canvas_response.dragged_by(egui::PointerButton::Middle) {
        let delta = canvas_response.drag_delta();
        state.camera.pan(delta.x, delta.y);
    }

    // IMGUI: Camera zoom (scroll wheel)
    // Unity equivalent: SceneView.CameraSettings zoom control
    let scroll = ctx.input(|i| i.raw_scroll_delta);
    if scroll.y != 0.0 {
        state.camera.zoom(scroll.y / 120.0);
    }

    // IMGUI: Camera help overlay (toggle with H key)
    // Unity equivalent: Handles.BeginGUI() with GUI.Box and GUI.Label
    if ctx.input(|i| i.key_pressed(egui::Key::H)) {
        state.show_camera_help = !state.show_camera_help;
    }
    if state.show_camera_help {
        let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
        let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;
        let help_lines = [
            "摄像机控制:",
            "  右键拖拽 - 旋转视角",
            "  中键拖拽 - 平移视角",
            "  滚轮 - 缩放",
            "  F - 聚焦选中物体",
            "  H - 显示/隐藏此帮助",
            "",
            "工具快捷键:",
            "  Q - 选择",
            "  W - 移动",
            "  E - 旋转",
            "  R - 缩放",
            "",
            "操作:",
            "  Ctrl+Z - 撤销",
            "  Ctrl+Y - 重做",
            "  Delete - 删除选中",
            "  Ctrl+D - 复制选中",
            "  Ctrl+S - 保存场景",
        ];
        let help_w = 200.0 * w_scale;
        let help_h = help_lines.len() as f32 * 14.0 * h_scale + 16.0 * h_scale;
        let help_rect = Rect::from_min_size(
            Pos2::new(
                canvas_rect.right() - help_w - 10.0 * w_scale,
                canvas_rect.bottom() - help_h - 10.0 * h_scale,
            ),
            Vec2::new(help_w, help_h),
        );
        let painter = gui.ui.painter_at(help_rect);
        painter.add(Shape::rect_filled(
            help_rect,
            Rounding::same(6.0 * h_scale),
            Color32::from_rgba_premultiplied(22, 22, 25, 220),
        ));
        for (i, line) in help_lines.iter().enumerate() {
            painter.text(
                Pos2::new(
                    help_rect.left() + 8.0 * w_scale,
                    help_rect.top() + 8.0 * h_scale + i as f32 * 14.0 * h_scale,
                ),
                egui::Align2::LEFT_TOP,
                *line,
                FontId::proportional(10.0 * h_scale),
                Color32::from_gray(180),
            );
        }
    }
}
