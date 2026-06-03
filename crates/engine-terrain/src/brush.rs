use engine_math::Vec3;

use crate::components::{BrushSettings, SculptMode, Terrain};

/// Apply a sculpting brush to the terrain at a world-space position.
///
/// Modifies the heightmap and marks affected chunks as dirty.
pub fn apply_sculpt_brush(
    terrain: &mut Terrain,
    world_pos: Vec3,
    brush: &BrushSettings,
    mode: SculptMode,
    dt: f32,
) {
    let res = terrain.resolution;
    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;

    // Convert world position to heightmap grid coordinates
    let grid_x = ((world_pos.x + half_w) / terrain.world_size.x * res as f32) as i32;
    let grid_z = ((world_pos.z + half_h) / terrain.world_size.y * res as f32) as i32;

    // Compute grid-space radius
    let grid_radius = (brush.radius / terrain.world_size.x * res as f32).ceil() as i32;

    let min_i = (grid_x - grid_radius).max(0) as u32;
    let max_i = (grid_x + grid_radius).min(res as i32) as u32;
    let min_j = (grid_z - grid_radius).max(0) as u32;
    let max_j = (grid_z + grid_radius).min(res as i32) as u32;

    // For flatten mode, sample the center height first
    let target_height = if mode == SculptMode::Flatten {
        let ci = grid_x.max(0).min(res as i32) as u32;
        let cj = grid_z.max(0).min(res as i32) as u32;
        terrain.heightmap[(cj * (res + 1) + ci) as usize]
    } else {
        0.0
    };

    // For smooth mode, snapshot neighbor heights first
    let snapshot = if mode == SculptMode::Smooth {
        Some(terrain.heightmap.clone())
    } else {
        None
    };

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
            let idx = (j * (res + 1) + i) as usize;

            match mode {
                SculptMode::Raise => {
                    terrain.heightmap[idx] += weight * dt;
                }
                SculptMode::Lower => {
                    terrain.heightmap[idx] -= weight * dt;
                }
                SculptMode::Smooth => {
                    if let Some(ref snap) = snapshot {
                        let avg = average_neighbors(snap, res, i, j);
                        terrain.heightmap[idx] = lerp(terrain.heightmap[idx], avg, weight);
                    }
                }
                SculptMode::Flatten => {
                    terrain.heightmap[idx] = lerp(terrain.heightmap[idx], target_height, weight);
                }
            }
        }
    }

    // Mark affected chunks as dirty
    terrain.mark_dirty_region(world_pos, brush.radius);
}

/// Get the average height of neighboring vertices.
fn average_neighbors(heightmap: &[f32], res: u32, i: u32, j: u32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0;

    for dj in -1i32..=1 {
        for di in -1i32..=1 {
            if di == 0 && dj == 0 {
                continue;
            }
            let ni = i as i32 + di;
            let nj = j as i32 + dj;
            if ni >= 0 && ni <= res as i32 && nj >= 0 && nj <= res as i32 {
                let idx = (nj as u32 * (res + 1) + ni as u32) as usize;
                sum += heightmap[idx];
                count += 1;
            }
        }
    }

    if count > 0 {
        sum / count as f32
    } else {
        heightmap[(j * (res + 1) + i) as usize]
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::BrushFalloff;
    use engine_math::Vec2;

    #[test]
    fn test_raise_brush() {
        let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
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
        // Center vertex should have increased height
        let center = terrain.get_height(2, 2);
        assert!(center > 0.0);
    }

    #[test]
    fn test_lower_brush() {
        let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        // Set some initial height
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
        assert!(center < 5.0); // height_scale=10, so 0.5*10=5.0
    }

    #[test]
    fn test_flatten_brush() {
        let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
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
        // All heights should converge toward the center height
    }

    #[test]
    fn test_dirty_chunks_marked() {
        let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
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
        assert!(!terrain.dirty_chunks.is_empty());
    }
}
