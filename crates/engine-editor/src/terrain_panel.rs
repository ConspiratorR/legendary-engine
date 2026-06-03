use engine_terrain::components::{
    BrushFalloff, PaintBrushSettings, PaintMode, SculptMode, Terrain, TerrainTextureLayers,
    VegetationData, VegetationType,
};

/// Terrain editor panel for the editor UI.
///
/// Draws terrain properties, brush settings, texture layer management,
/// and vegetation tools when a terrain entity is selected.
#[derive(Debug, Clone)]
pub struct TerrainPanel {
    /// Currently selected brush mode (sculpt).
    pub sculpt_mode: SculptMode,
    /// Currently selected paint mode.
    pub paint_mode: PaintMode,
    /// Sculpt brush settings.
    pub sculpt_brush: engine_terrain::components::BrushSettings,
    /// Paint brush settings.
    pub paint_brush: PaintBrushSettings,
    /// Active editing mode.
    pub edit_mode: TerrainEditMode,
    /// Selected texture layer index for painting.
    pub selected_layer: usize,
    /// New vegetation type name input buffer.
    pub new_veg_name: String,
}

/// Terrain editing modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainEditMode {
    /// Height sculpting mode.
    Sculpt,
    /// Texture painting mode.
    Paint,
    /// Vegetation placement mode.
    Vegetation,
}

impl Default for TerrainPanel {
    fn default() -> Self {
        Self {
            sculpt_mode: SculptMode::Raise,
            paint_mode: PaintMode::Paint,
            sculpt_brush: Default::default(),
            paint_brush: Default::default(),
            edit_mode: TerrainEditMode::Sculpt,
            selected_layer: 0,
            new_veg_name: String::new(),
        }
    }
}

impl TerrainPanel {
    /// Draw the terrain panel UI using egui.
    ///
    /// `terrain` is the currently selected terrain entity's component.
    /// `texture_layers` and `vegetation_data` are global resources.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        terrain: &mut Terrain,
        texture_layers: &mut TerrainTextureLayers,
        vegetation_data: &mut VegetationData,
    ) {
        ui.heading("Terrain Editor");

        // Mode selector
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.edit_mode, TerrainEditMode::Sculpt, "Sculpt");
            ui.selectable_value(&mut self.edit_mode, TerrainEditMode::Paint, "Paint");
            ui.selectable_value(
                &mut self.edit_mode,
                TerrainEditMode::Vegetation,
                "Vegetation",
            );
        });

        ui.separator();

        match self.edit_mode {
            TerrainEditMode::Sculpt => self.draw_sculpt_panel(ui, terrain),
            TerrainEditMode::Paint => self.draw_paint_panel(ui, texture_layers),
            TerrainEditMode::Vegetation => self.draw_vegetation_panel(ui, vegetation_data),
        }
    }

    fn draw_sculpt_panel(&mut self, ui: &mut egui::Ui, terrain: &mut Terrain) {
        ui.label("Terrain Properties");
        ui.horizontal(|ui| {
            ui.label("Resolution:");
            ui.label(format!("{}", terrain.resolution));
        });
        ui.horizontal(|ui| {
            ui.label("World Size:");
            ui.label(format!(
                "{:.1} x {:.1}",
                terrain.world_size.x, terrain.world_size.y
            ));
        });
        ui.horizontal(|ui| {
            ui.label("Height Scale:");
            ui.add(egui::Slider::new(&mut terrain.height_scale, 0.1..=200.0));
        });

        ui.separator();
        ui.label("Brush Settings");

        egui::ComboBox::from_label("Mode")
            .selected_text(format!("{:?}", self.sculpt_mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.sculpt_mode, SculptMode::Raise, "Raise");
                ui.selectable_value(&mut self.sculpt_mode, SculptMode::Lower, "Lower");
                ui.selectable_value(&mut self.sculpt_mode, SculptMode::Smooth, "Smooth");
                ui.selectable_value(&mut self.sculpt_mode, SculptMode::Flatten, "Flatten");
            });

        ui.add(egui::Slider::new(&mut self.sculpt_brush.radius, 0.5..=100.0).text("Radius"));
        ui.add(egui::Slider::new(&mut self.sculpt_brush.strength, 0.01..=1.0).text("Strength"));

        egui::ComboBox::from_label("Falloff")
            .selected_text(format!("{:?}", self.sculpt_brush.falloff))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.sculpt_brush.falloff,
                    BrushFalloff::Linear,
                    "Linear",
                );
                ui.selectable_value(
                    &mut self.sculpt_brush.falloff,
                    BrushFalloff::Smooth,
                    "Smooth",
                );
                ui.selectable_value(
                    &mut self.sculpt_brush.falloff,
                    BrushFalloff::Constant,
                    "Constant",
                );
            });
    }

    fn draw_paint_panel(&mut self, ui: &mut egui::Ui, texture_layers: &mut TerrainTextureLayers) {
        ui.label("Texture Layers");

        // Layer list
        let mut remove_idx = None;
        for (i, layer) in texture_layers.layers.iter().enumerate() {
            ui.horizontal(|ui| {
                let selected = self.selected_layer == i;
                if ui.selectable_label(selected, &layer.name).clicked() {
                    self.selected_layer = i;
                }
                if i > 0 && ui.small_button("-").clicked() {
                    remove_idx = Some(i);
                }
            });
        }

        if let Some(idx) = remove_idx {
            texture_layers.remove_layer(idx);
            if self.selected_layer >= texture_layers.layers.len() {
                self.selected_layer = texture_layers.layers.len() - 1;
            }
        }

        // Add layer button
        ui.horizontal(|ui| {
            let layer_count = texture_layers.layers.len();
            if ui.button("Add Layer").clicked() && layer_count < 4 {
                texture_layers.add_layer(format!("Layer {}", layer_count));
            }
            if layer_count >= 4 {
                ui.label("(max 4 layers)");
            }
        });

        ui.separator();
        ui.label("Paint Brush");

        egui::ComboBox::from_label("Mode")
            .selected_text(match self.paint_mode {
                PaintMode::Paint => "Paint",
                PaintMode::Erase => "Erase",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.paint_mode, PaintMode::Paint, "Paint");
                ui.selectable_value(&mut self.paint_mode, PaintMode::Erase, "Erase");
            });

        self.paint_brush.target_layer = self.selected_layer;
        self.paint_brush.mode = self.paint_mode;

        ui.add(egui::Slider::new(&mut self.paint_brush.radius, 0.5..=100.0).text("Radius"));
        ui.add(egui::Slider::new(&mut self.paint_brush.strength, 0.01..=1.0).text("Strength"));

        egui::ComboBox::from_label("Falloff")
            .selected_text(format!("{:?}", self.paint_brush.falloff))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.paint_brush.falloff,
                    BrushFalloff::Linear,
                    "Linear",
                );
                ui.selectable_value(
                    &mut self.paint_brush.falloff,
                    BrushFalloff::Smooth,
                    "Smooth",
                );
                ui.selectable_value(
                    &mut self.paint_brush.falloff,
                    BrushFalloff::Constant,
                    "Constant",
                );
            });
    }

    fn draw_vegetation_panel(&mut self, ui: &mut egui::Ui, vegetation_data: &mut VegetationData) {
        ui.label("Vegetation Types");

        let mut remove_idx = None;
        for (i, veg_type) in vegetation_data.types.iter().enumerate() {
            ui.collapsing(&veg_type.name, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Density:");
                    ui.label(format!("{:.2}", veg_type.density));
                });
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    ui.label(format!(
                        "{:.1} - {:.1}",
                        veg_type.scale_min, veg_type.scale_max
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Slope:");
                    ui.label(format!(
                        "{:.0}° - {:.0}°",
                        veg_type.slope_min, veg_type.slope_max
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("LOD:");
                    ui.label(format!(
                        "{:.0} / {:.0} / {:.0}",
                        veg_type.lod_distances[0],
                        veg_type.lod_distances[1],
                        veg_type.lod_distances[2]
                    ));
                });
                if ui.button("Remove").clicked() {
                    remove_idx = Some(i);
                }
            });
        }

        if let Some(idx) = remove_idx {
            vegetation_data.remove_type(idx);
        }

        ui.separator();

        // Add new vegetation type
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_veg_name);
            if ui.button("Add Type").clicked() && !self.new_veg_name.is_empty() {
                vegetation_data.add_type(VegetationType {
                    name: self.new_veg_name.clone(),
                    ..Default::default()
                });
                self.new_veg_name.clear();
            }
        });

        ui.separator();

        if ui.button("Regenerate All").clicked() {
            vegetation_data.dirty = true;
        }

        ui.label(format!(
            "Total instances: {}",
            vegetation_data.instances.len()
        ));
    }
}
