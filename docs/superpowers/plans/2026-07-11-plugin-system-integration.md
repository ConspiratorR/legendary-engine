# Plugin System Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the existing `PluginLoader` into the main app flow so users can load dynamic plugins at runtime.

**Architecture:** Add `load_dynamic_plugins()` method to `AppBuilder` that uses `PluginLoader` to scan a directory, load all registered plugins, and call their `build()` methods. Add a `DynamicPluginDemo` example that demonstrates the full flow.

**Tech Stack:** libloading 0.8, existing PluginLoader, serde_json

---

## Context

The `plugin_loader.rs` (301 lines) already implements:
- `PluginManifest` — JSON manifest with name, version, entry_point
- `DynamicPlugin` — loads shared library, calls entry point, gets `Box<dyn Plugin>`
- `PluginRegistry` — tracks installed plugins (name → path)
- `PluginLoader` — orchestrates install/uninstall/load_all/register_all

**What's missing:** Integration with `AppBuilder` so users can actually use it.

---

## File Structure

| File | Action | Purpose |
|------|--------|---------|
| `crates/engine-core/src/app.rs` | Modify | Add `load_dynamic_plugins()` method to `AppBuilder` |
| `crates/engine-core/examples/plugin_demo.rs` | Modify | Update to demonstrate dynamic plugin loading |
| `examples/test-plugin/src/lib.rs` | Verify | Ensure test plugin compiles correctly |

---

### Task 1: Add load_dynamic_plugins to AppBuilder

**Files:**
- Modify: `crates/engine-core/src/app.rs`

- [ ] **Step 1: Read app.rs to find AppBuilder**

Read `crates/engine-core/src/app.rs` to find the `AppBuilder` struct and its methods.

- [ ] **Step 2: Add load_dynamic_plugins method**

Add this method to the `impl AppBuilder` block:

```rust
/// Load all dynamic plugins from a directory.
///
/// Each plugin directory must contain a `plugin.json` manifest and
/// a shared library (`.dll`, `.so`, or `.dylib`).
///
/// # Safety
/// This function loads shared libraries and calls their entry points.
/// Plugins must be compiled for the correct target platform.
pub fn load_dynamic_plugins(&mut self, plugins_dir: &Path) -> Result<&mut Self, Box<dyn std::error::Error>> {
    use crate::plugin_loader::PluginLoader;

    let mut loader = PluginLoader::new(plugins_dir.join("registry.json"));
    loader.load_all()?;
    loader.register_all(self);
    Ok(self)
}
```

Also add `use std::path::Path;` at the top of the file if not already present.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-core`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/src/app.rs
git commit -m "feat(core): add load_dynamic_plugins method to AppBuilder"
```

---

### Task 2: Update plugin_demo example

**Files:**
- Modify: `crates/engine-core/examples/plugin_demo.rs`

- [ ] **Step 1: Read current plugin_demo.rs**

Read the existing example to understand its structure.

- [ ] **Step 2: Add dynamic plugin loading section**

Add a section that demonstrates loading plugins from a directory:

```rust
// Dynamic plugin loading example
let plugins_dir = std::path::Path::new("plugins");
if plugins_dir.exists() {
    match app_builder.load_dynamic_plugins(plugins_dir) {
        Ok(_) => println!("Loaded dynamic plugins from {:?}", plugins_dir),
        Err(e) => println!("No dynamic plugins loaded: {e}"),
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p engine-core --example plugin_demo`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/engine-core/examples/plugin_demo.rs
git commit -m "feat(core): update plugin_demo to show dynamic loading"
```

---

### Task 3: Add unit tests for load_dynamic_plugins

**Files:**
- Modify: `crates/engine-core/src/app.rs` (add tests module)

- [ ] **Step 1: Add test for load_dynamic_plugins with empty dir**

In the `#[cfg(test)]` module of `app.rs`, add:

```rust
#[test]
fn test_load_dynamic_plugins_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let mut builder = AppBuilder::new();
    let result = builder.load_dynamic_plugins(dir.path());
    assert!(result.is_ok());
}
```

Add `tempfile` to `[dev-dependencies]` in `crates/engine-core/Cargo.toml`:

```toml
[dev-dependencies]
engine-framework = { path = "../engine-framework" }
engine-physics = { path = "../engine-physics" }
tempfile = "3"
```

- [ ] **Step 2: Run test**

Run: `cargo test -p engine-core test_load_dynamic_plugins_empty_dir`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/engine-core/src/app.rs crates/engine-core/Cargo.toml
git commit -m "test(core): add unit test for load_dynamic_plugins"
```

---

### Task 4: Final verification

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
git commit -m "feat: integrate dynamic plugin loading into AppBuilder"
```
