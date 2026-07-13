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
}
