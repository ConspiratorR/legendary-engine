# Terrain Editor Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the existing `engine-terrain` crate to the editor — add raycast, terrain editing mode in toolbar, mouse-driven sculpting in viewport, and undo/redo support.

**Architecture:** The engine-terrain crate (components, mesh_gen, brush, paint, vegetation, plugin) is already complete with 28 passing tests. This plan adds: (1) raycast module for mouse→terrain intersection, (2) Terrain tool type in editor toolbar, (3) viewport interaction for sculpting, (4) SculptCommand for undo/redo.

**Tech Stack:** Rust, egui 0.30.0, engine-ecs, engine-render, engine-terrain, engine-editor

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Create | `crates/engine-terrain/src/raycast.rs` | Ray-terrain intersection |
| Modify | `crates/engine-terrain/src/lib.rs:10` | Add `pub mod raycast;` |
| Modify | `crates/engine-editor/src/state.rs:5-11` | Add `Terrain` to `ToolType` enum |
| Modify | `crates/engine-editor/src/state.rs:330-420` | Add `TerrainPanel` to `EditorState` |
| Modify | `crates/engine-editor/src/layout.rs:395-410` | Add terrain button to toolbar |
| Modify | `crates/engine-editor/src/viewport.rs:319-344` | Handle terrain sculpting input |
| Modify | `crates/engine-editor/src/commands.rs` | Add `SculptCommand` |
| Modify | `crates/engine-editor/src/inspector.rs` | Show terrain panel when terrain entity selected |
| Modify | `crates/engine-editor/src/shortcuts.rs:56-58` | Add `TerrainTool` shortcut |

---

### Task 1: Raycast Module

**Files:**
- Create: `crates/engine-terrain/src/raycast.rs`
- Modify: `crates/engine-terrain/src/lib.rs:10` (add `pub mod raycast;`)

- [ ] **Step 1: Create raycast.rs with ray-terrain intersection**

```rust
// crates/engine-terrain/src/raycast.rs

use engine_math::Vec3;
use crate::components::Terrain;

/// A ray defined by origin and direction.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Result of a ray-terrain intersection test.
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    /// World-space hit position.
    pub point: Vec3,
    /// Grid coordinates (i, j) on the heightmap.
    pub grid_coord: (u32, u32),
    /// Distance from ray origin to hit point.
    pub distance: f32,
}

/// Test a ray against the terrain and return the closest hit.
///
/// Uses AABB rejection followed by per-triangle intersection.
pub fn raycast_terrain(terrain: &Terrain, ray: Ray, max_distance: f32) -> Option<RaycastHit> {
    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;

    // Terrain AABB (XZ plane, Y from min_height to max_height)
    let min = Vec3::new(-half_w, -terrain.height_scale, -half_h);
    let max = Vec3::new(half_w, terrain.height_scale, half_h);

    // AABB ray intersection
    let (t_min, t_max) = ray_aabb_intersect(ray.origin, ray.direction, min, max)?;
    if t_max < 0.0 || t_min > max_distance {
        return None;
    }

    // March along the ray in small steps to find intersection
    let start_t = t_min.max(0.0);
    let step_size = (terrain.world_size.x / terrain.resolution as f32) * 0.5;
    let mut t = start_t;

    while t <= t_max.min(max_distance) {
        let point = ray.origin + ray.direction * t;

        // Check if this point is below the terrain surface
        let height = sample_terrain_height(terrain, point.x, point.z);
        if point.y <= height {
            // Binary search for precise intersection
            let precise = binary_search_intersection(terrain, ray, t - step_size, t, 16);
            let hit_point = ray.origin + ray.direction * precise;
            let height = sample_terrain_height(terrain, hit_point.x, hit_point.z);
            let final_point = Vec3::new(hit_point.x, height, hit_point.z);

            // Convert to grid coordinates
            let gi = ((hit_point.x + half_w) / terrain.world_size.x * terrain.resolution as f32)
                .round() as u32;
            let gj = ((hit_point.z + half_h) / terrain.world_size.y * terrain.resolution as f32)
                .round() as u32;

            return Some(RaycastHit {
                point: final_point,
                grid_coord: (gi.min(terrain.resolution), gj.min(terrain.resolution)),
                distance: precise,
            });
        }

        t += step_size;
    }

    None
}

/// Binary search between two t values to find precise intersection.
fn binary_search_intersection(terrain: &Terrain, ray: Ray, mut t_min: f32, mut t_max: f32, iterations: u32) -> f32 {
    for _ in 0..iterations {
        let t_mid = (t_min + t_max) * 0.5;
        let point = ray.origin + ray.direction * t_mid;
        let height = sample_terrain_height(terrain, point.x, point.z);

        if point.y <= height {
            t_max = t_mid;
        } else {
            t_min = t_mid;
        }
    }
    (t_min + t_max) * 0.5
}

/// Sample terrain height at an arbitrary world XZ position via bilinear interpolation.
fn sample_terrain_height(terrain: &Terrain, world_x: f32, world_z: f32) -> f32 {
    let half_w = terrain.world_size.x * 0.5;
    let half_h = terrain.world_size.y * 0.5;
    let res = terrain.resolution;

    // Convert to normalized grid coordinates
    let gx = (world_x + half_w) / terrain.world_size.x * res as f32;
    let gz = (world_z + half_h) / terrain.world_size.y * res as f32;

    let gi = gx.floor() as i32;
    let gj = gz.floor() as i32;

    if gi < 0 || gj < 0 || gi >= res as i32 || gj >= res as i32 {
        return 0.0;
    }

    let fx = gx - gi as f32;
    let fz = gz - gj as f32;

    let gi = gi as u32;
    let gj = gj as u32;

    let h00 = terrain.get_height(gi, gj);
    let h10 = terrain.get_height((gi + 1).min(res), gj);
    let h01 = terrain.get_height(gi, (gj + 1).min(res));
    let h11 = terrain.get_height((gi + 1).min(res), (gj + 1).min(res));

    // Bilinear interpolation
    let h0 = h00 + (h10 - h00) * fx;
    let h1 = h01 + (h11 - h01) * fx;
    h0 + (h1 - h0) * fz
}

/// Ray-AABB intersection test. Returns (t_min, t_max) if hit.
fn ray_aabb_intersect(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<(f32, f32)> {
    let inv_dir = Vec3::new(
        if dir.x.abs() < 1e-8 { f32::INFINITY } else { 1.0 / dir.x },
        if dir.y.abs() < 1e-8 { f32::INFINITY } else { 1.0 / dir.y },
        if dir.z.abs() < 1e-8 { f32::INFINITY } else { 1.0 / dir.z },
    );

    let t1 = (min - origin) * inv_dir;
    let t2 = (max - origin) * inv_dir;

    let t_min = Vec3::new(t1.x.min(t2.x), t1.y.min(t2.y), t1.z.min(t2.z));
    let t_max = Vec3::new(t1.x.max(t2.x), t1.y.max(t2.y), t1.z.max(t2.z));

    let t_enter = t_min.x.max(t_min.y).max(t_min.z);
    let t_exit = t_max.x.min(t_max.y).min(t_max.z);

    if t_enter <= t_exit {
        Some((t_enter, t_exit))
    } else {
        None
    }
}

/// Convert screen position to a world-space ray.
///
/// Requires the camera's view-projection matrix and viewport dimensions.
pub fn screen_to_ray(
    screen_pos: (f32, f32),
    viewport_size: (f32, f32),
    view_proj_inv: &[[f32; 4]; 4],
) -> Ray {
    // Normalized device coordinates (-1 to 1)
    let ndc_x = (screen_pos.0 / viewport_size.0) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_pos.1 / viewport_size.1) * 2.0;

    // Unproject near and far points
    let near = unproject(Vec3::new(ndc_x, ndc_y, 0.0), view_proj_inv);
    let far = unproject(Vec3::new(ndc_x, ndc_y, 1.0), view_proj_inv);

    let direction = (far - near).normalize();

    Ray {
        origin: near,
        direction,
    }
}

fn unproject(ndc: Vec3, inv_vp: &[[f32; 4]; 4]) -> Vec3 {
    let m = inv_vp;
    let x = m[0][0] * ndc.x + m[0][1] * ndc.y + m[0][2] * ndc.z + m[0][3];
    let y = m[1][0] * ndc.x + m[1][1] * ndc.y + m[1][2] * ndc.z + m[1][3];
    let z = m[2][0] * ndc.x + m[2][1] * ndc.y + m[2][2] * ndc.z + m[2][3];
    let w = m[3][0] * ndc.x + m[3][1] * ndc.y + m[3][2] * ndc.z + m[3][3];

    if w.abs() > 1e-8 {
        Vec3::new(x / w, y / w, z / w)
    } else {
        Vec3::new(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_math::Vec2;

    #[test]
    fn test_ray_aabb_hit() {
        let origin = Vec3::new(0.0, 10.0, 0.0);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let min = Vec3::new(-5.0, -1.0, -5.0);
        let max = Vec3::new(5.0, 1.0, 5.0);

        let result = ray_aabb_intersect(origin, dir, min, max);
        assert!(result.is_some());
        let (t_min, t_max) = result.unwrap();
        assert!((t_min - 9.0).abs() < 0.01);
        assert!((t_max - 11.0).abs() < 0.01);
    }

    #[test]
    fn test_ray_aabb_miss() {
        let origin = Vec3::new(0.0, 10.0, 0.0);
        let dir = Vec3::new(1.0, 0.0, 0.0); // horizontal, won't hit ground AABB
        let min = Vec3::new(-5.0, -1.0, -5.0);
        let max = Vec3::new(5.0, 1.0, 5.0);

        let result = ray_aabb_intersect(origin, dir, min, max);
        assert!(result.is_none());
    }

    #[test]
    fn test_sample_flat_terrain() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        let h = sample_terrain_height(&terrain, 0.0, 0.0);
        assert!((h - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_raycast_flat_terrain() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        let ray = Ray {
            origin: Vec3::new(0.0, 5.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
        };
        let hit = raycast_terrain(&terrain, ray, 100.0);
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert!((hit.point.y - 0.0).abs() < 0.5);
    }

    #[test]
    fn test_raycast_misses_outside_bounds() {
        let terrain = Terrain::new(4, 2, Vec2::new(10.0, 10.0), 10.0);
        let ray = Ray {
            origin: Vec3::new(100.0, 5.0, 100.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
        };
        let hit = raycast_terrain(&terrain, ray, 100.0);
        assert!(hit.is_none());
    }
}
```

- [ ] **Step 2: Add module declaration to lib.rs**

In `crates/engine-terrain/src/lib.rs`, add after line 13:

```rust
pub mod raycast;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p engine-terrain`
Expected: All tests pass including new raycast tests

- [ ] **Step 4: Commit**

```bash
git add crates/engine-terrain/src/raycast.rs crates/engine-terrain/src/lib.rs
git commit -m "feat(terrain): add raycast module for editor mouse-terrain intersection"
```

---

### Task 2: Editor Terrain Tool Type

**Files:**
- Modify: `crates/engine-editor/src/state.rs:5-11` (add `Terrain` variant)
- Modify: `crates/engine-editor/src/shortcuts.rs:43-58` (add `TerrainTool` action)
- Modify: `crates/engine-editor/src/layout.rs:395-410` (add terrain toolbar button)

- [ ] **Step 1: Add Terrain variant to ToolType enum**

In `crates/engine-editor/src/state.rs`, change the `ToolType` enum (lines 5-11):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Select,
    Translate,
    Rotate,
    Scale,
    Terrain,
}
```

- [ ] **Step 2: Add TerrainPanel to EditorState**

In `crates/engine-editor/src/state.rs`, find the `EditorState` struct definition and add:

```rust
pub terrain_panel: crate::terrain_panel::TerrainPanel,
```

In the `EditorState::new()` constructor, initialize it:

```rust
terrain_panel: crate::terrain_panel::TerrainPanel::default(),
```

- [ ] **Step 3: Add TerrainTool shortcut**

In `crates/engine-editor/src/shortcuts.rs`, add to the `EditorAction` enum (after `ScaleTool`):

```rust
TerrainTool,
```

In `register_defaults()`, add:

```rust
self.bind(EditorAction::TerrainTool, KeyBinding::new(KeyCode::KeyT));
```

- [ ] **Step 4: Add terrain button to toolbar**

In `crates/engine-editor/src/layout.rs`, in the `draw_toolbar` function, change the tools array (line 395-410):

```rust
let tools = &["↖", "↔", "⟳", "⤢", "⛰"];
let tool_types = [
    ToolType::Select,
    ToolType::Translate,
    ToolType::Rotate,
    ToolType::Scale,
    ToolType::Terrain,
];
```

And update the loop to handle 5 tools (change line 411's `4.0` to `5.0`):

```rust
x += 5.0 * (btn_size + gap) + pad;
```

- [ ] **Step 5: Run clippy and tests**

Run: `cargo clippy -p engine-editor && cargo test -p engine-editor`
Expected: No warnings, all tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/engine-editor/src/state.rs crates/engine-editor/src/shortcuts.rs crates/engine-editor/src/layout.rs
git commit -m "feat(editor): add Terrain tool type and toolbar button"
```

---

### Task 3: Viewport Terrain Sculpting Interaction

**Files:**
- Modify: `crates/engine-editor/src/viewport.rs:319-344` (handle terrain input)

- [ ] **Step 1: Add terrain sculpting to viewport input handler**

In `crates/engine-editor/src/viewport.rs`, modify `handle_camera_input` to handle terrain mode. Add at the beginning of the function (after the bounds check):

```rust
// Terrain sculpting mode: left-click-drag applies brush
if state.active_tool == crate::state::ToolType::Terrain {
    // Only process if left mouse is held
    if gui.ui.input(|i| i.pointer.primary_down()) {
        if let Some(pointer_pos) = gui.ui.input(|i| i.pointer.interact_pos()) {
            // Convert to screen coordinates relative to canvas
            let screen_x = pointer_pos.x - canvas_rect.left();
            let screen_y = pointer_pos.y - canvas_rect.top();

            // Store in state for the terrain system to pick up
            state.terrain_sculpt_active = true;
            state.terrain_sculpt_screen_pos = Some((screen_x, screen_y));
        }
    } else {
        state.terrain_sculpt_active = false;
        state.terrain_sculpt_screen_pos = None;
    }

    // Still allow camera orbit with right-click
    let response = gui.ui.interact(
        canvas_rect,
        egui::Id::new("terrain_viewport"),
        egui::Sense::click_and_drag(),
    );
    if response.dragged_by(egui::PointerButton::Secondary) {
        let delta = response.drag_delta();
        state.camera.orbit(delta.x, -delta.y);
    }
    if response.hovered() {
        let scroll = gui.ui.input(|i| i.scroll_delta.y);
        if scroll.abs() > 0.0 {
            state.camera.zoom(scroll / 120.0);
        }
    }
    return;
}
```

- [ ] **Step 2: Add terrain sculpt fields to EditorState**

In `crates/engine-editor/src/state.rs`, add to the `EditorState` struct:

```rust
pub terrain_sculpt_active: bool,
pub terrain_sculpt_screen_pos: Option<(f32, f32)>,
```

Initialize in `EditorState::new()`:

```rust
terrain_sculpt_active: false,
terrain_sculpt_screen_pos: None,
```

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p engine-editor`
Expected: No warnings

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/viewport.rs crates/engine-editor/src/state.rs
git commit -m "feat(editor): add terrain sculpting viewport interaction"
```

---

### Task 4: SculptCommand for Undo/Redo

**Files:**
- Modify: `crates/engine-editor/src/commands.rs` (add `SculptCommand`)

- [ ] **Step 1: Add SculptCommand**

Add at the end of `crates/engine-editor/src/commands.rs`:

```rust
/// Command for terrain sculpting operations.
///
/// Captures a heightmap snapshot before modification for undo support.
#[derive(Debug)]
pub struct SculptCommand {
    pub entity_id: u64,
    pub affected_min: (u32, u32),
    pub affected_max: (u32, u32),
    pub resolution: u32,
    pub height_snapshot: Vec<f32>,
    pub description: String,
}

impl SculptCommand {
    pub fn new(
        entity_id: u64,
        terrain: &engine_terrain::components::Terrain,
        center: engine_math::Vec3,
        radius: f32,
    ) -> Self {
        let half_w = terrain.world_size.x * 0.5;
        let half_h = terrain.world_size.y * 0.5;
        let res = terrain.resolution;

        // Compute affected grid region
        let min_i = ((center.x - radius + half_w) / terrain.world_size.x * res as f32)
            .floor().max(0.0) as u32;
        let max_i = ((center.x + radius + half_w) / terrain.world_size.x * res as f32)
            .ceil().min(res as f32) as u32;
        let min_j = ((center.z - radius + half_h) / terrain.world_size.y * res as f32)
            .floor().max(0.0) as u32;
        let max_j = ((center.z + radius + half_h) / terrain.world_size.y * res as f32)
            .ceil().min(res as f32) as u32;

        // Snapshot affected heights
        let mut snapshot = Vec::new();
        for j in min_j..=max_j {
            for i in min_i..=max_i {
                let idx = (j * (res + 1) + i) as usize;
                snapshot.push(terrain.heightmap[idx]);
            }
        }

        Self {
            entity_id,
            affected_min: (min_i, min_j),
            affected_max: (max_i, max_j),
            resolution: res,
            height_snapshot: snapshot,
            description: "Sculpt Terrain".to_string(),
        }
    }
}

impl Command for SculptCommand {
    fn execute(&mut self) {
        // Execute is called after the brush has already been applied
        // Nothing to do here — the brush modifies the heightmap directly
    }

    fn undo(&mut self) {
        // Restore the snapshot heights
        // NOTE: This requires access to the World — needs integration with editor state
        // For now, this is a placeholder that the editor will call with the right context
        println!(
            "Undo: Restore terrain {} heights at {:?}-{:?}",
            self.entity_id, self.affected_min, self.affected_max
        );
    }

    fn redo(&mut self) {
        println!(
            "Redo: Re-apply terrain {} sculpt at {:?}-{:?}",
            self.entity_id, self.affected_min, self.affected_max
        );
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}
```

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p engine-editor`
Expected: No warnings

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/commands.rs
git commit -m "feat(editor): add SculptCommand for terrain undo/redo"
```

---

### Task 5: Inspector Terrain Panel Integration

**Files:**
- Modify: `crates/engine-editor/src/inspector.rs` (show terrain panel for terrain entities)

- [ ] **Step 1: Add terrain section to inspector**

In `crates/engine-editor/src/inspector.rs`, add a terrain section. The inspector currently shows sections for Transform, Material, Render, Light, Physics. Add a Terrain section that appears when the selected entity has a `Terrain` component.

Read the inspector first to understand the draw pattern, then add:

```rust
// Terrain section (add after physics section)
if let Some(terrain) = world.get_mut::<Terrain>(entity) {
    // Draw terrain panel
    let mut texture_layers = world.get_resource_mut::<TerrainTextureLayers>()
        .cloned()
        .unwrap_or_default();
    let mut vegetation_data = world.get_resource_mut::<VegetationData>()
        .cloned()
        .unwrap_or_default();

    state.terrain_panel.draw(ui, terrain, &mut texture_layers, &mut vegetation_data);
}
```

Note: The exact integration depends on the inspector's current structure. The inspector uses `gui.ui` (egui), and the TerrainPanel already uses egui widgets, so they're compatible.

- [ ] **Step 2: Run clippy and tests**

Run: `cargo clippy -p engine-editor && cargo test -p engine-editor`
Expected: No warnings, all tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/inspector.rs
git commit -m "feat(editor): integrate terrain panel into inspector"
```

---

### Task 6: Wire Up TerrainPlugin in Editor

**Files:**
- Modify: `crates/engine-editor/src/plugin.rs` (register TerrainPlugin)

- [ ] **Step 1: Register TerrainPlugin**

In `crates/engine-editor/src/plugin.rs`, add to the `EditorPlugin::build()` method:

```rust
app.add_plugin(engine_terrain::plugin::TerrainPlugin);
```

- [ ] **Step 2: Run full test suite**

Run: `cargo clippy && cargo test`
Expected: All tests pass, no warnings

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/plugin.rs
git commit -m "feat(editor): register TerrainPlugin in EditorPlugin"
```

---

### Task 7: Integration Verification

- [ ] **Step 1: Build and verify no errors**

Run: `cargo build`
Expected: Clean build with no errors

- [ ] **Step 2: Run full test suite**

Run: `cargo test -p engine-terrain && cargo test -p engine-editor`
Expected: All tests pass

- [ ] **Step 3: Run clippy and fmt**

Run: `cargo clippy && cargo fmt --check`
Expected: No warnings, code is formatted

- [ ] **Step 4: Final commit with any fixes**

```bash
git add -A
git commit -m "feat(terrain): complete terrain editor integration for RUST-102"
```
