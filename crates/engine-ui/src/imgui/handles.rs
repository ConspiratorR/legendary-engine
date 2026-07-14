//! Handles class (matches Unity's Handles).

use engine_math::Vec3;

/// 3D handle utilities for scene editing (matches Unity's `Handles`).
pub struct Handles;

impl Handles {
    /// Draw a sphere handle cap (matches `Handles.SphereHandleCap`).
    pub fn SphereHandleCap(
        id: i32,
        position: Vec3,
        rotation: engine_math::Quat,
        size: f32,
        control_id: i32,
    ) -> bool {
        let _ = (id, position, rotation, size, control_id);
        false
    }

    /// Draw a cube handle cap (matches `Handles.CubeHandleCap`).
    pub fn CubeHandleCap(
        id: i32,
        position: Vec3,
        rotation: engine_math::Quat,
        size: f32,
        control_id: i32,
    ) -> bool {
        let _ = (id, position, rotation, size, control_id);
        false
    }

    /// Draw a cylinder handle cap (matches `Handles.CylinderHandleCap`).
    pub fn CylinderHandleCap(
        id: i32,
        position: Vec3,
        rotation: engine_math::Quat,
        size: f32,
        control_id: i32,
    ) -> bool {
        let _ = (id, position, rotation, size, control_id);
        false
    }

    /// Draw a arrow handle cap (matches `Handles.ArrowHandleCap`).
    pub fn ArrowHandleCap(
        id: i32,
        position: Vec3,
        rotation: engine_math::Quat,
        size: f32,
        control_id: i32,
    ) -> bool {
        let _ = (id, position, rotation, size, control_id);
        false
    }

    /// Draw a free move handle (matches `Handles.FreeMoveHandle`).
    pub fn FreeMoveHandle(position: Vec3, size: f32, snap: Vec3, control_id: i32) -> Vec3 {
        let _ = (size, snap, control_id);
        position
    }

    /// Draw a position handle (matches `Handles.PositionHandle`).
    pub fn PositionHandle(position: Vec3, rotation: engine_math::Quat) -> Vec3 {
        let _ = rotation;
        position
    }

    /// Draw a rotation handle (matches `Handles.RotationHandle`).
    pub fn RotationHandle(rotation: engine_math::Quat, size: f32) -> engine_math::Quat {
        let _ = size;
        rotation
    }

    /// Draw a scale handle (matches `Handles.ScaleHandle`).
    pub fn ScaleHandle(
        scale: Vec3,
        position: Vec3,
        rotation: engine_math::Quat,
        size: f32,
    ) -> Vec3 {
        let _ = (position, rotation, size);
        scale
    }

    /// Draw a wire sphere (matches `Handles.WireSphere`).
    pub fn WireSphere(center: Vec3, radius: f32) {
        let _ = (center, radius);
    }

    /// Draw a wire cube (matches `Handles.WireCube`).
    pub fn WireCube(center: Vec3, size: Vec3) {
        let _ = (center, size);
    }

    /// Draw a line (matches `Handles.DrawLine`).
    pub fn DrawLine(from: Vec3, to: Vec3) {
        let _ = (from, to);
    }

    /// Draw a dotted line (matches `Handles.DrawDottedLine`).
    pub fn DrawDottedLine(from: Vec3, to: Vec3, dash_size: f32) {
        let _ = (from, to, dash_size);
    }

    /// Draw a wire disc (matches `Handles.WireDisc`).
    pub fn WireDisc(center: Vec3, normal: Vec3, radius: f32) {
        let _ = (center, normal, radius);
    }

    /// Draw an arc (matches `Handles.Arc`).
    pub fn Arc(center: Vec3, normal: Vec3, from: Vec3, angle: f32, radius: f32) {
        let _ = (center, normal, from, angle, radius);
    }

    /// Draw a label (matches `Handles.Label`).
    pub fn Label(position: Vec3, text: &str) {
        let _ = (position, text);
    }

    /// Draw a pivot cap (matches `Handles.PivotCap`).
    pub fn PivotCap(position: Vec3, size: f32) {
        let _ = (position, size);
    }

    /// Draw a radius handle (matches `Handles.RadiusHandle`).
    pub fn RadiusHandle(rotation: engine_math::Quat, radius: f32, position: Vec3) -> f32 {
        let _ = (rotation, position);
        radius
    }

    // ── egui Painter methods (actual rendering) ──────────────────────────

    /// Draw a wire sphere using egui Painter (actual rendering).
    /// Unity: `Handles.WireSphere(Vector3, float)`
    pub fn WireSphereEgui(
        painter: &egui::Painter,
        center: [f32; 2],
        radius: f32,
        color: egui::Color32,
    ) {
        painter.circle(
            egui::Pos2::new(center[0], center[1]),
            radius,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, color),
        );
    }

    /// Draw a wire cube using egui Painter (actual rendering).
    /// Unity: `Handles.WireCube(Vector3, Vector3)`
    pub fn WireCubeEgui(
        painter: &egui::Painter,
        center: [f32; 2],
        size: [f32; 2],
        color: egui::Color32,
    ) {
        let rect = egui::Rect::from_center_size(
            egui::Pos2::new(center[0], center[1]),
            egui::Vec2::new(size[0], size[1]),
        );
        painter.rect(
            rect,
            0.0,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, color),
        );
    }

    /// Draw a line using egui Painter (actual rendering).
    /// Unity: `Handles.DrawLine(Vector3, Vector3)`
    pub fn DrawLineEgui(
        painter: &egui::Painter,
        from: [f32; 2],
        to: [f32; 2],
        color: egui::Color32,
    ) {
        painter.line_segment(
            [
                egui::Pos2::new(from[0], from[1]),
                egui::Pos2::new(to[0], to[1]),
            ],
            egui::Stroke::new(1.0, color),
        );
    }

    /// Draw a dotted line using egui Painter (actual rendering).
    /// Unity: `Handles.DrawDottedLine(Vector3, Vector3, float)`
    pub fn DrawDottedLineEgui(
        painter: &egui::Painter,
        from: [f32; 2],
        to: [f32; 2],
        color: egui::Color32,
        dash_size: f32,
    ) {
        let from_pos = egui::Pos2::new(from[0], from[1]);
        let to_pos = egui::Pos2::new(to[0], to[1]);
        let dir = (to_pos - from_pos).normalized();
        let len = (to_pos - from_pos).length();
        let mut pos = from_pos;
        let mut drawn = 0.0;
        while drawn < len {
            let end = (pos + dir * dash_size).min(to_pos);
            painter.line_segment([pos, end], egui::Stroke::new(1.0, color));
            pos = end + dir * dash_size;
            drawn += dash_size * 2.0;
        }
    }

    /// Draw a label using egui Painter (actual rendering).
    /// Unity: `Handles.Label(Vector3, string)`
    pub fn LabelEgui(
        painter: &egui::Painter,
        position: [f32; 2],
        text: &str,
        color: egui::Color32,
    ) {
        painter.text(
            egui::Pos2::new(position[0], position[1]),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(14.0),
            color,
        );
    }

    /// Draw a wire disc using egui Painter (actual rendering).
    /// Unity: `Handles.WireDisc(Vector3, Vector3, float)`
    pub fn WireDiscEgui(
        painter: &egui::Painter,
        center: [f32; 2],
        radius: f32,
        color: egui::Color32,
    ) {
        painter.circle(
            egui::Pos2::new(center[0], center[1]),
            radius,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, color),
        );
    }

    /// Draw an arc using egui Painter (actual rendering).
    /// Unity: `Handles.Arc(Vector3, Vector3, Vector3, float, float)`
    pub fn ArcEgui(
        painter: &egui::Painter,
        center: [f32; 2],
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        color: egui::Color32,
    ) {
        let steps = 20;
        let angle_step = (end_angle - start_angle) / steps as f32;
        for i in 0..steps {
            let a1 = start_angle + angle_step * i as f32;
            let a2 = start_angle + angle_step * (i + 1) as f32;
            let p1 = egui::Pos2::new(center[0] + radius * a1.cos(), center[1] + radius * a1.sin());
            let p2 = egui::Pos2::new(center[0] + radius * a2.cos(), center[1] + radius * a2.sin());
            painter.line_segment([p1, p2], egui::Stroke::new(1.0, color));
        }
    }
}
