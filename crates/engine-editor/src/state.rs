use egui::{Context, Pos2};
use engine_math::{Mat4, Vec3};
use engine_ui::GuiSkin;

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
    next_id: u64,
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
    pub show_grid: bool,
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
            show_grid: true,
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
        }
    }

    /// Runs one frame of the editor UI, drawing all panels via egui.
    pub fn frame(&mut self, ctx: &Context, skin: &GuiSkin) {
        crate::layout::frame(self, ctx, skin);
    }
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
