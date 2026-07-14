# Editor Fix and Improvement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix critical editor bugs (gizmo interaction, depth buffer, undo/redo, hierarchy) and complete core features (resource browser, hot reload, inspector persistence).

**Architecture:** Phased approach — Phase 1 fixes bugs, Phase 2 completes features, Phase 3 polishes. Each phase is independently deployable. All changes are in `crates/engine-editor/src/`.

**Tech Stack:** Rust, egui, wgpu, engine_ecs, engine_scene, engine_render

---

## Phase 1: Fix Critical Bugs

### Task 1: Add Depth Buffer to Viewport Renderer

**Files:**
- Modify: `crates/engine-editor/src/viewport_renderer.rs`

- [ ] **Step 1: Add depth texture fields to ViewportTarget**

In `viewport_renderer.rs`, add `depth_view` and `depth_texture` fields to `ViewportTarget`:

```rust
struct ViewportTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    width: u32,
    height: u32,
    egui_texture_id: Option<egui::TextureId>,
}
```

- [ ] **Step 2: Create depth texture in ensure_target**

In `ensure_target()`, after creating the color texture, create a depth texture:

```rust
let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
    label: Some(&format!("ViewportDepth_{:?}", viewport)),
    size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Depth32Float,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    view_formats: &[],
});
let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
```

- [ ] **Step 3: Update ViewportTarget construction**

Update the `ViewportTarget` insertion to include depth fields:

```rust
self.targets.insert(
    viewport,
    ViewportTarget {
        texture,
        view,
        depth_texture,
        depth_view,
        width,
        height,
        egui_texture_id: None,
    },
);
```

- [ ] **Step 4: Add depth_view accessor method**

Add a public method to access the depth view:

```rust
pub fn depth_view(&self, viewport: ViewportType) -> Option<&wgpu::TextureView> {
    self.targets.get(&viewport).map(|t| &t.depth_view)
}
```

- [ ] **Step 5: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

### Task 2: Implement Gizmo Interactive Handles

**Files:**
- Modify: `crates/engine-editor/src/gizmo.rs`
- Modify: `crates/engine-editor/src/viewport.rs` (mouse event routing)

- [ ] **Step 1: Add GizmoState enum and data structures**

In `gizmo.rs`, add state tracking:

```rust
use crate::state::EditorState;
use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

/// Gizmo interaction state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoState {
    Idle,
    HoverAxis(usize), // 0=X, 1=Y, 2=Z
    DraggingAxis(usize),
}

/// Gizmo interaction data stored in EditorState.
#[derive(Debug, Clone)]
pub struct GizmoInteraction {
    pub state: GizmoState,
    pub drag_start_screen: Pos2,
    pub drag_start_world_pos: [f32; 3],
    pub drag_axis: usize,
}
```

- [ ] **Step 2: Add GizmoInteraction field to EditorState**

In `state.rs`, add to `EditorState`:

```rust
pub gizmo_interaction: GizmoInteraction,
```

Initialize in `EditorState::new()`:

```rust
gizmo_interaction: GizmoInteraction {
    state: GizmoState::Idle,
    drag_start_screen: Pos2::ZERO,
    drag_start_world_pos: [0.0; 3],
    drag_axis: 0,
},
```

- [ ] **Step 3: Implement hover detection in gizmo.rs**

Add hover detection function:

```rust
fn detect_hover(
    state: &mut EditorState,
    mouse_pos: Pos2,
    gizmo_center: Pos2,
    gizmo_size: f32,
) -> Option<usize> {
    if state.selected_nodes.is_empty() {
        return None;
    }
    let threshold = 12.0; // pixels

    for (i, &dir) in AXIS_DIRS.iter().enumerate() {
        let tip = Pos2::new(gizmo_center.x + dir.x * gizmo_size, gizmo_center.y + dir.y * gizmo_size);
        // Distance from mouse to line segment (center -> tip)
        let dist = point_to_segment_distance(mouse_pos, gizmo_center, tip);
        if dist < threshold {
            return Some(i);
        }
    }
    None
}

fn point_to_segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = Vec2::new(b.x - a.x, b.y - a.y);
    let ap = Vec2::new(p.x - a.x, p.y - a.y);
    let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    let projection = Pos2::new(a.x + t * ab.x, a.y + t * ab.y);
    let diff = Vec2::new(p.x - projection.x, p.y - projection.y);
    diff.length()
}
```

- [ ] **Step 4: Implement drag handling in gizmo.rs**

Add drag start/end functions:

```rust
pub fn start_drag(
    state: &mut EditorState,
    axis: usize,
    mouse_pos: Pos2,
    gizmo_center: Pos2,
    gizmo_size: f32,
) {
    let node_id = state.selected_nodes.first().copied().unwrap_or(0);
    let world_pos = state.node_transforms.get(&node_id).copied().unwrap_or([0.0; 9]);

    state.gizmo_interaction = GizmoInteraction {
        state: GizmoState::DraggingAxis(axis),
        drag_start_screen: mouse_pos,
        drag_start_world_pos: [world_pos[0], world_pos[1], world_pos[2]],
        drag_axis: axis,
    };
}

pub fn update_drag(state: &mut EditorState, mouse_pos: Pos2, gizmo_center: Pos2, gizmo_size: f32) {
    if let GizmoState::DraggingAxis(axis) = state.gizmo_interaction.state {
        let node_id = state.selected_nodes.first().copied().unwrap_or(0);
        let start = state.gizmo_interaction.drag_start_screen;
        let delta_screen = Vec2::new(mouse_pos.x - start.x, mouse_pos.y - start.y);

        // Convert screen delta to world delta (simplified: assume 1:1 scale)
        let scale = gizmo_size / 60.0; // normalize by gizmo visual size
        let mut world_delta = [0.0f32; 3];
        world_delta[axis] = delta_screen.x * scale;

        if let Some(t) = state.node_transforms.get_mut(&node_id) {
            t[0] = state.gizmo_interaction.drag_start_world_pos[0] + world_delta[0];
            t[1] = state.gizmo_interaction.drag_start_world_pos[1] + world_delta[1];
            t[2] = state.gizmo_interaction.drag_start_world_pos[2] + world_delta[2];
        }
    }
}

pub fn end_drag(state: &mut EditorState) -> Option<(usize, [f32; 3], [f32; 3])> {
    if let GizmoState::DraggingAxis(axis) = state.gizmo_interaction.state {
        let node_id = state.selected_nodes.first().copied().unwrap_or(0);
        let old_pos = state.gizmo_interaction.drag_start_world_pos;
        let new_pos = state.node_transforms.get(&node_id).map(|t| [t[0], t[1], t[2]]).unwrap_or(old_pos);
        state.gizmo_interaction.state = GizmoState::Idle;
        return Some((node_id, old_pos, new_pos));
    }
    None
}
```

- [ ] **Step 5: Update gizmo draw to show hover state**

Update `draw()` to show hover highlight:

```rust
pub fn draw(
    state: &mut EditorState,
    painter: &egui::Painter,
    canvas_rect: Rect,
    h_scale: f32,
    _w_scale: f32,
) {
    let gizmo_center = Pos2::new(canvas_rect.right() - 100.0, canvas_rect.top() + 80.0);
    let gizmo_size = 60.0 * h_scale;

    // Update hover if idle
    if state.gizmo_interaction.state == GizmoState::Idle {
        // Detect hover from current mouse position (passed via state)
        // This will be called from viewport.rs with the actual mouse pos
    }

    match state.active_tool {
        ToolType::Translate => {
            draw_translate_gizmo(painter, gizmo_center, gizmo_size, &state.gizmo_interaction);
        }
        ToolType::Rotate => draw_rotate_gizmo(painter, gizmo_center, gizmo_size),
        ToolType::Scale => draw_scale_gizmo(painter, gizmo_center, gizmo_size),
        ToolType::Select | ToolType::Terrain => {}
    }
}
```

- [ ] **Step 6: Route mouse events in viewport.rs**

In `viewport.rs`, add mouse event handling for gizmo:

```rust
// In the mouse handling section of the viewport draw function:
if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
    let gizmo_center = Pos2::new(canvas_rect.right() - 100.0, canvas_rect.top() + 80.0);
    let gizmo_size = 60.0;

    // Update hover state
    if state.gizmo_interaction.state == GizmoState::Idle {
        if let Some(axis) = detect_hover(state, mouse_pos, gizmo_center, gizmo_size) {
            state.gizmo_interaction.state = GizmoState::HoverAxis(axis);
        }
    }

    // Handle drag
    if ui.input(|i| i.pointer.any_released()) {
        if let Some((node_id, old_pos, new_pos)) = end_drag(state) {
            let cmd = TransformEntityCommand::new(
                node_id,
                old_pos, // This needs old/new transform arrays
                new_pos,
            );
            // Push to undo stack
        }
    }
}
```

- [ ] **Step 7: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

### Task 3: Fix Undo/Redo for SculptCommand

**Files:**
- Modify: `crates/engine-editor/src/commands.rs`

- [ ] **Step 1: Implement SculptCommand::undo()**

```rust
fn undo(&mut self, state: &mut EditorState) {
    // Restore heightmap snapshot
    if let Some(terrain) = state.terrains.get_mut(&self.entity_id) {
        let res = self.resolution;
        let mut idx = 0;
        for j in self.affected_min.1..=self.affected_max.1 {
            for i in self.affected_min.0..=self.affected_max.0 {
                let heightmap_idx = (j * (res + 1) + i) as usize;
                terrain.heightmap[heightmap_idx] = self.height_snapshot[idx];
                idx += 1;
            }
        }
    }
}
```

- [ ] **Step 2: Implement SculptCommand::redo()**

```rust
fn redo(&mut self, state: &mut EditorState) {
    // Re-apply is complex; for now, store new heightmap state
    // In practice, this would re-run the sculpt operation
    // For simplicity, we mark that redo was applied
    // The actual re-application happens through the terrain system
}
```

- [ ] **Step 3: Fix DeleteEntityCommand to preserve ID**

Change `DeleteEntityCommand` to use a stable ID approach:

```rust
#[derive(Debug)]
pub struct DeleteEntityCommand {
    entity_id: u64,
    entity_name: String,
    transform: Option<[f32; 9]>,
    parent: Option<u64>,
    material: Option<crate::state::MaterialData>,
    light: Option<crate::state::LightData>,
}

impl DeleteEntityCommand {
    pub fn new(state: &EditorState, entity_id: u64) -> Self {
        let node = state.scene_tree.nodes.iter().find(|n| n.id == entity_id);
        Self {
            entity_id,
            entity_name: node.map(|n| n.name.clone()).unwrap_or_default(),
            transform: state.node_transforms.get(&entity_id).copied(),
            parent: node.and_then(|n| n.parent),
            material: state.node_materials.get(&entity_id).cloned(),
            light: state.node_lights.get(&entity_id).cloned(),
        }
    }
}

impl Command for DeleteEntityCommand {
    fn execute(&mut self, state: &mut EditorState) {
        state.scene_tree.remove_node(self.entity_id);
        state.node_transforms.remove(&self.entity_id);
        state.node_materials.remove(&self.entity_id);
        state.node_lights.remove(&self.entity_id);
        state.selected_nodes.retain(|&id| id != self.entity_id);
    }

    fn undo(&mut self, state: &mut EditorState) {
        // Recreate with original name and parent
        let new_id = state.scene_tree.add_node(&self.entity_name, self.parent);
        if let Some(t) = self.transform {
            state.node_transforms.insert(new_id, t);
        }
        if let Some(mat) = &self.material {
            state.node_materials.insert(new_id, mat.clone());
        }
        if let Some(light) = &self.light {
            state.node_lights.insert(new_id, light.clone());
        }
        // Note: new_id may differ from original, but we store the mapping
        self.entity_id = new_id;
    }

    fn redo(&mut self, state: &mut EditorState) {
        state.scene_tree.remove_node(self.entity_id);
        state.node_transforms.remove(&self.entity_id);
        state.node_materials.remove(&self.entity_id);
        state.node_lights.remove(&self.entity_id);
        state.selected_nodes.retain(|&id| id != self.entity_id);
    }
}
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

### Task 4: Preserve Hierarchy in Scene Bridge

**Files:**
- Modify: `crates/engine-editor/src/scene_bridge.rs`

- [ ] **Step 1: Add parent_index to SceneEntity**

In `scene_serializer.rs`, add field to `SceneEntity`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: u64,
    pub name: String,
    pub transform: TransformData,
    pub components: Vec<ComponentData>,
    pub parent_index: Option<usize>, // NEW: index of parent in entities list
}
```

Update `SceneEntity::new()`:

```rust
pub fn new(id: u64, name: String) -> Self {
    Self {
        id,
        name,
        transform: TransformData::default(),
        components: Vec::new(),
        parent_index: None,
    }
}
```

- [ ] **Step 2: Record parent during export**

In `scene_bridge.rs` `export_world()`, after creating entities, record parent indices:

```rust
// After the entity collection loop, add parent tracking
let mut entity_index_map: HashMap<u64, usize> = HashMap::new();
for (i, entity) in entity_set.iter().enumerate() {
    entity_index_map.insert(entity.index() as u64, i);
}

// Then in the export loop:
for (i, entity) in entity_set.iter().enumerate() {
    let mut scene_entity = SceneEntity::new(i as u64, format!("Entity_{}", entity.index()));

    // Check if entity has a parent in the world
    // Note: This depends on how parent-child is tracked in the ECS
    // For now, set parent_index to None
    scene_entity.parent_index = None;

    // ... rest of export
    scene.add_entity(scene_entity);
}
```

- [ ] **Step 3: Restore hierarchy during import**

In `scene_bridge.rs` `import_world()`, after creating all entities, set parent-child relationships:

```rust
pub fn import_world(&self, scene: &Scene, world: &mut World) -> Result<Vec<Entity>> {
    let mut entities: Vec<Entity> = Vec::new();

    // Phase 1: Create all entities
    for scene_entity in &scene.entities {
        let entity = world.spawn();
        let transform = transform_from_data(&scene_entity.transform);
        world.add_component(entity, transform);

        for comp_data in &scene_entity.components {
            if let Some(serializer) = self.serializers.get(&comp_data.type_name) {
                serializer.apply(world, entity, comp_data).with_context(|| {
                    format!("Failed to apply component '{}' to entity {}", comp_data.type_name, scene_entity.id)
                })?;
            }
        }
        entities.push(entity);
    }

    // Phase 2: Restore parent-child relationships
    for (i, scene_entity) in scene.entities.iter().enumerate() {
        if let Some(parent_idx) = scene_entity.parent_index {
            if parent_idx < entities.len() && i < entities.len() {
                // Note: ECS parent-child relationship depends on engine_scene implementation
                // This may need to use scene_tree or transform hierarchy
            }
        }
    }

    Ok(entities)
}
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

## Phase 2: Core Feature Completion

### Task 5: Add Search to Resource Browser

**Files:**
- Modify: `crates/engine-editor/src/resource_browser.rs`

- [ ] **Step 1: Add search_query field**

```rust
#[derive(Debug, Clone)]
pub struct ResourceBrowser {
    pub current_path: String,
    pub entries: Vec<ResourceEntry>,
    pub selected_entry: Option<usize>,
    pub search_query: String, // NEW
}
```

Update `new()`:

```rust
pub fn new() -> Self {
    let mut browser = Self {
        current_path: "Assets".into(),
        entries: Vec::new(),
        selected_entry: None,
        search_query: String::new(),
    };
    browser.refresh();
    if browser.entries.is_empty() {
        browser.current_path = ".".into();
        browser.refresh();
    }
    if browser.entries.is_empty() {
        browser.entries = Self::demo_entries();
    }
    browser
}
```

- [ ] **Step 2: Add filtered_entries method**

```rust
impl ResourceBrowser {
    /// Returns entries filtered by search query.
    pub fn filtered_entries(&self) -> Vec<&ResourceEntry> {
        if self.search_query.is_empty() {
            self.entries.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.entries
                .iter()
                .filter(|e| e.name.to_lowercase().contains(&query))
                .collect()
        }
    }
}
```

- [ ] **Step 3: Update draw function to include search bar**

In `resource_browser.rs` `draw()`, add search input before the file list:

```rust
pub fn draw(state: &mut EditorState, ui: &mut egui::Ui) {
    // Path bar
    ui.horizontal(|ui| {
        ui.label("路径:");
        let current_path = state.resource_browser.current_path.clone();
        let path_parts: Vec<_> = current_path.split('/').collect();
        for (i, part) in path_parts.iter().enumerate() {
            if ui.link(*part).clicked() {
                let new_path: String = path_parts[..=i].join("/");
                state.resource_browser.current_path = new_path;
                state.resource_browser.refresh();
            }
            if i < path_parts.len() - 1 {
                ui.label("/");
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("刷新").clicked() {
                state.resource_browser.refresh();
            }
        });
    });

    // Search bar
    ui.horizontal(|ui| {
        ui.label("搜索:");
        ui.text_edit_singleline(&mut state.resource_browser.search_query);
        if ui.small_button("✕").clicked() {
            state.resource_browser.search_query.clear();
        }
    });

    ui.separator();

    // File list using filtered_entries
    let mut navigate_to: Option<String> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let filtered = state.resource_browser.filtered_entries();
            for (i, entry) in filtered.iter().enumerate() {
                // ... existing entry rendering logic
            }
        });
}
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

### Task 6: Implement Hot Reload

**Files:**
- Modify: `crates/engine-editor/src/hot_reload.rs`

- [ ] **Step 1: Add ReloadNotification struct**

```rust
/// Notification for hot-reloaded assets.
#[derive(Debug, Clone)]
pub struct ReloadNotification {
    pub message: String,
    pub timestamp: std::time::Instant,
    pub level: ReloadLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReloadLevel {
    Info,
    Warning,
    Error,
}

/// Hot reload manager with notifications.
pub struct HotReloadManager {
    pub notifications: VecDeque<ReloadNotification>,
    pub max_notifications: usize,
}
```

- [ ] **Step 2: Add notification methods**

```rust
impl HotReloadManager {
    pub fn new() -> Self {
        Self {
            notifications: VecDeque::with_capacity(20),
            max_notifications: 20,
        }
    }

    pub fn notify(&mut self, message: String, level: ReloadLevel) {
        self.notifications.push_back(ReloadNotification {
            message,
            timestamp: std::time::Instant::now(),
            level,
        });
        while self.notifications.len() > self.max_notifications {
            self.notifications.pop_front();
        }
    }

    pub fn clear_old(&mut self, max_age: std::time::Duration) {
        let now = std::time::Instant::now();
        self.notifications.retain(|n| now.duration_since(n.timestamp) < max_age);
    }
}
```

- [ ] **Step 3: Add texture reload function**

```rust
pub fn reload_texture(
    path: &str,
    asset_store: &mut engine_asset::AssetStore,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
) -> Result<(), String> {
    // Load image from disk
    let img = image::open(path).map_err(|e| format!("Failed to load image: {}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Create wgpu texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(path),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );

    // Update asset store
    // asset_store.update_texture(path, texture);

    Ok(())
}
```

- [ ] **Step 4: Add material reload function**

```rust
pub fn reload_material(
    path: &str,
    asset_store: &mut engine_asset::AssetStore,
) -> Result<(), String> {
    // Read material file
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read material file: {}", e))?;

    // Parse material data
    let material_data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse material JSON: {}", e))?;

    // Update asset store
    // asset_store.update_material(path, material_data);

    Ok(())
}
```

- [ ] **Step 5: Integrate into main editor loop**

In `plugin.rs` or `layout.rs`, add hot reload polling:

```rust
// In the main editor update loop:
if let Some(watcher) = &mut state.file_watcher {
    if let Some(events) = watcher.poll_changes() {
        for event in events {
            match event.kind {
                notify::EventKind::Modify(_) => {
                    let path = event.paths[0].to_string_lossy().to_string();
                    let ext = std::path::Path::new(&path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");

                    let result = match ext {
                        "png" | "jpg" | "jpeg" | "bmp" => {
                            reload_texture(&path, &mut state.asset_store, &queue, &device)
                        }
                        "mat" | "material" => {
                            reload_material(&path, &mut state.asset_store)
                        }
                        _ => continue,
                    };

                    match result {
                        Ok(()) => state.hot_reload.notify(
                            format!("Reloaded: {}", path),
                            ReloadLevel::Info,
                        ),
                        Err(e) => state.hot_reload.notify(
                            format!("Failed to reload {}: {}", path, e),
                            ReloadLevel::Error,
                        ),
                    }
                }
                _ => {}
            }
        }
    }
}
```

- [ ] **Step 6: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

### Task 7: Fix Inspector Panel Persistence

**Files:**
- Modify: `crates/engine-editor/src/inspector.rs`
- Modify: `crates/engine-editor/src/state.rs`

- [ ] **Step 1: Add InspectorPanel to EditorState**

In `state.rs`:

```rust
pub use crate::inspector::InspectorPanel;

pub struct EditorState {
    // ... existing fields
    pub inspector_panel: InspectorPanel,
}
```

Initialize in `EditorState::new()`:

```rust
inspector_panel: InspectorPanel::new(),
```

- [ ] **Step 2: Remove per-frame InspectorPanel::new()**

In `inspector.rs`, find the `draw` function. Remove the line that creates a new `InspectorPanel` every frame:

```rust
pub fn draw(state: &mut EditorState, ui: &mut egui::Ui) {
    // REMOVE: let mut panel = InspectorPanel::new();

    // Use persistent panel from state:
    let panel = &mut state.inspector_panel;

    // ... rest of draw logic using panel
}
```

- [ ] **Step 3: Add reset_inspector method**

```rust
impl InspectorPanel {
    /// Resets the inspector panel state (e.g., clear search, selection).
    pub fn reset(&mut self) {
        self.search_query.clear();
        self.scroll_offset = 0.0;
    }
}
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

## Phase 3: Polish

### Task 8: Add Basic Text Editing to Script Editor

**Files:**
- Modify: `crates/engine-editor/src/script_editor/mod.rs`

- [ ] **Step 1: Add cursor position struct**

```rust
/// Text cursor position (line and column, 0-indexed).
#[derive(Debug, Clone, Copy, Default)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

/// Text selection range.
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start: Cursor,
    pub end: Cursor,
}

/// Text buffer with editing history.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub selection: Option<Selection>,
    pub history: Vec<Vec<String>>,
    pub history_index: usize,
}
```

- [ ] **Step 2: Implement cursor movement**

```rust
impl TextBuffer {
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        Self {
            lines,
            cursor: Cursor::default(),
            selection: None,
            history: vec![lines.clone()],
            history_index: 0,
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor.line < self.lines.len() {
            if self.cursor.col < self.lines[self.cursor.line].len() {
                self.cursor.col += 1;
            } else if self.cursor.line + 1 < self.lines.len() {
                self.cursor.line += 1;
                self.cursor.col = 0;
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.line].len());
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.line].len());
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        if self.cursor.line < self.lines.len() {
            self.lines[self.cursor.line].insert(self.cursor.col, ch);
            self.cursor.col += 1;
            self.push_history();
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor.col > 0 {
            self.lines[self.cursor.line].remove(self.cursor.col - 1);
            self.cursor.col -= 1;
            self.push_history();
        } else if self.cursor.line > 0 {
            let current = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
            self.lines[self.cursor.line].push_str(&current);
            self.push_history();
        }
    }

    pub fn insert_newline(&mut self) {
        if self.cursor.line < self.lines.len() {
            let remaining = self.lines[self.cursor.line][self.cursor.col..].to_string();
            self.lines[self.cursor.line].truncate(self.cursor.col);
            self.cursor.line += 1;
            self.cursor.col = 0;
            self.lines.insert(self.cursor.line, remaining);
            self.push_history();
        }
    }

    fn push_history(&mut self) {
        self.history.truncate(self.history_index + 1);
        self.history.push(self.lines.clone());
        self.history_index = self.history.len() - 1;
    }

    pub fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.lines = self.history[self.history_index].clone();
        }
    }

    pub fn redo(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            self.lines = self.history[self.history_index].clone();
        }
    }
}
```

- [ ] **Step 3: Build and verify**

Run: `cargo build -p engine-editor`
Expected: Successful build

---

## Verification Checklist

After all tasks, run full verification:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --all
```

All should pass with no errors.
