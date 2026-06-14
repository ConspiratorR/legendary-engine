//! Central editor state that all panels share.
//!
//! [`EditorState`] owns the scene hierarchy, selection, active tool, camera,
//! and references to sub-editor states (animation, material, node-graph, etc.).
//! Each panel module receives `&mut EditorState` during the frame to read and
//! mutate shared data.

use egui::{Context, Pos2};
use engine_math::{Mat4, Vec3};
use engine_render::instancing::{InstanceBatch, InstanceKey};
use engine_render::light::LightingUniform;
use engine_render::resource::material::MaterialStore;
use engine_render::resource::mesh::MeshStore;
use engine_render::shadow::ShadowMapConfig;
use engine_ui::GuiSkin;

/// Editor play mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    /// Normal editing mode.
    Editing,
    /// Game is running.
    Playing,
    /// Game is paused (state preserved, but not stepping).
    Paused,
}

/// Active transform tool in the viewport toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    /// Selection-only mode (click to select).
    Select,
    /// Translate (move) objects along axes.
    Translate,
    /// Rotate objects around axes.
    Rotate,
    /// Scale objects along axes.
    Scale,
    /// Terrain sculpting mode.
    Terrain,
}

/// Current gizmo drag interaction state.
#[derive(Debug, Clone)]
pub struct GizmoInteraction {
    /// Active axis mask (bit 0=X, 1=Y, 2=Z).
    pub axis: u8,
    /// Active plane mask, if dragging on a plane.
    pub plane: Option<u8>,
    /// Mouse position at drag start.
    pub start_mouse: Pos2,
    /// Initial value at drag start.
    pub start_value: f32,
}

/// A single node in the scene hierarchy tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Unique identifier for this node.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Icon emoji shown in the hierarchy panel.
    pub icon: String,
    /// Whether this node's children are visible in the hierarchy.
    pub expanded: bool,
    /// Parent node ID, or `None` for root nodes.
    pub parent: Option<u64>,
    /// Child node IDs.
    pub children: Vec<u64>,
}

/// The scene hierarchy tree, managing parent-child relationships between nodes.
#[derive(Debug, Clone)]
pub struct SceneTree {
    /// All nodes in the tree.
    pub nodes: Vec<TreeNode>,
    /// IDs of root-level nodes.
    pub root_ids: Vec<u64>,
    pub(crate) next_id: u64,
}

impl SceneTree {
    /// Creates a new scene tree with a default hierarchy (Root + 5 child nodes).
    pub fn new() -> Self {
        let root_id = 1;
        Self {
            nodes: vec![
                TreeNode {
                    id: 1,
                    name: "Root".into(),
                    icon: "📁".into(),
                    expanded: true,
                    parent: None,
                    children: vec![2, 3, 4, 5, 6],
                },
                TreeNode {
                    id: 2,
                    name: "Player".into(),
                    icon: "🎮".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
                TreeNode {
                    id: 3,
                    name: "Terrain".into(),
                    icon: "🏔".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
                TreeNode {
                    id: 4,
                    name: "Cube".into(),
                    icon: "📦".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
                TreeNode {
                    id: 5,
                    name: "Sphere".into(),
                    icon: "🔮".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
                TreeNode {
                    id: 6,
                    name: "Light".into(),
                    icon: "💡".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
            ],
            root_ids: vec![root_id],
            next_id: 7,
        }
    }

    /// Adds a new node with the given name under `parent` (or the root if `None`).
    /// Returns the new node's ID.
    pub fn add_node(&mut self, name: &str, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let parent_id = parent.unwrap_or(self.root_ids[0]);
        self.nodes.push(TreeNode {
            id,
            name: name.to_string(),
            icon: "📦".into(),
            expanded: false,
            parent: Some(parent_id),
            children: Vec::new(),
        });
        if let Some(p) = self.nodes.iter_mut().find(|n| n.id == parent_id) {
            p.children.push(id);
        }
        id
    }

    /// Removes a node and all its descendants from the tree.
    pub fn remove_node(&mut self, id: u64) {
        let parent_id = self
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.parent);
        if let Some(pid) = parent_id
            && let Some(p) = self.nodes.iter_mut().find(|n| n.id == pid)
        {
            p.children.retain(|c| *c != id);
        }
        let mut to_remove = vec![id];
        let mut i = 0;
        while i < to_remove.len() {
            let cids: Vec<u64> = self
                .nodes
                .iter()
                .filter(|n| n.parent == Some(to_remove[i]))
                .map(|n| n.id)
                .collect();
            to_remove.extend(cids);
            i += 1;
        }
        self.nodes.retain(|n| !to_remove.contains(&n.id));
    }

    /// Moves a node to a new parent. If `new_parent` is `None`, moves to root.
    pub fn reparent(&mut self, id: u64, new_parent: Option<u64>) {
        let old_parent = self
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.parent);
        if let Some(pid) = old_parent
            && let Some(p) = self.nodes.iter_mut().find(|n| n.id == pid)
        {
            p.children.retain(|c| *c != id);
        }
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.parent = new_parent;
        }
        if let Some(npid) = new_parent
            && let Some(p) = self.nodes.iter_mut().find(|n| n.id == npid)
        {
            p.children.push(id);
        }
    }

    /// Renames a node.
    pub fn rename(&mut self, id: u64, name: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.name = name.to_string();
        }
    }

    /// Searches nodes by name (case-insensitive substring match). Returns matching node IDs.
    pub fn search(&self, query: &str) -> Vec<u64> {
        if query.is_empty() {
            return Vec::new();
        }
        let q = query.to_lowercase();
        self.nodes
            .iter()
            .filter(|n| n.name.to_lowercase().contains(&q))
            .map(|n| n.id)
            .collect()
    }
}

impl Default for SceneTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Orbiting camera for the editor viewport.
#[derive(Debug, Clone)]
pub struct EditorCamera {
    /// World-space point the camera orbits around.
    pub target: Vec3,
    /// Distance from target.
    pub distance: f32,
    /// Horizontal angle in radians.
    pub yaw: f32,
    /// Vertical angle in radians (clamped to avoid gimbal lock).
    pub pitch: f32,
    /// Field of view in radians.
    pub fov: f64,
    /// Near clipping plane.
    pub near: f64,
    /// Far clipping plane.
    pub far: f64,
}

impl EditorCamera {
    /// Creates a camera with default orbiting parameters.
    pub fn new() -> Self {
        Self {
            target: Vec3::new(0.0, 2.0, 0.0),
            distance: 12.0,
            yaw: 0.0,
            pitch: -0.3,
            fov: 60.0_f64.to_radians(),
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Orbits the camera by the given mouse delta (pitch is clamped).
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * 0.005;
        self.pitch = (self.pitch + delta_y * 0.005).clamp(-1.55, 1.55);
    }

    /// Pans the camera (moves target in the camera's local right/up plane).
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.up();
        let speed = self.distance * 0.002;
        self.target -= right * delta_x * speed;
        self.target += up * delta_y * speed;
    }

    /// Zooms the camera (adjusts distance, clamped to [0.5, 500]).
    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * 1.1_f32.powf(-delta)).clamp(0.5, 500.0);
    }

    /// Returns the camera's world-space position.
    pub fn eye(&self) -> Vec3 {
        let dir = self.forward();
        self.target + dir * self.distance
    }

    fn forward(&self) -> Vec3 {
        let pitch_sin = self.pitch.sin();
        let pitch_cos = self.pitch.cos();
        Vec3::new(
            self.yaw.sin() * pitch_cos,
            pitch_sin,
            self.yaw.cos() * pitch_cos,
        )
    }

    fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    fn up(&self) -> Vec3 {
        Vec3::new(0.0, 1.0, 0.0)
    }

    /// Computes the view matrix (right-handed, looking at target).
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, self.up())
    }

    /// Computes the perspective projection matrix for the given aspect ratio.
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov as f32, aspect, self.near as f32, self.far as f32)
    }
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self::new()
    }
}

use crate::animation_editor::AnimationEditorState;
use crate::material_editor::MaterialEditorState;
use crate::node_graph::NodeGraphState;
use crate::performance_overlay::PerformanceOverlay;
use crate::performance_profiler::PerformanceProfilerState;
use crate::resource_browser::ResourceBrowser;
use crate::scene_serializer::SceneManager;
use crate::script_editor::ScriptEditorState;
use crate::shortcuts::{EditorAction, ShortcutManager};
use std::collections::HashMap;

/// Light property data for the editor inspector.
#[derive(Debug, Clone)]
pub struct LightData {
    pub light_type: LightType,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub direction: [f32; 3],
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
}

impl Default for LightData {
    fn default() -> Self {
        Self {
            light_type: LightType::Directional,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            range: 10.0,
            direction: [0.3, -1.0, -0.5],
            inner_angle: 15.0,
            outer_angle: 30.0,
            enabled: true,
        }
    }
}

/// PBR material property data for the editor inspector.
#[derive(Debug, Clone)]
pub struct MaterialData {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub emissive: [f32; 3],
}

impl Default for MaterialData {
    fn default() -> Self {
        Self {
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            ao: 1.0,
            emissive: [0.0; 3],
        }
    }
}

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

/// Scene data built from the editor state for 3D rendering.
pub struct EditorSceneData {
    pub mesh_store: MeshStore,
    pub material_store: MaterialStore,
    pub batches: Vec<InstanceBatch>,
    pub lighting: LightingUniform,
    pub light_direction: [f32; 3],
    pub camera_vp: Mat4,
    pub camera_pos: [f32; 3],
    pub shadow_config: ShadowMapConfig,
    pub scene_aabb_min: Vec3,
    pub scene_aabb_max: Vec3,
}

/// Central editor state holding all panel data, selections, and tool state.
#[derive(Debug, Clone)]
pub struct EditorState {
    pub selected_nodes: Vec<u64>,
    pub active_menu: Option<usize>,
    pub active_tool: ToolType,
    pub active_viewport_tab: usize,
    pub active_bottom_tab: usize,
    pub fps: u32,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub scene_tree: SceneTree,
    pub camera: EditorCamera,
    /// Camera used for the Game viewport tab (runtime perspective).
    pub game_camera: EditorCamera,
    pub show_grid: bool,
    pub show_debug_overlay: bool,
    pub gizmo_interaction: Option<GizmoInteraction>,
    pub gizmo_size: f32,
    pub hierarchy_search: String,
    pub node_transforms: HashMap<u64, [f32; 9]>,
    pub node_render: HashMap<u64, (String, String, bool)>,
    pub node_physics: HashMap<u64, (String, String)>,
    pub node_lights: HashMap<u64, LightData>,
    pub node_materials: HashMap<u64, MaterialData>,
    pub resource_browser: ResourceBrowser,
    pub scene_manager: SceneManager,
    pub status_message: Option<String>,
    pub node_graph_state: NodeGraphState,
    pub material_editor: MaterialEditorState,
    pub animation_editor: AnimationEditorState,
    pub script_editor: ScriptEditorState,
    pub performance_profiler: PerformanceProfilerState,
    pub performance_overlay: PerformanceOverlay,
    pub terrain_panel: crate::terrain_panel::TerrainPanel,
    pub terrain_sculpt_active: bool,
    pub terrain_sculpt_screen_pos: Option<(f32, f32)>,
    pub node_sprites: HashMap<u64, SpriteData>,
    pub node_particles: HashMap<u64, ParticleData>,
    pub node_audio: HashMap<u64, AudioData>,
    pub node_scripts: HashMap<u64, ScriptData>,
    pub node_tags: HashMap<u64, Vec<String>>,
    pub viewport_layout: crate::viewport_renderer::ViewportLayout,
    /// Current play mode state.
    pub play_state: PlayState,
    /// Snapshot of editor transforms before play (for restoring on stop).
    pub editor_transform_snapshot: HashMap<u64, [f32; 9]>,
    /// Elapsed time since play started.
    pub runtime_elapsed: f32,
    /// Autosave interval in seconds (0 = disabled).
    pub autosave_interval: f32,
    /// Time since last autosave.
    pub autosave_timer: f32,
    /// Whether autosave is enabled.
    pub autosave_enabled: bool,
    /// Keyboard shortcut manager.
    pub shortcuts: ShortcutManager,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorState {
    /// Creates a new editor state with default scene tree, camera, and panel data.
    pub fn new() -> Self {
        let mut node_transforms = HashMap::new();
        let mut node_render = HashMap::new();
        let mut node_physics = HashMap::new();
        let mut node_lights = HashMap::new();
        let mut node_materials = HashMap::new();
        for i in 1..=6 {
            node_transforms.insert(i, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
            node_render.insert(i, ("Default".into(), "Cube".into(), true));
            node_physics.insert(i, ("Static".into(), "Box".into()));
        }
        // Add a directional light to the Light node (id=6)
        node_lights.insert(6, LightData::default());
        // Add a material to Cube (id=4) and Sphere (id=5)
        node_materials.insert(4, MaterialData::default());
        node_materials.insert(
            5,
            MaterialData {
                base_color: [0.2, 0.6, 1.0, 1.0],
                metallic: 0.8,
                roughness: 0.1,
                ..Default::default()
            },
        );
        Self {
            selected_nodes: Vec::new(),
            active_menu: None,
            active_tool: ToolType::Translate,
            active_viewport_tab: 0,
            active_bottom_tab: 0,
            fps: 60,
            show_left_panel: true,
            show_right_panel: true,
            scene_tree: SceneTree::new(),
            camera: EditorCamera::new(),
            game_camera: EditorCamera {
                target: Vec3::new(0.0, 1.0, 0.0),
                distance: 8.0,
                yaw: 0.0,
                pitch: -0.2,
                fov: 60.0_f64.to_radians(),
                near: 0.1,
                far: 1000.0,
            },
            show_grid: true,
            show_debug_overlay: false,
            gizmo_interaction: None,
            gizmo_size: 60.0,
            hierarchy_search: String::new(),
            node_transforms,
            node_render,
            node_physics,
            node_lights,
            node_materials,
            resource_browser: ResourceBrowser::new(),
            scene_manager: SceneManager::new(),
            status_message: None,
            node_graph_state: NodeGraphState::default(),
            material_editor: MaterialEditorState::new(),
            animation_editor: AnimationEditorState::new(),
            script_editor: ScriptEditorState::new(),
            performance_profiler: PerformanceProfilerState::new(),
            performance_overlay: PerformanceOverlay::new(),
            terrain_panel: crate::terrain_panel::TerrainPanel::default(),
            terrain_sculpt_active: false,
            terrain_sculpt_screen_pos: None,
            node_sprites: HashMap::new(),
            node_particles: HashMap::new(),
            node_audio: HashMap::new(),
            node_scripts: HashMap::new(),
            node_tags: HashMap::new(),
            viewport_layout: crate::viewport_renderer::ViewportLayout::default(),
            play_state: PlayState::Editing,
            editor_transform_snapshot: HashMap::new(),
            runtime_elapsed: 0.0,
            autosave_interval: 300.0, // 5 minutes
            autosave_timer: 0.0,
            autosave_enabled: true,
            shortcuts: ShortcutManager::new(),
        }
    }

    /// Runs one frame of the editor UI, drawing all panels via egui.
    pub fn frame(
        &mut self,
        ctx: &Context,
        skin: &GuiSkin,
        renderer: &mut engine_render::renderer::Renderer,
        vp_renderer: &mut crate::viewport_renderer::ViewportRenderer,
        egui_state: &mut engine_ui::EguiState,
    ) {
        crate::layout::frame(self, ctx, skin, renderer, vp_renderer, egui_state);
    }

    /// Enter play mode: snapshot editor state.
    /// Returns true if state changed to Playing.
    pub fn play(&mut self) -> bool {
        if self.play_state == PlayState::Playing {
            return false;
        }
        self.editor_transform_snapshot = self.node_transforms.clone();
        self.runtime_elapsed = 0.0;
        self.play_state = PlayState::Playing;
        self.status_message = Some("Playing".into());
        true
    }

    /// Pause the runtime (freeze simulation, keep state).
    pub fn pause(&mut self) {
        if self.play_state == PlayState::Playing {
            self.play_state = PlayState::Paused;
            self.status_message = Some("Paused".into());
        } else if self.play_state == PlayState::Paused {
            self.play_state = PlayState::Playing;
            self.status_message = Some("Playing".into());
        }
    }

    /// Stop play mode: restore editor state.
    /// Returns true if state changed to Editing.
    pub fn stop(&mut self) -> bool {
        if self.play_state == PlayState::Editing {
            return false;
        }
        self.node_transforms = self.editor_transform_snapshot.clone();
        self.play_state = PlayState::Editing;
        self.status_message = Some("Stopped".into());
        true
    }

    /// Build a runtime ECS World from the current scene tree.
    pub fn build_runtime_world(&self) -> engine_ecs::world::World {
        let mut world = engine_ecs::world::World::new();

        // Add physics world resource
        world.insert_resource(engine_physics::world::PhysicsWorld::default());

        // Spawn entities from scene tree
        for node in &self.scene_tree.nodes {
            if node.parent.is_none() {
                continue;
            }
            let entity = world.spawn();
            let t = self
                .node_transforms
                .get(&node.id)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

            // Add Transform component
            world.add_component(
                entity,
                RuntimeTransform {
                    position: [t[0], t[1], t[2]],
                    rotation: [t[3], t[4], t[5], t[6]],
                    scale: [t[7], t[8], 1.0],
                },
            );

            // Add RigidBody if physics data exists
            if let Some((body_type, _collider_type)) = self.node_physics.get(&node.id) {
                let rb = match body_type.as_str() {
                    "Dynamic" => engine_physics::RigidBody::new_dynamic(),
                    "Kinematic" => engine_physics::RigidBody::new_kinematic(),
                    _ => engine_physics::RigidBody::new_static(),
                };
                world.add_component(entity, rb);
            }
        }

        world
    }

    /// Step the runtime ECS world and read back transforms.
    pub fn step_runtime(&mut self, world: &mut engine_ecs::world::World, dt: f32) {
        if self.play_state != PlayState::Playing {
            return;
        }
        self.runtime_elapsed += dt;

        // Step physics — recover from panics gracefully
        let mut pw = match world.remove_resource::<engine_physics::world::PhysicsWorld>() {
            Some(pw) => pw,
            None => return,
        };
        pw.step(world);
        world.insert_resource(pw);

        // Read back transforms from ECS to editor state
        let transform_indices = world.component_entities::<RuntimeTransform>();
        let node_ids: Vec<u64> = self
            .scene_tree
            .nodes
            .iter()
            .filter(|n| n.parent.is_some())
            .map(|n| n.id)
            .collect();

        for (i, &idx) in transform_indices.iter().enumerate() {
            if i >= node_ids.len() {
                break;
            }
            let node_id = node_ids[i];
            if let Some(tc) = world.get_by_index::<RuntimeTransform>(idx)
                && let Some(t) = self.node_transforms.get_mut(&node_id)
            {
                t[0] = tc.position[0];
                t[1] = tc.position[1];
                t[2] = tc.position[2];
            }
        }
    }

    /// Check if autosave should trigger (returns true if autosave needed).
    pub fn check_autosave(&mut self, dt: f32) -> bool {
        if !self.autosave_enabled || self.autosave_interval <= 0.0 {
            return false;
        }
        self.autosave_timer += dt;
        if self.autosave_timer >= self.autosave_interval {
            self.autosave_timer = 0.0;
            return true;
        }
        false
    }

    /// Process a keyboard shortcut action.
    pub fn handle_shortcut(&mut self, action: EditorAction) {
        match action {
            EditorAction::SaveScene => {
                let _ = self.scene_manager.save_current_scene();
                self.status_message = Some("Scene saved".into());
            }
            EditorAction::NewScene => {
                self.scene_manager.create_scene("Untitled".into());
                self.status_message = Some("New scene created".into());
            }
            EditorAction::Undo => {
                self.undo();
            }
            EditorAction::Redo => {
                self.redo();
            }
            EditorAction::Duplicate => {
                self.duplicate_selected();
            }
            EditorAction::Delete => {
                self.delete_selected();
            }
            EditorAction::SelectAll => {
                self.selected_nodes = self.scene_tree.nodes.iter().map(|n| n.id).collect();
            }
            EditorAction::DeselectAll => {
                self.selected_nodes.clear();
            }
            EditorAction::TranslateTool => {
                self.active_tool = ToolType::Translate;
            }
            EditorAction::RotateTool => {
                self.active_tool = ToolType::Rotate;
            }
            EditorAction::ScaleTool => {
                self.active_tool = ToolType::Scale;
            }
            EditorAction::TerrainTool => {
                self.active_tool = ToolType::Terrain;
            }
            EditorAction::Play => {
                self.play();
            }
            EditorAction::Stop => {
                self.stop();
            }
            EditorAction::ToggleGrid => {
                self.show_grid = !self.show_grid;
            }
            _ => {}
        }
    }

    /// Undo the last action.
    fn undo(&mut self) {
        // TODO: wire undo/redo command system
        self.status_message = Some("Undo".into());
    }

    /// Redo the last undone action.
    fn redo(&mut self) {
        // TODO: wire undo/redo command system
        self.status_message = Some("Redo".into());
    }

    /// Duplicate selected nodes.
    fn duplicate_selected(&mut self) {
        let selected = self.selected_nodes.clone();
        self.selected_nodes.clear();
        for &node_id in &selected {
            let new_id = self.scene_tree.add_node("Duplicate", Some(node_id));
            if let Some(t) = self.node_transforms.get(&node_id) {
                let mut new_t = *t;
                new_t[0] += 1.0; // offset slightly
                self.node_transforms.insert(new_id, new_t);
            }
            self.selected_nodes.push(new_id);
        }
        self.status_message = Some(format!("Duplicated {} nodes", selected.len()));
    }

    /// Delete selected nodes.
    fn delete_selected(&mut self) {
        let selected = self.selected_nodes.clone();
        for &node_id in &selected {
            self.scene_tree.remove_node(node_id);
            self.node_transforms.remove(&node_id);
            self.node_materials.remove(&node_id);
            self.node_lights.remove(&node_id);
            self.node_physics.remove(&node_id);
        }
        self.selected_nodes.clear();
        self.status_message = Some(format!("Deleted {} nodes", selected.len()));
    }

    /// Build scene data for 3D rendering from the current editor state.
    pub fn build_scene(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        aspect: f32,
        camera: &EditorCamera,
    ) -> EditorSceneData {
        let mut mesh_store = MeshStore::new();
        let mut material_store = MaterialStore::new(device);

        // Upload a default cube mesh
        let cube_mesh_id = mesh_store.upload(device, &cube_vertices(), Some(&cube_indices()));

        // Create a default PBR material
        let default_mat = engine_render::resource::material::PbrMaterial {
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            ao: 1.0,
            emissive: [0.0; 3],
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
        };
        let default_mat_id = material_store.add(device, queue, default_mat);

        // Build instance batches from scene tree nodes
        let mut batches: Vec<InstanceBatch> = Vec::new();
        let mut batch_map: std::collections::HashMap<(u64, u64), usize> = std::collections::HashMap::new();

        for node in &self.scene_tree.nodes {
            if node.parent.is_none() {
                continue;
            }
            let t = self
                .node_transforms
                .get(&node.id)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

            // Check if this node has a material override
            let mat_id = if let Some(mat_data) = self.node_materials.get(&node.id) {
                let mat = engine_render::resource::material::PbrMaterial {
                    base_color: mat_data.base_color,
                    metallic: mat_data.metallic,
                    roughness: mat_data.roughness,
                    ao: mat_data.ao,
                    emissive: mat_data.emissive,
                    base_color_texture: None,
                    normal_texture: None,
                    metallic_roughness_texture: None,
                };
                material_store.add(device, queue, mat)
            } else {
                default_mat_id
            };

            let key = (cube_mesh_id, mat_id);
            let batch_idx = if let Some(&idx) = batch_map.get(&key) {
                idx
            } else {
                let idx = batches.len();
                batches.push(InstanceBatch::new(InstanceKey::new(key.0, key.1)));
                batch_map.insert(key, idx);
                idx
            };

            // Build transform matrix from [pos, rot, scale]
            let pos = Vec3::new(t[0], t[1], t[2]);
            let rot = engine_math::Quat::from_xyzw(t[3], t[4], t[5], t[6]);
            let scale = Vec3::new(t[7], t[8], 1.0); // 2D scale for now
            let transform = Mat4::from_scale_rotation_translation(scale, rot, pos);
            batches[batch_idx].push(transform);
        }

        // If no nodes have transforms, add a default cube at origin
        if batches.is_empty() {
            let mut batch = InstanceBatch::new(InstanceKey::new(cube_mesh_id, default_mat_id));
            batch.push(Mat4::IDENTITY);
            batches.push(batch);
        }

        // Build lighting from scene light data
        let mut lighting = LightingUniform::default();
        let mut light_direction = [0.3_f32, -1.0, -0.5];

        for light_data in self.node_lights.values() {
            match light_data.light_type {
                LightType::Directional => {
                    light_direction = light_data.direction;
                    let dir_light = engine_render::light::DirectionalLight {
                        direction: light_data.direction,
                        color: [
                            light_data.color[0] * light_data.intensity,
                            light_data.color[1] * light_data.intensity,
                            light_data.color[2] * light_data.intensity,
                        ],
                        intensity: 1.0,
                        enabled: light_data.enabled,
                    };
                    let pos = [0.0_f32; 3];
                    lighting.set_directional_lights(&[(&dir_light, &pos)]);
                }
                LightType::Point => {
                    let point_light = engine_render::light::PointLight {
                        color: light_data.color,
                        intensity: light_data.intensity,
                        range: light_data.range,
                        enabled: light_data.enabled,
                    };
                    let pos = [0.0_f32; 3];
                    lighting.set_point_lights(&[(&point_light, &pos)]);
                }
                LightType::Spot => {
                    let spot_light = engine_render::light::SpotLight {
                        direction: light_data.direction,
                        color: light_data.color,
                        intensity: light_data.intensity,
                        range: light_data.range,
                        inner_angle: light_data.inner_angle.to_radians(),
                        outer_angle: light_data.outer_angle.to_radians(),
                        enabled: light_data.enabled,
                    };
                    let pos = [0.0_f32; 3];
                    lighting.set_spot_lights(&[(&spot_light, &pos)]);
                }
            }
        }

        // Compute camera VP matrix
        let camera_vp = camera.projection_matrix(aspect) * camera.view_matrix();
        let camera_pos = camera.eye().to_array();

        EditorSceneData {
            mesh_store,
            material_store,
            batches,
            lighting,
            light_direction,
            camera_vp,
            camera_pos,
            shadow_config: ShadowMapConfig::default(),
            scene_aabb_min: Vec3::new(-50.0, -50.0, -50.0),
            scene_aabb_max: Vec3::new(50.0, 50.0, 50.0),
        }
    }
}

/// Transform component for runtime ECS entities.
#[derive(Debug, Clone)]
pub struct RuntimeTransform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

fn cube_vertices() -> [engine_render::resource::mesh::MeshVertex; 24] {
    use engine_render::resource::mesh::MeshVertex;
    [
        // Front face
        MeshVertex { position: [-0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [0.0, 1.0] },
        MeshVertex { position: [ 0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [1.0, 1.0] },
        MeshVertex { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [1.0, 0.0] },
        MeshVertex { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [0.0, 0.0] },
        // Back face
        MeshVertex { position: [ 0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [0.0, 1.0] },
        MeshVertex { position: [-0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [1.0, 1.0] },
        MeshVertex { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [1.0, 0.0] },
        MeshVertex { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [0.0, 0.0] },
        // Top face
        MeshVertex { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0], uv: [0.0, 1.0] },
        MeshVertex { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0], uv: [1.0, 1.0] },
        MeshVertex { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0], uv: [1.0, 0.0] },
        MeshVertex { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0], uv: [0.0, 0.0] },
        // Bottom face
        MeshVertex { position: [-0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0], uv: [0.0, 1.0] },
        MeshVertex { position: [ 0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0], uv: [1.0, 1.0] },
        MeshVertex { position: [ 0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0], uv: [1.0, 0.0] },
        MeshVertex { position: [-0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0], uv: [0.0, 0.0] },
        // Right face
        MeshVertex { position: [ 0.5, -0.5,  0.5], normal: [ 1.0,  0.0,  0.0], uv: [0.0, 1.0] },
        MeshVertex { position: [ 0.5, -0.5, -0.5], normal: [ 1.0,  0.0,  0.0], uv: [1.0, 1.0] },
        MeshVertex { position: [ 0.5,  0.5, -0.5], normal: [ 1.0,  0.0,  0.0], uv: [1.0, 0.0] },
        MeshVertex { position: [ 0.5,  0.5,  0.5], normal: [ 1.0,  0.0,  0.0], uv: [0.0, 0.0] },
        // Left face
        MeshVertex { position: [-0.5, -0.5, -0.5], normal: [-1.0,  0.0,  0.0], uv: [0.0, 1.0] },
        MeshVertex { position: [-0.5, -0.5,  0.5], normal: [-1.0,  0.0,  0.0], uv: [1.0, 1.0] },
        MeshVertex { position: [-0.5,  0.5,  0.5], normal: [-1.0,  0.0,  0.0], uv: [1.0, 0.0] },
        MeshVertex { position: [-0.5,  0.5, -0.5], normal: [-1.0,  0.0,  0.0], uv: [0.0, 0.0] },
    ]
}

fn cube_indices() -> [u32; 36] {
    [
         0,  1,  2,  2,  3,  0, // front
         4,  5,  6,  6,  7,  4, // back
         8,  9, 10, 10, 11,  8, // top
        12, 13, 14, 14, 15, 12, // bottom
        16, 17, 18, 18, 19, 16, // right
        20, 21, 22, 22, 23, 20, // left
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_tree_new_has_root() {
        let tree = SceneTree::new();
        assert_eq!(tree.nodes.len(), 6);
    }

    #[test]
    fn test_add_node_creates_child() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let child = tree.add_node("NewNode", Some(root_id));
        assert!(tree.nodes.iter().any(|n| n.id == child));
    }

    #[test]
    fn test_remove_node_cascading() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let child = tree.add_node("Child", Some(root_id));
        let _grandchild = tree.add_node("Grandchild", Some(child));
        let n_before = tree.nodes.len();
        tree.remove_node(child);
        assert_eq!(tree.nodes.len(), n_before - 2);
    }

    #[test]
    fn test_camera_orbit_clamps_pitch() {
        let mut cam = EditorCamera::new();
        cam.orbit(0.0, 1000.0);
        assert!(cam.pitch < 1.56);
        cam.orbit(0.0, -1000.0);
        assert!(cam.pitch > -1.56);
    }
}
