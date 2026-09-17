# Unity 对齐 · 延后任务清单（2026-09-17 交接）

供**新会话**从零恢复。勿把本文件当已完成规格；以 `docs/unity-alignment-roadmap.md` 与 `main` 代码为准。

## 当前仓库状态（后续会话更新）

| 项 | 值 |
|----|-----|
| 主线 | PR #7 已 **MERGED**；后续 B/C/D 切片已合入 `main` |
| Feature | `unity-world-primary` 默认 **off**；数组仍是存储权威 |
| 本地领先 | 以 `git status` / `git log origin/main..HEAD` 为准 |

### 已完成（勿重做）

- P1 生命周期、P3 资产、P4 协程、编辑器 Play / SceneData
- **PR #7** + **A1/A2** 合并
- **B1–B4** Transform 回填 / 写路径 / Hierarchy 双读 / Destroy 一致
- **B5** `MonoBehaviourInstances` + `restore_monobehaviours_from_ecs`
- **B6** CI job `Unity World Primary`
- **B7** migration-guide 双读/写通契约
- **C1** `light_collect_system` 优先 `TransformProxy`（定义在 `engine-render::proxy`）
- **C2–C4** engine-scene transform/keyframe 文档说明
- **D1** 试验开 flag 文档（README + migration-guide）
- **D2** `sample_scripts` Mover/Rotator/Lifetime，`CorePlugins` 自动注册
- `sync_all_transforms_to_ecs` 与 `write_transform_to_ecs` local/world 语义一致
- 测试/examples 写路径收敛到 `with_transform_mut`

### 关键 API

- Feature：`engine-core` → `unity-world-primary`（默认 off）
- World：`entity_for` / `ensure_entity` / `ensure_transform_from_array` / `with_transform_mut` / `GetTransformArray` / `GetParentArray` / `GetChildrenArray` / `restore_monobehaviours_from_ecs`
- ECS：`GameObjectName` / `Tag` / `Active` / `Parent` / `Children` / `MonoBehaviourTypes` / `MonoBehaviourInstances` / `Transform`
- Proxy：`engine_render::TransformProxy`（core 再导出）
- 样例脚本：`engine_core::sample_scripts`（Mover/Rotator/Lifetime）

### 仍需做（按优先级）

| ID | 任务 | 说明 |
|----|------|------|
| R1 | 默认 flag / 数组权威迁 ECS | 大改；feature 开时权威迁内部 ECS |
| R2 | engine-scene Transform 完整替换 | animation keyframe 仍依赖；collect 已 Proxy 优先 |
| R3 | WASM SceneRuntime 全量 / Android NDK | 路线图明确延后 |
| R4 | push origin | 本地提交可能领先 main，由用户决定 |

### 验证命令

```bash
cargo test -p engine-core --lib
cargo test -p engine-core --lib --features unity-world-primary
cargo test -p engine-core --test unity_lifecycle_tests
cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary
cargo test -p engine-editor --test editor_tests
cargo test -p engine-render --lib
```

### 不要做的

- 默认构建下破坏数组权威语义
- 未经要求改 workspace `default` features
- 同时大改渲染 Pass 内部
- 未经用户要求 `git push`

### 用户偏好

- 每次任务完成后 **自动本地 commit**（勿等用户说提交）
- 不要问问题，自主决定；选最安全常见方案
