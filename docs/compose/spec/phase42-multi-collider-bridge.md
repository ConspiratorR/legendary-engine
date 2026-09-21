---
feature: phase42-multi-collider-bridge
status: delivered
updated: 2026-07-13
branch: main
commits: 375c29c6eb9816ddd594e5deeb87dea8ea14dff1..impl
---

# Phase 42 — 多碰撞体进物理桥

## Report

**What was built** — `sync_physics_from_unity` no longer keeps only one collider per GameObject. Priority **Sphere → Box → Capsule** remains **primary** on the Rigidbody entity; every additional Unity collider becomes a **kinematic secondary** ECS entity tagged `SecondaryCollider { parent, slot }` with shape/`is_sensor`/`offset` mapped from Unity. Secondary `Transform` follows the parent’s world pose each sync. Slots are reused; entities whose `(parent, slot)` is no longer live are **despawned** (collider shrink / RB removed / GO destroyed). Empty collider lists drop the primary `Collider` component. Engine-ecs gains `World::entity_from_index` for generation-safe handles.

**Verification** —
- `cargo test -p engine-physics --lib test_multi_collider_bridge_primary_and_secondary` → PASS (priority primary, secondary Box+follow, slot reuse, shrink GC)
- `cargo test -p engine-physics --lib` → PASS 84
- `cargo test -p engine-physics --test physics_tests` → PASS 66
- `cargo test -p engine-ecs --lib` → PASS 37
- `cargo build -p engine-physics --target wasm32-unknown-unknown` → PASS
- `pwsh scripts/run-compose-gates.ps1` → **11/11 PASS**（critical 修复 + fmt 后复跑）
- Re-review: critical #2 GC **FIXED**; no new criticals; Report finalize was remaining gap (this commit)

**Journey log** —
1. ECS stores one `Collider` per entity — multi-collider = secondary entities, not multi-component.
2. Review critical: without GC, shrinking colliders left phantom kinematic geometry; fixed with `live_secondary` + despawn after each sync.
3. `gen` is reserved in Rust 2024 — use `generation` in `entity_from_index`.
4. Secondary events do not map to parent MonoBehaviours (identity bridge links primaries only) — intentional boundary.
5. Compound impulse for secondary solid contacts remains deferred.

## [S1] Problem

Unity 同一 GameObject 可挂多个碰撞体（Sphere + Box + Capsule）。`unity_bridge` 用 else-if 优先级只把一个形状写入 Rigidbody 实体；其余碰撞体不进模拟。

## [S2] Design

| 项 | 契约 |
|----|------|
| 优先级 | Sphere → Box → Capsule 为 **primary**，挂 Rigidbody 实体 |
| Secondary | 其余碰撞体 → kinematic ECS 子实体：`RigidBody::new_kinematic()` + `Collider`（`is_trigger`→`is_sensor`，`center`→`offset`）+ `CoreTransform`（父 GO 世界位姿） |
| 跟随 | 每次 `sync_physics_from_unity` 用父 world position/rotation 重写 secondary Transform（offset 不在 Transform 上叠加） |
| 注册 | `SecondaryCollider { parent, slot }`；按 parent+slot 复用 |
| GC | 本轮不在 `live_secondary` 的 secondary **despawn**；碰撞体列表为空时 primary 移除 `Collider` |
| 响应边界 | secondary kinematic：参与 broadphase/sensor；**不** compound 冲量到父 body |
| 回调 | secondary 碰撞/trigger **不**自动映射父 MB |
| 导出 | `SecondaryCollider` / `ColliderPriority` 经 `engine_physics` 公开 |
| 回归 | 单碰撞体 / capsule / trigger 测试不回归 |

## [S3] Out of Scope

- compound 冲量/质心合成
- secondary 事件派发到父 MonoBehaviour
- 编辑器 Inspector 多碰撞体可视化
- Android / VR·AR / git push（用户自理）

## Tasks

- [x] T1: secondary 桥实体 + 跟随同步 — acceptance: 双碰撞体 GO → ECS 两套 Collider；primary 仍优先级形状；secondary Transform 跟父（covers: S2）
- [x] T2: native 回归测试 — acceptance: physics lib 覆盖 multi-collider presence + slot reuse + shrink GC（covers: S2; depends: T1）
- [x] T3: 文档 + 门禁 + review finalize — acceptance: gates 不回归；Report（covers: S2; depends: T2）
