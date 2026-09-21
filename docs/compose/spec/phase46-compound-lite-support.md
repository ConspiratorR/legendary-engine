---
feature: phase46-compound-lite-support
status: delivered
updated: 2026-07-13
branch: main
commits: 8cab67116d26f22486952a7281b389ab358ebd3a..impl
---

# Phase 46 — compound-lite：secondary 支撑冲量

## Report

**What was built** — Multi-collider secondary contacts now support the parent Dynamic body. `SecondaryCollider` stores `parent_entity` (primary Rigidbody ECS entity). After `PhysicsWorld::step`, `apply_secondary_contact_support` looks at collision pairs where one side is a **non-sensor secondary**, the other is non-sensor and **not the same GameObject**, and the parent body is Dynamic: it cancels the parent’s velocity component into the contact along the **other→secondary** normal, and lifts the parent ECS position by contact `depth` (clamped ≤0.08) when that velocity was into the contact. Hooked from `unity_physics_fixed_step`. Full compound COM/impulse remains out of scope.

**Verification** —
- `cargo test -p engine-physics --lib test_secondary_contact_supports_parent_dynamic` → PASS
- `cargo test -p engine-physics --lib` → PASS 88
- `cargo test -p engine-physics --test physics_tests` → PASS 66
- `pwsh scripts/run-compose-gates.ps1` → All gates passed
- Review: no criticals; T2 test calls step+support manually (wiring noted untested by that test)

**Journey log** —
1. Kinematic secondary alone cannot hold a Dynamic parent — gravity integrates inside `step` before post-hooks; velocity cancel alone still sinks ~g·dt² per frame.
2. Normals are A→B (`check_box_box`); flip when secondary is `idx_a`.
3. Same-GO primary↔secondary pairs must be skipped for support (not Unity-like contacts).
4. Positional lift (depth-clamped) is required for resting compound-lite; not a full solver impulse.
5. Multi-contact lifts can stack; secondary Transform refreshes on next `from_unity`.

## [S1] Problem

Phase 42 secondary 为 kinematic，父 Dynamic 感受不到脚部/secondary 与地面的支撑，会持续下落。

## [S2] Design

| 项 | 契约 |
|----|------|
| 标记 | `SecondaryCollider.parent_entity` = 父 primary Rigidbody 实体 |
| 后处理 | `apply_secondary_contact_support` 于 `unity_physics_fixed_step` 中 `step` 之后 |
| 规则 | 非 sensor secondary × 非 sensor other × 非同 GO × 父 Dynamic：`vn < 0` 时清零沿 other→sec 法线的速度分量，并沿该法线抬升 `depth.clamp(0,0.08)` |
| 跳过 | 同 GO、sensor、父非 Dynamic |
| 不做 | 完整 compound COM/惯性、旋转支撑 |

## [S3] Out of Scope

- 完整 compound 刚体（COM/惯性张量）
- secondary 接触对父的摩擦/旋转
- Android / VR·AR / git push（用户自理）

## Tasks

- [x] T1: parent_entity + support 后处理 — acceptance: API 与 step 接线（covers: S2）
- [x] T2: native 回归 — acceptance: 脚盒着地时父 GO 保持在接触高度区间（covers: S2; depends: T1）
- [x] T3: 文档 + 门禁 + finalize — acceptance: gates 不回归；Report（covers: S2; depends: T2）
