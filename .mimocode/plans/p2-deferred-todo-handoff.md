# Unity 对齐 · 延后任务清单（2026-09-17 交接）

供**新会话**从零恢复。勿把本文件当已完成规格；以 `docs/unity-alignment-roadmap.md` 与 `main` 代码为准。

## 当前仓库状态

| 项 | 值 |
|----|-----|
| 主线 | Unity PlayerLoop / MonoBehaviour / SceneRuntime 已在 `main` |
| 本迭代未合分支 | `p2-deferred-storage` → **PR #7 CONFLICTING** |
| PR #7 | https://github.com/ConspiratorR/legendary-engine/pull/7 |
| 分支 HEAD（本地已推） | `08931aa` |
| 基线曾对齐 | `origin/main` 上曾有 PR #1–#6 已合；main 此后可能继续前进 |

### PR #7 内容（待解冲突后合并）

1. `6928914` — `unity-world-primary` 下 `SetParent`/`sync_transforms` 写回 ECS；层级同步**只读数组** `get_transform_array`（勿用 dual-read `GetTransform` 算世界坐标）
2. `d53438e` — `GameObjectParent` / `GameObjectChildren`；`SetParent` 写通
3. `08931aa` — Destroy 内部 `ecs.despawn`；父节点 children 组件更新；`scene_bridge` 注释为可选旧路径
4. 文档：engine-scene Transform 盘点；lifecycle 延后状态

### 解冲突步骤（用户/主 agent 做，勿在受限 worktree 强行 merge）

```bash
git fetch origin
git checkout p2-deferred-storage
git merge origin/main
# 解决 crates/engine-core/src/world.rs 与 docs 冲突后
cargo test -p engine-core --lib
cargo test -p engine-core --lib --features unity-world-primary
git push
# 然后 gh pr merge 7
```

## 已完成（勿重做）

- P1 生命周期硬化、P3 资产、P4 协程、编辑器 Play / SceneData 孪生打开与自动保存  
- P2.4 **切片**：flag `unity-world-primary`（默认 **off**）、World 自持 handle↔entity、Name/Tag/Active/Transform/MBTypes 写通、dual-read 读侧、Hierarchy 组件、Destroy despawn  
- P2.5 **盘点**已写入 roadmap（非完整替换）  
- P2.6 编辑器主路径（视口 World 权威、`save_scene_bundle` / `open_scene_file` 优先 SceneData）

## 仍需做（按优先级）

### A. 立刻（收尾当前分支）

| ID | 任务 | 验收 |
|----|------|------|
| A1 | 解决 PR #7 与 main 冲突并合并 | PR MERGED；`main` 含 write-through 代码 |
| A2 | 合并后本地 `main` 拉取；删或归档分支 | `git status` 干净 |

### B. P2.4 完整存储迁移（大、feature 门控、默认 off）

数组 `gameobject_data` / `transforms` / `monobehaviours` **仍是权威**。目标：`unity-world-primary` 时权威迁到内部 ECS。

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| B1 | Transform 读路径 | feature 下 `GetTransform` 已 dual-read ECS；补：若 ECS 缺组件则从数组回填 | 单测 + lifecycle feature 测试 |
| B2 | Transform 写路径收敛 | 所有 `GetTransformMut` 改为 `with_transform_mut` 或事后 `sync_transform_to_ecs`；gizmo/命令/inspector 统一 | 全库 grep `GetTransformMut` 清单，逐个处理 |
| B3 | Hierarchy 权威 | feature 下 GetParent/GetChildren 优先 ECS Parent/Children；与数组双向一致 | 父子查询测试 |
| B4 | Destroy 统一 | destroy_internal 已 despawn；确认 pending_destroy / flush_destroy 路径一致 | 销毁后 entity 无效 |
| B5 | MonoBehaviour 存储 | feature 下 MB 列表存 ECS 资源或 per-entity；`MonoBehaviourTypes` 扩展为可恢复实例（依赖注册表） | SceneData 往返 |
| B6 | 默认 flag 开启试验 | 单独 CI job：`--features unity-world-primary` 全测 | CI 绿 |
| B7 | 文档 | migration-guide：双读/写通契约 | 文档更新 |

**风险**：勿在未跑 feature 测试时改 `sync_transforms` 世界坐标语义；层级必须用数组算父，再写 ECS。

### C. P2.5 收敛 engine-scene Transform

| ID | 任务 | 说明 |
|----|------|------|
| C1 | `engine-render::collect_system` | 评估改用 `TransformProxy` / core `Transform`（禁改 Pass 内部，只改数据源） |
| C2 | animation_editor keyframe | 暂保留 engine-scene keyframe；文档标明与 core Transform 关系 |
| C3 | 3D 视口 mesh 路径 | `build_scene` 已用 World 位姿；mesh 列表仍节点表 — 非 engine-scene Transform |
| C4 | 弃用警告 | `engine_scene::transform::Transform` 模块 doc：`#[deprecated]` 或 migration 注明 |

### D. 可选/产品向

| ID | 任务 |
|----|------|
| D1 | 编辑器默认打开 `unity-world-primary` 试验构建 |
| D2 | 更多 MonoBehaviour 注册进 SceneData |
| D3 | WASM / Android（原始终延后） |

## 关键 API（恢复上下文）

- Feature：`engine-core` → `unity-world-primary`（默认 off）  
- World：`entity_for` / `ensure_entity` / `sync_transform_to_ecs` / `sync_all_transforms_to_ecs` / `with_transform_mut` / `get_transform_array`  
- ECS 组件：`GameObjectName` / `Tag` / `Active` / `Parent` / `Children` / `MonoBehaviourTypes` / `Transform`  
- 编辑器：`save_scene_bundle` / `open_scene_file` / `UnityPlayHost`  
- 双读：feature 下 `GetTransform`/`GetName`/`GetTag`/`IsActive` 优先 ECS；**层级数学用数组**

## 验证命令

```bash
cargo test -p engine-core --lib
cargo test -p engine-core --lib --features unity-world-primary
cargo test -p engine-core --test unity_lifecycle_tests
cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary
cargo test -p engine-editor --test editor_tests
```

## 不要做的

- 默认构建下破坏现有数组权威语义  
- 同时大改渲染 Pass 内部  
- 未经用户要求合 `main`  
- 在沙箱 worktree 上强制 `git checkout main` / merge 跨分支（由用户或主会话处理）
