# v0.5.0 设计规格：ECS 自动渲染集成

**日期:** 2026-06-06
**状态:** 待审批
**目标:** 让 ECS 中的 Sprite/MeshRenderer/Camera/Light 组件自动在窗口中渲染，无需手动调用 renderer

---

## 问题陈述

当前引擎的所有 demo 都在命令行中以 ASCII 方式运行（platformer_demo、dungeon_demo）。
虽然 `sprite_demo` 和 `deferred_demo` 能打开窗口渲染，但它们绕过了 ECS，手动管理 renderer。
开发者添加 Sprite 组件后，必须手动收集组件、手动调用 `renderer.render_frame()`，无法开箱即用。

**目标状态：** 开发者只需往 ECS 添加组件，引擎自动在窗口中渲染。

---

## 架构设计

### 核心组件：RenderPlugin

新增 `engine-render/src/plugin.rs`，提供 `RenderPlugin2D` 和 `RenderPlugin3D`。

```
每帧执行流程：
┌─────────────────────────────────────────────┐
│ app.run()                                   │
│  ├─ pre_update_hooks (Time, Profiler 等)    │
│  ├─ ECS systems (用户游戏逻辑)              │
│  └─ post_update_hooks                       │
├─────────────────────────────────────────────┤
│ 渲染阶段 (RenderPlugin post-render hook)    │
│  ├─ camera_sort_system → CameraStack        │
│  ├─ sprite_collect_system → SpriteList      │
│  ├─ light_collect_system → LightingUniform  │
│  ├─ mesh_collect_system → InstanceBatch[]   │
│  └─ renderer.render_frame() / render_3d()   │
├─────────────────────────────────────────────┤
│ 呈现到窗口                                  │
└─────────────────────────────────────────────┘
```

### 数据流

```
ECS World
  ├─ Camera 组件 ──────→ camera_sort_system → CameraStack 资源
  ├─ Sprite 组件 ──────→ sprite_collect_system → CollectedSprites 资源
  ├─ Transform 组件 ───→ (被上述系统读取，计算世界矩阵)
  ├─ MeshRenderer 组件 ─→ mesh_collect_system → InstanceBatch[]
  ├─ PbrMaterial 组件 ──→ MaterialStore (GPU 资源)
  ├─ DirectionalLight ──→ light_collect_system → LightingUniform 资源
  └─ PointLight/SpotLight → (同上)
                                    │
                                    ▼
                            Renderer
                              ├─ render_frame() — 2D 精灵路径
                              └─ render_frame_3d() — 3D 延迟渲染路径
```

---

## 详细设计

### 1. RenderPlugin2D（`engine-render/src/plugin.rs`）

```rust
pub struct RenderPlugin2D {
    pub window: Arc<winit::window::Window>,
}

impl Plugin for RenderPlugin2D {
    fn build(&self, app: &mut AppBuilder) {
        // 1. 创建 Renderer 并插入为资源
        let renderer = Renderer::new(self.window.clone()).unwrap();
        let device = renderer.device.clone();
        let queue = renderer.queue.clone();
        app.insert_resource(renderer);
        app.insert_resource(device);
        app.insert_resource(queue);

        // 2. 创建 TextureBridge 并插入为资源
        let texture_layout = SpritePipeline::create_texture_layout(&device);
        let bridge = TextureBridge::new(&device, &queue, texture_layout);
        app.insert_resource(bridge);

        // 3. 创建 Asset Registry 并插入为资源
        app.insert_resource(engine_asset::registry::Registry::new());

        // 4. 注册 post-render hook 执行渲染
        app.add_post_render_hook(Box::new(|app| {
            render_2d_frame(app);
        }));
    }
}
```

### 2. RenderPlugin3D（`engine-render/src/plugin.rs`）

```rust
pub struct RenderPlugin3D {
    pub window: Arc<winit::window::Window>,
}

impl Plugin for RenderPlugin3D {
    fn build(&self, app: &mut AppBuilder) {
        // 类似 RenderPlugin2D，但注册 3D 渲染路径
        // 包括 MeshStore、MaterialStore 等资源
    }
}
```

### 3. sprite_collect_system

新增 `engine-render/src/collect_system.rs` 中的精灵收集：

```rust
/// 收集 ECS 中所有 Sprite + Transform 组件，生成渲染用的 Sprite 列表
pub fn sprite_collect_system(world: &mut World) {
    let mut sprites: Vec<CollectedSprite> = Vec::new();

    let entities = world.component_entities::<Sprite>();
    for idx in entities {
        if let Some(sprite) = world.get_by_index::<Sprite>(idx) {
            let transform = world
                .get_by_index::<Transform>(idx)
                .map(|t| t.compute_matrix())
                .unwrap_or(Mat4::IDENTITY);

            sprites.push(CollectedSprite {
                sprite: sprite.clone(),
                world_transform: transform,
            });
        }
    }

    world.insert_resource(CollectedSprites(sprites));
}
```

### 4. render_2d_frame 函数

```rust
fn render_2d_frame(app: &mut App) {
    let Some(renderer) = app.renderer_mut() else { return };

    // 1. 收集相机
    let cameras: Vec<&Camera> = ...; // 从 ECS 读取 Camera 组件

    // 2. 收集精灵
    let sprites: Vec<crate::sprite::Sprite> = ...; // 从 CollectedSprites 资源转换

    // 3. 获取 TextureBridge 和 Registry
    let bridge = app.world.get_resource_mut::<TextureBridge>().unwrap();
    let registry = app.world.get_resource::<Registry>().unwrap();

    // 4. 渲染
    let _ = renderer.render_frame(&cameras, &sprites, bridge, registry);
}
```

### 5. 修改 run_default()

`engine-core/src/engine.rs` 中的 `run_default()` 改为使用 RenderPlugin：

```rust
pub fn run_default(app_builder: AppBuilder) -> Result<(), EngineError> {
    let event_loop = EventLoop::new()?;
    let window = Arc::new(create_window(&WindowConfig::default(), &event_loop)?);

    // 自动添加 RenderPlugin2D
    app_builder.add_plugin(RenderPlugin2D { window: window.clone() });

    let mut app = app_builder.build();

    // 事件循环（保持现有结构，但移除手动渲染代码）
    event_loop.run(move |event, elwt| {
        // ... 窗口事件处理（现有代码不变）
        if let Event::AboutToWait = event {
            app.run(); // 包含渲染（通过 post-render hook）
        }
    })?;
    Ok(())
}
```

### 6. Camera 与 Transform 集成

Camera 组件需要与 Transform 关联以确定观察位置。

**方案 A（推荐）：** Camera 自带 view 矩阵，用户手动设置
- 保持现有 Camera 设计不变
- 3D 游戏中，用户在系统中根据 Transform 更新 Camera.view
- 2D 游戏中，Camera.view 通常为 Identity 或简单平移

**方案 B：** 自动从 Transform 计算 Camera.view
- 需要 camera_system 读取 Transform 组件
- 增加耦合，但更自动化

**选择方案 A** — 更灵活，不引入额外耦合。

---

## 需要修改的文件

| 文件 | 改动类型 | 说明 |
|------|----------|------|
| `engine-render/src/plugin.rs` | **新增** | RenderPlugin2D、RenderPlugin3D、render_2d_frame |
| `engine-render/src/lib.rs` | 修改 | 导出 plugin 模块 |
| `engine-render/src/collect_system.rs` | 修改 | 增加 sprite_collect_system、CollectedSprites |
| `engine-core/src/engine.rs` | 修改 | run_default() 使用 RenderPlugin |
| `engine-core/src/app.rs` | 修改 | 增加 renderer_mut() 的 pub 访问（已有）|
| `engine-core/Cargo.toml` | 修改 | 添加 engine-render 依赖（如需要）|

---

## 用户 API 示例

### 2D 精灵渲染

```rust
use engine_core::app::AppBuilder;
use engine_core::transform::Transform;
use engine_render::plugin::RenderPlugin2D;
use engine_render::camera::Camera;
use engine_render::sprite::Sprite;
use std::sync::Arc;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(create_window(&WindowConfig::default(), &event_loop).unwrap());

    let mut builder = AppBuilder::new();
    builder.add_plugin(RenderPlugin2D { window: window.clone() });

    let world = builder.world_mut();

    // 创建相机
    let cam = world.spawn();
    world.add_component(cam, Camera::orthographic(0.0, 800.0, 600.0, 0.0));

    // 创建精灵 — 自动渲染！
    let sprite = world.spawn();
    world.add_component(sprite, Sprite { ... });
    world.add_component(sprite, Transform::from_xyz(400.0, 300.0, 0.0));

    // 进入事件循环 — 精灵自动显示在窗口中
    run_default(builder); // 或自定义事件循环
}
```

### 3D 延迟渲染

```rust
let mut builder = AppBuilder::new();
builder.add_plugin(RenderPlugin3D { window: window.clone() });

let world = builder.world_mut();

// 创建 3D 相机
let cam = world.spawn();
world.add_component(cam, Camera::perspective(FRAC_PI_4, 0.1, 1000.0));

// 创建网格 — 自动渲染！
let mesh = world.spawn();
world.add_component(mesh, Handle::<Mesh>::new(...));
world.add_component(mesh, PbrMaterial { base_color: [1.0, 0.0, 0.0, 1.0], ... });
world.add_component(mesh, Transform::from_xyz(0.0, 0.0, -5.0));

// 添加光源
let light = world.spawn();
world.add_component(light, DirectionalLight { ... });
```

---

## 渲染路径选择

| 条件 | 渲染路径 |
|------|----------|
| ECS 中有 Sprite 组件，无 MeshRenderer | 2D 精灵渲染 (`render_frame`) |
| ECS 中有 MeshRenderer 组件 | 3D 延迟渲染 (`render_frame_3d`) |
| 两者共存 | 先 3D 后 2D（叠加模式） |
| 无渲染组件 | 跳过渲染（纯逻辑模式） |

---

## 成功标准

| 标准 | 验证方式 |
|------|----------|
| 2D 精灵自动渲染 | 添加 Sprite + Camera 组件，窗口中可见精灵 |
| 3D 网格自动渲染 | 添加 MeshRenderer + Camera + Light，窗口中可见 3D 物体 |
| 现有 demo 兼容 | platformer_demo 可选窗口化运行 |
| 零手动渲染代码 | 用户不需要调用 renderer.render_frame() |
| cargo test 通过 | 所有现有测试不回归 |
| cargo clippy 干净 | 零警告 |

---

## 不在范围内

- 编辑器集成
- 网络同步渲染
- 性能优化（超出基本集成需要）
- 新 crate 创建
- 破坏性 API 变更
