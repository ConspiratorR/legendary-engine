---
feature: phase18-unity-physics-bridge
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 95647a9..HEAD
---

# Phase 18 — SceneRuntime 物理 ↔ World 权威桥

## Report

**What was built** — Runtime Unity↔physics bridge in `engine-physics` (`unity_bridge.rs`): `sync_physics_from_unity` copies SceneRuntime Unity `Rigidbody`/colliders + pose onto identity-bridge ECS entities; `PhysicsWorld::step` simulates; `sync_physics_to_unity` writes **world pose → parent-local** via `InverseTransformPoint` + `SetLocalPosition`, and **round-trips linear/angular velocity** onto Unity `Rigidbody`. Velocity authority: existing bodies keep simulation velocity; Unity non-zero velocity seeds when published. `UnityPhysicsPlugin` FixedUpdate runs from→step→to when SceneRuntime exists. Headless `unity_physics_demo` uses **plugin only** (no double-step). Editor Play-host sync applies the same velocity/parent-local fixes.

**Verification** (after review critical fixes):

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **72** (gravity test asserts ΔY > 0.5 **and** Unity `velocity.y < -0.5`) |
| `cargo run -p engine-core --example unity_physics_demo` | **PASS** — plugin-only: Y **8.0 → 6.18**, vy **0 → −5.89** (≈ g·t); assert `end_y < 7.0` |
| `cargo test -p engine-editor --test editor_tests` | PASS 61 |
| `cargo test -p engine-core --lib` | PASS 261 |

**Journey log** —
1. Bridge lives in `engine-physics` (engine-core → physics would cycle).
2. **Critical:** copying Unity `velocity` every step without writeback zeroed simulation — fixed: sim owns velocity; `to_unity` writes velocity back.
3. **Critical:** world pose written as local corrupts parented bodies — fixed: `InverseTransformPoint` under parent.
4. Demo must not double-step (plugin FixedUpdate + manual step inflated weak asserts).
5. Signature of zero-reset gravity: ΔY ≈ n·g·dt² (tiny); free-fall ΔY ≈ ½g t².

## [S1] Problem

No runtime SceneRuntime physics bridge; only editor Play host synced Rigidbody.

## [S2] Design

unity_bridge + UnityPhysicsPlugin + demo as specified; PhysicsPlugin ECS-only path unchanged.

## [S3] Out of Scope

- Full MonoBehaviour collision callbacks, WASM/Android, git merge/push

## Tasks

- [x] T1: unity_bridge + UnityPhysicsPlugin (covers: S2)
- [x] T2: unity_physics_demo (covers: S2; depends: T1)
- [x] T3: 文档 + 门禁 + finalize (covers: S2; depends: T1,T2)
