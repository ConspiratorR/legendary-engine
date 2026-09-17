//! Identity bridge integration tests (GameObjectHandle ↔ Entity).

use engine_core::app::AppBuilder;
use engine_core::components::Material;
use engine_core::plugins::{CorePlugins, SceneRuntimePlugin};
use engine_core::{IdentityBridge, RenderProxy, TransformProxy};
use engine_ecs::world::World as EcsWorld;

#[test]
fn test_spawn_linked_creates_both_identities() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let (go, entity) = app.spawn_linked("Player").expect("SceneRuntime present");

    assert_eq!(app.entity_for_gameobject(go), Some(entity));
    assert_eq!(app.gameobject_for_entity(entity), Some(go));
    assert!(app.unity_world().unwrap().is_valid(go));
}

#[test]
fn test_sync_bridge_updates_transform_proxy() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let (go, entity) = app.spawn_linked("Mover").unwrap();
    {
        let world = app.unity_world().unwrap();
        let _ = world.with_transform_mut(go, |t| {
            t.SetLocalPosition(engine_math::Vec3::new(10.0, 0.0, -5.0));
        });
        world.sync_transforms();
    }

    app.sync_identity_bridge();

    let proxy = app.world.get::<TransformProxy>(entity).unwrap();
    assert_eq!(proxy.position.x, 10.0);
    assert_eq!(proxy.position.z, -5.0);
}

#[test]
fn test_sync_bridge_material_to_render_proxy() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let (go, entity) = app.spawn_linked("RedCube").unwrap();
    {
        let world = app.unity_world().unwrap();
        world.AddComponent(go, Material::new_with_color([0.0, 1.0, 0.0, 1.0]));
    }

    app.sync_identity_bridge();

    let proxy = app.world.get::<RenderProxy>(entity).unwrap();
    assert!(proxy.visible);
    assert_eq!(proxy.color, [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn test_run_with_lifecycle_syncs_bridge() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let (go, entity) = app.spawn_linked("Auto").unwrap();
    {
        let world = app.unity_world().unwrap();
        let _ = world.with_transform_mut(go, |t| {
            t.SetLocalPosition(engine_math::Vec3::new(1.0, 2.0, 3.0));
        });
        world.sync_transforms();
    }

    // run_with_lifecycle includes sync_identity_bridge
    app.run_with_lifecycle(0.016);

    let proxy = app.world.get::<TransformProxy>(entity).unwrap();
    assert_eq!(proxy.position, engine_math::Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn test_bridge_prune_on_destroy() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    let (go, entity) = app.spawn_linked("Temp").unwrap();
    app.unity_world().unwrap().DestroyImmediate(go);

    app.sync_identity_bridge();
    assert_eq!(app.entity_for_gameobject(go), None);
    assert_eq!(app.gameobject_for_entity(entity), None);
}

#[test]
fn test_standalone_bridge_unit() {
    // Direct use without App (e.g. editor tools)
    let mut unity = engine_core::world::World::new();
    let mut ecs = EcsWorld::new();
    let mut bridge = IdentityBridge::new();

    let go = unity.CreateGameObject("Standalone");
    let e = bridge.ensure_entity(go, &mut ecs);
    bridge.sync_all(&unity, &mut ecs);
    assert!(ecs.get::<TransformProxy>(e).is_some());
}

#[test]
fn test_scene_runtime_plugin_includes_bridge() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(SceneRuntimePlugin);
    let mut app = builder.build();
    assert!(app.scene_runtime().is_some());
    assert!(app.unity_world().is_some());
}

#[test]
fn test_run_with_lifecycle_auto_links_create_gameobject() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    // Create without spawn_linked — sync should still link
    let go = app.unity_world().unwrap().CreateGameObject("Unlinked");
    assert_eq!(app.entity_for_gameobject(go), None);

    app.run_with_lifecycle(0.016);

    let entity = app.entity_for_gameobject(go).expect("auto-linked");
    assert_eq!(app.gameobject_for_entity(entity), Some(go));
    let proxy = app.world.get::<TransformProxy>(entity).unwrap();
    assert_eq!(proxy.position, engine_math::Vec3::ZERO);
}

#[test]
fn test_link_unity_scene_after_bulk_create() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    let mut app = builder.build();

    {
        let world = app.unity_world().unwrap();
        let a = world.CreateGameObject("A");
        let b = world.CreateGameObject("B");
        world.SetParent(b, Some(a));
    }
    app.link_unity_scene();
    assert_eq!(app.scene_runtime().unwrap().bridge.len(), 2);
}

#[test]
fn test_require_unity_world_error_without_plugin() {
    let mut app = AppBuilder::new().build();
    let err = app.require_unity_world().unwrap_err();
    assert!(err.contains("SceneRuntime"));
    assert!(app.unity_world_ref().is_none());
}
