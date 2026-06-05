use engine_editor::commands::{CommandManager, CreateEntityCommand};
use engine_editor::resource_browser::ResourceBrowser;
use engine_editor::scene_serializer::{
    ComponentData, PropertyValue, Scene, SceneEntity, SceneManager,
};
use engine_editor::state::{EditorCamera, EditorState, SceneTree, ToolType};

// ── Editor creation ──

#[test]
fn editor_state_new_has_default_tree() {
    let state = EditorState::new();
    assert_eq!(state.scene_tree.nodes.len(), 6);
    assert_eq!(state.scene_tree.root_ids.len(), 1);
}

#[test]
fn editor_state_default_tool_is_translate() {
    let state = EditorState::new();
    assert_eq!(state.active_tool, ToolType::Translate);
}

#[test]
fn editor_state_new_has_camera() {
    let state = EditorState::new();
    assert!(state.camera.distance > 0.0);
    assert!(state.camera.fov > 0.0);
}

#[test]
fn editor_state_new_has_resource_browser() {
    let state = EditorState::new();
    assert!(!state.resource_browser.entries.is_empty());
    assert_eq!(state.resource_browser.current_path, "Assets");
}

#[test]
fn editor_state_new_has_scene_manager() {
    let state = EditorState::new();
    assert!(state.scene_manager.current_scene().is_none());
}

// ── Hierarchy panel ──

#[test]
fn scene_tree_new_creates_root_with_children() {
    let tree = SceneTree::new();
    let root_id = tree.root_ids[0];
    let root = tree.nodes.iter().find(|n| n.id == root_id).unwrap();
    assert_eq!(root.name, "Root");
    assert!(root.parent.is_none());
    assert_eq!(root.children.len(), 5);
}

#[test]
fn scene_tree_add_node_creates_child() {
    let mut tree = SceneTree::new();
    let root_id = tree.root_ids[0];
    let child_id = tree.add_node("TestNode", Some(root_id));
    let child = tree.nodes.iter().find(|n| n.id == child_id).unwrap();
    assert_eq!(child.name, "TestNode");
    assert_eq!(child.parent, Some(root_id));
    let root = tree.nodes.iter().find(|n| n.id == root_id).unwrap();
    assert!(root.children.contains(&child_id));
}

#[test]
fn scene_tree_add_node_default_parent_is_root() {
    let mut tree = SceneTree::new();
    let root_id = tree.root_ids[0];
    let child_id = tree.add_node("Orphan", None);
    let child = tree.nodes.iter().find(|n| n.id == child_id).unwrap();
    assert_eq!(child.parent, Some(root_id));
}

#[test]
fn scene_tree_remove_node_cascading() {
    let mut tree = SceneTree::new();
    let root_id = tree.root_ids[0];
    let child_id = tree.add_node("Parent", Some(root_id));
    let _grandchild = tree.add_node("Child", Some(child_id));
    let n_before = tree.nodes.len();
    tree.remove_node(child_id);
    assert_eq!(tree.nodes.len(), n_before - 2);
    assert!(!tree.nodes.iter().any(|n| n.id == child_id));
}

#[test]
fn scene_tree_reparent_moves_node() {
    let mut tree = SceneTree::new();
    let root_id = tree.root_ids[0];
    let a = tree.add_node("A", Some(root_id));
    let b = tree.add_node("B", Some(root_id));
    tree.reparent(a, Some(b));
    let a_node = tree.nodes.iter().find(|n| n.id == a).unwrap();
    assert_eq!(a_node.parent, Some(b));
    let b_node = tree.nodes.iter().find(|n| n.id == b).unwrap();
    assert!(b_node.children.contains(&a));
}

#[test]
fn scene_tree_rename_changes_name() {
    let mut tree = SceneTree::new();
    let root_id = tree.root_ids[0];
    let child_id = tree.add_node("Old", Some(root_id));
    tree.rename(child_id, "New");
    let node = tree.nodes.iter().find(|n| n.id == child_id).unwrap();
    assert_eq!(node.name, "New");
}

// ── Hierarchy search ──

#[test]
fn scene_tree_search_finds_matching_nodes() {
    let tree = SceneTree::new();
    let results = tree.search("player");
    assert!(!results.is_empty());
    assert!(results.iter().any(|&id| {
        tree.nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.to_lowercase().contains("player"))
            .unwrap_or(false)
    }));
}

#[test]
fn scene_tree_search_case_insensitive() {
    let tree = SceneTree::new();
    let upper = tree.search("PLAYER");
    let lower = tree.search("player");
    assert_eq!(upper.len(), lower.len());
    assert!(!upper.is_empty());
}

#[test]
fn scene_tree_search_empty_query_returns_empty() {
    let tree = SceneTree::new();
    assert!(tree.search("").is_empty());
}

#[test]
fn scene_tree_search_no_match_returns_empty() {
    let tree = SceneTree::new();
    assert!(tree.search("nonexistent_xyz").is_empty());
}

#[test]
fn scene_tree_search_finds_all_matching() {
    let mut tree = SceneTree::new();
    tree.add_node("TestA", None);
    tree.add_node("TestB", None);
    let results = tree.search("test");
    assert_eq!(results.len(), 2);
}

// ── Inspector panel ──

#[test]
fn editor_state_has_node_transforms() {
    let state = EditorState::new();
    assert_eq!(state.node_transforms.len(), 6);
    for i in 1..=6 {
        assert!(state.node_transforms.contains_key(&i));
    }
}

#[test]
fn editor_state_has_node_materials() {
    let state = EditorState::new();
    assert!(state.node_materials.contains_key(&4));
    assert!(state.node_materials.contains_key(&5));
}

#[test]
fn editor_state_has_node_lights() {
    let state = EditorState::new();
    assert!(state.node_lights.contains_key(&6));
}

#[test]
fn editor_state_has_node_physics() {
    let state = EditorState::new();
    assert_eq!(state.node_physics.len(), 6);
}

#[test]
fn editor_state_selection_starts_empty() {
    let state = EditorState::new();
    assert!(state.selected_nodes.is_empty());
}

// ── Resource browser ──

#[test]
fn resource_browser_new_has_entries() {
    let browser = ResourceBrowser::new();
    assert!(!browser.entries.is_empty());
}

#[test]
fn resource_browser_has_directories() {
    let browser = ResourceBrowser::new();
    let dirs: Vec<_> = browser.entries.iter().filter(|e| e.is_directory).collect();
    assert!(!dirs.is_empty());
    assert!(dirs.iter().any(|d| d.name == "Images"));
    assert!(dirs.iter().any(|d| d.name == "Audio"));
}

#[test]
fn resource_browser_has_files() {
    let browser = ResourceBrowser::new();
    let files: Vec<_> = browser.entries.iter().filter(|e| !e.is_directory).collect();
    assert!(!files.is_empty());
}

#[test]
fn resource_browser_default_path_is_assets() {
    let browser = ResourceBrowser::new();
    assert_eq!(browser.current_path, "Assets");
}

#[test]
fn resource_browser_no_selection_by_default() {
    let browser = ResourceBrowser::new();
    assert!(browser.selected_entry.is_none());
}

// ── Scene serialization ──

#[test]
fn scene_new_creates_empty_scene() {
    let scene = Scene::new("TestScene".to_string());
    assert_eq!(scene.name, "TestScene");
    assert!(scene.entities.is_empty());
}

#[test]
fn scene_add_and_get_entity() {
    let mut scene = Scene::new("Test".to_string());
    let entity = SceneEntity::new(1, "Player".to_string());
    scene.add_entity(entity);
    assert_eq!(scene.entities.len(), 1);
    assert_eq!(scene.get_entity(1).unwrap().name, "Player");
}

#[test]
fn scene_remove_entity() {
    let mut scene = Scene::new("Test".to_string());
    scene.add_entity(SceneEntity::new(1, "A".to_string()));
    scene.add_entity(SceneEntity::new(2, "B".to_string()));
    let removed = scene.remove_entity(1);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "A");
    assert_eq!(scene.entities.len(), 1);
}

#[test]
fn scene_remove_nonexistent_returns_none() {
    let mut scene = Scene::new("Test".to_string());
    assert!(scene.remove_entity(999).is_none());
}

#[test]
fn scene_get_entity_mut() {
    let mut scene = Scene::new("Test".to_string());
    scene.add_entity(SceneEntity::new(1, "Old".to_string()));
    scene.get_entity_mut(1).unwrap().name = "New".to_string();
    assert_eq!(scene.get_entity(1).unwrap().name, "New");
}

#[test]
fn scene_entity_add_remove_component() {
    let mut entity = SceneEntity::new(1, "Test".to_string());
    entity.add_component(
        ComponentData::new("MeshRenderer".to_string())
            .with_property("mesh", PropertyValue::String("cube.obj".to_string())),
    );
    assert_eq!(entity.components.len(), 1);
    let removed = entity.remove_component("MeshRenderer");
    assert!(removed.is_some());
    assert!(entity.components.is_empty());
}

#[test]
fn scene_to_string_pretty_contains_info() {
    let mut scene = Scene::new("PrettyTest".to_string());
    scene.add_entity(SceneEntity::new(1, "Entity1".to_string()));
    let s = scene.to_string_pretty();
    assert!(s.contains("PrettyTest"));
    assert!(s.contains("Entity1"));
}

#[test]
fn scene_serialize_deserialize_roundtrip() {
    let mut scene = Scene::new("RoundTrip".to_string());
    scene.settings.fog_enabled = true;

    let mut entity = SceneEntity::new(1, "Player".to_string());
    entity.transform.translation = [1.0, 2.0, 3.0];
    entity.transform.scale = [2.0, 2.0, 2.0];
    entity.add_component(
        ComponentData::new("MeshRenderer".to_string())
            .with_property("mesh", PropertyValue::String("cube.obj".to_string()))
            .with_property("visible", PropertyValue::Bool(true)),
    );
    scene.add_entity(entity);

    let json = serde_json::to_string_pretty(&scene).unwrap();
    let loaded: Scene = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.name, "RoundTrip");
    assert!(loaded.settings.fog_enabled);
    assert_eq!(loaded.entities.len(), 1);
    assert_eq!(loaded.entities[0].name, "Player");
    assert_eq!(loaded.entities[0].transform.translation, [1.0, 2.0, 3.0]);
    assert_eq!(loaded.entities[0].components.len(), 1);
}

#[test]
fn scene_manager_create_and_access() {
    let mut mgr = SceneManager::new();
    assert!(mgr.current_scene().is_none());
    mgr.create_scene("MyScene".to_string());
    assert!(mgr.current_scene().is_some());
    assert_eq!(mgr.current_scene().unwrap().name, "MyScene");
}

#[test]
fn scene_manager_new_entity() {
    let mut mgr = SceneManager::new();
    mgr.create_scene("Test".to_string());
    let id = mgr.new_entity("Cube".to_string());
    assert!(id.is_some());
    assert_eq!(mgr.current_scene().unwrap().entities.len(), 1);
}

#[test]
fn scene_manager_new_entity_without_scene_returns_none() {
    let mut mgr = SceneManager::new();
    assert!(mgr.new_entity("Cube".to_string()).is_none());
}

#[test]
fn scene_manager_modification_tracking() {
    let mut mgr = SceneManager::new();
    mgr.create_scene("Test".to_string());
    assert!(!mgr.is_modified());
    mgr.mark_modified();
    assert!(mgr.is_modified());
    mgr.mark_saved();
    assert!(!mgr.is_modified());
}

#[test]
fn scene_manager_save_and_load() {
    let dir = std::env::temp_dir().join("rust_engine_editor_test");
    let path = dir.join("test_scene.json");

    let mut mgr = SceneManager::new();
    mgr.create_scene("SaveLoadTest".to_string());
    mgr.new_entity("Cube".to_string());
    mgr.new_entity("Light".to_string());

    mgr.save_scene(&path).unwrap();
    assert!(!mgr.is_modified());
    assert_eq!(mgr.scene_path(), Some(path.as_path()));

    let mut mgr2 = SceneManager::new();
    mgr2.load_scene(&path).unwrap();

    let scene = mgr2.current_scene().unwrap();
    assert_eq!(scene.name, "SaveLoadTest");
    assert_eq!(scene.entities.len(), 2);
    assert_eq!(scene.entities[0].name, "Cube");
    assert_eq!(scene.entities[1].name, "Light");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scene_manager_save_current_without_path_fails() {
    let mut mgr = SceneManager::new();
    mgr.create_scene("NoPath".to_string());
    assert!(mgr.save_current_scene().is_err());
}

// ── Command / Undo system ──

#[test]
fn command_manager_execute_and_undo() {
    let mut mgr = CommandManager::new(100);
    assert!(!mgr.can_undo());
    assert!(!mgr.can_redo());

    let cmd = CreateEntityCommand::new(1, "Test".to_string(), None);
    mgr.execute(Box::new(cmd));
    assert!(mgr.can_undo());
    assert!(!mgr.can_redo());

    mgr.undo();
    assert!(!mgr.can_undo());
    assert!(mgr.can_redo());
}

#[test]
fn command_manager_redo() {
    let mut mgr = CommandManager::new(100);
    let cmd = CreateEntityCommand::new(1, "Test".to_string(), None);
    mgr.execute(Box::new(cmd));
    mgr.undo();
    assert!(mgr.can_redo());
    mgr.redo();
    assert!(mgr.can_undo());
    assert!(!mgr.can_redo());
}

#[test]
fn command_manager_clear() {
    let mut mgr = CommandManager::new(100);
    let cmd = CreateEntityCommand::new(1, "Test".to_string(), None);
    mgr.execute(Box::new(cmd));
    mgr.clear();
    assert!(!mgr.can_undo());
    assert!(!mgr.can_redo());
}

#[test]
fn command_manager_undo_redo_description() {
    let mut mgr = CommandManager::new(100);
    assert!(mgr.undo_description().is_none());

    let cmd = CreateEntityCommand::new(1, "Player".to_string(), None);
    mgr.execute(Box::new(cmd));
    assert_eq!(mgr.undo_description().unwrap(), "Create Player");

    mgr.undo();
    assert_eq!(mgr.redo_description().unwrap(), "Create Player");
}

#[test]
fn command_manager_max_history() {
    let mut mgr = CommandManager::new(2);
    for i in 0..5 {
        let cmd = CreateEntityCommand::new(i, format!("E{}", i), None);
        mgr.execute(Box::new(cmd));
    }
    // Only last 2 should be in undo stack
    mgr.undo();
    mgr.undo();
    // Third undo should fail (exceeded history)
    assert!(mgr.undo().is_none());
}

// ── EditorCamera ──

#[test]
fn editor_camera_orbit_clamps_pitch() {
    let mut cam = EditorCamera::new();
    cam.orbit(0.0, 10000.0);
    assert!(cam.pitch <= 1.56);
    cam.orbit(0.0, -10000.0);
    assert!(cam.pitch >= -1.56);
}

#[test]
fn editor_camera_zoom_clamps_distance() {
    let mut cam = EditorCamera::new();
    cam.zoom(10000.0);
    assert!(cam.distance >= 0.5);
    cam.zoom(-10000.0);
    assert!(cam.distance <= 500.0);
}

#[test]
fn editor_camera_eye_is_offset_from_target() {
    let cam = EditorCamera::new();
    let eye = cam.eye();
    let dist = (eye - cam.target).length();
    assert!((dist - cam.distance).abs() < 0.01);
}

#[test]
fn editor_camera_view_matrix_is_finite() {
    let cam = EditorCamera::new();
    let view = cam.view_matrix();
    for v in view.to_cols_array() {
        assert!(v.is_finite());
    }
}

#[test]
fn editor_camera_projection_matrix_is_finite() {
    let cam = EditorCamera::new();
    let proj = cam.projection_matrix(16.0 / 9.0);
    for v in proj.to_cols_array() {
        assert!(v.is_finite());
    }
}
