---
feature: phase11-r1-editor-acceptance
status: delivered
updated: 2026-07-13
branch: phase11-r1-read
commits: a8354f1cc86e95e085624302122b2b2be3695dc5..HEAD
---

# Phase 11 S5 — Editor Play / Scene Roundtrip / Mouse DPI

## Report

**What was built** — Phase 11 S5 on `phase11-r1-read`. Root-caused the long-standing editor mouse click offset: pick projected NDC with full `canvas_rect` while the 3D texture was painted into a smaller `img_rect` (HUD inset), and gizmo was pinned to a fixed HUD corner. Pick/gizmo/paint now share `scene_image_rect` + `project_world_to_image`; gizmo center follows the projected selected object (Game tab uses `game_camera` while playing). `play()` snapshots all World poses after sync; `stop()` restores World via `apply_node_transform_to_world` + `sync_transforms` so Play does not leave the scene dirty. Viewport RT is sized with `pixels_per_point` for DPI sharpness. Investigation notes live in session `notes.md`.

**Verification** —
- `cargo test -p engine-core --lib` — PASS (226)
- `cargo test -p engine-core --lib --features unity-world-primary` — PASS (246)
- `cargo test -p engine-editor --lib` — PASS (186)
- `cargo test -p engine-editor --lib pick_tests` — PASS (3)
- `cargo test -p engine-editor --test editor_tests` — PASS (60)
- `cargo run -p engine-editor` smoke 12s — PASS (process running, RTX 5080 / Vulkan)
- `cargo fmt` applied on touched crates; clippy: pre-existing Unity API naming only

**Journey log** —
1. DPI “offset” was primarily **geometry mismatch** (canvas vs image rect + HUD gizmo corner), not egui input scaling — `EguiState` already divides physical cursor by `scale_factor`.
2. `stop()` must restore **World**, not only `node_transforms`; `play()` must snapshot after `sync_node_transforms_from_world` so every mapped node has a baseline.
3. `scene_image_rect` is an in-canvas HUD inset after header exclusion — pick and paint must use the same helper (not double-count headers).
4. Game tab projected gizmo must use `game_camera` during play, matching the render camera switch.
5. Roundtrip test must call `play()` before `stop()` or restore is a no-op (`stop` early-outs in Editing).

## [S1] Problem

Phase 11 needed editor acceptance: dual-mode Play, scene bundle roundtrip, and root-cause on mouse click offset. Two concrete bugs: pick/gizmo space ≠ 3D image space; Play stop restored only `node_transforms` while World kept play poses.

## [S2] Design

### Coordinate contract

| Space | Units | Used by |
|-------|-------|---------|
| winit CursorMoved | physical px | `EguiState.handle_mouse_move` |
| egui pointer / canvas_rect | logical | UI + viewport canvas (header already excluded) |
| 3D image | `scene_image_rect(canvas_rect)` | paint + **pick + gizmo projection** |

### Play / stop contract

- `play()`: `sync_node_transforms_from_world` then snapshot `node_transforms`.
- `stop()`: restore snapshot → `apply_node_transform_to_world` per key → `world.sync_transforms` → `sync_node_transforms_from_world`.

### DPI / picking

Shared `scene_image_rect` + `project_world_to_image`; gizmo center = projected selection (Game camera when tab==1 and playing); RT × `pixels_per_point`.

## [S3] Out of Scope

- Full interactive Play automation beyond unit tests + process smoke
- Rewriting gizmo to Unity Handles API
- Default-on `unity-world-primary`
- git push

## Tasks

- [x] T1: Spec + DPI investigation notes (covers: S2)
- [x] T2: Play stop restores editor World + tests (covers: S2)
- [x] T3: Viewport pick/gizmo on `img_rect` + projected center + projection tests (covers: S2)
- [x] T4: Viewport RT × pixels_per_point; `EguiState::pixels_per_point` (covers: S2)
- [x] T5: Scene bundle + play/stop + editor tests green (covers: S2)
- [x] T6: Launch editor binary smoke (default features) (covers: S2)
- [x] T7: fmt + verify gate (covers: S2)
