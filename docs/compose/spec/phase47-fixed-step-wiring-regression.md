---
feature: phase47-fixed-step-wiring-regression
status: delivered
updated: 2026-07-13
branch: main
commits: eb27dadf0a9b9310ebedd86583eed40d9aa105e8..impl
---

# Phase 47 — unity_physics_fixed_step 全路径回归

## Report

**What was built** — Two native regressions lock the production physics entry path that phase 46’s manual test bypassed. `test_fixed_step_secondary_support_wiring` runs only `unity_physics_fixed_step` (from_unity → step → compound-lite support → to_unity) and asserts the multi-collider Walker stays at feet-contact height. `test_fixed_step_secondary_trigger_callback_wiring` runs fixed_step then `dispatch_unity_collision_enters_for_test` and asserts the parent GameObject receives secondary-box `OnTriggerEnter`. Manual phase-46 path test remains as a paired unit-level check.

**Verification** —
- `cargo test -p engine-physics --lib test_fixed_step` → PASS (2 tests)
- `cargo test -p engine-physics --lib` → PASS 90
- `pwsh scripts/run-compose-gates.ps1` → All gates passed
- Review: no criticals; noted that fixed_step does **not** itself dispatch MB events

**Journey log** —
1. `unity_physics_fixed_step` wires support but **not** gameplay dispatch — callback tests must call dispatch after fixed_step (plugin FixedUpdate path does both).
2. Production order: sync_from → PhysicsWorld::step → apply_secondary_contact_support → sync_to.
3. Duplicated Walker scene vs phase 46 is intentional paired regression (wiring vs unit).

## [S1] Problem

Phase 46 测试手动拼 step/support，未锁 `unity_physics_fixed_step` 接线；secondary 回调也缺少经 fixed_step 的证据。

## [S2] Design

| 项 | 契约 |
|----|------|
| 路径 | 测试只调用 `unity_physics_fixed_step`（+ 回调场景再 dispatch） |
| 支撑 | Walker Y ∈ (0.35, 0.75) |
| 回调 | 父 MB OnTriggerEnter hits ≥ 1 |
| 说明 | fixed_step **不**派发 MB 事件；与 `UnityPhysicsPlugin` 的 dispatch 分离 |

## [S3] Out of Scope

- 完整 compound COM
- Android / VR·AR / git push（用户自理）

## Tasks

- [x] T1: fixed_step 支撑/回调回归测试 — acceptance: 两场景均经 fixed_step 通过（covers: S2）
- [x] T2: 文档 + 门禁 + finalize — acceptance: gates 不回归；Report（covers: S2; depends: T1）
