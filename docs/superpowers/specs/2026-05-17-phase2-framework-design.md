# RustEngine Phase 2 — 游戏框架层设计

> **项目**: Rust 游戏引擎
> **作者**: ConspiratorR
> **许可证**: MIT
> **日期**: 2026-05-17
> **构建于**: Phase 1 核心引擎之上

## 1. 目标

在 Phase 1 核心引擎（ECS + 场景图 + wgpu 渲染 + 输入/音频）之上，构建游戏框架层：游戏状态管理（GameState 栈）、UI 系统（egui 集成）、输入映射系统（Action/Axis）。

## 2. 架构概览

```
新增 crate:
  engine-framework/     ← 游戏状态栈 + 生命周期 + StateCtx
  engine-ui/            ← egui 集成渲染

扩展 crate:
  engine-input/         ← 新增 action/ 模块：ActionMap, ActionState, Binding
```

依赖关系：
```
game-app
  └── engine-core
       ├── engine-framework  (依赖: core, scene)
       ├── engine-ui         (依赖: render, egui)
       ├── engine-input      (新增 action 模块)
       └── ...Phase 1 crates
```

所有新功能以 Plugin 形式注册到 AppBuilder，与 Phase 1 风格一致。

## 3. 场景管理（engine-framework）

### 3.1 GameState trait

```rust
pub trait GameState {
    /// 进入此状态时调用（push/replace 后）
    fn on_enter(&mut self, ctx: &mut StateCtx);

    /// 离开此状态时调用（被 pop/replace 时）
    fn on_exit(&mut self, ctx: &mut StateCtx);

    /// 每帧更新（仅在栈顶状态调用）
    fn update(&mut self, ctx: &mut StateCtx, dt: f32);
}
```

### 3.2 StateStack（延迟操作队列）

`StateStack` 作为 Resource。`push`/`pop`/`replace` 仅将操作加入队列，不在调用时立即执行生命周期回调——避免 `StateCtx` 的 borrow 冲突。

```rust
pub struct StateStack {
    states: Vec<Box<dyn GameState>>,
    pending: Vec<PendingOp>,
}

enum PendingOp {
    Push(Box<dyn GameState>),
    Pop,
    Replace(Box<dyn GameState>),
}

impl StateStack {
    pub fn push(&mut self, state: Box<dyn GameState>);

    pub fn pop(&mut self);

    pub fn replace(&mut self, state: Box<dyn GameState>);

    /// 框架 system 在每次 update 后调用，按序执行 pending 操作
    /// 调用 on_exit（旧状态）→ on_enter（新状态）
    pub fn flush(&mut self, world: &mut World, resources: &mut ResourceRegistry);

    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

- `push`: 将新状态加入 pending 队列
- `pop`: 将 Pop 加入 pending 队列
- `replace`: 将 Replace 加入 pending 队列（Pop + Push）
- `flush`: 由 `FrameworkPlugin` 的 system 在每帧状态 `update` 后调用，执行队列中的所有操作，触发对应的 `on_exit`/`on_enter`

### 3.3 StateCtx

```rust
pub struct StateCtx<'a> {
    world: &'a mut World,       // 状态可读写 ECS World
    resources: &'a mut ResourceRegistry,  // 状态可访问 Resource
    delta: f32,                 // 本帧 Delta Time
}
```

- 状态通过 `ctx.resources.get_mut::<StateStack>()` 访问状态栈并 push/pop

### 3.4 FrameworkPlugin + FrameworkResource

`FrameworkPlugin` 注册：
- `StateStack` 作为 Resource
- `FrameworkResource`（含 delta_time、frame_count）
- 内置 system 每帧更新栈顶状态（调用 `update` + 传递 `StateCtx`）

```rust
pub struct FrameworkPlugin;

impl Plugin for FrameworkPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(StateStack::new());
        app.insert_resource(FrameworkResource::new());
        app.add_system(framework_update_system);
    }
}
```

## 4. UI 系统（engine-ui）

### 4.1 egui 集成架构

```
engine-ui/src/
├── lib.rs              — EguiPlugin 导出
├── integration.rs      — EguiIntegration 资源 + state 管理
├── renderer.rs         — wgpu 渲染器（egui-wgpu）
└── input.rs            — winit 事件转换（egui-winit）
```

### 4.2 EguiPlugin

```rust
pub struct EguiPlugin;

impl Plugin for EguiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(EguiIntegration::new());
        app.add_system(egui_render_system);
    }
}
```

### 4.3 EguiIntegration

```rust
pub struct EguiIntegration {
    ctx: egui::Context,
    // 内部管理 egui-winit state + wgpu renderer
}

impl EguiIntegration {
    pub fn new() -> Self;
    pub fn context(&self) -> &egui::Context;   // 暴露 egui::Context
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent);
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, output: &wgpu::TextureView, window: &winit::window::Window);
}
```

### 4.4 渲染集成

- `EguiPlugin` 不依赖 `engine-core` 的事件循环——由 `engine-core` 在事件循环中调用 `egui_integration.handle_event(...)` 传递 winit 事件
- 渲染：`engine-core` 的 `Engine::run` 在最后一个 pass 中调用 `egui_integration.render(...)`
- 键盘/鼠标输入通过 `handle_event` 将 winit 事件转发给 egui

### 4.5 与 Engine 集成

egui 需要访问 winit 事件和 wgpu 资源，但 `engine-core` 的 `Engine::run` 拥有事件循环的所有权。集成方案：

- `Renderer` 注册为 Resource（供 egui 和其他 system 访问 wgpu Device/Queue）
- `EguiIntegration` 注册为 Resource
- `Engine::run` 在事件循环中调用：
  1. 将 winit 的 `WindowEvent::KeyboardInput`、`CursorMoved` 等转发给 `EguiIntegration::handle_event`
  2. 在 `AboutToWait` 回调中执行 `app.run()`（运行 schedule）
  3. schedule 运行完毕后，获取 `EguiIntegration` 资源并调用 `render()`（使用 Renderer 资源的 device/queue）

渲染管线顺序：
```
app.run()  →  ECS schedule (游戏逻辑 + UI 数据更新)
  ↓
renderer.present() 之前的最后阶段插入 egui render pass
  ↓
renderer.present()  →  swapchain 呈现
```

### 4.6 生命周期

Phase 2 暂不实现：
- 字体配置（使用 egui 默认字体）
- 自定义 theme
- 多窗口

## 5. 输入映射系统（engine-input/action）

### 5.1 架构

在 `engine-input` crate 内新增 `action` 模块：

```
engine-input/src/action/
├── mod.rs              — 公开 API: ActionMap, ActionState, Binding
├── binding.rs          — Binding 枚举（数字/轴绑定）
└── action.rs           — ActionMap 实现 + ActionState
```

### 5.2 ActionMap

```rust
pub struct ActionMap {
    bindings: Vec<Binding>,
    states: HashMap<String, ActionState>,
}

impl ActionMap {
    pub fn new() -> Self;

    /// 绑定按键到数字动作
    pub fn bind_key(&mut self, action: &str, key: KeyCode);

    /// 绑定两个按键到轴动作（正/负）
    pub fn bind_axis(&mut self, action: &str, positive: KeyCode, negative: KeyCode);

    /// 批量从迭代器绑定
    pub fn bind_all(&mut self, bindings: impl IntoIterator<Item = (String, Binding)>);

    /// 查询动作状态
    pub fn action(&self, name: &str) -> ActionState;

    /// 每帧调用，从 InputManager 刷新所有 action 状态
    pub fn update(&mut self, input: &InputManager);
}
```

### 5.3 ActionState

```rust
#[derive(Debug, Clone, Copy)]
pub struct ActionState {
    /// 当前帧的值（0.0-1.0 for digital, -1.0-1.0 for axis）
    pub value: f32,
}

impl ActionState {
    pub fn just_pressed(&self) -> bool;
    pub fn pressed(&self) -> bool;
    pub fn just_released(&self) -> bool;
}
```

### 5.4 Binding

```rust
pub enum Binding {
    Key(KeyCode),
    Axis { positive: KeyCode, negative: KeyCode },
    MouseButton(MouseButton),
}
```

### 5.5 ActionPlugin

```rust
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionMap::new());
        app.add_system(action_update_system);
    }
}
```

`action_update_system` 每帧从 `InputManager`（作为 Resource）读取按下状态，计算每个 action 的当前 value。

### 5.6 输入优先级

Phase 2 暂不实现：
- 游戏手柄输入
- 配置文件热加载
- 组合键（Ctrl+Shift+...）
- 双击/长按检测

## 6. Engine-Core 集成变更

Phase 2 需要以下 engine-core 变更以支持新子系统：

### 6.1 Renderer 注册为 Resource

`App` 中新增 `renderer` 字段，`Engine::run` 构造后注入。Renderer 通过 ResourceRegistry 暴露给 system：

```rust
// app.rs
pub struct App {
    world: World,
    schedule: Schedule,
    resources: ResourceRegistry,
    renderer: Option<Renderer>,  // 新增
}
```

`Renderer` 实现 Clone（Arc 包装 device/queue），通过 `resources.insert::<Renderer>(...)` 注册。

### 6.2 InputManager 注册为 Resource

`AppBuilder` 中默认注册一个 `InputManager` 资源。事件循环中从 winit 事件更新其状态：

```rust
// engine.rs 事件循环中
if let winit::event::WindowEvent::KeyboardInput { event, .. } = ... {
    app.input_mut().handle_keyboard_event(&event);
}
```

### 6.3 Event Loop 转发 winit 事件

`Engine::run` / `run_default` 更新为：
1. 将 winit 的 `WindowEvent` 分发给 `InputManager` Resource
2. 将 `WindowEvent` 分发给 `EguiIntegration` Resource（如果存在）
3. Schedule 运行完毕后，调用 `EguiIntegration::render()` 作为 overlay pass
4. 然后调用 `Renderer::present()`

渲染管线最终顺序：
```
Clear (black)
  → Sprite pass (engine-render)
  → 3D PBR pass
  → Egui overlay pass  ← 新增
  → Present swapchain
```

### 6.4 `App` 新增方法

```rust
impl App {
    pub fn input_mut(&mut self) -> &mut InputManager;
    pub fn renderer(&self) -> Option<&Renderer>;
    pub fn renderer_mut(&mut self) -> Option<&mut Renderer>;
    pub fn set_renderer(&mut self, renderer: Renderer);
    pub fn egui_mut(&mut self) -> Option<&mut EguiIntegration>;
}
```

## 7. Crate 配置

### engine-framework/Cargo.toml
```toml
[dependencies]
engine-core = { path = "../engine-core" }
engine-scene = { path = "../engine-scene" }
engine-ecs = { path = "../engine-ecs" }
```

### engine-ui/Cargo.toml
```toml
[dependencies]
engine-core = { path = "../engine-core" }
engine-render = { path = "../engine-render" }
egui = "0.28"
egui-wgpu = "0.28"
egui-winit = "0.28"
winit = "0.30"
```

### engine-input 新增依赖
无新外部依赖。仅新增内部模块。

## 8. Workspace 更新

更新根 Cargo.toml 将新 crate 加入 workspace members。

## 9. 示例更新

`examples/basic` 更新为演示 Phase 2 能力：

```rust
fn main() {
    AppBuilder::new()
        .add_plugin(FrameworkPlugin)
        .add_plugin(EguiPlugin::default())
        .add_plugin(ActionPlugin::default())
        .add_plugin(GamePlugin)
        .build()
        .run();
}

struct MenuState;
struct GameplayState;

impl GameState for MenuState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {
        // 初始化菜单场景
    }
    fn update(&mut self, ctx: &mut StateCtx, _dt: f32) {
        // 从 Resource 获取 ActionMap 检测输入
        let actions = ctx.resources.get::<ActionMap>().unwrap();
        if actions.action("start_game").just_pressed() {
            // 通过 Resource 获取 StateStack 进行状态切换
            ctx.resources.get_mut::<StateStack>().unwrap()
                .push(Box::new(GameplayState));
        }
    }
    fn on_exit(&mut self, _ctx: &mut StateCtx) {
        // 清理菜单
    }
}

impl GameState for GameplayState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {}
    fn update(&mut self, _ctx: &mut StateCtx, _dt: f32) {}
    fn on_exit(&mut self, _ctx: &mut StateCtx) {}
}
```

## 10. 不做清单（递延到后续阶段）

- 游戏手柄/触控输入
- Action/Axis 配置文件的自动加载与保存
- egui 字体/主题定制
- UI 动画过渡
- 多场景并行更新
- 场景序列化
