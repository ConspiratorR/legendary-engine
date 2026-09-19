---
feature: phase12-array-authority
status: in-progress
updated: 2026-07-13
branch: phase12-array-authority
commits: 212e1d3..HEAD
---

# Phase 12 — 完整数组权威迁 ECS（R1-full）

## Report

**What was built** — Under `unity-world-primary`, public World APIs and critical internal paths now treat **ECS as storage authority** for Identity, Hierarchy, Transform pose, Scene I/O, and MonoBehaviour **metadata**. Arrays (`gameobject_data` / `transforms[].parent|children|pose` / holders) are refreshable caches when the feature is on. Feature **off** keeps array authority with best-effort ECS mirrors (pre-phase12 semantics).

Concrete changes:
- `SetParent` feature-on path writes ECS `GameObjectParent`/`GameObjectChildren` first (`set_parent_ecs_primary`), then `sync_hierarchy_from_ecs` refreshes array links.
- Hierarchy reads/walks use `hierarchy_authority_parent/children`; `GetRootGameObjects` and cycle detection follow authority.
- `Destroy` walks authority children and updates parent ECS children after despawn.
- Identity `SetName`/`SetTag`/`SetLayer`/`SetActive` `ensure_entity` then write-through; dual-read tests cover dirty-array cases.
- `Instantiate` pose uses `with_ecs_transform_mut` + `seed_ecs_from_array`.
- `CollectMonoBehaviours` prefers ECS `MonoBehaviourInstances` when feature on; `sync_monobehaviour_types_to_ecs` / `collect_monobehaviours_from_holders` always read array holders (runtime dyn store) so sync never dual-reads ECS.
- Scene serialize uses dual-read `GetTransform`/`GetChildren`; `World::prepare_scene_io_cache` + `SceneSerializer::SavePrepared` refresh caches; Load seeds then optional `restore_monobehaviours_from_ecs`.
- Docs: migration-guide write-authority / array-API / still-deferred tables; README 阶段 12; roadmap P2.4.

**Out of scope still true:** default `unity-world-primary` flip (workspace stays `["audio"]`), `Box<dyn MonoBehaviour>` into ECS, engine-scene deletion, VR/AR/Android/WASM SceneRuntime full.

**Verification** (fresh on branch):

| Command | Result |
|---------|--------|
| `cargo fmt -p engine-core -p engine-editor -p engine-render -p engine-scene --check` | PASS |
| `cargo test -p engine-core --lib` | PASS 233 |
| `cargo test -p engine-core --lib --features unity-world-primary` | PASS 253 |
| `cargo test -p engine-core --test unity_lifecycle_tests` | PASS 21 |
| `cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary` | PASS 23 |
| `cargo test -p engine-core --test identity_bridge_tests` | PASS 10 |
| `cargo test -p engine-editor --test editor_tests` | PASS 60 |
| `cargo test -p engine-render --lib` | PASS 218; 29 ignored GPU **PRE-EXISTING** |
| `cargo test -p engine-scene --lib` | PASS 137 |
| `cargo build -p engine-core --examples` | PASS |

**Journey log** —
1. `CollectMonoBehaviours` initially dual-read ECS inside `sync_monobehaviour_types_to_ecs` (self-read). Split holders collector vs public Collect; sync always holders.
2. Feature-on hierarchy writes must not `sync_transform_to_ecs` from array (would clobber ECS local pose); use `ensure_transform_from_array` + `sync_transform_from_ecs`.
3. Array `Get*Array` remains for seed/export and feature-off authority; public hierarchy APIs use authority helpers.
4. Default-on readiness: dual-mode gate green, but flag flip is a **separate** decision (not this branch).
5. Worktree: user chose main worktree; branch `phase12-array-authority` local only — no push.

## [S1] Problem

Phase 11 delivered dual-read + pose write-authority slices, but hierarchy links, identity fields, scene I/O, MB metadata, and root walks still treated arrays as the write authority under `unity-world-primary`. Feature-on storage authority was incomplete.

## [S2] Design

See design section above (storage family table + contracts). Implemented as specified.

## [S3] Out of Scope

- Default flag flip
- Dyn MonoBehaviour into ECS
- engine-scene deletion
- VR/AR / Android NDK / WASM SceneRuntime full
- Render pass/WGSL rework
- Push / merge without user request

## Tasks

- [x] T1: 权威契约落盘 — migration-guide + world 注释 (covers: S2)
- [x] T2: 层级写权威 — SetParent ECS-first; Destroy/Instantiate (covers: S2; depends: T1)
- [x] T3: Identity 写权威硬化 + 分歧测试 (covers: S2; depends: T1)
- [x] T4: Pose/层级数学 cache 刷新契约 (covers: S2; depends: T2)
- [x] T5: Scene I/O 双模态契约 (covers: S2; depends: T4)
- [x] T6: MB 元数据权威 Instances (covers: S2; depends: T1)
- [x] T7: 内部路径审计 (covers: S2; depends: T2,T3) — holders-sync split; serialize dual-read; authority helpers on roots/destroy/cycle
- [x] T8: 文档与门禁 — README/roadmap/migration-guide + dual-mode gate (covers: S2; depends: T2,T3,T4,T5,T6,T7)

## 验证门禁（每个实现任务结束）

```bash
cargo fmt -p engine-core -p engine-editor -p engine-render -p engine-scene --check
cargo test -p engine-core --lib
cargo test -p engine-core --lib --features unity-world-primary
cargo test -p engine-core --test unity_lifecycle_tests
cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary
cargo test -p engine-core --test identity_bridge_tests
cargo test -p engine-editor --test editor_tests
cargo test -p engine-render --lib
cargo test -p engine-scene --lib
cargo build -p engine-core --examples
```

**不变量（feature off）：** 数组权威读写、生命周期回调顺序、SceneData 往返、与阶段 11 收尾 main 行为一致。

**目标（feature on）：** 公 API 与关键内部写路径上，ECS 为 Identity/Hierarchy/Pose/MB-metadata 权威；数组为可刷新 cache；Load/Instantiate 后 seed 齐全。

## 执行约束（来自项目偏好）

- 不问琐碎问题；选最安全、最常见方案；跟现有 dual-read / seed / sync 模式
- 每任务完成后 **自动本地 commit**（分支 `phase12-array-authority`，勿 push）
- 编辑器相关改动需可测或实机验证说明
- 本文件为唯一 feature 文档；阶段执行计划可同步 `.mimocode/plans/` 但不替代本 spec
