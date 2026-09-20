---
feature: phase20-collision-completeness
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 8c33b95..HEAD
---

# Phase 20 — 碰撞完善：相对速度 / e2e / 触发器

## Report

**What was built** — Collision dispatch now fills `Collision.relative_velocity` from physics body linear-velocity difference (`va − vb`). `World::invoke_trigger_enter` added; `UnityPhysicsPlugin` dispatches `SensorEvent.is_enter` to `MonoBehaviour::OnTriggerEnter`. Bridge e2e test drops a dynamic sphere onto a kinematic floor through `sync_*` + `dispatch_unity_collision_enters_for_test` and asserts HitCounter increments. Docs updated.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **76** (incl. e2e collision dispatch) |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 61 |

**Journey log** —
1. Sensor events share the same plugin dispatch path as solid collisions.
2. e2e uses kinematic floor + dynamic ball; hit count on either GO is valid.
3. `dispatch_unity_collision_enters_for_test` is a thin public alias for unit tests.
4. git merge/push not handled per user preference.

## [S1] Problem
Phase 19 residuals: zero relative_velocity, no plugin e2e, no trigger dispatch.

## [S2] Design
Implemented as specified.

## [S3] Out of Scope
Full Collision payload, WASM/Android, git merge/push

## Tasks

- [x] T1: relative_velocity 填充 (covers: S2)
- [x] T2: e2e 碰撞测试 (covers: S2)
- [x] T3: OnTriggerEnter (covers: S2)
- [x] T4: 门禁 + docs + finalize (covers: S2)
