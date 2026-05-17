use egui::{Context, Pos2};
use engine_math::{Mat4, Vec3};
use engine_ui::GuiSkin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Select,
    Translate,
    Rotate,
    Scale,
}

#[derive(Debug, Clone)]
pub struct GizmoInteraction {
    pub axis: u8,
    pub plane: Option<u8>,
    pub start_mouse: Pos2,
    pub start_value: f32,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: u64,
    pub name: String,
    pub icon: String,
    pub expanded: bool,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct SceneTree {
    pub nodes: Vec<TreeNode>,
    pub root_ids: Vec<u64>,
    next_id: u64,
}

impl SceneTree {
    pub fn new() -> Self {
        let root_id = 1;
        Self {
            nodes: vec![
                TreeNode { id: 1, name: "Root".into(), icon: "📁".into(), expanded: true, parent: None, children: vec![2, 3, 4, 5, 6] },
                TreeNode { id: 2, name: "Child 1".into(), icon: "📦".into(), expanded: false, parent: Some(1), children: vec![] },
                TreeNode { id: 3, name: "Child 2".into(), icon: "📦".into(), expanded: false, parent: Some(1), children: vec![] },
                TreeNode { id: 4, name: "Child 3".into(), icon: "📦".into(), expanded: false, parent: Some(1), children: vec![] },
                TreeNode { id: 5, name: "Child 4".into(), icon: "📦".into(), expanded: false, parent: Some(1), children: vec![] },
                TreeNode { id: 6, name: "Child 5".into(), icon: "📦".into(), expanded: false, parent: Some(1), children: vec![] },
            ],
            root_ids: vec![root_id],
            next_id: 7,
        }
    }

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

    pub fn remove_node(&mut self, id: u64) {
        let parent_id = self.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        if let Some(pid) = parent_id {
            if let Some(p) = self.nodes.iter_mut().find(|n| n.id == pid) {
                p.children.retain(|c| *c != id);
            }
        }
        let mut to_remove = vec![id];
        let mut i = 0;
        while i < to_remove.len() {
            let cids: Vec<u64> = self.nodes
                .iter()
                .filter(|n| n.parent == Some(to_remove[i]))
                .map(|n| n.id)
                .collect();
            to_remove.extend(cids);
            i += 1;
        }
        self.nodes.retain(|n| !to_remove.contains(&n.id));
    }

    pub fn reparent(&mut self, id: u64, new_parent: Option<u64>) {
        let old_parent = self.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        if let Some(pid) = old_parent {
            if let Some(p) = self.nodes.iter_mut().find(|n| n.id == pid) {
                p.children.retain(|c| *c != id);
            }
        }
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.parent = new_parent;
        }
        if let Some(npid) = new_parent {
            if let Some(p) = self.nodes.iter_mut().find(|n| n.id == npid) {
                p.children.push(id);
            }
        }
    }

    pub fn rename(&mut self, id: u64, name: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.name = name.to_string();
        }
    }

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

#[derive(Debug, Clone)]
pub struct EditorCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f64,
    pub near: f64,
    pub far: f64,
}

impl EditorCamera {
    pub fn new() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: 0.0,
            fov: 60.0_f64.to_radians(),
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * 0.005;
        self.pitch = (self.pitch + delta_y * 0.005).clamp(-1.55, 1.55);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.up();
        let speed = self.distance * 0.002;
        self.target -= right * delta_x * speed;
        self.target += up * delta_y * speed;
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * 1.1_f32.powf(-delta)).clamp(0.5, 500.0);
    }

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

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, self.up())
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov as f32, aspect, self.near as f32, self.far as f32)
    }
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self::new()
    }
}

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
}

impl EditorState {
    pub fn new() -> Self {
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
        }
    }

    pub fn frame(&mut self, _ctx: &Context, _skin: &GuiSkin) {}
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
        let root = tree.nodes.iter().find(|n| n.id == root_id).unwrap();
        assert!(root.children.contains(&child));
    }

    #[test]
    fn test_remove_node_cascading() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let child = tree.add_node("Child", Some(root_id));
        let grandchild = tree.add_node("Grandchild", Some(child));
        let n_before = tree.nodes.len();
        tree.remove_node(child);
        assert_eq!(tree.nodes.len(), n_before - 2);
        assert!(!tree.nodes.iter().any(|n| n.id == grandchild));
    }

    #[test]
    fn test_reparent_moves_node() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let a = tree.add_node("A", Some(root_id));
        let b = tree.add_node("B", Some(root_id));
        tree.reparent(a, Some(b));
        let node_b = tree.nodes.iter().find(|n| n.id == b).unwrap();
        assert!(node_b.children.contains(&a));
    }

    #[test]
    fn test_rename_changes_name() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let node = tree.add_node("Old", Some(root_id));
        tree.rename(node, "New");
        let n = tree.nodes.iter().find(|n| n.id == node).unwrap();
        assert_eq!(n.name, "New");
    }

    #[test]
    fn test_search_finds_by_name() {
        let mut tree = SceneTree::new();
        let root_id = tree.root_ids[0];
        let node = tree.add_node("PlayerCharacter", Some(root_id));
        let results = tree.search("player");
        assert!(results.contains(&node));
    }

    #[test]
    fn test_search_empty_query_returns_empty() {
        let tree = SceneTree::new();
        assert!(tree.search("").is_empty());
    }

    #[test]
    fn test_camera_initial_state() {
        let cam = EditorCamera::new();
        assert_eq!(cam.target, Vec3::ZERO);
        assert!((cam.distance - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_camera_orbit_clamps_pitch() {
        let mut cam = EditorCamera::new();
        cam.orbit(0.0, 1000.0);
        assert!(cam.pitch < 1.56);
        cam.orbit(0.0, -1000.0);
        assert!(cam.pitch > -1.56);
    }

    #[test]
    fn test_camera_zoom_clamps() {
        let mut cam = EditorCamera::new();
        cam.zoom(100.0);
        assert!((cam.distance - 0.5).abs() < 1e-6);
        cam.zoom(-100.0);
        assert!((cam.distance - 500.0).abs() < 1e-6);
    }

    #[test]
    fn test_view_matrix_returns_identity_equivalent() {
        let cam = EditorCamera::new();
        let view = cam.view_matrix();
        // In RH: camera at (0,0,10) looking at origin → z translation = -10
        assert!((view.w_axis[2] - (-10.0)).abs() < 1e-4);
    }
}
