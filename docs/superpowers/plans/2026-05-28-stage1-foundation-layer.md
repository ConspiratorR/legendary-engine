# Stage 1 Foundation Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 4 architectural issues in the render pipeline foundation — unify BindGroupLayout, auto-bridge asset→GPU texture loading, add depth sorting for transparent sprites, and clean up orphaned code.

**Architecture:** SpritePipeline owns the texture BindGroupLayout and exposes it. TextureBridge receives it externally and gains an `auto_sync()` method to auto-request textures from Registry. SpriteDraw gains a `depth` field for correct alpha blending order. Orphaned resource/texture.rs and dead code are removed.

**Tech Stack:** Rust 2024, wgpu, engine-asset, engine-render, bytemuck

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `crates/engine-render/src/pipeline/sprite.rs` | Modify | Expose `texture_bind_group_layout` via accessor |
| `crates/engine-render/src/texture_bridge.rs` | Modify | Accept external layout, add `auto_sync()` method |
| `crates/engine-render/src/renderer.rs` | Modify | Create pipeline first, pass layout to bridge, call auto_sync |
| `crates/engine-asset/src/types.rs` | Modify | Add `asset_path` field to `Texture` |
| `crates/engine-render/src/sprite.rs` | Modify | Add `depth` to `SpriteDraw`, sort by depth in `collect_batches` |
| `crates/engine-render/src/resource/texture.rs` | Delete | Orphaned duplicate of TextureStore |
| `crates/engine-render/src/resource/material.rs` | Delete | Empty stub |
| `crates/engine-render/src/resource/mod.rs` | Modify | Remove `texture` and `material` module declarations |

---

### Task 1: Unify BindGroupLayout

**Goal:** SpritePipeline owns the texture BindGroupLayout; TextureBridge receives it instead of creating its own duplicate.

**Files:**
- Modify: `crates/engine-render/src/pipeline/sprite.rs:37-41`
- Modify: `crates/engine-render/src/texture_bridge.rs:92-155`
- Modify: `crates/engine-render/src/renderer.rs:44-106`

- [ ] **Step 1: Add accessor to SpritePipeline**

In `crates/engine-render/src/pipeline/sprite.rs`, add a public accessor method after the existing struct impl. The `texture_bind_group_layout` field is already `pub` (line 40), so add a convenience method:

```rust
// Add after line 156, inside the impl block
impl SpritePipeline {
    // ... existing new() method ...

    /// Returns a reference to the texture bind group layout.
    /// Used by TextureBridge to create compatible bind groups.
    pub fn texture_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }
}
```

- [ ] **Step 2: Change TextureBridge::new() to accept external layout**

In `crates/engine-render/src/texture_bridge.rs`, change the `new()` signature and remove the duplicate layout creation:

Replace lines 92-155:
```rust
impl TextureBridge {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: wgpu::BindGroupLayout,
    ) -> Self {
        let (load_tx, load_rx) = crossbeam_channel::unbounded::<LoadRequest>();
        let (done_tx, done_rx) = crossbeam_channel::unbounded::<LoadResult>();

        std::thread::spawn(move || {
            for req in load_rx {
                let result = std::fs::read(&req.path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| image::load_from_memory(&bytes).map_err(|e| e.to_string()));

                let load_result = match result {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        LoadResult::Success {
                            handle_id: req.handle_id,
                            pixels: rgba.into_raw(),
                            width: w,
                            height: h,
                        }
                    }
                    Err(e) => LoadResult::Failure {
                        handle_id: req.handle_id,
                        error: e,
                    },
                };
                if done_tx.send(load_result).is_err() {
                    break;
                }
            }
        });

        Self {
            handle_to_id: HashMap::new(),
            states: HashMap::new(),
            completed_queue: done_rx,
            load_sender: load_tx,
            texture_store: TextureStore::new(device, queue, texture_layout),
            on_loaded: EventChannel::new(),
        }
    }
    // ... rest of methods unchanged ...
```

- [ ] **Step 3: Update Renderer::new() to create pipeline first, then pass layout**

In `crates/engine-render/src/renderer.rs`, change the creation order in `new()`:

Replace lines 74-93:
```rust
        let sprite_pipeline = Arc::new(SpritePipeline::new(&device, config.format));

        let camera_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform"),
            size: CAMERA_UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &sprite_pipeline.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.as_entire_binding(),
            }],
        });

        let sprite_renderer =
            SpriteRenderer::new(&device, sprite_pipeline.clone(), DEFAULT_SPRITE_CAPACITY);
```

This section stays the same. The TextureBridge is created externally (not in Renderer::new), so no change needed here. The key change is that wherever `TextureBridge::new()` is called, it now receives the layout.

- [ ] **Step 4: Update TextureBridge test helper**

In `crates/engine-render/src/texture_bridge.rs`, update the test helper to pass a layout:

Replace the test function `test_device()` and the tests that call `TextureBridge::new()`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_device() -> (wgpu::Device, wgpu::Queue) {
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
        (device, queue)
    }

    fn test_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_texture_layout"),
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
        })
    }

    #[test]
    fn test_resolve_unknown_returns_fallback() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let bridge = TextureBridge::new(&device, &queue, layout);
        let tex = Texture {
            id: "test".into(),
            width: 1,
            height: 1,
            data: vec![255, 0, 0, 255],
            channels: 4,
            asset_path: String::new(),
        };
        let handle = Handle::new(tex);
        assert_eq!(bridge.resolve(&handle), 0);
    }

    #[test]
    fn test_request_sets_pending_state() {
        let (device, queue) = test_device();
        let layout = test_layout(&device);
        let mut bridge = TextureBridge::new(&device, &queue, layout);
        let tex = Texture {
            id: "test".into(),
            width: 1,
            height: 1,
            data: vec![255, 0, 0, 255],
            channels: 4,
            asset_path: String::new(),
        };
        let handle = Handle::new(tex);
        bridge.request(&handle, "nonexistent_path.png");
        match bridge.state(&handle) {
            Some(LoadState::Pending) => {}
            other => panic!("Expected Some(Pending), got {:?}", other),
        }
    }
}
```

- [ ] **Step 5: Build and verify compilation**

Run: `cargo build -p engine-render`
Expected: Compiles without errors.

- [ ] **Step 6: Run tests**

Run: `cargo test -p engine-render`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-render/src/pipeline/sprite.rs crates/engine-render/src/texture_bridge.rs crates/engine-render/src/renderer.rs
git commit -m "refactor(render): unify BindGroupLayout — SpritePipeline owns it, TextureBridge receives it"
```

---

### Task 2: Add asset_path to Texture

**Goal:** Texture asset stores its file path so TextureBridge can auto-load it.

**Files:**
- Modify: `crates/engine-asset/src/types.rs:7-13`

- [ ] **Step 1: Add asset_path field to Texture**

In `crates/engine-asset/src/types.rs`, replace lines 7-13:

```rust
/// Image texture asset.
#[derive(Debug, Clone)]
pub struct Texture {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub channels: u8,
    pub asset_path: String,
}
```

- [ ] **Step 2: Find and fix all Texture construction sites**

Run: `grep -rn "Texture {" crates/` to find all places where `Texture` is constructed.

For each site, add `asset_path: String::new()` (or the actual path if known). Key locations:

In `crates/engine-asset/src/loader.rs` (or wherever textures are loaded from disk), set `asset_path` to the actual file path.

In test code, use `asset_path: String::new()`.

- [ ] **Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles without errors (all Texture constructions updated).

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A crates/engine-asset/src/
git commit -m "feat(asset): add asset_path field to Texture for auto-loading bridge"
```

---

### Task 3: Add auto_sync to TextureBridge

**Goal:** TextureBridge can scan a Registry and auto-request textures that haven't been loaded yet.

**Files:**
- Modify: `crates/engine-render/src/texture_bridge.rs`

- [ ] **Step 1: Add engine-asset Registry dependency**

Check `crates/engine-render/Cargo.toml` — it already depends on `engine-asset`. Verify `Registry` is accessible:

```rust
// Add to texture_bridge.rs imports (near line 1-5)
use engine_asset::registry::Registry;
```

- [ ] **Step 2: Add get_handles_of_type method to Registry**

In `crates/engine-asset/src/registry.rs`, the Registry uses `HashMap<String, Box<dyn Any>>`. Add a method to iterate all entries and downcast to a specific type. Add after the `contains` method (after line 37):

```rust
    /// Returns references to all stored handles of a given asset type.
    /// Iterates all entries and attempts to downcast each to Handle<T>.
    pub fn get_handles_of_type<T: Asset + 'static>(&self) -> Vec<&Handle<T>> {
        self.assets
            .values()
            .filter_map(|boxed| boxed.downcast_ref::<Handle<T>>())
            .collect()
    }
```

- [ ] **Step 3: Implement auto_sync method**

In `crates/engine-render/src/texture_bridge.rs`, add after the `texture_store_mut()` method (after line 227):

```rust
    /// Scans the Registry for Handle<Texture> assets and automatically
    /// requests loading for any that haven't been requested yet.
    /// Call this before flush() in the render loop.
    pub fn auto_sync(&mut self, registry: &Registry) {
        let handles = registry.get_handles_of_type::<Texture>();
        for handle in handles {
            let handle_id = HandleId::from_handle(handle);
            if !self.states.contains_key(&handle_id) {
                let path = &handle.get().asset_path;
                if !path.is_empty() {
                    self.request(handle, path);
                }
            }
        }
    }
```

Note: `handle.get()` returns `&Texture`, then `.asset_path` accesses the path field.

- [ ] **Step 4: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/engine-asset/src/registry.rs crates/engine-render/src/texture_bridge.rs
git commit -m "feat(render): add TextureBridge::auto_sync for automatic asset→GPU texture loading"
```

---

### Task 4: Integrate auto_sync into render_frame

**Goal:** Renderer calls auto_sync + flush automatically each frame.

**Files:**
- Modify: `crates/engine-render/src/renderer.rs:114-127`

- [ ] **Step 1: Update render_frame signature**

In `crates/engine-render/src/renderer.rs`, add `registry` parameter to `render_frame`:

Replace line 114-119:
```rust
    pub fn render_frame(
        &mut self,
        cameras: &[&crate::camera::Camera],
        all_sprites: &[crate::sprite::Sprite],
        bridge: &mut crate::texture_bridge::TextureBridge,
        registry: &engine_asset::registry::Registry,
    ) -> Result<(), wgpu::SurfaceError> {
```

- [ ] **Step 2: Add auto_sync call before flush**

After line 127 (`bridge.flush(...)`), insert auto_sync before it:

```rust
        bridge.auto_sync(registry);
        bridge.flush(&self.device, &self.queue);
```

- [ ] **Step 3: Update all callers of render_frame**

Search for `render_frame` callers and pass the registry. In the editor and examples, the registry is typically available from the app state.

Run: `grep -rn "render_frame" crates/ examples/` to find callers.

- [ ] **Step 4: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "feat(render): integrate auto_sync into render_frame for automatic texture loading"
```

---

### Task 5: Add depth sorting for transparent sprites

**Goal:** Sprites are sorted by depth (back-to-front) before batching for correct alpha blending.

**Files:**
- Modify: `crates/engine-render/src/sprite.rs:17-25,115-135`
- Modify: `crates/engine-render/src/renderer.rs:129-139`

- [ ] **Step 1: Add depth field to SpriteDraw**

In `crates/engine-render/src/sprite.rs`, add `depth` to `SpriteDraw`:

Replace lines 17-25:
```rust
#[derive(Clone)]
pub struct SpriteDraw {
    pub world_matrix: Mat4,
    pub color: [f32; 4],
    pub size: Vec2,
    pub texture_id: u64,
    pub flip_x: bool,
    pub flip_y: bool,
    pub depth: f32,
}
```

- [ ] **Step 2: Update SpriteDraw construction in renderer.rs**

In `crates/engine-render/src/renderer.rs`, fill `depth` from the sprite transform's z component:

Replace lines 129-139:
```rust
        let sprite_draws: Vec<SpriteDraw> = all_sprites
            .iter()
            .map(|s| {
                let pos = s.transform.transform_point3(engine_math::Vec3::ZERO);
                SpriteDraw {
                    world_matrix: s.transform,
                    color: s.color,
                    size: s.size,
                    texture_id: bridge.resolve(&s.texture),
                    flip_x: s.flip_x,
                    flip_y: s.flip_y,
                    depth: pos.z,
                }
            })
            .collect();
```

- [ ] **Step 3: Update collect_batches to sort by depth**

In `crates/engine-render/src/sprite.rs`, replace `collect_batches` (lines 115-135):

```rust
pub fn collect_batches(sprites: &[SpriteDraw]) -> Vec<SpriteBatch> {
    // Sort by depth (back-to-front) for correct alpha blending
    let mut sorted: Vec<&SpriteDraw> = sprites.iter().collect();
    sorted.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

    // Group by texture_id (stable sort preserves depth order within each group)
    let mut batch_map: std::collections::HashMap<u64, Vec<&SpriteDraw>> =
        std::collections::HashMap::new();
    for draw in sorted {
        batch_map.entry(draw.texture_id).or_default().push(draw);
    }

    let mut batches: Vec<SpriteBatch> = batch_map
        .into_iter()
        .map(|(tex_idx, draws)| {
            let mut batch = SpriteBatch::new(tex_idx);
            for draw in draws {
                batch.push(draw);
            }
            batch
        })
        .collect();

    batches.sort_by_key(|b| b.texture_id);
    batches
}
```

- [ ] **Step 4: Update test helper sprite_draw_default**

In `crates/engine-render/src/sprite.rs`, update the test helper (line 179-188):

```rust
    fn sprite_draw_default() -> SpriteDraw {
        SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_id: 0,
            flip_x: false,
            flip_y: false,
            depth: 0.0,
        }
    }
```

Also update `test_sprite_batch_push` (line 142-155) to include `depth: 0.0` in the SpriteDraw.

- [ ] **Step 5: Build and verify**

Run: `cargo build -p engine-render`
Expected: Compiles without errors.

- [ ] **Step 6: Run tests**

Run: `cargo test -p engine-render`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-render/src/sprite.rs crates/engine-render/src/renderer.rs
git commit -m "feat(render): add depth sorting for transparent sprites (back-to-front)"
```

---

### Task 6: Clean up orphaned code

**Goal:** Remove unused resource/texture.rs, resource/material.rs stub, and SpriteBatch::upload() dead code.

**Files:**
- Delete: `crates/engine-render/src/resource/texture.rs`
- Delete: `crates/engine-render/src/resource/material.rs`
- Modify: `crates/engine-render/src/resource/mod.rs`
- Modify: `crates/engine-render/src/sprite.rs:87-105`

- [ ] **Step 1: Remove module declarations from resource/mod.rs**

In `crates/engine-render/src/resource/mod.rs`, replace content:

```rust
pub mod mesh;
```

- [ ] **Step 2: Delete orphaned files**

```bash
rm crates/engine-render/src/resource/texture.rs
rm crates/engine-render/src/resource/material.rs
```

- [ ] **Step 3: Remove SpriteBatch::upload() dead code**

In `crates/engine-render/src/sprite.rs`, delete lines 87-105 (the `upload` method):

```rust
    // DELETE this entire method:
    // pub fn upload(&mut self, device: &wgpu::Device) { ... }
```

- [ ] **Step 4: Check for any references to removed items**

Run: `grep -rn "resource::texture\|resource::material\|SpriteBatch.*upload\|\.upload(" crates/ examples/`
Expected: No matches (or only matches that are already dead code).

- [ ] **Step 5: Build and verify**

Run: `cargo build`
Expected: Compiles without errors.

- [ ] **Step 6: Run tests**

Run: `cargo test -p engine-render`
Expected: All tests pass.

- [ ] **Step 7: Run clippy and fmt**

Run: `cargo clippy && cargo fmt --check`
Expected: No warnings or formatting issues.

- [ ] **Step 8: Commit**

```bash
git add -A crates/engine-render/src/resource/ crates/engine-render/src/sprite.rs
git commit -m "chore(render): remove orphaned resource/texture.rs, material.rs stub, and SpriteBatch::upload() dead code"
```

---

## Verification

After all tasks are complete:

1. `cargo build` — full workspace compiles
2. `cargo test -p engine-render` — all render tests pass
3. `cargo test -p engine-asset` — all asset tests pass
4. `cargo clippy` — no warnings
5. `cargo fmt --check` — formatting clean
6. Run `cargo run --example sprite_demo -p engine-core` — visual verification that sprites still render correctly
