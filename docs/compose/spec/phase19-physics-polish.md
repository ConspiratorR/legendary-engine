---
feature: phase19-physics-polish
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: be98bab..HEAD
---

# Phase 19 — 物理桥完善：旋转 / Sleep / 碰撞回调

## Report

**What was built** — Phase 18 residuals closed. `PhysicsWorld::integrate_bodies` integrates **angular velocity** into `Transform` rotation (semi-implicit quaternion). Bridge `to_unity` writes **world + local** position/rotation so dual-read `GetTransform` stays coherent. Unity `Rigidbody.use_gravity=false` remains **Dynamic** (`gravity_scale=0`); `is_sleeping`/`Sleep()` forces sim sleep and zeros velocities. `UnityPhysicsPlugin` dispatches `is_enter` collision events to `MonoBehaviour::OnCollisionEnter` via `World::invoke_collision_enter` (entity index → GameObject via identity bridge).

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **75** (rotation+sleep, collision invoke count, free-fall, parented local) |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 61 |

**Journey log** —
1. `use_gravity=false` must not create Static bodies — Unity still simulates Dynamic.
2. `Transform::Rotation()` is **world** field; `SetLocalRotation` alone leaves it stale for dual-read — write both.
3. Physics rotation test failed until body_type was Dynamic (Static skipped integrate).
4. Collision events use `u32` entity indices — map via `entity.index()` on bridge links.
5. git merge/push not handled per user preference.

## [S1] Problem
Phase 18 residuals: no rotation writeback, no Sleep gate, no collision → MB dispatch.

## [S2] Design
As implemented above.

## [S3] Out of Scope
- Full trigger/joint callbacks, WASM/Android, git merge/push

## Tasks

- [x] T1: 物理旋转积分 + to_unity 旋转写回 (covers: S2)
- [x] T2: Rigidbody.is_sleeping / Sleep + bridge (covers: S2)
- [x] T3: OnCollisionEnter 分发 (covers: S2)
- [x] T4: 门禁 + docs + finalize (covers: S2; depends: T1,T2,T3)
