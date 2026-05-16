# RustEngine (legendary-engine) 设计文档

> **项目**: Rust 游戏引擎
> **作者**: ConspiratorR
> **许可证**: MIT
> **日期**: 2026-05-17

## 1. 目标

基于 Rust 2024 构建一个 2D + 3D 兼顾的游戏引擎，采用混合架构（场景树管理 + ECS 底层存储），渲染后端使用 wgpu。一期目标是可运行的最小引擎骨架，能渲染精灵和静态网格、处理输入、播放简单音效。

## 2. 技术栈

| 层 | 技术 |
|---|---|
| 语言 | Rust 2024 edition, toolchain 1.95.0 |
| 渲染 | wgpu (Vulkan/Metal/DX12/WebGPU) |
| 窗口 | winit |
| 数学 | glam (re-export, 提供 extension trait) |
| 音频 | rodio |
| 错误处理 | anyhow / thiserror |
| 格式 | glTF 2.0, image crate, WAV/OGG |

## 3. Workspace 与 Crate 结构

```
RustEngine/
├── Cargo.toml                     # workspace root
├── crates/
│   ├── engine-core/               # Plugin trait, AppBuilder, Engine 高层 API
│   ├── engine-ecs/                # 轻量 ECS: World, SparseSet, Query, System, Schedule
│   ├── engine-scene/              # SceneNode 树, Hierarchy, Transform, SceneManager
│   ├── engine-render/             # wgpu 渲染抽象层（2D batch + 3D PBR）
│   ├── engine-asset/              # 资源加载、缓存、引用计数 Handle
│   ├── engine-input/              # 输入抽象（键盘/鼠标/游戏手柄）
│   ├── engine-audio/              # 音频播放（rodio）
│   ├── engine-math/               # glam re-export + extension trait
│   └── engine-window/             # winit 窗口管理 + 事件循环
├── examples/
│   └── basic/                     # 最小可运行示例
├── .github/workflows/             # CI
└── docs/
    └── superpowers/
        ├── specs/                 # 设计文档
        └── plans/                 # 实现计划
```

### 依赖关系

```
game-app
    └── engine-core
         ├── engine-ecs
         ├── engine-scene  (depends on ecs)
         ├── engine-render (depends on window)
         ├── engine-input  (depends on window)
         ├── engine-asset
         ├── engine-audio
         └── engine-math
```

`engine-core` 聚合所有模块，对外暴露 `EngineBuilder` / `Engine` API。用户只需依赖 `engine-core`。

## 4. 核心架构

### 4.1 Plugin 系统

```
engine-core/src/
├── lib.rs
├── app.rs              — AppBuilder + App
├── plugin.rs           — Plugin trait
├── resource.rs         — Res<T> 资源存储（ECS World 扩展）
└── engine.rs           — Engine: 聚合所有模块的高层 API
```

```rust
pub trait Plugin {
    fn build(&self, app: &mut AppBuilder);
}

pub struct AppBuilder {
    world: World,
    schedule: Schedule,
}

impl AppBuilder {
    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self;
    pub fn run(self);
}
```

`Engine` 是预配置的 Plugin 集合：
```rust
pub struct Engine;

impl Engine {
    pub fn new() -> AppBuilder { /* 注册默认插件 */ }
}
```

### 4.2 ECS 核心

```
engine-ecs/src/
├── lib.rs
├── world.rs         — World: Component SparseSet + Entity 分配器
├── component.rs     — Component trait + SparseSet 存储
├── entity.rs        — Entity 句柄 (64-bit: generation + index)
├── query.rs         — Query<C1, C2> 迭代器
├── system.rs        — System trait
└── schedule.rs      — Schedule: System 有序执行
```

- **Entity**: 64-bit `(generation:24, index:40)`，复用安全
- **SparseSet**: 每个 Component 类型一个 `Vec<Option<T>>` + `Vec<Entity>`，cache-friendly
- **Query**: 编译期 `Query<(&Pos, &Vel)>`，`World::borrow()` 时检查别名规则
- **Schedule**: 一期手动有序执行，无并行调度

### 4.3 Scene 层

```
engine-scene/src/
├── lib.rs
├── node.rs           — SceneNode: Entity 的句柄，维护父子关系
├── hierarchy.rs      — Hierarchy 组件 (ECS Component)
├── transform.rs      — Transform + GlobalTransform 组件
└── scene_manager.rs  — SceneManager: root node, frame lifecycle
```

- `SceneNode` 是 `Copy` 的 `Entity` 句柄，数据在 ECS `World` 中
- 父子关系通过 `Hierarchy` Component 存储
- `GlobalTransform` 通过脏标记传播，避免全量树遍历
- 用户操作 `SceneNode API`，不直接接触 ECS

示例：
```rust
fn setup(scene: &mut SceneManager) {
    let camera = scene.add_node("camera")
        .with_transform(Transform::from_xyz(0.0, 5.0, 10.0));
    let player = scene.add_node("player")
        .with_child(camera);
    player.add_component(PlayerController::new());
}
```

## 5. 渲染层

```
engine-render/src/
├── lib.rs
├── renderer.rs        — Renderer: wgpu 设备/队列/swapchain 管理
├── pipeline/
│   ├── mod.rs
│   ├── sprite.rs      — 2D 精灵批处理渲染
│   ├── pbr.rs         — 3D 渲染（简单漫反射 + 方向光阴影）
│   └── skybox.rs      — 天空盒
├── resource/
│   ├── mod.rs
│   ├── mesh.rs        — Mesh 缓冲区管理
│   ├── texture.rs     — 纹理上传 / GPU 缓存
│   └── material.rs    — Material 绑定组
└── view.rs            — Camera, Viewport, 渲染编排
```

- **Camera 驱动渲染**：Camera Component + MainView 资源
- **2D 批处理**：Sprites 合并为动态 vertex buffer，按纹理排序减少 state change
- **3D 正向渲染**：简单 PBR（金属/粗糙度），方向光 + 4 点光源 + 简单阴影
- **固定管线**：Shadow → Skybox → Opaque → Transparent → UI

### 一期渲染特性

- [x] 窗口创建 + Swapchain resize
- [x] 2D Sprite 批处理渲染
- [x] 3D Static Mesh 渲染（简单漫反射）
- [x] 简单方向光阴影（shadow map）
- [x] Camera 控制（正交/透视）
- [ ] PBR 材质（二期）
- [ ] 后处理（二期）
- [ ] 粒子系统（二期）
- [ ] 骨骼动画（二期）
- [ ] UI 渲染（二期）

## 6. Asset 系统

```
engine-asset/src/
├── lib.rs
├── asset.rs           — Asset trait + Handle<T> 句柄
├── loader.rs          — Loader trait
├── registry.rs        — Registry: 路径→Handle 映射
└── format/
    ├── mod.rs
    ├── gltf.rs        — glTF 2.0 加载
    ├── image.rs       — 图片加载
    └── audio.rs       — WAV/OGG 加载
```

- `Handle<T>`: `Arc<AssetInner<T>>`，引用计数自动回收
- 同步加载为主，异步和热重载一期不做

## 7. Input 系统

```
engine-input/src/
├── lib.rs
├── input_manager.rs   — 统一输入状态聚合
├── keyboard.rs        — KeyState 查询
├── mouse.rs           — 位置/Delta/按钮
└── gamepad.rs         — 游戏手柄（一期可选）
```

- 帧前刷新状态，帧中轮询
- 裸输入查询，无 Action/Axis 映射系统

```rust
fn player_system(input: Res<InputManager>, query: Query<&mut Transform, With<Player>>) {
    if input.key_down(KeyCode::W) { /* 前进 */ }
}
```

## 8. Audio 系统

```
engine-audio/src/
├── lib.rs
└── audio_manager.rs   — 2D 音效播放
```

- 基于 `rodio`，一期只做 2D 音效

## 9. Window 管理

```
engine-window/src/
├── lib.rs
└── window.rs          — winit 窗口 + 事件循环
```

- `WindowConfig` 可配置标题/大小/全屏/VSync
- 事件循环在 `App::run()` 中驱动

## 10. 一期不做清单

- 异步 Asset 加载
- 热重载
- 粒子系统
- 骨骼动画
- 后处理（Bloom/Tonemapping）
- 物理引擎
- UI 系统
- 网络
- 编辑器
- Action/Axis 输入映射
- 3D 空间音频

## 11. 风格约束

- 遵循 `cargo fmt` 格式
- 无 `unsafe`（除非不可避免且有文档说明）
- 使用 `anyhow` / `thiserror` 进行错误处理
- TDD 开发流程
- 频繁 commit
