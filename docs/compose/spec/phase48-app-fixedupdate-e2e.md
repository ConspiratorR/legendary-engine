---
feature: phase48-app-fixedupdate-e2e
status: delivered
updated: 2026-07-13
branch: main
commits: b772cf2379dfd15480229639a31737fd5f4ca1d1..eca09b2f6ef75bed5b4965a2bca84240b89453b3
---

# Phase 48 — App FixedUpdate e2e（UnityPhysicsPlugin）

## Report

**What was built** — Native e2e proves multi-collider compound-lite support through the **App lifecycle** path: `AppBuilder` holds a populated `SceneRuntime` resource; `UnityPhysicsPlugin` registers on `fixed_schedule`; the test drives **only** `app.run_with_lifecycle(0.02)` (no manual physics step). After 80 frames the Walker (primary sphere + feet Box secondary on a static floor) stays at feet-contact height. Production chain: `run_with_lifecycle` → FixedUpdate accumulator → `unity_physics_step_system` → `unity_physics_fixed_step` (sync → step → support → writeback) → dispatch.

**Verification** —
- `cargo test -p engine-physics --lib test_app_fixedupdate_unity_physics_plugin_support` → PASS
- `cargo test -p engine-physics --lib` → PASS 91
- `pwsh scripts/run-compose-gates.ps1` → All gates passed
- Review: no criticals

**Journey log** —
1. `UnityPhysicsPlugin::build` auto-inserts `PhysicsWorld` if missing — tests need not.
2. App lifecycle overwrites ECS `Time` from `App.time` each frame; fixed steps use `fixed_delta_time`.
3. Scene fixture intentionally mirrors phase46/47 for comparable support assertions.
4. `unity_physics_step_system` also dispatches MB collision/trigger after fixed_step.

## [S1] Problem

Phase 47 锁了 `unity_physics_fixed_step` 函数路径，未覆盖 App 帧循环 + `UnityPhysicsPlugin` 的 FixedUpdate 装配。

## [S2] Design

| 项 | 契约 |
|----|------|
| 装配 | `AppBuilder` + `SceneRuntime` resource + `UnityPhysicsPlugin`（PhysicsWorld 可由插件自动插入） |
| 驱动 | 仅 `app.run_with_lifecycle(0.02)` |
| 场景 | Floor + Walker（primary 球 + feet Box secondary） |
| 断言 | Walker Y ∈ (0.35, 0.75) |

## [S3] Out of Scope

- 完整 compound COM
- 编辑器 Play UI
- Android / VR·AR / git push（用户自理）

## Tasks

- [x] T1: App/plugin e2e 测试 — acceptance: 仅经 App FixedUpdate 驱动时 Walker Y 支撑成立（covers: S2）
- [x] T2: 文档 + 门禁 + finalize — acceptance: gates 不回归；Report（covers: S2; depends: T1）
