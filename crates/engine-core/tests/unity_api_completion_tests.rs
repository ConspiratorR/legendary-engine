use engine_core::bounds::Bounds;
use engine_core::components::{ForceMode, MeshFilter, Rigidbody};
use engine_core::mathf::Mathf;
use engine_core::monobehaviour::MonoBehaviour;
use engine_core::random::Random;
use engine_core::raycast::{LayerMask, Ray};
use engine_core::transform::Transform;
use engine_math::{Quat, Vec3};

#[test]
fn test_ray_creation() {
    let ray = Ray::new(Vec3::ZERO, Vec3::Y);
    assert_eq!(ray.origin, Vec3::ZERO);
    assert!((ray.direction.length() - 1.0).abs() < 0.001);
}

#[test]
fn test_ray_get_point() {
    let ray = Ray::new(Vec3::ZERO, Vec3::X);
    let point = ray.GetPoint(5.0);
    assert!((point.x - 5.0).abs() < 0.001);
}

#[test]
fn test_vec2() {
    let v = engine_math::Vec2::new(1.0, 2.0);
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 2.0);
}

#[test]
fn test_layer_mask() {
    assert_eq!(LayerMask::NameToLayer("Default"), 0);
    assert_eq!(LayerMask::LayerToName(0), "Default");
    let mask = LayerMask::GetMask(&["Default", "UI"]);
    assert_eq!(mask, 1 | (1 << 5));
}

#[test]
fn test_layer_mask_operators() {
    let a = LayerMask(1);
    let b = LayerMask(4);
    assert_eq!((a | b).0, 5);
    assert_eq!((a & b).0, 0);
}

#[test]
fn test_bounds_creation() {
    let b = Bounds::new(Vec3::ZERO, Vec3::ONE);
    assert_eq!(b.center, Vec3::ZERO);
    assert!((b.extents.x - 0.5).abs() < 0.001);
}

#[test]
fn test_bounds_contains() {
    let b = Bounds::new(Vec3::ZERO, Vec3::ONE);
    assert!(b.Contains(Vec3::ZERO));
    assert!(b.Contains(Vec3::new(0.4, 0.4, 0.4)));
    assert!(!b.Contains(Vec3::new(1.0, 1.0, 1.0)));
}

#[test]
fn test_bounds_intersects() {
    let a = Bounds::new(Vec3::ZERO, Vec3::ONE);
    let b = Bounds::new(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);
    let c = Bounds::new(Vec3::new(2.0, 0.0, 0.0), Vec3::ONE);
    assert!(a.Intersects(&b));
    assert!(!a.Intersects(&c));
}

#[test]
fn test_bounds_encapsulate() {
    let mut b = Bounds::new(Vec3::ZERO, Vec3::ONE);
    b.Encapsulate(Vec3::new(2.0, 2.0, 2.0));
    assert!(b.Contains(Vec3::new(2.0, 2.0, 2.0)));
}

#[test]
fn test_bounds_closest_point() {
    let b = Bounds::new(Vec3::ZERO, Vec3::ONE);
    let closest = b.ClosestPoint(Vec3::new(2.0, 0.0, 0.0));
    assert!((closest.x - 0.5).abs() < 0.001);
}

#[test]
fn test_bounds_min_max() {
    let b = Bounds::new(Vec3::ZERO, Vec3::ONE);
    let min = b.min();
    let max = b.max();
    assert!((min.x + 0.5).abs() < 0.001);
    assert!((max.x - 0.5).abs() < 0.001);
}

#[test]
fn test_mathf_constants() {
    assert!((Mathf::PI - std::f32::consts::PI).abs() < 0.001);
    assert!((Mathf::Rad2Deg - 180.0 / std::f32::consts::PI).abs() < 0.001);
}

#[test]
fn test_mathf_lerp() {
    assert!((Mathf::Lerp(0.0, 10.0, 0.5) - 5.0).abs() < 0.001);
    assert!((Mathf::Lerp(0.0, 10.0, 0.0) - 0.0).abs() < 0.001);
    assert!((Mathf::Lerp(0.0, 10.0, 1.0) - 10.0).abs() < 0.001);
    assert!((Mathf::Lerp(0.0, 10.0, 2.0) - 10.0).abs() < 0.001);
}

#[test]
fn test_mathf_lerp_unclamped() {
    assert!((Mathf::LerpUnclamped(0.0, 10.0, 2.0) - 20.0).abs() < 0.001);
}

#[test]
fn test_mathf_inverse_lerp() {
    assert!((Mathf::InverseLerp(0.0, 10.0, 5.0) - 0.5).abs() < 0.001);
    assert!(Mathf::InverseLerp(0.0, 10.0, -1.0).abs() < 0.001);
    assert!((Mathf::InverseLerp(0.0, 10.0, 11.0) - 1.0).abs() < 0.001);
}

#[test]
fn test_mathf_smooth_step() {
    let v = Mathf::SmoothStep(0.0, 1.0, 0.5);
    assert!((v - 0.5).abs() < 0.01);
}

#[test]
fn test_mathf_move_towards() {
    assert!((Mathf::MoveTowards(0.0, 10.0, 3.0) - 3.0).abs() < 0.001);
    assert!((Mathf::MoveTowards(0.0, 10.0, 15.0) - 10.0).abs() < 0.001);
}

#[test]
fn test_mathf_approximately() {
    assert!(Mathf::Approximately(1.0, 1.0));
    assert!(!Mathf::Approximately(1.0, 2.0));
}

#[test]
fn test_mathf_repeat() {
    assert!((Mathf::Repeat(7.0, 3.0) - 1.0).abs() < 0.001);
    assert!((Mathf::Repeat(-1.0, 3.0) - 2.0).abs() < 0.001);
}

#[test]
fn test_mathf_ping_pong() {
    assert!((Mathf::PingPong(1.5, 2.0) - 1.5).abs() < 0.001);
    assert!((Mathf::PingPong(3.0, 2.0) - 1.0).abs() < 0.001);
}

#[test]
fn test_mathf_power_of_two() {
    assert!(Mathf::IsPowerOfTwo(4));
    assert!(!Mathf::IsPowerOfTwo(5));
    assert_eq!(Mathf::NextPowerOfTwo(5), 8);
    assert_eq!(Mathf::ClosestPowerOfTwo(5), 4);
    assert_eq!(Mathf::NextPowerOfTwo(0), 1);
}

#[test]
fn test_mathf_color_space() {
    let linear = Mathf::GammaToLinearSpace(0.5);
    let gamma = Mathf::LinearToGammaSpace(linear);
    assert!((gamma - 0.5).abs() < 0.01);
}

#[test]
fn test_force_mode_force() {
    let mut rb = Rigidbody::default();
    rb.mass = 2.0;
    rb.AddForceWithMode(Vec3::new(10.0, 0.0, 0.0), ForceMode::Force);
    assert!((rb.velocity.x - 5.0).abs() < 0.001);
}

#[test]
fn test_force_mode_velocity_change() {
    let mut rb = Rigidbody::default();
    rb.mass = 2.0;
    rb.AddForceWithMode(Vec3::new(10.0, 0.0, 0.0), ForceMode::VelocityChange);
    assert!((rb.velocity.x - 10.0).abs() < 0.001);
}

#[test]
fn test_add_force_at_position() {
    let mut rb = Rigidbody::default();
    rb.mass = 1.0;
    rb.AddForceAtPosition(Vec3::new(0.0, 10.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO);
    assert!((rb.velocity.y - 10.0).abs() < 0.001);
    assert!(rb.angular_velocity.z.abs() > 0.0);
}

#[test]
fn test_transform_batch_transform_points() {
    let mut t = Transform::default();
    t.SetPosition(Vec3::new(1.0, 0.0, 0.0));
    let points = vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)];
    let transformed = t.TransformPoints(&points);
    assert_eq!(transformed.len(), 2);
    assert!((transformed[0].x - 1.0).abs() < 0.001);
    assert!((transformed[1].x - 2.0).abs() < 0.001);
}

#[test]
fn test_transform_batch_inverse_transform_points() {
    let mut t = Transform::default();
    t.SetPosition(Vec3::new(5.0, 0.0, 0.0));
    let points = vec![Vec3::new(5.0, 0.0, 0.0), Vec3::new(7.0, 0.0, 0.0)];
    let transformed = t.InverseTransformPoints(&points);
    assert_eq!(transformed.len(), 2);
    assert!(transformed[0].x.abs() < 0.001);
    assert!((transformed[1].x - 2.0).abs() < 0.001);
}

#[test]
fn test_transform_batch_transform_directions() {
    let mut t = Transform::default();
    t.SetLocalPosition(Vec3::new(5.0, 0.0, 0.0));
    let dirs = vec![Vec3::Z, Vec3::X];
    let transformed = t.TransformDirections(&dirs);
    assert_eq!(transformed.len(), 2);
    assert!((transformed[0].z - 1.0).abs() < 0.001);
    assert!((transformed[1].x - 1.0).abs() < 0.001);
}

#[test]
fn test_transform_batch_inverse_transform_directions() {
    let t = Transform::default();
    let dirs = vec![Vec3::Z, Vec3::X];
    let transformed = t.InverseTransformDirections(&dirs);
    assert_eq!(transformed.len(), 2);
    assert!((transformed[0].z - 1.0).abs() < 0.001);
    assert!((transformed[1].x - 1.0).abs() < 0.001);
}

#[test]
fn test_transform_child_count() {
    let t = Transform::default();
    assert_eq!(t.ChildCount(), 0);
}

#[test]
fn test_transform_set_local_position_rotation_and_scale() {
    let mut t = Transform::default();
    assert!(!t.HasChanged());
    t.SetLocalPositionAndRotationAndScale(
        Vec3::new(1.0, 2.0, 3.0),
        Quat::from_rotation_y(1.57),
        Vec3::new(2.0, 2.0, 2.0),
    );
    assert!(t.HasChanged());
    assert_eq!(t.LocalPosition(), Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(t.LocalScale(), Vec3::new(2.0, 2.0, 2.0));
}

#[test]
fn test_mesh_filter() {
    let mf = MeshFilter::default();
    assert_eq!(mf.mesh, "Cube");
}

#[test]
fn test_monobehaviour_new_callbacks() {
    use engine_core::behaviour::Behaviour;
    use engine_core::component::Component;
    use engine_core::gameobject::GameObjectHandle;

    struct TestBehaviour {
        validated: bool,
        joint_broken: bool,
        parent_changed: bool,
        children_changed: bool,
        rendered: bool,
        will_render: bool,
        pre_render: bool,
        post_render: bool,
        reset_called: bool,
    }

    impl Component for TestBehaviour {
        fn as_any(&self) -> &dyn std::any::Any { self }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    }

    impl Behaviour for TestBehaviour {
        fn Enabled(&self) -> bool { true }
        fn SetEnabled(&mut self, _enabled: bool) {}
        fn IsActiveAndEnabled(&self) -> bool { true }
        fn set_gameobject(&mut self, _handle: GameObjectHandle) {}
        fn gameobject_handle(&self) -> Option<GameObjectHandle> { None }
    }

    impl MonoBehaviour for TestBehaviour {
        fn TypeName(&self) -> &str { "TestBehaviour" }

        fn OnValidate(&mut self) { self.validated = true; }
        fn Reset(&mut self) { self.reset_called = true; }
        fn OnTransformParentChanged(&mut self) { self.parent_changed = true; }
        fn OnTransformChildrenChanged(&mut self) { self.children_changed = true; }
        fn OnJointBreak(&mut self, _breakForce: f32) { self.joint_broken = true; }
        fn OnRenderObject(&mut self) { self.rendered = true; }
        fn OnWillRenderObject(&mut self) { self.will_render = true; }
        fn OnPreRender(&mut self) { self.pre_render = true; }
        fn OnPostRender(&mut self) { self.post_render = true; }
    }

    let mut tb = TestBehaviour {
        validated: false,
        joint_broken: false,
        parent_changed: false,
        children_changed: false,
        rendered: false,
        will_render: false,
        pre_render: false,
        post_render: false,
        reset_called: false,
    };

    tb.OnValidate();
    assert!(tb.validated);

    tb.Reset();
    assert!(tb.reset_called);

    tb.OnTransformParentChanged();
    assert!(tb.parent_changed);

    tb.OnTransformChildrenChanged();
    assert!(tb.children_changed);

    tb.OnJointBreak(100.0);
    assert!(tb.joint_broken);

    tb.OnRenderObject();
    assert!(tb.rendered);

    tb.OnWillRenderObject();
    assert!(tb.will_render);

    tb.OnPreRender();
    assert!(tb.pre_render);

    tb.OnPostRender();
    assert!(tb.post_render);
}

#[test]
fn test_character_controller() {
    use engine_core::character_controller::CharacterController;
    use engine_math::Vec3;
    let mut cc = CharacterController::default();
    assert!(!cc.IsGrounded());
    cc.is_grounded = true;
    assert!(cc.IsGrounded());
    cc.SimpleMove(5.0);
    assert!((cc.velocity.z + 5.0).abs() < 0.001);
}

#[test]
fn test_scene_manager() {
    use engine_core::scene_management::SceneManager;
    let mut sm = SceneManager::new();
    assert_eq!(sm.SceneCount(), 0);
    let handle = sm.LoadScene("TestScene").unwrap();
    assert_eq!(sm.SceneCount(), 1);
    let active = sm.GetActiveScene().unwrap();
    assert_eq!(active.name, "TestScene");
    assert!(active.is_loaded);
    sm.UnloadScene(handle).unwrap();
    let loaded = sm.GetLoadedScenes();
    assert!(loaded.is_empty());
}

#[test]
fn test_scene_handle_invalid() {
    use engine_core::scene_management::SceneHandle;
    assert_eq!(SceneHandle::INVALID.0, u32::MAX);
}

#[test]
fn test_debug_log() {
    use engine_core::debug_utils::Debug;
    Debug::Log("test message");
    Debug::LogWarning("test warning");
    Debug::LogError("test error");
}

#[test]
fn test_gizmos_placeholders() {
    use engine_core::debug_utils::Gizmos;
    use engine_math::Vec3;
    Gizmos::DrawSphere(Vec3::ZERO, 1.0);
    Gizmos::DrawCube(Vec3::ZERO, Vec3::ONE);
    Gizmos::DrawWireSphere(Vec3::ZERO, 1.0);
    Gizmos::DrawWireCube(Vec3::ZERO, Vec3::ONE);
    Gizmos::DrawLine(Vec3::ZERO, Vec3::X);
    Gizmos::DrawRay(Vec3::ZERO, Vec3::Y);
}

#[test]
fn test_random_range() {
    for _ in 0..100 {
        let v = Random::Range(2.0, 5.0);
        assert!(v >= 2.0 && v <= 5.0);
    }
}

#[test]
fn test_random_value() {
    for _ in 0..100 {
        let v = Random::Value();
        assert!(v >= 0.0 && v <= 1.0);
    }
}
