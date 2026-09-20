---
feature: phase13-default-on-hardening
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: feffdb9..HEAD
---

# Phase 13 — 默认开 flag 验收 + 残余硬化

## Report

**What was built** — Continuation on `phase12-array-authority` after phase 12 R1-full. Parent walks (`hierarchy::get_ancestors/get_root/is_ancestor/get_depth`, `IsActiveInHierarchy`, `is_descendant_of`) are hop-capped so corrupt parent cycles terminate. Scene I/O prefers prepared saves: `SaveSceneJsonPrepared` / `SceneSerializer::SavePrepared`; editor `to_core_scene_data` / `export_core_scene_json` / `save_scene_bundle` refresh caches first; `SceneManager::SaveSceneJson` takes `&mut World` and uses prepared. `write_transform_to_ecs` root detection uses `hierarchy_authority_parent` when the feature is on.

`engine-core` default features are now `["audio", "unity-world-primary"]`. Opt-out: `default-features = false, features = ["audio"]`. In-repo feature-off test isolation: `engine-framework` / `engine-physics` / `engine-script` depend on engine-core with `default-features = false, features = ["audio"]`; `engine-editor` enables `unity-world-primary` explicitly. CI dual-mode job runs default (on) **and** `--no-default-features --features audio` (off). Docs (migration-guide, README 阶段 13, roadmap, architecture) record default-on + opt-out. Dyn MB holders remain array-backed.

**Verification** (after review critical fixes):

| Command | Result |
|---------|--------|
| `cargo test -p engine-core --lib` (default ON) | PASS **259** |
| `cargo test -p engine-core --lib --no-default-features --features audio` (OFF) | PASS **238** |
| `cargo test -p engine-core --test unity_lifecycle_tests` (ON) | PASS 23 |
| `cargo test -p engine-core --test unity_lifecycle_tests --no-default-features --features audio` | PASS 21 |
| `cargo test -p engine-core --test identity_bridge_tests` | PASS 10 |
| `cargo test -p engine-core --test editor_tests` | PASS 24 (1 ignored pre-existing) |
| `cargo test -p engine-editor --test editor_tests` | PASS 60 |
| `cargo fmt -p engine-core -p engine-editor -p engine-render -p engine-scene --check` | PASS |

**Journey log** —
1. Bare `cargo test -p engine-core --lib` after default flip is **feature ON**; feature-off CI must use `--no-default-features --features audio`.
2. Feature-off unit-test isolation requires workspace dependents that reverse-depend on engine-core to set `default-features = false` (dev-deps feature unification).
3. `SaveSceneJson` without prepare remains a footgun under default-on — prefer prepared APIs / SceneManager wrapper.
4. Historical phase11/12 specs still describe default-off as of those phases; current contract is migration-guide + this spec.
5. Branch still `phase12-array-authority` (user chose direct continuation); merge/push not done by agent.

## [S1] Problem

Phase 12 left residual hop-cap gaps, unprepared editor save, seed root using array parent, and `unity-world-primary` default **off** despite dual-mode readiness.

## [S2] Design

Hardening + default-on as specified: hop-capped parent walks; prepared scene I/O; hierarchy-authority seed roots; `engine-core` default includes `unity-world-primary`; opt-out + CI dual-mode with explicit feature-off commands; docs aligned.

## [S3] Out of Scope

- Delete `unity-world-primary` feature or array cache slots
- Dyn MB into ECS
- engine-scene deletion / VR/AR / Android NDK / WASM SceneRuntime full
- Push / merge to main / branch delete

## Tasks

- [x] T1: 父链 hop-cap — hierarchy + IsActiveInHierarchy；自环/互环单测 (covers: S2)
- [x] T2: Prepared scene save + 编辑器导出 (covers: S2; depends: T1)
- [x] T3: seed 根判定 hierarchy authority + 测试 (covers: S2)
- [x] T4: 默认 features 含 unity-world-primary + opt-out 文档 + CI 修正 (covers: S2; depends: T1,T2,T3)
- [x] T5: 门禁 + spec finalize (covers: S2; depends: T4)
