---
feature: phase35-unity-sim-demo
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 1c2a9ae..0619319
---

# Phase 35 — unity_sim_demo 综合示例

## Report

**What was built** — Headless `unity_sim_demo` wires AnimationClipPlayer + Rigidbody gravity + prepared SceneData + SceneRuntime reload + `UnityPhysicsPlugin`/`unity_physics_fixed_step`. Asserts Hero X advances (animation) and Ball Y drops (physics); JSON contains player + sphere collider. README / unity-storage-animation / BRANCH_INDEX document the command.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo run -p engine-core --example unity_sim_demo` | **PASS** — hero_x ↑；ball_y ↓（plugin FixedUpdate only） |
| `cargo test -p engine-core --lib` | PASS **261** |

**Journey log** —
1. `App::world_mut` is ECS World; SceneRuntime is a **resource**.
2. Demo uses **plugin-only** FixedUpdate physics (no manual double-step) — aligns with `unity_physics_demo`.
3. Combined demo is landing evidence without GPU.
4. git merge/push not handled per user preference.

## [S1] Problem
No single runnable demo tying animation + physics + SceneData.

## [S2] Design
unity_sim_demo as specified; asserts both axes of simulation.

## [S3] Out of Scope
GPU/window, WASM browser, git merge/push

## Tasks

- [x] T1: unity_sim_demo (covers: S2)
- [x] T2: README + 文档 + finalize (covers: S2; depends: T1)
