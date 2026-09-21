---
feature: phase36-wasm-scene-runtime-demo
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 72ac0d4..HEAD
---

# Phase 36 — WASM SceneRuntime web-demo 切片

## Report

**What was built** — After review criticals: SceneData JSON uses **`local_position` / `local_rotation` / `local_scale`** (schema-correct); `scene_runtime_json_smoke` result surfaces on the web `#status` DOM; unused `LoadSceneJson` import removed. web-demo depends on engine-core (`default-features=false`, `unity-world-primary`). Compose gates + BRANCH_INDEX include `cargo build --manifest-path examples/web-demo/Cargo.toml --target wasm32-unknown-unknown --lib`.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo build --manifest-path examples/web-demo/Cargo.toml --target wasm32-unknown-unknown --lib` | **PASS** (Finished) |
| `cargo test -p engine-core --lib` | PASS 261 |

**Journey log** —
1. Review C1: SceneData transform keys are **local_***, not translation/rotation/scale.
2. Review C2: rebuild wasm32 after fix; web-demo is **outside** root workspace (own lock/target).
3. Review M2: surface smoke on `#status` (no wasm logger in demo).
4. Review M3: gates + BRANCH_INDEX include web-demo wasm32 build.
5. git merge/push not handled per user preference.

## [S1] Problem
web-demo SceneRuntime smoke JSON + gates gaps.

## [S2] Design
Correct JSON keys + DOM status + wasm32 gate + docs.

## [S3] Out of Scope
Full browser SceneRuntime UI, Android, git merge/push

## Tasks

- [x] T1: web-demo SceneRuntime smoke + 编译 (covers: S2)
- [x] T2: 文档 + 门禁 + finalize (covers: S2; depends: T1)
