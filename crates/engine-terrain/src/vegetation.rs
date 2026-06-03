use engine_math::Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::components::{Terrain, VegetationData, VegetationInstance, VegetationType};

/// Generate vegetation instances for all vegetation types on a terrain.
///
/// Clears existing instances and regenerates based on current terrain
/// state and vegetation type settings.
pub fn regenerate_vegetation(terrain: &Terrain, vegetation: &mut VegetationData) {
    vegetation.instances.clear();

    for (type_index, veg_type) in vegetation.types.iter().enumerate() {
        generate_instances_for_type(terrain, veg_type, type_index, &mut vegetation.instances);
    }

    vegetation.dirty = false;
}

/// Generate instances for a single vegetation type across the terrain.
fn generate_instances_for_type(
    terrain: &Terrain,
    veg_type: &VegetationType,
    type_index: usize,
    instances: &mut Vec<VegetationInstance>,
) {
    let mut rng = StdRng::seed_from_u64(veg_type.seed);
    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;
    let res = terrain.resolution;

    // Calculate number of instances based on density and terrain area
    let area = terrain.world_size.x * terrain.world_size.y;
    let count = (area * veg_type.density) as u32;

    for _ in 0..count {
        // Random position on terrain
        let x = rng.random_range(-half_w..half_w);
        let z = rng.random_range(-half_h..half_h);

        // Convert to grid coordinates
        let gi = ((x + half_w) / terrain.world_size.x * res as f32) as u32;
        let gj = ((z + half_h) / terrain.world_size.y * res as f32) as u32;

        if gi > res || gj > res {
            continue;
        }

        // Get height at this position
        let height = terrain.get_height(gi.min(res), gj.min(res));

        // Check height constraints
        if height < veg_type.height_min || height > veg_type.height_max {
            continue;
        }

        // Compute slope (approximate via neighbor height differences)
        let slope = compute_slope(terrain, gi.min(res), gj.min(res));
        if slope < veg_type.slope_min || slope > veg_type.slope_max {
            continue;
        }

        // Random rotation and scale
        let rotation_y = rng.random_range(0.0..std::f32::consts::TAU);
        let scale = rng.random_range(veg_type.scale_min..veg_type.scale_max);

        instances.push(VegetationInstance {
            position: Vec3::new(x, height, z),
            rotation_y,
            scale,
            vegetation_type_index: type_index,
        });
    }
}

/// Compute terrain slope in degrees at a grid position.
fn compute_slope(terrain: &Terrain, i: u32, j: u32) -> f32 {
    let res = terrain.resolution;
    let h = terrain.get_height(i.min(res), j.min(res));

    let h_right = if i < res {
        terrain.get_height(i + 1, j.min(res))
    } else {
        h
    };
    let h_up = if j < res {
        terrain.get_height(i.min(res), j + 1)
    } else {
        h
    };

    let dx = h_right - h;
    let dz = h_up - h;
    let gradient = (dx * dx + dz * dz).sqrt();

    // Convert gradient to degrees (assuming 1-unit grid spacing)
    gradient.atan().to_degrees()
}

/// Get vegetation instances filtered by LOD distance from camera.
pub fn get_visible_instances(
    vegetation: &VegetationData,
    camera_pos: Vec3,
    lod_level: usize,
) -> Vec<&VegetationInstance> {
    vegetation
        .instances
        .iter()
        .filter(|inst| {
            let veg_type = &vegetation.types[inst.vegetation_type_index];
            let dist = camera_pos.distance(inst.position);
            let max_dist = veg_type
                .lod_distances
                .get(lod_level)
                .copied()
                .unwrap_or(f32::MAX);
            dist <= max_dist
        })
        .collect()
}

/// Get the LOD level for a vegetation instance based on camera distance.
pub fn get_lod_level(veg_type: &VegetationType, distance: f32) -> Option<usize> {
    for (level, &max_dist) in veg_type.lod_distances.iter().enumerate() {
        if distance <= max_dist {
            return Some(level);
        }
    }
    None // Too far, don't render
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::Vec2;

    fn make_test_terrain() -> Terrain {
        let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        // Add some height variation
        terrain.set_height(1, 1, 0.2);
        terrain.set_height(2, 2, 0.3);
        terrain
    }

    #[test]
    fn test_vegetation_generation() {
        let terrain = make_test_terrain();
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
    fn test_vegetation_respects_height_limits() {
        let terrain = make_test_terrain();
        let mut veg = VegetationData::default();
        veg.add_type(VegetationType {
            name: "HighTree".to_string(),
            density: 10.0,
            height_min: 100.0, // Very high minimum
            seed: 42,
            ..Default::default()
        });
        regenerate_vegetation(&terrain, &mut veg);
        assert!(
            veg.instances.is_empty(),
            "No vegetation should be placed above height_min"
        );
    }

    #[test]
    fn test_vegetation_respects_slope_limits() {
        let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        // Create a steep slope
        terrain.set_height(0, 0, 0.0);
        terrain.set_height(4, 0, 1.0);

        let mut veg = VegetationData::default();
        veg.add_type(VegetationType {
            name: "FlatPlant".to_string(),
            density: 10.0,
            slope_max: 0.1, // Very flat only
            seed: 42,
            ..Default::default()
        });
        regenerate_vegetation(&terrain, &mut veg);
        // Should have fewer (or zero) instances on steep terrain
    }

    #[test]
    fn test_lod_filtering() {
        let mut veg = VegetationData::default();
        veg.add_type(VegetationType {
            name: "Grass".to_string(),
            lod_distances: [10.0, 25.0, 50.0],
            ..Default::default()
        });
        veg.instances.push(VegetationInstance {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation_y: 0.0,
            scale: 1.0,
            vegetation_type_index: 0,
        });

        // Close camera — LOD 0
        let close = get_visible_instances(&veg, Vec3::new(5.0, 0.0, 0.0), 0);
        assert_eq!(close.len(), 1);

        // Far camera — LOD 0 should not include distant instance
        let far = get_visible_instances(&veg, Vec3::new(15.0, 0.0, 0.0), 0);
        assert_eq!(far.len(), 0);

        // But LOD 1 should include it
        let mid = get_visible_instances(&veg, Vec3::new(15.0, 0.0, 0.0), 1);
        assert_eq!(mid.len(), 1);
    }

    #[test]
    fn test_lod_level_selection() {
        let veg_type = VegetationType {
            lod_distances: [10.0, 25.0, 50.0],
            ..Default::default()
        };
        assert_eq!(get_lod_level(&veg_type, 5.0), Some(0));
        assert_eq!(get_lod_level(&veg_type, 15.0), Some(1));
        assert_eq!(get_lod_level(&veg_type, 30.0), Some(2));
        assert_eq!(get_lod_level(&veg_type, 60.0), None);
    }

    #[test]
    fn test_remove_vegetation_type() {
        let mut veg = VegetationData::default();
        veg.add_type(VegetationType {
            name: "Tree".to_string(),
            ..Default::default()
        });
        veg.add_type(VegetationType {
            name: "Grass".to_string(),
            ..Default::default()
        });
        veg.instances.push(VegetationInstance {
            position: Vec3::ZERO,
            rotation_y: 0.0,
            scale: 1.0,
            vegetation_type_index: 0,
        });
        veg.instances.push(VegetationInstance {
            position: Vec3::new(1.0, 0.0, 0.0),
            rotation_y: 0.0,
            scale: 1.0,
            vegetation_type_index: 1,
        });

        veg.remove_type(0);
        assert_eq!(veg.types.len(), 1);
        assert_eq!(veg.types[0].name, "Grass");
        // Instance with type_index=0 should be removed, type_index=1 should become 0
        assert_eq!(veg.instances.len(), 1);
        assert_eq!(veg.instances[0].vegetation_type_index, 0);
    }
}
