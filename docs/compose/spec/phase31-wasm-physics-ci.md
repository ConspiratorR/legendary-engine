---
feature: phase31-wasm-physics-ci
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: b1220fd..HEAD
---

# Phase 31 — WASM physics/SceneRuntime 编译面

## Report

**What was built** — After review criticals: `engine-core` dep in engine-physics is `default-features = false` (**no audio/rodio**); `rayon` is a **non-wasm** dependency and `par_iter` paths (`collect_pairs_parallel`, `for_each_island`) fall back to sequential iter on wasm32. CI wasm job builds engine-core (`--no-default-features --features unity-world-primary`) + engine-physics + render + editor lib. WASM_STATUS phase 31 notes single-thread physics on wasm; BRANCH_INDEX title 12–31 + gate list includes wasm physics/editor steps.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo build -p engine-physics --target wasm32-unknown-unknown` | PASS |
| `cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib` | PASS |
| `cargo test -p engine-physics --lib` | PASS 83 |
| `cargo test -p engine-physics --test physics_tests` | PASS 66 |
| `cargo test -p engine-core --lib` | PASS 261 |

**Journey log** —
1. `features=["audio"]` on engine-core dep re-enabled rodio on wasm — drop audio for physics.
2. `rayon::par_iter` must not be used on wasm32 — cfg-gate + sequential fallback.
3. Compile surface ≠ runtime-ready browser physics (single-thread noted in WASM_STATUS).
4. git merge/push not handled per user preference.

## [S1] Problem
WASM physics compile surface / CI / audio+rayon deps undocumented or risky.

## [S2] Design
No-audio core dep; native-only rayon; CI wasm steps; docs.

## [S3] Out of Scope
Browser SceneRuntime full, Android NDK, git merge/push

## Tasks

- [x] T1: 探测 + CI wasm steps (covers: S2)
- [x] T2: WASM_STATUS + BRANCH_INDEX + criticals (covers: S2)
