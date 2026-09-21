---
feature: phase37-scenedata-smoke-regression
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: f9f8211..HEAD
---

# Phase 37 — SceneData smoke 回归 + web-demo 构建收尾

## Report

**What was built** — Native regression `test_scenedata_smoke_json_local_transform_keys` locks the exact web-demo SceneData JSON schema (`local_position`/`local_rotation`/`local_scale`) via SceneRuntime `LoadSceneJson` + pose assert (1,2,3). `wasm-pack build --target web --release` in `examples/web-demo` succeeded; `pkg/web_demo.d.ts` exports `scene_runtime_json_smoke`. BRANCH_INDEX 12–37 + README 阶段 37.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-core --lib` | PASS **262** |
| `wasm-pack build` (web-demo release) | PASS (Done in 2m 50s) |
| pkg export smoke | present |

**Journey log** —
1. Phase36 critical: wrong transform JSON keys — locked by native test in phase37.
2. web-demo outside workspace: wasm-pack uses its own lock/target.
3. git merge/push not handled per user preference.

## [S1] Problem
No native regression for web-demo SceneData schema.

## [S2] Design
Load same JSON in engine-core test; wasm-pack refresh pkg; docs.

## [S3] Out of Scope
Browser manual UI QA, Android, git merge/push

## Tasks

- [x] T1: native SceneData smoke 回归测试 (covers: S2)
- [x] T2: 构建收尾 + docs + finalize (covers: S2; depends: T1)
