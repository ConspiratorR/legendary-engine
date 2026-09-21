---
feature: phase45-scenedata-multicollider-roundtrip
status: delivered
updated: 2026-07-13
branch: main
commits: 3db5e88d48d29344e89eb49e8b94763bd18b99ee..5e00562934cda209210ad92ae2bd6eb1c30de5cf
---

# Phase 45 — SceneData 多碰撞体往返 → 物理桥

## Report

**What was built** — End-to-end evidence that multi-collider GameObjects survive the SceneData path into the physics bridge. A GO authored with Box + Sphere + Capsule + Rigidbody is saved via `SaveSceneJsonPrepared`, loaded with `SceneRuntime::load_scene_json`, then `sync_physics_from_unity` rebuilds **primary Sphere** (priority) plus **secondary Box (slot 0) and Capsule (slot 1)** with centers and `is_trigger`→`is_sensor` intact. Bare `LoadSceneJson` also restores all three collider components. No production bridge logic change — this phase locks phase 42–44 behavior across serialization.

**Verification** —
- `cargo test -p engine-physics --lib test_scenedata_multicollider_roundtrip_rebuilds_bridge` → PASS
- `cargo test -p engine-physics --lib` → PASS 87
- `cargo test -p engine-core --lib` → PASS 263
- `pwsh scripts/run-compose-gates.ps1` → All gates passed
- Review: no criticals (evidence-only phase)

**Journey log** —
1. Formatters for Box/Sphere/Capsule already existed (phase 33); gap was multi-collider **combination** through prepared I/O → bridge.
2. Review: tests lock shape/offset/sensor, not full geometry float equality — acceptable for this AC.
3. Post-load `SetLocalPosition` in the test can mask transform-load bugs; acceptable for bridge-focused AC.
4. Compound impulse remains deferred.

## [S1] Problem

Phase 42–44 支持运行时多碰撞体进桥，缺少 SceneData 保存/加载后桥仍正确重建的证据。

## [S2] Design

| 项 | 契约 |
|----|------|
| 数据路径 | `SaveSceneJsonPrepared` → `SceneRuntime::load_scene_json` → `sync_physics_from_unity` |
| 期望 | JSON 含三碰撞体；桥 primary=Sphere，secondary=Box+Capsule（slot 0/1，offset/is_sensor 保留） |
| 回归 | 单碰撞体 SceneData 测试不回归 |

## [S3] Out of Scope

- compound 冲量
- 编辑器 UI 多碰撞体操作
- Android / VR·AR / git push（用户自理）

## Tasks

- [x] T1: native e2e 往返测试 — acceptance: 三碰撞体 GO 经 SceneData 后桥仍 primary+secondary（covers: S2）
- [x] T2: 文档 + 门禁 + finalize — acceptance: gates 不回归；Report（covers: S2; depends: T1）
