---
feature: phase16-unity-animation-demo
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 844c8da..2585cbc
---

# Phase 16 — Unity 动画权威路径示例

## Report

**What was built** — Headless example `crates/engine-core/examples/unity_animation_demo.rs` wires phases 12–15 end-to-end: register sample scripts → author `AnimationClip` + `AnimationClipPlayer` on a GameObject → `SaveSceneJsonPrepared` → `SceneRuntime::load_scene_json` → lifecycle `tick` drives `Update` → `apply_clip_pose` writes World local pose. Prints `unity_world_primary_feature()` (default true), start/end poses, and asserts X advances toward the clip target (8.0). README「运行示例」documents `cargo run -p engine-core --example unity_animation_demo`.

**Verification** —
| Command | Result |
|---------|--------|
| `cargo run -p engine-core --example unity_animation_demo` | **PASS** — feature `true`; pose `0 → 8.0`; Delta X 8.000; asserts held (end.x > start+0.5 **and** \|end.x−8.0\|<0.05) |
| `cargo build -p engine-core --examples` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo fmt -p engine-core --check` | PASS (after fmt) |

**Journey log** —
1. Demo proves AnimationClipPlayer + prepared I/O + SceneRuntime tick without GPU.
2. Non-looping clip parks at duration (8.0) — asserted, not print-only.
3. Happy-path smoke for phases 12–15 storage-authority animation.
4. Spec “门禁” now records demo + lib test + fmt (not only demo run).
5. git merge/push not handled per user preference.

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
