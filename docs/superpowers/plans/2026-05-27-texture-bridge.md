# Texture Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the asset system's `Handle<Texture>` to the render system's `TextureStore` via an async-loading `TextureBridge` with event notifications.

**Architecture:** A `TextureBridge` sits between asset and render layers. A background thread decodes images from disk; the render thread uploads them to GPU via `TextureStore`. An `EventChannel<T>` notifies game code when textures are ready.

**Tech Stack:** Rust 2024 edition, wgpu, crossbeam-channel, image crate

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/engine-core/src/event.rs` | Create | Generic `EventChannel<T>` publish/subscribe system |
| `crates/engine-core/src/lib.rs` | Modify | Add `pub mod event` |
| `crates/engine-asset/src/asset.rs` | Modify | Make `Handle.inner` `pub(crate)`, add `HandleId` |
| `crates/engine-render/Cargo.toml` | Modify | Add `crossbeam-channel` dependency |
| `crates/engine-render/src/texture_bridge.rs` | Create | `TextureBridge` with async loader, handle→id mapping, event emission |
| `crates/engine-render/src/lib.rs` | Modify | Add `pub mod texture_bridge` |
| `crates/engine-render/src/sprite.rs` | Modify | `Sprite.texture`: `u64` → `Handle<Texture>` |
| `crates/engine-render/src/renderer.rs` | Modify | Remove `texture_store` field, update `render_frame` signature |
| `crates/engine-core/examples/sprite_demo.rs` | Modify | Adapt to new API |

---

### Task 1: EventChannel<T>

**Files:**
- Create: `crates/engine-core/src/event.rs`
- Modify: `crates/engine-core/src/lib.rs:15`

- [ ] **Step 1: Add module declaration**

In `crates/engine-core/src/lib.rs`, add after line 15 (`pub mod transform;`):

```rust
pub mod event;
```

- [ ] **Step 2: Create EventChannel with failing tests**

Create `crates/engine-core/src/event.rs`:

```rust
use std::sync::Arc;

/// Unique identifier for a registered event listener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenerId(usize);

/// Generic synchronous publish/subscribe event channel.
///
/// Listeners are called in registration order when `emit()` is invoked.
/// Uses `Fn(&T)` so multiple listeners can fire without `&mut` conflicts.
pub struct EventChannel<T: Send + 'static> {
    listeners: Vec<Arc<dyn Fn(&T) + Send + Sync>>,
    next_id: usize,
}

impl<T: Send + 'static> EventChannel<T> {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            next_id: 0,
        }
    }

    /// Register a listener. Returns a `ListenerId` for later removal.
    pub fn subscribe(&mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> ListenerId {
        let id = ListenerId(self.next_id);
        self.next_id += 1;
        self.listeners.push(Arc::new(handler));
        id
    }

    /// Remove a listener by id. No-op if already removed.
    pub fn unsubscribe(&mut self, id: ListenerId) {
        // Store (id, arc) pairs so we can remove by id
        // Re-implement with a Vec of (ListenerId, Arc) instead
        // For now, this is a placeholder — see Step 3
        let _ = id;
    }

    /// Fire the event to all registered listeners.
    pub fn emit(&self, event: &T) {
        for listener in &self.listeners {
            listener(event);
        }
    }
}

impl<T: Send + 'static> Default for EventChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_emit_calls_listener() {
        let mut channel = EventChannel::<i32>::new();
        let received = Arc::new(AtomicUsize::new(0));
        let r = received.clone();
        channel.subscribe(move |val| {
            r.store(*val as usize, Ordering::Relaxed);
        });
        channel.emit(&42);
        assert_eq!(received.load(Ordering::Relaxed), 42);
    }

    #[test]
    fn test_multiple_listeners() {
        let mut channel = EventChannel::<i32>::new();
        let sum = Arc::new(AtomicUsize::new(0));

        let s1 = sum.clone();
        channel.subscribe(move |val| {
            s1.fetch_add(*val as usize, Ordering::Relaxed);
        });
        let s2 = sum.clone();
        channel.subscribe(move |val| {
            s2.fetch_add(*val as usize, Ordering::Relaxed);
        });

        channel.emit(&10);
        assert_eq!(sum.load(Ordering::Relaxed), 20);
    }

    #[test]
    fn test_unsubscribe_removes_listener() {
        let mut channel = EventChannel::<i32>::new();
        let received = Arc::new(AtomicUsize::new(0));
        let r = received.clone();
        let id = channel.subscribe(move |val| {
            r.store(*val as usize, Ordering::Relaxed);
        });
        channel.unsubscribe(id);
        channel.emit(&42);
        assert_eq!(received.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_default_is_empty() {
        let channel = EventChannel::<i32>::default();
        channel.emit(&1); // should not panic
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p engine-core event`
Expected: FAIL — `unsubscribe` is a no-op placeholder

- [ ] **Step 4: Fix unsubscribe to actually work**

Replace the `EventChannel` internals with a `(ListenerId, Arc)` pair approach:

```rust
pub struct EventChannel<T: Send + 'static> {
    listeners: Vec<(ListenerId, Arc<dyn Fn(&T) + Send + Sync>)>,
    next_id: usize,
}

impl<T: Send + 'static> EventChannel<T> {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            next_id: 0,
        }
    }

    pub fn subscribe(&mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> ListenerId {
        let id = ListenerId(self.next_id);
        self.next_id += 1;
        self.listeners.push((id, Arc::new(handler)));
        id
    }

    pub fn unsubscribe(&mut self, id: ListenerId) {
        self.listeners.retain(|(lid, _)| *lid != id);
    }

    pub fn emit(&self, event: &T) {
        for (_, listener) in &self.listeners {
            listener(event);
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p engine-core event`
Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/engine-core/src/event.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add EventChannel<T> generic event system"
```

---

### Task 2: HandleId

**Files:**
- Modify: `crates/engine-asset/src/asset.rs`

- [ ] **Step 1: Add HandleId and make inner pub(crate)**

In `crates/engine-asset/src/asset.rs`, change `struct HandleInner` visibility and add `HandleId`:

After line 4 (`};`), add:

```rust
/// Unique identifier for a Handle, derived from its inner Arc pointer.
/// All clones of the same Handle share the same Arc and thus the same HandleId.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandleId(usize);

impl HandleId {
    pub fn from_handle<T: Asset>(handle: &Handle<T>) -> Self {
        Self(Arc::as_ptr(&handle.inner) as *const () as usize)
    }
}
```

Change line 15 from:
```rust
struct HandleInner<T: Asset> {
```
to:
```rust
pub(crate) struct HandleInner<T: Asset> {
```

Also change line 12 from:
```rust
pub struct Handle<T: Asset> {
    inner: Arc<HandleInner<T>>,
```
to:
```rust
pub struct Handle<T: Asset> {
    pub(crate) inner: Arc<HandleInner<T>>,
```

- [ ] **Step 2: Add HandleId tests**

Add to the `#[cfg(test)] mod tests` block:

```rust
use crate::asset::HandleId;

#[test]
fn test_handle_id_same_for_clones() {
    let asset = MyAsset::new(1);
    let h1 = Handle::new(asset);
    let h2 = h1.clone();
    assert_eq!(HandleId::from_handle(&h1), HandleId::from_handle(&h2));
}

#[test]
fn test_handle_id_different_for_distinct_handles() {
    let h1 = Handle::new(MyAsset::new(1));
    let h2 = Handle::new(MyAsset::new(2));
    assert_ne!(HandleId::from_handle(&h1), HandleId::from_handle(&h2));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p engine-asset`
Expected: All tests PASS (including new HandleId tests)

- [ ] **Step 4: Commit**

```bash
git add crates/engine-asset/src/asset.rs
git commit -m "feat(asset): add HandleId for Handle<T> identification"
```

---

### Task 3: TextureBridge core

**Files:**
- Modify: `crates/engine-render/Cargo.toml`
- Create: `crates/engine-render/src/texture_bridge.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: Add crossbeam-channel dependency**

In `crates/engine-render/Cargo.toml`, add to `[dependencies]`:

```toml
crossbeam-channel = "0.5"
```

- [ ] **Step 2: Add module declaration**

In `crates/engine-render/src/lib.rs`, add after line 10 (`pub mod texture_store;`):

```rust
pub mod texture_bridge;
```

- [ ] **Step 3: Create TextureBridge with tests**

Create `crates/engine-render/src/texture_bridge.rs`:

```rust
use crate::texture_store::TextureStore;
use engine_asset::asset::{Asset, Handle, HandleId};
use engine_asset::types::Texture;
use engine_core::event::EventChannel;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// State of a texture load request.
#[derive(Clone, Debug)]
pub enum LoadState {
    /// Background thread is decoding.
    Pending,
    /// Uploaded to GPU. Value is the TextureStore id.
    Ready(u64),
    /// Load failed.
    Failed(String),
}

/// Fired when a texture finishes loading (success or failure).
#[derive(Clone, Debug)]
pub struct TextureLoaded {
    pub handle_id: HandleId,
    pub result: Result<u64, String>,
}

struct LoadRequest {
    handle_id: HandleId,
    path: String,
}

/// Result from the background loading thread.
enum LoadResult {
    Success {
        handle_id: HandleId,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
    },
    Failure {
        handle_id: HandleId,
        error: String,
    },
}

/// Bridge between the asset system (`Handle<Texture>`) and the render system
/// (`TextureStore` with `u64` ids).
///
/// Loads textures asynchronously on a background thread and uploads them to
/// the GPU on the render thread via `flush()`.
pub struct TextureBridge {
    handle_to_id: HashMap<HandleId, u64>,
    states: HashMap<HandleId, LoadState>,
    completed_queue: crossbeam_channel::Receiver<LoadResult>,
    load_sender: crossbeam_channel::Sender<LoadRequest>,
    texture_store: TextureStore,
    pub on_loaded: EventChannel<TextureLoaded>,
}

impl TextureBridge {
    /// Create a new bridge. Spawns a background loading thread.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let (load_tx, load_rx) = crossbeam_channel::unbounded::<LoadRequest>();
        let (done_tx, done_rx) = crossbeam_channel::unbounded::<DecodedTexture>();

        std::thread::spawn(move || {
            for req in load_rx {
                let result = std::fs::read(&req.path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| {
                        image::load_from_memory(&bytes)
                            .map_err(|e| e.to_string())
                    });

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
                let _ = done_tx.send(load_result);
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

    /// Submit an async texture load request.
    pub fn request(&mut self, handle: &Handle<Texture>, path: &str) {
        let handle_id = HandleId::from_handle(handle);
        if self.states.contains_key(&handle_id) {
            return; // already requested
        }
        self.states.insert(handle_id, LoadState::Pending);
        let _ = self.load_sender.send(LoadRequest {
            handle_id,
            path: path.to_string(),
        });
    }

    /// Process completed texture uploads. Call once per frame before rendering.
    pub fn flush(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
    ) {
        while let Ok(result) = self.completed_queue.try_recv() {
            match result {
                LoadResult::Success { handle_id, pixels, width, height } => {
                    match self.texture_store.load_from_bytes(
                        device,
                        queue,
                        texture_layout,
                        &pixels,
                        width,
                        height,
                    ) {
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
                }
                LoadResult::Failure { handle_id, error } => {
                    self.states.insert(handle_id, LoadState::Failed(error.clone()));
                    self.on_loaded.emit(&TextureLoaded {
                        handle_id,
                        result: Err(error),
                    });
                }
            }
        }
    }

    /// Resolve a Handle<Texture> to a TextureStore u64 id.
    /// Returns fallback id (0) if not yet loaded.
    pub fn resolve(&self, handle: &Handle<Texture>) -> u64 {
        let handle_id = HandleId::from_handle(handle);
        self.handle_to_id.get(&handle_id).copied().unwrap_or(0)
    }

    /// Query the load state of a handle.
    pub fn state(&self, handle: &Handle<Texture>) -> &LoadState {
        let handle_id = HandleId::from_handle(handle);
        self.states
            .get(&handle_id)
            .unwrap_or(&LoadState::Failed("never requested".into()))
    }

    /// Access the underlying TextureStore.
    pub fn texture_store(&self) -> &TextureStore {
        &self.texture_store
    }

    /// Mutable access to the underlying TextureStore.
    pub fn texture_store_mut(&mut self) -> &mut TextureStore {
        &mut self.texture_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_unknown_returns_fallback() {
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

        let bridge = TextureBridge::new(&device, &queue, &layout);

        // Create a handle with a dummy texture
        let tex = Texture {
            id: "test".into(),
            width: 1,
            height: 1,
            data: vec![255, 0, 0, 255],
            channels: 4,
        };
        let handle = Handle::new(tex);

        // Should return fallback (0) for unrequested handle
        assert_eq!(bridge.resolve(&handle), 0);
    }

    #[test]
    fn test_request_sets_pending_state() {
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

        let mut bridge = TextureBridge::new(&device, &queue, &layout);

        let tex = Texture {
            id: "test".into(),
            width: 1,
            height: 1,
            data: vec![255, 0, 0, 255],
            channels: 4,
        };
        let handle = Handle::new(tex);

        bridge.request(&handle, "nonexistent_path.png");

        match bridge.state(&handle) {
            LoadState::Pending => {} // expected
            other => panic!("Expected Pending, got {:?}", other),
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p engine-render texture_bridge`
Expected: Tests pass (or fail on GPU init in CI — acceptable for wgpu tests)

- [ ] **Step 5: Commit**

```bash
git add crates/engine-render/Cargo.toml crates/engine-render/src/texture_bridge.rs crates/engine-render/src/lib.rs
git commit -m "feat(render): add TextureBridge async texture loader"
```

---

### Task 4: Sprite component改造

**Files:**
- Modify: `crates/engine-render/src/sprite.rs:1-11`

- [ ] **Step 1: Update Sprite to use Handle<Texture> and carry transform**

In `crates/engine-render/src/sprite.rs`, update imports and the `Sprite` struct:

Replace lines 1-11:

```rust
use crate::pipeline::sprite::SpriteVertex;
use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_math::{Mat4, Vec2};
use wgpu::util::DeviceExt;

pub struct Sprite {
    pub texture: Handle<Texture>,
    pub color: [f32; 4],
    pub size: Vec2,
    pub transform: Mat4,
    pub flip_x: bool,
    pub flip_y: bool,
}
```

`SpriteDraw` and `SpriteBatch` remain unchanged (they use `u64` internally).

- [ ] **Step 2: Update tests**

Update the `sprite_draw_default` helper and tests — `SpriteDraw` still uses `texture_id: u64`, so the tests remain valid. No changes needed to `SpriteDraw` or `SpriteBatch` tests.

- [ ] **Step 3: Run tests**

Run: `cargo test -p engine-render sprite`
Expected: All sprite tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/sprite.rs
git commit -m "refactor(render): Sprite uses Handle<Texture> instead of u64"
```

---

### Task 5: Renderer integration

**Files:**
- Modify: `crates/engine-render/src/renderer.rs`

- [ ] **Step 1: Remove texture_store from Renderer**

In `crates/engine-render/src/renderer.rs`, remove the `texture_store` field and its initialization.

Remove line 5: `use crate::texture_store::TextureStore;`

Remove from struct definition (line 38): `pub texture_store: TextureStore,`

Remove from `new()` (lines 90-91):
```rust
let texture_store =
    TextureStore::new(&device, &queue, &sprite_pipeline.texture_bind_group_layout);
```

Remove from `Self { ... }` block (line 100): `texture_store,`

- [ ] **Step 2: Update render_frame signature**

Change `render_frame` to accept `&[Sprite]` and `&mut TextureBridge`:

Replace the `render_frame` method signature and first section:

```rust
pub fn render_frame(
    &mut self,
    cameras: &[&crate::camera::Camera],
    all_sprites: &[crate::sprite::Sprite],
    bridge: &mut crate::texture_bridge::TextureBridge,
) -> Result<(), wgpu::SurfaceError> {
    use crate::camera::{Camera, RenderTarget};
    use crate::frustum::Frustum;
    use crate::sprite::{SpriteDraw, SpriteBatch};

    // Flush pending texture uploads
    bridge.flush(&self.device, &self.queue, &self.sprite_pipeline.texture_bind_group_layout);

    // Convert Sprite → SpriteDraw
    let sprite_draws: Vec<SpriteDraw> = all_sprites
        .iter()
        .map(|s| SpriteDraw {
            world_matrix: s.transform,
            color: s.color,
            size: s.size,
            texture_id: bridge.resolve(&s.texture),
            flip_x: s.flip_x,
            flip_y: s.flip_y,
        })
        .collect();

    // ... rest of the method uses sprite_draws instead of all_sprites
```

Update all references to `self.texture_store` inside `render_frame` to `bridge.texture_store()`:

- Line 250-253: `self.texture_store.get_render_target_view(key)` → `bridge.texture_store().get_render_target_view(key)`
- Line 266: `self.texture_store.get_bind_group(b.texture_id)` → `bridge.texture_store().get_bind_group(b.texture_id)`

- [ ] **Step 3: Update deprecated present() method**

The `present()` method also references `self.texture_store`. Update it similarly or leave it deprecated with a note that it doesn't support the bridge pattern.

- [ ] **Step 4: Run tests**

Run: `cargo test -p engine-render`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "refactor(render): Renderer uses TextureBridge, removes direct TextureStore"
```

---

### Task 6: Update sprite_demo example

**Files:**
- Modify: `crates/engine-core/examples/sprite_demo.rs`

- [ ] **Step 1: Rewrite example to use TextureBridge**

Replace `crates/engine-core/examples/sprite_demo.rs`:

```rust
use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_math::{Mat4, Vec2, Vec3};
use engine_render::camera::{Camera, Color, Viewport};
use engine_render::renderer::Renderer;
use engine_render::sprite::Sprite;
use engine_render::texture_bridge::TextureBridge;
use engine_window::{window::WindowConfig, window::create_window};
use log::info;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Sprite Demo — TextureBridge");

    let event_loop = EventLoop::new().unwrap();
    let window = std::sync::Arc::new(create_window(
        &WindowConfig {
            title: "Sprite Demo — TextureBridge".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        },
        &event_loop,
    ));

    let mut renderer = Renderer::new(window);

    // Create bridge
    let mut bridge = TextureBridge::new(
        &renderer.device,
        &renderer.queue,
        &renderer.sprite_pipeline.texture_bind_group_layout,
    );

    // Request texture load (async)
    let tex_asset = Texture {
        id: "test".into(),
        width: 1,
        height: 1,
        data: vec![0; 4],
        channels: 4,
    };
    let handle = Handle::new(tex_asset);
    bridge.request(&handle, "assets/test.png");

    // Register load-complete listener
    bridge.on_loaded.subscribe(|e| {
        info!("Texture loaded: {:?} → {:?}", e.handle_id, e.result);
    });

    // Create sprites (will show fallback until texture loads)
    let sprites = vec![
        Sprite {
            texture: handle.clone(),
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            transform: Mat4::from_translation(Vec3::new(200.0, 300.0, 0.0)),
            flip_x: false,
            flip_y: false,
        },
        Sprite {
            texture: handle.clone(),
            color: [0.0, 1.0, 0.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            transform: Mat4::from_translation(Vec3::new(600.0, 300.0, 0.0)),
            flip_x: false,
            flip_y: false,
        },
    ];

    // Cameras
    let mut main_camera = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
    main_camera.priority = 0;
    main_camera.clear_color = Some(Color::new(0.1, 0.1, 0.1, 1.0));

    let mut mini_camera = Camera::orthographic(0.0, 400.0, 300.0, 0.0);
    mini_camera.priority = 1;
    mini_camera.viewport = Viewport::Relative {
        x: 0.6,
        y: 0.0,
        width: 0.4,
        height: 0.4,
    };
    mini_camera.clear_color = Some(Color::new(0.2, 0.2, 0.3, 1.0));

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match &event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    renderer.resize(size.width, size.height);
                }
                _ => {}
            }

            if let Event::AboutToWait = event {
                let cameras: Vec<&Camera> = vec![&main_camera, &mini_camera];
                let _ = renderer.render_frame(&cameras, &sprites, &mut bridge);
            }
        })
        .unwrap();
}
```

- [ ] **Step 2: Run the example**

Run: `cargo run --example sprite_demo -p engine-core`
Expected: Window opens, shows fallback checkerboard initially, then the texture after loading

- [ ] **Step 3: Run full test suite**

Run: `cargo test --all`
Expected: No regressions

- [ ] **Step 4: Run clippy and format**

Run: `cargo clippy && cargo fmt`
Expected: No warnings

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core/examples/sprite_demo.rs
git commit -m "refactor(example): sprite_demo uses TextureBridge"
```

---

## Verification

After all tasks, run the full verification sequence:

```bash
cargo clippy
cargo fmt --check
cargo test --all
cargo run --example sprite_demo -p engine-core
```

Expected:
- No clippy warnings
- All tests pass
- Example window renders sprites with textures loaded via bridge
