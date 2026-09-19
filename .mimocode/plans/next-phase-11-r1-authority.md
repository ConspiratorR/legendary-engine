# 阶段 11 计划 — 双 World 存储权威收敛（R1 续）

**日期：** 2026-09（当前会话）  
**分支：** `main`（与 `origin/main` 同步；`p2-*` / `unity-lifecycle-refactor` 均已合入）  
**Feature：** `unity-world-primary` 继续默认 **off**；关 feature 时数组仍是存储权威  
**依据：** `docs/unity-alignment-roadmap.md`、`docs/migration-guide.md`（Still deferred）、`.mimocode/plans/p2-deferred-todo-handoff.md`

---

## 0. 现状结论

| 项 | 状态 |
|----|------|
| 阶段 0–9 高/中优先级 | ✅ 全部完成 |
| Unity 对齐 P1/P3/P4/编辑器主路径 | ✅ 已在 main |
| P2.a 身份桥 / 渲染桥 / Play / SceneData | ✅ |
| P2.4 dual-write 切片（B1–B7、C1–C4、D1–D2） | ✅ 写通 Transform/Hierarchy/Active/MBInstances/Layer |
| R1 首刀（seed / Instantiate/Layer / ECS 主写路径 pose 缓存） | ✅ 首刀落地 |
| **数组存储权威 → ECS** | 🔗 本阶段主线 |
| engine-scene Transform 完整替换（动画 keyframe） | 🔗 本阶段次要 |
| 默认打开 `unity-world-primary` | ❌ 本阶段不做 |
| VR/AR、Android NDK、WASM SceneRuntime 全量 | ❌ 明确延后 |
| 视口/生命周期分支未合入提交 | ✅ 无（`git log main..branch` 为空） |

**下一步唯一正确方向：** 在 feature 开启时把「权威」逐步从数组迁到内部 ECS，同时保证默认构建零语义变化。

---

## 1. 阶段主题

> **主题：** R1 续 — `unity-world-primary` 下 ECS 读优先 + 写权威切片 + 数组降为缓存  
> **副线：** 仓库收尾、engine-scene 动画依赖切割、编辑器验收  
> **非目标：** 默认开 flag、删 engine-scene、VR/AR、Android、WASM SceneRuntime 全量、远程 push

---

## 2. 任务拆分

### S1 — 仓库与文档收尾（S，先做）

| ID | 任务 | 验收 |
|----|------|------|
| S1.1 | 删除已合入本地分支：`docs-unity-exit-criteria`、`p2-4-*`、`p2-6-*`、`p2-deferred-storage`、`p2-viewport-unity-source`、`unity-lifecycle-refactor` | `git branch` 仅剩 `main` |
| S1.2 | 同步 `README.md` 开发路线图阶段 10 状态、`docs/unity-alignment-roadmap.md` P2.b、`PROJECT_SUMMARY.md` 仍延后条目 | 文档与代码一致 |
| S1.3 | 双开测试门禁全绿（见 §4） | 双 feature 测试通过 |
| S1.4 | 本地 commit | `chore(engine): phase-11 baseline — cleanup branches, refresh roadmap` |

### S2 — R1 读路径：feature 下 ECS 优先（M，核心切片 1）

**目标：** 有 linked entity 时，Unity World 公共读 API 优先返回 ECS 镜像；无 entity 或关 feature 时走数组（现状）。

| ID | 任务 | 范围 | 验收 |
|----|------|------|------|
| S2.1 | 盘点读 API | `GetTransform` / `GetName` / `GetTag` / `GetActive` / `GetParent` / `GetChildren` / `GetLayer` | 清单写入本计划附录 |
| S2.2 | 统一 dual-read 辅助 | `world.rs` 已有优先模式的收敛，避免每处手写 fallback | 新增私有 helper 或沿用现有优先读模式 |
| S2.3 | Hierarchy/Active/Name/Tag/Layer 读路径 | 已有优先实现的补齐测试 | feature 开：改 ECS → 读 API 立刻反映 |
| S2.4 | Transform 读路径 | `GetTransform` 在 feature 下优先 ECS `Transform` | 与 `sync_transform_from_ecs` 一致 |
| S2.5 | 测试 | 双开各一组：关=数组；开=ECS 优先 | `cargo test -p engine-core --lib` 及 `--features unity-world-primary` 绿 |

**不做：** 改默认 feature；删数组字段；改渲染 Pass。

**提交：** `feat(engine-core): prefer ECS mirrors on Unity World reads under unity-world-primary`

### S3 — R1 写路径：权威切片 + 数组缓存（L，核心切片 2）

**目标：** feature 开启时，写 API **权威写 ECS**，数组仅作 pose/cache 回填；关 feature 时权威仍在数组。

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| S3.1 | Transform 写权威 | 推广 `with_ecs_transform_mut`：SetPosition/Rotation/Scale、Transform 赋值 | feature 下改 API → ECS 变；`sync_*_to_array` 可选回填 |
| S3.2 | Hierarchy 写权威 | SetParent / 移除子节点：feature 下写 ECS Parent/Children，数组双写保持一致 | 现有 dual-write 测试扩展 |
| S3.3 | Active/Name/Tag/Layer 写权威 | SetActive / SetName / SetTag / SetLayer | feature 下读优先已覆盖；写侧同步权威 |
| S3.4 | MB 元数据写权威 | Add/Remove/Enable Mono：`MonoBehaviourInstances` 为镜像权威；实例 holder 仍在数组 | B5 测试保持；补「数组清空后 feature 下可 restore」 |
| S3.5 | Destroy 一致 | feature 下 despawn entity + 数组清理（已有）回归 | lifecycle 测试双开绿 |
| S3.6 | Instantiate / 场景 Load | Seed 策略：Load 后 `seed_all_ecs_from_array`；Instantiate 复制后 seed | SceneData 往返测试 |
| S3.7 | 编辑器/示例写路径 | 禁止新代码裸 `GetTransformMut`；示例统一写通 API | grep 无新增裸写；examples 构建过 |
| S3.8 | 文档契约 | `migration-guide.md` 写清：feature 开=「ECS 权威 + 数组缓存」；关=「数组权威」 | 文档与测试断言一致 |

**风险控制：**
1. 每切片一个 commit，可独立 revert  
2. 全程不改 workspace default features  
3. 每步跑 §4 门禁  
4. 禁止同时大改渲染 Pass / 着色器  

**提交序列建议：**  
- `feat(engine-core): R1e transform write authority under unity-world-primary`  
- `feat(engine-core): R1f hierarchy/identity write authority under feature`  
- `feat(engine-core): R1g monobehaviour metadata authority + load seed`  
- `docs(migration): document ECS-authority dual-read/write contract`

### S4 — R2：engine-scene 动画依赖切割（M，可与 S2 并行）

**目标：** animation keyframe / 编辑器动画面板的数据位姿优先 core/Proxy；engine-scene Transform 仅回退。**不删包。**

| ID | 任务 | 验收 | 状态 |
|----|------|------|------|
| S4.1 | 盘点 `engine_scene::transform` 在 animation / animation_editor / collect 的读写点 | 清单在本文件更新 | ✅ |
| S4.2 | keyframe 播放写位姿时：feature 或 Proxy 存在 → 写 core/Proxy；否则写 engine-scene | `engine_core::animation_apply::apply_clip_pose` → World；编辑器预览写回 World | ✅ |
| S4.3 | 编辑器动画面板读位姿：优先 World/Proxy | `animation_pose_from_world`；空轨不漂移 | ✅ |
| S4.4 | `light_collect` / `collect_system` 保持 Proxy 优先（C1 已做） | 回归测试绿 | ✅ |
| S4.5 | 模块 doc 标注 engine-scene Transform「动画回退，非权威」 | keyframe/transform/migration-guide | ✅ |

**Inventory（S4.1）**
- `engine-render::collect_system` — Proxy → GlobalTransform（保留回退）
- `engine-scene::keyframe` — 纯 clip 格式 + 采样数学
- `engine-scene::transform` — 场景图回退位姿
- `engine_core::animation_apply` — 采样写入 World（权威）
- `engine-editor` preview / `apply_node_transform_to_world` — 快照 + World 写通

**提交：** `refactor(anim): prefer Unity/Proxy pose; engine-scene Transform as fallback`

### S5 — 编辑器与端到端验收（S–M）

| ID | 任务 | 验收 | 状态 |
|----|------|------|------|
| S5.1 | 双模式跑编辑器：默认 + 本地试验 `features=["unity-world-primary"]` | **必须实际启动编辑器**验证 | ✅ 默认模式进程启动；feature-on 需改 editor 依赖，默认仍 off |
| S5.2 | Play 模式：加载场景 → Play → 停止 → **场景不脏** | World 在 stop 后从 snapshot 恢复 | ✅ `stop()` 恢复 World + 测试 |
| S5.3 | 保存/打开 `.runtime.json` 往返 | Material/脚本/Transform 一致 | ✅ 既有测试 + play/tick/stop 后重开 |
| S5.4 | 鼠标点击 DPI 偏移根因 | 读源码；能修则修 | ✅ 根因：拾取用 canvas 而非 `img_rect`（顶栏 32pt）；gizmo 固定 HUD 角；已修投影+img_rect；notes.md |
| S5.5 | `cargo clippy` / `fmt`（相关 crate） | 无新增警告 | ✅ fmt 已跑；clippy 仅既有 Unity API 命名警告 |

### S6 — 收尾（S）

| ID | 任务 |
|----|------|
| S6.1 | 更新本计划勾选状态 + roadmap / PROJECT_SUMMARY |
| S6.2 | 全量 §4 门禁 |
| S6.3 | 本地 commit；**不 push**（除非用户明确要求） |

---

## 3. 明确不做（本阶段）

- 默认打开 `unity-world-primary` 或改 workspace default features  
- 删除 `engine-scene` 包或 `Transform` 类型本身  
- VR/AR（OpenXR）、Android NDK 运行时  
- WASM 上 SceneRuntime 全量  
- 渲染 Pass / WGSL 内部重构  
- 未经用户要求 `git push`  
- 重复实现已合并的 B/C/D 切片  

---

## 4. 验证门禁（每个任务结束）

```bash
cargo fmt -p engine-core -p engine-editor -p engine-render -p engine-scene --check
cargo clippy -p engine-core -p engine-editor -- -D warnings  # 既有基线警告除外
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

**语义不变量（关 feature）：**  
数组权威读写、生命周期回调顺序、SceneData 往返、与合入前 main 行为一致。

**语义目标（开 feature）：**  
有 entity 时读 API 优先 ECS；写 API 权威在 ECS；数组为缓存；Load/Instantiate 后 seed 齐全。

---

## 5. 建议执行顺序

```text
S1 收尾（0.5d）
  → S2 读路径（1–2d） ──┬──→ S3 写权威切片（2–3d）→ S6 收尾
  → S4 动画依赖（1–2d） ─┘
                         └──→ S5 编辑器验收（穿插，至少在 S3 每大块后一次）
```

**推荐第一批落地：** S1 + S2 + S5.1（编辑器双模式冒烟）  
**本迭代成功标准：**  
1. feature 下读路径 ECS 优先且双开测试绿  
2. 至少完成 S3.1–S3.3 写权威切片  
3. 编辑器双模式可 Play / 存取场景  
4. 默认构建零回归  
5. 文档契约与 roadmap 同步  

---

## 6. 附录：R1 读/写 API 盘点（执行时勾选）

| API | 读 dual-read | 写权威（feature） | 状态 |
|-----|--------------|-------------------|------|
| GetTransform / Set*Transform | ✅ ECS 优先 | `with_ecs_transform_mut` 已有 | 读 ✅ / 写 S3 |
| GetName / SetName | ✅ ECS 优先 | 写通 ECS 镜像 | 读 ✅ |
| GetTag / SetTag | ✅ ECS 优先 | 写通 ECS 镜像 | 读 ✅ |
| GetActive / SetActive | ✅ ECS 优先 | dual-write 已有 | 读 ✅ |
| GetParent / GetChildren / SetParent | ✅ ECS 优先 | dual-write 已有 | 读 ✅ |
| GetLayer / SetLayer | ✅ ECS 优先 | R1b/c 写通 | 读 ✅ |
| MonoBehaviour add/enable/remove | B5 Instances | holder 仍数组 | S3.4 |
| Instantiate / Load / Destroy | seed + despawn | 回归 | S3.5–3.6 |

**S2 完成注记：** dual-read 生产代码已在 main；本切片补 `test_dual_read_storage_mode_contract_when_ecs_diverges` 双模态验收 + 文档契约。分支删除被会话 ref-store 策略阻止，遗留分支见 compose spec Report。

---

## 7. 用户偏好（执行约束）

- 不问问题，自主决策；选最安全、常见方案  
- 跟现有 `sync_*_to_ecs` / `seed_*` / dual-read 模式，不发明新架构  
- 每任务完成后 **自动本地 commit**（勿等用户说提交）  
- 编辑器相关必须实机验证  
- 计划文件放在项目 `.mimocode/plans/`，勿写入全局 memory 当计划  
