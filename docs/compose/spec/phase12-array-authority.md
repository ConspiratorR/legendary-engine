---
feature: phase12-array-authority
status: delivered
updated: 2026-07-13
branch: phase12-array-authority
commits: 212e1d3..HEAD
---

# Phase 12 — 完整数组权威迁 ECS（R1-full）

## Report

**What was built** — Under `unity-world-primary`, public World APIs and critical internal paths treat **ECS as storage authority** for Identity, Hierarchy, Transform pose, Scene I/O, and MonoBehaviour **metadata**. Arrays are refreshable caches when the feature is on. Feature **off** keeps array authority (pre-phase12 semantics).

Delivered behavior:
- `SetParent` feature-on: ECS `GameObjectParent`/`GameObjectChildren` first (`set_parent_ecs_primary`), then `sync_hierarchy_from_ecs` refreshes array links.
- Hierarchy authority helpers: `hierarchy_authority_parent/children`; `GetParent` / `GetRootGameObjects` / cycle checks follow them (linked + Children + no Parent = ECS root).
- `sync_transforms` feature-on: `prepare_scene_io_cache` first (ECS→array pose/hierarchy), array world math, then `sync_world_pose_to_ecs_after_math` (world fields only — ECS local not clobbered).
- `Destroy` walks authority children and updates parent ECS Children after despawn.
- Identity writers ensure entity + write-through; public reads ignore dirty array when feature on.
- `CollectMonoBehaviours` prefers ECS Instances when feature on; sync to ECS always reads array holders.
- Scene serialize dual-reads `GetTransform`/`GetChildren`; `prepare_scene_io_cache` / `SavePrepared`; Load seeds then optional `restore_monobehaviours_from_ecs`.
- `is_descendant_of` hop-capped; SetActive compares authority under feature on.
- Docs: migration-guide R1-full tables, README 阶段 12, roadmap P2.4.

**Verification** (fresh after review fixes):

| Command | Result |
|---------|--------|
| `cargo fmt -p engine-core -p engine-editor -p engine-render -p engine-scene --check` | PASS |
| `cargo test -p engine-core --lib` | PASS 236 |
| `cargo test -p engine-core --lib --features unity-world-primary` | PASS 256 |
| `cargo test -p engine-core --test unity_lifecycle_tests` | PASS 21 |
| `cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary` | PASS 23 |
| `cargo test -p engine-core --test identity_bridge_tests` | PASS 10 |
| `cargo test -p engine-editor --test editor_tests` | PASS 60 |
| `cargo test -p engine-render --lib` | PASS 218; 29 ignored GPU **PRE-EXISTING** |
| `cargo test -p engine-scene --lib` | PASS 137 |
| `cargo build -p engine-core --examples` | PASS |
| Reviewer re-check `r1full_` tests feature-on | PASS 10/10; prior criticals MET |

**Ready-for-default-on assessment** — Dual-mode gate green; residual non-blocking items listed below. **Workspace default features stay `["audio"]`**; flip is out of scope for this branch.

**Journey log** —
1. `CollectMonoBehaviours` initially dual-read ECS inside MB sync (self-read). Split `collect_monobehaviours_from_holders` vs public Collect.
2. Feature-on SetParent/sync_transforms must not `write_transform_to_ecs` from array (clobbers ECS local). Pose path: ECS local → cache refresh → array world math → world-only write-back.
3. First review critical: `sync_transforms` still array-authoritative + array→ECS write. Fixed via `prepare_scene_io_cache` + `sync_world_pose_to_ecs_after_math`.
4. `GetParent` must share `hierarchy_authority_parent` root rule (Children-without-Parent = None), not fall through to dirty array.
5. Residuals (non-blocking): other `GetParent` while-walks (`IsActiveInHierarchy`, `hierarchy.rs`) lack hop caps; seed `write_transform_to_ecs` root test uses array `parent.is_none()`; editor save uses dual-read `Save` not `SavePrepared`. Branch local-only; no push.

## [S1] Problem

Phase 11 dual-read + pose write slices left hierarchy/identity/scene I/O/MB metadata array-authoritative under `unity-world-primary`. Feature-on storage authority was incomplete.

## [S2] Design

Implemented as specified: feature-on ECS authority for Identity/Hierarchy/Pose/Scene I/O/MB metadata; arrays as cache; dyn MB holders stay array-backed; feature-off array authority unchanged.

## [S3] Out of Scope

- Default flag flip (assessed ready, not landed)
- Dyn MonoBehaviour into ECS
- engine-scene deletion
- VR/AR / Android NDK / WASM SceneRuntime full
- Render pass rework
- Push / merge without user request

## Tasks

- [x] T1: 权威契约落盘 (covers: S2)
- [x] T2: 层级写权威 — SetParent ECS-first; Destroy/roots (covers: S2; depends: T1)
- [x] T3: Identity 写权威 + 分歧测试 (covers: S2; depends: T1)
- [x] T4: Pose cache / sync_transforms ECS 权威 (covers: S2; depends: T2) — after review fix
- [x] T5: Scene I/O 双模态 (covers: S2; depends: T4)
- [x] T6: MB 元数据权威 Instances (covers: S2; depends: T1)
- [x] T7: 内部路径审计 (covers: S2; depends: T2,T3)
- [x] T8: 文档与双模态门禁 (covers: S2; depends: T2,T3,T4,T5,T6,T7)