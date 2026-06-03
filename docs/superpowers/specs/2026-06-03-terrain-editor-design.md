# Terrain Editor — Heightmap & Sculpting Design

**Date**: 2026-06-03
**Issue**: RUST-102 (RUST-74.5)
**Author**: ToolsEngineer
**Status**: Approved

## Overview

Implement a terrain system with heightmap-based mesh generation and real-time sculpting tools for the RustEngine editor. Uses **chunked LOD** strategy — terrain is split into fixed-size chunks, each with its own mesh. Sculpting only rebuilds affected chunks.

## 1. Data Structures

### Terrain Component (attached to root entity)

```rust
pub struct Terrain {
    pub heightmap: Vec<f32>,          // flat array, size = (resolution+1)^2
    pub resolution: u32,              // vertices per axis (default 129)
    pub chunk_size: u32,              // vertices per chunk (default 64)
    pub world_size: Vec2,             // world dimensions (e.g. 100.0 x 100.0)
    pub height_scale: f32,            // height multiplier (e.g. 50.0)
    pub dirty_chunks: HashSet<(u32, u32)>,  // chunk coords needing rebuild
}
```

### TerrainChunk Component (attached to each chunk entity)

```rust
pub struct TerrainChunk {
    pub chunk_coord: (u32, u32),      // position in chunk grid
    pub mesh: Option<Mesh>,           // GPU buffers (vertex + index)
    pub dirty: bool,                  // needs mesh rebuild
}
```

### Design Decisions

- Heightmap data is centralized in `Terrain` — chunks reference coordinates, not copies
- Sculpting modifies `Terrain.heightmap`, marks affected chunks dirty
- Dirty chunks are rebuilt next frame via `TerrainMeshGenSystem`
- Chunks are ECS entities with `TerrainChunk` component — parented to terrain root

## 2. Mesh Generation

### Algorithm

For each chunk:

1. Iterate vertices `(i, j)` within chunk bounds
2. Compute world position: `(x, y, height)` where `x/z` from chunk offset + vertex index, `height` from heightmap lookup
3. **Normal calculation** via central differences:
   - `tangent_x = (1, 0, h_right - h_left)`
   - `tangent_y = (0, 1, h_up - h_down)`
   - `normal = normalize(cross(tangent_x, tangent_y))`
4. **UV**: normalized local coordinates `(i/chunk_size, j/chunk_size)`
5. **Indices**: each quad → 2 triangles (6 indices)
6. Call existing `Mesh::new(device, &vertices, Some(&indices))` to create GPU buffers

### Integration

- Reuses existing `MeshVertex { position, normal, uv }` (32 bytes, `engine-render/src/resource/mesh.rs:5`)
- Reuses existing `Mesh::new()` (`engine-render/src/resource/mesh.rs:45`)
- Renders through existing PBR pipeline — no new shader needed
- `Mesh` is stored in `TerrainChunk.mesh`, replaces old mesh on rebuild (old mesh dropped → GPU buffer freed)

### Edge Cases

- Boundary vertices: clamp normal samples to terrain bounds (flat normals at edges)
- Chunk boundary seams: each chunk generates its own edge vertices — normals may differ slightly. Acceptable for V1; can be fixed with shared vertex borders later.

## 3. Brush System

### Brush Modes

| Mode | Effect | Formula |
|------|--------|---------|
| Raise | Increase height while mouse held | `h += strength * falloff * dt` |
| Lower | Decrease height while mouse held | `h -= strength * falloff * dt` |
| Smooth | Average with neighbors | `h = lerp(h, avg_neighbors, strength)` |
| Flatten | Set to brush center height | `h = lerp(h, target_h, strength)` |

### Parameters

```rust
pub struct BrushSettings {
    pub mode: BrushMode,              // Raise / Lower / Smooth / Flatten
    pub radius: f32,                  // brush radius in world units
    pub strength: f32,                // 0.0 ~ 1.0
    pub falloff: BrushFalloff,        // Linear / Smooth / Constant
}

pub enum BrushMode { Raise, Lower, Smooth, Flatten }
pub enum BrushFalloff { Linear, Smooth, Constant }
```

### Sculpting Workflow

1. Mouse raycast → terrain intersection → world position `(x, z)`
2. Compute affected chunk range: `chunk_min..=chunk_max` from `(x - radius, z - radius)` to `(x + radius, z + radius)`
3. For each affected chunk, iterate its vertices:
   - Compute distance from vertex to brush center
   - If within radius: apply brush effect weighted by falloff
4. Mark all affected chunks as dirty
5. For Smooth mode: snapshot neighbor heights before modification (avoid order-dependent artifacts)

### Undo/Redo

```rust
pub struct SculptCommand {
    pub terrain_entity: Entity,
    pub affected_region: Rect,        // bounding box of modified area
    pub height_snapshot: Vec<f32>,    // pre-modification heights for affected region
}
```

- `execute()`: apply brush modifications
- `undo()`: restore heights from snapshot, mark chunks dirty
- `redo()`: re-apply (store the post-modification snapshot too)

## 4. Editor Integration

### Terrain Editing Mode

- New mode in editor toolbar: "Terrain" (alongside Select/Move/Rotate)
- When active: left-click-drag applies brush, right-click-drag orbits camera (standard)
- Mode-specific cursor: circle showing brush radius

### Mouse Raycasting

- From screen mouse position, cast ray through camera
- First test against terrain AABB (fast rejection)
- Then test against individual chunk triangles (find closest intersection)
- Return world position on terrain surface

### Brush Preview

- Draw circle on terrain surface at intersection point
- Circle radius = brush radius
- Optional: color gradient to visualize falloff

### Inspector Panel (terrain_panel.rs)

When terrain entity is selected, show:

**Terrain Properties:**
- Resolution (u32 input)
- World Size (Vec2 input)
- Height Scale (f32 slider)

**Brush Settings:**
- Mode dropdown (Raise/Lower/Smooth/Flatten)
- Radius slider (1.0 ~ 100.0)
- Strength slider (0.01 ~ 1.0)
- Falloff dropdown (Linear/Smooth/Constant)

## 5. File Structure

### New Crate: `engine-terrain`

```
crates/engine-terrain/
├── Cargo.toml
└── src/
    ├── lib.rs          # re-exports
    ├── components.rs   # Terrain, TerrainChunk, BrushSettings
    ├── mesh_gen.rs     # heightmap → Mesh generation
    ├── brush.rs        # BrushMode, apply_brush()
    ├── raycast.rs      # ray-terrain intersection
    └── plugin.rs       # TerrainPlugin (registers systems)
```

### Editor Integration

```
crates/engine-editor/src/
└── terrain_panel.rs    # terrain inspector UI
```

### Cargo.toml Dependencies

```toml
[dependencies]
engine-ecs = { path = "../engine-ecs" }
engine-render = { path = "../engine-render" }
engine-math = { path = "../engine-math" }
engine-core = { path = "../engine-core" }
log = "0.4"
```

## 6. Systems & Plugin

### TerrainPlugin

```rust
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(terrain_mesh_update_system);
    }
}
```

### terrain_mesh_update_system

- Query all entities with `TerrainChunk` where `dirty == true`
- For each dirty chunk: call `generate_chunk_mesh()` with device + heightmap data
- Replace `chunk.mesh` with new mesh, clear `dirty` flag
- Needs access to `wgpu::Device` — will be stored as a resource

## 7. Acceptance Criteria Mapping

| Requirement | Implementation |
|-------------|---------------|
| Create terrain entity and generate mesh | `Terrain` component + `terrain_mesh_update_system` |
| Sculpting tools modify terrain height | `BrushSettings` + `apply_brush()` + dirty chunk rebuild |
| Multiple brush modes and parameter adjustment | `BrushMode` enum + `BrushSettings` struct + Inspector panel |

## 8. Scope & Constraints

- **In scope**: heightmap terrain, 4 brush modes, chunk-based mesh, editor integration, undo/redo
- **Out of scope (V1)**: terrain texturing/splatting, LOD distance-based switching, terrain collision physics, procedural generation
- **Performance target**: 129×129 terrain (4×4 chunks of 64 vertices) at 60fps with real-time sculpting
- **No new shaders**: reuses existing PBR pipeline with `MeshVertex`
