---
feature: phase23-collision-exit-capsule
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 26d67e3..HEAD
---

# Phase 23 — Exit 回调 + CapsuleCollider 桥

## Report

**What was built** — `World::invoke_collision_exit` / `invoke_trigger_exit` added. `UnityPhysicsPlugin` dispatches physics `is_enter=false` collision and sensor events to `OnCollisionExit` / `OnTriggerExit`. Unity `CapsuleCollider` maps to physics `Collider::capsule` with `is_trigger` → `is_sensor` in runtime bridge and editor Play host. Unit test covers capsule mapping + enter/exit invoke counts on both parties.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **79** |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS 61 |

**Journey log** —
1. Physics already emitted exit events; only Unity dispatch + World invoke were missing.
2. CapsuleCollider already had `is_trigger`; bridge simply never synced it.
3. Dispatch now walks **all** collision/sensor events (enter + exit), not enter-only.
4. git merge/push not handled per user preference.

## [S1] Problem
No Unity exit callbacks; CapsuleCollider not bridged.

## [S2] Design
Exit invokes + plugin exit dispatch + capsule sync as specified.

## [S3] Out of Scope
Full Collision payload, browser SceneRuntime, git merge/push

## Tasks

- [x] T1: World invoke_collision_exit / invoke_trigger_exit (covers: S2)
- [x] T2: 插件 exit 分发 (covers: S2; depends: T1)
- [x] T3: CapsuleCollider 桥映射 (covers: S2)
- [x] T4: 测试 + docs + finalize (covers: S2)
