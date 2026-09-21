---
feature: phase39-web-demo-smoke-ui
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 1edb57c..HEAD
---

# Phase 39 — web-demo smoke 结果可视化

## Report

**What was built** — `examples/web-demo/index.html` adds a persistent `#scene-smoke` overlay (success green / failure `.err` red). `lib.rs` writes `scene_runtime_json_smoke` result into that element; the centered `#status` loader still hides after init. `wasm-pack` rebuilt `pkg/`. WASM_STATUS documents how to open the page and what to expect on the overlay.

**Verification**:

| Command | Result |
|---------|--------|
| `wasm-pack build` (web-demo release) | PASS (Done) |
| `cargo test -p engine-core --lib` | PASS 262 |
| index.html `#scene-smoke` present | PASS (source) |

**Journey log** —
1. Phase36 wrote smoke into `#status` then hid it — invisible in browser.
2. Overlay `#scene-smoke` stays visible for landing evidence.
3. git merge/push not handled per user preference.

## [S1] Problem
Smoke result not visible after status hide.

## [S2] Design
Persistent overlay + wasm-pack + docs.

## [S3] Out of Scope
Full SceneRuntime editor UI, Android, git merge/push

## Tasks

- [x] T1: index.html + lib.rs smoke 面板 (covers: S2)
- [x] T2: 文档 + finalize (covers: S2; depends: T1)
