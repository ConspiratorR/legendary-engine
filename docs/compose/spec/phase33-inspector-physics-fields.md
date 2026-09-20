---
feature: phase33-inspector-physics-fields
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 5393bc6..HEAD
---

# Phase 33 — Inspector 物理字段编辑

## Report

**What was built** — Inspector physics: Rigidbody sleep (uses `Sleep()`/`WakeUp()`) + velocity; Sphere/Box/Capsule shape fields; section shows for Rigidbody **or** any collider; add-component menu includes 球/胶囊碰撞体. **Persistence (review C1)**: editor `PhysicsDataSer` stores drag/angular_drag/use_gravity/is_sleeping/velocity + collider center/size/radius/height/direction/is_trigger (serde defaults for old files); core `SceneSerializer` adds Box/Sphere/Capsule formatters + Rigidbody `is_sleeping`. Collider type priority aligned **Sphere → Box → Capsule** (bridge order). Test asserts core JSON + editor scene bundle roundtrip of shape params.

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-core --lib` | PASS **261** (formatters 10) |
| `cargo test -p engine-editor --test editor_tests` | PASS **63** |

**Journey log** —
1. Review C1: authoring without persistence is lossy — schema + core formatters required.
2. Review C3: physics panel gated on RB only hid colliders; menu lacked Sphere/Capsule.
3. Review C4: Sleep checkbox must zero velocities via `Sleep()`.
4. Review C5: collider type priority aligned with unity_bridge.
5. git merge/push not handled per user preference.

## [S1] Problem
Inspector fields + persistence gaps from phase33 review.

## [S2] Design
UI + PhysicsDataSer + core collider formatters + Sleep semantics + menu items.

## [S3] Out of Scope
Collision gizmos, WASM/Android, git merge/push

## Tasks

- [x] T1: Inspector 物理字段 UI (covers: S2)
- [x] T2: 持久化 + 文档 + finalize (covers: S2)
