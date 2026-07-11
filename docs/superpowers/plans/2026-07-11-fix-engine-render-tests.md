# Fix engine-render Test Hanging Issue

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent 29 GPU-dependent tests from hanging in headless/CI environments by adding `#[ignore]` annotations and creating a shared test helper.

**Architecture:** Add `#[ignore]` to all tests that require `wgpu::Instance`/`adapter`/`device`/`queue`. Create a shared `test_gpu` module in `engine-render/src/lib.rs` to eliminate duplicated GPU setup code across 6 modules. Tests can be run with `cargo test -- --ignored` on machines with GPU.

**Tech Stack:** Rust, wgpu, pollster

---

## File Structure

| File | Action | Purpose |
|------|--------|---------|
| `crates/engine-render/src/lib.rs` | Modify | Add `#[cfg(test)] pub mod test_gpu;` |
| `crates/engine-render/src/test_gpu.rs` | Create | Shared `create_test_device()` and `create_test_device_with_features()` helpers |
| `crates/engine-render/src/deferred.rs` | Modify | `#[ignore]` 4 GPU tests, use shared helper |
| `crates/engine-render/src/ibl.rs` | Modify | `#[ignore]` 1 GPU test, use shared helper |
| `crates/engine-render/src/particle.rs` | Modify | `#[ignore]` 4 GPU tests, use shared helper |
| `crates/engine-render/src/texture_bridge.rs` | Modify | `#[ignore]` 5 GPU tests, use shared helper |
| `crates/engine-render/src/texture_store.rs` | Modify | `#[ignore]` 6 GPU tests, use shared helper |
| `crates/engine-render/src/post_process.rs` | Modify | `#[ignore]` 7 GPU tests, use shared helper |
| `crates/engine-render/src/tilemap.rs` | Modify | `#[ignore]` 2 GPU tests, use shared helper |

---

### Task 1: Create shared GPU test helper module

**Files:**
- Create: `crates/engine-render/src/test_gpu.rs`
- Modify: `crates/engine-render/src/lib.rs`

- [ ] **Step 1: Create `test_gpu.rs` with shared helpers**

```rust
//! Shared GPU test helpers.
//!
//! Tests using these helpers require a real GPU and will hang in headless environments.
//! Run with: `cargo test -p engine-render -- --ignored`

use pollster::block_on;

/// Create a wgpu Device and Queue with default features and limits.
///
/// # Panics
/// Panics if no GPU adapter is available.
pub fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
    create_test_device_with_features(wgpu::Features::empty(), wgpu::Limits::default())
}

/// Create a wgpu Device and Queue with specific features and limits.
///
/// # Panics
/// Panics if no GPU adapter is available.
pub fn create_test_device_with_features(
    features: wgpu::Features,
    limits: wgpu::Limits,
) -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("Failed to find a GPU adapter");

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("test device"),
            required_features: features,
            required_limits: limits,
            ..Default::default()
        },
        None,
    ))
    .expect("Failed to create GPU device");

    (device, queue)
}
```

- [ ] **Step 2: Register the module in `lib.rs`**

Add near the top of `crates/engine-render/src/lib.rs`, after existing `#[cfg(test)]` modules:

```rust
#[cfg(test)]
pub mod test_gpu;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p engine-render`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/test_gpu.rs crates/engine-render/src/lib.rs
git commit -m "test(render): add shared GPU test helper module"
```

---

### Task 2: Add `#[ignore]` to deferred.rs GPU tests

**Files:**
- Modify: `crates/engine-render/src/deferred.rs:581-730`

- [ ] **Step 1: Add `#[ignore]` to 4 GPU tests and use shared helper**

Replace the private `create_test_device()` at line 586 with an import from the shared module, and add `#[ignore]` to each GPU test:

In the `#[cfg(test)] mod tests` block:
- Remove the local `create_test_device()` function (lines 586-611)
- Add `use crate::test_gpu::create_test_device_with_features;` at the top of the module
- Add `#[ignore]` before each of these 4 tests:
  - `test_gbuffer_creation` (line 635)
  - `test_gbuffer_resize` (line 680)
  - `test_gbuffer_bind_group_layout` (line 704)
  - `test_deferred_pass_creation` (line 714)

The deferred module needs `PUSH_CONSTANTS` feature, so the call becomes:
```rust
let (device, queue) = create_test_device_with_features(
    wgpu::Features::PUSH_CONSTANTS,
    wgpu::Limits {
        max_push_constant_size: 128,
        ..wgpu::Limits::default()
    },
);
```

- [ ] **Step 2: Verify pure tests still pass**

Run: `cargo test -p engine-render deferred::tests`
Expected: 2 tests pass (`test_geometry_pass_uniform_size`, `test_geometry_pass_uniform_default`), 4 tests reported as ignored

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/deferred.rs
git commit -m "test(render): ignore GPU-dependent deferred tests"
```

---

### Task 3: Add `#[ignore]` to ibl.rs GPU test

**Files:**
- Modify: `crates/engine-render/src/ibl.rs:214-260`

- [ ] **Step 1: Add `#[ignore]` and use shared helper**

In the `#[cfg(test)] mod tests` block:
- Add `use crate::test_gpu::create_test_device;` at the top
- Add `#[ignore]` before `test_ibl_bind_group_layout_creation` (line 240)
- Replace the inline wgpu setup in that test with:
```rust
let (device, _queue) = create_test_device();
```

- [ ] **Step 2: Verify pure tests still pass**

Run: `cargo test -p engine-render ibl::tests`
Expected: 7 tests pass, 1 ignored

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/ibl.rs
git commit -m "test(render): ignore GPU-dependent IBL test"
```

---

### Task 4: Add `#[ignore]` to particle.rs GPU tests

**Files:**
- Modify: `crates/engine-render/src/particle.rs:501-900`

- [ ] **Step 1: Add `#[ignore]` and use shared helper**

In the `#[cfg(test)] mod tests` block:
- Add `use crate::test_gpu::create_test_device;` at the top
- Add `#[ignore]` before each of these 4 tests:
  - `test_update_particles_integration` (line 637)
  - `test_inactive_emitter_produces_no_draws` (line 721)
  - `test_burst_emitter` (line 791)
  - `test_particles_die_after_lifetime` (line 871)
- Replace inline wgpu setup in each with:
```rust
let (device, queue) = create_test_device();
```

- [ ] **Step 2: Verify pure tests still pass**

Run: `cargo test -p engine-render particle::tests`
Expected: 8 tests pass, 4 ignored

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/particle.rs
git commit -m "test(render): ignore GPU-dependent particle tests"
```

---

### Task 5: Add `#[ignore]` to texture_bridge.rs GPU tests

**Files:**
- Modify: `crates/engine-render/src/texture_bridge.rs:258-415`

- [ ] **Step 1: Add `#[ignore]` and use shared helper**

In the `#[cfg(test)] mod tests` block (line 258):
- Add `use crate::test_gpu::{create_test_device, test_layout};` — wait, `test_layout` is local. Keep the local `test_layout` helper but replace `test_device` with the shared one.
- Remove the local `test_device()` function (lines 263-285)
- Add `use crate::test_gpu::create_test_device;`
- Add `#[ignore]` before each of these 5 tests:
  - `test_resolve_unknown_returns_fallback` (line 312)
  - `test_auto_sync_uploads_in_memory_data` (line 329)
  - `test_auto_sync_skips_empty_data_falls_back_to_disk` (line 353)
  - `test_request_sets_pending_state` (line 376)
  - `test_request_idempotent` (line 397)
- Replace `test_device()` calls with `create_test_device()`

The `test_layout()` helper (line 287) stays local since it's specific to this module's bind group layout needs.

- [ ] **Step 2: Verify pure tests still pass**

Run: `cargo test -p engine-render texture_bridge`
Expected: 5 event_channel tests pass, 5 GPU tests ignored

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/texture_bridge.rs
git commit -m "test(render): ignore GPU-dependent texture_bridge tests"
```

---

### Task 6: Add `#[ignore]` to texture_store.rs GPU tests

**Files:**
- Modify: `crates/engine-render/src/texture_store.rs:311-430`

- [ ] **Step 1: Add `#[ignore]` and use shared helper**

In the `#[cfg(test)] mod tests` block (line 311):
- Remove the local `test_device()` function (lines 315-336)
- Add `use crate::test_gpu::create_test_device;`
- Keep the local `test_layout()` helper
- Add `#[ignore]` before each of these 6 tests:
  - `test_fallback_exists` (line 363)
  - `test_invalid_id_returns_fallback` (line 372)
  - `test_load_from_bytes` (line 382)
  - `test_unload` (line 396)
  - `test_cannot_unload_fallback` (line 409)
  - `test_load_from_image_data` (line 418)
- Replace `test_device()` calls with `create_test_device()`

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p engine-render`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/texture_store.rs
git commit -m "test(render): ignore GPU-dependent texture_store tests"
```

---

### Task 7: Add `#[ignore]` to post_process.rs GPU tests

**Files:**
- Modify: `crates/engine-render/src/post_process.rs:1026-1200`

- [ ] **Step 1: Add `#[ignore]` and use shared helper**

In the `#[cfg(test)] mod tests` block (line 1026):
- Remove the local `create_test_device()` function (lines 1030-1053)
- Add `use crate::test_gpu::create_test_device;`
- Add `#[ignore]` before each of these 7 tests:
  - `test_hdr_framebuffer_creation` (line 1055)
  - `test_hdr_framebuffer_resize` (line 1067)
  - `test_tonemapping_pass_creation` (line 1106)
  - `test_post_process_chain_creation` (line 1114)
  - `test_post_process_chain_minimal` (line 1132)
  - `test_post_process_chain_resize` (line 1152)
  - `test_composite_pass_creation` (line 1177)
- Replace `create_test_device()` calls with the shared version

- [ ] **Step 2: Verify pure tests still pass**

Run: `cargo test -p engine-render post_process::tests`
Expected: 5 tests pass, 7 ignored

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/post_process.rs
git commit -m "test(render): ignore GPU-dependent post_process tests"
```

---

### Task 8: Add `#[ignore]` to tilemap.rs GPU tests

**Files:**
- Modify: `crates/engine-render/src/tilemap.rs:343-600`

- [ ] **Step 1: Add `#[ignore]` and use shared helper**

In the `#[cfg(test)] mod tests` block (line 343):
- Add `use crate::test_gpu::create_test_device;`
- Add `#[ignore]` before each of these 2 tests:
  - `test_collect_tilemap_draws_orphan_layers` (line 501)
  - `test_collect_tilemap_draws_sorts_by_z_order` (line 573)
- Replace inline wgpu setup in each with:
```rust
let (device, queue) = create_test_device();
```

- [ ] **Step 2: Verify pure tests still pass**

Run: `cargo test -p engine-render tilemap::tests`
Expected: 10 tests pass, 2 ignored

- [ ] **Step 3: Commit**

```bash
git add crates/engine-render/src/tilemap.rs
git commit -m "test(render): ignore GPU-dependent tilemap tests"
```

---

### Task 9: Final verification

- [ ] **Step 1: Run all non-GPU tests**

Run: `cargo test -p engine-render`
Expected: All ~159 non-GPU tests pass, 29 tests reported as ignored, no hangs

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p engine-render`
Expected: No warnings

- [ ] **Step 3: Run full workspace test**

Run: `cargo test`
Expected: All tests across all crates pass (engine-render tests complete without hanging)

- [ ] **Step 4: Final commit if needed**

```bash
git add -A
git commit -m "test(render): complete GPU test isolation for CI compatibility"
```
