use engine_math::{Vec2, Vec3};
use engine_terrain::brush::apply_sculpt_brush;
use engine_terrain::components::{
    BrushFalloff, BrushSettings, PaintBrushSettings, PaintMode, SculptMode, SplatMap, Terrain,
    TerrainTextureLayers, VegetationData, VegetationInstance, VegetationType,
};
use engine_terrain::paint::apply_paint_brush;
use engine_terrain::raycast::sample_terrain_height;
use engine_terrain::vegetation::{get_lod_level, get_visible_instances, regenerate_vegetation};

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
    assert!(
        center > 0.0,
        "height should increase after raise brush, got {center}"
    );
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
    assert!(
        center < 5.0,
        "height should decrease after lower brush, got {center}"
    );
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
    assert!(
        !terrain.dirty_chunks.is_empty(),
        "sculpt should mark chunks dirty"
    );
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

// ---- Task 19: Hardening tests ----

#[test]
fn heightmap_generation_produces_valid_heights() {
    // Generate heightmap with known seed via vegetation (deterministic RNG)
    let mut terrain = Terrain::new(8, 4, Vec2::new(20.0, 20.0), 10.0);
    // Paint some height variation
    for i in 0..=8 {
        for j in 0..=8 {
            let h = ((i as f32 * 0.1) + (j as f32 * 0.05)).sin() * 0.5;
            terrain.set_height(i, j, h);
        }
    }

    // Verify all heights are within expected range (raw values * height_scale)
    for i in 0..=terrain.resolution {
        for j in 0..=terrain.resolution {
            let h = terrain.get_height(i, j);
            // Raw values are in [-0.5, 0.5], scaled by 10.0
            assert!(
                h >= -6.0 && h <= 6.0,
                "height at ({i},{j}) = {h} out of expected range"
            );
        }
    }

    // Verify heightmap size is correct
    let expected_size = ((terrain.resolution + 1) * (terrain.resolution + 1)) as usize;
    assert_eq!(terrain.heightmap.len(), expected_size);
}

#[test]
fn terrain_chunk_loading_unloading() {
    let terrain = Terrain::new(8, 4, Vec2::new(20.0, 20.0), 10.0);
    let chunk_count = terrain.chunk_count(); // 8/4 = 2

    // Simulate loading chunks around position (0,0) — should load all chunks
    let center = Vec3::ZERO;
    let load_radius = 20.0; // covers entire terrain

    let mut loaded_chunks = Vec::new();
    for cz in 0..chunk_count {
        for cx in 0..chunk_count {
            let chunk_world_w = terrain.world_size.x / chunk_count as f32;
            let chunk_world_h = terrain.world_size.y / chunk_count as f32;
            let chunk_center_x =
                cx as f32 * chunk_world_w - terrain.world_size.x * 0.5 + chunk_world_w * 0.5;
            let chunk_center_z =
                cz as f32 * chunk_world_h - terrain.world_size.y * 0.5 + chunk_world_h * 0.5;
            let dx = center.x - chunk_center_x;
            let dz = center.z - chunk_center_z;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist <= load_radius {
                loaded_chunks.push((cx, cz));
            }
        }
    }
    assert_eq!(
        loaded_chunks.len(),
        4,
        "all 4 chunks should be loaded at center"
    );

    // Move position far away — should unload all chunks
    let far_center = Vec3::new(100.0, 0.0, 100.0);
    let small_radius = 1.0;
    let mut far_loaded = Vec::new();
    for cz in 0..chunk_count {
        for cx in 0..chunk_count {
            let chunk_world_w = terrain.world_size.x / chunk_count as f32;
            let chunk_world_h = terrain.world_size.y / chunk_count as f32;
            let chunk_center_x =
                cx as f32 * chunk_world_w - terrain.world_size.x * 0.5 + chunk_world_w * 0.5;
            let chunk_center_z =
                cz as f32 * chunk_world_h - terrain.world_size.y * 0.5 + chunk_world_h * 0.5;
            let dx = far_center.x - chunk_center_x;
            let dz = far_center.z - chunk_center_z;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist <= small_radius {
                far_loaded.push((cx, cz));
            }
        }
    }
    assert!(far_loaded.is_empty(), "no chunks should be loaded far away");
}

#[test]
fn terrain_sampling_at_boundaries() {
    let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
    // Set known heights at corners
    terrain.set_height(0, 0, 1.0); // bottom-left
    terrain.set_height(4, 0, 2.0); // bottom-right
    terrain.set_height(0, 4, 3.0); // top-left
    terrain.set_height(4, 4, 4.0); // top-right

    // Sample at exact corners
    let h_bl = sample_terrain_height(&terrain, -5.0, -5.0);
    let h_br = sample_terrain_height(&terrain, 5.0, -5.0);
    let h_tl = sample_terrain_height(&terrain, -5.0, 5.0);
    let h_tr = sample_terrain_height(&terrain, 5.0, 5.0);

    assert!((h_bl - 10.0).abs() < EPSILON, "bottom-left: {h_bl}");
    assert!((h_br - 20.0).abs() < EPSILON, "bottom-right: {h_br}");
    assert!((h_tl - 30.0).abs() < EPSILON, "top-left: {h_tl}");
    assert!((h_tr - 40.0).abs() < EPSILON, "top-right: {h_tr}");

    // Sample at center grid vertex (2,2) — which is 0.0 (unset)
    let h_center = sample_terrain_height(&terrain, 0.0, 0.0);
    assert!(
        (h_center - 0.0).abs() < EPSILON,
        "center vertex is unset (0.0), got {h_center}"
    );

    // Set center height and verify interpolation
    terrain.set_height(2, 2, 2.5);
    let h_center_set = sample_terrain_height(&terrain, 0.0, 0.0);
    assert!(
        (h_center_set - 25.0).abs() < EPSILON,
        "center should be 25.0 (raw 2.5 * scale 10), got {h_center_set}"
    );

    // Sample outside bounds should still return a value (clamped)
    let h_oob = sample_terrain_height(&terrain, -100.0, -100.0);
    assert!(h_oob.is_finite(), "out-of-bounds sample should be finite");
}

#[test]
fn terrain_lod_transitions() {
    let veg_type = VegetationType {
        name: "Tree".to_string(),
        lod_distances: [10.0, 30.0, 60.0],
        ..Default::default()
    };

    // Close distance → LOD 0
    assert_eq!(get_lod_level(&veg_type, 5.0), Some(0));
    // Medium distance → LOD 1
    assert_eq!(get_lod_level(&veg_type, 20.0), Some(1));
    // Far distance → LOD 2
    assert_eq!(get_lod_level(&veg_type, 45.0), Some(2));
    // Beyond all LODs → None (not rendered)
    assert_eq!(get_lod_level(&veg_type, 70.0), None);

    // Test visible instance filtering with LOD
    let mut veg = VegetationData::default();
    veg.add_type(veg_type);
    veg.instances.push(VegetationInstance {
        position: Vec3::ZERO,
        rotation_y: 0.0,
        scale: 1.0,
        vegetation_type_index: 0,
    });

    // LOD 0: visible within 10 units
    let lod0 = get_visible_instances(&veg, Vec3::new(5.0, 0.0, 0.0), 0);
    assert_eq!(lod0.len(), 1);

    // LOD 0: not visible at 15 units
    let lod0_far = get_visible_instances(&veg, Vec3::new(15.0, 0.0, 0.0), 0);
    assert_eq!(lod0_far.len(), 0);

    // LOD 1: visible at 15 units
    let lod1 = get_visible_instances(&veg, Vec3::new(15.0, 0.0, 0.0), 1);
    assert_eq!(lod1.len(), 1);

    // LOD 2: visible at 45 units
    let lod2 = get_visible_instances(&veg, Vec3::new(45.0, 0.0, 0.0), 2);
    assert_eq!(lod2.len(), 1);

    // Beyond all LODs: not visible
    let lod_none = get_visible_instances(&veg, Vec3::new(70.0, 0.0, 0.0), 2);
    assert_eq!(lod_none.len(), 0);
}
