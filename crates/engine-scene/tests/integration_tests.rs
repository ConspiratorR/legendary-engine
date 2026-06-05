use engine_ecs::world::World;
use engine_math::{Mat4, Vec3};
use engine_scene::hierarchy::{Children, Parent};
use engine_scene::node::SceneNode;
use engine_scene::scene_manager::SceneManager;
use engine_scene::transform::{GlobalTransform, Transform};

// ---------------------------------------------------------------------------
// ECS + Scene integration
// ---------------------------------------------------------------------------

#[test]
fn ecs_spawn_entity_with_scene_components() {
    let mut world = World::new();
    let entity = world.spawn();

    world.add_component(entity, Transform::from_xyz(1.0, 2.0, 3.0));
    world.add_component(entity, GlobalTransform::default());
    world.add_component(entity, Children::new());

    let t = world.get::<Transform>(entity).expect("Transform missing");
    assert_eq!(t.translation, Vec3::new(1.0, 2.0, 3.0));

    let gt = world
        .get::<GlobalTransform>(entity)
        .expect("GlobalTransform missing");
    assert_eq!(gt.0, Mat4::IDENTITY);

    let children = world.get::<Children>(entity).expect("Children missing");
    assert!(children.0.is_empty());
}

#[test]
fn ecs_entity_parent_child_relationship() {
    let mut world = World::new();

    let parent = world.spawn();
    let child = world.spawn();

    world.add_component(parent, Transform::default());
    world.add_component(parent, Children::new());
    world.add_component(child, Transform::default());
    world.add_component(child, Parent(parent));

    // Wire child into parent's children list
    world.get_mut::<Children>(parent).unwrap().0.push(child);

    let children = world.get::<Children>(parent).unwrap();
    assert_eq!(children.0.len(), 1);
    assert_eq!(children.0[0], child);

    let p = world.get::<Parent>(child).unwrap();
    assert_eq!(p.0, parent);
}

// ---------------------------------------------------------------------------
// SceneManager integration (cross-crate: engine-ecs + engine-scene)
// ---------------------------------------------------------------------------

#[test]
fn scene_manager_creates_root_with_ecs_components() {
    let sm = SceneManager::new();
    let root = sm.root();

    assert_eq!(sm.name(root), "root");
}

#[test]
fn scene_manager_add_node_spawns_ecs_entity() {
    let mut sm = SceneManager::new();
    let node: SceneNode = sm.add_node("Player").into();

    let entity = node.entity();
    let world = sm.world_mut();

    assert!(world.get::<Transform>(entity).is_some());
    assert!(world.get::<GlobalTransform>(entity).is_some());
    assert!(world.get::<Children>(entity).is_some());
}

#[test]
fn scene_manager_set_parent_wires_ecs_hierarchy() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm.add_node("Parent").into();
    let child: SceneNode = sm.add_node("Child").into();

    sm.set_parent(child, parent);

    let p = sm.world_mut().get::<Parent>(child.entity()).unwrap();
    assert_eq!(p.0, parent.entity());

    let children = sm.world_mut().get::<Children>(parent.entity()).unwrap();
    assert!(children.0.contains(&child.entity()));
}

// ---------------------------------------------------------------------------
// Scene hierarchy sync: parent-child transform propagation
// ---------------------------------------------------------------------------

#[test]
fn sync_transforms_root_identity() {
    let mut sm = SceneManager::new();
    sm.sync_transforms();

    let root = sm.root();
    let gt = sm
        .world_mut()
        .get::<GlobalTransform>(root.entity())
        .unwrap();
    assert_eq!(gt.0, Mat4::IDENTITY);
}

#[test]
fn sync_transforms_single_child_translation() {
    let mut sm = SceneManager::new();
    let child: SceneNode = sm
        .add_node("Child")
        .with_transform(Transform::from_xyz(0.0, 5.0, 0.0))
        .into();

    sm.sync_transforms();

    let gt = sm
        .world_mut()
        .get::<GlobalTransform>(child.entity())
        .unwrap();
    let expected = Transform::from_xyz(0.0, 5.0, 0.0).to_matrix();
    assert_matrices_eq(gt.0, expected);
}

#[test]
fn sync_transforms_parent_translation_propagates() {
    let mut sm = SceneManager::new();
    let parent: SceneNode = sm
        .add_node("Parent")
        .with_transform(Transform::from_xyz(10.0, 0.0, 0.0))
        .into();

    let child: SceneNode = sm.add_node("Child").into();
    sm.set_parent(child, parent);
    *sm.transform_mut(child) = Transform::from_xyz(0.0, 3.0, 0.0);

    sm.sync_transforms();

    let child_gt = sm
        .world_mut()
        .get::<GlobalTransform>(child.entity())
        .unwrap();
    let expected = Transform::from_xyz(10.0, 3.0, 0.0).to_matrix();
    assert_matrices_eq(child_gt.0, expected);
}

#[test]
fn sync_transforms_three_level_hierarchy() {
    let mut sm = SceneManager::new();

    let a: SceneNode = sm
        .add_node("A")
        .with_transform(Transform::from_xyz(1.0, 0.0, 0.0))
        .into();

    let b: SceneNode = sm.add_node("B").into();
    sm.set_parent(b, a);
    *sm.transform_mut(b) = Transform::from_xyz(2.0, 0.0, 0.0);

    let c: SceneNode = sm.add_node("C").into();
    sm.set_parent(c, b);
    *sm.transform_mut(c) = Transform::from_xyz(3.0, 0.0, 0.0);

    sm.sync_transforms();

    let a_gt = sm.world_mut().get::<GlobalTransform>(a.entity()).unwrap();
    assert_matrices_eq(a_gt.0, Transform::from_xyz(1.0, 0.0, 0.0).to_matrix());

    let b_gt = sm.world_mut().get::<GlobalTransform>(b.entity()).unwrap();
    assert_matrices_eq(b_gt.0, Transform::from_xyz(3.0, 0.0, 0.0).to_matrix());

    let c_gt = sm.world_mut().get::<GlobalTransform>(c.entity()).unwrap();
    assert_matrices_eq(c_gt.0, Transform::from_xyz(6.0, 0.0, 0.0).to_matrix());
}

#[test]
fn sync_transforms_scale_propagation() {
    let mut sm = SceneManager::new();

    let parent: SceneNode = sm
        .add_node("Parent")
        .with_transform(Transform {
            translation: Vec3::ZERO,
            rotation: engine_math::Quat::IDENTITY,
            scale: Vec3::new(2.0, 2.0, 2.0),
        })
        .into();

    let child: SceneNode = sm.add_node("Child").into();
    sm.set_parent(child, parent);
    *sm.transform_mut(child) = Transform::from_xyz(1.0, 0.0, 0.0);

    sm.sync_transforms();

    let child_gt = sm
        .world_mut()
        .get::<GlobalTransform>(child.entity())
        .unwrap();
    let expected = Transform {
        translation: Vec3::new(2.0, 0.0, 0.0),
        rotation: engine_math::Quat::IDENTITY,
        scale: Vec3::new(2.0, 2.0, 2.0),
    }
    .to_matrix();
    assert_matrices_eq(child_gt.0, expected);
}

#[test]
fn sync_transforms_reparent_updates_global() {
    let mut sm = SceneManager::new();

    let parent_a: SceneNode = sm
        .add_node("A")
        .with_transform(Transform::from_xyz(10.0, 0.0, 0.0))
        .into();

    let parent_b: SceneNode = sm
        .add_node("B")
        .with_transform(Transform::from_xyz(20.0, 0.0, 0.0))
        .into();

    let child: SceneNode = sm.add_node("Child").into();
    sm.set_parent(child, parent_a);
    *sm.transform_mut(child) = Transform::from_xyz(1.0, 0.0, 0.0);

    sm.sync_transforms();

    let gt = sm
        .world_mut()
        .get::<GlobalTransform>(child.entity())
        .unwrap();
    assert_matrices_eq(gt.0, Transform::from_xyz(11.0, 0.0, 0.0).to_matrix());

    // Reparent to B
    sm.set_parent(child, parent_b);
    sm.sync_transforms();

    let gt = sm
        .world_mut()
        .get::<GlobalTransform>(child.entity())
        .unwrap();
    assert_matrices_eq(gt.0, Transform::from_xyz(21.0, 0.0, 0.0).to_matrix());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn assert_matrices_eq(a: Mat4, b: Mat4) {
    let eps = 1e-5;
    for i in 0..4 {
        for j in 0..4 {
            let diff = (a.col(i)[j] - b.col(i)[j]).abs();
            assert!(
                diff < eps,
                "Matrices differ at [{j}][{i}]: {} vs {} (diff={diff})",
                a.col(i)[j],
                b.col(i)[j],
            );
        }
    }
}

#[test]
fn sync_transforms_rotation_propagation() {
    let mut sm = SceneManager::new();

    let parent: SceneNode = sm
        .add_node("Parent")
        .with_transform(Transform {
            translation: Vec3::ZERO,
            rotation: engine_math::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::ONE,
        })
        .into();

    let child: SceneNode = sm.add_node("Child").into();
    sm.set_parent(child, parent);
    *sm.transform_mut(child) = Transform::from_xyz(10.0, 0.0, 0.0);

    sm.sync_transforms();

    let child_gt = sm
        .world_mut()
        .get::<GlobalTransform>(child.entity())
        .unwrap();
    let pos = child_gt.0.transform_point3(Vec3::ZERO);
    // After 90° Y rotation, (10,0,0) → (0,0,-10)
    assert!(pos.x.abs() < 1e-4, "x should be ~0, got {}", pos.x);
    assert!(
        (pos.z - (-10.0)).abs() < 1e-4,
        "z should be ~-10, got {}",
        pos.z
    );
}
