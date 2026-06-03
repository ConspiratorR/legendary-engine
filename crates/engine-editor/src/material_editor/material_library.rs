use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use std::collections::HashMap;

use crate::node_graph::export::MaterialParams;

/// A saved material preset.
#[derive(Debug, Clone)]
pub struct MaterialPreset {
    pub name: String,
    pub params: MaterialParams,
    pub category: String,
}

/// Material library for browsing, saving, and applying material presets.
#[derive(Debug, Clone)]
pub struct MaterialLibrary {
    pub presets: Vec<MaterialPreset>,
    pub search_text: String,
    pub selected_preset: Option<usize>,
    pub expanded_categories: HashMap<String, bool>,
    pub visible: bool,
}

impl Default for MaterialLibrary {
    fn default() -> Self {
        let mut expanded = HashMap::new();
        expanded.insert("Metal".to_string(), true);
        expanded.insert("Plastic".to_string(), true);
        expanded.insert("Wood".to_string(), false);
        expanded.insert("Glass".to_string(), false);
        expanded.insert("Custom".to_string(), true);

        Self {
            presets: default_presets(),
            search_text: String::new(),
            selected_preset: None,
            expanded_categories: expanded,
            visible: false,
        }
    }
}

impl MaterialLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draw the material library panel.
    pub fn draw(&mut self, ui: &mut egui::Ui, rect: Rect) {
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
            "材质库",
            egui::FontId::proportional(12.0 * h_scale),
            Color32::from_rgb(200, 200, 200),
        );

        // Search bar
        let search_h = 28.0 * h_scale;
        let search_rect = Rect::from_min_size(
            Pos2::new(rect.left() + pad, rect.top() + header_h + pad),
            Vec2::new(rect.width() - pad * 2.0, search_h),
        );
        painter.add(Shape::rect_filled(
            search_rect,
            Rounding::same(4.0),
            Color32::from_rgb(30, 30, 34),
        ));

        let search_id = egui::Id::new("material_lib_search");
        let mut search_text = self.search_text.clone();
        let resp = ui.put(
            search_rect,
            egui::TextEdit::singleline(&mut search_text)
                .id(search_id)
                .font(egui::FontId::proportional(11.0 * h_scale))
                .text_color(Color32::from_gray(200))
                .frame(false)
                .hint_text("搜索材质..."),
        );
        if resp.changed() {
            self.search_text = search_text;
        }

        // Content area
        let content_top = rect.top() + header_h + search_h + pad * 2.0;
        let content_rect = Rect::from_min_size(
            Pos2::new(rect.left(), content_top),
            Vec2::new(rect.width(), rect.bottom() - content_top - pad),
        );

        // Group presets by category
        let filter = self.search_text.to_lowercase();
        let mut categories: Vec<(String, Vec<(usize, &MaterialPreset)>)> = Vec::new();

        for (i, preset) in self.presets.iter().enumerate() {
            if !filter.is_empty()
                && !preset.name.to_lowercase().contains(&filter)
                && !preset.category.to_lowercase().contains(&filter)
            {
                continue;
            }
            if let Some(entry) = categories.iter_mut().find(|(c, _)| *c == preset.category) {
                entry.1.push((i, preset));
            } else {
                categories.push((preset.category.clone(), vec![(i, preset)]));
            }
        }

        // Draw categories and presets
        let mut y = content_rect.top();
        let category_h = 22.0 * h_scale;
        let preset_h = 36.0 * h_scale;

        for (category, presets) in &categories {
            if y > content_rect.bottom() {
                break;
            }

            let is_expanded = self
                .expanded_categories
                .get(category)
                .copied()
                .unwrap_or(true);

            // Category header
            let header_rect = Rect::from_min_size(
                Pos2::new(content_rect.left() + pad, y),
                Vec2::new(content_rect.width() - pad * 2.0, category_h),
            );

            let arrow = if is_expanded { "▼" } else { "▶" };
            let header_text = format!("{} {}", arrow, category);

            let resp = ui.put(
                header_rect,
                egui::Label::new(
                    egui::RichText::new(&header_text)
                        .font(egui::FontId::proportional(11.0 * h_scale))
                        .color(Color32::from_gray(140)),
                ),
            );

            if resp.clicked() {
                let entry = self
                    .expanded_categories
                    .entry(category.clone())
                    .or_insert(true);
                *entry = !*entry;
            }

            y += category_h;

            if is_expanded {
                for &(idx, preset) in presets {
                    if y + preset_h > content_rect.bottom() {
                        break;
                    }

                    let preset_rect = Rect::from_min_size(
                        Pos2::new(content_rect.left() + pad * 2.0, y),
                        Vec2::new(content_rect.width() - pad * 4.0, preset_h),
                    );

                    let is_selected = self.selected_preset == Some(idx);
                    let item_resp = ui.allocate_rect(preset_rect, egui::Sense::click());

                    // Background
                    if is_selected {
                        painter.add(Shape::rect_filled(
                            preset_rect,
                            Rounding::same(4.0),
                            Color32::from_rgb(0, 80, 140),
                        ));
                    } else if item_resp.hovered() {
                        painter.add(Shape::rect_filled(
                            preset_rect,
                            Rounding::same(4.0),
                            Color32::from_rgb(35, 35, 42),
                        ));
                    }

                    // Color swatch
                    let swatch_size = 20.0 * h_scale;
                    let swatch_rect = Rect::from_min_size(
                        Pos2::new(
                            preset_rect.left() + 4.0,
                            preset_rect.center().y - swatch_size / 2.0,
                        ),
                        Vec2::new(swatch_size, swatch_size),
                    );
                    let c = &preset.params.base_color;
                    painter.add(Shape::rect_filled(
                        swatch_rect,
                        Rounding::same(3.0),
                        Color32::from_rgb(
                            (c[0] * 255.0) as u8,
                            (c[1] * 255.0) as u8,
                            (c[2] * 255.0) as u8,
                        ),
                    ));
                    painter.add(Shape::rect_stroke(
                        swatch_rect,
                        Rounding::same(3.0),
                        Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 70)),
                    ));

                    // Name
                    painter.text(
                        Pos2::new(
                            preset_rect.left() + swatch_size + 10.0,
                            preset_rect.center().y,
                        ),
                        egui::Align2::LEFT_CENTER,
                        &preset.name,
                        egui::FontId::proportional(11.0 * h_scale),
                        if is_selected {
                            Color32::WHITE
                        } else {
                            Color32::from_gray(180)
                        },
                    );

                    // Metallic/Roughness hint
                    let hint = format!(
                        "M:{:.1} R:{:.1}",
                        preset.params.metallic, preset.params.roughness
                    );
                    painter.text(
                        Pos2::new(preset_rect.right() - 4.0, preset_rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        hint,
                        egui::FontId::proportional(9.0 * h_scale),
                        Color32::from_gray(80),
                    );

                    if item_resp.clicked() {
                        self.selected_preset = Some(idx);
                    }

                    y += preset_h;
                }
            }

            y += 4.0; // spacing between categories
        }
    }

    /// Get the currently selected preset's material params.
    pub fn selected_params(&self) -> Option<&MaterialParams> {
        self.selected_preset
            .and_then(|idx| self.presets.get(idx))
            .map(|p| &p.params)
    }

    /// Save the current material params as a new custom preset.
    pub fn save_preset(&mut self, name: String, params: MaterialParams) {
        self.presets.push(MaterialPreset {
            name,
            params,
            category: "Custom".to_string(),
        });
    }
}

fn default_presets() -> Vec<MaterialPreset> {
    vec![
        MaterialPreset {
            name: "默认".into(),
            params: MaterialParams {
                base_color: [0.8, 0.8, 0.8, 1.0],
                metallic: 0.0,
                roughness: 0.5,
                ..Default::default()
            },
            category: "Plastic".into(),
        },
        MaterialPreset {
            name: "光滑塑料".into(),
            params: MaterialParams {
                base_color: [0.9, 0.9, 0.9, 1.0],
                metallic: 0.0,
                roughness: 0.1,
                ..Default::default()
            },
            category: "Plastic".into(),
        },
        MaterialPreset {
            name: "粗糙塑料".into(),
            params: MaterialParams {
                base_color: [0.6, 0.6, 0.6, 1.0],
                metallic: 0.0,
                roughness: 0.8,
                ..Default::default()
            },
            category: "Plastic".into(),
        },
        MaterialPreset {
            name: "钢".into(),
            params: MaterialParams {
                base_color: [0.7, 0.7, 0.75, 1.0],
                metallic: 1.0,
                roughness: 0.2,
                ..Default::default()
            },
            category: "Metal".into(),
        },
        MaterialPreset {
            name: "金".into(),
            params: MaterialParams {
                base_color: [1.0, 0.84, 0.0, 1.0],
                metallic: 1.0,
                roughness: 0.1,
                ..Default::default()
            },
            category: "Metal".into(),
        },
        MaterialPreset {
            name: "铜".into(),
            params: MaterialParams {
                base_color: [0.95, 0.64, 0.54, 1.0],
                metallic: 1.0,
                roughness: 0.25,
                ..Default::default()
            },
            category: "Metal".into(),
        },
        MaterialPreset {
            name: "铝".into(),
            params: MaterialParams {
                base_color: [0.9, 0.9, 0.92, 1.0],
                metallic: 1.0,
                roughness: 0.35,
                ..Default::default()
            },
            category: "Metal".into(),
        },
        MaterialPreset {
            name: "橡木".into(),
            params: MaterialParams {
                base_color: [0.65, 0.45, 0.25, 1.0],
                metallic: 0.0,
                roughness: 0.7,
                ..Default::default()
            },
            category: "Wood".into(),
        },
        MaterialPreset {
            name: "松木".into(),
            params: MaterialParams {
                base_color: [0.8, 0.65, 0.4, 1.0],
                metallic: 0.0,
                roughness: 0.6,
                ..Default::default()
            },
            category: "Wood".into(),
        },
        MaterialPreset {
            name: "透明玻璃".into(),
            params: MaterialParams {
                base_color: [0.95, 0.95, 0.95, 0.3],
                metallic: 0.0,
                roughness: 0.0,
                ..Default::default()
            },
            category: "Glass".into(),
        },
        MaterialPreset {
            name: "有色玻璃".into(),
            params: MaterialParams {
                base_color: [0.2, 0.5, 0.8, 0.5],
                metallic: 0.0,
                roughness: 0.05,
                ..Default::default()
            },
            category: "Glass".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_library_default() {
        let lib = MaterialLibrary::new();
        assert!(!lib.presets.is_empty());
        assert!(lib.selected_preset.is_none());
        assert!(lib.search_text.is_empty());
    }

    #[test]
    fn test_material_library_selected_params() {
        let mut lib = MaterialLibrary::new();
        assert!(lib.selected_params().is_none());
        lib.selected_preset = Some(0);
        assert!(lib.selected_params().is_some());
    }

    #[test]
    fn test_material_library_save_preset() {
        let mut lib = MaterialLibrary::new();
        let count_before = lib.presets.len();
        lib.save_preset("Test".into(), MaterialParams::default());
        assert_eq!(lib.presets.len(), count_before + 1);
        assert_eq!(lib.presets.last().unwrap().name, "Test");
        assert_eq!(lib.presets.last().unwrap().category, "Custom");
    }

    #[test]
    fn test_default_presets_have_categories() {
        let presets = default_presets();
        let categories: std::collections::HashSet<&str> =
            presets.iter().map(|p| p.category.as_str()).collect();
        assert!(categories.contains("Metal"));
        assert!(categories.contains("Plastic"));
        assert!(categories.contains("Wood"));
        assert!(categories.contains("Glass"));
    }
}
