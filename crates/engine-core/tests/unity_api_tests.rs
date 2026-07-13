//! Unity-style API integration tests.
//!
//! These tests demonstrate the Unity-style API usage pattern in RustEngine.

#[cfg(test)]
mod tests {
    use engine_core::component::Component;
    use engine_core::gameobject::{GameObject, GameObjectHandle};
    use engine_core::transform::Transform;
    use engine_core::world::World;
    use engine_math::Vec3;
    use std::any::Any;

    // ============================================================
    // Test Components
    // ============================================================

    #[derive(Debug)]
    struct Health {
        hp: f32,
        max_hp: f32,
    }

    impl Component for Health {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct PlayerController {
        speed: f32,
    }

    impl Component for PlayerController {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    // ============================================================
    // Unity-style API Tests
    // ============================================================

    #[test]
    fn test_unity_style_gameobject_creation() {
        let mut world = World::new();

        // Create GameObject (Unity: new GameObject("name"))
        let player = world.CreateGameObject("Player");

        // Verify creation
        assert_eq!(world.GetName(player), "Player");
        assert_eq!(world.GetTag(player), "Untagged");
        assert_eq!(world.GetLayer(player), 0);
        assert!(world.IsActive(player));
    }

    #[test]
    fn test_unity_style_tag_layer() {
        let mut world = World::new();
        let player = world.CreateGameObject("Player");

        // Set tag (Unity: GameObject.tag = "Player")
        world.SetTag(player, "Player");
        assert_eq!(world.GetTag(player), "Player");
        assert!(world.CompareTag(player, "Player"));

        // Set layer (Unity: GameObject.layer = 5)
        world.SetLayer(player, 5);
        assert_eq!(world.GetLayer(player), 5);
    }

    #[test]
    fn test_unity_style_active_state() {
        let mut world = World::new();
        let player = world.CreateGameObject("Player");

        // Set active (Unity: GameObject.SetActive(false))
        world.SetActive(player, false);
        assert!(!world.IsActive(player));

        world.SetActive(player, true);
        assert!(world.IsActive(player));
    }

    #[test]
    fn test_unity_style_parent_child() {
        let mut world = World::new();
        let parent = world.CreateGameObject("Parent");
        let child = world.CreateGameObject("Child");

        // Set parent (Unity: Transform.SetParent)
        world.SetParent(child, Some(parent));

        assert_eq!(world.GetParent(child), Some(parent));
        assert!(world.GetChildren(parent).contains(&child));
        assert_eq!(world.GetChildCount(parent), 1);
    }

    #[test]
    fn test_unity_style_transform() {
        let mut world = World::new();
        let player = world.CreateGameObject("Player");

        // Set transform (Unity: Transform.position = ...)
        if let Some(t) = world.GetTransformMut(player) {
            t.SetLocalPosition(Vec3::new(10.0, 0.0, 5.0));
            t.SetLocalScale(Vec3::new(2.0, 2.0, 2.0));
        }

        // Verify
        let t = world.GetTransform(player).unwrap();
        assert_eq!(t.LocalPosition(), Vec3::new(10.0, 0.0, 5.0));
        assert_eq!(t.LocalScale(), Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_unity_style_component_access() {
        let mut world = World::new();
        let player = world.CreateGameObject("Player");

        // Add component (Unity: GameObject.AddComponent<Health>())
        world.AddComponent(player, Health { hp: 100.0, max_hp: 100.0 });

        // Get component (Unity: GameObject.GetComponent<Health>())
        let health = world.GetComponent::<Health>(player).unwrap();
        assert_eq!(health.hp, 100.0);
        assert_eq!(health.max_hp, 100.0);

        // Has component (Unity: GameObject.GetComponent<Health>() != null)
        assert!(world.HasComponent::<Health>(player));
        assert!(!world.HasComponent::<PlayerController>(player));
    }

    #[test]
    fn test_unity_style_find_objects() {
        let mut world = World::new();

        let player = world.CreateGameObject("Player");
        world.SetTag(player, "Player");

        let enemy = world.CreateGameObject("Enemy");
        world.SetTag(enemy, "Enemy");

        // Find by name (Unity: GameObject.Find("Player"))
        assert_eq!(world.Find("Player"), Some(player));
        assert_eq!(world.Find("Enemy"), Some(enemy));

        // Find by tag (Unity: GameObject.FindWithTag("Player"))
        assert_eq!(world.FindWithTag("Player"), Some(player));
        assert_eq!(world.FindWithTag("Enemy"), Some(enemy));

        // Find all by tag (Unity: GameObject.FindGameObjectsWithTag("Player"))
        let players = world.FindGameObjectsWithTag("Player");
        assert!(players.contains(&player));
    }

    #[test]
    fn test_unity_style_hierarchy() {
        let mut world = World::new();

        let root = world.CreateGameObject("Root");
        let child1 = world.CreateGameObject("Child1");
        let child2 = world.CreateGameObject("Child2");
        let grandchild = world.CreateGameObject("Grandchild");

        world.SetParent(child1, Some(root));
        world.SetParent(child2, Some(root));
        world.SetParent(grandchild, Some(child1));

        // Verify hierarchy
        assert_eq!(world.GetParent(child1), Some(root));
        assert_eq!(world.GetParent(child2), Some(root));
        assert_eq!(world.GetParent(grandchild), Some(child1));

        assert_eq!(world.GetChildCount(root), 2);
        assert_eq!(world.GetChildCount(child1), 1);
        assert_eq!(world.GetChildCount(child2), 0);
        assert_eq!(world.GetChildCount(grandchild), 0);

        // Get root game objects
        let roots = world.GetRootGameObjects();
        assert!(roots.contains(&root));
    }

    #[test]
    fn test_unity_style_destroy() {
        let mut world = World::new();
        let player = world.CreateGameObject("Player");

        // Destroy (Unity: Object.Destroy(gameObject))
        world.DestroyImmediate(player);

        // Verify destroyed
        assert!(world.Find("Player").is_none());
    }

    #[test]
    fn test_unity_style_instantiate() {
        let mut world = World::new();

        // Create template
        let template = world.CreateGameObject("Enemy");
        world.SetTag(template, "Enemy");
        world.AddComponent(template, Health { hp: 50.0, max_hp: 50.0 });

        // Instantiate (Unity: Object.Instantiate(template))
        let clone = world.Instantiate(template);

        // Verify clone exists
        assert!(world.GetName(clone).contains("Enemy"));
    }

    #[test]
    fn test_unity_complete_workflow() {
        let mut world = World::new();

        // 1. Create player
        let player = world.CreateGameObject("Player");
        world.SetTag(player, "Player");
        world.SetLayer(player, 6);
        world.AddComponent(player, Health { hp: 100.0, max_hp: 100.0 });
        world.AddComponent(player, PlayerController { speed: 5.0 });

        // 2. Create parent-child hierarchy
        let camera = world.CreateGameObject("MainCamera");
        world.SetParent(camera, Some(player));

        // 3. Modify transform
        if let Some(t) = world.GetTransformMut(player) {
            t.SetLocalPosition(Vec3::new(0.0, 1.0, 0.0));
        }

        // 4. Verify everything
        assert_eq!(world.GetName(player), "Player");
        assert_eq!(world.GetTag(player), "Player");
        assert_eq!(world.GetLayer(player), 6);
        assert!(world.HasComponent::<Health>(player));
        assert!(world.HasComponent::<PlayerController>(player));
        assert_eq!(world.GetParent(camera), Some(player));
        assert_eq!(world.GetChildCount(player), 1);

        let t = world.GetTransform(player).unwrap();
        assert_eq!(t.LocalPosition(), Vec3::new(0.0, 1.0, 0.0));
    }

    // ============================================================
    // Built-in Component Tests
    // ============================================================

    #[test]
    fn test_rigidbody_component() {
        use engine_core::components::Rigidbody;

        let mut world = World::new();
        let cube = world.CreateGameObject("Cube");

        // Add Rigidbody (Unity: GameObject.AddComponent<Rigidbody>())
        world.AddComponent(cube, Rigidbody {
            mass: 2.0,
            use_gravity: true,
            is_kinematic: false,
            ..Default::default()
        });

        // Get and modify
        let rb = world.GetComponent::<Rigidbody>(cube).unwrap();
        assert_eq!(rb.mass, 2.0);
        assert!(rb.use_gravity);

        // Add force
        let rb = world.GetComponentMut::<Rigidbody>(cube).unwrap();
        rb.AddForce(Vec3::new(0.0, 10.0, 0.0));
        assert!(rb.velocity.y > 0.0);
    }

    #[test]
    fn test_collider_components() {
        use engine_core::components::{BoxCollider, SphereCollider, CapsuleCollider, ColliderTrait};

        let mut world = World::new();
        let cube = world.CreateGameObject("Cube");
        let sphere = world.CreateGameObject("Sphere");
        let capsule = world.CreateGameObject("Capsule");

        // Add colliders
        world.AddComponent(cube, BoxCollider {
            size: Vec3::new(1.0, 1.0, 1.0),
            is_trigger: false,
            ..Default::default()
        });

        world.AddComponent(sphere, SphereCollider {
            radius: 0.5,
            is_trigger: true,
            ..Default::default()
        });

        world.AddComponent(capsule, CapsuleCollider {
            height: 2.0,
            radius: 0.5,
            is_trigger: false,
            ..Default::default()
        });

        // Verify
        let box_col = world.GetComponent::<BoxCollider>(cube).unwrap();
        assert_eq!(box_col.size, Vec3::new(1.0, 1.0, 1.0));
        assert!(!box_col.IsTrigger());

        let sphere_col = world.GetComponent::<SphereCollider>(sphere).unwrap();
        assert_eq!(sphere_col.radius, 0.5);
        assert!(sphere_col.IsTrigger());

        let capsule_col = world.GetComponent::<CapsuleCollider>(capsule).unwrap();
        assert_eq!(capsule_col.height, 2.0);
        assert_eq!(capsule_col.radius, 0.5);
    }

    #[test]
    fn test_camera_component() {
        use engine_core::components::Camera;

        let mut world = World::new();
        let cam = world.CreateGameObject("MainCamera");

        world.AddComponent(cam, Camera {
            field_of_view: 60.0,
            near_clip: 0.1,
            far_clip: 1000.0,
            ..Default::default()
        });

        let camera = world.GetComponent::<Camera>(cam).unwrap();
        assert_eq!(camera.field_of_view, 60.0);
        assert_eq!(camera.near_clip, 0.1);
        assert_eq!(camera.far_clip, 1000.0);
        assert!(!camera.orthographic);

        // Test projection matrix
        let proj = camera.ProjectionMatrix();
        assert!(proj.col(0)[0] != 0.0);
    }

    #[test]
    fn test_light_component() {
        use engine_core::components::{Light, LightType};

        let mut world = World::new();
        let light = world.CreateGameObject("PointLight");

        world.AddComponent(light, Light {
            light_type: LightType::Point,
            color: [1.0, 0.8, 0.6],
            intensity: 2.0,
            range: 15.0,
            ..Default::default()
        });

        let light_comp = world.GetComponent::<Light>(light).unwrap();
        assert_eq!(light_comp.light_type, LightType::Point);
        assert_eq!(light_comp.color, [1.0, 0.8, 0.6]);
        assert_eq!(light_comp.intensity, 2.0);
        assert_eq!(light_comp.range, 15.0);
    }

    #[test]
    fn test_mesh_renderer_component() {
        use engine_core::components::MeshRenderer;

        let mut world = World::new();
        let cube = world.CreateGameObject("Cube");

        world.AddComponent(cube, MeshRenderer {
            mesh: "Cube".to_string(),
            material: "Default".to_string(),
            cast_shadows: true,
            receive_shadows: true,
        });

        let renderer = world.GetComponent::<MeshRenderer>(cube).unwrap();
        assert_eq!(renderer.mesh, "Cube");
        assert_eq!(renderer.material, "Default");
        assert!(renderer.cast_shadows);
        assert!(renderer.receive_shadows);
    }

    #[test]
    fn test_sprite_renderer_component() {
        use engine_core::components::SpriteRenderer;

        let mut world = World::new();
        let sprite = world.CreateGameObject("PlayerSprite");

        world.AddComponent(sprite, SpriteRenderer {
            sprite: "player.png".to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
            sorting_order: 0,
        });

        let renderer = world.GetComponent::<SpriteRenderer>(sprite).unwrap();
        assert_eq!(renderer.sprite, "player.png");
        assert_eq!(renderer.color, [1.0, 1.0, 1.0, 1.0]);
        assert!(!renderer.flip_x);
        assert!(!renderer.flip_y);
    }

    #[test]
    fn test_audio_source_component() {
        use engine_core::components::AudioSource;

        let mut world = World::new();
        let audio = world.CreateGameObject("MusicPlayer");

        world.AddComponent(audio, AudioSource {
            clip: "background_music.ogg".to_string(),
            volume: 0.8,
            pitch: 1.0,
            loop_playing: true,
            play_on_awake: true,
            spatial_blend: 0.0,
            ..Default::default()
        });

        let source = world.GetComponent::<AudioSource>(audio).unwrap();
        assert_eq!(source.clip, "background_music.ogg");
        assert_eq!(source.volume, 0.8);
        assert!(source.loop_playing);
        assert!(source.play_on_awake);
        assert_eq!(source.spatial_blend, 0.0);
    }

    #[test]
    fn test_mixed_components() {
        use engine_core::components::{Rigidbody, BoxCollider, MeshRenderer};

        let mut world = World::new();
        let cube = world.CreateGameObject("PhysicsCube");

        // Add multiple components (Unity: multiple AddComponent calls)
        world.AddComponent(cube, Rigidbody {
            mass: 1.0,
            use_gravity: true,
            ..Default::default()
        });
        world.AddComponent(cube, BoxCollider::default());
        world.AddComponent(cube, MeshRenderer::default());

        // Verify all components exist
        assert!(world.HasComponent::<Rigidbody>(cube));
        assert!(world.HasComponent::<BoxCollider>(cube));
        assert!(world.HasComponent::<MeshRenderer>(cube));

        // Modify via reference
        let rb = world.GetComponentMut::<Rigidbody>(cube).unwrap();
        rb.mass = 5.0;
        assert_eq!(rb.mass, 5.0);
    }

    #[test]
    fn test_material_component() {
        use engine_core::components::Material;

        let mut world = World::new();
        let cube = world.CreateGameObject("Cube");

        // Add Material (Unity: GameObject.AddComponent<Material>())
        world.AddComponent(cube, Material::new_with_color([1.0, 0.0, 0.0, 1.0]));

        // Get and modify
        let mat = world.GetComponent::<Material>(cube).unwrap();
        assert_eq!(mat.Color(), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(mat.Metallic(), 0.0);
        assert_eq!(mat.Smoothness(), 0.5);

        // Set properties
        let mat = world.GetComponentMut::<Material>(cube).unwrap();
        mat.SetMetallic(0.8);
        mat.SetSmoothness(0.9);
        assert_eq!(mat.Metallic(), 0.8);
        assert_eq!(mat.Smoothness(), 0.9);
    }
}
