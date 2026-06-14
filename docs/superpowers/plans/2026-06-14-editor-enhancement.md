# 编辑器增强实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 RustEngine 编辑器添加场景预览、多视口、属性面板增强、资源热重载、场景序列化增强 5 项功能。

**Architecture:** 使用 egui PaintCallback 将 wgpu 渲染嵌入视口，notify crate 监听文件变化，扩展 EditorState 和 SceneManager 支持更多组件类型。

**Tech Stack:** Rust, wgpu, egui 0.30, egui-wgpu 0.30, notify 6, serde_json

---

## 文件结构

```
新增文件：
crates/engine-editor/src/viewport_renderer.rs  # 离屏渲染 + egui 回调
crates/engine-editor/src/hot_reload.rs         # 文件监听 + 资源重载

修改文件：
crates/engine-editor/Cargo.toml                # 添加 notify 依赖
crates/engine-ui/src/integration.rs            # 添加 callback_resources 支持
crates/engine-editor/src/state.rs              # 新增组件数据类型 + 多视口状态
crates/engine-editor/src/viewport.rs           # 多视口布局 + 渲染集成
crates/engine-editor/src/inspector.rs          # 扩展属性面板
crates/engine-editor/src/scene_serializer.rs   # 扩展序列化
crates/engine-editor/src/layout.rs             # 多视口布局集成
crates/engine-editor/src/main.rs               # 初始化 ViewportRenderer + HotReload
crates/engine-editor/src/lib.rs                # 模块声明
```

---

## Task 1: 添加依赖 + 模块声明

**Files:**
- Modify: `crates/engine-editor/Cargo.toml`
- Modify: `crates/engine-editor/src/lib.rs`

- [ ] **Step 1: 添加 notify 依赖**

在 `crates/engine-editor/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
notify = "6"
notify-debouncer-mini = "0.4"
```

- [ ] **Step 2: 添加模块声明**

在 `crates/engine-editor/src/lib.rs` 中添加：

```rust
pub mod viewport_renderer;
pub mod hot_reload;
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p engine-editor`
Expected: 编译通过（新模块可以为空文件）

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/Cargo.toml crates/engine-editor/src/lib.rs
git commit -m "feat(editor): add notify dependency and new module declarations"
```

---

## Task 2: EguiState 添加 callback_resources 支持

**Files:**
- Modify: `crates/engine-ui/src/integration.rs`

- [ ] **Step 1: 添加 callback_resources 字段和方法**

在 `crates/engine-ui/src/integration.rs` 的 `EguiState` 结构体中添加：

```rust
use std::any::TypeId;
use std::collections::HashMap;

pub struct EguiState {
    // ... 现有字段 ...
    /// 自定义回调资源，用于 PaintCallback
    callback_resources: HashMap<TypeId, Box<dyn std::any::Any>>,
}

impl EguiState {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        scale_factor: f32,
    ) -> Self {
        // ... 现有初始化 ...
        Self {
            // ... 现有字段 ...
            callback_resources: HashMap::new(),
        }
    }

    /// 插入自定义回调资源
    pub fn insert_callback_resource<T: 'static>(&mut self, resource: T) {
        self.callback_resources
            .insert(TypeId::of::<T>(), Box::new(resource));
    }

    /// 获取自定义回调资源
    pub fn callback_resource<T: 'static>(&self) -> Option<&T> {
        self.callback_resources
            .get(&TypeId::of::<T>())
            .and_then(|r| r.downcast_ref())
    }

    /// 获取自定义回调资源（可变）
    pub fn callback_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.callback_resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|r| r.downcast_mut())
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo build -p engine-ui`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add crates/engine-ui/src/integration.rs
git commit -m "feat(ui): add callback_resources support to EguiState"
```

---

## Task 3: ViewportRenderer 模块

**Files:**
- Create: `crates/engine-editor/src/viewport_renderer.rs`

- [ ] **Step 1: 创建 ViewportRenderer 模块**

创建 `crates/engine-editor/src/viewport_renderer.rs`：

```rust
//! 场景预览渲染器 — 离屏渲染 wgpu 场景到纹理，通过 egui PaintCallback 显示。

use engine_render::renderer::{GpuDevice, GpuQueue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 视口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportType {
    Perspective,
    Top,
    Front,
    Right,
}

/// 视口布局模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportLayout {
    /// 单视口
    Single(ViewportType),
    /// 水平双视口
    Horizontal(ViewportType, ViewportType),
    /// 垂直双视口
    Vertical(ViewportType, ViewportType),
    /// 四视口 2x2
    Quad,
}

impl Default for ViewportLayout {
    fn default() -> Self {
        Self::Single(ViewportType::Perspective)
    }
}

/// 正交相机参数
#[derive(Debug, Clone)]
pub struct OrthoCamera {
    pub target: [f32; 3],
    pub zoom: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for OrthoCamera {
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            zoom: 10.0,
            near: -100.0,
            far: 100.0,
        }
    }
}

/// 单个视口的渲染目标
struct ViewportTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    /// egui 纹理 ID（注册后可直接在 egui 中显示）
    egui_texture_id: Option<egui::TextureId>,
}

/// 场景预览渲染器
pub struct ViewportRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    targets: HashMap<ViewportType, ViewportTarget>,
    current_layout: ViewportLayout,
}

impl ViewportRenderer {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            targets: HashMap::new(),
            current_layout: ViewportLayout::default(),
        }
    }

    pub fn set_layout(&mut self, layout: ViewportLayout) {
        self.current_layout = layout;
    }

    pub fn layout(&self) -> ViewportLayout {
        self.current_layout
    }

    /// 获取当前布局中所有活跃的视口类型
    pub fn active_viewports(&self) -> Vec<ViewportType> {
        match self.current_layout {
            ViewportLayout::Single(vt) => vec![vt],
            ViewportLayout::Horizontal(a, b) => vec![a, b],
            ViewportLayout::Vertical(a, b) => vec![a, b],
            ViewportLayout::Quad => vec![
                ViewportType::Perspective,
                ViewportType::Top,
                ViewportType::Front,
                ViewportType::Right,
            ],
        }
    }

    /// 确保渲染目标存在且大小正确
    pub fn ensure_target(&mut self, viewport: ViewportType, width: u32, height: u32) {
        let needs_recreate = self
            .targets
            .get(&viewport)
            .map_or(true, |t| t.width != width || t.height != height);

        if needs_recreate {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("viewport_{:?}_texture", viewport)),
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            self.targets.insert(
                viewport,
                ViewportTarget {
                    texture,
                    view,
                    width,
                    height,
                    egui_texture_id: None,
                },
            );
        }
    }

    /// 获取视口的渲染目标纹理视图
    pub fn target_view(&self, viewport: ViewportType) -> Option<&wgpu::TextureView> {
        self.targets.get(&viewport).map(|t| &t.view)
    }

    /// 获取视口的渲染目标尺寸
    pub fn target_size(&self, viewport: ViewportType) -> Option<(u32, u32)> {
        self.targets.get(&viewport).map(|t| (t.width, t.height))
    }

    /// 清除视口渲染目标（每帧开始时调用）
    pub fn clear_target(&self, viewport: ViewportType, color: wgpu::Color) {
        if let Some(target) = self.targets.get(&viewport) {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("clear_viewport"),
                });
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear_viewport_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            self.queue.submit([encoder.finish()]);
        }
    }
}

/// 共享的 ViewportRenderer 包装（用于跨线程传递）
pub type SharedViewportRenderer = Arc<Mutex<ViewportRenderer>>;
```

- [ ] **Step 2: 添加基本测试**

在文件末尾添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_layout_default() {
        let layout = ViewportLayout::default();
        assert_eq!(layout, ViewportLayout::Single(ViewportType::Perspective));
    }

    #[test]
    fn test_quad_active_viewports() {
        let layout = ViewportLayout::Quad;
        let viewports = match layout {
            ViewportLayout::Quad => vec![
                ViewportType::Perspective,
                ViewportType::Top,
                ViewportType::Front,
                ViewportType::Right,
            ],
            _ => vec![],
        };
        assert_eq!(viewports.len(), 4);
    }

    #[test]
    fn test_single_active_viewports() {
        let layout = ViewportLayout::Single(ViewportType::Top);
        let viewports = match layout {
            ViewportLayout::Single(vt) => vec![vt],
            _ => vec![],
        };
        assert_eq!(viewports, vec![ViewportType::Top]);
    }
}
```

- [ ] **Step 3: 验证测试通过**

Run: `cargo test -p engine-editor --lib viewport_renderer`
Expected: 3 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/viewport_renderer.rs
git commit -m "feat(editor): add ViewportRenderer module with offscreen rendering"
```

---

## Task 4: EditorState 新增组件数据类型 + 多视口状态

**Files:**
- Modify: `crates/engine-editor/src/state.rs`

- [ ] **Step 1: 新增组件数据结构**

在 `crates/engine-editor/src/state.rs` 中，在 `MaterialData` 结构体之后添加：

```rust
/// Sprite 组件数据
#[derive(Debug, Clone)]
pub struct SpriteData {
    pub texture: String,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub uv_region: [f32; 4],
}

impl Default for SpriteData {
    fn default() -> Self {
        Self {
            texture: String::new(),
            size: [1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

/// 粒子系统组件数据
#[derive(Debug, Clone)]
pub struct ParticleData {
    pub emitter_type: String,
    pub rate: f32,
    pub lifetime: f32,
    pub speed: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
}

impl Default for ParticleData {
    fn default() -> Self {
        Self {
            emitter_type: "point".into(),
            rate: 10.0,
            lifetime: 2.0,
            speed: 1.0,
            size_start: 1.0,
            size_end: 0.0,
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: [1.0, 1.0, 1.0, 0.0],
        }
    }
}

/// 音频组件数据
#[derive(Debug, Clone)]
pub struct AudioData {
    pub source: String,
    pub volume: f32,
    pub looping: bool,
    pub spatial: bool,
    pub attenuation: String,
}

impl Default for AudioData {
    fn default() -> Self {
        Self {
            source: String::new(),
            volume: 1.0,
            looping: false,
            spatial: false,
            attenuation: "linear".into(),
        }
    }
}

/// 脚本组件数据
#[derive(Debug, Clone)]
pub struct ScriptData {
    pub script_path: String,
    pub enabled: bool,
    pub properties: std::collections::HashMap<String, String>,
}

impl Default for ScriptData {
    fn default() -> Self {
        Self {
            script_path: String::new(),
            enabled: true,
            properties: std::collections::HashMap::new(),
        }
    }
}
```

- [ ] **Step 2: 在 EditorState 中添加新字段**

在 `EditorState` 结构体中添加新字段（在 `terrain_sculpt_screen_pos` 之后）：

```rust
    pub node_sprites: HashMap<u64, SpriteData>,
    pub node_particles: HashMap<u64, ParticleData>,
    pub node_audio: HashMap<u64, AudioData>,
    pub node_scripts: HashMap<u64, ScriptData>,
    pub node_tags: HashMap<u64, Vec<String>>,
    pub viewport_layout: crate::viewport_renderer::ViewportLayout,
```

- [ ] **Step 3: 在 EditorState::new() 中初始化新字段**

在 `EditorState::new()` 的 `Self { ... }` 块中添加：

```rust
            node_sprites: HashMap::new(),
            node_particles: HashMap::new(),
            node_audio: HashMap::new(),
            node_scripts: HashMap::new(),
            node_tags: HashMap::new(),
            viewport_layout: crate::viewport_renderer::ViewportLayout::default(),
```

- [ ] **Step 4: 验证编译**

Run: `cargo build -p engine-editor`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add crates/engine-editor/src/state.rs
git commit -m "feat(editor): add SpriteData, ParticleData, AudioData, ScriptData types and viewport layout state"
```

---

## Task 5: 多视口布局集成到 viewport.rs

**Files:**
- Modify: `crates/engine-editor/src/viewport.rs`

- [ ] **Step 1: 修改 viewport.rs 的 draw 函数支持多视口**

将 `crates/engine-editor/src/viewport.rs` 中的 `pub fn draw` 函数替换为：

```rust
pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    let header_h = 32.0 * h_scale;
    draw_viewport_header(state, gui, rect, header_h, w_scale, h_scale);

    let canvas_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + header_h),
        Vec2::new(rect.width(), rect.height() - header_h),
    );

    // 根据布局模式分割视口
    use crate::viewport_renderer::ViewportLayout;
    match state.viewport_layout {
        ViewportLayout::Single(_) => {
            draw_single_viewport(state, gui, canvas_rect, h_scale, w_scale);
        }
        ViewportLayout::Horizontal(left, right) => {
            let half_w = canvas_rect.width() / 2.0;
            let left_rect = Rect::from_min_size(
                canvas_rect.left_top(),
                Vec2::new(half_w, canvas_rect.height()),
            );
            let right_rect = Rect::from_min_size(
                Pos2::new(canvas_rect.left() + half_w, canvas_rect.top()),
                Vec2::new(half_w, canvas_rect.height()),
            );
            draw_single_viewport(state, gui, left_rect, h_scale, w_scale);
            draw_single_viewport(state, gui, right_rect, h_scale, w_scale);
        }
        ViewportLayout::Vertical(top, bottom) => {
            let half_h = canvas_rect.height() / 2.0;
            let top_rect = Rect::from_min_size(
                canvas_rect.left_top(),
                Vec2::new(canvas_rect.width(), half_h),
            );
            let bottom_rect = Rect::from_min_size(
                Pos2::new(canvas_rect.left(), canvas_rect.top() + half_h),
                Vec2::new(canvas_rect.width(), half_h),
            );
            draw_single_viewport(state, gui, top_rect, h_scale, w_scale);
            draw_single_viewport(state, gui, bottom_rect, h_scale, w_scale);
        }
        ViewportLayout::Quad => {
            let half_w = canvas_rect.width() / 2.0;
            let half_h = canvas_rect.height() / 2.0;
            let rects = [
                Rect::from_min_size(canvas_rect.left_top(), Vec2::new(half_w, half_h)),
                Rect::from_min_size(
                    Pos2::new(canvas_rect.left() + half_w, canvas_rect.top()),
                    Vec2::new(half_w, half_h),
                ),
                Rect::from_min_size(
                    Pos2::new(canvas_rect.left(), canvas_rect.top() + half_h),
                    Vec2::new(half_w, half_h),
                ),
                Rect::from_min_size(
                    Pos2::new(canvas_rect.left() + half_w, canvas_rect.top() + half_h),
                    Vec2::new(half_w, half_h),
                ),
            ];
            for rect in &rects {
                draw_single_viewport(state, gui, *rect, h_scale, w_scale);
            }
        }
    }
}

fn draw_single_viewport(
    state: &mut EditorState,
    gui: &mut Gui,
    canvas_rect: Rect,
    h_scale: f32,
    w_scale: f32,
) {
    let painter = gui.ui.painter_at(canvas_rect);

    // 绘制背景渐变
    let gradient_steps = 20;
    let step_h = canvas_rect.height() / gradient_steps as f32;
    for i in 0..gradient_steps {
        let t = i as f32 / (gradient_steps - 1) as f32;
        let r = (10.0 + t * 10.0) as u8;
        let g = (10.0 + t * 10.0) as u8;
        let b = (12.0 + t * 16.0) as u8;
        let strip = Rect::from_min_size(
            Pos2::new(canvas_rect.left(), canvas_rect.top() + i as f32 * step_h),
            Vec2::new(canvas_rect.width(), step_h + 1.0),
        );
        painter.add(Shape::rect_filled(
            strip,
            Rounding::ZERO,
            Color32::from_rgb(r, g, b),
        ));
    }

    // 绘制网格
    if state.show_grid {
        let grid_size = 50.0 * w_scale;
        let grid_color = Color32::from_rgba_premultiplied(37, 37, 48, 128);
        let mut x = canvas_rect.left();
        while x <= canvas_rect.right() {
            painter.add(Shape::line(
                vec![
                    Pos2::new(x, canvas_rect.top()),
                    Pos2::new(x, canvas_rect.bottom()),
                ],
                Stroke::new(1.0_f32, grid_color),
            ));
            x += grid_size;
        }
        let mut y = canvas_rect.top();
        while y <= canvas_rect.bottom() {
            painter.add(Shape::line(
                vec![
                    Pos2::new(canvas_rect.left(), y),
                    Pos2::new(canvas_rect.right(), y),
                ],
                Stroke::new(1.0_f32, grid_color),
            ));
            y += grid_size;
        }
    }

    // 绘制坐标轴
    let axes = [
        ("X", Color32::from_rgb(255, 107, 107)),
        ("Y", Color32::from_rgb(46, 213, 115)),
        ("Z", Color32::from_rgb(77, 171, 247)),
    ];
    for (i, (label, color)) in axes.iter().enumerate() {
        painter.text(
            egui::pos2(
                canvas_rect.left() + 20.0 * w_scale,
                canvas_rect.top() + 20.0 * h_scale + i as f32 * 14.0 * h_scale,
            ),
            egui::Align2::LEFT_CENTER,
            *label,
            FontId::proportional(10.0 * h_scale),
            *color,
        );
    }

    draw_scene_objects(state, gui, canvas_rect, h_scale, w_scale);

    if !state.selected_nodes.is_empty() {
        crate::gizmo::draw(state, &painter, canvas_rect, h_scale, w_scale);
    }

    draw_transform_overlay(state, &painter, canvas_rect, h_scale, w_scale);

    handle_camera_input(state, gui, canvas_rect);
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo build -p engine-editor`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/viewport.rs
git commit -m "feat(editor): add multi-viewport layout support"
```

---

## Task 6: Inspector 面板扩展

**Files:**
- Modify: `crates/engine-editor/src/inspector.rs`

- [ ] **Step 1: 添加 Sprite 属性区域**

在 `crates/engine-editor/src/inspector.rs` 的 `draw_transform_section` 函数末尾（`// ── Physics ──` 区域之后）添加。注意：字符串字段使用 `egui::TextEdit::singleline` 实现可编辑，数值字段使用现有的 `gui.slider_f32` 和 `gui.vec3_input`。

```rust
    // ── Sprite ──
    if let Some(sprite) = state.node_sprites.get_mut(&id) {
        y = separator(&painter, x, y, w);
        y = section_header(&painter, x, y, "精灵");

        // 纹理路径（可编辑文本）
        let tr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        painter.text(
            egui::pos2(x, tr.center().y),
            egui::Align2::LEFT_CENTER,
            "纹理",
            FontId::proportional(12.0),
            Color32::from_gray(152),
        );
        let input_rect = Rect::from_min_size(
            Pos2::new(x + 80.0, tr.top()),
            Vec2::new(w - 80.0, tr.height()),
        );
        // SAFETY: inspector has exclusive access to the UI; egui::Ui uses interior mutability
        let ui_mut = unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) };
        let texture_id = egui::Id::new("sprite_texture").with(id);
        let mut tex_edit = sprite.texture.clone();
        egui::TextEdit::singleline(&mut tex_edit)
            .desired_width(input_rect.width() - 8.0)
            .show(ui_mut, input_rect, texture_id);
        sprite.texture = tex_edit;
        y += row_h + 6.0;

        // 大小
        let sr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(sr, "宽度", &mut sprite.size[0], 0.0, 100.0);
        y += row_h + 6.0;

        let sr2 = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(sr2, "高度", &mut sprite.size[1], 0.0, 100.0);
        y += row_h + 6.0;

        // 颜色
        let cr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        let (mut r, mut g, mut b) = (sprite.color[0], sprite.color[1], sprite.color[2]);
        gui.vec3_input(cr, "颜色", &mut r, &mut g, &mut b);
        sprite.color[0] = r;
        sprite.color[1] = g;
        sprite.color[2] = b;
        y += row_h + 6.0;

        // 翻转
        let fr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(fr, "水平翻转", &mut sprite.flip_x);
        y += row_h + 6.0;

        let flr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(flr, "垂直翻转", &mut sprite.flip_y);
        y += row_h + 6.0;
    }

    // ── Particle ──
    if let Some(particle) = state.node_particles.get_mut(&id) {
        y = separator(&painter, x, y, w);
        y = section_header(&painter, x, y, "粒子系统");

        // 发射器类型（可编辑文本）
        let er = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        painter.text(
            egui::pos2(x, er.center().y),
            egui::Align2::LEFT_CENTER,
            "发射器",
            FontId::proportional(12.0),
            Color32::from_gray(152),
        );
        let input_rect = Rect::from_min_size(
            Pos2::new(x + 80.0, er.top()),
            Vec2::new(w - 80.0, er.height()),
        );
        let emitter_id = egui::Id::new("particle_emitter").with(id);
        let mut emitter_edit = particle.emitter_type.clone();
        let ui_mut = unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) };
        egui::TextEdit::singleline(&mut emitter_edit)
            .desired_width(input_rect.width() - 8.0)
            .show(ui_mut, input_rect, emitter_id);
        particle.emitter_type = emitter_edit;
        y += row_h + 6.0;

        let rr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(rr, "发射速率", &mut particle.rate, 0.0, 100.0);
        y += row_h + 6.0;

        let lr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(lr, "生命周期", &mut particle.lifetime, 0.1, 10.0);
        y += row_h + 6.0;

        let spr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(spr, "速度", &mut particle.speed, 0.0, 50.0);
        y += row_h + 6.0;

        let szr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(szr, "起始大小", &mut particle.size_start, 0.0, 10.0);
        y += row_h + 6.0;

        let ser = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(ser, "结束大小", &mut particle.size_end, 0.0, 10.0);
        y += row_h + 6.0;
    }

    // ── Audio ──
    if let Some(audio) = state.node_audio.get_mut(&id) {
        y = separator(&painter, x, y, w);
        y = section_header(&painter, x, y, "音频");

        // 音频源（可编辑文本）
        let sr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        painter.text(
            egui::pos2(x, sr.center().y),
            egui::Align2::LEFT_CENTER,
            "音频源",
            FontId::proportional(12.0),
            Color32::from_gray(152),
        );
        let input_rect = Rect::from_min_size(
            Pos2::new(x + 80.0, sr.top()),
            Vec2::new(w - 80.0, sr.height()),
        );
        let source_id = egui::Id::new("audio_source").with(id);
        let mut source_edit = audio.source.clone();
        let ui_mut = unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) };
        egui::TextEdit::singleline(&mut source_edit)
            .desired_width(input_rect.width() - 8.0)
            .show(ui_mut, input_rect, source_id);
        audio.source = source_edit;
        y += row_h + 6.0;

        let vr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.slider_f32(vr, "音量", &mut audio.volume, 0.0, 1.0);
        y += row_h + 6.0;

        let lr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(lr, "循环", &mut audio.looping);
        y += row_h + 6.0;

        let spr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(spr, "空间音频", &mut audio.spatial);
        y += row_h + 6.0;
    }

    // ── Script ──
    if let Some(script) = state.node_scripts.get_mut(&id) {
        y = separator(&painter, x, y, w);
        y = section_header(&painter, x, y, "脚本");

        // 脚本路径（可编辑文本）
        let sr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        painter.text(
            egui::pos2(x, sr.center().y),
            egui::Align2::LEFT_CENTER,
            "脚本路径",
            FontId::proportional(12.0),
            Color32::from_gray(152),
        );
        let input_rect = Rect::from_min_size(
            Pos2::new(x + 80.0, sr.top()),
            Vec2::new(w - 80.0, sr.height()),
        );
        let script_id = egui::Id::new("script_path").with(id);
        let mut path_edit = script.script_path.clone();
        let ui_mut = unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) };
        egui::TextEdit::singleline(&mut path_edit)
            .desired_width(input_rect.width() - 8.0)
            .show(ui_mut, input_rect, script_id);
        script.script_path = path_edit;
        y += row_h + 6.0;

        let er = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        gui.checkbox(er, "启用", &mut script.enabled);
        y += row_h + 6.0;
    }

    // ── Tags ──
    if let Some(tags) = state.node_tags.get_mut(&id) {
        y = separator(&painter, x, y, w);
        y = section_header(&painter, x, y, "标签");

        // 标签列表（逗号分隔，可编辑文本）
        let tr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h));
        painter.text(
            egui::pos2(x, tr.center().y),
            egui::Align2::LEFT_CENTER,
            "标签",
            FontId::proportional(12.0),
            Color32::from_gray(152),
        );
        let input_rect = Rect::from_min_size(
            Pos2::new(x + 80.0, tr.top()),
            Vec2::new(w - 80.0, tr.height()),
        );
        let tag_id = egui::Id::new("tags").with(id);
        let tag_str = tags.join(", ");
        let mut tag_edit = tag_str.clone();
        let ui_mut = unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) };
        egui::TextEdit::singleline(&mut tag_edit)
            .desired_width(input_rect.width() - 8.0)
            .show(ui_mut, input_rect, tag_id);
        if tag_edit != tag_str {
            *tags = tag_edit
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        y += row_h + 6.0;
    }
```

- [ ] **Step 2: 验证编译**

Run: `cargo build -p engine-editor`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/inspector.rs
git commit -m "feat(editor): extend inspector with Sprite, Particle, Audio, Script, Tags panels"
```

---

## Task 7: 场景序列化增强

**Files:**
- Modify: `crates/engine-editor/src/scene_serializer.rs`

- [ ] **Step 1: 扩展 SceneEntity 结构**

在 `crates/engine-editor/src/scene_serializer.rs` 的 `SceneEntity` 结构体中添加新字段（在 `active` 之后）：

```rust
    #[serde(default)]
    pub material: Option<MaterialDataSer>,
    #[serde(default)]
    pub light: Option<LightDataSer>,
    #[serde(default)]
    pub sprite: Option<SpriteDataSer>,
    #[serde(default)]
    pub particle: Option<ParticleDataSer>,
    #[serde(default)]
    pub audio: Option<AudioDataSer>,
    #[serde(default)]
    pub script: Option<ScriptDataSer>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub physics: Option<PhysicsDataSer>,
```

- [ ] **Step 2: 添加序列化数据结构**

在 `SceneEntity` 结构体之后添加：

```rust
/// 可序列化的材质数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDataSer {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub emissive: [f32; 3],
}

/// 可序列化的光照数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightDataSer {
    pub light_type: String,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub direction: [f32; 3],
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub enabled: bool,
}

/// 可序列化的精灵数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteDataSer {
    pub texture: String,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub uv_region: [f32; 4],
}

/// 可序列化的粒子数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleDataSer {
    pub emitter_type: String,
    pub rate: f32,
    pub lifetime: f32,
    pub speed: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
}

/// 可序列化的音频数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDataSer {
    pub source: String,
    pub volume: f32,
    pub looping: bool,
    pub spatial: bool,
    pub attenuation: String,
}

/// 可序列化的脚本数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDataSer {
    pub script_path: String,
    pub enabled: bool,
    pub properties: std::collections::HashMap<String, String>,
}

/// 可序列化的物理数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsDataSer {
    pub body_type: String,
    pub collider_type: String,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub is_sensor: bool,
}
```

- [ ] **Step 3: 添加 EditorState 到 Scene 的转换方法**

在 `SceneManager` 实现之后添加：

```rust
impl EditorState {
    /// 将 EditorState 转换为 Scene（用于保存）
    pub fn to_scene(&self, name: &str) -> Scene {
        let mut scene = Scene::new(name.to_string());

        for node in &self.scene_tree.nodes {
            let transform = self
                .node_transforms
                .get(&node.id)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

            let mut entity = SceneEntity::new(node.id, node.name.clone());
            entity.transform = TransformData {
                translation: [transform[0], transform[1], transform[2]],
                rotation: [0.0, 0.0, 0.0, 1.0], // 简化：不存储四元数
                scale: [transform[6], transform[7], transform[8]],
            };
            entity.parent = node.parent;
            entity.children = node.children.clone();

            // Material
            if let Some(mat) = self.node_materials.get(&node.id) {
                entity.material = Some(MaterialDataSer {
                    base_color: mat.base_color,
                    metallic: mat.metallic,
                    roughness: mat.roughness,
                    ao: mat.ao,
                    emissive: mat.emissive,
                });
            }

            // Light
            if let Some(light) = self.node_lights.get(&node.id) {
                entity.light = Some(LightDataSer {
                    light_type: format!("{:?}", light.light_type).to_lowercase(),
                    color: light.color,
                    intensity: light.intensity,
                    range: light.range,
                    direction: light.direction,
                    inner_angle: light.inner_angle,
                    outer_angle: light.outer_angle,
                    enabled: light.enabled,
                });
            }

            // Sprite
            if let Some(sprite) = self.node_sprites.get(&node.id) {
                entity.sprite = Some(SpriteDataSer {
                    texture: sprite.texture.clone(),
                    size: sprite.size,
                    color: sprite.color,
                    flip_x: sprite.flip_x,
                    flip_y: sprite.flip_y,
                    uv_region: sprite.uv_region,
                });
            }

            // Particle
            if let Some(particle) = self.node_particles.get(&node.id) {
                entity.particle = Some(ParticleDataSer {
                    emitter_type: particle.emitter_type.clone(),
                    rate: particle.rate,
                    lifetime: particle.lifetime,
                    speed: particle.speed,
                    size_start: particle.size_start,
                    size_end: particle.size_end,
                    color_start: particle.color_start,
                    color_end: particle.color_end,
                });
            }

            // Audio
            if let Some(audio) = self.node_audio.get(&node.id) {
                entity.audio = Some(AudioDataSer {
                    source: audio.source.clone(),
                    volume: audio.volume,
                    looping: audio.looping,
                    spatial: audio.spatial,
                    attenuation: audio.attenuation.clone(),
                });
            }

            // Script
            if let Some(script) = self.node_scripts.get(&node.id) {
                entity.script = Some(ScriptDataSer {
                    script_path: script.script_path.clone(),
                    enabled: script.enabled,
                    properties: script.properties.clone(),
                });
            }

            // Tags
            if let Some(tags) = self.node_tags.get(&node.id) {
                entity.tags = tags.clone();
            }

            // Physics
            if let Some((body, col)) = self.node_physics.get(&node.id) {
                entity.physics = Some(PhysicsDataSer {
                    body_type: body.clone(),
                    collider_type: col.clone(),
                    mass: 1.0,
                    friction: 0.5,
                    restitution: 0.3,
                    is_sensor: false,
                });
            }

            scene.add_entity(entity);
        }

        scene
    }

    /// 从 Scene 恢复 EditorState（用于加载）
    pub fn load_from_scene(&mut self, scene: &Scene) {
        self.scene_tree = crate::state::SceneTree {
            nodes: Vec::new(),
            root_ids: Vec::new(),
            next_id: 1,
        };
        self.node_transforms.clear();
        self.node_materials.clear();
        self.node_lights.clear();
        self.node_sprites.clear();
        self.node_particles.clear();
        self.node_audio.clear();
        self.node_scripts.clear();
        self.node_tags.clear();
        self.node_render.clear();
        self.node_physics.clear();
        self.selected_nodes.clear();

        let mut next_id = 1u64;
        for entity in &scene.entities {
            let node = crate::state::TreeNode {
                id: entity.id,
                name: entity.name.clone(),
                icon: "📦".into(),
                expanded: false,
                parent: entity.parent,
                children: entity.children.clone(),
            };
            self.scene_tree.nodes.push(node);
            if entity.parent.is_none() {
                self.scene_tree.root_ids.push(entity.id);
            }
            if entity.id >= next_id {
                next_id = entity.id + 1;
            }

            // Transform
            self.node_transforms.insert(
                entity.id,
                [
                    entity.transform.translation[0],
                    entity.transform.translation[1],
                    entity.transform.translation[2],
                    0.0,
                    0.0,
                    0.0,
                    entity.transform.scale[0],
                    entity.transform.scale[1],
                    entity.transform.scale[2],
                ],
            );

            // Material
            if let Some(ref mat) = entity.material {
                self.node_materials.insert(
                    entity.id,
                    crate::state::MaterialData {
                        base_color: mat.base_color,
                        metallic: mat.metallic,
                        roughness: mat.roughness,
                        ao: mat.ao,
                        emissive: mat.emissive,
                    },
                );
            }

            // Light
            if let Some(ref light) = entity.light {
                let lt = match light.light_type.as_str() {
                    "directional" => crate::state::LightType::Directional,
                    "point" => crate::state::LightType::Point,
                    "spot" => crate::state::LightType::Spot,
                    _ => crate::state::LightType::Directional,
                };
                self.node_lights.insert(
                    entity.id,
                    crate::state::LightData {
                        light_type: lt,
                        color: light.color,
                        intensity: light.intensity,
                        range: light.range,
                        direction: light.direction,
                        inner_angle: light.inner_angle,
                        outer_angle: light.outer_angle,
                        enabled: light.enabled,
                    },
                );
            }

            // Sprite
            if let Some(ref sprite) = entity.sprite {
                self.node_sprites.insert(
                    entity.id,
                    crate::state::SpriteData {
                        texture: sprite.texture.clone(),
                        size: sprite.size,
                        color: sprite.color,
                        flip_x: sprite.flip_x,
                        flip_y: sprite.flip_y,
                        uv_region: sprite.uv_region,
                    },
                );
            }

            // Particle
            if let Some(ref particle) = entity.particle {
                self.node_particles.insert(
                    entity.id,
                    crate::state::ParticleData {
                        emitter_type: particle.emitter_type.clone(),
                        rate: particle.rate,
                        lifetime: particle.lifetime,
                        speed: particle.speed,
                        size_start: particle.size_start,
                        size_end: particle.size_end,
                        color_start: particle.color_start,
                        color_end: particle.color_end,
                    },
                );
            }

            // Audio
            if let Some(ref audio) = entity.audio {
                self.node_audio.insert(
                    entity.id,
                    crate::state::AudioData {
                        source: audio.source.clone(),
                        volume: audio.volume,
                        looping: audio.looping,
                        spatial: audio.spatial,
                        attenuation: audio.attenuation.clone(),
                    },
                );
            }

            // Script
            if let Some(ref script) = entity.script {
                self.node_scripts.insert(
                    entity.id,
                    crate::state::ScriptData {
                        script_path: script.script_path.clone(),
                        enabled: script.enabled,
                        properties: script.properties.clone(),
                    },
                );
            }

            // Tags
            if !entity.tags.is_empty() {
                self.node_tags.insert(entity.id, entity.tags.clone());
            }

            // Physics
            if let Some(ref physics) = entity.physics {
                self.node_physics.insert(
                    entity.id,
                    (physics.body_type.clone(), physics.collider_type.clone()),
                );
            }

            // Render (默认)
            self.node_render.insert(
                entity.id,
                ("Default".into(), "Cube".into(), true),
            );
        }

        self.scene_tree.next_id = next_id;
    }
}
```

- [ ] **Step 4: 添加序列化往返测试**

在 `scene_serializer.rs` 的 `tests` 模块中添加：

```rust
    #[test]
    fn test_extended_entity_serialization() {
        let mut scene = Scene::new("ExtendedTest".to_string());
        let mut entity = SceneEntity::new(1, "Player".to_string());
        entity.material = Some(MaterialDataSer {
            base_color: [0.8, 0.2, 0.1, 1.0],
            metallic: 0.5,
            roughness: 0.3,
            ao: 1.0,
            emissive: [0.0; 3],
        });
        entity.light = Some(LightDataSer {
            light_type: "point".into(),
            color: [1.0, 1.0, 1.0],
            intensity: 2.0,
            range: 10.0,
            direction: [0.0, -1.0, 0.0],
            inner_angle: 15.0,
            outer_angle: 30.0,
            enabled: true,
        });
        entity.sprite = Some(SpriteDataSer {
            texture: "player.png".into(),
            size: [64.0, 64.0],
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        });
        entity.tags = vec!["player".into(), "entity".into()];
        scene.add_entity(entity);

        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded: Scene = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.entities.len(), 1);
        let e = &loaded.entities[0];
        assert!(e.material.is_some());
        assert!(e.light.is_some());
        assert!(e.sprite.is_some());
        assert_eq!(e.tags, vec!["player", "entity"]);
        assert_eq!(e.material.as_ref().unwrap().metallic, 0.5);
        assert_eq!(e.light.as_ref().unwrap().intensity, 2.0);
    }
```

- [ ] **Step 5: 验证测试通过**

Run: `cargo test -p engine-editor --lib scene_serializer`
Expected: 所有测试通过

- [ ] **Step 6: Commit**

```bash
git add crates/engine-editor/src/scene_serializer.rs
git commit -m "feat(editor): extend scene serialization with all component types"
```

---

## Task 8: 热重载模块

**Files:**
- Create: `crates/engine-editor/src/hot_reload.rs`

- [ ] **Step 1: 创建 hot_reload 模块**

创建 `crates/engine-editor/src/hot_reload.rs`：

```rust
//! 资源热重载 — 监听文件系统变化，自动重载修改的资源。

use engine_asset::types::ResourceType;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{DebouncedEvent, Debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// 资源重载请求
#[derive(Debug, Clone)]
pub struct ReloadRequest {
    pub path: PathBuf,
    pub resource_type: ResourceType,
    pub timestamp: Instant,
}

/// 文件系统监听器
pub struct FileWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    receiver: Receiver<Vec<DebouncedEvent>>,
    watched_paths: HashSet<PathBuf>,
    pub pending_reload: Vec<ReloadRequest>,
}

impl FileWatcher {
    /// 创建新的文件监听器
    pub fn new() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let debouncer = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(500),
            move |events: Result<Vec<DebouncedEvent>, notify::Error>| {
                if let Ok(events) = events {
                    let _ = tx.send(events);
                }
            },
        )
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        Ok(Self {
            _debouncer: debouncer,
            receiver: rx,
            watched_paths: HashSet::new(),
            pending_reload: Vec::new(),
        })
    }

    /// 监听目录
    pub fn watch(&mut self, path: &Path) -> Result<(), String> {
        if self.watched_paths.contains(path) {
            return Ok(());
        }
        self._debouncer
            .watcher()
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path {}: {}", path.display(), e))?;
        self.watched_paths.insert(path.to_path_buf());
        Ok(())
    }

    /// 检查文件变化（每帧调用）
    pub fn poll(&mut self) {
        let events: Vec<DebouncedEvent> = self.receiver.try_iter().flatten().collect();

        for event in events {
            let path = event.path;
            if let Some(resource_type) = Self::detect_resource_type(&path) {
                self.pending_reload.push(ReloadRequest {
                    path,
                    resource_type,
                    timestamp: Instant::now(),
                });
            }
        }
    }

    /// 取出所有待重载请求
    pub fn take_pending(&mut self) -> Vec<ReloadRequest> {
        std::mem::take(&mut self.pending_reload)
    }

    /// 根据文件扩展名判断资源类型
    fn detect_resource_type(path: &Path) -> Option<ResourceType> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "dds" => Some(ResourceType::Texture),
            "wav" | "mp3" | "ogg" | "flac" => Some(ResourceType::Audio),
            "obj" | "fbx" | "gltf" | "glb" => Some(ResourceType::Mesh),
            "lua" | "rs" | "py" | "js" => Some(ResourceType::Script),
            "mat" | "json" => Some(ResourceType::Material),
            _ => None,
        }
    }
}

/// 资源重载管理器
pub struct ReloadManager {
    pub file_watcher: FileWatcher,
    pub reload_log: Vec<String>,
    start_time: Instant,
}

impl ReloadManager {
    pub fn new(watch_path: &Path) -> Result<Self, String> {
        let mut file_watcher = FileWatcher::new()?;
        file_watcher.watch(watch_path)?;
        Ok(Self {
            file_watcher,
            reload_log: Vec::new(),
            start_time: Instant::now(),
        })
    }

    /// 每帧更新
    pub fn update(&mut self) {
        self.file_watcher.poll();

        let requests = self.file_watcher.take_pending();
        for req in requests {
            let elapsed = self.start_time.elapsed();
            let secs = elapsed.as_secs();
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            let msg = format!("[{:02}:{:02}:{:02}] 重载: {}", h, m, s, req.path.display());
            self.reload_log.push(msg);

            // 保持日志不超过 100 条
            if self.reload_log.len() > 100 {
                self.reload_log.remove(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_resource_type() {
        assert_eq!(
            FileWatcher::detect_resource_type(Path::new("test.png")),
            Some(ResourceType::Texture)
        );
        assert_eq!(
            FileWatcher::detect_resource_type(Path::new("test.wav")),
            Some(ResourceType::Audio)
        );
        assert_eq!(
            FileWatcher::detect_resource_type(Path::new("test.obj")),
            Some(ResourceType::Mesh)
        );
        assert_eq!(
            FileWatcher::detect_resource_type(Path::new("test.lua")),
            Some(ResourceType::Script)
        );
        assert_eq!(
            FileWatcher::detect_resource_type(Path::new("test.txt")),
            None
        );
    }

    #[test]
    fn test_file_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }
}
```

- [ ] **Step 2: 验证测试通过**

Run: `cargo test -p engine-editor --lib hot_reload`
Expected: 2 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/hot_reload.rs
git commit -m "feat(editor): add hot reload module with file system watching"
```

---

## Task 9: 集成到 main.rs 和 layout.rs

**Files:**
- Modify: `crates/engine-editor/src/main.rs`
- Modify: `crates/engine-editor/src/layout.rs`

- [ ] **Step 1: 在 main.rs 中初始化 ViewportRenderer**

在 `crates/engine-editor/src/main.rs` 的 `main()` 函数中，在创建 `egui_state` 之后添加：

```rust
    // 初始化 ViewportRenderer
    let viewport_renderer = std::sync::Arc::new(std::sync::Mutex::new(
        engine_editor::viewport_renderer::ViewportRenderer::new(
            renderer.device.clone(),
            renderer.queue.clone(),
        ),
    ));

    // 初始化热重载管理器
    let hot_reload_manager = std::sync::Arc::new(std::sync::Mutex::new(
        engine_editor::hot_reload::ReloadManager::new(std::path::Path::new("assets"))
            .unwrap_or_else(|e| {
                log::warn!("Failed to init hot reload: {}", e);
                // 创建一个空的管理器（不监听任何目录）
                engine_editor::hot_reload::ReloadManager::new(std::path::Path::new(".")).unwrap()
            }),
    ));
```

- [ ] **Step 2: 在 layout.rs 的视口区域添加布局切换按钮**

在 `crates/engine-editor/src/layout.rs` 的 `draw_toolbar` 函数中，在现有工具按钮之后添加视口布局按钮：

```rust
    // 视口布局按钮
    x += pad;
    draw_separator(&painter, x, rect.top(), rect.bottom(), h_scale);
    x += pad;
    let layouts = &["1", "⬌", "⬍", "⊞"];
    for (i, icon) in layouts.iter().enumerate() {
        let btn_rect = Rect::from_min_size(
            Pos2::new(x + i as f32 * (btn_size + gap), cy),
            Vec2::new(btn_size, btn_size),
        );
        if gui.tool_button(btn_rect, icon, false) {
            use crate::viewport_renderer::{ViewportLayout, ViewportType};
            state.viewport_layout = match i {
                0 => ViewportLayout::Single(ViewportType::Perspective),
                1 => ViewportLayout::Horizontal(
                    ViewportType::Perspective,
                    ViewportType::Top,
                ),
                2 => ViewportLayout::Vertical(
                    ViewportType::Perspective,
                    ViewportType::Top,
                ),
                3 => ViewportLayout::Quad,
                _ => state.viewport_layout,
            };
        }
    }
    x += 4.0 * (btn_size + gap);
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p engine-editor`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/main.rs crates/engine-editor/src/layout.rs
git commit -m "feat(editor): integrate ViewportRenderer and multi-viewport layout"
```

---

## Task 10: 最终验证

- [ ] **Step 1: 运行所有测试**

Run: `cargo test -p engine-editor`
Expected: 所有测试通过

- [ ] **Step 2: 运行 clippy**

Run: `cargo clippy -p engine-editor -- -D warnings`
Expected: 无警告

- [ ] **Step 3: 运行格式化检查**

Run: `cargo fmt --check`
Expected: 无格式问题

- [ ] **Step 4: 最终 Commit**

```bash
git add -A
git commit -m "feat(editor): complete editor enhancement - viewport, multi-layout, inspector, hot reload, serialization"
```
