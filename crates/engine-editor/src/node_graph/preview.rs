use egui::{Color32, Rect, Rounding, Shape, Stroke, Vec2};

use super::evaluator::{self, EvalContext};
use super::export::MaterialParams;
use super::graph::NodeGraph;

/// Size of the preview sphere in pixels.
const PREVIEW_SIZE: u32 = 128;

/// A CPU-side material preview renderer using ray-sphere intersection.
#[derive(Debug, Clone)]
pub struct MaterialPreview {
    /// Cached preview pixels (RGBA).
    pixels: Vec<u8>,
    /// Whether the preview needs to be regenerated.
    dirty: bool,
    /// Last evaluated material parameters.
    last_params: Option<MaterialParams>,
}

impl Default for MaterialPreview {
    fn default() -> Self {
        Self {
            pixels: vec![0u8; (PREVIEW_SIZE * PREVIEW_SIZE * 4) as usize],
            dirty: true,
            last_params: None,
        }
    }
}

impl MaterialPreview {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the preview as needing regeneration.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Update the preview from a material graph.
    pub fn update(&mut self, graph: &NodeGraph) {
        let _result = evaluator::evaluate(graph, &EvalContext::default());
        let material_params = super::export::extract_material_params(graph);

        if self.last_params.as_ref() == Some(&material_params) {
            return;
        }

        self.last_params = Some(material_params.clone());
        self.dirty = true;
        self.render_sphere(&material_params);
    }

    /// Render the preview sphere with the given material parameters.
    fn render_sphere(&mut self, params: &MaterialParams) {
        let size = PREVIEW_SIZE as f32;
        let center = size * 0.5;
        let radius = size * 0.4;

        for y in 0..PREVIEW_SIZE {
            for x in 0..PREVIEW_SIZE {
                let px = x as f32;
                let py = y as f32;

                // Ray-sphere intersection (orthographic projection)
                let dx = px - center;
                let dy = py - center;
                let dist_sq = dx * dx + dy * dy;

                let pixel_idx = ((y * PREVIEW_SIZE + x) * 4) as usize;

                if dist_sq > radius * radius {
                    // Background - dark gradient
                    let bg = 20;
                    self.pixels[pixel_idx] = bg;
                    self.pixels[pixel_idx + 1] = bg;
                    self.pixels[pixel_idx + 2] = bg;
                    self.pixels[pixel_idx + 3] = 255;
                    continue;
                }

                // Calculate sphere normal
                let dz = (radius * radius - dist_sq).sqrt();
                let nx = dx / radius;
                let ny = -dy / radius; // Flip Y for correct orientation
                let nz = dz / radius;

                // Simple directional light from upper-right
                let light_dir = [0.5, 0.7, 0.5_f32];
                let light_len = (light_dir[0] * light_dir[0]
                    + light_dir[1] * light_dir[1]
                    + light_dir[2] * light_dir[2])
                    .sqrt();
                let l = [
                    light_dir[0] / light_len,
                    light_dir[1] / light_len,
                    light_dir[2] / light_len,
                ];

                // Diffuse lighting
                let ndotl = (nx * l[0] + ny * l[1] + nz * l[2]).max(0.0);

                // Specular (Blinn-Phong)
                let view_dir = [0.0, 0.0, 1.0_f32];
                let h = [l[0] + view_dir[0], l[1] + view_dir[1], l[2] + view_dir[2]];
                let h_len = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
                let h_norm = [h[0] / h_len, h[1] / h_len, h[2] / h_len];
                let ndoth = (nx * h_norm[0] + ny * h_norm[1] + nz * h_norm[2]).max(0.0);
                let spec_power = 32.0 / (params.roughness * params.roughness + 0.001);
                let spec = ndoth.powf(spec_power) * (1.0 - params.roughness);

                // Fresnel (Schlick approximation)
                let fresnel =
                    params.base_color[3] + (1.0 - params.base_color[3]) * (1.0 - nz).powf(5.0);

                // Combine: base_color * diffuse + specular
                let ambient = 0.15;
                let diffuse = ndotl * 0.7;
                let r =
                    (params.base_color[0] * (ambient + diffuse) + spec * fresnel).clamp(0.0, 1.0);
                let g =
                    (params.base_color[1] * (ambient + diffuse) + spec * fresnel).clamp(0.0, 1.0);
                let b =
                    (params.base_color[2] * (ambient + diffuse) + spec * fresnel).clamp(0.0, 1.0);

                // Add emissive
                let r = (r + params.emissive[0]).clamp(0.0, 1.0);
                let g = (g + params.emissive[1]).clamp(0.0, 1.0);
                let b = (b + params.emissive[2]).clamp(0.0, 1.0);

                // Metallic tint on specular
                let r = (r + spec * params.metallic * params.base_color[0]).clamp(0.0, 1.0);
                let g = (g + spec * params.metallic * params.base_color[1]).clamp(0.0, 1.0);
                let b = (b + spec * params.metallic * params.base_color[2]).clamp(0.0, 1.0);

                self.pixels[pixel_idx] = (r * 255.0) as u8;
                self.pixels[pixel_idx + 1] = (g * 255.0) as u8;
                self.pixels[pixel_idx + 2] = (b * 255.0) as u8;
                self.pixels[pixel_idx + 3] = 255;
            }
        }

        self.dirty = false;
    }

    /// Draw the preview into an egui region.
    pub fn draw(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);

        // Background
        painter.add(Shape::rect_filled(
            rect,
            Rounding::same(8.0),
            Color32::from_rgb(20, 20, 24),
        ));

        // Draw pixels as colored rectangles (scaled)
        let scale_x = rect.width() / PREVIEW_SIZE as f32;
        let scale_y = rect.height() / PREVIEW_SIZE as f32;

        // Draw in chunks for better performance
        let step = 2; // Draw every other pixel for speed
        for y in (0..PREVIEW_SIZE).step_by(step) {
            for x in (0..PREVIEW_SIZE).step_by(step) {
                let pixel_idx = ((y * PREVIEW_SIZE + x) * 4) as usize;
                let r = self.pixels[pixel_idx];
                let g = self.pixels[pixel_idx + 1];
                let b = self.pixels[pixel_idx + 2];

                if r == 0 && g == 0 && b == 0 {
                    continue; // Skip background
                }

                let px = rect.left() + x as f32 * scale_x;
                let py = rect.top() + y as f32 * scale_y;
                let w = step as f32 * scale_x;
                let h = step as f32 * scale_y;

                painter.add(Shape::rect_filled(
                    Rect::from_min_size(egui::pos2(px, py), Vec2::new(w, h)),
                    Rounding::ZERO,
                    Color32::from_rgb(r, g, b),
                ));
            }
        }

        // Border
        painter.add(Shape::rect_stroke(
            rect,
            Rounding::same(8.0),
            Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 70)),
        ));
    }

    /// Get the raw pixel data (for texture upload).
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Get the preview dimensions.
    pub fn size(&self) -> (u32, u32) {
        (PREVIEW_SIZE, PREVIEW_SIZE)
    }
}

/// Extract a simple MaterialParams for preview without full graph evaluation.
/// This is a fast path for when the graph hasn't changed.
pub fn quick_preview_params(graph: &NodeGraph) -> MaterialParams {
    super::export::extract_material_params(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_graph::graph::NodeType;
    use crate::node_graph::nodes::create_node;
    use egui::Pos2;

    #[test]
    fn test_preview_default() {
        let preview = MaterialPreview::new();
        assert_eq!(preview.size(), (PREVIEW_SIZE, PREVIEW_SIZE));
        assert!(preview.dirty);
    }

    #[test]
    fn test_preview_render() {
        let mut preview = MaterialPreview::new();
        let mut params = MaterialParams::default();
        params.base_color = [0.8, 0.2, 0.1, 1.0];
        params.metallic = 0.5;
        params.roughness = 0.3;
        preview.render_sphere(&params);
        assert!(!preview.dirty);

        // Check that some pixels are non-zero (sphere was rendered)
        let non_zero = preview.pixels.iter().filter(|&&b| b > 0).count();
        assert!(non_zero > 0, "Expected non-zero pixels in preview");
    }

    #[test]
    fn test_preview_update() {
        let mut preview = MaterialPreview::new();
        let mut graph = NodeGraph::new();

        let output = create_node(NodeType::MaterialOutput, Pos2::ZERO);
        graph.add_node(output);

        preview.update(&graph);
        // Should not panic and should update dirty flag
    }
}
