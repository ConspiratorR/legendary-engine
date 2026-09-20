---
feature: phase20-collision-completeness
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 8c33b95..HEAD
---

# Phase 20 — 碰撞完善：相对速度 / e2e / 触发器

## Report

**What was built** — After review criticals: Unity `is_trigger` maps to physics `Collider.is_sensor` (runtime + editor bridges); `PhysicsWorld::step` clears/samples gameplay collision/sensor events **once per frame** with sub-step enter dedupe (`frame_entered_*`) so enter events survive multi-substep; dispatch fills `relative_velocity = va − vb` from ECS bodies; `World::invoke_trigger_enter` + sensor dispatch to `OnTriggerEnter`; e2e tests for solid collision (hit count) and trigger volume (`is_sensor` assert + OnTrigger hits); unit test asserts relative_velocity ±3 on X for known body velocities.

**Verification** (after critical fixes):

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **78** |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 61 |
| `cargo fmt -p engine-physics -p engine-core -p engine-editor --check` | PASS |

**Journey log** —
1. Sub-steps used to `clear()` sensor/collision enter events; only the last substep remained with `is_enter=false` — frame-level event lifetime required.
2. Unity trigger volumes never reached physics until `is_trigger` → `is_sensor` was copied in both bridges.
3. e2e solid-collision relative_velocity is timing-sensitive; unit test with injected event is the reliable oracle.
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
