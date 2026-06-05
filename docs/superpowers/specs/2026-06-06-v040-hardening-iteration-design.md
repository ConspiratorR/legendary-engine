# v0.4.0 Hardening Iteration Design

**Date:** 2026-06-06
**Status:** Approved
**Scope:** 全面加固现有代码库，标记 v0.3.0/v0.4.0 版本
**Strategy:** 自底向上逐 crate 处理（方案 A）

---

## Overview

RustEngine 已完成全部 9 个开发阶段（14 crate），v0.3.0（2D 平台跳跃）和 v0.4.0（3D 地牢探索）demo 已实现但未打标签。代码审计显示：

- 物理引擎基本完整（仅关节约束和圆柱碰撞有小缺口）
- 网络系统完全实现（非桩代码）
- 脚本系统（Lua/WASM）完全实现
- 全部测试通过，零 clippy 警告
- 25 处 `#[allow(dead_code)]` 分散在各处

本迭代目标：清理技术债务、补全测试和文档、修复物理小缺口、打磨 demo、打版本标签。

---

## 执行顺序

按依赖层级从底到顶逐 crate 处理：

| 批次 | Crate | 层级 | 重点 |
|------|-------|------|------|
| 1 | engine-math | 0 | 边界测试、文档 |
| 2 | engine-jobs | 0 | 并发测试、调度器文档 |
| 3 | engine-window | 0 | 错误处理、平台兼容 |
| 4 | engine-audio | 1 | 播放测试、混音器文档 |
| 5 | engine-asset | 1 | 加载管线文档 |
| 6 | engine-ecs | 1 | 查询迭代性能、内存布局 |
| 7 | engine-scene | 2 | 层级同步测试、序列化文档 |
| 8 | engine-input | 2 | API 一致性、操作映射文档 |
| 9 | engine-render | 3 | GPU 资源生命周期、渲染图文档 |
| 10 | engine-core | 4 | 插件系统文档、耦合审计 |
| 11 | engine-framework | 5 | 状态栈测试、生命周期文档 |
| 12 | engine-physics | 5 | 关节约束修复 + 圆柱碰撞 |
| 13 | engine-network | 5 | 连接测试、协议文档 |
| 14 | engine-script | 5 | WASM/Lua 集成测试、沙箱安全 |
| 15 | engine-ui | 5 | 组件 API 文档 |
| 16 | engine-terrain | 5 | 测试覆盖从 0 补起 |
| 17 | engine-editor | 6 | UI 组件测试、工作流文档 |
| 18 | Demo 打磨 | — | platformer_demo + dungeon_demo |
| 19 | 版本标记 | — | git tag v0.3.0 + v0.4.0 |

---

## Per-Crate Checklist

每个 crate 统一执行：

1. **Dead Code 清理** — 审计 `#[allow(dead_code)]`，接入运行时或移除
2. **错误处理统一** — 确保有 `error.rs` + `XxxError`（thiserror），消除生产代码 `unwrap()`/`expect()`
3. **测试补全** — 核心逻辑 >80% 覆盖率，边界情况测试
4. **文档补全** — crate 级 `//!` 文档、所有公开函数 `///` 文档
5. **API 一致性** — 命名规范、公开接口审查
6. **Clippy 清洁** — `cargo clippy -p <crate>` 零警告

---

## 物理引擎专项修复

### 关节约束求解（`joint.rs`）

当前 `JointSolver.solve_springs()` 只处理弹簧力，Hinge/BallSocket 位置约束未实现。

修复：
- Hinge：添加角度约束（限制旋转轴 + 角度范围）
- BallSocket：添加距离约束（限制最大位移）
- 新增 `solve_constraints()` 方法，与 `solve_springs()` 并行调用

### 圆柱碰撞（`collider.rs`）

当前 Cylinder 退化为包围球近似。

修复：
- Cylinder-Sphere：最近点投影算法
- Cylinder-AABB：分离轴算法
- Cylinder-OBB/Capsule：保留包围球近似（复杂度高、收益低）

---

## Demo 打磨

### platformer_demo
- 补充内联注释说明引擎 API 使用
- Windows 零警告零崩溃运行

### dungeon_demo
- 补充内联注释
- 检查 3D 渲染管线完整性（阴影、光照、材质）

---

## 版本标记

| Tag | 内容 | 时机 |
|-----|------|------|
| v0.3.0 | platformer_demo + physics_2d + 质量迭代 | 加固开始前补打 |
| v0.4.0 | dungeon_demo + 全面加固 | 本轮完成后打 |

---

## 成功标准

| 标准 | 指标 |
|------|------|
| 全部测试通过 | `cargo test --all` 零失败 |
| 零 clippy 警告 | `cargo clippy --all` 干净 |
| 无生产代码 unwrap | `unwrap()`/`expect()` 仅在测试/基准 |
| Dead code 清零 | `#[allow(dead_code)]` 为 0 或有文档说明 |
| 测试覆盖 | 核心逻辑 >80% |
| 文档完整 | 每个 crate 有 `//!`，公开函数有 `///` |
| 物理缺口修复 | 关节约束 + 圆柱碰撞基础算法 |
| Demo 可运行 | 两个 demo 零警告零崩溃 |
| 版本标记 | v0.3.0 + v0.4.0 tag 存在 |

---

## Out of Scope

- 新功能或新子系统
- 大版本升级
- 破坏性 API 变更
- 编辑器 UI 重新设计
- 性能优化（除非发现明显瓶颈）
- 新 crate 创建
