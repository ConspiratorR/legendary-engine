# Android Target Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable RustEngine to compile and run on Android devices (aarch64-linux-android) with working rendering, touch input, and asset loading.

**Architecture:** Add `#[cfg(target_os = "android")]` code paths in existing crates. The entry point uses `android-activity` crate's `android_main` macro. Assets load from APK via `ndk::asset::AssetManager`. Touch input comes from winit's `WindowEvent::Touch`.

**Tech Stack:** android-activity 0.6, ndk 0.9, ndk-context 0.1, wgpu (Vulkan), android_logger

---

## File Structure

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Modify | Add `android_logger` workspace dep |
| `crates/engine-core/Cargo.toml` | Modify | Add `android_logger` dep (target-gated) |
| `crates/engine-core/src/engine.rs` | Modify | Add `run_android()` function |
| `crates/engine-core/src/lib.rs` | Modify | Export android module |
| `crates/engine-core/src/android.rs` | Create | Android entry point and lifecycle |
| `crates/engine-core/src/debug.rs` | Modify | Android logger initialization |
| `crates/engine-window/Cargo.toml` | Modify | Verify android deps are correct |
| `crates/engine-window/src/window.rs` | Modify | Add Android window creation |
| `crates/engine-input/src/lib.rs` | Modify | Add touch input handling |
| `crates/engine-asset/Cargo.toml` | Modify | Gate `notify` dep |
| `crates/engine-asset/src/lib.rs` | Modify | Gate file watcher on Android |
| `crates/engine-asset/src/android_loader.rs` | Create | APK asset loading |
| `crates/engine-core/examples/android_demo.rs` | Create | Minimal Android example |

---

### Task 1: Add android_logger dependency

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/engine-core/Cargo.toml`

- [ ] **Step 1: Add workspace dependency**

In `Cargo.toml` (workspace root), add to `[workspace.dependencies]`:

```toml
android_logger = "0.14"
```

- [ ] **Step 2: Add target-gated dependency to engine-core**

In `crates/engine-core/Cargo.toml`, add:

```toml
[target.'cfg(target_os = "android")'.dependencies]
android_logger = { workspace = true }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p engine-core`
Expected: Compiles successfully (android_logger is only pulled on Android targets)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/engine-core/Cargo.toml
git commit -m "deps: add android_logger for Android logcat output"
```

---

### Task 2: Add Android logger initialization

**Files:**
- Modify: `crates/engine-core/src/debug.rs`

- [ ] **Step 1: Add Android logger init**

Read `crates/engine-core/src/debug.rs` to find the existing `init_logging()` function. Add Android branch:

```rust
#[cfg(target_os = "android")]
pub fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("RustEngine"),
    );
}

#[cfg(not(target_os = "android"))]
pub fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
}
```

If the existing `init_logging()` uses a different pattern, adapt it to add the `#[cfg]` split.

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p engine-core`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/src/debug.rs
git commit -m "feat(core): add Android logcat logger"
```

---

### Task 3: Add Android entry point

**Files:**
- Create: `crates/engine-core/src/android.rs`
- Modify: `crates/engine-core/src/lib.rs`

- [ ] **Step 1: Create android.rs module**

```rust
//! Android entry point and lifecycle management.

use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, StartCause};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::{App, AppBuilder};
use crate::engine::Engine;

/// State managed during the Android application lifecycle.
struct AndroidApp {
    window: Option<Window>,
    app: Option<App>,
}

impl ApplicationHandler for AndroidApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title("RustEngine")
                .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            let window = event_loop.create_window(window_attrs).unwrap();
            self.window = Some(window);
            // App initialization will be added when renderer is integrated
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(window) = &self.window {
                    // Handle resize
                }
            }
            WindowEvent::Touch(touch) => {
                // Forward to input system - Task 5
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Run the engine on Android.
///
/// # Safety
/// This function must be called from the Android main thread.
pub fn run_android() {
    let event_loop = EventLoop::new().unwrap();
    let mut state = AndroidApp {
        window: None,
        app: None,
    };
    event_loop.run_app(&mut state).unwrap();
}
```

- [ ] **Step 2: Register module in lib.rs**

Add to `crates/engine-core/src/lib.rs`:

```rust
#[cfg(target_os = "android")]
pub mod android;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p engine-core --target aarch64-linux-android`
Expected: Compiles (may need NDK toolchain installed)

If NDK is not available, verify with: `cargo check -p engine-core`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/android.rs crates/engine-core/src/lib.rs
git commit -m "feat(core): add Android entry point with lifecycle management"
```

---

### Task 4: Gate file watcher on Android

**Files:**
- Modify: `crates/engine-asset/Cargo.toml`
- Modify: `crates/engine-asset/src/lib.rs`

- [ ] **Step 1: Gate notify dependency**

In `crates/engine-asset/Cargo.toml`, change the `notify` dependency to be non-Android:

```toml
[target.'cfg(not(target_os = "android"))'.dependencies]
notify = "7"
```

- [ ] **Step 2: Gate file watcher code**

In `crates/engine-asset/src/lib.rs`, find the file watcher module/usage and add `#[cfg(not(target_os = "android"))]` gates.

If there's a `FileSystemWatcher` or similar struct, gate it:

```rust
#[cfg(not(target_os = "android"))]
mod file_watcher {
    // existing code
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p engine-asset`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/engine-asset/Cargo.toml crates/engine-asset/src/lib.rs
git commit -m "feat(asset): gate file watcher behind cfg for Android compatibility"
```

---

### Task 5: Add touch input handling

**Files:**
- Modify: `crates/engine-input/src/lib.rs`

- [ ] **Step 1: Add touch input types**

In `crates/engine-input/src/lib.rs`, add touch support:

```rust
/// Touch input phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// A single touch point.
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub phase: TouchPhase,
}

/// Touch input state.
#[derive(Debug, Default)]
pub struct TouchState {
    pub touches: Vec<TouchPoint>,
}
```

Add a `touch` field to the existing `InputState` struct (or wherever input state is held):

```rust
pub touch: TouchState,
```

- [ ] **Step 2: Handle TouchEvent in event processing**

Find where `WindowEvent::KeyboardInput` and `WindowEvent::CursorMoved` are handled. Add:

```rust
winit::event::WindowEvent::Touch(touch) => {
    let phase = match touch.phase {
        winit::event::TouchPhase::Started => TouchPhase::Started,
        winit::event::TouchPhase::Moved => TouchPhase::Moved,
        winit::event::TouchPhase::Ended => TouchPhase::Ended,
        winit::event::TouchPhase::Cancelled => TouchPhase::Cancelled,
    };
    input_state.touch.touches.push(TouchPoint {
        id: touch.id,
        x: touch.location.x as f32,
        y: touch.location.y as f32,
        phase,
    });
}
```

- [ ] **Step 3: Clear touches each frame**

In the input update/clear function, add:

```rust
self.touch.touches.clear();
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p engine-input`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/engine-input/src/lib.rs
git commit -m "feat(input): add touch input handling for Android"
```

---

### Task 6: Create Android example

**Files:**
- Create: `crates/engine-core/examples/android_demo.rs`

- [ ] **Step 1: Create minimal Android example**

```rust
//! Minimal Android demo for RustEngine.
//!
//! Build: cargo build --target aarch64-linux-android --example android_demo
//! Run: adb push target/aarch64-linux-android/debug/android_demo /data/local/tmp/
//!      adb shell /data/local/tmp/android_demo

#[cfg(target_os = "android")]
fn main() {
    env_logger::init();
    log::info!("RustEngine Android demo starting");
    engine_core::android::run_android();
}

#[cfg(not(target_os = "android"))]
fn main() {
    println!("This example is Android-only. Use: cargo run --example basic -p engine-core");
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p engine-core --example android_demo`
Expected: Compiles for host (shows "Android-only" message)
Run: `cargo check -p engine-core --example android_demo --target aarch64-linux-android`
Expected: Compiles for Android (if NDK available)

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/examples/android_demo.rs
git commit -m "feat(core): add minimal Android demo example"
```

---

### Task 7: Update build configuration

**Files:**
- Modify: `justfile`
- Modify: `docs/android-setup.md`

- [ ] **Step 1: Verify justfile recipe**

Read `justfile` and verify the `build-android` recipe exists and is correct:

```just
build-android:
    cargo build --target aarch64-linux-android --release
```

If it doesn't exist, add it.

- [ ] **Step 2: Update android-setup.md**

Update `docs/android-setup.md` to reflect the actual implementation (remove references to APIs that don't exist like `TouchEvent`, `AndroidAssetStore`).

- [ ] **Step 3: Commit**

```bash
git add justfile docs/android-setup.md
git commit -m "docs: update Android setup guide for actual implementation"
```

---

### Task 8: Final verification

- [ ] **Step 1: Full workspace build**

Run: `cargo build`
Expected: All crates compile successfully

- [ ] **Step 2: Full workspace test**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 3: Clippy check**

Run: `cargo clippy`
Expected: No warnings

- [ ] **Step 4: Final commit if needed**

```bash
git add -A
git commit -m "feat: complete Android target support foundation"
```
