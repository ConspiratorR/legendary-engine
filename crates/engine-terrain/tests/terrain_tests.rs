use engine_terrain::brush::apply_sculpt_brush;
use engine_terrain::components::{
    BrushFalloff, BrushSettings, PaintBrushSettings, PaintMode, SplatMap, SculptMode, Terrain,
    TerrainTextureLayers, VegetationData, VegetationType,
};
use engine_terrain::paint::apply_paint_brush;
use engine_terrain::vegetation::regenerate_vegetation;
use engine_math::{Vec2, Vec3};

const EPSILON: f32 = 1e-5;

fn flat_terrain() -> Terrain {
    Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0)
}

// ---- Terrain creation ----

#[test]
fn terrain_creation_dimensions() {
    let terrain = flat_terrain();
    assert_eq!(terrain.resolution, 4);
    assert_eq!(terrain.chunk_size, 2);
    assert_eq!(terrain.chunk_count(), 2);
}

#[test]
fn terrain_creation_heightmap_size() {
    let terrain = flat_terrain();
    // (resolution + 1)^2
    assert_eq!(terrain.heightmap.len(), 25);
}

#[test]
fn terrain_creation_flat() {
    let terrain = flat_terrain();
    for h in &terrain.heightmap {
        assert!((h - 0.0).abs() < EPSILON);
    }
}

// ---- Height get/set ----

#[test]
fn set_then_get_height() {
    let mut terrain = flat_terrain();
    terrain.set_height(2, 3, 0.5);
    // get_height returns raw * height_scale
    assert!((terrain.get_height(2, 3) - 5.0).abs() < EPSILON);
}

#[test]
fn get_height_out_of_bounds_returns_zero() {
    let terrain = flat_terrain();
    assert!((terrain.get_height(10, 10) - 0.0).abs() < EPSILON);
}

#[test]
fn set_height_out_of_bounds_is_noop() {
    let mut terrain = flat_terrain();
    terrain.set_height(10, 10, 999.0);
    // heightmap unchanged
    for h in &terrain.heightmap {
        assert!((h - 0.0).abs() < EPSILON);
    }
}

#[test]
fn height_scale_multiplier() {
    let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 25.0);
    terrain.set_height(0, 0, 2.0);
    assert!((terrain.get_height(0, 0) - 50.0).abs() < EPSILON);
}

// ---- Paint layer ----

#[test]
fn paint_shifts_weight_to_target_layer() {
    let mut splat = SplatMap::new(4);
    // Initially all weight on layer 0
    assert_eq!(splat.get_weights(0, 0), [255, 0, 0, 0]);

    splat.paint(0, 0, 1, 1.0);
    let w = splat.get_weights(0, 0);
    assert!(w[1] > 0, "layer 1 weight should increase");
    assert!(w[0] < 255, "layer 0 weight should decrease");
}

#[test]
fn paint_with_zero_strength_is_noop() {
    let mut splat = SplatMap::new(4);
    splat.paint(0, 0, 1, 0.0);
    assert_eq!(splat.get_weights(0, 0), [255, 0, 0, 0]);
}

#[test]
fn paint_layer_above_3_is_ignored() {
    let mut splat = SplatMap::new(4);
    splat.paint(0, 0, 4, 1.0);
    assert_eq!(splat.get_weights(0, 0), [255, 0, 0, 0]);
}

#[test]
fn erase_redistributes_weight_to_layer0() {
    let mut splat = SplatMap::new(4);
    splat.paint(0, 0, 1, 1.0);
    let w_after_paint = splat.get_weights(0, 0);
    assert!(w_after_paint[1] > 0);

    splat.erase(0, 0, 1, 1.0);
    let w_after_erase = splat.get_weights(0, 0);
    assert_eq!(w_after_erase[1], 0);
    assert!(w_after_erase[0] > 0, "weight returns to layer 0");
}

// ---- Multiple layers ----

#[test]
fn paint_multiple_layers_preserves_sum() {
    let mut splat = SplatMap::new(4);
    splat.paint(0, 0, 1, 0.5);
    splat.paint(0, 0, 2, 0.5);
    let w = splat.get_weights(0, 0);
    let sum: u32 = w.iter().map(|&v| v as u32).sum();
    // Weights should roughly sum to 255 (± rounding)
    assert!(
        (255 - sum as i32).unsigned_abs() <= 4,
        "weights sum to {sum}, expected ~255"
    );
}

#[test]
fn texture_layers_add_and_remove() {
    let mut layers = TerrainTextureLayers::default();
    assert_eq!(layers.layers.len(), 1);

    layers.add_layer("Grass".to_string());
    layers.add_layer("Rock".to_string());
    assert_eq!(layers.layers.len(), 3);
    assert_eq!(layers.layers[1].name, "Grass");
    assert_eq!(layers.layers[2].name, "Rock");

    layers.remove_layer(1);
    assert_eq!(layers.layers.len(), 2);
    assert_eq!(layers.layers[1].name, "Rock");
}

#[test]
fn cannot_remove_base_layer() {
    let mut layers = TerrainTextureLayers::default();
    layers.remove_layer(0);
    assert_eq!(layers.layers.len(), 1, "base layer should remain");
}

#[test]
fn apply_paint_brush_via_sculpt_system() {
    let terrain = flat_terrain();
    let mut splat = SplatMap::new(terrain.resolution);
    let brush = PaintBrushSettings {
        radius: 10.0,
        strength: 1.0,
        falloff: BrushFalloff::Constant,
        target_layer: 2,
        mode: PaintMode::Paint,
    };
    apply_paint_brush(&terrain, &mut splat, Vec3::new(0.0, 0.0, 0.0), &brush);
    let w = splat.get_weights(2, 2);
    assert!(w[2] > 0, "layer 2 should have weight after brush");
}

// ---- Sculpt ----

#[test]
fn sculpt_raise_increases_height() {
    let mut terrain = flat_terrain();
    let brush = BrushSettings {
        radius: 10.0,
        strength: 1.0,
        falloff: BrushFalloff::Constant,
    };
    apply_sculpt_brush(
        &mut terrain,
        Vec3::new(0.0, 0.0, 0.0),
        &brush,
        SculptMode::Raise,
        0.016,
    );
    let center = terrain.get_height(2, 2);
    assert!(center > 0.0, "height should increase after raise brush, got {center}");
}

#[test]
fn sculpt_lower_decreases_height() {
    let mut terrain = flat_terrain();
    terrain.set_height(2, 2, 0.5);
    let brush = BrushSettings {
        radius: 10.0,
        strength: 1.0,
        falloff: BrushFalloff::Constant,
    };
    apply_sculpt_brush(
        &mut terrain,
        Vec3::new(0.0, 0.0, 0.0),
        &brush,
        SculptMode::Lower,
        0.016,
    );
    let center = terrain.get_height(2, 2);
    assert!(center < 5.0, "height should decrease after lower brush, got {center}");
}

#[test]
fn sculpt_flatten_converges_to_center() {
    let mut terrain = flat_terrain();
    terrain.set_height(2, 2, 0.5);
    terrain.set_height(1, 1, 0.1);

    let brush = BrushSettings {
        radius: 10.0,
        strength: 1.0,
        falloff: BrushFalloff::Constant,
    };
    apply_sculpt_brush(
        &mut terrain,
        Vec3::new(0.0, 0.0, 0.0),
        &brush,
        SculptMode::Flatten,
        0.016,
    );
    // After flatten with full strength, all vertices in radius should be very close
    let h_center = terrain.heightmap[2 * 5 + 2]; // raw value
    let h_corner = terrain.heightmap[1 * 5 + 1]; // raw value
    assert!(
        (h_center - h_corner).abs() < 0.5,
        "flatten should converge heights: center={h_center}, corner={h_corner}"
    );
}

#[test]
fn sculpt_smooth_reduces_variance() {
    let mut terrain = flat_terrain();
    // Create a spike
    terrain.set_height(2, 2, 1.0);
    terrain.set_height(1, 1, 0.0);
    terrain.set_height(3, 3, 0.0);

    let brush = BrushSettings {
        radius: 10.0,
        strength: 1.0,
        falloff: BrushFalloff::Constant,
    };
    apply_sculpt_brush(
        &mut terrain,
        Vec3::new(0.0, 0.0, 0.0),
        &brush,
        SculptMode::Smooth,
        0.016,
    );
    // The spike should be reduced
    let h_after = terrain.heightmap[2 * 5 + 2];
    assert!(h_after < 1.0, "smooth should reduce spike, got {h_after}");
}

#[test]
fn sculpt_marks_dirty_chunks() {
    let mut terrain = flat_terrain();
    let brush = BrushSettings {
        radius: 5.0,
        strength: 0.5,
        falloff: BrushFalloff::Linear,
    };
    apply_sculpt_brush(
        &mut terrain,
        Vec3::new(0.0, 0.0, 0.0),
        &brush,
        SculptMode::Raise,
        0.016,
    );
    assert!(!terrain.dirty_chunks.is_empty(), "sculpt should mark chunks dirty");
}

#[test]
fn brush_falloff_weights() {
    // Linear: 1 at center, 0 at edge
    assert!((BrushFalloff::Linear.weight(0.0) - 1.0).abs() < EPSILON);
    assert!((BrushFalloff::Linear.weight(1.0) - 0.0).abs() < EPSILON);

    // Smooth: ~1 at center, ~0 at edge
    assert!((BrushFalloff::Smooth.weight(0.0) - 1.0).abs() < EPSILON);
    assert!(BrushFalloff::Smooth.weight(1.0) < 0.01);

    // Constant: 1 inside, 0 outside
    assert!((BrushFalloff::Constant.weight(0.5) - 1.0).abs() < EPSILON);
    assert!((BrushFalloff::Constant.weight(1.5) - 0.0).abs() < EPSILON);
}

// ---- Vegetation ----

#[test]
fn vegetation_generation_produces_instances() {
    let mut terrain = flat_terrain();
    terrain.set_height(1, 1, 0.2);

    let mut veg = VegetationData::default();
    veg.add_type(VegetationType {
        name: "Tree".to_string(),
        density: 0.5,
        seed: 123,
        ..Default::default()
    });
    regenerate_vegetation(&terrain, &mut veg);
    assert!(!veg.instances.is_empty());
    assert!(!veg.dirty);
}

#[test]
fn vegetation_respects_height_limits() {
    let terrain = flat_terrain();
    let mut veg = VegetationData::default();
    veg.add_type(VegetationType {
        name: "HighTree".to_string(),
        density: 10.0,
        height_min: 100.0,
        seed: 42,
        ..Default::default()
    });
    regenerate_vegetation(&terrain, &mut veg);
    assert!(veg.instances.is_empty(), "no instances above height_min");
}

#[test]
fn vegetation_type_add_and_remove() {
    let mut veg = VegetationData::default();
    veg.add_type(VegetationType {
        name: "Tree".to_string(),
        ..Default::default()
    });
    veg.add_type(VegetationType {
        name: "Grass".to_string(),
        ..Default::default()
    });
    assert_eq!(veg.types.len(), 2);

    veg.remove_type(0);
    assert_eq!(veg.types.len(), 1);
    assert_eq!(veg.types[0].name, "Grass");
}
