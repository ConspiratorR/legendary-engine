# P2 后续详细计划（PR #7 合入后）

基线：`main` 含 PR #7 + B1–B4/B6/B7 + C2–C4 文档（commit `147f053` 及 merge `d9e0130`）。  
Feature：`unity-world-primary` 默认 **off**；数组仍是存储权威。  
本计划覆盖「收尾 → B5 完整 → C1 → D 可选」四阶段。

---

## 0. 范围与非目标

### 本迭代做

1. 仓库收尾（分支、roadmap、examples 写通）
2. **B5** MonoBehaviour 实例可恢复存储（feature 门控）
3. **C1** `light_collect_system` 数据源可选读 core/Proxy 位姿（不动 Pass 内部）

### 本迭代不做

- 默认打开 `unity-world-primary`
- 删除 `engine_scene::transform::Transform`（只文档标注）
- WASM / Android（D3）
- 渲染 Pass / 着色器内部重构
- 未经要求的 `main` 合并或远程 push

---

## Phase 1 — 收尾稳固（约 0.5 天）

### 1.1 清理已合并分支

| 步骤 | 命令/动作 | 验收 |
|------|-----------|------|
| 确认已合 | `git branch --merged main` 含 `p2-deferred-storage` | 是 |
| 删本地分支 | `git branch -d p2-deferred-storage` | 分支不存在 |
| 删远程（若仍在） | `git push origin --delete p2-deferred-storage` | 远程无该分支 |
| 勿动 | `.opencode/`、`openspec/`（本地未跟踪，不入库） | 仍 untracked |

### 1.2 更新 `docs/unity-alignment-roadmap.md`

P2.b 表改为反映真实状态：

| ID | 新状态文案 |
|----|------------|
| P2.4 | dual-read + Transform/Hierarchy/Active/MBTypes **写通已完成**；完整数组权威迁移（实例进 ECS）→ **B5 剩余** |
| P2.5 | 盘点 + 模块 doc 完成；`collect_system` 仍读 `GlobalTransform`（本计划 C1 处理） |
| 新行 P2.12 | B5：MB 实例（TypeName+props）进 ECS，SceneData 往返以注册表恢复 |

### 1.3 Examples 写通收敛

| 文件 | 行（约） | 改法 |
|------|----------|------|
| `crates/engine-core/examples/basic.rs` | 47, 54 | `with_transform_mut` |
| `crates/engine-core/examples/unity_gameplay_demo.rs` | 73, 121 | 同上 |
| `crates/engine-core/examples/runtime_scene_demo.rs` | 43 | 同上 |

可选：`tests/unity_api_tests.rs`、`tests/editor_tests.rs`、`editor/tests/editor_tests.rs` 同样收敛（测试可后置）。

### 1.4 验证

```bash
cargo test -p engine-core --lib
cargo test -p engine-core --lib --features unity-world-primary
cargo test -p engine-core --test unity_lifecycle_tests
cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary
cargo test -p engine-core --test editor_tests
cargo test -p engine-editor --test editor_tests
cargo build -p engine-core --examples
```

**提交建议**（一个 commit）：  
`chore: post-merge cleanup — roadmap, examples write-through, drop merged branch`

---

## Phase 2 — B5 完整 MonoBehaviour 存储（约 1–2 天）

### 目标

feature 开启时，MB **元数据**（类型名、enabled、props）在 ECS 上可恢复；数组 `monobehaviours` 仍是实例 holder 权威，但 **Save/Load 与 dual 一致性**不依赖「碰巧还在数组里」。

### 2.1 ECS 组件设计（新）

在 `world.rs`（或独立小模块）增加：

```rust
/// 可恢复的 MonoBehaviour 描述（不持有 dyn 实例）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonoBehaviourInstance {
    pub type_name: String,
    pub enabled: bool,
    pub props: Option<serde_json::Value>,
}

/// 覆盖 MonoBehaviourTypes；与数组 CollectMonoBehaviours 对齐
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MonoBehaviourInstances(pub Vec<MonoBehaviourInstance>);
```

说明：

- **不要**把 `Box<dyn MonoBehaviour>` 放进 ECS。
- 保留现有 `MonoBehaviourTypes`（仅名字列表）或让 B5a 写 `MonoBehaviourInstances` 并逐步弃用 Types——推荐 **Instances 为权威镜像**，Types 可同步或删除（见 2.5）。

### 2.2 写通点

| API | 动作 |
|-----|------|
| `AddMonoBehaviourBoxed` | 现有 `sync_monobehaviour_types_to_ecs` → 改为/同时 `sync_monobehaviour_instances_to_ecs` |
| `RestoreMonoBehaviours` | 每 add 后同步 |
| Enable/Disable 路径（若有 `SetEnabled` 写回） | 同步 enabled 标志 |
| `destroy_internal` | entity 已 despawn，无需额外；父无关 |

同步实现：从数组 `CollectMonoBehaviours(handle)` 生成 `Vec<MonoBehaviourInstance>` 写 ECS。

### 2.3 读/恢复路径

`Load` / `SpawnGameObject`：

1. 默认仍走 `RestoreMonoBehaviours`（注册表 + 数组）——**保持现状，保证关 feature 正确**。
2. feature 开启时，若以后需要「只读 ECS 重建」，提供：

```rust
pub fn restore_monobehaviours_from_ecs(&mut self, handle: GameObjectHandle) -> bool
```

- 从 ECS `MonoBehaviourInstances` 读列表
- 用 `MonoBehaviourRegistry::global().create(type_name)`
- `DeserializeProps` + `SetEnabled`
- 写回数组 holder
- 成功返回 true；无组件或未注册类型则 false

**本阶段验收不要求 Load 自动改走 ECS**；先保证：  
**写通后 Save → Load → Collect 往返与 props 一致**，且 feature 下 ECS 上有 Instances。

### 2.4 测试清单

| 测试 | 位置 | 断言 |
|------|------|------|
| `test_b5_instances_on_ecs_after_add` | `world.rs` unit, feature | Add 后 ECS `MonoBehaviourInstances` 含 type_name/enabled/props |
| `test_b5_instances_match_collect` | 同上 | ECS 列表 == `CollectMonoBehaviours` |
| `test_b5_scene_roundtrip_props` | `serialization` 或 lifecycle feature | 已有 `test_monobehaviour_scene_roundtrip_via_registry` 在 feature 下仍绿；另加 feature 下 ECS Instances 与 load 后一致 |
| `test_b5_restore_from_ecs` | world unit, feature | 手动清数组 monos → `restore_monobehaviours_from_ecs` → count/props 恢复 |

### 2.5 兼容与风险

| 风险 | 缓解 |
|------|------|
| `MonoBehaviourTypes` 与 Instances 双份漂移 | 同步函数一次写两份，或 Instances 写完后更新 Types；文档写明 Instances 优先 |
| 未注册类型 | Restore 已 warn+skip；ECS 侧同样 skip，不 panic |
| 关 feature 行为变化 | 所有新写通仅 `cfg(feature = "unity-world-primary")` 或无条件写 ECS（现在 Name/Tag 已无条件写）——**建议与现有 `sync_*_to_ecs` 一致：有 entity 就写，不依赖 feature**，与 B5 测试一致 |
| 性能 | 每 Add/Restore 一次 O(n) clone props JSON；n 通常很小 |

### 2.6 文档

- `docs/migration-guide.md` P2.4 节：补 `MonoBehaviourInstances` 与 `restore_monobehaviours_from_ecs`
- roadmap P2.12 勾选

**提交建议**：  
`feat(engine-core): persist recoverable MonoBehaviour metadata on ECS under dual-write`

---

## Phase 3 — C1 `collect_system` 数据源（约 0.5–1 天）

### 现状

`engine-render/src/collect_system.rs`：

- 从 ECS 读 `DirectionalLight` / `PointLight` / `SpotLight`
- 位置来自 `engine_scene::transform::GlobalTransform` 的 Mat4 平移

### 目标（不改 Pass 内部）

光源位置优先可从 **core 镜像** 读取，缺省回退 `GlobalTransform`。

### 3.1 实现策略（推荐最小侵入）

在 `extract_position` 调用处增加 fallback 链：

1. `engine_core::transform::Transform`（或 identity_bridge 的 `TransformProxy`）→ 取 `Position` / `position`
2. 否则 `GlobalTransform` → Mat4 平移
3. 否则 `DEFAULT_POSITION`

需要：

- 确认 `engine-render` 是否已依赖 `engine-core`（通常 **没有**，避免环）。
- 若无依赖：用 **`TransformProxy`**（若 Proxy 在 engine-render 或 engine-ecs 侧），或定义 **本地轻量 `LightPose { position: [f32;3] }`** 由 bridge 写入。
- IdentityBridge 在 `sync_transforms` 时已有 `TransformProxy.position`；检查 `render_phase` 是否已把 Proxy 写到 light entity 上。若 light 与 GO 同一 entity，可直接读 Proxy。

### 3.2 落地步骤

1. 读 `identity_bridge` / `render_phase`：light entity 是否有 `TransformProxy`
2. 改 `collect_system`：`Proxy.position` → else `GlobalTransform`
3. 单测：只写 Proxy 时位置生效；只有 GlobalTransform 时回退；两者都无 → `[0,0,0]`
4. **禁止**改 lighting GPU pass / shader

### 3.3 验证

```bash
cargo test -p engine-render
cargo test -p engine-render --lib collect
# 若 render 依赖 core feature 则再跑 core feature 测
```

**提交建议**：  
`feat(engine-render): prefer TransformProxy position in light_collect_system`

---

## Phase 4 — 可选（B5+C1 绿后）

### D1 试验构建默认开 flag

- `engine-core/Cargo.toml` **不改 default**
- 增加 feature `unity-world-primary-exp` 或文档写明：

```toml
# 试验：引擎/编辑器本地构建
engine-core = { path = "...", features = ["unity-world-primary"] }
```

- 仅文档 + 可选 `Cargo.toml` 注释；**不要**改 default features

### D2 更多 MB 注册

- 清点编辑器/示例内脚本，对需要进 SceneData 的调用 `register_mono_behaviour::<T>()`
- 每个类型补 SerializeProps/DeserializeProps
- 验收：示例场景保存后重开脚本仍在

### D3 WASM / Android

继续延后；本迭代结束时在 roadmap「仍延后」保持勾选状态说明。

---

## 执行顺序与依赖

```text
Phase 1 收尾 ──► Phase 2 B5 ──► Phase 3 C1 ──► Phase 4 D1/D2
                     │
                     └── 不依赖 C1；可与 C1 并行若分两人
```

建议 **严格串行 Phase 1→2**，避免 examples/文档与 B5 同文件冲突。

---

## 总验证门禁（每 Phase 结束）

```bash
cargo fmt -p engine-core -p engine-editor -p engine-render -p engine-scene
cargo test -p engine-core --lib
cargo test -p engine-core --lib --features unity-world-primary
cargo test -p engine-core --test unity_lifecycle_tests
cargo test -p engine-core --test unity_lifecycle_tests --features unity-world-primary
cargo test -p engine-core --test identity_bridge_tests
cargo test -p engine-core --test editor_tests
cargo test -p engine-editor --test editor_tests
cargo test -p engine-render
cargo test -p engine-scene --lib
```

CI 已有 `Unity World Primary` job；push 后看该 job 是否绿。

---

## 工作量与风险总表

| Phase | 工作量 | 风险 | 回滚 |
|-------|--------|------|------|
| 1 收尾 | S | 低 | 单 commit revert |
| 2 B5 | M | 中（双份状态漂移） | feature 门控 + Instances 同步函数可关 |
| 3 C1 | S–M | 低（fallback 保 GlobalTransform） | 回退一行依赖顺序 |
| 4 D | S | 低 | 文档级 |

---

## 完成定义（本计划整体）

- [ ] `p2-deferred-storage` 分支已删
- [ ] roadmap 反映 B1–B7/C 文档已合 + B5/C1 状态
- [ ] examples 无裸 `GetTransformMut` 写路径（或明确 sync 后读）
- [ ] feature 下 ECS 有 `MonoBehaviourInstances`，roundtrip 测试绿
- [ ] `restore_monobehaviours_from_ecs` 有单测
- [ ] `light_collect_system` 优先 Proxy，回退 GlobalTransform，测试绿
- [ ] 默认构建数组权威语义未破坏（feature off 全测绿）
