# 编辑器持久化 & 资源热重载设计

**日期**: 2026-06-27
**状态**: 设计中
**模块**: engine-editor

## 概述

分 4 个阶段完善编辑器功能，按依赖链排列：

| Phase | 内容 | 依赖 |
|-------|------|------|
| A | 场景保存/加载基础设施 | 无（基础） |
| B | 资源热重载实际执行 | A 的热重载通知 |
| C | 视口修复 & UX 改进 | A-B |
| D | 深度完善（unsafe、gizmo、地形） | A-C |

本文档覆盖 Phase A~D 的全部设计。

---

## Phase A: 场景保存/加载基础设施

### A1. 修复 `to_scene()` 同步所有组件数据

**问题**: `EditorState::to_scene()` (`scene_serializer.rs:463`) 只导出了 Transform/Material/Light/Render/Physics，遗漏了 Sprite/Particle/Audio/Script/Tags。

**方案**: 在 `to_scene()` 中为每个 node 补充：

```rust
// Sprite
entity.sprite = self.node_sprites.get(&node.id).map(|s| SpriteDataSer {
    texture: s.texture.clone(),
    size: s.size,
    color: s.color,
    flip_x: s.flip_x,
    flip_y: s.flip_y,
    uv_region: s.uv_region,
});

// Particle
entity.particle = self.node_particles.get(&node.id).map(|p| ParticleDataSer {
    emitter_type: p.emitter_type.clone(),
    rate: p.rate,
    lifetime: p.lifetime,
    speed: p.speed,
    size_start: p.size_start,
    size_end: p.size_end,
    color_start: p.color_start,
    color_end: p.color_end,
});

// Audio
entity.audio = self.node_audio.get(&node.id).map(|a| AudioDataSer {
    source: a.source.clone(),
    volume: a.volume,
    looping: a.looping,
    spatial: a.spatial,
    attenuation: a.attenuation.clone(),
});

// Script
entity.script = self.node_scripts.get(&node.id).map(|s| ScriptDataSer {
    script_path: s.script_path.clone(),
    enabled: s.enabled,
    properties: s.properties.clone(),
});

// Tags
entity.tags = self.node_tags.get(&node.id).cloned().unwrap_or_default();
```

**新增反向方法** `EditorState::load_from_scene(scene: &Scene)`:
遍历 `scene.entities`，恢复所有节点数据到各 HashMap。同时重建 `SceneTree`。

**修改文件**: `scene_serializer.rs`

### A2. 集成 rfd 文件对话框

**修改文件**: `layout.rs`

在菜单处理中添加文件对话框调用：

```rust
(0, 1 /* 打开场景 */) => {
    #[cfg(feature = "native-dialogs")]
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Scene", &["scene.json"])
        .pick_file()
    {
        // 加载场景
        if let Ok(scene) = state.scene_manager.load_scene(&path) {
            state.load_from_scene(&scene);
            state.log_info(&format!("场景已加载: {}", path.display()));
        }
    }
}
(0, 3 /* 另存为 */) => {
    #[cfg(feature = "native-dialogs")]
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Scene", &["scene.json"])
        .set_file_name("scene.scene.json")
        .save_file()
    {
        let scene = state.to_scene("Scene");
        state.scene_manager.set_current_scene(scene);
        let _ = state.scene_manager.save_scene(&path);
        state.log_info(&format!("场景已保存: {}", path.display()));
    }
}
```

同时使用 `#[cfg(feature = "native-dialogs")]` 条件编译，WASM 环境下跳过。

### A3. 完整绑定菜单项

**修改文件**: `layout.rs`、`state.rs`

菜单 action 映射表：

| 菜单路径 | case | 行为 |
|----------|------|------|
| 文件→新建 | `(0,0)` | 重置场景树 + 清空所有组件 HashMap + `scene_manager.create_scene()` |
| 文件→打开 | `(0,1)` | 文件对话框 → 加载 |
| 文件→保存 | `(0,2)` | 路径存在 → `save_current_scene()`；不存在 → 同另存为 |
| 文件→另存为 | `(0,3)` | 文件对话框 → 保存 |
| 文件→退出 | `(0,4)` | 检查修改 → 确认 → `elwt.exit()` |
| 编辑→撤销 | `(1,0)` | `state.undo()` |
| 编辑→重做 | `(1,1)` | `state.redo()` |
| 编辑→剪切 | `(1,2)` | `state.cut_selected()` |
| 编辑→复制 | `(1,3)` | `state.copy_selected()` |
| 编辑→粘贴 | `(1,4)` | `state.paste()` |
| 场景→创建空节点 | `(2,0)` | 创建空节点 |
| 场景→创建立方体 | `(2,1)` | 创建立方体 |
| 场景→创建球体 | `(2,2)` | 创建球体 |
| 场景→创建光源 | `(2,3)` | 创建方向光 |
| 视图→层级面板 | `(3,0)` | 切换 show_left_panel |
| 视图→检视面板 | `(3,1)` | 切换 show_right_panel |
| 视图→资源浏览器 | `(3,2)` | 切换底部面板 tab |
| 资源→导入资源 | `(4,0)` | 文件对话框选择文件 → 复制到 assets/ |
| 资源→加载模型 | `(4,1)` | 文件对话框选择 glTF → `state.load_model()` |
| 资源→加载预制件 | `(4,2)` | 文件对话框选择 prefab → `state.load_prefab()` |
| 资源→刷新资源 | `(4,3)` | `state.resource_browser.refresh()` |
| 帮助→关于 | `(5,0)` | 显示版本信息弹窗 |

新增 `EditorState::new_scene()` 方法，重置所有数据。

---

## Phase B: 资源热重载

### 问题

当前 `ReloadManager` (`hot_reload.rs`) 检测文件变化后只记录日志，不执行实际的资源重新加载：

- 纹理变化 → 不重新创建 GPU 纹理
- 网格变化 → 不重新上传顶点缓冲
- 材质变化 → 不重新编译

### 方案

在 `ReloadManager` 中添加重载处理回调：

```rust
pub struct ReloadManager {
    file_watcher: FileWatcher,
    reload_log: Vec<String>,
    start_time: Instant,
    /// 待处理的资源路径列表
    pending_reloads: Vec<PathBuf>,
}
```

每帧在编辑器主循环中调用 `reload_manager.process_pending(renderer, &mut editor_state)`：

```rust
pub fn process_pending(&mut self, renderer: &mut Renderer, state: &mut EditorState) {
    self.file_watcher.poll();
    for req in self.file_watcher.take_pending() {
        let ext = req.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" => {
                // 重新加载纹理到 GPU（通过 renderer 的纹理存储）
                self.reload_texture(&req.path, renderer, state);
            }
            "gltf" | "glb" => {
                // 重新加载模型
                state.load_model(&req.path);
                self.reload_log.push(format!("模型已重载: {}", req.path.display()));
            }
            "lua" | "wasm" => {
                // 标记脚本需要重新加载
                self.reload_log.push(format!("脚本已变更: {}", req.path.display()));
            }
            _ => {}
        }
    }
}
```

**纹理重载**：由于编辑器中纹理通过 `renderer.texture_store` 管理，需要暴露纹理更新接口。

**修改文件**: `hot_reload.rs`、`main.rs`

状态栏显示："已重载: texture.png"

---

## Phase C: 视口修复 & UX 改进

### C1. 多视口正确分配相机

**修改文件**: `viewport.rs`

当前 `draw_single_viewport` 从 `state.viewport_layout` 推导视口类型，Quad 模式下永远是 Perspective。

**方案**: 将视口类型作为参数传入 `draw_single_viewport`：

```rust
// Quad 模式下
let viewport_types = [
    ViewportType::Perspective,
    ViewportType::Top,
    ViewportType::Front,
    ViewportType::Right,
];
for (i, rect) in rects.iter().enumerate() {
    draw_single_viewport(state, gui, *rect, viewport_types[i], ...);
}
```

在 Top/Front/Right 模式下使用正交投影：

```rust
let camera_vp = match viewport_type {
    ViewportType::Perspective | _ if game_tab => {
        camera.projection_matrix(aspect) * camera.view_matrix()
    }
    ViewportType::Top => {
        ortho_projection(aspect) * top_view_matrix()
    }
    ViewportType::Front => {
        ortho_projection(aspect) * front_view_matrix()
    }
    ViewportType::Right => {
        ortho_projection(aspect) * right_view_matrix()
    }
};
```

新增正交相机构建函数：

```rust
fn ortho_projection(aspect: f32) -> Mat4 {
    let size = 10.0;
    Mat4::orthographic_rh(
        -size * aspect, size * aspect,
        -size, size,
        -1000.0, 1000.0,
    )
}
```

### C2. 缩放工具 bug 修复

**修改文件**: `viewport.rs:674`

```rust
// 修复前
t[6] = (start_pos[0] * scale_factor).max(0.01);
t[7] = (start_pos[1] * scale_factor).max(0.01);

// 修复后
t[6] = (t[6] * scale_factor).max(0.01);
t[7] = (t[7] * scale_factor).max(0.01);
```

### C3. 变换叠加层显示小数

**修改文件**: `viewport.rs:555`

```rust
// 修复前
("X", sel_trans[0] as i32, ...)
// 修复后
("X", format!("{:.1}", sel_trans[0]), ...)
```

格式改为 `format!("{} {:.1}", label, val)`。

### C4. Inspector 搜索框添加清除按钮

**修改文件**: `inspector.rs`

在搜索框右侧添加"✕"按钮，同 hierarchy 实现。

---

## Phase D: 深度完善

### D1. 消除 unsafe 代码

**问题**: 多处使用 `unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) }` 绕过借用检查。

**方案**: 使用 `egui::UiBuilder` + `allocate_ui` 模式获取嵌套 Ui，或通过 `gui.ui.child_ui` 方法获取安全的子 Ui 引用。

对于 ComboBox（`inspector.rs:365`），使用 `ui.add(egui::ComboBox::...)` 而非 `unsafe` 指针转换。

对于 TextEdit，使用 `gui.ui.text_edit_singleline(...)` 的返回值更新数据。

### D2. Gizmo 交互完善

**问题**: 视口左上角的 Gizmo UI 只显示 Translate 箭头且不可交互。

**方案**: 
- Translate gizmo: 当前通过视口拖拽实现（viewport.rs:619-687），保持
- Rotate gizmo: 添加旋转环交互，鼠标拖拽映射到绕 Y 和 X 轴旋转
- Scale gizmo: 添加缩放交互，鼠标拖拽计算统一缩放因子

### D3. 地形工具集成到视口

**问题**: 地形模式只在检查器显示浮动面板，不实际雕刻。

**方案**: 在 `viewport.rs` 的地形检测中，将鼠标位置映射到地形坐标，修改地形高度图。

### D4. Plugin 注册修复

**问题**: `EditorPlugin` 未注册编辑器系统。

**方案**: 如果未来需要嵌入式编辑器模式，将 `EditorState` 包装成 `Send + Sync` 并注册为资源。

---

## 质量保障

- 保存/加载往返测试：保存场景到临时文件 → 加载 → 验证所有数据一致
- 菜单绑定调用对应 state 方法
- Hot reload 测试：修改文件 → 检测到事件 → 触发回调

## 新增/修改文件

```
Phase A:
  modify: crates/engine-editor/src/scene_serializer.rs  # to_scene 补充 + load from scene
  modify: crates/engine-editor/src/state.rs             # new_scene() 方法
  modify: crates/engine-editor/src/layout.rs             # 完整菜单绑定 + rfd 集成

Phase B:
  modify: crates/engine-editor/src/hot_reload.rs         # process_pending 实际重载
  modify: crates/engine-editor/src/main.rs               # 每帧调用 process_pending

Phase C:
  modify: crates/engine-editor/src/viewport.rs           # 多视口相机 + 缩放修复 + 小数
  modify: crates/engine-editor/src/inspector.rs           # 搜索清除按钮

Phase D:
  modify: crates/engine-editor/src/inspector.rs           # 消除 unsafe
  modify: crates/engine-editor/src/viewport.rs            # 地形交互
  modify: crates/engine-editor/src/gizmo.rs               # 旋转/缩放交互
  modify: crates/engine-editor/src/plugin.rs              # 注册修复
```
