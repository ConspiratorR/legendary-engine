---
feature: phase13-default-on-hardening
status: designed
updated: 2026-07-13
branch: phase12-array-authority
commits: # filled at delivery
---

# Phase 13 — 默认开 flag 验收 + 残余硬化

## Report

（设计阶段留空）

## [S1] Problem

阶段 12 在 `unity-world-primary` 下交付了 R1-full 存储权威契约，但：

1. 复审遗留：除 `is_descendant_of` 外，父链遍历（`IsActiveInHierarchy`、`hierarchy::get_ancestors/get_root/is_ancestor/get_depth`）无 hop 上限，损坏的 parent 环会挂起。
2. 编辑器场景导出仍走 `Save`/`SaveSceneJson`，未在序列化前 `prepare_scene_io_cache`（feature on 时 pose cache 可能陈旧）。
3. `write_transform_to_ecs` 根节点判定用数组 `parent.is_none()`，seed 路径可能写入错误 world pose。
4. workspace/`engine-core` 默认 features 仍为 `["audio"]`，`unity-world-primary` 需手动打开；阶段 13 验收要求默认构建即 ECS 权威契约。

## [S2] Design

### 残余硬化

| 项 | 契约 |
|----|------|
| 父链 hop-cap | 统一上限（沿用 `MAX_HOPS = 4096`）；超限/自环视为断链（root/无祖先），不挂起 |
| 层级工具 | `hierarchy::get_ancestors/get_root/is_ancestor/get_depth` 使用带 cap 的 `GetParent` 行走 |
| `IsActiveInHierarchy` | 父链 walk 使用 cap |
| 编辑器 Save | `to_core_scene_data`/`export_core_scene_json` 及 bundle 保存：先 `world.prepare_scene_io_cache()`，再序列化；提供 mut 路径或在调用前刷新 |
| seed 根判定 | feature on 时 `write_transform_to_ecs` 用 `hierarchy_authority_parent` 是否为 None 判断 root |
| `SaveSceneJson` | 增加 `SaveSceneJsonPrepared(&mut World)` 或文档要求调用方 prepare；编辑器改走 prepared |

### 默认开启 flag

1. `engine-core` Cargo.toml：`default = ["audio", "unity-world-primary"]`。
2. Feature **仍存在**：`default-features = false` + `features = ["audio"]` 可退回数组权威（迁移/兼容）。
3. 文档：migration-guide / README / roadmap 说明默认 on，以及如何 opt-out。
4. 门禁：默认构建（现等于 feature on）全绿；显式 `--no-default-features --features audio`（或仅 default-features off）仍可编译测试 feature-off 语义（若 CI 成本高则至少本地跑 core lib）。
5. **不**删除 feature，**不**改 dyn MB 进 ECS，**不** push/merge 除非用户要求。

### 验证

- 默认：`cargo test -p engine-core --lib` 等（将启用 unity-world-primary）
- 兼容：`cargo test -p engine-core --lib --no-default-features --features audio`（feature-off 语义）
- 编辑器：`cargo test -p engine-editor --test editor_tests`
- fmt + 相关 crate 测试与阶段 12 门禁对齐

## [S3] Out of Scope

- 删除 `unity-world-primary` feature 或数组 cache 槽
- dyn MB 迁 ECS
- engine-scene 删除 / VR/AR / Android NDK / WASM SceneRuntime 全量
- 未经要求 push / 把 phase12 合入 main（用户已选续写分支）

## Tasks

- [ ] T1: 父链 hop-cap — `hierarchy.rs` 四函数 + `IsActiveInHierarchy` — acceptance: 自环/互环 parent 下调用返回且不挂；有单测（covers: S2）
- [ ] T2: 编辑器/序列化 prepared save — `prepare_scene_io_cache` + prepared JSON；编辑器导出路径 — acceptance: feature-on 下 ECS pose 变更后 export 与 ECS 一致（covers: S2；depends: T1）
- [ ] T3: seed 根判定用 hierarchy authority — `write_transform_to_ecs` — acceptance: feature-on dirty array parent 不影响 seed world pose 根刷新（covers: S2）
- [ ] T4: 默认 features 打开 `unity-world-primary` + opt-out 文档 — Cargo.toml + migration-guide/README/roadmap — acceptance: 默认 `cargo test -p engine-core --lib` 含 R1full 契约；`--no-default-features --features audio` 仍绿（covers: S2；depends: T1,T2,T3）
- [ ] T5: 门禁 + spec finalize — 双模态（default / feature-off）记录 — acceptance: Report 写入命令与计数（covers: S2；depends: T4）
