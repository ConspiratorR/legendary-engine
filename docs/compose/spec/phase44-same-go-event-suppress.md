---
feature: phase44-same-go-event-suppress
status: delivered
updated: 2026-07-13
branch: main
commits: 42784eb5ebb7db97a1d50f6cf8c44ef19c334ebf..impl
---

# Phase 44 — 同 GO 碰撞对事件抑制

## Report

**What was built** — After phase 43 mapped secondary collider events to parent GameObjects, overlapping colliders on the **same** GameObject could dispatch `OnCollision*` / `OnTrigger*` with `other == this`. Unity does not raise callbacks for collider pairs on one GameObject. Dispatch now **skips** any collision or sensor event whose resolved handles are equal (`a == b`), after `go_for_physics_id` and before invoke. Broadphase/narrowphase still detect the overlap; only gameplay callbacks are suppressed. Cross-GO secondary→parent delivery (phase 43) is unchanged.

**Verification** —
- `cargo test -p engine-physics --lib test_same_go_collider_pair_does_not_self_dispatch` → PASS (hits==0)
- `cargo test -p engine-physics --lib test_secondary_trigger_maps_to_parent_monobehaviour` → PASS
- `cargo test -p engine-physics --lib` → PASS 86
- `cargo test -p engine-physics --test physics_tests` → PASS 66
- `pwsh scripts/run-compose-gates.ps1` → All gates passed
- Review: no criticals; S2 dispatch-only contract confirmed

**Journey log** —
1. Suppression is **handle-resolution level** (after secondary→parent map), not physics-pair filtering.
2. Future compound work must not re-dispatch `a == b` pairs.
3. Sensor events ignore RigidBody type — kinematic same-GO overlap still produces sensor pairs; keep that when hardening tests.
4. Simulation still sees same-GO contacts; only MB callbacks are filtered.

## [S1] Problem

Phase 43 后，同 GO 上 primary 与 secondary 重叠会解析为同一 `GameObjectHandle`，产生 `other == this` 的自触发回调。Unity 对同 GameObject 碰撞体对不派发事件。

## [S2] Design

| 项 | 契约 |
|----|------|
| 抑制 | dispatch 解析后 `a == b` → **跳过** collision/sensor 事件 |
| 不变 | 跨 GO（含 secondary→其它父）与 phase 43 一致 |
| 作用点 | 仅派发层；broadphase/接触求解不变 |
| 回归 | phase43 secondary→父 测试仍通过 |

## [S3] Out of Scope

- compound 冲量
- 同 GO 对在物理层移除接触
- Android / VR·AR / git push（用户自理）

## Tasks

- [x] T1: dispatch 同 GO 跳过 — acceptance: a==b 不 invoke（covers: S2）
- [x] T2: native 回归 — acceptance: 同 GO 不自触发；跨 GO secondary 仍回调（covers: S2; depends: T1）
- [x] T3: 文档 + 门禁 + finalize — acceptance: gates 不回归；Report（covers: S2; depends: T2）
