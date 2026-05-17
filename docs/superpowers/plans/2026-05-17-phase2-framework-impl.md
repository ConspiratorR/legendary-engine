# Phase 2 — Game Framework Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add game state management (GameState stack), egui-based UI system, and Action/Axis input mapping on top of the Phase 1 engine.

**Architecture:** engine-core App gains pre/post update hooks. engine-framework and engine-ui register hooks via their Plugin. engine-input gains action module.

**Tech Stack:** Rust 2024, egui 0.28, egui-wgpu 0.28, egui-winit 0.28

**Spec:** `docs/superpowers/specs/2026-05-17-phase2-framework-design.md`

---

## File Structure

```
crates/engine-input/src/action/
├── mod.rs                  — pub use ActionMap, ActionState, Binding
├── binding.rs              — Binding enum
└── action_map.rs           — ActionMap + ActionState + action_update_system + tests

crates/engine-framework/
├── Cargo.toml
└── src/
    ├── lib.rs              — pub mod + re-exports
    ├── state.rs            — GameState trait
    ├── stack.rs            — StateStack (pending queue + flush) + tests
    ├── ctx.rs              — StateCtx
    ├── resource.rs         — FrameworkResource
    └── plugin.rs           — FrameworkPlugin + hooks

crates/engine-ui/
├── Cargo.toml
└── src/
    ├── lib.rs              — pub mod + re-exports
    ├── integration.rs      — EguiIntegration resource + tests
    └── plugin.rs           — EguiPlugin + hooks

Modified:
  crates/engine-input/src/lib.rs        — add `pub mod action;`
  crates/engine-core/src/app.rs         — add renderer, hooks, run_frame()
  crates/engine-core/src/engine.rs      — event loop: forward events
  crates/engine-core/src/lib.rs         — add pub mod plugins;
  crates/engine-core/src/plugins.rs     — ActionPlugin
  crates/engine-render/src/renderer.rs  — GpuDevice, GpuQueue
  Cargo.toml                            — add members
  examples/basic/src/main.rs            — Phase 2 demo
```

---

### Task 1: engine-input — Action module

**Files:**
- Create: `crates/engine-input/src/action/binding.rs`
- Create: `crates/engine-input/src/action/action_map.rs`
- Create: `crates/engine-input/src/action/mod.rs`
- Modify: `crates/engine-input/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

```rust
// In action_map.rs
#[cfg(test)]
mod tests {
    use crate::action::Binding;
    use crate::action::action_map::ActionMap;
    use crate::keyboard::KeyCode;
    use crate::input_manager::InputManager;

    #[test]
    fn test_bind_key_not_pressed_initially() {
        let map = ActionMap::new();
        map.bind_key("jump", KeyCode::Space);
        assert!(!map.action("jump").pressed());
    }

    #[test]
    fn test_just_pressed_after_key_down() {
        let mut map = ActionMap::new();
        map.bind_key("fire", KeyCode::KeyE);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyE);
        map.update(&input);
        assert!(map.action("fire").just_pressed());
        assert!(map.action("fire").pressed());
    }

    #[test]
    fn test_just_released_after_key_up() {
        let mut map = ActionMap::new();
        map.bind_key("fire", KeyCode::KeyE);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyE);
        map.update(&input);
        input.release(KeyCode::KeyE);
        map.update(&input);
        assert!(map.action("fire").just_released());
        assert!(!map.action("fire").pressed());
    }

    #[test]
    fn test_axis_positive_value() {
        let mut map = ActionMap::new();
        map.bind_axis("move_x", KeyCode::KeyD, KeyCode::KeyA);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyD);
        map.update(&input);
        assert!((map.action("move_x").value - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_axis_negative_value() {
        let mut map = ActionMap::new();
        map.bind_axis("move_x", KeyCode::KeyD, KeyCode::KeyA);
        let mut input = InputManager::new();
        input.press(KeyCode::KeyA);
        map.update(&input);
        assert!((map.action("move_x").value - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_unknown_action_returns_default() {
        let map = ActionMap::new();
        assert_eq!(map.action("nonexistent").value, 0.0);
        assert!(!map.action("nonexistent").pressed());
    }

    #[test]
    fn test_bind_all_batch() {
        let mut map = ActionMap::new();
        map.bind_all(vec![
            ("jump".to_string(), Binding::Key(KeyCode::Space)),
            ("crouch".to_string(), Binding::Key(KeyCode::ShiftLeft)),
        ]);
        assert_eq!(map.bindings().len(), 2);
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-input`
Expected: Compilation fails — module `action` not found.

- [ ] **Step 3: Implement binding.rs**

```rust
use crate::keyboard::KeyCode;

#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    Key(KeyCode),
    Axis { positive: KeyCode, negative: KeyCode },
}
```

- [ ] **Step 4: Implement action_map.rs**

```rust
use std::collections::HashMap;
use crate::keyboard::KeyCode;
use crate::input_manager::InputManager;
use crate::action::Binding;

#[derive(Debug, Clone, Copy)]
pub struct ActionState {
    pub value: f32,
    just_pressed: bool,
    just_released: bool,
    previous: f32,
}

impl ActionState {
    pub fn just_pressed(&self) -> bool { self.just_pressed }
    pub fn pressed(&self) -> bool { self.value != 0.0 }
    pub fn just_released(&self) -> bool { self.just_released }
}

pub struct ActionMap {
    bindings: Vec<(String, Binding)>,
    states: HashMap<String, ActionState>,
}

impl ActionMap {
    pub fn new() -> Self { Self { bindings: Vec::new(), states: HashMap::new() } }

    pub fn bind_key(&mut self, action: &str, key: KeyCode) {
        self.bindings.push((action.to_string(), Binding::Key(key)));
    }

    pub fn bind_axis(&mut self, action: &str, positive: KeyCode, negative: KeyCode) {
        self.bindings.push((action.to_string(), Binding::Axis { positive, negative }));
    }

    pub fn bind_all(&mut self, bindings: impl IntoIterator<Item = (String, Binding)>) {
        self.bindings.extend(bindings);
    }

    pub fn bindings(&self) -> &[(String, Binding)] { &self.bindings }

    pub fn action(&self, name: &str) -> ActionState {
        self.states.get(name).copied().unwrap_or(ActionState {
            value: 0.0, just_pressed: false, just_released: false, previous: 0.0,
        })
    }

    pub fn update(&mut self, input: &InputManager) {
        let mut values: HashMap<String, f32> = HashMap::new();
        for (name, binding) in &self.bindings {
            let v = match binding {
                Binding::Key(key) => if input.key_down(*key) { 1.0 } else { 0.0 },
                Binding::Axis { positive, negative } => {
                    let mut v = 0.0;
                    if input.key_down(*positive) { v += 1.0; }
                    if input.key_down(*negative) { v -= 1.0; }
                    v
                }
            };
            let e = values.entry(name.clone()).or_insert(0.0);
            if v.abs() > e.abs() { *e = v; }
        }
        for (name, value) in &values {
            let prev = self.states.get(name.as_str()).map(|s| s.value).unwrap_or(0.0);
            let was = prev != 0.0;
            let now = *value != 0.0;
            self.states.insert(name.clone(), ActionState {
                value: *value, just_pressed: !was && now,
                just_released: was && !now, previous: prev,
            });
        }
    }
}

pub fn action_update_system(_world: &mut engine_ecs::world::World) {}
```

- [ ] **Step 5: Implement action/mod.rs**

```rust
pub mod action_map;
pub mod binding;
pub use action_map::{ActionMap, ActionState, action_update_system};
pub use binding::Binding;
```

- [ ] **Step 6: Update lib.rs**

Add `pub mod action;` to `crates/engine-input/src/lib.rs`.

- [ ] **Step 7: Run tests to pass**

Run: `cargo test -p engine-input`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/engine-input/
git commit -m "feat(engine-input): ActionMap, ActionState, Binding for input mapping"
```

---

### Task 2: engine-core — Hook system, Resources, event loop

**Files:**
- Modify: `crates/engine-core/src/app.rs`
- Modify: `crates/engine-core/src/engine.rs`
- Modify: `crates/engine-core/src/lib.rs`
- Create: `crates/engine-core/src/plugins.rs`
- Modify: `crates/engine-render/src/renderer.rs`

- [ ] **Step 1: Write failing tests**

```rust
// In app.rs mod tests
    use crate::app::App;
    use engine_input::input_manager::InputManager;

    #[test]
    fn test_app_has_input_manager() {
        let app = App::new();
        assert!(app.resources.get::<InputManager>().is_some());
    }

    #[test]
    fn test_renderer_starts_none() {
        let app = App::new();
        assert!(app.renderer().is_none());
    }

    #[test]
    fn test_pre_hook_runs_during_run() {
        let mut app = App::new();
        let mut flag = false;
        app.pre_update_hooks.push(Box::new(|_: &mut App| {
            // hook ran
        }));
        app.run(); // just verify no panic
    }
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-core`
Expected: Compilation fails — InputManager not registered.

- [ ] **Step 3: Add GpuDevice/GpuQueue to renderer.rs**

```rust
use std::sync::Arc;

#[derive(Clone)]
pub struct GpuDevice(pub Arc<wgpu::Device>);
#[derive(Clone)]
pub struct GpuQueue(pub Arc<wgpu::Queue>);
```

- [ ] **Step 4: Update app.rs**

App struct gains `renderer: Option<Renderer>`, `pre_update_hooks`, `post_update_hooks`. `App::new()` registers `InputManager` as a Resource. `App::run()` calls hooks and updates input/actions:

```rust
use engine_input::input_manager::InputManager;

pub struct App {
    pub world: World,
    pub schedule: Schedule,
    pub resources: ResourceRegistry,
    renderer: Option<engine_render::renderer::Renderer>,
    pub pre_update_hooks: Vec<Box<dyn FnMut(&mut App)>>,
    pub post_update_hooks: Vec<Box<dyn FnMut(&mut App)>>,
}

impl App {
    pub fn new() -> Self {
        let mut resources = ResourceRegistry::new();
        resources.insert(InputManager::new());
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources,
            renderer: None,
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        for hook in &mut self.pre_update_hooks { hook(self); }
        if let Some(input) = self.resources.get_mut::<InputManager>() {
            input.update_frame();
        }
        if let Some(actions) = self.resources.get_mut::<engine_input::action::ActionMap>() {
            if let Some(input) = self.resources.get::<InputManager>() {
                actions.update(input);
            }
        }
        self.schedule.run(&mut self.world);
        for hook in &mut self.post_update_hooks { hook(self); }
    }

    pub fn input_mut(&mut self) -> &mut InputManager {
        self.resources.get_mut::<InputManager>().unwrap()
    }

    pub fn renderer(&self) -> Option<&engine_render::renderer::Renderer> { self.renderer.as_ref() }
    pub fn renderer_mut(&mut self) -> Option<&mut engine_render::renderer::Renderer> { self.renderer.as_mut() }

    pub fn set_renderer(&mut self, renderer: engine_render::renderer::Renderer) {
        let device = renderer.device.clone();
        let queue = renderer.queue.clone();
        self.resources.insert(engine_render::renderer::GpuDevice(Arc::new(device)));
        self.resources.insert(engine_render::renderer::GpuQueue(Arc::new(queue)));
        self.renderer = Some(renderer);
    }

    pub fn resources_mut(&mut self) -> &mut ResourceRegistry { &mut self.resources }
    pub fn world_mut(&mut self) -> &mut World { &mut self.world }
}
```

Update `AppBuilder`:

```rust
pub struct AppBuilder {
    world: World,
    schedule: Schedule,
    resources: ResourceRegistry,
    pre_update_hooks: Vec<Box<dyn FnMut(&mut App)>>,
    post_update_hooks: Vec<Box<dyn FnMut(&mut App)>>,
}

impl AppBuilder {
    pub fn new() -> Self {
        let mut resources = ResourceRegistry::new();
        resources.insert(InputManager::new());
        Self {
            world: World::new(),
            schedule: Schedule::new(),
            resources,
            pre_update_hooks: Vec::new(),
            post_update_hooks: Vec::new(),
        }
    }

    pub fn add_pre_update_hook(&mut self, hook: Box<dyn FnMut(&mut App)>) -> &mut Self {
        self.pre_update_hooks.push(hook);
        self
    }

    pub fn add_post_update_hook(&mut self, hook: Box<dyn FnMut(&mut App)>) -> &mut Self {
        self.post_update_hooks.push(hook);
        self
    }
}
```

Update `From<AppBuilder>`:

```rust
impl From<AppBuilder> for App {
    fn from(b: AppBuilder) -> Self {
        Self {
            world: b.world,
            schedule: b.schedule,
            resources: b.resources,
            renderer: None,
            pre_update_hooks: b.pre_update_hooks,
            post_update_hooks: b.post_update_hooks,
        }
    }
}
```

Also add `pub fn resources_mut(&mut self) -> &mut ResourceRegistry` and `pub fn world_mut(&mut self) -> &mut World` to `App`.

- [ ] **Step 5: Create plugins.rs**

```rust
use engine_input::action::ActionMap;
use crate::app::AppBuilder;
use crate::plugin::Plugin;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionMap::new());
    }
}
```

- [ ] **Step 6: Update lib.rs**

```rust
pub mod plugins;
```

- [ ] **Step 7: Update engine.rs**

Update `run_default` to forward winit keyboard/mouse events to InputManager:

```rust
use winit::event::{Event, WindowEvent, ElementState, MouseButton};
use winit::event_loop::{ControlFlow, EventLoop};

#[allow(deprecated)]
pub fn run_default(app_builder: AppBuilder) {
    let mut app = app_builder.build();
    let event_loop = EventLoop::new().unwrap();
    let window = std::sync::Arc::new(create_window(&WindowConfig::default(), &event_loop));
    let renderer = engine_render::renderer::Renderer::new(window.clone());
    app.set_renderer(renderer);

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        match &event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => elwt.exit(),
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                if let Some(r) = app.renderer_mut() { r.resize(size.width, size.height); }
            }
            Event::WindowEvent { event: WindowEvent::KeyboardInput { event: ke, .. }, .. } => {
                let input = app.input_mut();
                if let Some(key) = ke.physical_key {
                    if ke.state == ElementState::Pressed { input.press(key); }
                    else { input.release(key); }
                }
            }
            Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                app.input_mut().mouse_mut().position = (position.x, position.y);
            }
            Event::WindowEvent { event: WindowEvent::MouseInput { state, button, .. }, .. } => {
                let input = app.input_mut();
                let pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => input.mouse_mut().left_button = pressed,
                    MouseButton::Right => input.mouse_mut().right_button = pressed,
                    _ => {}
                }
            }
            _ => {}
        }
        if let Event::AboutToWait = event {
            app.run();
            if let Some(r) = app.renderer_mut() { let _ = r.present(); }
        }
    }).unwrap();
}
```

Remove the old imports (`winit::event::{Event, WindowEvent}`, `winit::event_loop::ControlFlow`) that are now explicit.

- [ ] **Step 8: Run tests to pass**

Run: `cargo test -p engine-core`
Expected: Tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/engine-core/ crates/engine-render/
git commit -m "feat(engine-core): hook system, InputManager/Renderer as resources, event loop forwarding"
```

---

### Task 3: engine-framework — Game state management

**Files:**
- Create: `crates/engine-framework/Cargo.toml`
- Create: `crates/engine-framework/src/lib.rs`
- Create: `crates/engine-framework/src/state.rs`
- Create: `crates/engine-framework/src/stack.rs`
- Create: `crates/engine-framework/src/ctx.rs`
- Create: `crates/engine-framework/src/resource.rs`
- Create: `crates/engine-framework/src/plugin.rs`

- [ ] **Step 1: Write failing tests**

```rust
// In stack.rs
#[cfg(test)]
mod tests {
    use crate::GameState;
    use crate::StateCtx;
    use crate::StateStack;
    use engine_ecs::world::World;
    use engine_core::resource::ResourceRegistry;

    struct TestState;
    impl GameState for TestState {
        fn on_enter(&mut self, _: &mut StateCtx) {}
        fn on_exit(&mut self, _: &mut StateCtx) {}
        fn update(&mut self, _: &mut StateCtx, _: f32) {}
    }

    #[test] fn test_push_then_flush_adds_state() {
        let mut w = World::new(); let mut r = ResourceRegistry::new(); let mut s = StateStack::new();
        s.push(Box::new(TestState));
        assert_eq!(s.len(), 0);
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 1);
    }

    #[test] fn test_pop_removes_state() {
        let mut w = World::new(); let mut r = ResourceRegistry::new(); let mut s = StateStack::new();
        s.push(Box::new(TestState)); s.flush(&mut w, &mut r);
        s.pop(); s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 0);
    }

    #[test] fn test_replace_swaps() {
        let mut w = World::new(); let mut r = ResourceRegistry::new(); let mut s = StateStack::new();
        s.push(Box::new(TestState)); s.flush(&mut w, &mut r);
        s.replace(Box::new(TestState)); s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 1);
    }

    #[test] fn test_empty_state() {
        let s = StateStack::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test] fn test_multiple_pending() {
        let mut w = World::new(); let mut r = ResourceRegistry::new(); let mut s = StateStack::new();
        s.push(Box::new(TestState)); s.push(Box::new(TestState));
        s.flush(&mut w, &mut r);
        assert_eq!(s.len(), 2);
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-framework`
Expected: Compilation fails — crate doesn't exist.

- [ ] **Step 3: Create Cargo.toml**

```toml
[package]
name = "engine-framework"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-core = { path = "../engine-core" }
engine-scene = { path = "../engine-scene" }
engine-ecs = { path = "../engine-ecs" }
```

- [ ] **Step 4: Implement state.rs**

```rust
use crate::StateCtx;

pub trait GameState {
    fn on_enter(&mut self, _: &mut StateCtx) {}
    fn on_exit(&mut self, _: &mut StateCtx) {}
    fn update(&mut self, _: &mut StateCtx, _: f32) {}
}
```

- [ ] **Step 5: Implement ctx.rs**

```rust
use engine_ecs::world::World;
use engine_core::resource::ResourceRegistry;

pub struct StateCtx<'a> {
    pub world: &'a mut World,
    pub resources: &'a mut ResourceRegistry,
    pub delta: f32,
}
```

- [ ] **Step 6: Implement stack.rs**

```rust
use crate::{GameState, StateCtx};
use engine_ecs::world::World;
use engine_core::resource::ResourceRegistry;

enum PendingOp { Push(Box<dyn GameState>), Pop, Replace(Box<dyn GameState>) }

pub struct StateStack {
    states: Vec<Box<dyn GameState>>,
    pending: Vec<PendingOp>,
}

impl StateStack {
    pub fn new() -> Self { Self { states: Vec::new(), pending: Vec::new() } }
    pub fn push(&mut self, state: Box<dyn GameState>) { self.pending.push(PendingOp::Push(state)); }
    pub fn pop(&mut self) { self.pending.push(PendingOp::Pop); }
    pub fn replace(&mut self, state: Box<dyn GameState>) { self.pending.push(PendingOp::Replace(state)); }
    pub fn len(&self) -> usize { self.states.len() }
    pub fn is_empty(&self) -> bool { self.states.is_empty() }

    pub fn flush(&mut self, world: &mut World, resources: &mut ResourceRegistry) {
        let ops = std::mem::take(&mut self.pending);
        for op in ops { match op {
            PendingOp::Push(mut s) => { s.on_enter(&mut StateCtx { world, resources, delta: 0.0 }); self.states.push(s); }
            PendingOp::Pop => { if let Some(mut s) = self.states.pop() { s.on_exit(&mut StateCtx { world, resources, delta: 0.0 }); } }
            PendingOp::Replace(mut s) => {
                if let Some(mut o) = self.states.pop() { o.on_exit(&mut StateCtx { world, resources, delta: 0.0 }); }
                s.on_enter(&mut StateCtx { world, resources, delta: 0.0 }); self.states.push(s);
            }
        }}
    }

    pub fn update_top(&mut self, world: &mut World, resources: &mut ResourceRegistry, dt: f32) {
        if let Some(top) = self.states.last_mut() {
            top.update(&mut StateCtx { world, resources, delta: dt }, dt);
        }
    }
}
```

- [ ] **Step 7: Implement resource.rs**

```rust
pub struct FrameworkResource {
    pub delta_time: f32,
    pub frame_count: u64,
}

impl FrameworkResource {
    pub fn new() -> Self { Self { delta_time: 0.0, frame_count: 0 } }
}
```

- [ ] **Step 8: Implement plugin.rs**

```rust
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use crate::{StateStack, FrameworkResource};

pub struct FrameworkPlugin;

impl Plugin for FrameworkPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(StateStack::new());
        app.insert_resource(FrameworkResource::new());
        app.add_pre_update_hook(Box::new(|app: &mut App| {
            if let Some(fw) = app.resources_mut().get_mut::<FrameworkResource>() {
                fw.frame_count += 1;
            }
            if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                let dt = 0.016; // approximate; improved later
                stack.update_top(app.world_mut(), app.resources_mut(), dt);
            }
        }));
        app.add_post_update_hook(Box::new(|app: &mut App| {
            if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
                stack.flush(app.world_mut(), app.resources_mut());
            }
        }));
    }
}
```

- [ ] **Step 9: Implement lib.rs**

```rust
pub mod state;
pub mod stack;
pub mod ctx;
pub mod resource;
pub mod plugin;

pub use state::GameState;
pub use stack::StateStack;
pub use ctx::StateCtx;
pub use resource::FrameworkResource;
pub use plugin::FrameworkPlugin;
```

- [ ] **Step 10: Run tests to pass**

Run: `cargo test -p engine-framework`
Expected: All tests pass.

- [ ] **Step 11: Commit**

```bash
git add crates/engine-framework/
git commit -m "feat(engine-framework): GameState, StateStack, StateCtx, FrameworkPlugin"
```

---

### Task 4: engine-ui — egui integration

**Files:**
- Create: `crates/engine-ui/Cargo.toml`
- Create: `crates/engine-ui/src/lib.rs`
- Create: `crates/engine-ui/src/integration.rs`
- Create: `crates/engine-ui/src/plugin.rs`

- [ ] **Step 1: Write failing tests**

```rust
// In integration.rs
#[cfg(test)]
mod tests {
    use crate::integration::EguiIntegration;

    #[test]
    fn test_egui_integration_create() {
        let _egui = EguiIntegration::new();
    }

    #[test]
    fn test_egui_context_accessible() {
        let egui = EguiIntegration::new();
        let _ctx = egui.context();
    }
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p engine-ui`
Expected: Compilation fails — crate doesn't exist.

- [ ] **Step 3: Create Cargo.toml**

```toml
[package]
name = "engine-ui"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
engine-core = { path = "../engine-core" }
engine-render = { path = "../engine-render" }
egui = "0.28"
egui-wgpu = "0.28"
egui-winit = "0.28"
winit = "0.30"
```

- [ ] **Step 4: Implement integration.rs**

```rust
pub struct EguiIntegration {
    ctx: egui::Context,
    state: Option<egui_winit::State>,
    renderer: Option<egui_wgpu::Renderer>,
    output_format: wgpu::TextureFormat,
}

impl EguiIntegration {
    pub fn new(output_format: wgpu::TextureFormat) -> Self {
        Self { ctx: egui::Context::default(), state: None, renderer: None, output_format }
    }

    pub fn context(&self) -> &egui::Context { &self.ctx }

    pub fn init_with_window(&mut self, window: &winit::window::Window) {
        if self.state.is_none() {
            self.state = Some(egui_winit::State::new(
                egui::ViewportId::from_hash_of("main"), window, None, None,
            ));
        }
    }

    pub fn handle_event(&mut self, event: &winit::event::WindowEvent, window: &winit::window::Window) {
        self.init_with_window(window);
        if let Some(state) = &mut self.state {
            let _ = state.on_event(&self.ctx, event);
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        window: &winit::window::Window,
    ) {
        self.init_with_window(window);
        let raw_input = self.state.as_ref()
            .map(|s| s.take_egui_input(window))
            .unwrap_or_default();
        let full_output = self.ctx.run(raw_input, |_ctx| {});
        let tris = self.ctx.tessellate(full_output.shapes, self.ctx.pixels_per_point());
        let renderer = self.renderer.get_or_insert_with(|| {
            egui_wgpu::Renderer::new(device, self.output_format, None, 1)
        });
        for (id, delta) in &full_output.textures_delta.set {
            renderer.update_texture(device, queue, *id, delta);
        }
        renderer.update_buffers(device, queue, &tris, self.ctx.pixels_per_point());
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        renderer.render(&mut rpass, &tris, &Default::default());
    }
}
```

Note: The implementer may need to adapt egui API calls to match egui 0.28's actual surface (e.g., `take_egui_input`, `Renderer::new` signature, `tessellate` method). The structural approach is correct. Also update `EguiPlugin::build()` to pass the output format from the Renderer's surface config.

- [ ] **Step 5: Implement plugin.rs**

```rust
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_render::renderer::{GpuDevice, GpuQueue};
use crate::integration::EguiIntegration;

pub struct EguiPlugin;

impl Plugin for EguiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(EguiIntegration::new());
        app.add_post_update_hook(Box::new(|app: &mut App| {
            // Egui rendering happens in the post-update hook
            // The actual rendering with wgpu is done by the event loop after app.run()
        }));
    }
}

/// Called from the event loop after app.run() to render egui overlay.
/// This function is separate from the hook because it needs the surface texture view.
pub fn render_egui_overlay(
    egui: &mut EguiIntegration,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    output: &wgpu::TextureView,
    window: &winit::window::Window,
) {
    egui.render(device, queue, encoder, output, window);
}
```

- [ ] **Step 6: Implement lib.rs**

```rust
pub mod integration;
pub mod plugin;
pub use integration::EguiIntegration;
pub use plugin::EguiPlugin;
```

- [ ] **Step 7: Update engine.rs for egui rendering**

Refactor the `AboutToWait` event handler in `run_default` to inline the encoder setup so egui can share it:

```rust
if let Event::AboutToWait = event {
    app.run();
    if let Some(r) = app.renderer_mut() {
        let output = r.surface.get_current_texture().ok();
        if let Some(output) = output {
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = r.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });
            // Render egui overlay (if present)
            let device = r.device.clone();
            let queue = r.queue.clone();
            if let Some(egui) = app.resources.get_mut::<EguiIntegration>() {
                egui.render(&device, &queue, &mut encoder, &view, &window);
            }
            r.queue.submit([encoder.finish()]);
            output.present();
        }
    }
}
```

Add `use engine_ui::EguiIntegration;` to the imports.

- [ ] **Step 8: Run tests to pass**

Run: `cargo test -p engine-ui`
Expected: Tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/engine-ui/
git commit -m "feat(engine-ui): egui integration with EguiIntegration, EguiPlugin"
```

---

### Task 5: Workspace + example demo

**Files:**
- Modify: `Cargo.toml` (root workspace)
- Modify: `examples/basic/src/main.rs`

- [ ] **Step 1: Update workspace Cargo.toml**

Add to `members` list:
```toml
    "crates/engine-framework",
    "crates/engine-ui",
```

- [ ] **Step 2: Update example main.rs**

```rust
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_core::plugins::ActionPlugin;
use engine_framework::{FrameworkPlugin, GameState, StateCtx, StateStack};
use engine_ui::EguiPlugin;
use engine_input::action::ActionMap;
use engine_input::keyboard::KeyCode;

struct MenuState;
struct GameplayState;

impl GameState for MenuState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {
        println!("Menu: enter");
    }
    fn update(&mut self, ctx: &mut StateCtx, _dt: f32) {
        let actions = ctx.resources.get::<ActionMap>().unwrap();
        if actions.action("start").just_pressed() {
            println!("Menu -> Gameplay");
            ctx.resources.get_mut::<StateStack>().unwrap()
                .replace(Box::new(GameplayState));
        }
    }
    fn on_exit(&mut self, _ctx: &mut StateCtx) {
        println!("Menu: exit");
    }
}

impl GameState for GameplayState {
    fn on_enter(&mut self, _ctx: &mut StateCtx) {
        println!("Gameplay: enter");
    }
    fn update(&mut self, _ctx: &mut StateCtx, _dt: f32) {}
    fn on_exit(&mut self, _ctx: &mut StateCtx) {
        println!("Gameplay: exit");
    }
}

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Bind actions
        let actions = app.resources_mut().get_mut::<ActionMap>().unwrap();
        actions.bind_key("start", KeyCode::Space);

        // Push initial state
        let stack = app.resources_mut().get_mut::<StateStack>().unwrap();
        stack.push(Box::new(MenuState));
    }
}

fn main() {
    AppBuilder::new()
        .add_plugin(FrameworkPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(ActionPlugin)
        .add_plugin(GamePlugin)
        .build()
        .run();
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: All crates compile without errors.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml examples/basic/
git commit -m "feat(examples): Phase 2 demo with GameState, egui, and Action mapping"
```

---

## Spec Coverage

| Spec Section | Task |
|---|---|
| GameState trait | Task 3 (state.rs) |
| StateStack (pending ops) | Task 3 (stack.rs) |
| StateCtx | Task 3 (ctx.rs) |
| FrameworkPlugin + FrameworkResource | Task 3 (plugin.rs, resource.rs) |
| EguiIntegration | Task 4 (integration.rs) |
| EguiPlugin | Task 4 (plugin.rs) |
| Egui rendering | Task 4 (plugin.rs) + Task 2 (engine.rs update) |
| ActionMap | Task 1 (action_map.rs) |
| Binding | Task 1 (binding.rs) |
| ActionPlugin | Task 2 (plugins.rs) |
| Engine-core integration (Renderer Resource, InputManager Resource, hooks) | Task 2 (app.rs, engine.rs) |
| Workspace + example | Task 5 |
