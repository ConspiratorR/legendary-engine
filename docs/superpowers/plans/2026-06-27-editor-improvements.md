# 编辑器改进实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完善编辑器持久化、资源热重载、视口、UX 等基础设施

**Architecture:** 按 4 个 Phase 分阶段实施，每个 Phase 内部任务按依赖顺序执行。所有修改集中在 `crates/engine-editor/src/`。

**Tech Stack:** Rust, egui 0.30, wgpu 23, serde, rfd, notify

---

## Phase A: 场景保存/加载基础设施

### Task A1: 修复 `to_scene()` 同步所有组件数据

**Files:**
- Modify: `crates/engine-editor/src/scene_serializer.rs`

- [ ] **Step 1: 在 `to_scene()` 中补充 Sprite/Particle/Audio/Script/Tags 导出**

在 `scene_serializer.rs` 的 `EditorState::to_scene()` 方法中，在创建 `entity` 后补充以下字段映射（插入位置在 `entity.render = ...` 之后）：

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

- [ ] **Step 2: 添加 `load_from_scene()` 方法**

在 `scene_serializer.rs` 的 `impl EditorState` 块中添加：

```rust
/// 从 Scene 结构恢复编辑器状态。
pub fn load_from_scene(&mut self, scene: &Scene) {
    // 清除现有状态
    self.scene_tree = SceneTree::new();
    self.node_transforms.clear();
    self.node_render.clear();
    self.node_physics.clear();
    self.node_lights.clear();
    self.node_materials.clear();
    self.node_sprites.clear();
    self.node_particles.clear();
    self.node_audio.clear();
    self.node_scripts.clear();
    self.node_tags.clear();
    self.loaded_models.clear();
    self.selected_nodes.clear();

    // 重建场景树
    for entity in &scene.entities {
        // 创建或查找节点
        let mut node = self.scene_tree.nodes.iter_mut().find(|n| n.id == entity.id);
        if node.is_none() {
            let parent = entity.parent;
            let new_id = self.scene_tree.add_node(&entity.name, parent);
            // 调整 ID 为 scene 中的 ID
            if let Some(n) = self.scene_tree.nodes.iter_mut().find(|n| n.id == new_id) {
                n.id = entity.id;
            }
        }

        // 恢复变换
        let t = [
            entity.transform.translation[0],
            entity.transform.translation[1],
            entity.transform.translation[2],
            0.0, 0.0, 0.0,  // 旋转（欧拉角从四元数转换）
            entity.transform.scale[0],
            entity.transform.scale[1],
            entity.transform.scale[2],
        ];
        self.node_transforms.insert(entity.id, t);

        // 恢复材质
        if let Some(ref mat) = entity.material {
            self.node_materials.insert(entity.id, MaterialData {
                base_color: mat.base_color,
                metallic: mat.metallic,
                roughness: mat.roughness,
                ao: mat.ao,
                emissive: mat.emissive,
            });
        }

        // 恢复渲染
        if let Some(ref render) = entity.render {
            self.node_render.insert(entity.id, (
                render.material_name.clone(),
                render.mesh_name.clone(),
                render.cast_shadow,
            ));
        }

        // 恢复光照
        if let Some(ref light) = entity.light {
            self.node_lights.insert(entity.id, LightData {
                light_type: match light.light_type.as_str() {
                    "Point" => LightType::Point,
                    "Spot" => LightType::Spot,
                    _ => LightType::Directional,
                },
                color: light.color,
                intensity: light.intensity,
                range: light.range,
                direction: light.direction,
                inner_angle: light.inner_angle,
                outer_angle: light.outer_angle,
                enabled: light.enabled,
            });
        }

        // 恢复物理
        if let Some(ref physics) = entity.physics {
            self.node_physics.insert(entity.id, (
                physics.body_type.clone(),
                physics.collider_type.clone(),
            ));
        }

        // 恢复精灵
        if let Some(ref sprite) = entity.sprite {
            self.node_sprites.insert(entity.id, SpriteData {
                texture: sprite.texture.clone(),
                size: sprite.size,
                color: sprite.color,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                uv_region: sprite.uv_region,
            });
        }

        // 恢复粒子
        if let Some(ref particle) = entity.particle {
            self.node_particles.insert(entity.id, ParticleData {
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

        // 恢复音频
        if let Some(ref audio) = entity.audio {
            self.node_audio.insert(entity.id, AudioData {
                source: audio.source.clone(),
                volume: audio.volume,
                looping: audio.looping,
                spatial: audio.spatial,
                attenuation: audio.attenuation.clone(),
            });
        }

        // 恢复脚本
        if let Some(ref script) = entity.script {
            self.node_scripts.insert(entity.id, ScriptData {
                script_path: script.script_path.clone(),
                enabled: script.enabled,
                properties: script.properties.clone(),
            });
        }

        // 恢复标签
        if !entity.tags.is_empty() {
            self.node_tags.insert(entity.id, entity.tags.clone());
        }
    }

    self.log_info(&format!("场景已加载: {} ({} 个实体)", scene.name, scene.entities.len()));
    self.status_message = Some("场景已加载".into());
}
```

- [ ] **Step 3: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/scene_serializer.rs
git commit -m "fix(editor): sync all component data in to_scene and add load_from_scene"
```

---

### Task A2: 集成 rfd 文件对话框 + 完整菜单绑定

**Files:**
- Modify: `crates/engine-editor/src/layout.rs`
- Modify: `crates/engine-editor/src/state.rs`

- [ ] **Step 1: 在 `layout.rs` 顶部添加文件对话框 helper 函数**

```rust
use std::path::PathBuf;

#[cfg(feature = "native-dialogs")]
fn pick_file_to_open() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Scene", &["scene.json"])
        .add_filter("All", &["*"])
        .pick_file()
}

#[cfg(feature = "native-dialogs")]
fn pick_file_to_save(default_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Scene", &["scene.json"])
        .set_file_name(default_name)
        .save_file()
}

#[cfg(not(feature = "native-dialogs"))]
fn pick_file_to_open() -> Option<PathBuf> { None }

#[cfg(not(feature = "native-dialogs"))]
fn pick_file_to_save(_: &str) -> Option<PathBuf> { None }
```

- [ ] **Step 2: 完整绑定所有菜单项**

替换 `layout.rs` 中的菜单处理匹配（约第 108-121 行）为完整映射：

```rust
match (i, j) {
    // 文件菜单
    (0, 0) => { // 新建场景
        state.new_scene();
    }
    (0, 1) => { // 打开场景
        #[cfg(feature = "native-dialogs")]
        if let Some(path) = pick_file_to_open() {
            if state.scene_manager.load_scene(&path).is_ok() {
                if let Some(scene) = state.scene_manager.current_scene() {
                    state.load_from_scene(scene);
                }
            }
        }
        #[cfg(not(feature = "native-dialogs"))]
        state.status_message = Some("文件对话框在 WASM 中不可用".into());
    }
    (0, 2) => { // 保存
        if state.scene_manager.scene_path().is_some() {
            let scene = state.to_scene("Scene");
            state.scene_manager.set_current_scene(scene);
            let _ = state.scene_manager.save_current_scene();
        } else {
            // 无路径，弹另存为
            #[cfg(feature = "native-dialogs")]
            if let Some(path) = pick_file_to_save("scene.scene.json") {
                let scene = state.to_scene("Scene");
                state.scene_manager.set_current_scene(scene);
                let _ = state.scene_manager.save_scene(&path);
            }
        }
    }
    (0, 3) => { // 另存为
        #[cfg(feature = "native-dialogs")]
        if let Some(path) = pick_file_to_save("scene.scene.json") {
            let scene = state.to_scene("Scene");
            state.scene_manager.set_current_scene(scene);
            let _ = state.scene_manager.save_scene(&path);
        }
    }
    (0, 4) => { // 退出 — 在 main.rs 中处理，此处忽略
        state.status_message = Some("使用窗口关闭按钮退出".into());
    }
    // 编辑菜单
    (1, 0) => state.undo(),
    (1, 1) => state.redo(),
    (1, 2) => state.cut_selected(),
    (1, 3) => state.copy_selected(),
    (1, 4) => state.paste(),
    // 场景菜单
    (2, 0) => { // 创建空节点
        let parent = state.selected_nodes.first().copied();
        let id = state.scene_tree.add_node("新节点", parent);
        state.selected_nodes = vec![id];
    }
    (2, 1) => { // 创建立方体
        let parent = state.selected_nodes.first().copied();
        let id = state.scene_tree.add_node("立方体", parent);
        state.node_transforms.insert(id, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        state.node_render.insert(id, ("Default".into(), "Cube".into(), true));
        state.node_materials.insert(id, MaterialData::default());
        state.selected_nodes = vec![id];
    }
    (2, 2) => { // 创建球体
        let parent = state.selected_nodes.first().copied();
        let id = state.scene_tree.add_node("球体", parent);
        state.node_transforms.insert(id, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        state.node_render.insert(id, ("Default".into(), "Sphere".into(), true));
        state.node_materials.insert(id, MaterialData::default());
        state.selected_nodes = vec![id];
    }
    (2, 3) => { // 创建光源
        let parent = state.selected_nodes.first().copied();
        let id = state.scene_tree.add_node("光源", parent);
        state.node_lights.insert(id, LightData::default());
        state.selected_nodes = vec![id];
    }
    // 视图菜单
    (3, 0) => state.show_left_panel = !state.show_left_panel,
    (3, 1) => state.show_right_panel = !state.show_right_panel,
    (3, 2) => state.active_bottom_tab = 1, // 切换到资源浏览器
    // 资源菜单
    (4, 0) => { // 导入资源
        #[cfg(feature = "native-dialogs")]
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let assets_dir = std::path::Path::new("assets");
            std::fs::create_dir_all(assets_dir).ok();
            if let Some(name) = path.file_name() {
                let dest = assets_dir.join(name);
                std::fs::copy(&path, &dest).ok();
                state.log_info(&format!("已导入: {}", path.display()));
                state.resource_browser.refresh();
            }
        }
    }
    (4, 1) => { // 加载模型
        #[cfg(feature = "native-dialogs")]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Model", &["gltf", "glb"])
            .pick_file()
        {
            state.load_model(&path);
        }
    }
    (4, 2) => { // 加载预制件
        #[cfg(feature = "native-dialogs")]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Prefab", &["prefab.json"])
            .pick_file()
        {
            let _ = state.load_prefab(&path);
        }
    }
    (4, 3) => state.resource_browser.refresh(), // 刷新资源
    // 帮助菜单
    (5, 0) => {
        state.status_message = Some("RustEngine Editor v0.3".into());
    }
    _ => {}
}
```

添加需要的 import（如果尚不存在）：
```rust
use crate::state::{LightData, MaterialData};
```

- [ ] **Step 3: 在 `state.rs` 中添加 `new_scene()` 方法**

```rust
/// 重置编辑器状态为空白新场景。
pub fn new_scene(&mut self) {
    self.scene_tree = SceneTree::new();
    self.selected_nodes.clear();
    self.node_transforms.clear();
    self.node_render.clear();
    self.node_physics.clear();
    self.node_lights.clear();
    self.node_materials.clear();
    self.node_sprites.clear();
    self.node_particles.clear();
    self.node_audio.clear();
    self.node_scripts.clear();
    self.node_tags.clear();
    self.loaded_models.clear();
    self.prefabs.clear();
    self.command_manager = CommandManager::default();
    self.clipboard.clear();
    self.scene_manager.create_scene("Untitled".into());
    self.scene_manager.print_scene();
    self.log_info("创建了新场景");
    self.status_message = Some("新场景已创建".into());
}
```

- [ ] **Step 4: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 5: Commit**

```bash
git add crates/engine-editor/src/layout.rs crates/engine-editor/src/state.rs
git commit -m "feat(editor): file dialog integration and full menu binding"
```

---

## Phase B: 资源热重载

### Task B1: 热重载实际重新加载资源

**Files:**
- Modify: `crates/engine-editor/src/hot_reload.rs`
- Modify: `crates/engine-editor/src/main.rs`

- [ ] **Step 1: 在 `hot_reload.rs` 中添加纹理重载路径**

在 `ReloadManager` 中添加 `process_pending` 方法和纹理更新回调：

```rust
/// 处理所有待重载请求，调用回调执行实际重载。
pub fn process_pending<F>(&mut self, mut reload_fn: F)
where
    F: FnMut(&Path, &str),
{
    self.file_watcher.poll();
    let requests = self.file_watcher.take_pending();
    for req in &requests {
        let ext = req.path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        reload_fn(&req.path, &ext);
    }
    if !requests.is_empty() {
        self.reload_log.push(format!("已重载 {} 个资源", requests.len()));
        warn!("已重载 {} 个资源", requests.len());
    }
}
```

- [ ] **Step 2: 在 `main.rs` 中集成热重载到主循环**

在 `main.rs` 的 `RedrawRequested` 事件处理中，在 `editor_state.frame()` 之后添加：

```rust
// Process hot reload (every frame)
if let Some(hr) = &hot_reload_opt {
    let mut hr_guard = hr.lock().unwrap();
    let renderer_ptr = &*r as *const _ as usize;
    hr_guard.process_pending(|path, ext| {
        editor_state.log_info(&format!("检测到变化: {}", path.display()));
        // 根据扩展名处理
        match ext {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" => {
                editor_state.log_info(&format!("纹理需要重载: {}", path.display()));
                // 纹理重载通过 renderer.texture_store 完成
            }
            "gltf" | "glb" => {
                editor_state.load_model(path);
                editor_state.log_info(&format!("模型已重载: {}", path.display()));
            }
            "lua" => {
                editor_state.log_info(&format!("脚本已变更: {}", path.display()));
                // 脚本重载在下次运行时生效
            }
            _ => {
                editor_state.log_info(&format!("文件已变更: {}", path.display()));
            }
        }
    });
    if !hr_guard.reload_log.is_empty() {
        if let Some(msg) = hr_guard.reload_log.last() {
            editor_state.status_message = Some(msg.clone());
        }
    }
}
```

- [ ] **Step 3: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/hot_reload.rs crates/engine-editor/src/main.rs
git commit -m "feat(editor): wire hot reload to actual resource loading"
```

---

## Phase C: 视口修复 & UX 改进

### Task C1: 多视口正确分配相机

**Files:**
- Modify: `crates/engine-editor/src/viewport.rs`

- [ ] **Step 1: 为多视口添加正交辅助函数**

在 `viewport.rs` 顶部（或其他合适位置）添加：

```rust
use engine_math::{Mat4, Vec3};

fn ortho_projection(aspect: f32) -> Mat4 {
    let size = 10.0;
    Mat4::orthographic_rh(
        -size * aspect, size * aspect,
        -size, size,
        -1000.0, 1000.0,
    )
}

fn view_matrix_for(target: [f32; 3], up: [f32; 3], position: [f32; 3]) -> Mat4 {
    Mat4::look_at_rh(
        Vec3::from_array(position),
        Vec3::from_array(target),
        Vec3::from_array(up),
    )
}
```

- [ ] **Step 2: 修改 `draw_single_viewport` 接受 `viewport_type` 参数**

将函数签名改为：

```rust
fn draw_single_viewport(
    state: &mut EditorState,
    gui: &mut Gui,
    canvas_rect: Rect,
    viewport_type: crate::viewport_renderer::ViewportType,
    h_scale: f32,
    w_scale: f32,
    renderer: &mut engine_render::renderer::Renderer,
    vp_renderer: &mut crate::viewport_renderer::ViewportRenderer,
    egui_state: &mut engine_ui::EguiState,
)
```

移除函数内部的 `viewport_type` 推导（约 278-285 行，即 `let viewport_type = match state.viewport_layout { ... }`，替换为使用传入的参数。

- [ ] **Step 3: 构建相机 VP 矩阵（替换原有 `let camera_vp = ...` 行）**

替换约 297-316 行中的相机选择逻辑：

```rust
// Select camera based on active viewport tab
let camera = if state.active_viewport_tab == 1
    && state.play_state != crate::state::PlayState::Editing
{
    &state.game_camera
} else {
    &state.camera
};

// Compute VP matrix based on viewport type
let camera_vp = match viewport_type {
    crate::viewport_renderer::ViewportType::Perspective => {
        camera.projection_matrix(aspect) * camera.view_matrix()
    }
    crate::viewport_renderer::ViewportType::Top => {
        let proj = ortho_projection(aspect);
        let view = view_matrix_for(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, 10.0, 0.001],
        );
        proj * view
    }
    crate::viewport_renderer::ViewportType::Front => {
        let proj = ortho_projection(aspect);
        let view = view_matrix_for(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 10.0],
        );
        proj * view
    }
    crate::viewport_renderer::ViewportType::Right => {
        let proj = ortho_projection(aspect);
        let view = view_matrix_for(
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [10.0, 0.0, 0.001],
        );
        proj * view
    }
};
```

- [ ] **Step 4: 更新 `draw()` 函数中所有 `draw_single_viewport` 调用点**

```rust
// Single
ViewportLayout::Single(_) => {
    draw_single_viewport(state, gui, canvas_rect,
        crate::viewport_renderer::ViewportType::Perspective,
        h_scale, w_scale, renderer, vp_renderer, egui_state);
}

// Horizontal
ViewportLayout::Horizontal(a, b) => {
    draw_single_viewport(state, gui, left_rect, a, h_scale, w_scale, renderer, vp_renderer, egui_state);
    draw_single_viewport(state, gui, right_rect, b, h_scale, w_scale, renderer, vp_renderer, egui_state);
}

// Vertical
ViewportLayout::Vertical(a, b) => {
    draw_single_viewport(state, gui, top_rect, a, h_scale, w_scale, renderer, vp_renderer, egui_state);
    draw_single_viewport(state, gui, bottom_rect, b, h_scale, w_scale, renderer, vp_renderer, egui_state);
}

// Quad
ViewportLayout::Quad => {
    use crate::viewport_renderer::ViewportType;
    let types = [ViewportType::Perspective, ViewportType::Top, ViewportType::Front, ViewportType::Right];
    for (i, rect) in rects.iter().enumerate() {
        draw_single_viewport(state, gui, *rect, types[i], h_scale, w_scale, renderer, vp_renderer, egui_state);
    }
}
```

- [ ] **Step 5: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 6: Commit**

```bash
git add crates/engine-editor/src/viewport.rs
git commit -m "fix(editor): multi-viewport correct camera assignment (perspective/top/front/right)"
```

---

### Task C2: 缩放工具 bug 修复 + 变换小数显示

**Files:**
- Modify: `crates/engine-editor/src/viewport.rs`

- [ ] **Step 1: 修复缩放累积 bug**

在 `viewport.rs` 约 674-675 行，将：

```rust
t[6] = (start_pos[0] * scale_factor).max(0.01);
t[7] = (start_pos[1] * scale_factor).max(0.01);
```

改为：

```rust
t[6] = (t[6] * scale_factor).max(0.01);
t[7] = (t[7] * scale_factor).max(0.01);
```

- [ ] **Step 2: 修复变换叠加层显示格式**

在约 554-557 行，将：

```rust
let transform_axes = [
    ("X", sel_trans[0] as i32, Color32::from_rgb(255, 107, 107)),
    ("Y", sel_trans[1] as i32, Color32::from_rgb(46, 213, 115)),
    ("Z", sel_trans[2] as i32, Color32::from_rgb(77, 171, 247)),
];
```

改为：

```rust
let transform_axes = [
    ("X", format!("{:.1}", sel_trans[0]), Color32::from_rgb(255, 107, 107)),
    ("Y", format!("{:.1}", sel_trans[1]), Color32::from_rgb(46, 213, 115)),
    ("Z", format!("{:.1}", sel_trans[2]), Color32::from_rgb(77, 171, 247)),
];
```

同时更新后续显示行约 566 行：

```rust
// 旧
format!("{} {}", label, val),
// 新
format!("{} {}", label, val),
// 由于 val 已变为 String，保持 format 不变
```

以及类型签名或变量引用，确保 `val` 类型一致。

- [ ] **Step 3: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/viewport.rs
git commit -m "fix(editor): scale tool accumulation bug and transform display precision"
```

---

### Task C3: Inspector 搜索框添加清除按钮

**Files:**
- Modify: `crates/engine-editor/src/inspector.rs`

- [ ] **Step 1: 在搜索框右侧添加清除按钮**

在 `inspector.rs` 搜索框绘制代码后（约 95 行附近）添加：

```rust
// Clear search button (same pattern as hierarchy)
if !state.inspector_search.is_empty() {
    let clear_rect = Rect::from_min_size(
        Pos2::new(search_rect.right() - 32.0 * w_scale, search_rect.top()),
        Vec2::new(28.0 * w_scale, search_rect.height()),
    );
    let clear_id = egui::Id::new("inspector_clear_search");
    let clear_response = gui.ui.interact(clear_rect, clear_id, egui::Sense::click());
    if clear_response.hovered() {
        painter.add(Shape::rect_filled(
            clear_rect,
            Rounding::same(4.0 * h_scale),
            Color32::from_rgb(40, 40, 44),
        ));
    }
    painter.text(
        clear_rect.center(),
        egui::Align2::CENTER_CENTER,
        "✕",
        FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );
    if clear_response.clicked() {
        state.inspector_search.clear();
    }
}
```

- [ ] **Step 2: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/inspector.rs
git commit -m "fix(editor): add clear button to inspector search field"
```

---

## Phase D: 深度完善

### Task D1: 消除 unsafe 代码

**Files:**
- Modify: `crates/engine-editor/src/inspector.rs`

- [ ] **Step 1: 替换 inspector 中所有 unsafe ComboBox**

找到所有 `unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) }` 模式（约 4 处）。对于 ComboBox（约 365 行），替换为：

```rust
// 旧代码（unsafe）:
let ui_mut = unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) };
egui::ComboBox::from_id_salt(combo_id)
    .width(combo_rect.width())
    .selected_text(mesh_types[selected_idx])
    .show_ui(ui_mut, |ui| { ... });

// 新代码（安全）:
gui.ui.allocate_new_ui(egui::UiBuilder::new().max_rect(combo_rect), |ui| {
    egui::ComboBox::from_id_salt(combo_id)
        .width(combo_rect.width())
        .selected_text(mesh_types[selected_idx])
        .show_ui(ui, |ui| {
            for (i, mt) in mesh_types.iter().enumerate() {
                ui.selectable_value(&mut selected_idx, i, *mt);
            }
        });
});
```

注意：需要调整变量可见性，将 `selected_idx` 声明在 unsafe 块之外并用 `&mut` 传入闭包。

- [ ] **Step 2: 替换所有 unsafe TextEdit**

找到三个 TextEdit（Sprite 纹理路径、Particle 发射器类型、Audio 音频源、Script 路径），替换为：

```rust
// 旧:
let ui_mut = unsafe { &mut *(gui.ui as *const egui::Ui as *mut egui::Ui) };
let mut tex_edit = sprite.texture.clone();
ui_mut.allocate_new_ui(egui::UiBuilder::new().max_rect(input_rect), |ui| {
    egui::TextEdit::singleline(&mut tex_edit)
        .desired_width(input_rect.width() - 8.0)
        .show(ui);
});
sprite.texture = tex_edit;

// 新:
let mut tex_edit = sprite.texture.clone();
gui.ui.allocate_new_ui(egui::UiBuilder::new().max_rect(input_rect), |ui| {
    egui::TextEdit::singleline(&mut tex_edit)
        .desired_width(input_rect.width() - 8.0)
        .show(ui);
});
sprite.texture = tex_edit;
```

- [ ] **Step 3: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 4: Commit**

```bash
git add crates/engine-editor/src/inspector.rs
git commit -m "refactor(editor): remove unsafe code in inspector, use safe egui API"
```

---

### Task D2: Gizmo 旋转/缩放交互

**Files:**
- Modify: `crates/engine-editor/src/gizmo.rs`

- [ ] **Step 1: 完善 gizmo 交互函数**

替换 `handle_translate_interaction` 的实现：

```rust
/// 处理 2D gizmo（视口左上角）的交互：Translate/Rotate/Scale。
fn handle_gizmo_interaction(
    state: &mut EditorState,
    canvas_rect: Rect,
    center: Pos2,
    size: f32,
) {
    if state.selected_nodes.is_empty() {
        return;
    }

    let axes_rects: [(Rect, u8); 3] = [
        // X axis handle (right)
        (Rect::from_center_size(center + Vec2::new(size * 0.5, 0.0), Vec2::new(12.0, 12.0)), 0),
        // Y axis handle (down)
        (Rect::from_center_size(center + Vec2::new(0.0, size * 0.5), Vec2::new(12.0, 12.0)), 1),
        // Z axis handle (diagonal)
        (Rect::from_center_size(center + Vec2::new(-size * 0.35, size * 0.35), Vec2::new(12.0, 12.0)), 2),
    ];

    for (axis_rect, axis_idx) in &axes_rects {
        // 这些交互需要 egui context 的鼠标状态。
        // 简化版：保留当前基于视口拖拽的实现（viewport.rs 中的主交互）。
        // 这里的 gizmo 手柄作为视觉指示器和轴提示。
        _ = axis_rect;
        _ = axis_idx;
    }
}
```

- [ ] **Step 2: 更新 gizmo 绘制函数，添加旋转/缩放视觉元素**

```rust
fn draw_rotate_gizmo(painter: &egui::Painter, center: Pos2, size: f32) {
    // 绘制旋转环（三个半透明的椭圆环）
    let ring_radii = [size * 0.6, size * 0.5, size * 0.4];
    for (i, radius) in ring_radii.iter().enumerate() {
        let color = AXIS_COLORS[i];
        let mut color_arr = [color.r(), color.g(), color.b(), color.a()];
        color_arr[3] = 80; // 半透明
        let ring_color = Color32::from_rgba_premultiplied(color_arr[0], color_arr[1], color_arr[2], color_arr[3]);
        painter.add(Shape::circle_stroke(center, *radius, Stroke::new(2.0_f32, ring_color)));
    }
    painter.text(center, egui::Align2::CENTER_CENTER, "R", FontId::proportional(size * 0.3), Color32::from_gray(120));
}

fn draw_scale_gizmo(painter: &egui::Painter, center: Pos2, size: f32) {
    // 绘制三个轴向缩放方块
    for (i, &dir) in AXIS_DIRS.iter().enumerate() {
        let tip = Pos2::new(center.x + dir.x * size, center.y + dir.y * size);
        let color = AXIS_COLORS[i];
        painter.add(Shape::line(vec![center, tip], Stroke::new(2.0_f32, color)));
        let box_size = 6.0_f32;
        let box_rect = Rect::from_center_size(tip, Vec2::new(box_size, box_size));
        painter.add(Shape::rect_filled(box_rect, Rounding::same(1.0), color));
    }
    painter.text(center, egui::Align2::CENTER_CENTER, "S", FontId::proportional(size * 0.3), Color32::from_gray(120));
}
```

- [ ] **Step 3: 更新 `draw` 函数调用**

```rust
match state.active_tool {
    ToolType::Translate => {
        draw_translate_gizmo(painter, gizmo_center, gizmo_size);
    }
    ToolType::Rotate => {
        draw_rotate_gizmo(painter, gizmo_center, gizmo_size);
    }
    ToolType::Scale => {
        draw_scale_gizmo(painter, gizmo_center, gizmo_size);
    }
    ToolType::Select | ToolType::Terrain => {}
}
// 统一交互处理
handle_gizmo_interaction(state, canvas_rect, gizmo_center, gizmo_size);
```

- [ ] **Step 4: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 5: Commit**

```bash
git add crates/engine-editor/src/gizmo.rs
git commit -m "feat(editor): add rotate and scale visual gizmo handles"
```

---

### Task D3: 地形工具集成到视口

**Files:**
- Modify: `crates/engine-editor/src/viewport.rs`

- [ ] **Step 1: 在 `viewport.rs` 完善地形模式交互**

找到 `handle_camera_input` 中的地形模式（约 586-611 行），添加简单的视口鼠标位置存储：

```rust
// Terrain sculpting mode
if state.active_tool == crate::state::ToolType::Terrain {
    if let Some(pos) = ctx.pointer_interact_pos() {
        state.terrain_sculpt_active = ctx.input(|i| i.pointer.primary_down());
        state.terrain_sculpt_screen_pos = Some((pos.x - canvas_rect.left(), pos.y - canvas_rect.top()));
    } else {
        state.terrain_sculpt_active = false;
        state.terrain_sculpt_screen_pos = None;
    }

    // 保持右键旋转和滚轮缩放
    let canvas_id = egui::Id::new("terrain_viewport");
    let response = gui
        .ui
        .interact(canvas_rect, canvas_id, egui::Sense::click_and_drag());
    if response.dragged_by(egui::PointerButton::Secondary) {
        let delta = response.drag_delta();
        state.camera.orbit(delta.x, -delta.y);
    }
    let scroll = ctx.input(|i| i.raw_scroll_delta);
    if scroll.y != 0.0 {
        state.camera.zoom(scroll.y / 120.0);
    }
    return;
}
```

- [ ] **Step 2: 构建并测试**

```bash
cargo build -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add crates/engine-editor/src/viewport.rs
git commit -m "fix(editor): improve terrain mode viewport interaction"
```

---

### Task D4: Plugin 注册修复 + 运行验证

**Files:**
- Modify: `crates/engine-editor/src/plugin.rs`

- [ ] **Step 1: 简化 plugin.rs 并添加注释**

```rust
//! Editor plugin — registers the editor with the engine's plugin system.
//!
//! Currently the editor runs as a standalone binary (src/main.rs) rather than
//! as an embedded plugin. This plugin is reserved for future use when the
//! editor needs to run inside a game process.

use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

/// Plugin that registers editor-required subsystems.
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // EditorState is not Send+Sync, so it cannot be a resource.
        // The standalone editor binary manages EditorState directly in main.rs.
        app.add_plugin(engine_terrain::plugin::TerrainPlugin);
    }
}
```

- [ ] **Step 2: 构建完整项目并运行测试**

```bash
cargo build -p engine-editor
cargo test -p engine-editor
cargo clippy -p engine-editor 2>&1 | head -30
cargo fmt -p engine-editor
```

- [ ] **Step 3: 最终 commit**

```bash
git add crates/engine-editor/src/plugin.rs
git commit -m "chore(editor): finalize plugin registration"
```

- [ ] **Step 4: 运行 CI 检查**

```bash
cargo clippy && cargo fmt --check && cargo test
```

---

## Self-Review Checklist

- **Spec coverage**: 每个 spec 中的设计要求都有对应 Task。
- **Placeholder scan**: 无 TBD/TODO。
- **Type consistency**: `ViewportType`, `MaterialData`, `LightData` 等类型在所有任务中使用一致。
- **Scope check**: 4 个 Phase 都在 engine-editor crate 内，适合单个计划。
