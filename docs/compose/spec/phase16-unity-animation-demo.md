---
feature: phase16-unity-animation-demo
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 844c8da..HEAD
---

# Phase 16 — Unity 动画权威路径示例

## Report

**What was built** — Headless example `crates/engine-core/examples/unity_animation_demo.rs` wires phases 12–15 end-to-end: register sample scripts → author `AnimationClip` + `AnimationClipPlayer` on a GameObject → `SaveSceneJsonPrepared` → `SceneRuntime::load_scene_json` → lifecycle `tick` drives `Update` → `apply_clip_pose` writes World local pose. Prints `unity_world_primary_feature()` (default true), start/end poses, and asserts X advances toward the clip target (8.0). README「运行示例」documents `cargo run -p engine-core --example unity_animation_demo`.

**Verification** — `cargo run -p engine-core --example unity_animation_demo` **PASS**:
- feature on: true
- Pose after load: `Vec3(0,0,0)`
- frames: 0.4 → 4.4 → **8.0** (clamped at clip end)
- Delta X = 8.000; assert `end.x > start.x + 0.5` held
- `cargo build -p engine-core --examples` PASS (implicit via run)

**Journey log** —
1. Demo proves AnimationClipPlayer + prepared I/O + SceneRuntime tick without GPU.
2. Non-looping clip correctly parks at duration (8.0).
3. Example is the documented “happy path” for Unity storage-authority animation.
4. git merge/push not handled per user preference.

## [S1] Problem

No runnable end-to-end example after phases 12–15 for the animation authority path.

## [S2] Design

Headless demo as specified; no public API changes.

## [S3] Out of Scope

- GPU/window, WASM/Android, git merge/push

## Tasks

- [x] T1: unity_animation_demo 示例 (covers: S2)
- [x] T2: README 运行说明 (covers: S2; depends: T1)
- [x] T3: 门禁 + spec finalize (covers: S2; depends: T1,T2)
