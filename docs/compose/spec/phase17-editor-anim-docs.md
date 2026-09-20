---
feature: phase17-editor-anim-docs
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 7526eb0..HEAD
---

# Phase 17 — 编辑器动画恢复 + Unity 路径文档

## Report

**What was built** — Editor integration test `animation_clip_player_survives_editor_scene_data_roundtrip` (engine-editor): register sample scripts → attach `AnimationClipPlayer` on `AnimNode` → `EditorState::to_core_scene_data` (prepared) → JSON contains player + clip name → reload via `LoadSceneJson` → `CollectMonoBehaviours` restores `AnimationClipPlayer` props (`speed=1.5`, clip `editor_walk`). New `docs/unity-storage-animation.md` documents default storage authority, opt-out, prepared I/O, animation apply/player, and demo commands. PROJECT_SUMMARY Unity section + README doc index updated.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-editor --test editor_tests` | PASS **61** |
| `cargo test -p engine-core --lib` | PASS 261 (prior baseline; no core API change) |

**Journey log** —
1. Editor default scene has multiple roots — load must find handle by name `AnimNode`, not `handles[0]`.
2. `to_core_scene_data` already uses prepared cache refresh (phase 13).
3. Docs consolidate phases 12–16 contracts in one page for contributors.
4. git merge/push not handled per user preference.

## [S1] Problem

No editor-side SceneData test for AnimationClipPlayer; Unity authority docs were scattered.

## [S2] Design

Editor roundtrip test + `docs/unity-storage-animation.md` + PROJECT_SUMMARY/README sync.

## [S3] Out of Scope

- WASM/Android/VR, dyn MB, git merge/push

## Tasks

- [x] T1: editor_tests AnimationClipPlayer SceneData 往返 (covers: S2)
- [x] T2: docs/unity-storage-animation.md + PROJECT_SUMMARY + README (covers: S2)
- [x] T3: 门禁 + spec finalize (covers: S2; depends: T1,T2)
