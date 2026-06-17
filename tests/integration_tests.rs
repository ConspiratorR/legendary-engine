use engine_asset::store::AssetStore;
use engine_asset::types::Texture;
use engine_ecs::world::World;
use engine_math::Vec3;
use engine_scene::node::SceneTree;

/// Test: Create world → add entities → query components
#[test]
fn ecs_workflow_spawn_query_despawn() {
    let mut world = World::new();

    // Spawn entities
    let e1 = world.spawn();
    let e2 = world.spawn();

    // Add components
    world.add_component(e1, Vec3::new(1.0, 2.0, 3.0));
    world.add_component(e2, Vec3::new(4.0, 5.0, 6.0));

    // Query
    let pos1 = world.get::<Vec3>(e1).unwrap();
    assert_eq!(pos1.x, 1.0);

    // Modify
    if let Some(pos) = world.get_mut::<Vec3>(e1) {
        pos.x = 10.0;
    }

    // Verify
    let pos1 = world.get::<Vec3>(e1).unwrap();
    assert_eq!(pos1.x, 10.0);

    // Despawn
    world.despawn(e1);
    assert!(world.get::<Vec3>(e1).is_none());
}

/// Test: Scene tree → add nodes → reparent → verify hierarchy
#[test]
fn scene_tree_workflow() {
    let mut tree = SceneTree::new();
    let root_id = tree.root_ids[0];

    // Add children
    let child1 = tree.add_node("Child1", Some(root_id));
    let child2 = tree.add_node("Child2", Some(root_id));
    let grandchild = tree.add_node("Grandchild", Some(child1));

    // Verify hierarchy
    let root = tree.nodes.iter().find(|n| n.id == root_id).unwrap();
    assert!(root.children.contains(&child1));
    assert!(root.children.contains(&child2));

    // Reparent grandchild to child2
    tree.reparent(grandchild, Some(child2));

    // Verify reparent
    let c1 = tree.nodes.iter().find(|n| n.id == child1).unwrap();
    let c2 = tree.nodes.iter().find(|n| n.id == child2).unwrap();
    assert!(!c1.children.contains(&grandchild));
    assert!(c2.children.contains(&grandchild));
}

/// Test: Asset store → load → handle → cleanup
#[test]
fn asset_store_workflow() {
    let store = AssetStore::new();
    assert_eq!(store.len(), 0);
}

/// Test: World → insert resource → get resource
#[test]
fn world_resource_workflow() {
    let mut world = World::new();

    #[derive(Debug, PartialEq)]
    struct Score(i32);

    world.insert_resource(Score(100));
    let score = world.get_resource::<Score>().unwrap();
    assert_eq!(*score, Score(100));
}

/// Test: Multiple components on same entity
#[test]
fn multi_component_entity() {
    let mut world = World::new();
    let entity = world.spawn();

    #[derive(Debug)]
    struct Position(Vec3);
    #[derive(Debug)]
    struct Velocity(Vec3);
    #[derive(Debug)]
    struct Health(f32);

    world.add_component(entity, Position(Vec3::ZERO));
    world.add_component(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
    world.add_component(entity, Health(100.0));

    assert!(world.get::<Position>(entity).is_some());
    assert!(world.get::<Velocity>(entity).is_some());
    assert!(world.get::<Health>(entity).is_some());
}

/// Test: Scene tree search
#[test]
fn scene_tree_search() {
    let mut tree = SceneTree::new();
    let root_id = tree.root_ids[0];

    tree.add_node("Player", Some(root_id));
    tree.add_node("Player2", Some(root_id));
    tree.add_node("Enemy", Some(root_id));

    let results = tree.search("Player");
    assert_eq!(results.len(), 2);
}
