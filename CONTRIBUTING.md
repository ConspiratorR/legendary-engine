# RustEngine 贡献指南

## 开发环境设置

### 必需工具
- Rust 1.95.0 或更高版本
- Git
- Cargo

### 推荐 IDE
- VS Code + rust-analyzer
- CLion + Rust 插件
- Zed 编辑器

## 项目结构

```
RustEngine/
├── crates/                    # 所有引擎模块 (17个)
│   ├── engine-core/          # 核心系统和入口点
│   ├── engine-ecs/           # 实体组件系统
│   ├── engine-render/        # 渲染系统 (wgpu)
│   ├── engine-input/         # 输入处理
│   ├── engine-scene/         # 场景管理
│   ├── engine-asset/         # 资源加载
│   ├── engine-audio/         # 音频系统
│   ├── engine-ui/            # UI 系统 (egui)
│   ├── engine-window/        # 窗口管理 (winit)
│   ├── engine-math/          # 数学库 (glam)
│   ├── engine-framework/     # 游戏状态栈
│   ├── engine-editor/        # 编辑器
│   ├── engine-physics/       # 物理引擎
│   ├── engine-network/       # 网络系统
│   ├── engine-jobs/          # 任务系统
│   ├── engine-script/        # 脚本系统 (Lua/WASM)
│   └── engine-terrain/       # 地形系统
├── examples/                 # 示例代码
├── docs/                     # 文档
└── .github/workflows/        # CI/CD 配置
```

## 开发流程

### 1. 克隆仓库
```bash
git clone https://github.com/ConspiratorR/RustEngine.git
cd RustEngine
```

### 2. 创建功能分支
```bash
git checkout -b feature/your-feature-name
```

### 3. 开发
```bash
# 开发模式构建
cargo build

# 运行测试
cargo test

# 代码检查
cargo clippy

# 格式化
cargo fmt
```

### 4. 测试
```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test -p engine-core

# 运行示例
cargo run --example basic -p engine-core
```

### 5. 提交
```bash
# 添加更改
git add .

# 提交（使用语义化提交信息）
git commit -m "feat: 添加新功能"

# 推送
git push origin feature/your-feature-name
```

## 代码规范

### Rust 编码规范
1. 运行 `cargo fmt` 格式化代码
2. 运行 `cargo clippy` 确保通过所有检查
3. 添加文档注释（`///`）
4. 为公共 API 编写示例

### 提交信息规范
遵循 Conventional Commits 规范：

```
type(scope): description

feat: 新功能
fix: 错误修复
docs: 文档更改
style: 代码格式（不影响功能）
refactor: 代码重构
perf: 性能优化
test: 测试更改
chore: 构建过程或辅助工具的变动
```

示例：
```
feat(engine-ecs): 添加查询系统

- 添加 QueryPair 结构
- 实现可变的查询迭代器
- 添加测试用例

Closes #123
```

## 模块开发指南

### 添加新模块
1. 在 `crates/` 下创建新目录
2. 初始化 `Cargo.toml`
3. 在 workspace `Cargo.toml` 中添加成员
4. 实现模块

### 添加新功能到现有模块
1. 在对应模块中实现功能
2. 添加单元测试
3. 在示例中演示用法
4. 更新文档

## 测试策略

### 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        assert_eq!(feature(), expected_value);
    }
}
```

### 集成测试
在 `tests/` 目录中创建集成测试文件。

## 文档

### 为 crate 编写文档
```rust
//! 我的 crate 描述
//!
//! # 示例
//!
//! ```
//! use my_crate::feature;
//! assert_eq!(feature(), expected);
//! ```
```

### 为函数编写文档
```rust
/// 函数描述
///
/// # 参数
///
/// * `input` - 输入参数描述
///
/// # 返回值
///
/// 返回值描述
///
/// # 示例
///
/// ```
/// assert_eq!(my_function(1), 2);
/// ```
pub fn my_function(input: i32) -> i32 {
    input + 1
}
```

## 版本管理

### 版本号格式
遵循语义化版本 (SemVer)：
- MAJOR.MINOR.PATCH
- 例如: 0.1.0

### 发布流程
1. 更新版本号
2. 更新 CHANGELOG
3. 创建 Git tag
4. 提交并推送

## 获取帮助

- 查看 [README.md](../README.md) 了解项目概述
- 查看 [docs/](../docs/) 目录中的文档
- 查看 examples/ 目录中的示例代码
- 提交 Issue 报告问题

## 许可证

本项目使用 Apache License 2.0 - 详见 [LICENSE](../LICENSE) 文件
