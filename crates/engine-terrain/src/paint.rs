use engine_math::Vec3;

use crate::components::{PaintBrushSettings, PaintMode, SplatMap, Terrain};

/// Apply a texture painting brush to the terrain splat map at a world-space position.
pub fn apply_paint_brush(
    terrain: &Terrain,
    splat_map: &mut SplatMap,
    world_pos: Vec3,
    brush: &PaintBrushSettings,
) {
    let res = terrain.resolution;
    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;

    // Convert world position to heightmap grid coordinates
    let grid_x = ((world_pos.x + half_w) / terrain.world_size.x * res as f32) as i32;
    let grid_z = ((world_pos.z + half_h) / terrain.world_size.y * res as f32) as i32;

    let grid_radius = (brush.radius / terrain.world_size.x * res as f32).ceil() as i32;

    let min_i = (grid_x - grid_radius).max(0) as u32;
    let max_i = (grid_x + grid_radius).min(res as i32) as u32;
    let min_j = (grid_z - grid_radius).max(0) as u32;
    let max_j = (grid_z + grid_radius).min(res as i32) as u32;

    for j in min_j..=max_j {
        for i in min_i..=max_i {
            let dx = i as f32 - grid_x as f32;
            let dz = j as f32 - grid_z as f32;
            let dist = (dx * dx + dz * dz).sqrt();
            let normalized_dist = dist / grid_radius as f32;

            if normalized_dist > 1.0 {
                continue;
            }

            let weight = brush.falloff.weight(normalized_dist) * brush.strength;

            match brush.mode {
                PaintMode::Paint => {
                    splat_map.paint(i, j, brush.target_layer, weight);
                }
                PaintMode::Erase => {
                    splat_map.erase(i, j, brush.target_layer, weight);
                }
            }
        }
    }
}

/// Snapshot of splat map state for undo/redo.
#[derive(Debug, Clone)]
pub struct SplatMapSnapshot {
    pub data: Vec<[u8; 4]>,
}

impl SplatMapSnapshot {
    /// Capture the current splat map state for later undo.
    pub fn capture(splat_map: &SplatMap) -> Self {
        Self {
            data: splat_map.data.clone(),
        }
    }

    /// Restore a previously captured splat map state.
    pub fn restore(&self, splat_map: &mut SplatMap) {
        splat_map.data.clone_from(&self.data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::BrushFalloff;
    use engine_math::Vec2;

    fn make_test_terrain() -> Terrain {
        Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0)
    }

    #[test]
    fn test_paint_increases_layer_weight() {
        let terrain = make_test_terrain();
        let mut splat = SplatMap::new(terrain.resolution);
        let brush = PaintBrushSettings {
            radius: 10.0,
            strength: 1.0,
            falloff: BrushFalloff::Constant,
            target_layer: 1,
            mode: PaintMode::Paint,
        };
        apply_paint_brush(&terrain, &mut splat, Vec3::new(0.0, 0.0, 0.0), &brush);
        let w = splat.get_weights(2, 2);
        assert!(w[1] > 0, "Layer 1 weight should be > 0 after painting");
    }

    #[test]
    fn test_erase_redistributes_to_layer0() {
        let terrain = make_test_terrain();
        let mut splat = SplatMap::new(terrain.resolution);

        // First paint layer 1
        let paint_brush = PaintBrushSettings {
            radius: 10.0,
            strength: 1.0,
            falloff: BrushFalloff::Constant,
            target_layer: 1,
            mode: PaintMode::Paint,
        };
        apply_paint_brush(&terrain, &mut splat, Vec3::new(0.0, 0.0, 0.0), &paint_brush);

        // Then erase layer 1
        let erase_brush = PaintBrushSettings {
            radius: 10.0,
            strength: 1.0,
            falloff: BrushFalloff::Constant,
            target_layer: 1,
            mode: PaintMode::Erase,
        };
        apply_paint_brush(&terrain, &mut splat, Vec3::new(0.0, 0.0, 0.0), &erase_brush);

        let w = splat.get_weights(2, 2);
        assert_eq!(w[1], 0, "Layer 1 should be 0 after erasing");
        assert!(w[0] > 0, "Layer 0 should receive redistributed weight");
    }

    #[test]
    fn test_snapshot_restore() {
        let terrain = make_test_terrain();
        let mut splat = SplatMap::new(terrain.resolution);

        let snapshot = SplatMapSnapshot::capture(&splat);

        // Modify
        let brush = PaintBrushSettings {
            radius: 10.0,
            strength: 1.0,
            falloff: BrushFalloff::Constant,
            target_layer: 2,
            mode: PaintMode::Paint,
        };
        apply_paint_brush(&terrain, &mut splat, Vec3::new(0.0, 0.0, 0.0), &brush);
        assert!(splat.get_weights(2, 2)[2] > 0);

        // Restore
        snapshot.restore(&mut splat);
        assert_eq!(splat.get_weights(2, 2)[2], 0);
    }
}
