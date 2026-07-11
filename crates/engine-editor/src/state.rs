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
                    children: vec![2, 3, 4, 5, 6, 7, 8, 9],
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
                    name: "Red Cube".into(),
                    icon: "📦".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
                TreeNode {
                    id: 5,
                    name: "Blue Sphere".into(),
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
                TreeNode {
                    id: 7,
                    name: "Green Cylinder".into(),
                    icon: "🟢".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
                TreeNode {
                    id: 8,
                    name: "Gold Sphere".into(),
                    icon: "🟡".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
                TreeNode {
                    id: 9,
                    name: "White Cube".into(),
                    icon: "⬜".into(),
                    expanded: false,
                    parent: Some(1),
                    children: vec![],
                },
            ],
            root_ids: vec![root_id],
            next_id: 10,
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
use crate::commands::CommandManager;
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
#[derive(Debug)]
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
    pub show_camera_help: bool,
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
    /// Whether the add component menu is open.
    pub show_add_component_menu: bool,
    /// Whether the remove component menu is open.
    pub show_remove_component_menu: bool,
    /// Sky/background color for the viewport (RGB 0.0-1.0).
    pub sky_color: [f32; 3],
    /// Loaded model meshes: name → (vertices, indices)
    pub loaded_models: std::collections::HashMap<
        String,
        (Vec<engine_render::resource::mesh::MeshVertex>, Vec<u32>),
    >,
    /// Loaded prefab definitions: name → PrefabDef
    pub prefabs: std::collections::HashMap<String, engine_scene::prefab::PrefabDef>,
    /// Node currently being dragged in hierarchy (for reparent).
    pub drag_source: Option<u64>,
    /// Node being hovered during drag (drop target).
    pub drag_hover_target: Option<u64>,
    /// Undo/redo command manager.
    pub command_manager: CommandManager,
    /// Pending transform edit: (node_id, original_transform) — captured when editing starts.
    pub pending_transform_edit: Option<(u64, [f32; 9])>,
    /// Clipboard for copy/paste: (transform, material).
    pub clipboard: Vec<([f32; 9], Option<MaterialData>)>,
    /// Log messages for the console panel.
    pub log_messages: Vec<LogEntry>,
    /// Inspector search filter text.
    pub inspector_search: String,
    /// Gizmo drag state (axis index 0=X, 1=Y, 2=Z, None=not dragging).
    pub gizmo_drag_axis: Option<u8>,
    /// Screen position where gizmo drag started.
    pub gizmo_drag_start_screen: Option<(f32, f32)>,
    /// Full transform [px,py,pz,rx,ry,rz,sx,sy,sz] when gizmo drag started.
    pub gizmo_drag_start_pos: Option<[f32; 9]>,
    /// Whether the object creation menu is open.
    pub show_create_menu: bool,
}

/// A single log entry for the console panel.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

/// Log level for console display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
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
        for i in 1..=9 {
            node_physics.insert(i, ("Static".into(), "Box".into()));
        }
        // Assign different mesh types per node
        node_render.insert(1, ("Default".into(), "Cube".into(), true)); // Root
        node_render.insert(2, ("Default".into(), "Cube".into(), true)); // Player
        node_render.insert(3, ("Default".into(), "Plane".into(), true)); // Terrain
        node_render.insert(4, ("Default".into(), "Cube".into(), true)); // Red Cube
        node_render.insert(5, ("Default".into(), "Sphere".into(), true)); // Blue Sphere
        node_render.insert(6, ("Default".into(), "Sphere".into(), true)); // Light marker
        node_render.insert(7, ("Default".into(), "Cylinder".into(), true)); // Green Cylinder
        node_render.insert(8, ("Default".into(), "Sphere".into(), true)); // Gold Sphere
        node_render.insert(9, ("Default".into(), "Cube".into(), true)); // White Cube

        // Position objects in an interesting layout
        node_transforms.insert(1, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]); // Root
        node_transforms.insert(2, [-4.0, 0.5, 2.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]); // Player
        node_transforms.insert(3, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 20.0, 1.0, 20.0]); // Terrain
        node_transforms.insert(4, [0.0, 0.5, 0.0, 0.0, 0.4, 0.0, 1.0, 1.0, 1.0]); // Red Cube (rotated)
        node_transforms.insert(5, [3.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.4, 1.4, 1.4]); // Blue Sphere
        node_transforms.insert(6, [0.0, 5.0, -3.0, 0.0, 0.0, 0.0, 0.3, 0.3, 0.3]); // Light marker
        node_transforms.insert(7, [-3.0, 0.8, -2.0, 0.0, 0.0, 0.0, 1.0, 1.6, 1.0]); // Green Cylinder
        node_transforms.insert(8, [5.0, 1.0, 3.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0]); // Gold Sphere
        node_transforms.insert(9, [-2.0, 0.4, -4.0, 0.0, 0.3, 0.0, 0.8, 0.8, 0.8]); // White Cube

        // Add lights
        node_lights.insert(6, LightData::default());

        // Add varied materials
        node_materials.insert(
            4,
            MaterialData {
                base_color: [0.9, 0.2, 0.2, 1.0],
                metallic: 0.1,
                roughness: 0.6,
                ..Default::default()
            },
        );
        node_materials.insert(
            5,
            MaterialData {
                base_color: [0.2, 0.4, 0.9, 1.0],
                metallic: 0.9,
                roughness: 0.1,
                ..Default::default()
            },
        );
        node_materials.insert(
            7,
            MaterialData {
                base_color: [0.2, 0.8, 0.3, 1.0],
                metallic: 0.0,
                roughness: 0.7,
                ..Default::default()
            },
        );
        node_materials.insert(
            8,
            MaterialData {
                base_color: [1.0, 0.85, 0.0, 1.0],
                metallic: 1.0,
                roughness: 0.2,
                ..Default::default()
            },
        );
        node_materials.insert(9, MaterialData::default());
        // Make Player (id=2) dynamic for physics testing
        node_physics.insert(2, ("Dynamic".into(), "Box".into()));
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
            show_camera_help: false,
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
            gizmo_drag_axis: None,
            gizmo_drag_start_screen: None,
            gizmo_drag_start_pos: None,
            show_create_menu: false,
            drag_source: None,
            drag_hover_target: None,
            command_manager: CommandManager::default(),
            pending_transform_edit: None,
            clipboard: Vec::new(),
            log_messages: Vec::new(),
            inspector_search: String::new(),
            show_add_component_menu: false,
            show_remove_component_menu: false,
            sky_color: [0.15, 0.20, 0.30],
            loaded_models: std::collections::HashMap::new(),
            prefabs: std::collections::HashMap::new(),
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
        self.log_info("运行模式已启动");
        true
    }

    /// Pause the runtime (freeze simulation, keep state).
    pub fn pause(&mut self) {
        if self.play_state == PlayState::Playing {
            self.play_state = PlayState::Paused;
            self.status_message = Some("Paused".into());
            self.log_info("运行模式已暂停");
        } else if self.play_state == PlayState::Paused {
            self.play_state = PlayState::Playing;
            self.status_message = Some("Playing".into());
            self.log_info("运行模式已恢复");
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
        self.log_info("运行模式已停止");
        true
    }

    /// Build a runtime ECS World from the current scene tree.
    pub fn build_runtime_world(&self) -> engine_ecs::world::World {
        let mut world = engine_ecs::world::World::new();

        // Add physics world resource
        world.insert_resource(engine_physics::world::PhysicsWorld::default());

        // Add input manager resource
        world.insert_resource(engine_input::input_manager::InputManager::new());

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
        let _span = tracing::info_span!("step_runtime").entered();
        if self.play_state != PlayState::Playing {
            return;
        }
        self.runtime_elapsed += dt;

        // Step input
        if let Some(input) = world.get_resource_mut::<engine_input::input_manager::InputManager>() {
            input.update_frame();
        }

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
                // Sync EditorState to scene before saving
                let scene = self.to_scene("Untitled");
                self.scene_manager.set_current_scene(scene);
                match self.scene_manager.save_current_scene() {
                    Ok(()) => {
                        let entity_count = self
                            .scene_manager
                            .current_scene()
                            .map(|s| s.entities.len())
                            .unwrap_or(0);
                        self.log_info(&format!("场景已保存 ({} 个实体)", entity_count));
                        self.status_message = Some("场景已保存".into());
                    }
                    Err(e) => {
                        self.log_error(&format!("保存失败: {}", e));
                        self.status_message = Some(format!("保存失败: {}", e));
                    }
                }
            }
            EditorAction::LoadScene => {
                // This is handled in main.rs with file dialog
                self.status_message = Some("请使用文件菜单加载场景".into());
            }
            EditorAction::NewScene => {
                self.scene_manager.create_scene("Untitled".into());
                self.status_message = Some("新场景已创建".into());
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
            EditorAction::FocusOnSelection => {
                self.focus_on_selection();
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
            EditorAction::NextFrame => {
                self.active_viewport_tab = (self.active_viewport_tab + 1) % 3;
            }
            EditorAction::PrevFrame => {
                self.active_viewport_tab = if self.active_viewport_tab == 0 {
                    2
                } else {
                    self.active_viewport_tab - 1
                };
            }
            EditorAction::ViewportScene => {
                self.active_viewport_tab = 0;
            }
            EditorAction::ViewportGame => {
                self.active_viewport_tab = 1;
            }
            EditorAction::ViewportPhysics => {
                self.active_viewport_tab = 2;
            }
            _ => {}
        }
    }

    /// Undo the last action.
    pub fn undo(&mut self) {
        let mut cm = std::mem::take(&mut self.command_manager);
        if cm.undo(self).is_some() {
            self.status_message = Some(format!(
                "Undo: {}",
                cm.redo_description().unwrap_or_default()
            ));
        } else {
            self.status_message = Some("Nothing to undo".into());
        }
        self.command_manager = cm;
    }

    /// Redo the last undone action.
    pub fn redo(&mut self) {
        let mut cm = std::mem::take(&mut self.command_manager);
        if cm.redo(self).is_some() {
            self.status_message = Some(format!(
                "Redo: {}",
                cm.undo_description().unwrap_or_default()
            ));
        } else {
            self.status_message = Some("Nothing to redo".into());
        }
        self.command_manager = cm;
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
        if selected.is_empty() {
            return;
        }
        // Record undo commands for each deleted node
        let mut cm = std::mem::take(&mut self.command_manager);
        for &node_id in &selected {
            let cmd = crate::commands::DeleteEntityCommand::new(self, node_id);
            cm.execute(Box::new(cmd), self);
        }
        self.command_manager = cm;
        self.selected_nodes.clear();
        self.status_message = Some(format!("Deleted {} nodes", selected.len()));
    }

    /// Focus camera on the first selected object.
    fn focus_on_selection(&mut self) {
        if let Some(&first_id) = self.selected_nodes.first()
            && let Some(t) = self.node_transforms.get(&first_id)
        {
            self.camera.target = engine_math::Vec3::new(t[0], t[1], t[2]);
            self.camera.distance = 5.0;
            self.status_message = Some("Focused on selection".into());
        }
    }

    /// Load a glTF/GLB model file and register its meshes.
    pub fn load_model(&mut self, path: &std::path::Path) {
        use engine_asset::format::gltf;
        use engine_render::resource::mesh::MeshVertex;

        match gltf::load_gltf(path) {
            Ok(data) => {
                let mut count = 0;
                for mesh in &data.meshes {
                    let vertices: Vec<MeshVertex> = mesh
                        .vertices
                        .iter()
                        .map(|v| MeshVertex {
                            position: v.position,
                            normal: v.normal,
                            uv: v.tex_coord,
                        })
                        .collect();
                    let indices = mesh.indices.clone();
                    let name = if mesh.name.is_empty() {
                        format!(
                            "{}_{}",
                            path.file_stem().unwrap_or_default().to_string_lossy(),
                            count
                        )
                    } else {
                        mesh.name.clone()
                    };
                    self.loaded_models.insert(name.clone(), (vertices, indices));
                    self.log_info(&format!("已加载网格: {}", name));
                    count += 1;
                }
                self.log_info(&format!(
                    "模型已加载: {} ({} 个网格)",
                    path.display(),
                    count
                ));
                self.status_message = Some(format!("已加载模型: {}", path.display()));
            }
            Err(e) => {
                self.log_error(&format!("模型加载失败: {}", e));
                self.status_message = Some(format!("模型加载失败: {}", e));
            }
        }
    }

    /// Create a prefab from the selected nodes.
    pub fn create_prefab_from_selection(&mut self, name: &str) {
        use engine_scene::prefab::{ComponentTemplate, PrefabDef, PrefabNode};
        use engine_scene::serialization::PropertyValue;

        if self.selected_nodes.is_empty() {
            self.status_message = Some("请先选择对象".into());
            return;
        }

        let root_id = self.selected_nodes[0];
        let root_node = match self.scene_tree.nodes.iter().find(|n| n.id == root_id) {
            Some(n) => n,
            None => return,
        };

        let mut prefab = PrefabDef::new(name);
        prefab.root.name = root_node.name.clone();

        // Set transform from node
        if let Some(t) = self.node_transforms.get(&root_id) {
            prefab.root.transform = engine_scene::serialization::TransformData {
                translation: [t[0], t[1], t[2]],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [t[6], t[7], t[8]],
            };
        }

        // Add material component if exists
        if let Some(mat) = self.node_materials.get(&root_id) {
            prefab.root.components.push(
                ComponentTemplate::new("Material")
                    .with_property("base_color", PropertyValue::Vec4(mat.base_color))
                    .with_property("metallic", PropertyValue::Float(mat.metallic))
                    .with_property("roughness", PropertyValue::Float(mat.roughness)),
            );
        }

        // Add child nodes
        for child_id in &root_node.children {
            if let Some(child) = self.scene_tree.nodes.iter().find(|n| n.id == *child_id) {
                let mut child_node = PrefabNode::new(&child.name);
                if let Some(t) = self.node_transforms.get(child_id) {
                    child_node.transform = engine_scene::serialization::TransformData {
                        translation: [t[0], t[1], t[2]],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [t[6], t[7], t[8]],
                    };
                }
                prefab.root.children.push(child_node);
            }
        }

        self.prefabs.insert(name.to_string(), prefab);
        self.log_info(&format!("预制件已创建: {}", name));
        self.status_message = Some(format!("预制件已创建: {}", name));
    }

    /// Instantiate a prefab into the scene.
    pub fn instantiate_prefab(&mut self, name: &str, position: [f32; 3]) {
        let prefab = match self.prefabs.get(name) {
            Some(p) => p,
            None => {
                self.status_message = Some(format!("预制件不存在: {}", name));
                return;
            }
        };

        let parent = self.scene_tree.root_ids.first().copied();
        let root_id = self.scene_tree.add_node(&prefab.root.name, parent);

        // Set root transform
        let mut t = [0.0; 9];
        t[0] = position[0] + prefab.root.transform.translation[0];
        t[1] = position[1] + prefab.root.transform.translation[1];
        t[2] = position[2] + prefab.root.transform.translation[2];
        t[6] = prefab.root.transform.scale[0];
        t[7] = prefab.root.transform.scale[1];
        t[8] = prefab.root.transform.scale[2];
        self.node_transforms.insert(root_id, t);

        // Create child nodes
        for child_node in &prefab.root.children {
            let child_id = self.scene_tree.add_node(&child_node.name, Some(root_id));
            let mut ct = [0.0; 9];
            ct[0] = position[0] + child_node.transform.translation[0];
            ct[1] = position[1] + child_node.transform.translation[1];
            ct[2] = position[2] + child_node.transform.translation[2];
            ct[6] = child_node.transform.scale[0];
            ct[7] = child_node.transform.scale[1];
            ct[8] = child_node.transform.scale[2];
            self.node_transforms.insert(child_id, ct);
        }

        self.selected_nodes = vec![root_id];
        self.log_info(&format!("预制件已实例化: {}", name));
        self.status_message = Some(format!("预制件已实例化: {}", name));
    }

    /// Save a prefab to a file.
    pub fn save_prefab(&self, name: &str, path: &std::path::Path) -> anyhow::Result<()> {
        let prefab = self
            .prefabs
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("预制件不存在: {}", name))?;
        let json = serde_json::to_string_pretty(prefab)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a prefab from a file.
    pub fn load_prefab(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = std::fs::read_to_string(path)?;
        let prefab: engine_scene::prefab::PrefabDef = serde_json::from_str(&json)?;
        let name = prefab.name.clone();
        self.prefabs.insert(name.clone(), prefab);
        self.log_info(&format!("预制件已加载: {}", name));
        Ok(())
    }

    /// Cut selected nodes (copy + delete).
    pub fn cut_selected(&mut self) {
        self.copy_selected();
        self.delete_selected();
        self.status_message = Some("已剪切".into());
    }

    /// Copy selected nodes to clipboard (stores transform data).
    pub fn copy_selected(&mut self) {
        self.clipboard.clear();
        for &id in &self.selected_nodes {
            if let Some(t) = self.node_transforms.get(&id) {
                self.clipboard
                    .push((*t, self.node_materials.get(&id).cloned()));
            }
        }
        self.status_message = Some(format!("已复制 {} 个对象", self.clipboard.len()));
    }

    /// Paste clipboard contents as new nodes.
    pub fn paste(&mut self) {
        if self.clipboard.is_empty() {
            self.status_message = Some("剪贴板为空".into());
            return;
        }
        self.selected_nodes.clear();
        let parent = self.scene_tree.root_ids.first().copied();
        for (transform, material) in &self.clipboard {
            let new_id = self.scene_tree.add_node("Pasted", parent);
            let mut t = *transform;
            t[0] += 1.0; // offset slightly
            self.node_transforms.insert(new_id, t);
            if let Some(mat) = material {
                self.node_materials.insert(new_id, mat.clone());
            }
            self.selected_nodes.push(new_id);
        }
        self.status_message = Some(format!("已粘贴 {} 个对象", self.clipboard.len()));
    }

    /// Add a log message to the console.
    pub fn log(&mut self, level: LogLevel, message: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let hours = (secs / 3600) % 24;
        let minutes = (secs / 60) % 60;
        let seconds = secs % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        self.log_messages.push(LogEntry {
            timestamp,
            level,
            message,
        });
        // Keep last 1000 messages
        if self.log_messages.len() > 1000 {
            self.log_messages.remove(0);
        }
    }

    /// Add an info log message.
    pub fn log_info(&mut self, msg: &str) {
        self.log(LogLevel::Info, msg.to_string());
    }

    /// Add a warning log message.
    pub fn log_warn(&mut self, msg: &str) {
        self.log(LogLevel::Warn, msg.to_string());
    }

    /// Add an error log message.
    pub fn log_error(&mut self, msg: &str) {
        self.log(LogLevel::Error, msg.to_string());
    }

    /// Build the project by running `cargo build`.
    pub fn build_project(&mut self) {
        self.log_info("开始构建项目...");
        self.status_message = Some("构建中...".into());

        let output = std::process::Command::new("cargo")
            .arg("build")
            .arg("--manifest-path")
            .arg("Cargo.toml")
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    self.log_info("构建成功!");
                    self.status_message = Some("构建成功".into());
                    if !stdout.is_empty() {
                        for line in stdout.lines().take(20) {
                            self.log_info(line);
                        }
                    }
                } else {
                    self.log_error("构建失败!");
                    self.status_message = Some("构建失败".into());
                    if !stderr.is_empty() {
                        for line in stderr.lines().take(30) {
                            self.log_error(line);
                        }
                    }
                }
            }
            Err(e) => {
                self.log_error(&format!("无法执行 cargo: {}", e));
                self.status_message = Some(format!("构建错误: {}", e));
            }
        }
    }

    /// Run the project by entering play mode.
    pub fn run_project(&mut self) {
        self.play();
    }

    /// Build scene data for 3D rendering from the current editor state.
    pub fn build_scene(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        aspect: f32,
        camera: &EditorCamera,
    ) -> EditorSceneData {
        let _span = tracing::info_span!("build_scene").entered();
        let mut mesh_store = MeshStore::new();
        let mut material_store = MaterialStore::new(device);
        material_store.init_default_texture(queue);

        // Upload all primitive meshes
        let cube_mesh_id = mesh_store.upload(device, &cube_vertices(), Some(&cube_indices()));
        let (sphere_v, sphere_i) = sphere_mesh(16, 32);
        let sphere_mesh_id = mesh_store.upload(device, &sphere_v, Some(&sphere_i));
        let (plane_v, plane_i) = plane_mesh(10);
        let plane_mesh_id = mesh_store.upload(device, &plane_v, Some(&plane_i));
        let (cyl_v, cyl_i) = cylinder_mesh(24);
        let cylinder_mesh_id = mesh_store.upload(device, &cyl_v, Some(&cyl_i));

        let mut mesh_map: std::collections::HashMap<String, u64> = [
            ("Cube".to_string(), cube_mesh_id),
            ("Sphere".to_string(), sphere_mesh_id),
            ("Plane".to_string(), plane_mesh_id),
            ("Cylinder".to_string(), cylinder_mesh_id),
        ]
        .into();

        // Upload loaded model meshes
        for (name, (verts, idxs)) in &self.loaded_models {
            let mid = mesh_store.upload(device, verts, Some(idxs));
            mesh_map.insert(name.clone(), mid);
        }

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
        let mut batch_map: std::collections::HashMap<(u64, u64), usize> =
            std::collections::HashMap::new();

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

            // Select mesh based on node's render data
            let mesh_id = self
                .node_render
                .get(&node.id)
                .and_then(|(_, mesh_name, _)| mesh_map.get(mesh_name.as_str()))
                .copied()
                .unwrap_or(cube_mesh_id);

            let key = (mesh_id, mat_id);
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

        // Compute scene AABB from actual object positions
        let mut aabb_min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut aabb_max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        for node in &self.scene_tree.nodes {
            if node.parent.is_none() {
                continue;
            }
            if let Some(t) = self.node_transforms.get(&node.id) {
                let pos = Vec3::new(t[0], t[1], t[2]);
                let half = Vec3::new(
                    t[6].abs().max(0.5),
                    t[7].abs().max(0.5),
                    t[8].abs().max(0.5),
                );
                aabb_min = aabb_min.min(pos - half);
                aabb_max = aabb_max.max(pos + half);
            }
        }
        let margin = Vec3::new(5.0, 5.0, 5.0);
        aabb_min -= margin;
        aabb_max += margin;

        EditorSceneData {
            mesh_store,
            material_store,
            batches,
            lighting,
            light_direction,
            camera_vp,
            camera_pos,
            shadow_config: ShadowMapConfig::default(),
            scene_aabb_min: aabb_min,
            scene_aabb_max: aabb_max,
        }
    }

    /// Reset editor state to a blank new scene.
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
        MeshVertex {
            position: [-0.5, -0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 1.0],
        },
        MeshVertex {
            position: [0.5, -0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 1.0],
        },
        MeshVertex {
            position: [0.5, 0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 0.0],
        },
        MeshVertex {
            position: [-0.5, 0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
        },
        // Back face
        MeshVertex {
            position: [0.5, -0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
            uv: [0.0, 1.0],
        },
        MeshVertex {
            position: [-0.5, -0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
            uv: [1.0, 1.0],
        },
        MeshVertex {
            position: [-0.5, 0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
            uv: [1.0, 0.0],
        },
        MeshVertex {
            position: [0.5, 0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
            uv: [0.0, 0.0],
        },
        // Top face
        MeshVertex {
            position: [-0.5, 0.5, 0.5],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 1.0],
        },
        MeshVertex {
            position: [0.5, 0.5, 0.5],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 1.0],
        },
        MeshVertex {
            position: [0.5, 0.5, -0.5],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
        },
        MeshVertex {
            position: [-0.5, 0.5, -0.5],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        },
        // Bottom face
        MeshVertex {
            position: [-0.5, -0.5, -0.5],
            normal: [0.0, -1.0, 0.0],
            uv: [0.0, 1.0],
        },
        MeshVertex {
            position: [0.5, -0.5, -0.5],
            normal: [0.0, -1.0, 0.0],
            uv: [1.0, 1.0],
        },
        MeshVertex {
            position: [0.5, -0.5, 0.5],
            normal: [0.0, -1.0, 0.0],
            uv: [1.0, 0.0],
        },
        MeshVertex {
            position: [-0.5, -0.5, 0.5],
            normal: [0.0, -1.0, 0.0],
            uv: [0.0, 0.0],
        },
        // Right face
        MeshVertex {
            position: [0.5, -0.5, 0.5],
            normal: [1.0, 0.0, 0.0],
            uv: [0.0, 1.0],
        },
        MeshVertex {
            position: [0.5, -0.5, -0.5],
            normal: [1.0, 0.0, 0.0],
            uv: [1.0, 1.0],
        },
        MeshVertex {
            position: [0.5, 0.5, -0.5],
            normal: [1.0, 0.0, 0.0],
            uv: [1.0, 0.0],
        },
        MeshVertex {
            position: [0.5, 0.5, 0.5],
            normal: [1.0, 0.0, 0.0],
            uv: [0.0, 0.0],
        },
        // Left face
        MeshVertex {
            position: [-0.5, -0.5, -0.5],
            normal: [-1.0, 0.0, 0.0],
            uv: [0.0, 1.0],
        },
        MeshVertex {
            position: [-0.5, -0.5, 0.5],
            normal: [-1.0, 0.0, 0.0],
            uv: [1.0, 1.0],
        },
        MeshVertex {
            position: [-0.5, 0.5, 0.5],
            normal: [-1.0, 0.0, 0.0],
            uv: [1.0, 0.0],
        },
        MeshVertex {
            position: [-0.5, 0.5, -0.5],
            normal: [-1.0, 0.0, 0.0],
            uv: [0.0, 0.0],
        },
    ]
}

fn cube_indices() -> [u32; 36] {
    [
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // top
        12, 13, 14, 14, 15, 12, // bottom
        16, 17, 18, 18, 19, 16, // right
        20, 21, 22, 22, 23, 20, // left
    ]
}

fn sphere_mesh(
    stacks: u32,
    slices: u32,
) -> (Vec<engine_render::resource::mesh::MeshVertex>, Vec<u32>) {
    use engine_render::resource::mesh::MeshVertex;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for i in 0..=stacks {
        let phi = std::f32::consts::PI * i as f32 / stacks as f32;
        for j in 0..=slices {
            let theta = std::f32::consts::TAU * j as f32 / slices as f32;
            let x = phi.sin() * theta.cos();
            let y = phi.cos();
            let z = phi.sin() * theta.sin();
            vertices.push(MeshVertex {
                position: [x * 0.5, y * 0.5, z * 0.5],
                normal: [x, y, z],
                uv: [j as f32 / slices as f32, i as f32 / stacks as f32],
            });
        }
    }
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * (slices + 1) + j;
            let b = a + slices + 1;
            indices.push(a);
            indices.push(b);
            indices.push(a + 1);
            indices.push(a + 1);
            indices.push(b);
            indices.push(b + 1);
        }
    }
    (vertices, indices)
}

fn plane_mesh(subdivisions: u32) -> (Vec<engine_render::resource::mesh::MeshVertex>, Vec<u32>) {
    use engine_render::resource::mesh::MeshVertex;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let step = 1.0 / subdivisions as f32;
    for i in 0..=subdivisions {
        for j in 0..=subdivisions {
            let x = -0.5 + j as f32 * step;
            let z = -0.5 + i as f32 * step;
            vertices.push(MeshVertex {
                position: [x, 0.0, z],
                normal: [0.0, 1.0, 0.0],
                uv: [
                    j as f32 / subdivisions as f32,
                    i as f32 / subdivisions as f32,
                ],
            });
        }
    }
    for i in 0..subdivisions {
        for j in 0..subdivisions {
            let a = i * (subdivisions + 1) + j;
            let b = a + subdivisions + 1;
            indices.push(a);
            indices.push(b);
            indices.push(a + 1);
            indices.push(a + 1);
            indices.push(b);
            indices.push(b + 1);
        }
    }
    (vertices, indices)
}

fn cylinder_mesh(slices: u32) -> (Vec<engine_render::resource::mesh::MeshVertex>, Vec<u32>) {
    use engine_render::resource::mesh::MeshVertex;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    // Side vertices
    for i in 0..=1 {
        let y = -0.5 + i as f32;
        for j in 0..=slices {
            let theta = std::f32::consts::TAU * j as f32 / slices as f32;
            let x = theta.cos() * 0.5;
            let z = theta.sin() * 0.5;
            vertices.push(MeshVertex {
                position: [x, y, z],
                normal: [theta.cos(), 0.0, theta.sin()],
                uv: [j as f32 / slices as f32, i as f32],
            });
        }
    }
    // Side indices
    for j in 0..slices {
        let a = j;
        let b = j + slices + 1;
        indices.push(a);
        indices.push(b);
        indices.push(a + 1);
        indices.push(a + 1);
        indices.push(b);
        indices.push(b + 1);
    }
    // Top cap
    let top_center = vertices.len() as u32;
    vertices.push(MeshVertex {
        position: [0.0, 0.5, 0.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.5, 0.5],
    });
    for j in 0..slices {
        let theta = std::f32::consts::TAU * j as f32 / slices as f32;
        vertices.push(MeshVertex {
            position: [theta.cos() * 0.5, 0.5, theta.sin() * 0.5],
            normal: [0.0, 1.0, 0.0],
            uv: [(theta.cos() + 1.0) * 0.5, (theta.sin() + 1.0) * 0.5],
        });
    }
    for j in 0..slices {
        indices.push(top_center);
        indices.push(top_center + 1 + j);
        indices.push(top_center + 1 + (j + 1) % slices);
    }
    // Bottom cap
    let bot_center = vertices.len() as u32;
    vertices.push(MeshVertex {
        position: [0.0, -0.5, 0.0],
        normal: [0.0, -1.0, 0.0],
        uv: [0.5, 0.5],
    });
    for j in 0..slices {
        let theta = std::f32::consts::TAU * j as f32 / slices as f32;
        vertices.push(MeshVertex {
            position: [theta.cos() * 0.5, -0.5, theta.sin() * 0.5],
            normal: [0.0, -1.0, 0.0],
            uv: [(theta.cos() + 1.0) * 0.5, (theta.sin() + 1.0) * 0.5],
        });
    }
    for j in 0..slices {
        indices.push(bot_center);
        indices.push(bot_center + 1 + (j + 1) % slices);
        indices.push(bot_center + 1 + j);
    }
    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_tree_new_has_root() {
        let tree = SceneTree::new();
        assert_eq!(tree.nodes.len(), 9);
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
