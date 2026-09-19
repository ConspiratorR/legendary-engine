---
feature: phase11-r1-anim-pose-cut
status: designed
updated: 2026-07-13
branch: phase11-r1-read
commits: 
---

# Phase 11 S4 — Animation Keyframe Pose Authority

## Report

## [S1] Problem

Animation keyframe **format** lives in `engine_scene::keyframe`, while gameplay pose authority is `engine_core::World`. Editor preview wrote only the `node_transforms` snapshot and never applied to World, so animated objects would not drive viewport/gameplay poses. Empty clip tracks zeroed transform axes (drift). Docs still described engine-scene Transform as something keyframes "sample" as authority.

## [S2] Design

### Inventory (S4.1)

| Location | Reads/Writes | After S4 |
|----------|--------------|----------|
| `engine-render/src/collect_system.rs` | Proxy then GlobalTransform | unchanged (C1); docs updated |
| `engine-scene/src/keyframe.rs` | Clip sample math only | docs: format not authority |
| `engine-scene/src/transform.rs` | Legacy scene-graph pose | docs: fallback only |
| `engine-core/src/animation_apply.rs` (new) | Apply clip → World | **new authority apply path** |
| `engine-editor` animation preview | node_transforms | write-through World after preview |
| `EditorState::apply_node_transform_to_world` | World write | `with_ecs_transform_mut` |
| `EditorState::animation_pose_from_world` (new) | World → snapshot | dual-read preferred |

### Contracts

1. Clip format stays in `engine_scene::keyframe` (serde + interpolation). Do not move keyframe storage onto engine-scene Transform.
2. `apply_clip_pose` uses `with_ecs_transform_mut` — feature on = ECS authority + array cache; feature off = array authority.
3. Editor preview: sample tracks **with keyframes only**; untracked axes keep current pose; then write World.
4. `collect_system` stays Proxy-first.
5. No default feature flip; no render Pass changes; no package deletion.

## [S3] Out of Scope

- Full Mecanim/state-machine rewrite
- Deleting `engine_scene::transform` or keyframe module
- Default-on `unity-world-primary`
- VR/AR / Android / WASM SceneRuntime
- git push / branch-ref deletion

## Tasks

- [ ] T1: Inventory recorded in this doc + phase plan appendix (covers: S2)
- [ ] T2: `engine_core::animation_apply::apply_clip_pose` + unit tests (covers: S2)
- [ ] T3: Editor preview empty-track preservation + World write-through after preview (covers: S2)
- [ ] T4: `apply_node_transform_to_world` → `with_ecs_transform_mut`; `animation_pose_from_world` helper (covers: S2)
- [ ] T5: Docs: keyframe/transform module notes + migration-guide S4 table (covers: S2)
- [ ] T6: Dual-mode/related verification gate PASS (covers: S2)
