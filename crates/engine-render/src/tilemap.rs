//! 2D tilemap rendering (Phase A — basic tile rendering).
//!
//! Provides tileset definitions, tile layers, and tilemap collection with
//! a conversion function that produces [`SpriteDraw`] entries for the
//! existing sprite rendering pipeline.
//!
//! Collision (Phase B), autotiling (Phase C), dynamic modification (Phase D),
//! and tile propagation (Phase E) are deferred to follow-ups.

use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_math::{Mat4, Vec2, Vec3};
use std::collections::HashMap;

use crate::sprite::SpriteDraw;

// ---------------------------------------------------------------------------
// Tileset
// ---------------------------------------------------------------------------

/// A tile sheet texture split into a uniform grid of tiles.
#[derive(Clone)]
pub struct Tileset {
    pub texture: Handle<Texture>,
    pub texture_width: u32,
    pub texture_height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub tile_count: u32,
}

impl Tileset {
    /// Creates a tileset, deriving grid dimensions from texture and tile sizes.
    pub fn new(
        texture: Handle<Texture>,
        texture_width: u32,
        texture_height: u32,
        tile_width: u32,
        tile_height: u32,
    ) -> Self {
        assert!(tile_width > 0 && tile_height > 0, "tile size must be > 0");
        let columns = texture_width / tile_width;
        let rows = texture_height / tile_height;
        Self {
            texture,
            texture_width,
            texture_height,
            tile_width,
            tile_height,
            columns,
            tile_count: columns * rows,
        }
    }

    /// Computes the UV region `[u_min, v_min, u_max, v_max]` for the given
    /// tile index. Returns full-texture UV for out-of-bounds indices.
    pub fn tile_uv(&self, index: u32) -> [f32; 4] {
        if index == 0 || index >= self.tile_count {
            return [0.0, 0.0, 1.0, 1.0];
        }
        let col = index % self.columns;
        let row = index / self.columns;
        let u_min = col as f32 * self.tile_width as f32 / self.texture_width as f32;
        let v_min = row as f32 * self.tile_height as f32 / self.texture_height as f32;
        let u_max = (col + 1) as f32 * self.tile_width as f32 / self.texture_width as f32;
        let v_max = (row + 1) as f32 * self.tile_height as f32 / self.texture_height as f32;
        [u_min, v_min, u_max, v_max]
    }
}

// ---------------------------------------------------------------------------
// TileLayer
// ---------------------------------------------------------------------------

/// A 2D grid of tile indices that references a [`Tileset`] by index into a
/// [`TilesetStore`].
#[derive(Debug, Clone)]
pub struct TileLayer {
    pub tileset_index: usize,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<u32>,
    pub tile_size: Vec2,
    pub z_order: i32,
    pub offset: Vec2,
}

impl TileLayer {
    pub fn new(tileset_index: usize, width: u32, height: u32, tile_size: Vec2) -> Self {
        Self {
            tileset_index,
            width,
            height,
            tiles: vec![0; (width * height) as usize],
            tile_size,
            z_order: 0,
            offset: Vec2::ZERO,
        }
    }

    pub fn with_z_order(mut self, z: i32) -> Self {
        self.z_order = z;
        self
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: u32) {
        if x < self.width && y < self.height {
            self.tiles[(y * self.width + x) as usize] = tile;
        }
    }

    pub fn get_tile(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            self.tiles[(y * self.width + x) as usize]
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Tilemap
// ---------------------------------------------------------------------------

/// A collection of tile layer entity references.
#[derive(Debug, Clone)]
pub struct Tilemap {
    pub layers: Vec<u32>,
}

impl Default for Tilemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Tilemap {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn with_layers(mut self, layers: Vec<u32>) -> Self {
        self.layers = layers;
        self
    }
}

// ---------------------------------------------------------------------------
// TilesetStore (ECS resource)
// ---------------------------------------------------------------------------

/// ECS resource holding shared tileset data.
/// Insert via `world.insert_resource(TilesetStore::new())`.
pub struct TilesetStore {
    pub tilesets: Vec<Tileset>,
}

impl Default for TilesetStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TilesetStore {
    pub fn new() -> Self {
        Self {
            tilesets: Vec::new(),
        }
    }

    pub fn add(&mut self, tileset: Tileset) -> usize {
        let idx = self.tilesets.len();
        self.tilesets.push(tileset);
        idx
    }
}

// ---------------------------------------------------------------------------
// TilemapDrawBuffer (ECS resource)
// ---------------------------------------------------------------------------

/// Output of tilemap conversion — pre-built [`SpriteDraw`] entries.
/// Read after [`collect_tilemap_draws`] and merge into your sprite draw list.
pub struct TilemapDrawBuffer {
    draws: Vec<SpriteDraw>,
}

impl Default for TilemapDrawBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TilemapDrawBuffer {
    pub fn new() -> Self {
        Self { draws: Vec::new() }
    }

    pub fn draws(&self) -> &[SpriteDraw] {
        &self.draws
    }
}

// ---------------------------------------------------------------------------
// Tilemap sprite generation
// ---------------------------------------------------------------------------

/// Convert all tiles in a [`TileLayer`] into [`SpriteDraw`] entries.
///
/// Only non-zero tiles are emitted. Each tile gets the correct UV region
/// from the tileset and is positioned in world space based on the layer's
/// offset, tile coordinates, and tile size.
pub fn layer_to_sprite_draws(
    layer: &TileLayer,
    tileset: &Tileset,
    texture_id: u64,
    camera_depth: f32,
) -> Vec<SpriteDraw> {
    let mut draws = Vec::new();

    for y in 0..layer.height {
        for x in 0..layer.width {
            let tile_index = layer.tiles[(y * layer.width + x) as usize];
            if tile_index == 0 {
                continue;
            }

            let uv = tileset.tile_uv(tile_index);
            let world_x = layer.offset.x + x as f32 * layer.tile_size.x + layer.tile_size.x * 0.5;
            let world_y = layer.offset.y + y as f32 * layer.tile_size.y + layer.tile_size.y * 0.5;
            let depth = camera_depth + layer.z_order as f32;

            draws.push(SpriteDraw {
                world_matrix: Mat4::from_translation(Vec3::new(world_x, world_y, depth)),
                color: [1.0, 1.0, 1.0, 1.0],
                size: layer.tile_size,
                texture_id,
                flip_x: false,
                flip_y: false,
                depth,
                uv_region: uv,
            });
        }
    }

    draws
}

/// Collect SpriteDraw entries from all tilemap layers in the world.
///
/// Reads `Tilemap`, `TileLayer` (as ECS components by entity index),
/// and `TilesetStore` (ECS resource). Populates `TilemapDrawBuffer`.
///
/// Call each frame before rendering, then merge
/// `TilemapDrawBuffer::draws()` with your sprite draw list.
pub fn collect_tilemap_draws(
    world: &mut engine_ecs::world::World,
    bridge: &crate::texture_bridge::TextureBridge,
) {
    let tilemap_indices: Vec<u32> = world.component_entities::<Tilemap>();
    let layer_indices: Vec<u32> = world.component_entities::<TileLayer>();

    // Snapshot tileset store.
    let tilesets_snapshot: Vec<Tileset> = world
        .get_resource::<TilesetStore>()
        .map(|s| s.tilesets.clone())
        .unwrap_or_default();

    // Snapshot tilemaps (to know which layers belong to which).
    let tilemap_snapshots: Vec<(u32, Tilemap)> = tilemap_indices
        .iter()
        .filter_map(|&idx| world.get_by_index::<Tilemap>(idx).map(|t| (idx, t.clone())))
        .collect();

    // Snapshot layers.
    let layer_snapshots: Vec<(u32, TileLayer)> = layer_indices
        .iter()
        .filter_map(|&idx| {
            world
                .get_by_index::<TileLayer>(idx)
                .map(|l| (idx, l.clone()))
        })
        .collect();

    // Ensure output buffer exists.
    if world.get_resource::<TilemapDrawBuffer>().is_none() {
        world.insert_resource(TilemapDrawBuffer::new());
    }

    // Collect layers that belong to any tilemap, or orphan layers directly.
    let mut layers_to_render: Vec<&TileLayer> = Vec::new();

    if !tilemap_snapshots.is_empty() {
        // Build entity-index → layer lookup.
        let layer_lookup: HashMap<u32, &TileLayer> =
            layer_snapshots.iter().map(|(idx, l)| (*idx, l)).collect();

        for (_tm_idx, tilemap) in &tilemap_snapshots {
            for &layer_entity_idx in &tilemap.layers {
                if let Some(layer) = layer_lookup.get(&layer_entity_idx) {
                    layers_to_render.push(layer);
                }
            }
        }
    } else {
        // No tilemap component — render all TileLayer entities directly.
        for (_idx, layer) in &layer_snapshots {
            layers_to_render.push(layer);
        }
    }

    // Sort by z_order for correct layering.
    layers_to_render.sort_by_key(|l| l.z_order);

    let mut all_draws: Vec<SpriteDraw> = Vec::new();

    for layer in layers_to_render {
        let tileset = match tilesets_snapshot.get(layer.tileset_index) {
            Some(ts) => ts,
            None => continue,
        };

        let texture_id = bridge.resolve(&tileset.texture);
        let draws = layer_to_sprite_draws(layer, tileset, texture_id, 0.0);
        all_draws.extend(draws);
    }

    if let Some(buf) = world.get_resource_mut::<TilemapDrawBuffer>() {
        buf.draws = all_draws;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_texture() -> Handle<Texture> {
        Handle::new(Texture {
            id: "test_tileset".into(),
            width: 64,
            height: 64,
            data: vec![255; 64 * 64 * 4],
            channels: 4,
            asset_path: std::path::PathBuf::new(),
        })
    }

    // -- Tileset tests --

    #[test]
    fn test_tileset_new() {
        let ts = Tileset::new(test_texture(), 128, 64, 32, 32);
        assert_eq!(ts.columns, 4);
        assert_eq!(ts.tile_count, 8);
    }

    #[test]
    fn test_tileset_uv_first_tile() {
        let ts = Tileset::new(test_texture(), 128, 32, 32, 32);
        let uv = ts.tile_uv(0);
        // tile 0 = empty placeholder (returns full texture UV)
        assert_eq!(uv, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_tileset_uv_tile_1() {
        let ts = Tileset::new(test_texture(), 128, 32, 32, 32);
        // 4 columns, 1 row, tile 1 = col 1, row 0
        let uv = ts.tile_uv(1);
        assert!((uv[0] - 0.25).abs() < 1e-6); // u_min = 32/128
        assert!((uv[1] - 0.0).abs() < 1e-6);
        assert!((uv[2] - 0.5).abs() < 1e-6); // u_max = 64/128
        assert!((uv[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_tileset_uv_second_row() {
        let ts = Tileset::new(test_texture(), 64, 64, 32, 32);
        // 2x2 grid, tile 2 = col 0, row 1
        let uv = ts.tile_uv(2);
        assert!((uv[0] - 0.0).abs() < 1e-6);
        assert!((uv[1] - 0.5).abs() < 1e-6); // v_min = 32/64
        assert!((uv[2] - 0.5).abs() < 1e-6);
        assert!((uv[3] - 1.0).abs() < 1e-6);
    }

    // -- TileLayer tests --

    #[test]
    fn test_tile_layer_get_set() {
        let mut layer = TileLayer::new(0, 3, 3, Vec2::new(32.0, 32.0));
        layer.set_tile(1, 2, 5);
        assert_eq!(layer.get_tile(1, 2), 5);
        assert_eq!(layer.get_tile(0, 0), 0);
    }

    #[test]
    fn test_tile_layer_out_of_bounds() {
        let layer = TileLayer::new(0, 2, 2, Vec2::new(32.0, 32.0));
        assert_eq!(layer.get_tile(5, 5), 0);
    }

    // -- layer_to_sprite_draws tests --

    #[test]
    fn test_layer_to_draws_skips_empty() {
        let ts = Tileset::new(test_texture(), 64, 64, 32, 32);
        let layer = TileLayer::new(0, 2, 2, Vec2::new(32.0, 32.0));
        let draws = layer_to_sprite_draws(&layer, &ts, 0, 0.0);
        assert!(draws.is_empty());
    }

    #[test]
    fn test_layer_to_draws_generates_positions() {
        let ts = Tileset::new(test_texture(), 64, 64, 32, 32);
        let mut layer = TileLayer::new(0, 3, 2, Vec2::new(32.0, 32.0));
        layer.set_tile(0, 0, 1);
        layer.set_tile(2, 1, 3);

        let draws = layer_to_sprite_draws(&layer, &ts, 0, 0.0);
        assert_eq!(draws.len(), 2);

        // Tile at (0,0): center at (16, 16) + offset (0,0)
        let d0 = &draws[0];
        assert!((d0.world_matrix.w_axis.x - 16.0).abs() < 1e-4);
        assert!((d0.world_matrix.w_axis.y - 16.0).abs() < 1e-4);

        // Tile at (2,1): center at (2*32+16, 1*32+16) = (80, 48)
        let d1 = &draws[1];
        assert!((d1.world_matrix.w_axis.x - 80.0).abs() < 1e-4);
        assert!((d1.world_matrix.w_axis.y - 48.0).abs() < 1e-4);
    }

    #[test]
    fn test_layer_to_draws_with_offset() {
        let ts = Tileset::new(test_texture(), 64, 64, 32, 32);
        let mut layer = TileLayer::new(0, 2, 2, Vec2::new(32.0, 32.0));
        layer.offset = Vec2::new(100.0, 200.0);
        layer.set_tile(1, 0, 1);

        let draws = layer_to_sprite_draws(&layer, &ts, 0, 0.0);
        assert_eq!(draws.len(), 1);

        // Tile at (1,0): center = 100 + 32 + 16 = 148, 200 + 0 + 16 = 216
        let d = &draws[0];
        assert!((d.world_matrix.w_axis.x - 148.0).abs() < 1e-4);
        assert!((d.world_matrix.w_axis.y - 216.0).abs() < 1e-4);
    }

    #[test]
    fn test_layer_to_draws_z_order_affects_depth() {
        let ts = Tileset::new(test_texture(), 64, 64, 32, 32);
        let mut layer = TileLayer::new(0, 1, 1, Vec2::new(32.0, 32.0));
        layer.z_order = 5;
        layer.set_tile(0, 0, 1);

        let draws = layer_to_sprite_draws(&layer, &ts, 0, 10.0);
        assert_eq!(draws.len(), 1);
        // depth = camera_depth + z_order = 10.0 + 5 = 15.0
        assert!((draws[0].depth - 15.0).abs() < 1e-4);
    }

    #[test]
    fn test_layer_to_draws_correct_uv() {
        let ts = Tileset::new(test_texture(), 64, 64, 32, 32);
        let mut layer = TileLayer::new(0, 1, 1, Vec2::new(32.0, 32.0));
        layer.set_tile(0, 0, 3);

        let draws = layer_to_sprite_draws(&layer, &ts, 0, 0.0);
        assert_eq!(draws.len(), 1);

        // Tile 3 in 2x2 grid: col=1, row=1
        let uv = draws[0].uv_region;
        assert!((uv[0] - 0.5).abs() < 1e-6); // u_min
        assert!((uv[1] - 0.5).abs() < 1e-6); // v_min
        assert!((uv[2] - 1.0).abs() < 1e-6); // u_max
        assert!((uv[3] - 1.0).abs() < 1e-6); // v_max
    }

    // -- Tilemap tests --

    #[test]
    fn test_tilemap_with_layers() {
        let tm = Tilemap::new().with_layers(vec![0, 1, 2]);
        assert_eq!(tm.layers.len(), 3);
    }

    // -- Integration test --

    #[test]
    fn test_collect_tilemap_draws_orphan_layers() {
        use engine_ecs::world::World;

        let mut world = World::new();
        let tex = test_texture();

        // Tileset store with one tileset (2x2 grid from 64x64 with 32x32 tiles).
        let mut store = TilesetStore::new();
        store.add(Tileset::new(tex, 64, 64, 32, 32));
        world.insert_resource(store);
        world.insert_resource(TilemapDrawBuffer::new());

        // Create a layer with a few tiles.
        let mut layer = TileLayer::new(0, 3, 3, Vec2::new(32.0, 32.0));
        layer.set_tile(0, 0, 1);
        layer.set_tile(1, 1, 2);
        layer.set_tile(2, 2, 3);

        let e = world.spawn();
        world.add_component(e, layer);

        // No TextureBridge needed for resolve (texture_id 0 is fine for test).
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bridge = crate::texture_bridge::TextureBridge::new(&device, &queue, layout);

        collect_tilemap_draws(&mut world, &bridge);

        let buf = world.get_resource::<TilemapDrawBuffer>().unwrap();
        assert_eq!(buf.draws.len(), 3);
    }

    #[test]
    fn test_collect_tilemap_draws_sorts_by_z_order() {
        use engine_ecs::world::World;

        let mut world = World::new();

        let mut store = TilesetStore::new();
        store.add(Tileset::new(test_texture(), 64, 64, 32, 32));
        world.insert_resource(store);
        world.insert_resource(TilemapDrawBuffer::new());

        // Back layer (z_order 0).
        let mut back = TileLayer::new(0, 1, 1, Vec2::new(32.0, 32.0));
        back.z_order = 0;
        back.set_tile(0, 0, 1);

        // Front layer (z_order 10).
        let mut front = TileLayer::new(0, 1, 1, Vec2::new(32.0, 32.0));
        front.z_order = 10;
        front.set_tile(0, 0, 2);

        // Add front first, back second — z_order sort should reorder.
        let e1 = world.spawn();
        world.add_component(e1, front);
        let e2 = world.spawn();
        world.add_component(e2, back);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bridge = crate::texture_bridge::TextureBridge::new(&device, &queue, layout);

        collect_tilemap_draws(&mut world, &bridge);

        let buf = world.get_resource::<TilemapDrawBuffer>().unwrap();
        assert_eq!(buf.draws.len(), 2);

        // z_order 0 (back) should come first.
        assert!(buf.draws[0].depth < buf.draws[1].depth);
    }
}
