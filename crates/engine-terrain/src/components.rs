use engine_math::{Vec2, Vec3};
use std::collections::HashSet;

/// Core terrain component — attached to the root terrain entity.
///
/// Holds the heightmap data and configuration. Chunks are separate
/// ECS entities parented to this root.
#[derive(Debug, Clone)]
pub struct Terrain {
    /// Flat heightmap array, size = (resolution + 1)^2.
    pub heightmap: Vec<f32>,
    /// Vertices per axis (default 129).
    pub resolution: u32,
    /// Vertices per chunk edge (default 64).
    pub chunk_size: u32,
    /// World dimensions (width, depth).
    pub world_size: Vec2,
    /// Height multiplier.
    pub height_scale: f32,
    /// Chunk coordinates that need mesh rebuild.
    pub dirty_chunks: HashSet<(u32, u32)>,
}

impl Terrain {
    /// Create a new flat terrain.
    pub fn new(resolution: u32, chunk_size: u32, world_size: Vec2, height_scale: f32) -> Self {
        let total = ((resolution + 1) * (resolution + 1)) as usize;
        Self {
            heightmap: vec![0.0; total],
            resolution,
            chunk_size,
            world_size,
            height_scale,
            dirty_chunks: HashSet::new(),
        }
    }

    /// Get height at grid coordinate (i, j).
    pub fn get_height(&self, i: u32, j: u32) -> f32 {
        if i <= self.resolution && j <= self.resolution {
            let idx = (j * (self.resolution + 1) + i) as usize;
            self.heightmap[idx] * self.height_scale
        } else {
            0.0
        }
    }

    /// Set height at grid coordinate (i, j).
    pub fn set_height(&mut self, i: u32, j: u32, value: f32) {
        if i <= self.resolution && j <= self.resolution {
            let idx = (j * (self.resolution + 1) + i) as usize;
            self.heightmap[idx] = value;
        }
    }

    /// Mark all chunks overlapping a world-space region as dirty.
    pub fn mark_dirty_region(&mut self, center: Vec3, radius: f32) {
        let chunk_count = self.chunk_count();
        let chunk_world_w = self.world_size.x / chunk_count as f32;
        let chunk_world_h = self.world_size.y / chunk_count as f32;

        let min_cx = ((center.x - radius - self.world_size.x * 0.5) / chunk_world_w)
            .floor()
            .max(0.0) as u32;
        let max_cx = ((center.x + radius - self.world_size.x * 0.5) / chunk_world_w)
            .ceil()
            .min(chunk_count as f32 - 1.0) as u32;
        let min_cz = ((center.z - radius - self.world_size.y * 0.5) / chunk_world_h)
            .floor()
            .max(0.0) as u32;
        let max_cz = ((center.z + radius - self.world_size.y * 0.5) / chunk_world_h)
            .ceil()
            .min(chunk_count as f32 - 1.0) as u32;

        for cx in min_cx..=max_cx {
            for cz in min_cz..=max_cz {
                self.dirty_chunks.insert((cx, cz));
            }
        }
    }

    /// Number of chunks per axis.
    pub fn chunk_count(&self) -> u32 {
        self.resolution / self.chunk_size
    }
}

/// Component attached to each terrain chunk entity.
pub struct TerrainChunk {
    /// Position in chunk grid.
    pub chunk_coord: (u32, u32),
    /// GPU mesh handle (vertex + index buffers).
    pub mesh: Option<engine_render::resource::mesh::Mesh>,
    /// Whether this chunk needs a mesh rebuild.
    pub dirty: bool,
}

impl TerrainChunk {
    /// Create a new chunk at the given grid coordinate, marked dirty for initial mesh generation.
    pub fn new(chunk_coord: (u32, u32)) -> Self {
        Self {
            chunk_coord,
            mesh: None,
            dirty: true,
        }
    }
}

/// Settings for terrain brushes (sculpting and painting).
#[derive(Debug, Clone)]
pub struct BrushSettings {
    pub radius: f32,
    pub strength: f32,
    pub falloff: BrushFalloff,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            radius: 5.0,
            strength: 0.3,
            falloff: BrushFalloff::Smooth,
        }
    }
}

/// Falloff curve for brush influence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushFalloff {
    /// Linear falloff from center to edge.
    Linear,
    /// Smooth (cosine) falloff.
    Smooth,
    /// No falloff — uniform influence within radius.
    Constant,
}

impl BrushFalloff {
    /// Compute falloff weight at normalized distance `t` (0.0 = center, 1.0 = edge).
    pub fn weight(self, t: f32) -> f32 {
        match self {
            BrushFalloff::Linear => (1.0 - t).max(0.0),
            BrushFalloff::Smooth => {
                let t = t.min(1.0);
                (1.0 + (t * std::f32::consts::PI).cos()) * 0.5
            }
            BrushFalloff::Constant => {
                if t <= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// Sculpting brush modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SculptMode {
    Raise,
    Lower,
    Smooth,
    Flatten,
}

/// Splat map for terrain texture painting.
///
/// Stores per-vertex blend weights for up to 4 texture layers.
/// Weights are stored as RGBA channels, each channel = one layer weight.
/// Weights should sum to 1.0 at each vertex.
#[derive(Debug, Clone)]
pub struct SplatMap {
    /// Width/height of the splat map (matches terrain resolution + 1).
    pub resolution: u32,
    /// RGBA weights per pixel. Each pixel is 4 bytes (R=layer0, G=layer1, B=layer2, A=layer3).
    pub data: Vec<[u8; 4]>,
}

impl SplatMap {
    /// Create a new splat map with all weight on layer 0.
    pub fn new(resolution: u32) -> Self {
        let total = ((resolution + 1) * (resolution + 1)) as usize;
        Self {
            resolution,
            data: vec![[255, 0, 0, 0]; total],
        }
    }

    /// Get weights at grid coordinate (i, j).
    pub fn get_weights(&self, i: u32, j: u32) -> [u8; 4] {
        if i <= self.resolution && j <= self.resolution {
            let idx = (j * (self.resolution + 1) + i) as usize;
            self.data[idx]
        } else {
            [255, 0, 0, 0]
        }
    }

    /// Set weights at grid coordinate (i, j).
    pub fn set_weights(&mut self, i: u32, j: u32, weights: [u8; 4]) {
        if i <= self.resolution && j <= self.resolution {
            let idx = (j * (self.resolution + 1) + i) as usize;
            self.data[idx] = weights;
        }
    }

    /// Paint a layer at grid coordinate (i, j) with given strength (0.0-1.0).
    /// Increases the target layer weight and decreases others proportionally.
    pub fn paint(&mut self, i: u32, j: u32, layer: usize, strength: f32) {
        if layer >= 4 {
            return;
        }
        let weights = self.get_weights(i, j);
        let mut w = [
            weights[0] as f32,
            weights[1] as f32,
            weights[2] as f32,
            weights[3] as f32,
        ];

        let increase = strength * 255.0;
        let old_target = w[layer];
        w[layer] = (w[layer] + increase).min(255.0);
        let actual_increase = w[layer] - old_target;

        // Decrease other layers proportionally
        let other_sum: f32 = w
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != layer)
            .map(|(_, v)| *v)
            .sum();
        if other_sum > 0.0 {
            for (k, wk) in w.iter_mut().enumerate() {
                if k != layer {
                    *wk = (*wk - actual_increase * (*wk / other_sum)).max(0.0);
                }
            }
        }

        self.set_weights(i, j, [w[0] as u8, w[1] as u8, w[2] as u8, w[3] as u8]);
    }

    /// Erase a layer at grid coordinate (i, j) — redistribute its weight to layer 0.
    pub fn erase(&mut self, i: u32, j: u32, layer: usize, strength: f32) {
        if layer == 0 || layer >= 4 {
            return;
        }
        let weights = self.get_weights(i, j);
        let mut w = [
            weights[0] as f32,
            weights[1] as f32,
            weights[2] as f32,
            weights[3] as f32,
        ];

        let decrease = strength * 255.0;
        let actual_decrease = w[layer].min(decrease);
        w[layer] -= actual_decrease;
        w[0] = (w[0] + actual_decrease).min(255.0);

        self.set_weights(i, j, [w[0] as u8, w[1] as u8, w[2] as u8, w[3] as u8]);
    }
}

/// A texture layer in the terrain material.
#[derive(Debug, Clone)]
pub struct TextureLayer {
    /// Display name.
    pub name: String,
    /// Texture asset handle (key into TextureStore).
    pub texture_handle: Option<u64>,
    /// Tiling factor for UV coordinates.
    pub tiling: f32,
    /// Base color tint (used when no texture is assigned).
    pub tint: [f32; 3],
}

impl Default for TextureLayer {
    fn default() -> Self {
        Self {
            name: "Layer".to_string(),
            texture_handle: None,
            tiling: 16.0,
            tint: [0.5, 0.5, 0.5],
        }
    }
}

/// Resource holding all texture layers for terrain painting.
#[derive(Debug, Clone)]
pub struct TerrainTextureLayers {
    pub layers: Vec<TextureLayer>,
}

impl Default for TerrainTextureLayers {
    fn default() -> Self {
        Self {
            layers: vec![TextureLayer {
                name: "Base".to_string(),
                ..Default::default()
            }],
        }
    }
}

impl TerrainTextureLayers {
    /// Add a new texture layer and return its index.
    pub fn add_layer(&mut self, name: String) -> usize {
        self.layers.push(TextureLayer {
            name,
            ..Default::default()
        });
        self.layers.len() - 1
    }

    /// Remove a texture layer by index. The base layer (index 0) cannot be removed.
    pub fn remove_layer(&mut self, index: usize) {
        if index > 0 && index < self.layers.len() {
            self.layers.remove(index);
        }
    }
}

/// Painting brush mode for terrain textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintMode {
    /// Add weight to a texture layer.
    Paint,
    /// Remove weight from a texture layer (redistribute to layer 0).
    Erase,
}

/// Settings for the terrain texture painting brush.
#[derive(Debug, Clone)]
pub struct PaintBrushSettings {
    pub radius: f32,
    pub strength: f32,
    pub falloff: BrushFalloff,
    pub target_layer: usize,
    pub mode: PaintMode,
}

impl Default for PaintBrushSettings {
    fn default() -> Self {
        Self {
            radius: 5.0,
            strength: 0.3,
            falloff: BrushFalloff::Smooth,
            target_layer: 0,
            mode: PaintMode::Paint,
        }
    }
}

/// Definition of a vegetation type that can be placed on terrain.
#[derive(Debug, Clone)]
pub struct VegetationType {
    /// Display name.
    pub name: String,
    /// Mesh asset handle for rendering.
    pub mesh_handle: Option<u64>,
    /// Texture/material handle.
    pub material_handle: Option<u64>,
    /// Minimum scale factor for random sizing.
    pub scale_min: f32,
    /// Maximum scale factor for random sizing.
    pub scale_max: f32,
    /// Minimum slope angle (degrees) where this vegetation can appear.
    pub slope_min: f32,
    /// Maximum slope angle (degrees) where this vegetation can appear.
    pub slope_max: f32,
    /// Minimum height where this vegetation can appear.
    pub height_min: f32,
    /// Maximum height where this vegetation can appear.
    pub height_max: f32,
    /// Density multiplier (instances per unit area).
    pub density: f32,
    /// Random seed for reproducible placement.
    pub seed: u64,
    /// LOD distances: (close, medium, far). Beyond `far`, vegetation is not rendered.
    pub lod_distances: [f32; 3],
}

impl Default for VegetationType {
    fn default() -> Self {
        Self {
            name: "Vegetation".to_string(),
            mesh_handle: None,
            material_handle: None,
            scale_min: 0.8,
            scale_max: 1.2,
            slope_min: 0.0,
            slope_max: 45.0,
            height_min: f32::NEG_INFINITY,
            height_max: f32::INFINITY,
            density: 1.0,
            seed: 42,
            lod_distances: [20.0, 50.0, 100.0],
        }
    }
}

/// A single placed vegetation instance.
#[derive(Debug, Clone, Copy)]
pub struct VegetationInstance {
    pub position: Vec3,
    pub rotation_y: f32,
    pub scale: f32,
    pub vegetation_type_index: usize,
}

/// Resource holding all vegetation types and their placed instances.
#[derive(Debug, Clone, Default)]
pub struct VegetationData {
    pub types: Vec<VegetationType>,
    pub instances: Vec<VegetationInstance>,
    /// Whether instances need regeneration.
    pub dirty: bool,
}

impl VegetationData {
    /// Add a vegetation type and mark instances for regeneration. Returns the new type index.
    pub fn add_type(&mut self, veg_type: VegetationType) -> usize {
        self.types.push(veg_type);
        self.dirty = true;
        self.types.len() - 1
    }

    /// Remove a vegetation type by index, cleaning up associated instances and adjusting indices.
    pub fn remove_type(&mut self, index: usize) {
        if index < self.types.len() {
            self.types.remove(index);
            self.instances
                .retain(|inst| inst.vegetation_type_index != index);
            // Adjust indices
            for inst in &mut self.instances {
                if inst.vegetation_type_index > index {
                    inst.vegetation_type_index -= 1;
                }
            }
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_creation() {
        let terrain = Terrain::new(128, 64, Vec2::new(100.0, 100.0), 50.0);
        assert_eq!(terrain.resolution, 128);
        assert_eq!(terrain.chunk_size, 64);
        assert_eq!(terrain.heightmap.len(), 129 * 129);
        assert_eq!(terrain.chunk_count(), 2);
    }

    #[test]
    fn test_terrain_height_get_set() {
        let mut terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        terrain.set_height(2, 3, 0.5);
        assert!((terrain.get_height(2, 3) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_splat_map_creation() {
        let sm = SplatMap::new(4);
        assert_eq!(sm.data.len(), 25);
        assert_eq!(sm.get_weights(0, 0), [255, 0, 0, 0]);
    }

    #[test]
    fn test_splat_map_paint() {
        let mut sm = SplatMap::new(4);
        sm.paint(0, 0, 1, 1.0);
        let w = sm.get_weights(0, 0);
        assert!(w[1] > 0);
    }

    #[test]
    fn test_splat_map_erase() {
        let mut sm = SplatMap::new(4);
        sm.paint(0, 0, 1, 1.0);
        sm.erase(0, 0, 1, 1.0);
        let w = sm.get_weights(0, 0);
        assert_eq!(w[1], 0);
    }

    #[test]
    fn test_brush_falloff() {
        assert!((BrushFalloff::Linear.weight(0.0) - 1.0).abs() < 1e-6);
        assert!((BrushFalloff::Linear.weight(1.0) - 0.0).abs() < 1e-6);
        assert!((BrushFalloff::Constant.weight(0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_texture_layers() {
        let mut layers = TerrainTextureLayers::default();
        assert_eq!(layers.layers.len(), 1);
        layers.add_layer("Grass".to_string());
        assert_eq!(layers.layers.len(), 2);
        assert_eq!(layers.layers[1].name, "Grass");
    }

    #[test]
    fn test_vegetation_data() {
        let mut data = VegetationData::default();
        data.add_type(VegetationType {
            name: "Tree".to_string(),
            density: 0.5,
            ..Default::default()
        });
        assert_eq!(data.types.len(), 1);
        assert!(data.dirty);
    }
}
