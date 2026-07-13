use engine_core::gameobject::{Component, GameObject, GameObjectHandle};
use engine_core::prefab::{Prefab, PrefabRegistry, PrefabValue};
use engine_core::serialization::{SceneData, SceneSerializer, load_scene_json, save_scene_json};
use engine_core::transform::Transform;
use engine_core::undo::{CreateObjectCommand, DestroyObjectCommand, UndoSystem};
use engine_core::world::World;
use engine_math::{Quat, Vec3};
use std::any::Any;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct TestComponent {
    value: i32,
}

impl Component for TestComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct HealthComponent {
    hp: f32,
}

impl Component for HealthComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[allow(dead_code)]
struct HierarchyResult {
    world: World,
    root: GameObjectHandle,
    child1: GameObjectHandle,
    child2: GameObjectHandle,
    grandchild: GameObjectHandle,
}

fn make_world_with_hierarchy() -> HierarchyResult {
    let mut world = World::new();
    let root = world.spawn(GameObject::new("Root"));
    let child1 = world.spawn(GameObject::new("Child1"));
    let child2 = world.spawn(GameObject::new("Child2"));
    world.set_parent(child1, Some(root));
    world.set_parent(child2, Some(root));

    let grandchild = world.spawn(GameObject::new("Grandchild"));
    world.set_parent(grandchild, Some(child1));

    HierarchyResult {
        world,
        root,
        child1,
        child2,
        grandchild,
    }
}

// ===========================================================================
// Prefab Workflow Integration Tests
// ===========================================================================

#[test]
fn prefab_workflow_create_and_instantiate() {
    let mut world = World::new();
    let mut go = GameObject::new("Player");
    go.set_tag("PlayerTag");
    go.set_layer(5);
    go.add_component(Transform::from_xyz(1.0, 2.0, 3.0));
    let handle = world.spawn(go);

    let player_go = world.get_gameobject(handle).unwrap();
    let prefab = Prefab::create("PlayerPrefab", player_go, &world);

    assert_eq!(prefab.name(), "PlayerPrefab");
    assert!(prefab.id().0 > 0);

    let mut new_world = World::new();
    let instance = prefab.instantiate(&mut new_world);

    assert_eq!(instance.prefab_id(), prefab.id());
    let spawned = new_world
        .get_gameobject(instance.game_object_handle())
        .unwrap();
    assert_eq!(spawned.name(), "Player");
    assert_eq!(spawned.tag(), "PlayerTag");
    assert_eq!(spawned.layer(), 5);
}

#[test]
fn prefab_workflow_registry_lifecycle() {
    let mut registry = PrefabRegistry::new();
    let world = World::new();

    let go = GameObject::new("Enemy");
    let prefab = Prefab::create("EnemyPrefab", &go, &world);
    let id = registry.register(prefab);

    assert_eq!(registry.count(), 1);
    assert!(registry.contains(id));
    assert_eq!(registry.find_by_name("EnemyPrefab").unwrap().id(), id);

    let mut new_world = World::new();
    let instance = registry.instantiate(id, &mut new_world).unwrap();
    assert_eq!(instance.prefab_id(), id);
    assert_eq!(
        new_world
            .get_gameobject(instance.game_object_handle())
            .unwrap()
            .name(),
        "Enemy"
    );

    let removed = registry.remove(id);
    assert!(removed.is_some());
    assert!(!registry.contains(id));
    assert_eq!(registry.count(), 0);
}

#[test]
fn prefab_workflow_overrides_and_revert() {
    let mut world = World::new();
    let go = GameObject::new("Tower");
    let prefab = Prefab::create("TowerPrefab", &go, &world);
    let mut instance = prefab.instantiate(&mut world);

    assert!(!instance.has_overrides());

    instance.apply_override("Health", PrefabValue::Float(100.0));
    instance.apply_override("Damage", PrefabValue::Int(25));
    instance.apply_override("Active", PrefabValue::Bool(true));
    instance.apply_override("Name", PrefabValue::String("Super Tower".into()));

    assert!(instance.has_overrides());
    assert_eq!(instance.override_count(), 4);
    assert!(instance.has_override("Health"));
    assert!(instance.has_override("Damage"));
    assert_eq!(
        instance.get_override("Health"),
        Some(&PrefabValue::Float(100.0))
    );
    assert_eq!(instance.get_override("Damage"), Some(&PrefabValue::Int(25)));

    let mut paths = instance.override_paths();
    paths.sort();
    assert_eq!(paths, vec!["Active", "Damage", "Health", "Name"]);

    // Revert single
    assert!(instance.revert("Health"));
    assert_eq!(instance.override_count(), 3);
    assert!(!instance.has_override("Health"));

    // Revert nonexistent
    assert!(!instance.revert("NonExistent"));

    // Revert all
    instance.revert_all();
    assert!(!instance.has_overrides());
    assert_eq!(instance.override_count(), 0);
}

#[test]
fn prefab_workflow_multiple_instances() {
    let mut world = World::new();
    let go = GameObject::new("Tree");
    let prefab = Prefab::create("TreePrefab", &go, &world);

    let mut instance1 = prefab.instantiate(&mut world);
    let mut instance2 = prefab.instantiate(&mut world);
    let instance3 = prefab.instantiate(&mut world);

    instance1.apply_override("Position", PrefabValue::Float(0.0));
    instance2.apply_override("Position", PrefabValue::Float(10.0));

    assert!(instance1.has_overrides());
    assert!(instance2.has_overrides());
    assert!(!instance3.has_overrides());

    // All reference the same prefab
    assert_eq!(instance1.prefab_id(), instance2.prefab_id());
    assert_eq!(instance2.prefab_id(), instance3.prefab_id());
}

#[test]
fn prefab_workflow_hierarchy_preserved_through_instantiate() {
    let h = make_world_with_hierarchy();

    let root_go = h.world.get_gameobject(h.root).unwrap();
    let prefab = Prefab::create("HierarchyPrefab", root_go, &h.world);

    let root_node = prefab.root();
    assert_eq!(root_node.name(), "Root");
    assert_eq!(root_node.children().len(), 2);
    assert_eq!(root_node.children()[0].name(), "Child1");
    assert_eq!(root_node.children()[1].name(), "Child2");
    assert_eq!(root_node.children()[0].children().len(), 1);
    assert_eq!(root_node.children()[0].children()[0].name(), "Grandchild");

    let mut new_world = World::new();
    let instance = prefab.instantiate(&mut new_world);

    let root = new_world
        .get_gameobject(instance.game_object_handle())
        .unwrap();
    assert_eq!(root.name(), "Root");
    assert_eq!(root.children().len(), 2);

    let c1 = new_world.get_gameobject(root.children()[0]).unwrap();
    assert_eq!(c1.name(), "Child1");
    assert_eq!(c1.children().len(), 1);

    let gc = new_world.get_gameobject(c1.children()[0]).unwrap();
    assert_eq!(gc.name(), "Grandchild");
}

#[test]
fn prefab_workflow_override_overwrite_and_value_types() {
    let mut world = World::new();
    let go = GameObject::new("Item");
    let prefab = Prefab::create("ItemPrefab", &go, &world);
    let mut instance = prefab.instantiate(&mut world);

    instance.apply_override("X", PrefabValue::Float(1.0));
    instance.apply_override("X", PrefabValue::Float(99.0));
    assert_eq!(instance.override_count(), 1);
    assert_eq!(instance.get_override("X"), Some(&PrefabValue::Float(99.0)));

    instance.apply_override("BoolVal", PrefabValue::Bool(false));
    instance.apply_override("IntVal", PrefabValue::Int(-1));
    instance.apply_override("StrVal", PrefabValue::String("abc".into()));

    assert_eq!(instance.override_count(), 4);
    assert_eq!(
        instance.get_override("BoolVal"),
        Some(&PrefabValue::Bool(false))
    );
    assert_eq!(instance.get_override("IntVal"), Some(&PrefabValue::Int(-1)));
    assert_eq!(
        instance.get_override("StrVal"),
        Some(&PrefabValue::String("abc".into()))
    );
}

// ===========================================================================
// Serialization Workflow Integration Tests
// ===========================================================================

#[test]
fn serialization_workflow_empty_scene_roundtrip() {
    let world = World::new();
    let json = save_scene_json(&world, "Empty").unwrap();

    let mut loaded_world = World::new();
    let handles = load_scene_json(&json, &mut loaded_world).unwrap();

    assert!(handles.is_empty());
    assert_eq!(loaded_world.count(), 0);
}

#[test]
fn serialization_workflow_single_object_roundtrip() {
    let mut world = World::new();
    let mut go = GameObject::new("Player");
    go.set_tag("Hero");
    go.set_layer(10);
    go.set_active(true);
    world.spawn(go);

    let json = save_scene_json(&world, "Level1").unwrap();
    assert!(json.contains("Player"));
    assert!(json.contains("Hero"));

    let mut loaded_world = World::new();
    let handles = load_scene_json(&json, &mut loaded_world).unwrap();
    assert_eq!(handles.len(), 1);

    let loaded = loaded_world.get_gameobject(handles[0]).unwrap();
    assert_eq!(loaded.name(), "Player");
    assert_eq!(loaded.tag(), "Hero");
    assert_eq!(loaded.layer(), 10);
    assert!(loaded.is_active());
}

#[test]
fn serialization_workflow_hierarchy_roundtrip() {
    let mut world = World::new();
    let root = world.spawn(GameObject::new("Level"));
    let child1 = world.spawn(GameObject::new("Player"));
    let child2 = world.spawn(GameObject::new("Camera"));
    world.set_parent(child1, Some(root));
    world.set_parent(child2, Some(root));

    let grandchild = world.spawn(GameObject::new("Weapon"));
    world.set_parent(grandchild, Some(child1));

    let json = save_scene_json(&world, "GameScene").unwrap();

    let mut loaded_world = World::new();
    let handles = load_scene_json(&json, &mut loaded_world).unwrap();

    assert_eq!(handles.len(), 1);
    let loaded_root = loaded_world.get_gameobject(handles[0]).unwrap();
    assert_eq!(loaded_root.name(), "Level");
    assert_eq!(loaded_root.children().len(), 2);

    let loaded_player = loaded_world
        .get_gameobject(loaded_root.children()[0])
        .unwrap();
    assert_eq!(loaded_player.name(), "Player");
    assert_eq!(loaded_player.children().len(), 1);

    let loaded_weapon = loaded_world
        .get_gameobject(loaded_player.children()[0])
        .unwrap();
    assert_eq!(loaded_weapon.name(), "Weapon");

    let loaded_camera = loaded_world
        .get_gameobject(loaded_root.children()[1])
        .unwrap();
    assert_eq!(loaded_camera.name(), "Camera");
}

#[test]
fn serialization_workflow_transform_roundtrip() {
    let mut world = World::new();
    let mut go = GameObject::new("Actor");
    let mut t = Transform::from_xyz(10.0, 20.0, 30.0);
    t.set_local_rotation(Quat::from_rotation_y(1.57));
    t.set_local_scale(Vec3::new(2.0, 3.0, 4.0));
    go.add_component(t);
    world.spawn(go);

    let serializer = SceneSerializer::new();
    let scene = serializer.save(&world, "TransformTest");

    assert_eq!(scene.game_objects[0].components.len(), 1);
    assert_eq!(scene.game_objects[0].components[0].type_name, "Transform");

    let mut loaded_world = World::new();
    let handles = serializer.load(&scene, &mut loaded_world);

    let loaded = loaded_world.get_gameobject(handles[0]).unwrap();
    let lt = loaded.get_component::<Transform>().unwrap();

    assert_eq!(lt.local_position.x, 10.0);
    assert_eq!(lt.local_position.y, 20.0);
    assert_eq!(lt.local_position.z, 30.0);
    assert_eq!(lt.local_scale.x, 2.0);
    assert_eq!(lt.local_scale.y, 3.0);
    assert_eq!(lt.local_scale.z, 4.0);

    let expected_rot = Quat::from_rotation_y(1.57);
    assert!((lt.local_rotation.x - expected_rot.x).abs() < 1e-5);
    assert!((lt.local_rotation.y - expected_rot.y).abs() < 1e-5);
    assert!((lt.local_rotation.z - expected_rot.z).abs() < 1e-5);
    assert!((lt.local_rotation.w - expected_rot.w).abs() < 1e-5);
}

#[test]
fn serialization_workflow_json_roundtrip_preserves_structure() {
    let mut world = World::new();
    let root = world.spawn(GameObject::new("Scene"));
    let child = world.spawn(GameObject::new("Prop"));
    world.set_parent(child, Some(root));

    let json = save_scene_json(&world, "JsonTest").unwrap();
    let scene_deserialized: SceneData = serde_json::from_str(&json).unwrap();

    assert_eq!(scene_deserialized.name, "JsonTest");
    assert_eq!(scene_deserialized.version, 1);
    assert_eq!(scene_deserialized.game_objects.len(), 1);
    assert_eq!(scene_deserialized.game_objects[0].children.len(), 1);
    assert_eq!(scene_deserialized.game_objects[0].children[0].name, "Prop");

    let mut loaded_world = World::new();
    let handles = load_scene_json(&json, &mut loaded_world).unwrap();
    assert_eq!(loaded_world.count(), 2);
    let loaded_root = loaded_world.get_gameobject(handles[0]).unwrap();
    assert_eq!(loaded_root.children().len(), 1);
}

#[test]
fn serialization_workflow_multiple_roots() {
    let mut world = World::new();
    world.spawn(GameObject::new("Player"));
    world.spawn(GameObject::new("Enemy"));
    world.spawn(GameObject::new("Environment"));

    let json = save_scene_json(&world, "MultiRoot").unwrap();

    let mut loaded_world = World::new();
    let handles = load_scene_json(&json, &mut loaded_world).unwrap();

    assert_eq!(handles.len(), 3);
    let mut names: Vec<&str> = handles
        .iter()
        .map(|&h| loaded_world.get_gameobject(h).unwrap().name())
        .collect();
    names.sort();
    assert_eq!(names, vec!["Enemy", "Environment", "Player"]);
}

#[test]
fn serialization_workflow_inactive_objects_preserved() {
    let mut world = World::new();
    let mut go = GameObject::new("InactiveObj");
    go.set_active(false);
    let mut t = Transform::from_xyz(5.0, 5.0, 5.0);
    t.set_local_scale(Vec3::new(0.5, 0.5, 0.5));
    go.add_component(t);
    world.spawn(go);

    let json = save_scene_json(&world, "Inactive").unwrap();

    let mut loaded_world = World::new();
    let handles = load_scene_json(&json, &mut loaded_world).unwrap();
    let loaded = loaded_world.get_gameobject(handles[0]).unwrap();

    assert!(!loaded.is_active());
    let lt = loaded.get_component::<Transform>().unwrap();
    assert_eq!(lt.local_position.x, 5.0);
    assert_eq!(lt.local_scale.x, 0.5);
}

// ===========================================================================
// Undo/Redo Workflow Integration Tests
// ===========================================================================

#[test]
fn undo_workflow_create_and_undo() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let handle = system.execute(Box::new(CreateObjectCommand::new("Player")), &mut world);

    assert!(world.is_valid(handle));
    assert_eq!(world.get_gameobject(handle).unwrap().name(), "Player");
    assert!(system.can_undo());
    assert!(!system.can_redo());

    system.undo(&mut world);

    assert!(!world.is_valid(handle));
    assert!(!system.can_undo());
    assert!(system.can_redo());
}

#[test]
fn undo_workflow_create_undo_redo_cycle() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let handle = system.execute(Box::new(CreateObjectCommand::new("Enemy")), &mut world);
    assert_eq!(world.get_gameobject(handle).unwrap().name(), "Enemy");

    system.undo(&mut world);
    assert!(!world.is_valid(handle));

    system.redo(&mut world);
    assert!(world.count() > 0);
    assert!(world.find_gameobject("Enemy").is_some());
    assert!(system.can_undo());
    assert!(!system.can_redo());
}

#[test]
fn undo_workflow_destroy_and_undo_restores() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let mut go = GameObject::new("Tree");
    go.set_tag("Environment");
    go.set_layer(3);
    let handle = world.spawn(go);

    let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
    system.execute(Box::new(cmd), &mut world);

    assert!(!world.is_valid(handle));

    system.undo(&mut world);

    assert!(world.find_gameobject("Tree").is_some());
    let restored_handle = world.find_gameobject("Tree").unwrap();
    let restored = world.get_gameobject(restored_handle).unwrap();
    assert_eq!(restored.tag(), "Environment");
    assert_eq!(restored.layer(), 3);
}

#[test]
fn undo_workflow_destroy_with_children_restores_hierarchy() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let parent = world.spawn(GameObject::new("Parent"));
    let child1 = world.spawn(GameObject::new("Child1"));
    let child2 = world.spawn(GameObject::new("Child2"));
    world.set_parent(child1, Some(parent));
    world.set_parent(child2, Some(parent));

    let cmd = DestroyObjectCommand::new(&world, parent).unwrap();
    system.execute(Box::new(cmd), &mut world);

    assert!(!world.is_valid(parent));
    assert!(!world.is_valid(child1));
    assert!(!world.is_valid(child2));

    system.undo(&mut world);

    assert!(world.find_gameobject("Parent").is_some());
    assert!(world.find_gameobject("Child1").is_some());
    assert!(world.find_gameobject("Child2").is_some());

    let restored_parent = world.find_gameobject("Parent").unwrap();
    let children = world.get_children(restored_parent);
    assert_eq!(children.len(), 2);
}

#[test]
fn undo_workflow_destroy_with_components_restores_data() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let mut go = GameObject::new("Warrior");
    go.add_component(HealthComponent { hp: 200.0 });
    go.add_component(TestComponent { value: 42 });
    let handle = world.spawn(go);

    {
        let warrior = world.get_gameobject(handle).unwrap();
        assert_eq!(
            warrior.get_component::<HealthComponent>().unwrap().hp,
            200.0
        );
        assert_eq!(warrior.get_component::<TestComponent>().unwrap().value, 42);
    }

    let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
    system.execute(Box::new(cmd), &mut world);

    system.undo(&mut world);

    let restored = world.find_gameobject("Warrior").unwrap();
    let warrior = world.get_gameobject(restored).unwrap();
    assert_eq!(
        warrior.get_component::<HealthComponent>().unwrap().hp,
        200.0
    );
    assert_eq!(warrior.get_component::<TestComponent>().unwrap().value, 42);
}

#[test]
fn undo_workflow_max_history_limits_undo() {
    let mut world = World::new();
    let mut system = UndoSystem::new(3);

    system.execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    system.execute(Box::new(CreateObjectCommand::new("B")), &mut world);
    system.execute(Box::new(CreateObjectCommand::new("C")), &mut world);
    system.execute(Box::new(CreateObjectCommand::new("D")), &mut world);

    // Only 3 commands kept, oldest (A) was dropped
    assert!(system.undo(&mut world).is_some());
    assert!(system.undo(&mut world).is_some());
    assert!(system.undo(&mut world).is_some());
    assert!(system.undo(&mut world).is_none());
}

#[test]
fn undo_workflow_new_command_clears_redo() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    system.execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    system.execute(Box::new(CreateObjectCommand::new("B")), &mut world);

    system.undo(&mut world);
    assert!(system.can_redo());

    // New command clears redo stack
    system.execute(Box::new(CreateObjectCommand::new("C")), &mut world);
    assert!(!system.can_redo());
}

#[test]
fn undo_workflow_clear_resets_everything() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    system.execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    system.execute(Box::new(CreateObjectCommand::new("B")), &mut world);
    system.undo(&mut world);

    assert!(system.can_undo());
    assert!(system.can_redo());

    system.clear();

    assert!(!system.can_undo());
    assert!(!system.can_redo());
}

#[test]
fn undo_workflow_descriptions() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    assert_eq!(system.undo_description(), None);
    assert_eq!(system.redo_description(), None);

    system.execute(Box::new(CreateObjectCommand::new("Player")), &mut world);
    assert_eq!(
        system.undo_description(),
        Some("Create 'Player'".to_string())
    );
    assert_eq!(system.redo_description(), None);

    system.undo(&mut world);
    assert_eq!(system.undo_description(), None);
    assert_eq!(
        system.redo_description(),
        Some("Create 'Player'".to_string())
    );
}

#[test]
fn undo_workflow_create_with_builder_options() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let cmd = CreateObjectCommand::new("SpecialObj")
        .with_tag("Special")
        .with_layer(7)
        .with_active(false);

    let handle = system.execute(Box::new(cmd), &mut world);

    let go = world.get_gameobject(handle).unwrap();
    assert_eq!(go.name(), "SpecialObj");
    assert_eq!(go.tag(), "Special");
    assert_eq!(go.layer(), 7);
    assert!(!go.is_active());
}

#[test]
fn undo_workflow_multiple_undo_redo_cycles() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let _h1 = system.execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    let h2 = system.execute(Box::new(CreateObjectCommand::new("B")), &mut world);
    let h3 = system.execute(Box::new(CreateObjectCommand::new("C")), &mut world);

    assert_eq!(world.count(), 3);

    system.undo(&mut world);
    assert_eq!(world.count(), 2);
    assert!(!world.is_valid(h3));

    system.undo(&mut world);
    assert_eq!(world.count(), 1);
    assert!(!world.is_valid(h2));

    system.redo(&mut world);
    assert_eq!(world.count(), 2);
    assert!(world.find_gameobject("B").is_some());

    system.redo(&mut world);
    assert_eq!(world.count(), 3);
    assert!(world.find_gameobject("C").is_some());
}

#[test]
fn undo_workflow_create_parent_then_destroy_captures_children() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    // Create parent
    let parent_handle = system.execute(Box::new(CreateObjectCommand::new("Parent")), &mut world);

    // Manually add children to the created parent
    let c1 = world.spawn(GameObject::new("Child1"));
    let c2 = world.spawn(GameObject::new("Child2"));
    world.set_parent(c1, Some(parent_handle));
    world.set_parent(c2, Some(parent_handle));

    // Destroy the parent (should capture children)
    let destroy_cmd = DestroyObjectCommand::new(&world, parent_handle).unwrap();
    system.execute(Box::new(destroy_cmd), &mut world);

    assert!(!world.is_valid(parent_handle));
    assert!(!world.is_valid(c1));
    assert!(!world.is_valid(c2));

    // Undo should restore parent AND children
    system.undo(&mut world);

    assert!(world.find_gameobject("Parent").is_some());
    assert!(world.find_gameobject("Child1").is_some());
    assert!(world.find_gameobject("Child2").is_some());

    let restored_parent = world.find_gameobject("Parent").unwrap();
    let children = world.get_children(restored_parent);
    assert_eq!(children.len(), 2);
}
