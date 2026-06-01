# Texture Loading → Sprite Pipeline Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect engine-asset's `ImageData` directly to the SpritePipeline's texture binding, completing the file→GPU texture pipeline.

**Architecture:** Add `TextureStore::load_from_image_data()` to accept `ImageData` directly. Modify `TextureBridge::auto_sync()` to upload in-memory pixel data from `Texture.data` without re-reading from disk. Add `ImageData::load()` convenience constructor. The existing `load_from_bytes` remains as the low-level primitive.

**Tech Stack:** Rust, wgpu, engine-asset (ImageData, Texture, Handle), engine-render (TextureStore, TextureBridge)

---

## Files

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `crates/engine-asset/src/format/image.rs` | Add `ImageData::load()` convenience method |
| Modify | `crates/engine-render/src/texture_store.rs` | Add `load_from_image_data()` method |
| Modify | `crates/engine-render/src/texture_bridge.rs` | Upload from `Texture.data` when non-empty, skip disk re-read |
| Test | `crates/engine-render/src/texture_store.rs` | Test `load_from_image_data` |
| Test | `crates/engine-render/src/texture_bridge.rs` | Test in-memory upload path |

---

### Task 1: Add `ImageData::load()` convenience constructor

**Files:**
- Modify: `crates/engine-asset/src/format/image.rs:1-29`

- [ ] **Step 1: Add `ImageData::load()` method**

Add a `load` associated function that combines `load_image` + `from_dynamic` into one call:

```rust
impl ImageData {
    pub fn load(path: &str) -> Result<Self, String> {
        let img = load_image(path)?;
        Ok(Self::from_dynamic(&img))
    }

    pub fn from_dynamic(img: &DynamicImage) -> Self {
        // ... existing implementation unchanged
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p engine-asset`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add crates/engine-asset/src/format/image.rs
git commit -m "feat(asset): add ImageData::load() convenience constructor"
```

---

### Task 2: Add `TextureStore::load_from_image_data()`

**Files:**
- Modify: `crates/engine-render/src/texture_store.rs:134-210`

- [ ] **Step 1: Add `load_from_image_data` method**

Add a method that accepts `&engine_asset::format::image::ImageData` and delegates to `load_from_bytes`:

```rust
pub fn load_from_image_data(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image_data: &engine_asset::format::image::ImageData,
) -> Result<u64, TextureLoadError> {
    self.load_from_bytes(device, queue, &image_data.pixels, image_data.width, image_data.height)
}
```

Place this after the existing `load_from_bytes` method (after line 210).

- [ ] **Step 2: Add test for `load_from_image_data`**

Add a test in the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_load_from_image_data() {
    let (device, queue) = test_device();
    let layout = test_layout(&device);
    let mut store = TextureStore::new(&device, &queue, layout);
    let image_data = engine_asset::format::image::ImageData {
        pixels: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255],
        width: 2,
        height: 2,
        format: engine_asset::format::image::PixelFormat::Rgba8,
    };
    let id = store.load_from_image_data(&device, &queue, &image_data).unwrap();
    assert!(store.contains(id));
    assert_eq!(store.get_size(id), (2, 2));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p engine-render -- test_load_from_image_data`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/texture_store.rs
git commit -m "feat(render): add TextureStore::load_from_image_data()"
```

---

### Task 3: Modify `TextureBridge::auto_sync` to upload from in-memory data

**Files:**
- Modify: `crates/engine-render/src/texture_bridge.rs:214-225`

The key change: when `auto_sync` finds a `Handle<Texture>` whose `data` field is non-empty, upload it directly to the GPU via `texture_store.load_from_bytes()` instead of sending a disk-load request.

- [ ] **Step 1: Modify `auto_sync` to check `Texture.data`**

Replace the `auto_sync` method:

```rust
pub fn auto_sync(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, registry: &engine_asset::registry::Registry) {
    let handles = registry.get_handles_of_type::<Texture>();
    for handle in handles {
        let handle_id = HandleId::from_handle(handle);
        if self.states.contains_key(&handle_id) {
            continue;
        }
        let tex = handle.get();
        if !tex.data.is_empty() && tex.width > 0 && tex.height > 0 {
            // Upload directly from in-memory pixel data
            match self.texture_store.load_from_bytes(device, queue, &tex.data, tex.width, tex.height) {
                Ok(texture_id) => {
                    self.handle_to_id.insert(handle_id, texture_id);
                    self.states.insert(handle_id, LoadState::Ready(texture_id));
                    self.on_loaded.emit(&TextureLoaded {
                        handle_id,
                        result: Ok(texture_id),
                    });
                }
                Err(e) => {
                    let msg = e.to_string();
                    self.states.insert(handle_id, LoadState::Failed(msg.clone()));
                    self.on_loaded.emit(&TextureLoaded {
                        handle_id,
                        result: Err(msg),
                    });
                }
            }
        } else if !tex.asset_path.as_os_str().is_empty() {
            // Fall back to async disk load
            self.request(handle, &tex.asset_path.to_string_lossy());
        }
    }
}
```

- [ ] **Step 2: Update `render_frame` caller signature**

In `crates/engine-render/src/renderer.rs:128`, `auto_sync` is called. Update the call to pass `device` and `queue`:

```rust
bridge.auto_sync(&self.device, &self.queue, registry);
```

- [ ] **Step 3: Run full build**

Run: `cargo build -p engine-render`
Expected: success

- [ ] **Step 4: Run all tests**

Run: `cargo test -p engine-render`
Expected: all existing tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/engine-render/src/texture_bridge.rs crates/engine-render/src/renderer.rs
git commit -m "feat(render): bridge uploads from Texture.data when available"
```

---

### Task 4: Add end-to-end test for file→GPU path

**Files:**
- Modify: `crates/engine-render/src/texture_bridge.rs` (test module)

- [ ] **Step 1: Add test for in-memory texture upload via bridge**

Add a test that creates a `Texture` asset with pre-populated `data`, stores it in a `Registry`, and verifies `auto_sync` uploads it:

```rust
#[test]
fn test_auto_sync_uploads_in_memory_data() {
    let (device, queue) = test_device();
    let layout = test_layout(&device);
    let mut bridge = TextureBridge::new(&device, &queue, layout);
    let mut registry = engine_asset::registry::Registry::new();

    let tex = Texture {
        id: "inline_tex".into(),
        width: 1,
        height: 1,
        data: vec![255, 0, 0, 255], // 1x1 red RGBA8
        channels: 4,
        asset_path: PathBuf::new(),
    };
    let handle = registry.store("inline_tex", tex);

    bridge.auto_sync(&device, &queue, &registry);

    let id = bridge.resolve(&handle);
    assert_ne!(id, 0, "should have uploaded to GPU, not fallback");
    assert!(matches!(bridge.state(&handle), Some(LoadState::Ready(_))));
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p engine-render -- test_auto_sync_uploads_in_memory_data`
Expected: PASS

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p engine-render`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/texture_bridge.rs
git commit -m "test(render): verify in-memory texture upload via bridge"
```

---

### Task 5: Verify full pipeline + clippy/fmt

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -p engine-asset -p engine-render`
Expected: no warnings

- [ ] **Step 2: Run fmt check**

Run: `cargo fmt --check`
Expected: no diff

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p engine-asset -p engine-render`
Expected: all pass

- [ ] **Step 4: Final commit if needed**

If fmt or clippy required changes:
```bash
git add -A
git commit -m "chore: apply fmt/clippy fixes"
```
