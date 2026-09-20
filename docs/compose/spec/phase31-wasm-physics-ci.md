---
feature: phase31-wasm-physics-ci
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: b1220fd..HEAD
---

# Phase 31 — WASM physics/SceneRuntime 编译面

## Report

**What was built** — Probe: `engine-physics` and `engine-editor` lib build for `wasm32-unknown-unknown` on this machine. CI `wasm` job now builds **engine-core** (`--no-default-features --features unity-world-primary`) and **engine-physics** in addition to render/editor lib. WASM_STATUS lists engine-physics ✅ + phase 31 notes; BRANCH_INDEX/README row 31. SceneRuntime JSON path remains in engine-core (phase 22); browser full runtime still deferred.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo build -p engine-physics --target wasm32-unknown-unknown` | PASS |
| `cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib` | PASS |
| `cargo test -p engine-core --lib` | PASS 261 |

**Journey log** —
1. engine-physics wasm succeeds via engine-core `default-features=false, features=["audio"]`.
2. CI wasm coverage expanded so regressions fail before merge.
3. git merge/push not handled per user preference.

## [S1] Problem
WASM compile surface undocumented for physics; CI omitted core/physics.

## [S2] Design
Probe + CI steps + WASM_STATUS/BRANCH_INDEX.

## [S3] Out of Scope
Browser SceneRuntime full, Android NDK, git merge/push

## Tasks

- [x] T1: 探测 + CI wasm steps (covers: S2)
- [x] T2: WASM_STATUS + BRANCH_INDEX + finalize (covers: S2)
