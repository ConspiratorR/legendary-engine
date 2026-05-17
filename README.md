# legendary-engine

基于 Rust 编写的高性能游戏开发引擎

## 特性

- 🦀 **纯 Rust 实现** - 内存安全，高性能
- 📦 **模块化架构** - 清晰分离的 crate 设计
- 🎮 **ECS (Entity Component System)** - 灵活高效的实体组件系统
- 🎨 **wgpu 渲染** - 现代跨平台渲染后端
- ⌨️ **输入系统** - 完整的键盘、鼠标和操作映射
- 🎬 **场景管理** - 场景图和层级系统
- 🖼️ **UI系统** - 内置 ImGui 集成
- 🧩 **插件系统** - 灵活的功能扩展

## 项目结构

```
legendary-engine/
├── crates/
│   ├── engine-core/       # 核心引擎，应用系统和插件
│   ├── engine-ecs/        # 实体组件系统 (ECS)
│   ├── engine-render/     # wgpu 渲染系统
│   ├── engine-input/      # 输入管理系统
│   ├── engine-scene/      # 场景管理和节点
│   ├── engine-asset/      # 资源加载和管理
│   ├── engine-audio/      # 音频系统
│   ├── engine-ui/         # UI 系统 (ImGui)
│   ├── engine-window/     # 窗口管理 (winit)
│   ├── engine-math/       # 数学库 (glam)
│   ├── engine-framework/  # 框架层
│   └── engine-editor/     # 编辑器
└── examples/
    └── ...                # 示例程序
```

## 快速开始

### 环境要求

- Rust 1.95.0 或更高版本
- Cargo 工具链

### 构建项目

```bash
# 开发模式
cargo build

# 发布模式（优化）
cargo build --release
```

### 运行示例

```bash
# 基本 ECS 示例
cargo run --example basic -p engine-core

# 输入处理示例
cargo run --example input_demo -p engine-core

# 完整功能演示
cargo run --example complete_demo -p engine-core

# 粒子系统演示
cargo run --example particle_demo -p engine-core
```

### 运行测试

```bash
# 运行所有测试
cargo test --all

# 查看详细测试信息
cargo test -- --nocapture
```

### 代码质量

```bash
# 格式化代码
cargo fmt

# Linting检查
cargo clippy
```

## 使用指南

### 创建一个基本应用

```rust
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_core::plugins::CorePlugins;
use engine_core::engine::{Engine, run_default};

// 创建插件
struct MyGamePlugin;
impl Plugin for MyGamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        // 添加系统、资源等
        // app.add_system(my_system());
    }
}

fn main() {
    let mut app_builder = Engine::new();
    app_builder.add_plugin(CorePlugins);
    app_builder.add_plugin(MyGamePlugin);
    
    // 运行默认配置（窗口 + 渲染）
    // run_default(app_builder);
}
```

### ECS 使用

```rust
use engine_ecs::world::World;
use engine_ecs::query::QueryPair;
use engine_math::Vec3;

// 定义组件
struct Position(Vec3);
struct Velocity(Vec3);

// 创建世界
let mut world = World::new();

// 创建实体
let entity = world.spawn();
world.add_component(entity, Position(Vec3::new(0.0, 0.0, 0.0)));
world.add_component(entity, Velocity(Vec3::new(1.0, 2.0, 0.0)));

// 查询组件
let query = QueryPair::<Position, Velocity>::new();
for (pos, vel) in query.iter(&world) {
    println!("Pos: {:?}, Vel: {:?}", pos.0, vel.0);
}

// 可变查询更新
for (pos, vel) in query.iter_mut(&mut world) {
    pos.0 += vel.0 * 0.016;
}
```

### 时间管理

```rust
use engine_core::time::Time;

// Time 资源会自动被 CorePlugins 管理
fn my_system(app: &App) {
    if let Some(time) = app.resources().get::<Time>() {
        let delta = time.delta_seconds();
        let elapsed = time.elapsed_seconds();
        let fps = time.fps();
        
        println!("FPS: {:.1}", fps);
    }
}
```

## 开发计划

- [x] 基础 ECS 系统
- [x] 插件系统
- [x] 输入系统
- [x] 时间管理
- [ ] 完整的 2D 渲染
- [ ] 物理引擎集成
- [ ] 音频播放
- [ ] 网络功能
- [ ] 完整编辑器

## 贡献

欢迎贡献代码！请确保提交前运行：
- `cargo fmt` - 格式化
- `cargo clippy` - 代码检查
- `cargo test` - 运行所有测试

## 许可证

MIT License
