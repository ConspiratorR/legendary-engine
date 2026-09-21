---
feature: phase36-wasm-scene-runtime-demo
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 72ac0d4..HEAD
---

# Phase 36 — WASM SceneRuntime web-demo 切片

## Report

**What was built** — `examples/web-demo` depends on `engine-core` (default-features=false, `unity-world-primary`). New wasm export `scene_runtime_json_smoke()` loads inline SceneData via `LoadSceneJson`, ticks SceneRuntime, returns Probe pose + feature flag string. Called from web-demo `start()` for browser log. Native `engine-core --lib` unchanged; wasm32 build of web-demo **PASS**.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo build --manifest-path examples/web-demo/Cargo.toml --target wasm32-unknown-unknown --lib` | **PASS** |
| `cargo test -p engine-core --lib` | PASS 261 |

**Journey log** —
1. web-demo was standalone wgpu/winit; engine-core wasm dep is feasible (phase 21 gates).
2. SceneRuntime **JSON smoke** is compile+logic evidence, not full browser lifecycle.
3. git merge/push not handled per user preference.

## [S1] Problem
web-demo had no SceneRuntime/wasm evidence.

## [S2] Design
engine-core dep + json smoke export + docs.

## [S3] Out of Scope
Full browser SceneRuntime UI, Android, git merge/push

## Tasks

- [x] T1: web-demo SceneRuntime smoke + 编译 (covers: S2)
- [x] T2: 文档 + finalize (covers: S2; depends: T1)
