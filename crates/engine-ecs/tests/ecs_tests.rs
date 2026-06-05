use engine_ecs::query::{Query, QueryPair};
use engine_ecs::world::World;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity {
    dx: f32,
    dy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Health(i32);

// -- Entity spawn tests --

#[test]
fn spawn_returns_distinct_entities() {
    let mut world = World::new();
    let e1 = world.spawn();
    let e2 = world.spawn();
    assert_ne!(e1, e2);
}

#[test]
fn spawn_entity_has_zero_generation() {
    let mut world = World::new();
    let e = world.spawn();
    assert_eq!(e.generation(), 0);
}

#[test]
fn despawned_entity_generation_increments() {
    let mut world = World::new();
    let e = world.spawn();
    assert_eq!(e.generation(), 0);
    world.despawn(e);
    // Re-spawn reuses the index but with incremented generation
    let e2 = world.spawn();
    assert_eq!(e2.index(), e.index());
    assert_eq!(e2.generation(), 1);
}

// -- Component add/remove tests --

#[test]
fn add_and_get_component() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 1.0, y: 2.0 });
    let pos = world.get::<Position>(e).unwrap();
    assert_eq!(pos.x, 1.0);
    assert_eq!(pos.y, 2.0);
}

#[test]
fn get_component_on_empty_entity_returns_none() {
    let mut world = World::new();
    let e = world.spawn();
    assert!(world.get::<Position>(e).is_none());
}

#[test]
fn remove_component_returns_value() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(100));
    let removed = world.remove_component::<Health>(e);
    assert_eq!(removed, Some(Health(100)));
    assert!(world.get::<Health>(e).is_none());
}

#[test]
fn remove_nonexistent_component_returns_none() {
    let mut world = World::new();
    let e = world.spawn();
    assert_eq!(world.remove_component::<Health>(e), None);
}

#[test]
fn add_multiple_component_types() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 0.0, y: 0.0 });
    world.add_component(e, Velocity { dx: 1.0, dy: 0.5 });
    world.add_component(e, Health(50));

    assert!(world.get::<Position>(e).is_some());
    assert!(world.get::<Velocity>(e).is_some());
    assert!(world.get::<Health>(e).is_some());
}

#[test]
fn overwrite_component_with_new_value() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Health(100));
    world.add_component(e, Health(50));
    assert_eq!(world.get::<Health>(e).unwrap().0, 50);
}

// -- Query iteration tests --

#[test]
fn query_single_component_iter() {
    let mut world = World::new();
    for i in 0..10 {
        let e = world.spawn();
        world.add_component(
            e,
            Position {
                x: i as f32,
                y: 0.0,
            },
        );
    }

    let query = Query::<Position>::new();
    let positions: Vec<&Position> = query.iter(&world).collect();
    assert_eq!(positions.len(), 10);
    for (i, pos) in positions.iter().enumerate() {
        assert_eq!(pos.x, i as f32);
    }
}

#[test]
fn query_skips_entities_without_component() {
    let mut world = World::new();
    let e1 = world.spawn();
    world.add_component(e1, Position { x: 1.0, y: 0.0 });
    let _e2 = world.spawn(); // no Position
    let e3 = world.spawn();
    world.add_component(e3, Position { x: 3.0, y: 0.0 });

    let query = Query::<Position>::new();
    let count = query.iter(&world).count();
    assert_eq!(count, 2);
}

#[test]
fn query_pair_iter_only_matching_entities() {
    let mut world = World::new();
    let e1 = world.spawn();
    world.add_component(e1, Position { x: 0.0, y: 0.0 });
    world.add_component(e1, Velocity { dx: 1.0, dy: 0.0 });
    let e2 = world.spawn();
    world.add_component(e2, Position { x: 1.0, y: 0.0 });
    // e2 has no Velocity
    let e3 = world.spawn();
    world.add_component(e3, Velocity { dx: 2.0, dy: 0.0 });
    // e3 has no Position

    let query = QueryPair::<Position, Velocity>::new();
    let pairs: Vec<(&Position, &Velocity)> = query.iter(&world).collect();
    assert_eq!(pairs.len(), 1); // only e1
    assert_eq!(pairs[0].0.x, 0.0);
    assert_eq!(pairs[0].1.dx, 1.0);
}

#[test]
fn query_iter_mut_modifies_components() {
    let mut world = World::new();
    for _ in 0..5 {
        let e = world.spawn();
        world.add_component(e, Position { x: 0.0, y: 0.0 });
    }

    let query = Query::<Position>::new();
    for pos in query.iter_mut(&mut world) {
        pos.x += 10.0;
    }

    for i in 0..5u32 {
        let pos = world.get_by_index::<Position>(i).unwrap();
        assert_eq!(pos.x, 10.0);
    }
}

#[test]
fn query_large_iteration_consistency() {
    let mut world = World::new();
    let n = 10_000;
    for i in 0..n {
        let e = world.spawn();
        world.add_component(
            e,
            Position {
                x: i as f32,
                y: 0.0,
            },
        );
    }

    let query = Query::<Position>::new();
    let sum: f32 = query.iter(&world).map(|p| p.x).sum();
    let expected = (0..n).map(|i| i as f32).sum::<f32>();
    assert_eq!(sum, expected);
}

// -- Entity deletion tests --

#[test]
fn despawn_removes_component_access() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 1.0, y: 2.0 });
    world.despawn(e);
    assert!(world.get::<Position>(e).is_none());
}

#[test]
fn despawn_removes_all_components() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 0.0, y: 0.0 });
    world.add_component(e, Velocity { dx: 1.0, dy: 1.0 });
    world.despawn(e);
    assert!(world.get::<Position>(e).is_none());
    assert!(world.get::<Velocity>(e).is_none());
}

#[test]
fn entity_count_after_despawn() {
    let mut world = World::new();
    let e1 = world.spawn();
    let e2 = world.spawn();
    let e3 = world.spawn();
    assert_eq!(world.entity_count(), 3);

    world.despawn(e2);
    assert_eq!(world.entity_count(), 2);

    world.despawn(e1);
    world.despawn(e3);
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn stale_entity_handle_does_not_access_new_data() {
    let mut world = World::new();
    let e1 = world.spawn();
    world.add_component(e1, Health(100));
    world.despawn(e1);

    // e1 is stale — accessing it should return None
    assert!(world.get::<Health>(e1).is_none());
}

#[test]
fn despawn_entity_with_large_index_is_noop() {
    let mut world = World::new();
    // An entity handle with an index that was never spawned
    let fake = engine_ecs::entity::Entity::new(9999, 0);
    // Should silently return without panic
    world.despawn(fake);
    assert_eq!(world.entity_count(), 0);
}

// -- get_mut tests --

#[test]
fn get_mut_modifies_component_in_place() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 0.0, y: 0.0 });

    if let Some(pos) = world.get_mut::<Position>(e) {
        pos.x = 42.0;
        pos.y = 99.0;
    }

    let pos = world.get::<Position>(e).unwrap();
    assert_eq!(pos.x, 42.0);
    assert_eq!(pos.y, 99.0);
}

#[test]
fn get_mut_returns_none_for_missing_component() {
    let mut world = World::new();
    let e = world.spawn();
    assert!(world.get_mut::<Position>(e).is_none());
}

#[test]
fn get_mut_returns_none_for_despawned_entity() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 1.0, y: 2.0 });
    world.despawn(e);
    assert!(world.get_mut::<Position>(e).is_none());
}

// -- Resource tests --

#[test]
fn insert_and_get_resource() {
    let mut world = World::new();
    #[derive(Debug, PartialEq)]
    struct Gravity(f32);
    world.insert_resource(Gravity(9.81));
    let g = world.get_resource::<Gravity>().unwrap();
    assert_eq!(g.0, 9.81);
}

#[test]
fn get_resource_mut_modifies_resource() {
    let mut world = World::new();
    #[derive(Debug, PartialEq)]
    struct Score(u32);
    world.insert_resource(Score(0));

    if let Some(score) = world.get_resource_mut::<Score>() {
        score.0 = 100;
    }

    assert_eq!(world.get_resource::<Score>().unwrap().0, 100);
}

#[test]
fn remove_resource_returns_value() {
    let mut world = World::new();
    #[derive(Debug, PartialEq)]
    struct Config(String);
    world.insert_resource(Config("test".to_string()));

    let removed = world.remove_resource::<Config>();
    assert_eq!(removed, Some(Config("test".to_string())));
    assert!(world.get_resource::<Config>().is_none());
}

#[test]
fn get_resource_returns_none_for_missing() {
    let world = World::new();
    #[derive(Debug)]
    struct Missing;
    assert!(world.get_resource::<Missing>().is_none());
}

// -- component_entities tests --

#[test]
fn component_entities_returns_matching_indices() {
    let mut world = World::new();
    let e1 = world.spawn();
    let e2 = world.spawn();
    let _e3 = world.spawn(); // no Position
    world.add_component(e1, Position { x: 0.0, y: 0.0 });
    world.add_component(e2, Position { x: 1.0, y: 1.0 });

    let entities = world.component_entities::<Position>();
    assert_eq!(entities.len(), 2);
    assert!(entities.contains(&e1.index()));
    assert!(entities.contains(&e2.index()));
}

#[test]
fn component_entities_empty_for_no_components() {
    let world = World::new();
    let entities = world.component_entities::<Position>();
    assert!(entities.is_empty());
}

// -- Query on empty world --

#[test]
fn query_single_on_empty_world() {
    let world = World::new();
    let query = Query::<Position>::new();
    let results: Vec<&Position> = query.iter(&world).collect();
    assert!(results.is_empty());
}

#[test]
fn query_pair_on_empty_world() {
    let world = World::new();
    let query = QueryPair::<Position, Velocity>::new();
    let results: Vec<(&Position, &Velocity)> = query.iter(&world).collect();
    assert!(results.is_empty());
}

// -- Generation tracking --

#[test]
fn generation_increments_on_each_despawn() {
    let mut world = World::new();
    let e0 = world.spawn();
    assert_eq!(e0.generation(), 0);

    world.despawn(e0);
    let e1 = world.spawn();
    assert_eq!(e1.index(), e0.index());
    assert_eq!(e1.generation(), 1);

    world.despawn(e1);
    let e2 = world.spawn();
    assert_eq!(e2.index(), e0.index());
    assert_eq!(e2.generation(), 2);
}

#[test]
fn stale_handle_never_accesses_reused_slot() {
    let mut world = World::new();
    let e1 = world.spawn();
    world.add_component(e1, Health(100));
    world.despawn(e1);

    let e2 = world.spawn();
    world.add_component(e2, Health(200));

    // e1 is stale — must not see e2's data
    assert!(world.get::<Health>(e1).is_none());
    assert_eq!(world.get::<Health>(e2).unwrap().0, 200);
}

// -- Archetype-like migration (component add/remove changes queryability) --

#[test]
fn adding_component_makes_entity_queryable() {
    let mut world = World::new();
    let e = world.spawn();
    // No Velocity initially
    let query = Query::<Velocity>::new();
    assert_eq!(query.iter(&world).count(), 0);

    // Add Velocity — now queryable
    world.add_component(e, Velocity { dx: 1.0, dy: 0.0 });
    assert_eq!(query.iter(&world).count(), 1);
}

#[test]
fn removing_component_makes_entity_unqueryable() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Velocity { dx: 1.0, dy: 0.0 });
    let query = Query::<Velocity>::new();
    assert_eq!(query.iter(&world).count(), 1);

    world.remove_component::<Velocity>(e);
    assert_eq!(query.iter(&world).count(), 0);
}

#[test]
fn query_pair_respects_component_add_remove() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Position { x: 0.0, y: 0.0 });

    let query = QueryPair::<Position, Velocity>::new();
    assert_eq!(query.iter(&world).count(), 0);

    // Add second component — pair now matches
    world.add_component(e, Velocity { dx: 1.0, dy: 0.0 });
    assert_eq!(query.iter(&world).count(), 1);

    // Remove one — pair no longer matches
    world.remove_component::<Velocity>(e);
    assert_eq!(query.iter(&world).count(), 0);
}
