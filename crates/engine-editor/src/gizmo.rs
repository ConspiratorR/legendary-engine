//! Viewport gizmo — translate, rotate, and scale handles drawn over the
//! 3-D viewport for interactive object manipulation.

use crate::state::{EditorState, GizmoInteraction, GizmoState, ToolType};
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

const AXIS_COLORS: [Color32; 3] = [
    Color32::from_rgb(255, 107, 107),
    Color32::from_rgb(46, 213, 115),
    Color32::from_rgb(77, 171, 247),
];

const AXIS_DIRS: [Vec2; 3] = [
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, -1.0),
    Vec2::new(-0.7, 0.7),
];

/// Compute distance from point `p` to line segment `a`-`b`.
fn point_to_segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = Vec2::new(b.x - a.x, b.y - a.y);
    let ap = Vec2::new(p.x - a.x, p.y - a.y);
    let ab_len_sq = ab.dot(ab);
    if ab_len_sq < 1e-6 {
        return ap.length();
    }
    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let projection = Pos2::new(a.x + t * ab.x, a.y + t * ab.y);
    let diff = Vec2::new(p.x - projection.x, p.y - projection.y);
    diff.length()
}

/// Detect which gizmo axis the mouse is hovering over.
/// Returns axis index (0=X, 1=Y, 2=Z) if close enough, otherwise `None`.
pub fn detect_hover(
    state: &EditorState,
    mouse_pos: Pos2,
    gizmo_center: Pos2,
    gizmo_size: f32,
    h_scale: f32,
) -> Option<usize> {
    if state.selected_nodes.is_empty() {
        return None;
    }
    let threshold = 12.0 * h_scale;
    for (i, &dir) in AXIS_DIRS.iter().enumerate() {
        let tip = Pos2::new(
            gizmo_center.x + dir.x * gizmo_size,
            gizmo_center.y + dir.y * gizmo_size,
        );
        let dist = point_to_segment_distance(mouse_pos, gizmo_center, tip);
        if dist < threshold {
            return Some(i);
        }
    }
    None
}

/// Start dragging a gizmo axis.
pub fn start_drag(state: &mut EditorState, axis: usize, mouse_pos: Pos2) {
    let node_id = state.selected_nodes.first().copied().unwrap_or(0);
    let world_pos = state
        .node_transforms
        .get(&node_id)
        .copied()
        .unwrap_or([0.0; 9]);
    if let Some(ref mut interaction) = state.gizmo_interaction {
        interaction.state = GizmoState::DraggingAxis(axis);
        interaction.drag_start_screen = mouse_pos;
        interaction.drag_start_world_pos = [world_pos[0], world_pos[1], world_pos[2]];
        interaction.drag_axis = axis;
        interaction.drag_start_full_transform = world_pos;
    }
    // Also update the legacy drag fields for backward compatibility
    state.gizmo_drag_axis = Some(axis as u8);
    state.gizmo_drag_start_screen = Some((mouse_pos.x, mouse_pos.y));
    state.gizmo_drag_start_pos = Some(world_pos);
}

/// Update drag position (called each frame while dragging).
pub fn update_drag(state: &mut EditorState, mouse_pos: Pos2) {
    let (axis, start_screen, start_world) = if let Some(ref interaction) = state.gizmo_interaction {
        match interaction.state {
            GizmoState::DraggingAxis(axis) => (
                axis,
                interaction.drag_start_screen,
                interaction.drag_start_world_pos,
            ),
            _ => return,
        }
    } else {
        return;
    };

    let node_id = state.selected_nodes.first().copied().unwrap_or(0);
    let delta_screen = Vec2::new(mouse_pos.x - start_screen.x, mouse_pos.y - start_screen.y);
    let scale = state.camera.distance * 0.003;
    let mut world_delta = [0.0f32; 3];
    let screen_delta = match axis {
        0 => delta_screen.x,  // X axis: horizontal mouse movement
        1 => -delta_screen.y, // Y axis: vertical mouse movement (negate because screen Y is down)
        2 => delta_screen.x,  // Z axis: horizontal mouse movement (screen-space proxy)
        _ => 0.0,
    };
    world_delta[axis] = screen_delta * scale;

    if let Some(t) = state.node_transforms.get_mut(&node_id) {
        t[0] = start_world[0] + world_delta[0];
        t[1] = start_world[1] + world_delta[1];
        t[2] = start_world[2] + world_delta[2];
    }

    // Sync to World API (write-through under unity-world-primary)
    if let Some(handle) = state.GetHandle(node_id) {
        let pos = engine_math::Vec3::new(
            start_world[0] + world_delta[0],
            start_world[1] + world_delta[1],
            start_world[2] + world_delta[2],
        );
        let _ = state.world.with_transform_mut(handle, |transform| {
            transform.SetPosition(pos);
        });
    }
}

/// End drag, returning (node_id, old_full_transform, new_full_transform) if a drag was active.
pub fn end_drag(state: &mut EditorState) -> Option<(u64, [f32; 9], [f32; 9])> {
    let old_full = if let Some(ref interaction) = state.gizmo_interaction {
        match interaction.state {
            GizmoState::DraggingAxis(_) => interaction.drag_start_full_transform,
            _ => return None,
        }
    } else {
        return None;
    };

    let node_id = state.selected_nodes.first().copied().unwrap_or(0);
    let new_full = state
        .node_transforms
        .get(&node_id)
        .copied()
        .unwrap_or(old_full);

    // Reset state
    if let Some(ref mut interaction) = state.gizmo_interaction {
        interaction.state = GizmoState::Idle;
    }
    state.gizmo_drag_axis = None;
    state.gizmo_drag_start_screen = None;
    state.gizmo_drag_start_pos = None;

    Some((node_id, old_full, new_full))
}

/// Get the current gizmo center and size for the given canvas rect.
pub fn gizmo_metrics(canvas_rect: Rect, h_scale: f32) -> (Pos2, f32) {
    let gizmo_center = Pos2::new(canvas_rect.right() - 100.0, canvas_rect.top() + 80.0);
    let gizmo_size = 60.0 * h_scale;
    (gizmo_center, gizmo_size)
}

pub fn draw(
    state: &mut EditorState,
    painter: &egui::Painter,
    canvas_rect: Rect,
    h_scale: f32,
    _w_scale: f32,
    mouse_pos: Option<Pos2>,
) {
    let (gizmo_center, gizmo_size) = gizmo_metrics(canvas_rect, h_scale);

    // Update hover state when not dragging — supports axis-to-axis transitions
    if !matches!(
        state
            .gizmo_interaction
            .as_ref()
            .map(|i| i.state)
            .unwrap_or(GizmoState::Idle),
        GizmoState::DraggingAxis(_)
    ) {
        if let Some(mp) = mouse_pos {
            if let Some(axis) = detect_hover(state, mp, gizmo_center, gizmo_size, h_scale) {
                if let Some(ref mut interaction) = state.gizmo_interaction {
                    interaction.state = GizmoState::HoverAxis(axis);
                }
            } else if let Some(ref mut interaction) = state.gizmo_interaction {
                interaction.state = GizmoState::Idle;
            }
        }
    }

    let interaction = state.gizmo_interaction.as_ref().unwrap();

    match state.active_tool {
        ToolType::Translate => {
            draw_translate_gizmo(painter, gizmo_center, gizmo_size, interaction);
        }
        ToolType::Rotate => draw_rotate_gizmo(painter, gizmo_center, gizmo_size, interaction),
        ToolType::Scale => draw_scale_gizmo(painter, gizmo_center, gizmo_size, interaction),
        ToolType::Select | ToolType::Terrain => {}
    }
}

fn draw_translate_gizmo(
    painter: &egui::Painter,
    center: Pos2,
    size: f32,
    interaction: &GizmoInteraction,
) {
    for (i, &dir) in AXIS_DIRS.iter().enumerate() {
        let tip = Pos2::new(center.x + dir.x * size, center.y + dir.y * size);
        let is_active = matches!(interaction.state, GizmoState::HoverAxis(a) if a == i)
            || matches!(interaction.state, GizmoState::DraggingAxis(a) if a == i);
        let color = if is_active {
            Color32::WHITE
        } else {
            AXIS_COLORS[i]
        };
        let stroke_width = if is_active { 4.0_f32 } else { 3.0_f32 };
        painter.add(Shape::line(
            vec![center, tip],
            Stroke::new(stroke_width, color),
        ));
        let arrow_base = Pos2::new(
            center.x + dir.x * (size - 8.0),
            center.y + dir.y * (size - 8.0),
        );
        let perp = Vec2::new(-dir.y, dir.x);
        painter.add(Shape::convex_polygon(
            vec![tip, arrow_base + perp * 4.0, arrow_base - perp * 4.0],
            color,
            Stroke::NONE,
        ));
    }
}

fn draw_rotate_gizmo(
    painter: &egui::Painter,
    center: Pos2,
    size: f32,
    _interaction: &GizmoInteraction,
) {
    for (i, &start_angle) in [0.0, 90.0_f32.to_radians(), 180.0_f32.to_radians()]
        .iter()
        .enumerate()
    {
        let color = AXIS_COLORS[i];
        let mut points = Vec::with_capacity(31);
        for a in 0..=30 {
            let angle = start_angle + a as f32 * 120.0_f32.to_radians() / 30.0;
            let p = Pos2::new(center.x + angle.cos() * size, center.y + angle.sin() * size);
            points.push(p);
        }
        painter.add(Shape::line(points, Stroke::new(2.0_f32, color)));
    }
}

fn draw_scale_gizmo(
    painter: &egui::Painter,
    center: Pos2,
    size: f32,
    interaction: &GizmoInteraction,
) {
    for (i, &dir) in AXIS_DIRS.iter().enumerate() {
        let tip = Pos2::new(center.x + dir.x * size, center.y + dir.y * size);
        let is_active = matches!(interaction.state, GizmoState::HoverAxis(a) if a == i)
            || matches!(interaction.state, GizmoState::DraggingAxis(a) if a == i);
        let color = if is_active {
            Color32::WHITE
        } else {
            AXIS_COLORS[i]
        };
        painter.add(Shape::line(
            vec![center, tip],
            Stroke::new(
                if is_active { 3.0_f32 } else { 2.0_f32 },
                Color32::from_rgba_premultiplied(
                    color.r(),
                    color.g(),
                    color.b(),
                    if is_active { 255 } else { 100 },
                ),
            ),
        ));
        let cube_size = if is_active { 12.0 } else { 10.0 };
        let cube_rect = Rect::from_center_size(tip, Vec2::new(cube_size, cube_size));
        painter.add(Shape::rect_filled(cube_rect, Rounding::ZERO, color));
    }
    let center_cube = Rect::from_center_size(center, Vec2::new(10.0, 10.0));
    painter.add(Shape::rect_filled(
        center_cube,
        Rounding::same(2.0),
        Color32::WHITE,
    ));
}
