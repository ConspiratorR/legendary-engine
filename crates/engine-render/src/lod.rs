use engine_math::Vec3;

/// A single LOD (Level of Detail) entry.
///
/// Each entry maps a distance range to a mesh representation.
/// Entries should be sorted by `min_distance` ascending.
#[derive(Debug, Clone)]
pub struct LodLevel {
    /// Minimum distance at which this LOD is active.
    pub min_distance: f32,
    /// Maximum distance at which this LOD is active.
    pub max_distance: f32,
    /// Opaque identifier for the mesh at this LOD (e.g., asset handle key).
    /// The renderer interprets this to select the actual GPU mesh.
    pub mesh_id: u64,
}

impl LodLevel {
    pub fn new(min_distance: f32, max_distance: f32, mesh_id: u64) -> Self {
        Self {
            min_distance,
            max_distance,
            mesh_id,
        }
    }
}

/// LOD configuration attached as an ECS component.
///
/// Holds a sorted list of LOD levels.  The system selects the appropriate
/// level based on the distance from the camera to the entity's position.
///
/// # Example
///
/// ```ignore
/// let lod = LodConfig::new(vec![
///     LodLevel::new(0.0, 50.0, MESH_HIGH),    // LOD 0: high detail
///     LodLevel::new(50.0, 150.0, MESH_MED),   // LOD 1: medium
///     LodLevel::new(150.0, f32::MAX, MESH_LOW), // LOD 2: low detail
/// ]);
/// ```
#[derive(Debug, Clone)]
pub struct LodConfig {
    /// Sorted LOD levels (ascending by `min_distance`).
    pub levels: Vec<LodLevel>,
    /// Bias multiplier applied to the computed distance.
    /// Values > 1.0 push to lower detail sooner; < 1.0 keep higher detail longer.
    pub distance_bias: f32,
}

impl LodConfig {
    /// Create a new LOD configuration.
    ///
    /// `levels` are sorted by `min_distance` if not already sorted.
    pub fn new(mut levels: Vec<LodLevel>) -> Self {
        levels.sort_by(|a, b| {
            a.min_distance
                .partial_cmp(&b.min_distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self {
            levels,
            distance_bias: 1.0,
        }
    }

    /// Create a LOD configuration with a custom distance bias.
    pub fn with_bias(mut self, bias: f32) -> Self {
        self.distance_bias = bias;
        self
    }

    /// Select the LOD level for a given distance.
    ///
    /// Returns the `mesh_id` of the matching level, or `None` if no levels
    /// are configured.
    pub fn select(&self, distance: f32) -> Option<u64> {
        let effective = distance * self.distance_bias;
        for level in &self.levels {
            if effective >= level.min_distance && effective < level.max_distance {
                return Some(level.mesh_id);
            }
        }
        // Fallback: return the last (lowest detail) level.
        self.levels.last().map(|l| l.mesh_id)
    }

    /// Select the LOD level index for a given distance.
    ///
    /// Returns `(index, mesh_id)`.
    pub fn select_index(&self, distance: f32) -> Option<(usize, u64)> {
        let effective = distance * self.distance_bias;
        for (i, level) in self.levels.iter().enumerate() {
            if effective >= level.min_distance && effective < level.max_distance {
                return Some((i, level.mesh_id));
            }
        }
        self.levels
            .last()
            .map(|l| (self.levels.len() - 1, l.mesh_id))
    }
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            levels: Vec::new(),
            distance_bias: 1.0,
        }
    }
}

/// Batch LOD selection for many entities.
///
/// Given camera position and per-entity positions + LOD configs, returns the
/// selected mesh_id for each entity.
pub fn select_lods(
    camera_pos: Vec3,
    positions: &[Vec3],
    configs: &[&LodConfig],
) -> Vec<Option<u64>> {
    positions
        .iter()
        .zip(configs.iter())
        .map(|(pos, config)| {
            let dist = (*pos - camera_pos).length();
            config.select(dist)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_level_config() -> LodConfig {
        LodConfig::new(vec![
            LodLevel::new(0.0, 50.0, 0),
            LodLevel::new(50.0, 150.0, 1),
            LodLevel::new(150.0, f32::MAX, 2),
        ])
    }

    #[test]
    fn test_select_high_detail() {
        let config = three_level_config();
        assert_eq!(config.select(10.0), Some(0));
    }

    #[test]
    fn test_select_medium_detail() {
        let config = three_level_config();
        assert_eq!(config.select(80.0), Some(1));
    }

    #[test]
    fn test_select_low_detail() {
        let config = three_level_config();
        assert_eq!(config.select(200.0), Some(2));
    }

    #[test]
    fn test_select_boundary() {
        let config = three_level_config();
        // At exactly 50.0, should be LOD 1 (min_distance=50.0)
        assert_eq!(config.select(50.0), Some(1));
    }

    #[test]
    fn test_select_empty() {
        let config = LodConfig::default();
        assert_eq!(config.select(10.0), None);
    }

    #[test]
    fn test_select_index() {
        let config = three_level_config();
        assert_eq!(config.select_index(10.0), Some((0, 0)));
        assert_eq!(config.select_index(80.0), Some((1, 1)));
        assert_eq!(config.select_index(200.0), Some((2, 2)));
    }

    #[test]
    fn test_distance_bias() {
        let config = three_level_config().with_bias(2.0);
        // Distance 20 with bias 2.0 => effective 40, still LOD 0
        assert_eq!(config.select(20.0), Some(0));
        // Distance 30 with bias 2.0 => effective 60, LOD 1
        assert_eq!(config.select(30.0), Some(1));
    }

    #[test]
    fn test_batch_select() {
        let config = three_level_config();
        let camera = Vec3::ZERO;
        let positions = vec![Vec3::new(10.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0)];
        let configs = vec![&config, &config];
        let results = select_lods(camera, &positions, &configs);
        assert_eq!(results[0], Some(0));
        assert_eq!(results[1], Some(1));
    }

    #[test]
    fn test_unsorted_levels_are_sorted() {
        let config = LodConfig::new(vec![
            LodLevel::new(150.0, f32::MAX, 2),
            LodLevel::new(0.0, 50.0, 0),
            LodLevel::new(50.0, 150.0, 1),
        ]);
        assert_eq!(config.select(10.0), Some(0));
        assert_eq!(config.select(200.0), Some(2));
    }
}
