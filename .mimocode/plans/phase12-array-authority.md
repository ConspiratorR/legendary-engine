# 阶段 12 执行计划指针 — 数组权威迁 ECS（R1-full）

**Feature 文档（唯一权威）：** `docs/compose/spec/phase12-array-authority.md`  
**分支：** `phase12-array-authority`（当前主 worktree，用户选择不建独立 worktree）  
**Feature flag：** `unity-world-primary` 默认仍 **off**  
**基线：** 阶段 11 S6 已在本地 main（dual-read + 写权威切片 + 动画位姿 + 编辑器验收）

## 本阶段做什么

在 `features = ["unity-world-primary"]` 下把「数组存储权威」迁到内部 ECS：Identity / Hierarchy / Pose / Scene I/O / MB 元数据在 feature on 时以 ECS 为权威，数组降为可刷新 cache。**不**把 `Box<dyn MonoBehaviour>` 迁入 ECS，**不**默认开 flag。

## 任务（与 spec Tasks 对齐）

| ID | 摘要 | 状态 |
|----|------|------|
| T1 | 权威契约落盘（migration-guide + world 注释） | 待做 |
| T2 | 层级写权威（SetParent / Destroy / Instantiate） | 待做 |
| T3 | Identity 写权威硬化 + 分歧测试 | 待做 |
| T4 | Pose / 层级数学 cache 刷新契约 | 待做 |
| T5 | Scene I/O 双模态（Load seed → ECS 权威；Save 前刷新） | 待做 |
| T6 | MB 元数据权威（Instances）；holder 仍数组 | 待做 |
| T7 | 内部路径审计 | 待做 |
| T8 | 文档 + 双模态门禁 + ready 评估（不 flip） | 待做 |

## 门禁

见 spec「验证门禁」；feature off 语义不变量必须保持。

## 明确不做

默认开 flag、删 engine-scene、VR/AR、Android NDK、WASM SceneRuntime 全量、渲染 Pass 重构、未经要求 push/merge。

## 恢复说明

若会话压缩后 Compose Next 指令丢失，重新加载 `compose-next` skill，再读本文件与 `docs/compose/spec/phase12-array-authority.md`。
