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
│   │   └── examples/      # 示例程序（basic, platformer, dungeon 等）
│   ├── engine-ecs/        # 实体组件系统 (ECS)
│   ├── engine-render/     # wgpu 渲染系统
│   ├── engine-input/      # 输入管理系统
│   ├── engine-scene/      # 场景管理和节点
│   ├── engine-asset/      # 资源加载和管理
│   ├── engine-audio/      # 音频系统
│   ├── engine-ui/         # UI 系统 (ImGui)
│   ├── engine-window/     # 窗口管理 (winit)
│   ├── engine-math/       # 数学库 (glam)
│   ├── engine-framework/  # 框架层（游戏状态栈）
│   ├── engine-editor/     # 编辑器
│   ├── engine-physics/    # 物理引擎（3D + 2D）
│   ├── engine-network/    # 网络系统
│   ├── engine-jobs/       # 任务系统
│   ├── engine-script/     # 脚本系统（Lua/WASM）
│   └── engine-terrain/    # 地形系统
└── docs/                  # 教程和文档
```

## 快速开始

### 环境要求

- Rust 1.95.0 或更高版本
- Cargo 工具链

### 跨平台支持

RustEngine 支持以下平台：
- **Windows** - 原生支持
- **macOS** - Metal 渲染后端
- **Linux** - Wayland/X11 支持
- **Android** - NDK 交叉编译（实验性）
- **Web/WASM** - 浏览器运行 (实验性, 需 `--target wasm32-unknown-unknown`)

### 构建项目

```bash
# 开发模式
cargo build

# 发布模式（优化）
cargo build --release

# 使用 just 构建自动化（推荐安装 just）
just build          # 等同于 cargo build
just build-release  # 等同于 cargo build --release
just ci             # 运行完整 CI 检查（fmt + clippy + build + test）
```

### 运行示例

```bash
# 基本 ECS 示例
cargo run --example basic -p engine-core

# 跨平台示例（推荐）
cargo run --example cross_platform -p engine-core

# 输入处理示例
cargo run --example input_demo -p engine-core

# 完整功能演示
cargo run --example complete_demo -p engine-core

# 粒子系统演示
cargo run --example particle_demo -p engine-core
```

### 俄罗斯方块

完整的可玩俄罗斯方块游戏，基于 ECS 自动渲染集成构建。

```bash
cargo run --example tetris -p engine-core
```

### Web Demo (WASM)

浏览器中运行的 wgpu 渲染演示。

```bash
cd examples/web-demo
wasm-pack build --target web --release
python -m http.server 8080
# 浏览器访问 http://localhost:8080
```

**功能特性:**
- wgpu 渲染引擎
- 异步初始化
- 窗口事件处理
- 深色背景渲染

**操作说明：**

| 按键 | 功能 |
|------|------|
| ← → / A D | 左右移动 |
| ↑ / X | 顺时针旋转 |
| Z | 逆时针旋转 |
| ↓ / S | 软降（加速下落） |
| 空格 | 硬降（直接落底） |
| C | 暂存方块（Hold） |
| P | 暂停 / 恢复 |
| Esc | 游戏结束后重新开始 |

**游戏特性：**
- SRS 旋转系统 + 完整踢墙表
- 0.5 秒锁定延迟（落地后仍可操作）
- DAS/ARR 操作手感（长按自动重复）
- 7-Bag 随机器（标准 Tetris）
- Ghost piece（落点预览）
- Hold 暂存 + Next 预览队列
- Combo 连击加分
- 消行动画 + 暂停遮罩

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

## 开发路线图

当前项目有 **17 个 crate**，核心基础设施已就绪。按优先级划分为以下阶段：

### 阶段 0 — 核心基础 ✅ 已完成

| 模块 | 状态 | 说明 |
|------|------|------|
| **engine-math** | ✅ | Glam 重导出 + 扩展（`Vec2/3/4`, `Mat4`, `Quat`） |
| **engine-ecs** | ✅ | 完整的稀疏集 ECS：生成实体、组件注册表、类型擦除存储、查询、调度器 |
| **engine-core** | ✅ | 应用构建器、插件系统、时间管理、配置系统、日志、性能分析 |
| **engine-window** | ✅ | winit 0.30 窗口创建 |
| **engine-input** | ✅ | 键盘/鼠标状态追踪、操作映射、输入动作 |
| **engine-scene** | ✅ | 场景节点、父子层级、Transform/GlobalTransform 同步 |
| **engine-asset** | ✅ | 资源句柄（`Arc` 引用计数）、类型注册表、文件系统扫描器、图片/glTF/音频加载器 |
| **engine-framework** | ✅ | 游戏状态栈（push/pop/replace）、状态生命周期回调 |

### 阶段 1 — 渲染管线 🔨 当前重点

| 模块 | 状态 | 说明 |
|------|------|------|
| **Render Graph** | ✅ | 纹理/缓冲资源管理、编译（依赖图）、执行（逐 Pass 回调） |
| **Sprite Pipeline** | ✅ | WGSL 着色器（摄像机 Uniform + 纹理采样）、Alpha 混合管线 |
| **Sprite 组件 & 批处理** | ✅ | `Sprite` 组件、`SpriteBatch`（按纹理分组）、`collect_batches` |
| **摄像机系统** | ✅ | ECS 集成、优先级排序、视锥裁剪、多摄像机支持 |
| **Sprite 示例** | ✅ | `sprite_demo` — 渲染图集成验证 |
| **纹理加载 → Sprite** | ✅ | `EventChannel` + `TextureStore` 桥接（[texture_bridge.rs](crates/engine-render/src/texture_bridge.rs), 290 行） |
| **Sprite 批量绘制** | ✅ | `SpriteRenderer` + `PersistentBuffer` GPU 上传（[sprite_renderer.rs](crates/engine-render/src/sprite_renderer.rs), 133 行） |
| **2D 精灵动画** | ✅ | 精灵表、帧序列、循环模式、ECS 更新系统（[animation.rs](crates/engine-render/src/animation.rs), 482 行） |
| **2D 粒子系统** | ✅ | 粒子发射器、生命周期、颜色/大小渐变、ECS 模拟（[particle.rs](crates/engine-render/src/particle.rs), 829 行） |
| **Tilemap 支持** | ✅ | 图块集、瓦片图层、`SpriteDraw` 转换（[tilemap.rs](crates/engine-render/src/tilemap.rs), 557 行, 14 测试） |

### 阶段 2 — 框架 & 状态管理

| 模块 | 状态 | 说明 |
|------|------|------|
| **State Stack** | ✅ | 基础状态栈已完成 |
| **Menu / Title 状态** | ✅ | 标题画面、主菜单（新游戏/继续/设置/退出） |
| **Pause / GameOver 状态** | ✅ | 暂停菜单、游戏结束、状态生命周期 |
| **Save/Load** | ✅ | 槽位管理、JSON 序列化、分类键值存储 |

### 阶段 3 — 3D 渲染管线

| 模块 | 状态 | 说明 |
|------|------|------|
| **PBR 管线** | ✅ | Camera UBO + Model Push Constant + Blinn-Phong 光照 |
| **Mesh 渲染** | ✅ | 顶点/索引缓冲、Camera UBO 集成、深度测试 |
| **材质系统** | ✅ | PBR 材质组件（base_color, metallic, roughness, ao, emissive）、GPU Uniform |
| **光照** | ✅ | 方向光、点光源、聚光灯（ECS 组件 + 多光源 Shader） |
| **阴影** | ✅ | ShadowPass、级联阴影(CSM)、深度纹理、ShadowUniform |
| **模型加载** | ✅ | glTF/GLB 几何体加载（顶点、法线、UV、索引） |
| **环境贴图 / IBL** | ✅ | IBL 探针、GGX 重要性采样、BRDF LUT、IblUniform |
| **延迟渲染** | ✅ | G-Buffer（albedo/normal/position/material/depth）、几何/光照双 Pass |
| **3D 粒子** | ✅ | Particle3DSystem（Sphere/Cone/Box 发射器、颜色/大小曲线、爆发发射） |

### 阶段 4 — 物理引擎

| 模块 | 状态 | 说明 |
|------|------|------|
| **RigidBody** | ✅ | 刚体定义、力/冲量、速度、阻尼 |
| **Collider** | ✅ | 球体/盒体/胶囊/圆柱碰撞体、摩擦力/恢复系数/密度 |
| **碰撞检测** | ✅ | Sphere/AABB/OBB/Capsule 全组合碰撞（SAT 算法）、8 种碰撞对 |
| **物理世界 step** | ✅ | 子步模拟、Baumgarte 约束求解、摩擦力/恢复力（[world.rs](crates/engine-physics/src/world.rs), 377 行, 6 测试） |
| **接触点求解** | ✅ | ContactSolver（暖启动、累积冲量、Coulomb 摩擦约束） |
| **连续碰撞检测** | ✅ | Sphere-Sphere/Sphere-AABB 扫掠测试、CcdBody 组件 |
| **关节系统** | ✅ | 铰链、球窝、弹簧约束、JointSolver 弹簧力求解 |
| **ECS 集成** | ✅ | `PhysicsPlugin` 已注册 physics_step_system（[plugin.rs](crates/engine-physics/src/plugin.rs), 45 行） |

### 阶段 5 — 音频系统

| 模块 | 状态 | 说明 |
|------|------|------|
| **基础播放** | ✅ | `AudioManager` 基于 rodio，支持文件解码和播放 |
| **音量控制** | ✅ | 主音量、音效/音乐分轨音量、播放句柄控制 |
| **3D 空间音频** | ✅ | 距离衰减（3种模型）、多普勒效应、立体声声像定位 |
| **音频混音器** | ✅ | 命名总线、独立音量/静音、默认 6 总线（master/sfx/music/ambient/voice/ui） |
| **流式播放** | ✅ | AudioStream 探测、StreamingConfig 配置、格式检测 |

### 阶段 6 — 网络

| 模块 | 状态 | 说明 |
|------|------|------|
| **消息序列化** | ✅ | 握手、实体更新、玩家输入、聊天、断线重连 |
| **连接管理** | ✅ | 连接状态追踪、消息队列 |
| **底层 Socket I/O** | ✅ | UdpSocket、TcpListener、TcpConnection、PacketQueue |
| **服务器/客户端** | ✅ | GameServer/GameClient、会话管理、消息路由、ECS 集成 |
| **权威服务器** | ✅ | 权威模式、状态快照同步、输入转发、延迟补偿 |

### 阶段 7 — 编辑器 & 工具链

| 模块 | 状态 | 说明 |
|------|------|------|
| **编辑器 UI** | ✅ | 菜单栏、工具栏、层级面板、视口、检查器、状态栏 |
| **场景树** | ✅ | 增删改查、父子重排、级联删除、搜索 |
| **Gizmo** | ✅ | 平移/旋转/缩放手柄 |
| **Inspector** | ✅ | Transform/材质(PBR)/渲染/光照/物理属性面板 |
| **撤销/重做** | ✅ | 命令模式 |
| **编辑器摄像机** | ✅ | 轨道/平移/缩放 |
| **场景序列化** | ✅ | JSON 序列化/反序列化、ECS ↔ Scene 双向桥接 |
| **资源浏览器** | ✅ | 文件浏览、路径导航 |
| **节点图编辑器** | ✅ | NodeGraph 数据结构、拓扑排序、10+ 内置节点、导出 |
| **动画编辑器** | ✅ | 时间轴、关键帧编辑、贝塞尔曲线、预览、导入/导出 |
| **Prefab 系统** | ✅ | 可复用场景模板、实例化、覆盖、嵌套 |
| **可视化脚本** | ✅ | 蓝图组件、执行流节点、数据节点、ECS 集成 |
| **资产 .meta 文件** | ✅ | GUID 系统、导入设置、序列化 |
| **性能分析** | ✅ | tracing 插桩、热路径追踪 |

### 阶段 8 — 动画系统

| 模块 | 状态 | 说明 |
|------|------|------|
| **关键帧动画** | ✅ | Position/Rotation/Scale 关键帧、线性/步进/三次插值、AnimationPlayer |
| **骨骼动画** | ✅ | Joint/Skeleton/Skin、SkeletalAnimationPlayer、骨骼混合、矩阵调色板 |
| **状态机** | ✅ | AnimationStateMachine、条件过渡、混合过渡、参数系统 |
| **IK/FK** | ✅ | CCD/FABRIK 反向运动学、正向运动学、IK 链/目标 |

### 阶段 9 — 发布 & 生态

| 项目 | 状态 | 说明 |
|------|------|------|
| **CI/CD** | ✅ | GitHub Actions (fmt + clippy + build + test, Ubuntu/Windows 矩阵) |
| **跨平台构建** | ✅ | justfile、CI 矩阵 (Ubuntu/Windows/macOS)、条件编译 |
| **基准测试** | ✅ | Criterion (ECS 11 项 + Physics 6 项) |
| **文档** | ✅ | 7 篇教程 (docs/)、全 crate 文档注释 |
| **示例游戏** | ✅ | game_flow_demo — 完整游戏流程（菜单→游戏→暂停→结束）+ ECS/物理/渲染/音频演示 |
| **WASM/Web 支持** | ✅ | 浏览器运行 (实验性) — wgpu WebGPU/WebGL2、feature flags、cfg-gating |

## 跨平台开发

### 平台特定代码

使用条件编译隔离平台特定代码：

```rust
#[cfg(windows)]
fn platform_specific() {
    // Windows 特定代码
}

#[cfg(target_os = "macos")]
fn platform_specific() {
    // macOS 特定代码
}

#[cfg(target_os = "linux")]
fn platform_specific() {
    // Linux 特定代码
}
```

### 交叉编译

```bash
# 安装目标平台
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-linux-android
rustup target add wasm32-unknown-unknown

# 交叉编译
just build-linux    # Linux
just build-macos    # macOS
just build-android  # Android

# WASM/Web 构建
cargo build -p engine-render --target wasm32-unknown-unknown
cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib
```

### CI/CD

项目使用 GitHub Actions 进行跨平台 CI：
- Ubuntu (Linux)
- Windows
- macOS

每个平台都会运行完整的构建和测试套件。

## 文档

- [快速开始](docs/quick-start.md) — 从零开始创建项目
- [ECS 教程](docs/ecs-tutorial.md) — 学习实体组件系统
- [渲染管线](docs/rendering-pipeline.md) — 设置渲染
- [物理系统](docs/physics-system.md) — 添加物理模拟
- [音频系统](docs/audio-system.md) — 添加音频
- [编辑器指南](docs/editor-guide.md) — 使用编辑器
- [架构概述](docs/architecture.md) — 引擎架构设计
- [资产管线](docs/asset-pipeline.md) — 资产加载和管理
- [插件系统](docs/plugin-system.md) — 扩展引擎功能
- [迁移指南](docs/migration-guide.md) — 从 Unity/Godot/Bevy 迁移
- [Android 设置](docs/android-setup.md) — Android 目标构建指南
- [贡献指南](docs/contributing.md) — 如何贡献代码
- [WASM 状态](WASM_STATUS.md) — Web/WASM 支持详情

## 贡献

欢迎贡献代码！请确保提交前运行：
- `cargo fmt` - 格式化
- `cargo clippy` - 代码检查
- `cargo test` - 运行所有测试
- `just ci` - 运行完整 CI 检查（推荐）

## 许可证

Apache License 2.0
