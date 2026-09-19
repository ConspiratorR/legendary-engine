---
feature: phase12-array-authority
status: in-progress
updated: 2026-07-13
branch: phase12-array-authority
commits: # filled at delivery
---

# Phase 12 — 完整数组权威迁 ECS（R1-full）

## Report

（设计阶段留空；交付时填写）

## [S1] Problem

阶段 11 在 `unity-world-primary` 下已交付：dual-read 优先 ECS、`SetLocal*` / `with_ecs_transform_mut` 写权威切片、动画位姿写回 World、编辑器 Play/拾取验收。但 **存储权威仍不完整**：

- 公共 **读** 在 feature 开启时优先 ECS；
- **位姿写** 权威可落在 ECS（数组为 pose 缓存）；
- **层级链接、GameObject 身份字段、场景 I/O、MonoBehaviour 实例 holder、世界位姿合成 / 根遍历** 仍以数组为权威写源；
- 文档明确：`GetTransformArray` / `GetParentArray` / `GetChildrenArray` 仍用于 hierarchy math 与 scene I/O；MB holder 数组权威，ECS 仅可恢复镜像。

因此 feature 开启时，系统仍处在「读优先 ECS + 写路径混权」状态：公读公写可能一致，但内部分歧、Save/Load 边界、`sync_*` 方向仍有未覆盖契约。完整「数组降为缓存」是 roadmap / migration-guide 唯一仍 🔞 的阶段 10–11 主线，也是后续评估默认开 flag 的前置条件。

## [S2] Design

### 目标语义（仅在 feature **on**）

| 存储族 | 权威（feature on） | 数组角色 | 公共读 | 公共写 |
|--------|-------------------|----------|--------|--------|
| Identity（Name/Tag/Active/Layer） | ECS 镜像组件 | 缓存 / 兼容 `GameObject` 字段 | ECS 优先 | 公 API 写 ECS，再刷数组 |
| Hierarchy（Parent/Children） | ECS `GameObjectParent`/`GameObjectChildren` | 缓存（用于 sync 回刷与兼容路径） | ECS 优先 | 公 API 写 ECS，再刷数组；环检测在写前用当前权威或显式 array 快照 |
| Transform 位姿 | ECS `Transform`（local 权威） | local pose 缓存 | ECS 优先 | `SetLocal*` / `with_ecs_transform_mut` 已是权威路径 |
| 世界位姿 / 层级数学 | 缓存刷新后的数组 world/local（`sync_transforms` 消费） | 必须在数学前从 ECS 权威刷新 pose | 公 API 不暴露脏缓存 | 写 pose 后由权威路径触发 cache 刷新 |
| Scene I/O | feature on：Save 前 `sync_all_transforms_from_ecs` + 身份/层级一致性；Load 后 seed ECS → 权威切到 ECS | Load 时先物化数组再 seed | 序列化契约见下 | 序列化仍产出 SceneData；反序列化 seed 双侧 |
| MB **元数据**（type_name/enabled/props） | ECS `MonoBehaviourInstances` | 可重建缓存 | feature on 下 Serialize/Collect 元数据以 ECS 为准（有 entity 时） | Add/Enable/Remove 写通 ECS；holder 运行时仍数组 |
| MB **运行时实例** `Box<dyn>` | 数组 `monobehaviours` holder（**不迁 ECS**） | 运行时唯一源 | 不变 | 不变 |

Feature **off**：与阶段 11 及当前 main 一致——数组权威；ECS 镜像 best-effort；**零语义变化**（门禁不变量）。

### 架构约束

1. **不删** `gameobject_data` / `transforms` / `monobehaviours` 槽；不把 `dyn MonoBehaviour` 放入 ECS。
2. 继续沿用现有 `sync_*_to_ecs` / `seed_*_from_array` / `with_ecs_transform_mut` / dual-read 模式，不发明第四套存储。
3. `seed_ecs_from_array` / `Get*Array` **仅** 作为：Load/Instantiate/工具导入边界、以及 feature **off** 权威路径；feature **on** 的公写不得「只写数组、指望事后 seed」。
4. `sync_hierarchy_to_ecs` 在 feature on 的公写路径上改为「从当前权威刷新缓存」：优先以 ECS 已写入的 Parent/Children 为准回写数组；feature off 仍可数组→ECS seed。
5. 不改渲染 Pass / WGSL 内部；不改 workspace default features；不 push 除非用户明确要求。

### 关键行为契约

**写层级（`SetParent` 等，feature on）**

1. 写前用**当前权威**（有 entity 且有 ECS Parent/Children 时用 ECS，否则数组）做环检测/合法性。
2. 更新 ECS `GameObjectParent` / `GameObjectChildren`（及子/新旧父对称）。
3. 将 ECS 层级刷新到数组 `transforms[].parent/children`（cache）。
4. 触发 pose cache 刷新（`write_transform_to_ecs` / `sync_transform_from_ecs` 按需要，保持 local/world 契约与阶段 11 B1/R1d 一致）。
5. feature off：保持现有数组权威 + best-effort ECS mirror。

**场景 Save（feature on）**

1. `sync_all_transforms_from_ecs`（pose cache）。
2. 确保身份/层级/MB 元数据与 ECS 权威一致（已 dual-write 的路径不回归）。
3. 序列化仍读稳定字段；禁止在 Save 路径上把 dual-read 污染进 hierarchy math（继续用明确的 cache API 或刷新后的数组）。

**场景 Load（feature on）**

1. 物化数组 + GameObject 字段（现状）。
2. `seed_all_ecs_from_array`（或 per-handle seed）。
3. 之后所有公 API 按 ECS 权威运行；数组为缓存。

**MB（feature on）**

1. `AddMonoBehaviour*` / enable/disable / remove：更新 holder **并** 写通 `MonoBehaviourInstances`（现状已有 types/instances 同步）。
2. `Serialize` / SceneData 往返 / 元数据 Collect：有 entity 且 Instances 存在时以 ECS 为准；否则数组 holder。
3. `restore_monobehaviours_from_ecs` 仍为 holder 重建入口；Load 默认是否自动改走 ECS 重建：**是**（feature on），并保留数组路径为 fallback。

### 验证边界

- 双模态（default / `--features unity-world-primary`）跑通阶段 11 §4 门禁 + 本阶段新增 contract 测试。
- 新增测试必须含 **人为制造 ECS/数组分歧** 后断言：feature on 公 API 读写跟 ECS；feature off 跟数组（模式类似 `test_dual_read_storage_mode_contract_when_ecs_diverges`）。
- 默认构建与合入前行为一致：生命周期顺序、SceneData 往返、无 feature 时 API 结果不变。

## [S3] Out of Scope

- 默认打开 `unity-world-primary` / 修改 workspace `default` features（仅产出「是否 ready」评估结论，不落地 flip）
- 将 `Box<dyn MonoBehaviour>` 或完整组件系统迁入 ECS
- 删除 `engine-scene` 包或 `Transform` 类型
- VR/AR、Android NDK 运行时、WASM SceneRuntime 全量
- 渲染 Pass / WGSL / editor 大 UI 重构
- 未经用户要求的 `git push` / 远程 PR
- 合并 `phase12-array-authority` → `main`（Finish 阶段由用户决定）

## Tasks

- [ ] T1: 权威契约落盘 — 更新 migration-guide「Write-authority / Array-authoritative」表，明确 feature on 时各存储族权威与数组 cache 角色；world.rs 模块注释同步 — acceptance: 文档表覆盖 Identity/Hierarchy/Pose/Scene I/O/MB metadata；无 TBD（covers: S2）
- [ ] T2: 层级写权威 — `SetParent` 及 Destroy/Instantiate 相关层级更新在 feature on 时以 ECS Parent/Children 为写权威并回刷数组 cache；环检测使用当前权威 — acceptance: 双模态测试；feature on 下人为改 ECS parent 后公 `GetParent` 与随后写路径一致；feature off 行为不变（covers: S2；depends: T1）
- [ ] T3: Identity 写权威硬化 — `SetName`/`SetTag`/`SetActive`/`SetLayer` 在 feature on 写后公读不依赖数组；补分歧写→读测试 — acceptance: 新测试在 feature on 下将数组侧改脏后 `Get*` 仍读 ECS；feature off 测试仍绿（covers: S2；depends: T1）
- [ ] T4: Pose/层级数学 cache 刷新 — feature on 下 `sync_transforms` / 世界位姿路径消费「自 ECS 刷新后的」pose cache；公 `GetTransform` 不暴露长期不一致 — acceptance: `with_ecs_transform_mut` / `SetLocal*` 后不经手写 sync 的层级数学与 Save 前刷新契约有测试覆盖（covers: S2；depends: T2）
- [ ] T5: Scene I/O 双模态契约 — Load：seed ECS 后权威在 ECS；Save：feature on 先刷新 pose cache 再序列化 — acceptance: 往返测试 default + feature on；feature on Save 前人为改 ECS pose 后落盘与 ECS 一致（covers: S2；depends: T4）
- [ ] T6: MB 元数据权威 — feature on 下 Serialize/restore/Instances 以 ECS 元数据为准；holder 仍为 dyn 运行时存储；Load 可 `restore_monobehaviours_from_ecs` — acceptance: feature on 清空 holder 元数据后 restore/serialize 路径可恢复；feature off 不变（covers: S2；depends: T1）
- [ ] T7: 内部路径审计 — 检查 `world.rs`/`serialization.rs`/`identity_bridge.rs`/editor 写路径：feature on 公写不绕过权威；`Get*Array` 调用点标注 cache vs seed 边界 — acceptance: 审计清单写入本文件 Report/Journey；发现的绕过点在本阶段修完或记为 Out of Scope 附录（covers: S2；depends: T2,T3）
- [ ] T8: 文档与门禁 — 同步 README 路线图阶段 11/12、`unity-alignment-roadmap.md` P2.4、`docs/migration-guide.md` Still deferred；跑完整双模态门禁并记录 — acceptance: 门禁表写入 Report；默认 features 仍为 `["audio"]`；ready-for-default-on 仅评估不落地（covers: S2；depends: T2,T3,T4,T5,T6,T7）

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
