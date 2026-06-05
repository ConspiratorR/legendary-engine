# engine-core Coupling Audit

**Date:** 2026-06-05
**Crate:** `engine-core`
**Role:** Central "God crate" — depends on 7 other engine crates.

## Dependencies

### 1. `engine-ecs` (required)

**Why:** Core ECS World, Schedule, ParallelSchedule, system trait. Used by `App`, `AppBuilder`, and all systems.

**Could be optional?** No — this is the foundation. Without ECS there is no app.

**Could be inverted?** Partially. The `App`/`AppBuilder` could be generic over a `World` trait, but the cost outweighs the benefit for a game engine where ECS is always present.

**Coupling risk:** HIGH — `App` directly exposes `World` and `Schedule` as public fields. Any ECS API change ripples into engine-core.

---

### 2. `engine-input` (required)

**Why:** `InputManager` is inserted as a default resource in `AppBuilder::new()` and `App::new()`. Also used by `ActionPlugin` (`ActionMap`).

**Could be optional?** Yes — could be behind a feature flag. Headless/server builds don't need input.

**Could be inverted?** Yes — `InputManager` could be registered via a plugin rather than hard-coded in `AppBuilder::new()`. The `ActionPlugin` already does this correctly.

**Coupling risk:** MEDIUM — the hard-coded `InputManager` insertion in `AppBuilder::new()` is unnecessary coupling. A `InputPlugin` would be cleaner.

---

### 3. `engine-render` (required)

**Why:** `Renderer` is stored as an optional field in `App`. Used for `set_renderer()`, `renderer()`, `renderer_mut()`, `split_renderer_mut()`, `split_renderer_ref()`.

**Could be optional?** Yes — could be behind a feature flag. Server builds don't need rendering.

**Could be inverted?** Yes — `Renderer` could be stored as a resource via a `RenderPlugin` rather than a dedicated field on `App`. This would remove the `engine-render` dependency entirely from the core struct.

**Coupling risk:** MEDIUM — the `Renderer` field on `App` forces the dependency. Moving to a plugin/resource pattern would decouple it.

---

### 4. `engine-scene` (required)

**Why:** Re-exported via `EngineError` conversion (`SceneError`). Used by the error enum.

**Could be optional?** Yes — only needed if scene management is used.

**Could be inverted?** Yes — the `From<SceneError>` impl could be behind a feature flag.

**Coupling risk:** LOW — only used for error conversion. Minimal surface area.

---

### 5. `engine-window` (required)

**Why:** Used by `engine.rs::run_default()` to create a window via `create_window()` and `WindowConfig`.

**Could be optional?** Yes — only needed for the default windowed entry point. Headless builds skip it.

**Could be inverted?** Yes — `run_default()` could be moved to a separate crate or behind a feature flag. The `Engine` struct doesn't need it.

**Coupling risk:** LOW — only used in `engine.rs`, which is a convenience function.

---

### 6. `engine-math` (required)

**Why:** `Vec3` used by `Transform`. `MathError` used by `EngineError`.

**Could be optional?** No — math is fundamental.

**Could be inverted?** No — math types are value types used everywhere.

**Coupling risk:** LOW — stable, well-defined API.

---

### 7. `engine-audio` (optional, behind `audio` feature)

**Why:** `AudioError` conversion in `EngineError`.

**Could be optional?** Already optional via feature flag. ✓

**Could be inverted?** Already inverted via feature flag. ✓

**Coupling risk:** NONE — properly decoupled.

---

### 8. `thiserror` (external)

**Why:** Used by `EngineError` for `#[derive(Error)]`.

**Could be optional?** No — standard error handling dependency.

**Coupling risk:** NONE — stable external crate.

---

### 9. `winit` (external)

**Why:** Used by `engine.rs::run_default()` for the event loop.

**Could be optional?** Yes — only needed for the default windowed entry point.

**Could be inverted?** Yes — `run_default()` should be behind a feature flag or in a separate crate.

**Coupling risk:** LOW — only used in one function.

---

### 10. `log` + `env_logger` (external)

**Why:** Used by `debug.rs` for `env_logger` initialization.

**Could be optional?** Yes — could be behind a feature flag.

**Could be inverted?** Yes — logging initialization could be delegated to a plugin.

**Coupling risk:** NONE — standard Rust logging ecosystem.

---

## Summary

| Dependency | Required? | Could Invert? | Coupling Risk |
|------------|-----------|---------------|---------------|
| engine-ecs | Yes | Partially | HIGH |
| engine-input | No | Yes | MEDIUM |
| engine-render | No | Yes | MEDIUM |
| engine-scene | No | Yes | LOW |
| engine-window | No | Yes | LOW |
| engine-math | Yes | No | LOW |
| engine-audio | Optional | Already done | NONE |
| thiserror | Yes | No | NONE |
| winit | No | Yes | LOW |
| log/env_logger | No | Yes | NONE |

## Recommendations

1. **Make `engine-input` optional** — move `InputManager` insertion to an `InputPlugin`. The `AppBuilder::new()` should not hard-code input.

2. **Make `engine-render` optional** — store `Renderer` as a resource via `RenderPlugin` instead of a dedicated field on `App`. This removes the most complex dependency.

3. **Make `engine-window` and `winit` optional** — move `run_default()` behind a `window` feature flag.

4. **Make `engine-scene` optional** — feature-gate the `SceneError` conversion.

5. **Consider splitting** — `engine-core` could be split into `engine-app` (AppBuilder/App/Plugin) and `engine-core` (time/config/logger/profiler/math). The app layer would depend on ECS; the core layer would not.
