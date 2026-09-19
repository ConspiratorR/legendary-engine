---
feature: phase11-r1-anim-pose-cut
status: delivered
updated: 2026-07-13
branch: phase11-r1-read
commits: 0bbd92b742eb36bcc23332a69eb4945f90210b2..HEAD
---

# Phase 11 S4 — Animation Keyframe Pose Authority

## Report

**What was built** — Phase 11 S4 on `phase11-r1-read`. Keyframe **format** stays in `engine_scene::keyframe` (pure math + serde). New `engine_core::animation_apply::apply_clip_pose` samples clips and writes Unity World via `with_ecs_transform_mut` (ECS-primary under `unity-world-primary`). Editor animation preview now (1) seeds `node_transforms` from `animation_pose_from_world` (World dual-read first), (2) applies only non-empty tracks so partial clips do not zero axes, (3) writes through `apply_node_transform_to_world`. `engine_scene::transform::Transform` is documented as scene-graph **fallback**, not pose authority. `collect_system` remains Proxy-first with GlobalTransform fallback.

**Verification** —
- `cargo test -p engine-core --lib` — PASS (225 before polish; animation_apply 4 after empty-track fix)
- `cargo test -p engine-core --lib --features unity-world-primary` — PASS (245)
- `cargo test -p engine-core --lib animation_apply` — PASS (4)
- `cargo test -p engine-scene --lib` — PASS (137)
- `cargo test -p engine-render --lib collect` — PASS (9; 2 GPU ignored PRE-EXISTING)
- `cargo test -p engine-editor --lib` — PASS (183)
- `cargo test -p engine-editor --lib animation_editor` — PASS (31 after polish)
- `cargo test -p engine-editor --test editor_tests` — PASS (58)
- `cargo build -p engine-core --examples` — PASS

**Journey log** —
1. Inventory: editor preview wrote only `node_transforms`; runtime clip apply did not exist; collect already Proxy-first.
2. Reviewer residual: `animation_pose_from_world` was dead code — wired as preview seed so World is dual-read authority.
3. Reviewer residual: empty-but-`Some(vec![])` tracks sampled as zeros — `apply_clip_pose` now treats empty tracks as absent.
4. Clip format intentionally stays on `engine_scene::keyframe`; do not re-point storage at engine-scene Transform.
5. `apply_clip_pose` is the documented gameplay apply path; editor still uses ComponentCurve → snapshot → World (unified later in runtime animation wiring).

## [S1] Problem

Animation keyframe **format** lives in `engine_scene::keyframe`, while gameplay pose authority is `engine_core::World`. Editor preview wrote only the `node_transforms` snapshot and never applied to World. Empty clip tracks zeroed transform axes. Docs still described engine-scene Transform as something keyframes "sample" as authority.

## [S2] Design

### Inventory (S4.1)

| Location | Reads/Writes | After S4 |
|----------|--------------|----------|
| `engine-render/src/collect_system.rs` | Proxy then GlobalTransform | unchanged (C1); docs updated |
| `engine-scene/src/keyframe.rs` | Clip sample math only | docs: format not authority |
| `engine-scene/src/transform.rs` | Legacy scene-graph pose | docs: fallback only |
| `engine-core/src/animation_apply.rs` | Apply clip → World | **authority apply path** |
| `engine-editor` animation preview | World seed → tracks → World write | dual-read + write-through |
| `EditorState::apply_node_transform_to_world` | World write | `with_ecs_transform_mut` |
| `EditorState::animation_pose_from_world` | World → snapshot | **used as preview seed** |

### Contracts

1. Clip format stays in `engine_scene::keyframe`. Do not move keyframe storage onto engine-scene Transform.
2. `apply_clip_pose` uses `with_ecs_transform_mut`; empty/absent tracks are not written.
3. Editor preview: seed from World (`animation_pose_from_world`); sample only non-empty tracks; write World.
4. `collect_system` stays Proxy-first.
5. No default feature flip; no render Pass changes; no package deletion.

## [S3] Out of Scope

- Full Mecanim/state-machine rewrite
- Deleting `engine_scene::transform` or keyframe module
- Default-on `unity-world-primary`
- VR/AR / Android / WASM SceneRuntime
- Production runtime MonoBehaviour animation clip auto-player (API ready; wiring later)
- git push / branch-ref deletion

## Tasks

- [x] T1: Inventory recorded in this doc + phase plan appendix (covers: S2)
- [x] T2: `engine_core::animation_apply::apply_clip_pose` + unit tests (covers: S2)
- [x] T3: Editor preview empty-track preservation + World write-through after preview (covers: S2)
- [x] T4: `apply_node_transform_to_world` → `with_ecs_transform_mut`; `animation_pose_from_world` wired as preview seed (covers: S2)
- [x] T5: Docs: keyframe/transform module notes + migration-guide S4 table (covers: S2)
- [x] T6: Verification gate PASS (covers: S2)
