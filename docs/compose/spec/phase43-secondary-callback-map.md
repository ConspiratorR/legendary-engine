---
feature: phase43-secondary-callback-map
status: delivered
updated: 2026-07-13
branch: main
commits: a05787155a5be5f02249a63e80f0a2cea3665451..impl
---

# Phase 43 — secondary 碰撞事件映射父 MB

## Report

**What was built** — Physics dispatch now resolves secondary collider events to the parent GameObject. `go_for_physics_id(runtime, ecs, id)` tries the identity bridge first (primary entities), then `SecondaryCollider.parent`. Collision and sensor enter/exit/trigger callbacks fire on the parent GO’s MonoBehaviours; `Collision.other` / `TriggerData.other` are the **resolved** GameObject handles; `relative_velocity` still comes from each side’s primary `RigidBody`. Missing/despawned secondary tags skip the event (same as before for unmapped ids). Docs (README 阶段 43, BRANCH_INDEX, unity-storage-animation) updated.

**Verification** —
- `cargo test -p engine-physics --lib test_secondary_trigger_maps_to_parent_monobehaviour` → PASS
- `cargo test -p engine-physics --lib` → PASS 85
- `cargo test -p engine-physics --test physics_tests` → PASS 66
- `pwsh scripts/run-compose-gates.ps1` → **All gates passed** (11/11)
- Review: Spec/Correctness/Consistency met; **no criticals**; Report finalize was remaining bookkeeping

**Journey log** —
1. Phase 42 shipped secondaries without parent callbacks; phase 43 closes that dispatch gap only (not compound impulse).
2. Events carry raw ECS indices — Unity GO mapping is bridge-first then `SecondaryCollider.parent`.
3. Test geometry: probe at x=0.8 overlaps secondary box (half=1) but not primary sphere (r=0.2) so parent hits come from secondary.
4. Non-blocking watch: same-GO primary↔secondary pairs may now surface as parent callbacks; Unity typically suppresses same-GameObject pairs — not covered in S3.
5. Compound impulse / secondary-owned MB entities remain deferred.

## [S1] Problem

Phase 42 secondary kinematic entities participate in physics but `go_for_physics_id` only used the identity bridge, so secondary collision/trigger events never reached parent GameObject MonoBehaviours.

## [S2] Design

| 项 | 契约 |
|----|------|
| 解析 | `go_for_physics_id(runtime, ecs, id)`：先 identity bridge；再 `SecondaryCollider` → `tag.parent` |
| 事件语义 | secondary enter/exit/trigger 投递父 GO；`other` 为解析后的对方 GO |
| relative_velocity | 双方解析后 GO 的 primary `RigidBody` 速度 |
| 双向 | 父 GO 与对方各收一次（与 primary 路径一致） |
| 退化 | 无 tag / 已 despawn → 跳过 |
| 回归 | primary-only dispatch 不回归 |

## [S3] Out of Scope

- compound 冲量 / secondary 独立 MB 实体
- 同 GO primary↔secondary 对内事件抑制（Unity 常见行为；本阶段未做）
- 编辑器可视化
- Android / VR·AR / git push（用户自理）

## Tasks

- [x] T1: dispatch 解析 secondary→parent — acceptance: `go_for_physics_id` 识别 SecondaryCollider（covers: S2）
- [x] T2: native 回归 — acceptance: secondary trigger 触发父 MB 回调（covers: S2; depends: T1）
- [x] T3: 文档 + 门禁 + review finalize — acceptance: gates 不回归；Report（covers: S2; depends: T2）
