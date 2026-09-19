---
feature: phase11-r1-editor-acceptance
status: in-progress
updated: 2026-07-13
branch: phase11-r1-read
commits: 
---

# Phase 11 S5 — Editor Play / Scene Roundtrip / Mouse DPI

## Report

## [S1] Problem

Phase 11 needs editor-side acceptance: dual-mode Play, scene bundle roundtrip, and root-cause work on the long-standing mouse click offset. Investigation found two concrete bugs:

1. **Pick/gizmo space ≠ 3D image space.** The 3D texture is painted into `img_rect` = canvas minus a `32 * h_scale` top strip, but selection projects NDC using the **full** `canvas_rect` — systematic vertical offset (~32 logical points; worse feel on high-DPI).
2. **Play stop restores only `node_transforms`.** `tick_unity_play_host` mirrors runtime poses into the editor **World** every frame; `stop()` never writes World back from `editor_transform_snapshot`, so after Stop the World stays at play-time poses (scene dirty).

Secondary: viewport render targets used logical point counts, so at DPI≠1 the 3D view is under-sampled (blur) which amplifies “offset” perception.

## [S2] Design

### Coordinate contract

| Space | Units | Used by |
|-------|-------|---------|
| winit CursorMoved | physical px | `EguiState.handle_mouse_move` |
| egui pointer / canvas_rect | logical points | UI, pick, gizmo |
| 3D image | `img_rect` inside canvas | texture paint + **all pick/gizmo projections** |

`EguiState::begin_frame` already divides physical mouse by `scale_factor`. Picks must use `img_rect`, not full canvas.

### Play / stop contract

- `play()`: snapshot `node_transforms` (existing) **and** treat that as pre-play World pose baseline.
- During play: Unity host may drive editor World for viewport preview.
- `stop()`: restore `node_transforms` from snapshot, then **`apply_node_transform_to_world` for each snapshot entry** + `world.sync_transforms()`, so World authority matches pre-play poses.

### Scene roundtrip

Existing `save_scene_bundle` / `open_scene_file` (runtime twin) stay; S5 locks them with a transform-preservation test after play-stop cycle + dual-mode editor tests.

### DPI / picking fix

1. Shared helper `scene_image_rect(canvas_rect, h_scale)`.
2. `project_world_to_image(world_pos, vp, img_rect)` for selection + gizmo center.
3. Gizmo center = projected selected object; fallback HUD corner when none.
4. Viewport render size = logical size × `pixels_per_point` for sharpness; paint still fills `img_rect` logical box.
5. Document findings in session `notes.md` (DPI investigation).

## [S3] Out of Scope

- Full interactive Play automation (GPU window) beyond process launch smoke + unit tests
- Rewriting entire gizmo to Unity Handles API
- Default-on `unity-world-primary`
- git push

## Tasks

- [ ] T1: Spec + DPI investigation notes in session notes.md (covers: S2)
- [ ] T2: Play stop restores editor World from snapshot + test (covers: S2)
- [ ] T3: Viewport pick/gizmo use `img_rect` + projected gizmo center + unit-testable projection helper (covers: S2)
- [ ] T4: Viewport render target × pixels_per_point; EguiState accessor (covers: S2)
- [ ] T5: Scene bundle + play/stop + dual-mode editor tests green (covers: S2)
- [ ] T6: Launch editor binary smoke (default features); note feature-on build (covers: S2)
- [ ] T7: clippy/fmt on touched crates; verify gate (covers: S2)
