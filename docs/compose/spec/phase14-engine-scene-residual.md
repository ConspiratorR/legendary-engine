---
feature: phase14-engine-scene-residual
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 7df5723..HEAD
---

# Phase 14 — R2 engine-scene Transform 残余收敛

## Report

**What was built** — Continuation on `phase12-array-authority` (user: ignore git merge gates). Keyframe format stays in `engine_scene::keyframe`. `engine_core::animation_apply` now **re-exports** `AnimationClip` / keyframe types so gameplay can `use engine_core::{AnimationClip, apply_clip_pose}` without a direct engine-scene dependency. New sample MonoBehaviour **`AnimationClipPlayer`** holds clip + time/speed/playing, advances in `Update`, and writes poses via `apply_clip_pose` (World storage authority). Registered with SceneData (`script_type` + serde props). Docs clarify package-kept boundary: engine-scene `Transform`/`SceneManager` remain package-internal + editor legacy `scene_bridge`; collect stays Proxy-first. migration-guide Phase 14 R2 table + roadmap P2.5 updated.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-core --lib` (default ON) | PASS **260** |
| `cargo test -p engine-core --lib --no-default-features --features audio` | PASS **239** |
| `cargo test -p engine-scene --lib` | PASS 137 |
| `cargo test -p engine-editor --test editor_tests` | PASS 60 |
| `cargo test -p engine-render --lib collect` | PASS (GPU ignored PRE-EXISTING) |
| `cargo build -p engine-core --examples` | PASS |
| `cargo fmt --check` (core/scene/editor/render) | PASS |

**Journey log** —
1. S4 left “runtime wiring later”; R2 closes that with `AnimationClipPlayer`, not a new pose path.
2. Re-exports are the R2 boundary for “gameplay need not depend on engine-scene”; do not move keyframe math into engine-core.
3. Package deletion remains out of scope — docs + legacy bridge only.
4. Collect path unchanged (Proxy → GlobalTransform).
5. git merge/push intentionally not handled (“你不用管git”).

## [S1] Problem

After S4, clip apply existed but gameplay still needed engine-scene for types and had no Update-loop player; R2 residual meaning “engine-scene Transform still referenced” needed a clear non-deletion contract.

## [S2] Design

As implemented: re-export clip types; `AnimationClipPlayer` → `apply_clip_pose` → World; engine-scene package kept; editor scene_bridge legacy; collect Proxy-first.

## [S3] Out of Scope

- Delete engine-scene / transform / keyframe
- Mecanim rewrite
- Rewriting engine-scene SceneManager storage to Unity World
- VR/AR / Android / WASM SceneRuntime full
- git merge/push

## Tasks

- [x] T1: engine-core 再导出 keyframe 类型 + docs (covers: S2)
- [x] T2: `AnimationClipPlayer` + 注册 (covers: S2; depends: T1)
- [x] T3: 播放器/apply 测试双模态 (covers: S2; depends: T2)
- [x] T4: 文档 R2 边界 (engine-scene / migration-guide / roadmap) (covers: S2)
- [x] T5: 门禁 + spec finalize (covers: S2; depends: T2,T3,T4)
