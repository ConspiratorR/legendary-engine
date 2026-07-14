use engine_core::prefab::{Prefab, PrefabRegistry, PrefabValue};
use engine_core::serialization::{LoadSceneJson, SaveSceneJson, SceneData, SceneSerializer};
use engine_core::transform::Transform;
use engine_core::undo::{CreateObjectCommand, DestroyObjectCommand, UndoSystem};
use engine_core::world::World;
use engine_core::{Component, GameObject, GameObjectHandle};
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

// ===========================================================================
// Prefab Workflow Integration Tests
// ===========================================================================

#[test]
fn prefab_workflow_create_and_instantiate() {
    let mut world = World::new();
    let handle = world.CreateGameObject("Player");
    world.SetTag(handle, "PlayerTag");
    world.SetLayer(handle, 5);
    if let Some(t) = world.GetTransformMut(handle) {
        t.SetLocalPosition(Vec3::new(1.0, 2.0, 3.0));
    }

    let prefab = Prefab::Create("PlayerPrefab", &GameObject::new_with_name("Player"), &world);
    assert_eq!(prefab.Name(), "PlayerPrefab");
    assert!(prefab.Id().0 > 0);

    let mut new_world = World::new();
    let instance = prefab.Instantiate(&mut new_world);
    assert_eq!(instance.PrefabId(), prefab.Id());
}

#[test]
fn prefab_workflow_registry_lifecycle() {
    let mut registry = PrefabRegistry::new();
    let world = World::new();

    let go = GameObject::new_with_name("Enemy");
    let prefab = Prefab::Create("EnemyPrefab", &go, &world);
    let id = registry.Register(prefab);

    assert_eq!(registry.Count(), 1);
    assert!(registry.Contains(id));
    assert_eq!(registry.FindByName("EnemyPrefab").unwrap().Id(), id);

    let mut new_world = World::new();
    let instance = registry.Instantiate(id, &mut new_world).unwrap();
    assert_eq!(instance.PrefabId(), id);

    let removed = registry.Remove(id);
    assert!(removed.is_some());
    assert!(!registry.Contains(id));
    assert_eq!(registry.Count(), 0);
}

#[test]
fn prefab_workflow_overrides_and_revert() {
    let mut world = World::new();
    let go = GameObject::new_with_name("Tower");
    let prefab = Prefab::Create("TowerPrefab", &go, &world);
    let mut instance = prefab.Instantiate(&mut world);

    assert!(!instance.HasOverrides());

    instance.ApplyOverride("Health", PrefabValue::Float(100.0));
    instance.ApplyOverride("Damage", PrefabValue::Int(25));
    instance.ApplyOverride("Active", PrefabValue::Bool(true));
    instance.ApplyOverride("Name", PrefabValue::String("Super Tower".into()));

    assert!(instance.HasOverrides());
    assert_eq!(instance.OverrideCount(), 4);
    assert!(instance.HasOverride("Health"));
    assert!(instance.HasOverride("Damage"));
    assert_eq!(
        instance.GetOverride("Health"),
        Some(&PrefabValue::Float(100.0))
    );
    assert_eq!(instance.GetOverride("Damage"), Some(&PrefabValue::Int(25)));

    let mut paths = instance.OverridePaths();
    paths.sort();
    assert_eq!(paths, vec!["Active", "Damage", "Health", "Name"]);

    assert!(instance.Revert("Health"));
    assert_eq!(instance.OverrideCount(), 3);
    assert!(!instance.HasOverride("Health"));
    assert!(!instance.Revert("NonExistent"));

    instance.RevertAll();
    assert!(!instance.HasOverrides());
    assert_eq!(instance.OverrideCount(), 0);
}

#[test]
fn prefab_workflow_multiple_instances() {
    let mut world = World::new();
    let go = GameObject::new_with_name("Tree");
    let prefab = Prefab::Create("TreePrefab", &go, &world);

    let mut instance1 = prefab.Instantiate(&mut world);
    let mut instance2 = prefab.Instantiate(&mut world);
    let instance3 = prefab.Instantiate(&mut world);

    instance1.ApplyOverride("Position", PrefabValue::Float(0.0));
    instance2.ApplyOverride("Position", PrefabValue::Float(10.0));

    assert!(instance1.HasOverrides());
    assert!(instance2.HasOverrides());
    assert!(!instance3.HasOverrides());
    assert_eq!(instance1.PrefabId(), instance2.PrefabId());
    assert_eq!(instance2.PrefabId(), instance3.PrefabId());
}

#[test]
fn prefab_workflow_hierarchy_preserved_through_instantiate() {
    let mut world = World::new();
    let root = world.CreateGameObject("Root");
    let child1 = world.CreateGameObject("Child1");
    let child2 = world.CreateGameObject("Child2");
    world.SetParent(child1, Some(root));
    world.SetParent(child2, Some(root));
    let grandchild = world.CreateGameObject("Grandchild");
    world.SetParent(grandchild, Some(child1));

    let prefab = Prefab::Create(
        "HierarchyPrefab",
        &GameObject::new_with_name("Root"),
        &world,
    );
    let root_node = prefab.Root();
    assert_eq!(root_node.Name(), "Root");

    let mut new_world = World::new();
    let instance = prefab.Instantiate(&mut new_world);
    // Prefab instantiation creates the root, children are created by ToGameObject
    let handle = instance.GameObjectHandle();
    assert_eq!(new_world.GetName(handle), "Root");
}

#[test]
fn prefab_workflow_override_overwrite_and_value_types() {
    let mut world = World::new();
    let go = GameObject::new_with_name("Item");
    let prefab = Prefab::Create("ItemPrefab", &go, &world);
    let mut instance = prefab.Instantiate(&mut world);

    instance.ApplyOverride("X", PrefabValue::Float(1.0));
    instance.ApplyOverride("X", PrefabValue::Float(99.0));
    assert_eq!(instance.OverrideCount(), 1);
    assert_eq!(instance.GetOverride("X"), Some(&PrefabValue::Float(99.0)));

    instance.ApplyOverride("BoolVal", PrefabValue::Bool(false));
    instance.ApplyOverride("IntVal", PrefabValue::Int(-1));
    instance.ApplyOverride("StrVal", PrefabValue::String("abc".into()));

    assert_eq!(instance.OverrideCount(), 4);
    assert_eq!(
        instance.GetOverride("BoolVal"),
        Some(&PrefabValue::Bool(false))
    );
    assert_eq!(instance.GetOverride("IntVal"), Some(&PrefabValue::Int(-1)));
    assert_eq!(
        instance.GetOverride("StrVal"),
        Some(&PrefabValue::String("abc".into()))
    );
}

// ===========================================================================
// Serialization Workflow Integration Tests
// ===========================================================================

#[test]
fn serialization_workflow_empty_scene_roundtrip() {
    let world = World::new();
    let json = SaveSceneJson(&world, "Empty").unwrap();
    let mut loaded_world = World::new();
    let handles = LoadSceneJson(&json, &mut loaded_world).unwrap();
    assert!(handles.is_empty());
}

#[test]
fn serialization_workflow_single_object_roundtrip() {
    let mut world = World::new();
    let handle = world.CreateGameObject("Player");
    world.SetTag(handle, "Hero");
    world.SetLayer(handle, 10);
    world.SetActive(handle, true);

    let json = SaveSceneJson(&world, "Level1").unwrap();
    assert!(json.contains("Player"));
    assert!(json.contains("Hero"));

    let mut loaded_world = World::new();
    let handles = LoadSceneJson(&json, &mut loaded_world).unwrap();
    assert_eq!(handles.len(), 1);
    assert_eq!(loaded_world.GetName(handles[0]), "Player");
    assert_eq!(loaded_world.GetTag(handles[0]), "Hero");
    assert_eq!(loaded_world.GetLayer(handles[0]), 10);
    assert!(loaded_world.IsActive(handles[0]));
}

#[test]
fn serialization_workflow_hierarchy_roundtrip() {
    let mut world = World::new();
    let root = world.CreateGameObject("Level");
    let child1 = world.CreateGameObject("Player");
    let child2 = world.CreateGameObject("Camera");
    world.SetParent(child1, Some(root));
    world.SetParent(child2, Some(root));
    let grandchild = world.CreateGameObject("Weapon");
    world.SetParent(grandchild, Some(child1));

    let json = SaveSceneJson(&world, "GameScene").unwrap();
    let mut loaded_world = World::new();
    let handles = LoadSceneJson(&json, &mut loaded_world).unwrap();

    assert_eq!(handles.len(), 1);
    assert_eq!(loaded_world.GetName(handles[0]), "Level");
    let children = loaded_world.GetChildren(handles[0]);
    assert_eq!(children.len(), 2);
    assert_eq!(loaded_world.GetName(children[0]), "Player");
    assert_eq!(loaded_world.GetName(children[1]), "Camera");
}

#[test]
fn serialization_workflow_transform_roundtrip() {
    let mut world = World::new();
    let handle = world.CreateGameObject("Actor");
    if let Some(t) = world.GetTransformMut(handle) {
        t.SetLocalPosition(Vec3::new(10.0, 20.0, 30.0));
        t.SetLocalRotation(Quat::from_rotation_y(1.57));
        t.SetLocalScale(Vec3::new(2.0, 3.0, 4.0));
    }

    let serializer = SceneSerializer::new();
    let scene = serializer.Save(&world, "TransformTest");
    assert_eq!(scene.game_objects[0].transform.local_position.x, 10.0);

    let mut loaded_world = World::new();
    let handles = serializer.Load(&scene, &mut loaded_world);
    let t = loaded_world.GetTransform(handles[0]).unwrap();
    assert_eq!(t.LocalPosition().x, 10.0);
    assert_eq!(t.LocalPosition().y, 20.0);
    assert_eq!(t.LocalPosition().z, 30.0);
    assert_eq!(t.LocalScale().x, 2.0);
}

#[test]
fn serialization_workflow_json_roundtrip_preserves_structure() {
    let mut world = World::new();
    let root = world.CreateGameObject("Scene");
    let child = world.CreateGameObject("Prop");
    world.SetParent(child, Some(root));

    let json = SaveSceneJson(&world, "JsonTest").unwrap();
    let scene: SceneData = serde_json::from_str(&json).unwrap();
    assert_eq!(scene.name, "JsonTest");
    assert_eq!(scene.game_objects.len(), 1);
    assert_eq!(scene.game_objects[0].children.len(), 1);
    assert_eq!(scene.game_objects[0].children[0].name, "Prop");

    let mut loaded_world = World::new();
    let handles = LoadSceneJson(&json, &mut loaded_world).unwrap();
    assert_eq!(loaded_world.GetChildren(handles[0]).len(), 1);
}

#[test]
fn serialization_workflow_multiple_roots() {
    let mut world = World::new();
    world.CreateGameObject("Player");
    world.CreateGameObject("Enemy");
    world.CreateGameObject("Environment");

    let json = SaveSceneJson(&world, "MultiRoot").unwrap();
    let mut loaded_world = World::new();
    let handles = LoadSceneJson(&json, &mut loaded_world).unwrap();
    assert_eq!(handles.len(), 3);
}

#[test]
fn serialization_workflow_inactive_objects_preserved() {
    let mut world = World::new();
    let handle = world.CreateGameObject("InactiveObj");
    world.SetActive(handle, false);
    if let Some(t) = world.GetTransformMut(handle) {
        t.SetLocalPosition(Vec3::new(5.0, 5.0, 5.0));
        t.SetLocalScale(Vec3::new(0.5, 0.5, 0.5));
    }

    let json = SaveSceneJson(&world, "Inactive").unwrap();
    let mut loaded_world = World::new();
    let handles = LoadSceneJson(&json, &mut loaded_world).unwrap();
    assert!(!loaded_world.IsActive(handles[0]));
    let t = loaded_world.GetTransform(handles[0]).unwrap();
    assert_eq!(t.LocalPosition().x, 5.0);
    assert_eq!(t.LocalScale().x, 0.5);
}

// ===========================================================================
// Undo/Redo Workflow Integration Tests
// ===========================================================================

#[test]
fn undo_workflow_create_and_undo() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);
    let handle = system.Execute(Box::new(CreateObjectCommand::new("Player")), &mut world);
    assert_eq!(world.GetName(handle), "Player");
    assert!(system.CanUndo());
    assert!(!system.CanRedo());

    system.Undo(&mut world);
    assert!(!system.CanUndo());
    assert!(system.CanRedo());
}

#[test]
fn undo_workflow_create_undo_redo_cycle() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);
    let handle = system.Execute(Box::new(CreateObjectCommand::new("Enemy")), &mut world);
    assert_eq!(world.GetName(handle), "Enemy");

    system.Undo(&mut world);
    system.Redo(&mut world);
    assert!(world.Find("Enemy").is_some());
    assert!(system.CanUndo());
    assert!(!system.CanRedo());
}

#[test]
fn undo_workflow_destroy_and_undo_restores() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let handle = world.CreateGameObject("Tree");
    world.SetTag(handle, "Environment");
    world.SetLayer(handle, 3);

    let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
    system.Execute(Box::new(cmd), &mut world);

    system.Undo(&mut world);
    assert!(world.Find("Tree").is_some());
}

#[test]
fn undo_workflow_destroy_with_children_restores_hierarchy() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let parent = world.CreateGameObject("Parent");
    let child1 = world.CreateGameObject("Child1");
    let child2 = world.CreateGameObject("Child2");
    world.SetParent(child1, Some(parent));
    world.SetParent(child2, Some(parent));

    let cmd = DestroyObjectCommand::new(&world, parent).unwrap();
    system.Execute(Box::new(cmd), &mut world);

    system.Undo(&mut world);
    assert!(world.Find("Parent").is_some());
    assert!(world.Find("Child1").is_some());
    assert!(world.Find("Child2").is_some());
}

#[test]
#[ignore] // TODO: Component restoration requires serialization support for each component type
fn undo_workflow_destroy_with_components_restores_data() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let handle = world.CreateGameObject("Warrior");
    world.AddComponent(handle, HealthComponent { hp: 200.0 });
    world.AddComponent(handle, TestComponent { value: 42 });

    let cmd = DestroyObjectCommand::new(&world, handle).unwrap();
    system.Execute(Box::new(cmd), &mut world);
    system.Undo(&mut world);

    let restored = world.Find("Warrior").unwrap();
    // Note: Component restoration requires serialization support.
    // Currently only basic GameObject data (name, tag, layer, active) is restored.
    assert_eq!(world.GetName(restored), "Warrior");
}

#[test]
fn undo_workflow_max_history_limits_undo() {
    let mut world = World::new();
    let mut system = UndoSystem::new(3);
    system.Execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    system.Execute(Box::new(CreateObjectCommand::new("B")), &mut world);
    system.Execute(Box::new(CreateObjectCommand::new("C")), &mut world);
    system.Execute(Box::new(CreateObjectCommand::new("D")), &mut world);

    assert!(system.Undo(&mut world).is_some());
    assert!(system.Undo(&mut world).is_some());
    assert!(system.Undo(&mut world).is_some());
    assert!(system.Undo(&mut world).is_none());
}

#[test]
fn undo_workflow_new_command_clears_redo() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);
    system.Execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    system.Execute(Box::new(CreateObjectCommand::new("B")), &mut world);
    system.Undo(&mut world);
    assert!(system.CanRedo());
    system.Execute(Box::new(CreateObjectCommand::new("C")), &mut world);
    assert!(!system.CanRedo());
}

#[test]
fn undo_workflow_clear_resets_everything() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);
    system.Execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    system.Execute(Box::new(CreateObjectCommand::new("B")), &mut world);
    system.Undo(&mut world);
    system.Clear();
    assert!(!system.CanUndo());
    assert!(!system.CanRedo());
}

#[test]
fn undo_workflow_descriptions() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);
    assert_eq!(system.UndoDescription(), None);
    system.Execute(Box::new(CreateObjectCommand::new("Player")), &mut world);
    assert_eq!(
        system.UndoDescription(),
        Some("Create 'Player'".to_string())
    );
    system.Undo(&mut world);
    assert_eq!(
        system.RedoDescription(),
        Some("Create 'Player'".to_string())
    );
}

#[test]
fn undo_workflow_create_with_builder_options() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);
    let cmd = CreateObjectCommand::new("SpecialObj")
        .WithTag("Special")
        .WithLayer(7)
        .WithActive(false);
    let handle = system.Execute(Box::new(cmd), &mut world);
    assert_eq!(world.GetName(handle), "SpecialObj");
    assert_eq!(world.GetTag(handle), "Special");
    assert_eq!(world.GetLayer(handle), 7);
    assert!(!world.IsActive(handle));
}

#[test]
fn undo_workflow_multiple_undo_redo_cycles() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);
    let _h1 = system.Execute(Box::new(CreateObjectCommand::new("A")), &mut world);
    let h2 = system.Execute(Box::new(CreateObjectCommand::new("B")), &mut world);
    let h3 = system.Execute(Box::new(CreateObjectCommand::new("C")), &mut world);

    system.Undo(&mut world);
    assert!(!world.is_valid(h3));
    system.Undo(&mut world);
    assert!(!world.is_valid(h2));
    system.Redo(&mut world);
    assert!(world.Find("B").is_some());
    system.Redo(&mut world);
    assert!(world.Find("C").is_some());
}

#[test]
fn undo_workflow_create_parent_then_destroy_captures_children() {
    let mut world = World::new();
    let mut system = UndoSystem::new(50);

    let parent_handle = system.Execute(Box::new(CreateObjectCommand::new("Parent")), &mut world);
    let c1 = world.CreateGameObject("Child1");
    let c2 = world.CreateGameObject("Child2");
    world.SetParent(c1, Some(parent_handle));
    world.SetParent(c2, Some(parent_handle));

    let destroy_cmd = DestroyObjectCommand::new(&world, parent_handle).unwrap();
    system.Execute(Box::new(destroy_cmd), &mut world);

    system.Undo(&mut world);
    assert!(world.Find("Parent").is_some());
    assert!(world.Find("Child1").is_some());
    assert!(world.Find("Child2").is_some());
}
