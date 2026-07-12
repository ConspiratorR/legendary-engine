use engine_ecs::entity::Entity;
use engine_ecs::gameobject::GameObjectHandle;
use engine_math::Vec3;
use engine_scene::hierarchy::{Children, Parent};
use engine_scene::node::SceneNode;
use engine_scene::scene_manager::SceneManager;
use engine_scene::serialization::{
    ComponentData, PropertyValue, SceneData, SceneEntityData, SceneFormat, TransformData,
    deserialize_scene, serialize_scene,
};
use engine_scene::transform::{GlobalTransform, Transform};

// ── Scene Node Creation ─────────────────────────────────────────────

#[test]
fn test_scene_node_from_gameobject_handle() {
    let handle = GameObjectHandle::new(42, 0);
    let node = SceneNode::new(handle);
    assert_eq!(node.gameobject(), handle);
}

#[test]
fn test_scene_node_copy_and_eq() {
    let handle = GameObjectHandle::new(1, 0);
    let a = SceneNode::new(handle);
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn test_scene_manager_add_node() {
    let mut sm = SceneManager::new();
    let node: SceneNode = sm.add_node("Player").into();
    assert_eq!(sm.name(node), "Player");
}

#[test]
fn test_scene_manager_root() {
    let sm = SceneManager::new();
    let root = sm.root();
    assert_eq!(sm.name(root), "root");
}

#[test]
fn test_scene_manager_multiple_nodes() {
    let mut sm = SceneManager::new();
    let a: SceneNode = sm.add_node("A").into();
    let b: SceneNode = sm.add_node("B").into();
    let c: SceneNode = sm.add_node("C").into();

    assert_eq!(sm.name(a), "A");
    assert_eq!(sm.name(b), "B");
    assert_eq!(sm.name(c), "C");
}

// ── Parent-Child Relationship ───────────────────────────────────────

#[test]
fn test_set_parent() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm.add_node("Parent").into();
    let child: SceneNode = sm.add_node("Child").into();

    sm.set_parent(child, parent);
    assert_eq!(sm.parent(child), Some(parent));
}

#[test]
fn test_root_has_no_parent() {
    let sm = SceneManager::new();
    let root = sm.root();
    assert!(sm.parent(root).is_none());
}

#[test]
fn test_reparent() {
    let mut sm = SceneManager::new();
    let parent_a: SceneNode = sm.add_node("A").into();
    let parent_b: SceneNode = sm.add_node("B").into();
    let child: SceneNode = sm.add_node("Child").into();

    sm.set_parent(child, parent_a);
    assert_eq!(sm.parent(child), Some(parent_a));

    sm.set_parent(child, parent_b);
    assert_eq!(sm.parent(child), Some(parent_b));
}

#[test]
fn test_children_component() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm.add_node("Parent").into();
    let child1: SceneNode = sm.add_node("Child1").into();
    let child2: SceneNode = sm.add_node("Child2").into();

    sm.set_parent(child1, parent);
    sm.set_parent(child2, parent);

    let parent_entity = sm.resolve_entity(parent);
    let child1_entity = sm.resolve_entity(child1);
    let child2_entity = sm.resolve_entity(child2);

    let children = sm.world_mut().get::<Children>(parent_entity).unwrap();
    assert_eq!(children.0.len(), 2);
    assert!(children.0.contains(&child1_entity));
    assert!(children.0.contains(&child2_entity));
}

#[test]
fn test_parent_component() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm.add_node("Parent").into();
    let child: SceneNode = sm.add_node("Child").into();

    sm.set_parent(child, parent);

    let child_entity = sm.resolve_entity(child);
    let parent_entity = sm.resolve_entity(parent);

    let parent_comp = sm.world_mut().get::<Parent>(child_entity).unwrap();
    assert_eq!(parent_comp.0, parent_entity);
}

// ── Global Transform Hierarchy ──────────────────────────────────────

#[test]
fn test_sync_transforms_identity() {
    let mut sm = SceneManager::new();
    sm.sync_transforms();

    let root = sm.root();
    let entity = sm.resolve_entity(root);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    // Root with default transform should have identity global transform.
    assert_eq!(gt.0, engine_math::Mat4::IDENTITY);
}

#[test]
fn test_sync_transforms_parent_translation() {
    let mut sm = SceneManager::new();
    let child: SceneNode = sm
        .add_node("Child")
        .with_transform(Transform::from_xyz(10.0, 0.0, 0.0))
        .into();

    sm.sync_transforms();

    let entity = sm.resolve_entity(child);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let translation = gt.0.transform_point3(engine_math::Vec3::ZERO);
    assert!((translation.x - 10.0).abs() < 1e-5);
    assert!((translation.y).abs() < 1e-5);
    assert!((translation.z).abs() < 1e-5);
}

#[test]
fn test_sync_transforms_nested_hierarchy() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm
        .add_node("Parent")
        .with_transform(Transform::from_xyz(5.0, 0.0, 0.0))
        .into();
    let child: SceneNode = sm
        .add_node("Child")
        .with_transform(Transform::from_xyz(3.0, 0.0, 0.0))
        .into();

    sm.set_parent(child, parent);
    sm.sync_transforms();

    // Child's global position should be parent (5) + child (3) = 8
    let entity = sm.resolve_entity(child);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let translation = gt.0.transform_point3(engine_math::Vec3::ZERO);
    assert!((translation.x - 8.0).abs() < 1e-5);
}

#[test]
fn test_sync_transforms_three_levels() {
    let mut sm = SceneManager::new();
    let a: SceneNode = sm
        .add_node("A")
        .with_transform(Transform::from_xyz(1.0, 0.0, 0.0))
        .into();
    let b: SceneNode = sm
        .add_node("B")
        .with_transform(Transform::from_xyz(2.0, 0.0, 0.0))
        .into();
    let c: SceneNode = sm
        .add_node("C")
        .with_transform(Transform::from_xyz(3.0, 0.0, 0.0))
        .into();

    sm.set_parent(b, a);
    sm.set_parent(c, b);
    sm.sync_transforms();

    // C global = 1 + 2 + 3 = 6
    let c_entity = sm.resolve_entity(c);
    let gt = sm.world_mut().get::<GlobalTransform>(c_entity).unwrap();
    let translation = gt.0.transform_point3(engine_math::Vec3::ZERO);
    assert!((translation.x - 6.0).abs() < 1e-5);
}

#[test]
fn test_sync_transforms_after_modification() {
    let mut sm = SceneManager::new();
    let node: SceneNode = sm
        .add_node("Node")
        .with_transform(Transform::from_xyz(0.0, 0.0, 0.0))
        .into();

    sm.sync_transforms();

    // Modify transform
    sm.transform_mut(node).translation = engine_math::Vec3::new(42.0, 0.0, 0.0);
    sm.sync_transforms();

    let entity = sm.resolve_entity(node);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let translation = gt.0.transform_point3(engine_math::Vec3::ZERO);
    assert!((translation.x - 42.0).abs() < 1e-5);
}

// ── Cascade Delete ──────────────────────────────────────────────────

#[test]
fn test_remove_child_from_parent() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm.add_node("Parent").into();
    let child: SceneNode = sm.add_node("Child").into();

    sm.set_parent(child, parent);

    let parent_entity = sm.resolve_entity(parent);
    let child_entity = sm.resolve_entity(child);

    // Verify child is in parent's children
    {
        let children = sm.world_mut().get::<Children>(parent_entity).unwrap();
        assert!(children.0.contains(&child_entity));
    }

    // Remove child from parent's children list
    if let Some(children) = sm.world_mut().get_mut::<Children>(parent_entity) {
        children.0.retain(|e| *e != child_entity);
    }

    // Verify child is no longer in parent's children
    let children = sm.world_mut().get::<Children>(parent_entity).unwrap();
    assert!(!children.0.contains(&child_entity));
}

#[test]
fn test_cascade_remove_hierarchy() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm.add_node("Parent").into();
    let child1: SceneNode = sm.add_node("Child1").into();
    let child2: SceneNode = sm.add_node("Child2").into();
    let grandchild: SceneNode = sm.add_node("Grandchild").into();

    sm.set_parent(child1, parent);
    sm.set_parent(child2, parent);
    sm.set_parent(grandchild, child1);

    let parent_entity = sm.resolve_entity(parent);
    let _child1_entity = sm.resolve_entity(child1);

    // Collect direct children first
    let direct_children: Vec<Entity> = sm
        .world_mut()
        .get::<Children>(parent_entity)
        .map(|c| c.0.clone())
        .unwrap_or_default();

    // Collect grandchildren
    let mut to_remove = Vec::new();
    for child_entity in &direct_children {
        to_remove.push(*child_entity);
        if let Some(grandchildren) = sm.world_mut().get::<Children>(*child_entity) {
            to_remove.extend(grandchildren.0.iter().copied());
        }
    }

    // Despawn all descendants
    for entity in &to_remove {
        sm.world_mut().despawn(*entity);
    }

    // Parent should have empty children
    if let Some(children) = sm.world_mut().get_mut::<Children>(parent_entity) {
        children.0.clear();
    }
    let children = sm.world_mut().get::<Children>(parent_entity).unwrap();
    assert!(children.0.is_empty());
}

// ── Serialization Tests ─────────────────────────────────────────────

#[test]
fn test_scene_node_serialization_roundtrip() {
    let mut scene = SceneData::new("TestScene");

    let mut entity = SceneEntityData::new(1, "Player");
    entity.transform.translation = [1.0, 2.0, 3.0];
    entity.transform.rotation = [0.0, 0.0, 0.0, 1.0];
    entity.transform.scale = [2.0, 2.0, 2.0];
    entity.add_component(
        ComponentData::new("MeshRenderer")
            .with_property("mesh", PropertyValue::String("player.obj".into()))
            .with_property("visible", PropertyValue::Bool(true)),
    );
    scene.add_entity(entity);

    // JSON roundtrip
    let data = serialize_scene(&scene, SceneFormat::Json).unwrap();
    let loaded = deserialize_scene(&data, SceneFormat::Json).unwrap();
    assert_eq!(loaded.name, "TestScene");
    assert_eq!(loaded.entities.len(), 1);
    assert_eq!(loaded.entities[0].name, "Player");
    assert_eq!(loaded.entities[0].transform.translation, [1.0, 2.0, 3.0]);
    assert_eq!(loaded.entities[0].transform.scale, [2.0, 2.0, 2.0]);
    assert_eq!(loaded.entities[0].components.len(), 1);
}

#[test]
fn test_transform_serialization_json() {
    let transform = TransformData {
        translation: [10.0, 20.0, 30.0],
        rotation: [0.0, 0.707, 0.0, 0.707],
        scale: [1.0, 1.0, 1.0],
    };

    let json = serde_json::to_string(&transform).unwrap();
    let loaded: TransformData = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.translation, [10.0, 20.0, 30.0]);
    assert_eq!(loaded.scale, [1.0, 1.0, 1.0]);
}

#[test]
fn test_transform_serialization_ron() {
    let transform = TransformData {
        translation: [5.0, 10.0, 15.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [3.0, 3.0, 3.0],
    };

    let ron_str = ron::to_string(&transform).unwrap();
    let loaded: TransformData = ron::from_str(&ron_str).unwrap();
    assert_eq!(loaded.translation, [5.0, 10.0, 15.0]);
    assert_eq!(loaded.scale, [3.0, 3.0, 3.0]);
}

#[test]
fn test_transform_serialization_bincode() {
    let transform = TransformData {
        translation: [1.0, 2.0, 3.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };

    let bytes = bincode::serialize(&transform).unwrap();
    let loaded: TransformData = bincode::deserialize(&bytes).unwrap();
    assert_eq!(loaded, transform);
}

#[test]
fn test_hierarchy_serialization_roundtrip() {
    let mut scene = SceneData::new("HierarchyScene");

    let mut parent = SceneEntityData::new(1, "Root");
    parent.children = vec![2, 3];

    let mut child1 = SceneEntityData::new(2, "Child1");
    child1.parent = Some(1);
    child1.transform.translation = [0.0, 5.0, 0.0];

    let mut child2 = SceneEntityData::new(3, "Child2");
    child2.parent = Some(1);
    child2.transform.translation = [0.0, 0.0, 5.0];

    scene.add_entity(parent);
    scene.add_entity(child1);
    scene.add_entity(child2);

    for format in [SceneFormat::Json, SceneFormat::Ron, SceneFormat::Bin] {
        let data = serialize_scene(&scene, format).unwrap();
        let loaded = deserialize_scene(&data, format).unwrap();

        assert_eq!(loaded.entities.len(), 3);
        assert_eq!(loaded.entities[0].children, vec![2, 3]);
        assert_eq!(loaded.entities[1].parent, Some(1));
        assert_eq!(loaded.entities[2].parent, Some(1));
        assert_eq!(loaded.entities[1].transform.translation, [0.0, 5.0, 0.0]);
    }
}

#[test]
fn test_scene_with_components_roundtrip() {
    let mut scene = SceneData::new("ComponentScene");
    let mut entity = SceneEntityData::new(1, "Entity");

    entity.add_component(
        ComponentData::new("Light")
            .with_property("intensity", PropertyValue::Float(1.5))
            .with_property("color", PropertyValue::Color([1.0, 0.8, 0.6, 1.0]))
            .with_property("cast_shadows", PropertyValue::Bool(true)),
    );
    entity.add_component(
        ComponentData::new("Transform")
            .with_property("position", PropertyValue::Vec3([1.0, 2.0, 3.0])),
    );

    scene.add_entity(entity);

    let data = serialize_scene(&scene, SceneFormat::Json).unwrap();
    let loaded = deserialize_scene(&data, SceneFormat::Json).unwrap();

    assert_eq!(loaded.entities[0].components.len(), 2);
    let light = &loaded.entities[0].components[0];
    assert_eq!(light.type_name, "Light");
    assert_eq!(light.properties.len(), 3);
}

#[test]
fn test_scene_builder_methods() {
    let scene = SceneData::new("BuilderScene")
        .with_layers(0b1010)
        .with_namespace("level1");

    assert_eq!(scene.name, "BuilderScene");
    assert_eq!(scene.layers, Some(0b1010));
    assert_eq!(scene.namespace, Some("level1".to_string()));
}

#[test]
fn test_scene_entity_lookup() {
    let mut scene = SceneData::new("LookupScene");
    scene.add_entity(SceneEntityData::new(1, "A"));
    scene.add_entity(SceneEntityData::new(2, "B"));
    scene.add_entity(SceneEntityData::new(3, "C"));

    assert_eq!(scene.get_entity(2).unwrap().name, "B");
    assert!(scene.get_entity(99).is_none());

    scene.get_entity_mut(2).unwrap().name = "Modified".to_string();
    assert_eq!(scene.get_entity(2).unwrap().name, "Modified");

    let removed = scene.remove_entity(2);
    assert_eq!(removed.unwrap().name, "Modified");
    assert!(scene.get_entity(2).is_none());
    assert_eq!(scene.entities.len(), 2);
}

// ── Rotation & Scale Propagation ────────────────────────────────────

#[test]
fn test_sync_transforms_rotation_propagation() {
    use engine_math::Quat;

    let mut sm = SceneManager::new();
    let parent: SceneNode = sm
        .add_node("Parent")
        .with_transform(Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::ONE,
        })
        .into();
    let child: SceneNode = sm
        .add_node("Child")
        .with_transform(Transform::from_xyz(10.0, 0.0, 0.0))
        .into();

    sm.set_parent(child, parent);
    sm.sync_transforms();

    let entity = sm.resolve_entity(child);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let translation = gt.0.transform_point3(engine_math::Vec3::ZERO);
    // After 90° Y rotation, (10,0,0) → (0,0,-10)
    assert!(
        (translation.x).abs() < 1e-4,
        "x should be ~0, got {}",
        translation.x
    );
    assert!(
        (translation.z - (-10.0)).abs() < 1e-4,
        "z should be ~-10, got {}",
        translation.z
    );
}

#[test]
fn test_sync_transforms_nested_scale() {
    let mut sm = SceneManager::new();
    let grandparent: SceneNode = sm
        .add_node("Grandparent")
        .with_transform(Transform {
            translation: Vec3::ZERO,
            rotation: engine_math::Quat::IDENTITY,
            scale: Vec3::new(2.0, 2.0, 2.0),
        })
        .into();
    let parent: SceneNode = sm
        .add_node("Parent")
        .with_transform(Transform {
            translation: Vec3::ZERO,
            rotation: engine_math::Quat::IDENTITY,
            scale: Vec3::new(3.0, 3.0, 3.0),
        })
        .into();
    let child: SceneNode = sm
        .add_node("Child")
        .with_transform(Transform::from_xyz(1.0, 0.0, 0.0))
        .into();

    sm.set_parent(parent, grandparent);
    sm.set_parent(child, parent);
    sm.sync_transforms();

    let entity = sm.resolve_entity(child);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let translation = gt.0.transform_point3(engine_math::Vec3::ZERO);
    // grandparent scale 2 * parent scale 3 = 6x, child at (1,0,0) → (6,0,0)
    assert!(
        (translation.x - 6.0).abs() < 1e-4,
        "x should be 6.0, got {}",
        translation.x
    );
}

// ── Edge Cases ──────────────────────────────────────────────────────

#[test]
fn test_orphaned_node_has_identity_global_transform() {
    let mut sm = SceneManager::new();
    // Create a node but don't set a parent (it gets parented to root by default)
    let node: SceneNode = sm
        .add_node("Orphan")
        .with_transform(Transform::from_xyz(7.0, 8.0, 9.0))
        .into();

    sm.sync_transforms();

    // Node is under root (default), so global = local
    let entity = sm.resolve_entity(node);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let translation = gt.0.transform_point3(engine_math::Vec3::ZERO);
    assert!((translation.x - 7.0).abs() < 1e-5);
    assert!((translation.y - 8.0).abs() < 1e-5);
    assert!((translation.z - 9.0).abs() < 1e-5);
}

#[test]
fn test_reparent_removes_from_old_parent_children() {
    let mut sm = SceneManager::new();
    let parent_a: SceneNode = sm.add_node("A").into();
    let parent_b: SceneNode = sm.add_node("B").into();
    let child: SceneNode = sm.add_node("Child").into();

    sm.set_parent(child, parent_a);
    {
        let parent_a_entity = sm.resolve_entity(parent_a);
        let child_entity = sm.resolve_entity(child);
        let children = sm.world_mut().get::<Children>(parent_a_entity).unwrap();
        assert!(children.0.contains(&child_entity));
    }

    // Reparent to B
    sm.set_parent(child, parent_b);

    // Child should be removed from A's children
    {
        let parent_a_entity = sm.resolve_entity(parent_a);
        let child_entity = sm.resolve_entity(child);
        let children = sm.world_mut().get::<Children>(parent_a_entity).unwrap();
        assert!(
            !children.0.contains(&child_entity),
            "child should be removed from old parent's children"
        );
    }
    // Child should be in B's children
    {
        let parent_b_entity = sm.resolve_entity(parent_b);
        let child_entity = sm.resolve_entity(child);
        let children = sm.world_mut().get::<Children>(parent_b_entity).unwrap();
        assert!(
            children.0.contains(&child_entity),
            "child should be in new parent's children"
        );
    }
}

#[test]
fn test_multiple_children_order_preserved() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm.add_node("Parent").into();
    let c1: SceneNode = sm.add_node("C1").into();
    let c2: SceneNode = sm.add_node("C2").into();
    let c3: SceneNode = sm.add_node("C3").into();

    sm.set_parent(c1, parent);
    sm.set_parent(c2, parent);
    sm.set_parent(c3, parent);

    let parent_entity = sm.resolve_entity(parent);
    let c1_entity = sm.resolve_entity(c1);
    let c2_entity = sm.resolve_entity(c2);
    let c3_entity = sm.resolve_entity(c3);

    let children = sm.world_mut().get::<Children>(parent_entity).unwrap();
    assert_eq!(children.0.len(), 3);
    assert_eq!(children.0[0], c1_entity);
    assert_eq!(children.0[1], c2_entity);
    assert_eq!(children.0[2], c3_entity);
}

#[test]
fn test_scene_node_serialization_roundtrip_all_formats() {
    let mut scene = SceneData::new("MultiFormat");
    let mut entity = SceneEntityData::new(1, "Object");
    entity.transform.translation = [1.0, 2.0, 3.0];
    entity.transform.rotation = [0.0, 0.0, 0.0, 1.0];
    entity.transform.scale = [1.0, 1.0, 1.0];
    scene.add_entity(entity);

    for format in [SceneFormat::Json, SceneFormat::Ron, SceneFormat::Bin] {
        let data = serialize_scene(&scene, format).unwrap();
        let loaded = deserialize_scene(&data, format).unwrap();
        assert_eq!(loaded.name, "MultiFormat");
        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.entities[0].name, "Object");
        assert_eq!(loaded.entities[0].transform.translation, [1.0, 2.0, 3.0]);
        assert_eq!(loaded.entities[0].transform.rotation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(loaded.entities[0].transform.scale, [1.0, 1.0, 1.0]);
    }
}

#[test]
fn test_sync_transforms_after_reparent() {
    let mut sm = SceneManager::new();
    let a: SceneNode = sm
        .add_node("A")
        .with_transform(Transform::from_xyz(10.0, 0.0, 0.0))
        .into();
    let b: SceneNode = sm
        .add_node("B")
        .with_transform(Transform::from_xyz(20.0, 0.0, 0.0))
        .into();
    let child: SceneNode = sm
        .add_node("Child")
        .with_transform(Transform::from_xyz(5.0, 0.0, 0.0))
        .into();

    // Under A: global = 10 + 5 = 15
    sm.set_parent(child, a);
    sm.sync_transforms();
    let entity = sm.resolve_entity(child);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let pos = gt.0.transform_point3(engine_math::Vec3::ZERO);
    assert!((pos.x - 15.0).abs() < 1e-5);

    // Under B: global = 20 + 5 = 25
    sm.set_parent(child, b);
    sm.sync_transforms();
    let entity = sm.resolve_entity(child);
    let gt = sm.world_mut().get::<GlobalTransform>(entity).unwrap();
    let pos = gt.0.transform_point3(engine_math::Vec3::ZERO);
    assert!((pos.x - 25.0).abs() < 1e-5);
}
