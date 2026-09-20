---
feature: phase24-play-callbacks-capsule-offset
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 8e94ae3..HEAD
---

# Phase 24 — Play 物理回调 + Capsule 偏移 + 序列化 is_sensor

## Report

**What was built** — Phase 23 residuals closed. Unity `CapsuleCollider.center` maps to physics `Collider.offset` (runtime + editor); `direction != 1` logs a warning (shape remains Y). Scene serializer writes `PhysicsDataSer.is_sensor` from actual collider `is_trigger`. Editor `tick_unity_play_host` moves `PhysicsWorld` onto ECS, steps, then calls public `engine_physics::dispatch_unity_collision_enters` so Play mode fires `OnCollision/Trigger Enter/Exit` on MonoBehaviours. Editor test asserts Play collision hits and capsule offset.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS 79 |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo test -p engine-editor --test editor_tests` | PASS **62** |

**Journey log** —
1. `PhysicsWorld` is not `Clone` — Play host moves resource onto ECS via `mem::replace` + restore.
2. Dispatch function must be public for editor reuse.
3. Capsule non-Y direction is warn-only until oriented capsule shape exists.
4. git merge/push not handled per user preference.

## [S1] Problem
Phase 23 residuals: Play callbacks, capsule center, serialized is_sensor.

## [S2] Design
Implemented as specified.

## [S3] Out of Scope
Oriented capsule shape, WASM browser SceneRuntime, git merge/push

## Tasks

- [x] T1: Capsule offset + direction warn (covers: S2)
- [x] T2: scene_serializer is_sensor (covers: S2)
- [x] T3: Editor Play 物理回调分发 (covers: S2)
- [x] T4: 测试 + docs + finalize (covers: S2)
