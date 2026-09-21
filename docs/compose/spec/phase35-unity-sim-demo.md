---
feature: phase35-unity-sim-demo
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 1c2a9ae..HEAD
---

# Phase 35 — unity_sim_demo 综合示例

## Report

**What was built** — Headless `unity_sim_demo` wires AnimationClipPlayer + Rigidbody gravity + prepared SceneData + SceneRuntime reload + `UnityPhysicsPlugin`/`unity_physics_fixed_step`. Asserts Hero X advances (animation) and Ball Y drops (physics); JSON contains player + sphere collider. README / unity-storage-animation / BRANCH_INDEX document the command.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo run -p engine-core --example unity_sim_demo` | **PASS** — hero_x 0→4.8；ball_y 5→2.295；JSON asserts held |

**Journey log** —
1. `App::world_mut` is the ECS World; SceneRuntime is a **resource**, not `app.unity_world()`.
2. Combined demo is landing evidence for phases 12–25 without GPU.
3. git merge/push not handled per user preference.

## [S1] Problem
No single runnable demo tying animation + physics + SceneData.

## [S2] Design
unity_sim_demo as specified; asserts both axes of simulation.

## [S3] Out of Scope
GPU/window, WASM browser, git merge/push

## Tasks

- [x] T1: unity_sim_demo (covers: S2)
- [x] T2: README + 文档 + finalize (covers: S2; depends: T1)
