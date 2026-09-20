---
feature: phase18-unity-physics-bridge
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 95647a9..HEAD
---

# Phase 18 — SceneRuntime 物理 ↔ World 权威桥

## Report

**What was built** — Runtime Unity↔physics bridge in `engine-physics` (`unity_bridge.rs`): `sync_physics_from_unity` copies SceneRuntime Unity `Rigidbody`/colliders + world pose onto identity-bridge ECS entities; `PhysicsWorld::step` simulates; `sync_physics_to_unity` writes simulated world position via `World::SetLocalPosition` (storage authority). `UnityPhysicsPlugin` registers a FixedUpdate system that no-ops without SceneRuntime. Headless example `unity_physics_demo` spawns a falling ball and asserts World Y decreases. Docs updated (`unity-storage-animation.md`, README run list + 阶段 18).

**Verification**:

| Command | Result |
|---------|--------|
| `cargo test -p engine-physics --lib` | PASS **72** (incl. gravity writes World pose; skip-without-rigidbody) |
| `cargo run -p engine-core --example unity_physics_demo` | **PASS** — Y 8.0 → 7.76 over 30 frames; assert held |
| `cargo test -p engine-core --lib` | PASS 261 |
| `cargo build -p engine-core --examples` | PASS |
| `cargo fmt -p engine-physics -p engine-core` | PASS |

**Journey log** —
1. engine-core cannot depend on engine-physics (cycle) — bridge lives in `engine-physics`.
2. Editor Play-host sync pattern reused for runtime SceneRuntime.
3. `AppBuilder` needs `.build()` before `run_with_lifecycle` / `unity_world_ref`.
4. Write-back uses `SetLocalPosition` (works feature on/off); do not dual-write array-only.
5. git merge/push not handled per user preference.

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
