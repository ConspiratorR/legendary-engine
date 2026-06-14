# 编辑器增强设计

**日期**: 2026-06-14
**状态**: 设计中
**模块**: engine-editor, engine-render, engine-ui

## 概述

为 RustEngine 编辑器添加 5 项增强功能：
1. **场景预览** — 把 wgpu 渲染结果嵌入 egui 视口（方案 B：egui 自定义渲染回调）
2. **多视口** — 支持 2x2 分屏（透视/顶/前/右）
3. **属性面板增强** — 补全更多组件属性编辑
4. **资源热重载** — 监听文件变化自动刷新
5. **场景序列化增强** — 完整保存所有组件数据

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 渲染集成 | egui CallbackFn | 零拷贝，直接在 egui 渲染流程中插入 wgpu 渲染 |
| 多视口布局 | 2x2 网格 | 标准编辑器布局，支持单视口/双视口/四视口切换 |
| 热重载方案 | notify crate | 跨平台文件监听，社区成熟 |
| 序列化格式 | JSON | 已有基础，扩展即可 |

## 架构

```
用户操作
    │
    ├─ 视口交互 ──→ ViewportRenderer ──→ wgpu 渲染 ──→ egui CallbackFn ──→ 屏幕
    │                    │
    │                    ├─ 主视口（透视）
    │                    ├─ 顶视图
    │                    ├─ 前视图
    │                    └─ 右视图
    │
    ├─ 属性编辑 ──→ InspectorPanel ──→ EditorState 更新
    │                    │
    │                    ├─ Transform
    │                    ├─ Material (PBR)
    │                    ├─ Light
    │                    ├─ Physics
    │                    ├─ Sprite
    │                    ├─ Particle
    │                    ├─ Audio
    │                    ├─ Script
    │                    └─ Tags/Layers
    │
    ├─ 资源修改 ──→ FileWatcher ──→ 资源重载 ──→ GPU 更新
    │
    └─ 保存/加载 ──→ SceneManager ──→ 完整 JSON 序列化
```

## 1. 场景预览（ViewportRenderer）

### 核心思路

使用 egui 0.30 的 `PaintCallback` + `CallbackFn` 在 egui 渲染流程中插入自定义 wgpu 渲染。

### 新增文件

```
crates/engine-editor/src/viewport_renderer.rs
```

### ViewportRenderer 结构

```rust
pub struct ViewportRenderer {
    /// 每个视口的渲染目标纹理
    render_targets: HashMap<ViewportId, wgpu::Texture>,
    /// 每个视口的纹理视图
    texture_views: HashMap<ViewportId, wgpu::TextureView>,
    /// 视口相机
    cameras: HashMap<ViewportId, ViewportCamera>,
    /// 渲染管线（复用现有 Renderer 的管线）
    /// 通过 CallbackFn 获取 Renderer 引用
}
```

### egui 集成方式

egui 0.30 的 `PaintCallback` 运行在 egui 的 wgpu 渲染 pass 内部。回调通过 `Painter` 访问底层 `RenderPass`，可以插入自定义渲染命令。

```rust
// 在视口面板中
let callback = egui::PaintCallback {
    rect: viewport_rect,
    callback: std::sync::Arc::new(egui::CallbackFn::new(move |info, painter| {
        // painter 提供对 wgpu RenderPass 的访问
        // 通过 painter.render_pass() 获取当前 pass
        // 或通过 painter.callback_resources 获取共享资源
    })),
};
ui.painter().add(callback);
```

**关键实现细节**：

由于 CallbackFn 是 `Fn` 闭包，不能捕获 `&mut Renderer`。解决方案：

1. 使用 `Arc<Mutex<ViewportRenderer>>` 共享渲染器引用
2. 通过 `egui_wgpu::CallbackResources` 传递自定义资源
3. 在回调中锁定渲染器并执行渲染

```rust
// 在 main.rs 中创建共享渲染器
let viewport_renderer = Arc::new(Mutex::new(ViewportRenderer::new(&device)));

// 注册到 egui 回调资源
egui_state.set_callback_resources(viewport_renderer.clone());

// 在回调中使用
let callback = egui::PaintCallback {
    rect: viewport_rect,
    callback: std::sync::Arc::new(egui::CallbackFn::new(move |info, painter| {
        let resources: &ViewportRenderer = painter.callback_resources.get().unwrap();
        let mut renderer = resources.lock().unwrap();
        renderer.render_to_target(info, &scene);
    })),
};
```

### 渲染流程

1. 每帧开始时，更新所有视口相机
2. 对每个视口：
   a. 创建离屏渲染目标（如果不存在或大小变化）
   b. 通过 CallbackFn 注册渲染回调
   c. 在 egui 渲染时，回调执行实际的 wgpu 渲染到离屏纹理
   d. 回调同时注册离屏纹理为 egui 纹理
3. egui 使用注册的纹理在视口区域绘制

### 视口相机

```rust
pub struct ViewportCamera {
    pub camera_type: ViewportType,
    pub orbit_camera: EditorCamera,  // 透视视口
    pub ortho_camera: OrthoCamera,   // 正交视口
    pub render_target_size: (u32, u32),
}

pub enum ViewportType {
    Perspective,
    Top,
    Front,
    Right,
}
```

## 2. 多视口

### 布局模式

```rust
pub enum ViewportLayout {
    Single(ViewportType),           // 单视口
    Horizontal(ViewportType, ViewportType),  // 水平双视口
    Vertical(ViewportType, ViewportType),    // 垂直双视口
    Quad,                           // 四视口 2x2
}
```

### 2x2 四视口布局

```
┌─────────────┬─────────────┐
│  透视视口    │   顶视图    │
│ (Perspective)│   (Top)     │
├─────────────┼─────────────┤
│  前视图      │   右视图    │
│  (Front)     │  (Right)    │
└─────────────┴─────────────┘
```

### 视口切换

- 工具栏添加视口布局按钮
- 快捷键切换：F1-F4 切换单视口，F5 切换四视口
- 每个视口有独立的相机和渲染目标

## 3. 属性面板增强

### 新增组件数据

```rust
// EditorState 新增字段
pub node_sprites: HashMap<u64, SpriteData>,
pub node_particles: HashMap<u64, ParticleData>,
pub node_audio: HashMap<u64, AudioData>,
pub node_scripts: HashMap<u64, ScriptData>,
pub node_tags: HashMap<u64, Vec<String>>,
```

### SpriteData

```rust
pub struct SpriteData {
    pub texture: String,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub uv_region: [f32; 4],  // [u_min, v_min, u_max, v_max]
}
```

### ParticleData

```rust
pub struct ParticleData {
    pub emitter_type: String,  // "point", "circle", "cone"
    pub rate: f32,
    pub lifetime: f32,
    pub speed: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
}
```

### AudioData

```rust
pub struct AudioData {
    pub source: String,
    pub volume: f32,
    pub looping: bool,
    pub spatial: bool,
    pub attenuation: String,  // "linear", "inverse", "exponential"
}
```

### ScriptData

```rust
pub struct ScriptData {
    pub script_path: String,
    pub enabled: bool,
    pub properties: HashMap<String, String>,
}
```

### Inspector 面板扩展

在 `inspector.rs` 中为每种组件添加属性编辑区：

- **Sprite 区域**：纹理选择、大小、颜色、翻转、UV
- **Particle 区域**：发射器类型、速率、生命周期、速度、大小渐变、颜色渐变
- **Audio 区域**：音频源、音量、循环、空间音频、衰减模型
- **Script 区域**：脚本路径、启用开关、自定义属性
- **Tags 区域**：标签列表、添加/删除标签

## 4. 资源热重载

### 新增文件

```
crates/engine-editor/src/hot_reload.rs
```

### FileWatcher 结构

```rust
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<DebouncedEvent>,
    watched_paths: HashSet<PathBuf>,
    pending_reload: Vec<ReloadRequest>,
}

pub struct ReloadRequest {
    pub path: PathBuf,
    pub resource_type: ResourceType,
    pub timestamp: std::time::Instant,
}
```

### 监听流程

1. 编辑器启动时，监听 `assets/` 目录
2. 文件变化时，收到 `DebouncedEvent::Write` 事件
3. 根据文件扩展名判断资源类型
4. 添加到待重载队列
5. 下一帧执行重载：
   - 纹理：重新加载图片，更新 GPU 纹理
   - 模型：重新加载 mesh，更新顶点缓冲
   - 材质：重新解析材质参数
   - 脚本：重新编译/解释
6. 状态栏显示重载通知

### 依赖

```toml
[dependencies]
notify = "6"
notify-debouncer-mini = "0.4"
```

## 5. 场景序列化增强

### 扩展 SceneEntity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: u64,
    pub name: String,
    pub transform: TransformData,
    pub components: Vec<ComponentData>,
    pub children: Vec<u64>,
    pub parent: Option<u64>,
    pub active: bool,
    // 新增字段
    pub material: Option<MaterialData>,
    pub light: Option<LightData>,
    pub sprite: Option<SpriteData>,
    pub particle: Option<ParticleData>,
    pub audio: Option<AudioData>,
    pub script: Option<ScriptData>,
    pub tags: Vec<String>,
    pub physics: Option<PhysicsData>,
}
```

### PhysicsData

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsData {
    pub body_type: String,  // "static", "dynamic", "kinematic"
    pub collider_type: String,  // "box", "sphere", "capsule"
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub is_sensor: bool,
}
```

### 保存流程

1. 遍历 EditorState 中所有组件数据
2. 转换为 SceneEntity 结构
3. 序列化为 JSON
4. 写入文件

### 加载流程

1. 读取 JSON 文件
2. 反序列化为 Scene 结构
3. 恢复 EditorState 中所有组件数据
4. 重建场景树

## 新增/修改文件清单

```
新增文件：
crates/engine-editor/src/viewport_renderer.rs  # 场景预览渲染器
crates/engine-editor/src/hot_reload.rs         # 资源热重载

修改文件：
crates/engine-editor/src/viewport.rs           # 集成渲染回调 + 多视口
crates/engine-editor/src/inspector.rs          # 扩展属性面板
crates/engine-editor/src/state.rs              # 新增组件数据字段
crates/engine-editor/src/scene_serializer.rs   # 扩展序列化
crates/engine-editor/src/layout.rs             # 多视口布局
crates/engine-editor/src/plugin.rs             # 注册热重载
crates/engine-editor/src/main.rs               # 初始化 ViewportRenderer
crates/engine-editor/Cargo.toml                # 添加 notify 依赖
```

## 测试策略

1. **ViewportRenderer 测试**：渲染到纹理、纹理尺寸变化
2. **多视口测试**：布局切换、相机类型
3. **Inspector 测试**：组件数据读写
4. **热重载测试**：文件变化检测、资源重载
5. **序列化测试**：完整保存/加载往返测试

## 性能考量

- 视口渲染：4 个视口 = 4 次渲染调用，但编辑器通常不需要 60fps
- 热重载：使用 debounce 避免频繁重载
- 序列化：JSON 格式足够，不需要二进制格式

## 依赖

- `notify = "6"` — 文件系统监听
- `notify-debouncer-mini = "0.4"` — 防抖
- `egui = "0.30.0"` — 已有
- `egui-wgpu = "0.30.0"` — 已有
