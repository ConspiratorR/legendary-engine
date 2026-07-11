# Mod System Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the existing `ModLoader` from `engine-script` into the engine's plugin system so WASM mods can be loaded and executed at runtime.

**Architecture:** Create a `ModPlugin` that wraps `ModLoader` and registers it as a resource. Add a system that calls `run_systems()` each frame. Wire lifecycle hooks (init/update) for WASM mods.

**Tech Stack:** wasmtime 45, existing ModLoader, engine-ecs System trait

---

## Context

`engine-script/src/mod_system.rs` (278 lines) already implements:
- `ModManifest` — JSON manifest with name, version, entry_point, dependencies
- `Mod` — loaded mod with manifest, WasmSystem, WasmSandbox
- `ModLoader` — scans directories, resolves dependencies (topological sort), loads WASM modules
- `ModLoadError` — comprehensive error types

**What's missing:** Integration with `AppBuilder` so users can load and run mods.

---

## File Structure

| File | Action | Purpose |
|------|--------|---------|
| `crates/engine-script/src/mod_plugin.rs` | Create | ModPlugin that wraps ModLoader |
| `crates/engine-script/src/lib.rs` | Modify | Add mod_plugin module |
| `crates/engine-core/examples/mod_demo.rs` | Modify | Update to demonstrate mod loading |
| `crates/engine-script/Cargo.toml` | Verify | Ensure engine-core dependency exists |

---

### Task 1: Create ModPlugin

**Files:**
- Create: `crates/engine-script/src/mod_plugin.rs`

- [ ] **Step 1: Create mod_plugin.rs**

```rust
//! Plugin for loading and running WASM mods.

use crate::mod_system::{ModLoader, ModLoadError};
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Plugin that loads and manages WASM mods.
pub struct ModPlugin {
    mods_dir: PathBuf,
}

impl ModPlugin {
    /// Create a new ModPlugin that loads mods from the given directory.
    pub fn new(mods_dir: impl Into<PathBuf>) -> Self {
        Self {
            mods_dir: mods_dir.into(),
        }
    }
}

impl Plugin for ModPlugin {
    fn build(&self, app: &mut AppBuilder) {
        match ModLoader::new(self.mods_dir.clone()) {
            Ok(loader) => {
                let mut loader = loader;
                if let Err(e) = loader.load_all() {
                    log::warn!("Failed to load some mods: {e}");
                }
                log::info!(
                    "ModPlugin: loaded {} mods from {:?}",
                    loader.loaded_mods().len(),
                    self.mods_dir
                );
                app.insert_resource(Arc::new(RwLock::new(loader)));
            }
            Err(e) => {
                log::warn!("ModPlugin: failed to initialize ModLoader: {e}");
            }
        }
    }
}

/// System that runs all loaded WASM mods each frame.
pub fn mod_update_system(world: &mut engine_ecs::world::World) {
    let Some(loader) = world.get_resource::<Arc<RwLock<ModLoader>>>() else {
        return;
    };
    let Ok(mut loader) = loader.write() else {
        return;
    };
    if let Some(dt) = world.get_resource::<engine_core::time::Time>() {
        let delta = dt.delta_seconds();
        loader.run_systems(delta);
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-script`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/engine-script/src/mod_plugin.rs
git commit -m "feat(script): add ModPlugin for WASM mod loading"
```

---

### Task 2: Register mod_plugin module

**Files:**
- Modify: `crates/engine-script/src/lib.rs`

- [ ] **Step 1: Add module declaration**

In `crates/engine-script/src/lib.rs`, add:

```rust
pub mod mod_plugin;
```

Also add to the prelude:

```rust
pub use mod_plugin::{ModPlugin, mod_update_system};
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p engine-script`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/engine-script/src/lib.rs
git commit -m "feat(script): register mod_plugin module"
```

---

### Task 3: Update mod_demo example

**Files:**
- Modify: `crates/engine-core/examples/mod_demo.rs`

- [ ] **Step 1: Read current mod_demo.rs**

Read the existing example to understand its structure.

- [ ] **Step 2: Rewrite mod_demo to use ModPlugin**

```rust
//! Mod System Demo
//!
//! Demonstrates how to load and run WASM mods.
//!
//! Usage:
//! ```
//! cargo run --example mod_demo -p engine-core
//! ```

use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_script::{ModPlugin, mod_update_system};

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Mod System Demo ===");
    println!();

    let mut app = AppBuilder::new();
    app.add_plugin(CorePlugins);

    // Load WASM mods from the mods directory
    let mods_dir = std::path::Path::new("mods");
    if mods_dir.exists() {
        println!("Loading WASM mods from {:?}...", mods_dir);
        app.add_plugin(ModPlugin::new(mods_dir));
        app.add_system(mod_update_system);
    } else {
        println!("No 'mods' directory found. Skipping mod loading.");
        println!("To load WASM mods, create a 'mods' directory with mod subdirectories.");
        println!("Each mod directory should contain:");
        println!("  - mod.json (manifest)");
        println!("  - <entry_point>.wasm (compiled WASM module)");
    }

    // Build and run a few frames
    let mut app = app.build();
    for _ in 0..3 {
        app.run();
    }

    println!();
    println!("Demo complete!");
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-core --example mod_demo`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/mod_demo.rs
git commit -m "feat(core): update mod_demo to use ModPlugin"
```

---

### Task 4: Add unit test for ModPlugin

**Files:**
- Modify: `crates/engine-script/src/mod_plugin.rs`

- [ ] **Step 1: Add test module**

At the bottom of `mod_plugin.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_plugin_creation() {
        let plugin = ModPlugin::new("/tmp/test_mods");
        assert_eq!(plugin.mods_dir, std::path::PathBuf::from("/tmp/test_mods"));
    }

    #[test]
    fn test_mod_plugin_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = ModPlugin::new(dir.path());
        let mut app = AppBuilder::new();
        // Should not panic, just log a warning
        plugin.build(&mut app);
    }
}
```

Add `tempfile` to `[dev-dependencies]` in `crates/engine-script/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p engine-script mod_plugin`
Expected: Tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/engine-script/src/mod_plugin.rs crates/engine-script/Cargo.toml
git commit -m "test(script): add unit tests for ModPlugin"
```

---

### Task 5: Final verification

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
git commit -m "feat: integrate WASM mod system into engine"
```
