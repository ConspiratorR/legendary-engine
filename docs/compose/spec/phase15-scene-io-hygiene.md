---
feature: phase15-scene-io-hygiene
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: f8dac1d..HEAD
---

# Phase 15 — 默认开启后的 Scene I/O 集成卫生

## Report

**What was built** — After phase 13 default-on, in-repo Scene JSON callers that still used unprepared free-function `SaveSceneJson(&World)` were migrated to **`SaveSceneJsonPrepared(&mut World)`** so feature-on builds refresh pose/hierarchy caches before serialize. Call sites: `sample_scripts` tests, `world.rs` R1 load test, `serialization` roundtrip test, `tests/editor_tests.rs`, `tests/unity_lifecycle_tests.rs`, `examples/runtime_scene_demo.rs`. Free-function `SaveSceneJson` is **kept** as low-level `&World` API with rustdoc pointing at prepared paths. migration-guide Still deferred updated: unprepared free function remains for immutable/prepared worlds only.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-core --lib` | PASS **261** |
| `cargo test -p engine-core --lib --no-default-features --features audio` | PASS **240** |
| `cargo test -p engine-core --test editor_tests` | PASS 24 (1 ignored) |
| `cargo test -p engine-core --test unity_lifecycle_tests` | PASS 23 |
| `cargo test -p engine-core --test unity_lifecycle_tests --no-default-features --features audio` | PASS 21 |
| `cargo build -p engine-core --examples` | PASS |
| `cargo fmt -p engine-core --check` | PASS |

**Journey log** —
1. Default-on makes bare `cargo test` = ECS authority; unprepared save can write stale array pose caches.
2. In-repo tests/examples now demonstrate the prepared contract; free function stays for `&World` cases.
3. PowerShell `Set-Content` without UTF-8 encoding corrupted `unity_lifecycle_tests.rs` — restored via git checkout and rewrote with UTF-8 no-BOM.
4. `SaveSceneJsonPrepared(&mut x)` requires `let mut x`; editor tests needed `mut world`.
5. git merge/push not handled per user preference.

## [S1] Problem

Unprepared `SaveSceneJson(&World)` in tests/examples under default-on `unity-world-primary` could serialize stale array pose caches.

## [S2] Design

Migrate in-repo callers to prepared free function; keep unprepared API documented as low-level; feature-off prepare is no-op.

## [S3] Out of Scope

- Delete `SaveSceneJson`
- WASM/Android/VR/AR
- git merge/push

## Tasks

- [x] T1: 迁移 tests/sample_scripts/examples → SaveSceneJsonPrepared (covers: S2)
- [x] T2: 自由函数文档 + migration-guide Still deferred (covers: S2)
- [x] T3: 门禁 + spec finalize (covers: S2; depends: T1,T2)
