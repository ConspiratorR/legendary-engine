//! Bridge Unity [`SceneRuntime`] World poses ↔ ECS physics components (phase 18).
//!
//! Runtime counterpart of editor `UnityPlayHost` physics sync:
//! 1. **from_unity** — walk SceneRuntime GameObjects; for each Unity `Rigidbody`
//!    write ECS `engine_core::transform::Transform` (world pose) + physics
//!    `RigidBody` / `Collider` onto the identity-bridge entity.
//! 2. **PhysicsWorld::step** — simulates on ECS `Transform` + `RigidBody`.
//! 3. **to_unity** — write simulated **world** position back onto Unity World
//!    via storage-authority `with_ecs_transform_mut` / `SetLocalPosition`
//!    (ECS-primary when `unity-world-primary` is on; array fallback when off).
//!
//! Unity World remains the gameplay surface; physics ECS is the simulation store.

use crate::body::RigidBody;
use crate::collider::Collider;
use crate::world::PhysicsWorld;
use engine_core::components::{BoxCollider, Rigidbody, SphereCollider};
use engine_core::gameobject::GameObjectHandle;
use engine_core::scene_runtime::SceneRuntime;
use engine_core::transform::Transform as CoreTransform;
use engine_ecs::world::World as EcsWorld;
use engine_math::Vec3;
use std::collections::HashMap;

/// Collect every valid GameObject handle under SceneRuntime World roots.
pub fn collect_unity_handles(unity: &engine_core::world::World) -> Vec<GameObjectHandle> {
    let mut out = Vec::new();
    let mut stack: Vec<GameObjectHandle> = unity.GetRootGameObjects();
    while let Some(h) = stack.pop() {
        out.push(h);
        stack.extend(unity.GetChildren(h));
    }
    out
}

/// Copy Unity Rigidbody / Transform / colliders into ECS physics components
/// on identity-bridge entities (creates links when missing).
pub fn sync_physics_from_unity(runtime: &mut SceneRuntime, ecs: &mut EcsWorld) -> usize {
    runtime.world.sync_transforms();

    let handles = collect_unity_handles(&runtime.world);

    let mut go_to_entity: HashMap<GameObjectHandle, engine_ecs::entity::Entity> = HashMap::new();
    for &go in &handles {
        let e = match runtime.entity_for(go) {
            Some(e) => e,
            None => runtime.link_to_ecs(go, ecs),
        };
        go_to_entity.insert(go, e);
    }

    let mut synced = 0usize;
    for (&go, &entity) in &go_to_entity {
        let Some(unity_rb) = runtime.world.GetComponent::<Rigidbody>(go) else {
            continue;
        };

        // Full Transform for physics integration (world pose + rotation).
        if let Some(t) = runtime.world.GetTransform(go) {
            let pos = t.Position();
            let rot = t.Rotation();
            let scale = t.LossyScale();
            let mut proxy = CoreTransform::from_position_rotation_scale(pos, rot, scale);
            proxy.SetPosition(pos);
            proxy.SetRotation(rot);
            if ecs.get::<CoreTransform>(entity).is_some() {
                *ecs.get_mut::<CoreTransform>(entity).unwrap() = proxy;
            } else {
                ecs.add_component(entity, proxy);
            }
        }

        let mut body = if ecs.get::<RigidBody>(entity).is_some() {
            ecs.get_mut::<RigidBody>(entity).unwrap().clone()
        } else if unity_rb.is_kinematic {
            RigidBody::new_kinematic()
        } else {
            // Unity: use_gravity=false still yields a Dynamic Rigidbody.
            RigidBody::new_dynamic()
        };
        body.mass = unity_rb.mass.max(0.001);
        body.linear_damping = unity_rb.drag;
        body.angular_damping = unity_rb.angular_drag;
        body.gravity_scale = if unity_rb.use_gravity { 1.0 } else { 0.0 };
        let is_new = ecs.get::<RigidBody>(entity).is_none();
        if unity_rb.is_sleeping {
            // Sleep gate: stop simulation state regardless of prior velocities.
            body.is_sleeping = true;
            body.linear_velocity = Vec3::ZERO;
            body.angular_velocity = Vec3::ZERO;
        } else if is_new || unity_rb.velocity != Vec3::ZERO {
            body.is_sleeping = false;
            body.linear_velocity = unity_rb.velocity;
        }
        if !unity_rb.is_sleeping && (is_new || unity_rb.angular_velocity != Vec3::ZERO) {
            body.angular_velocity = unity_rb.angular_velocity;
        }
        if ecs.get::<RigidBody>(entity).is_some() {
            *ecs.get_mut::<RigidBody>(entity).unwrap() = body;
        } else {
            ecs.add_component(entity, body);
        }

        if let Some(sc) = runtime.world.GetComponent::<SphereCollider>(go) {
            let col = Collider::sphere(sc.radius.max(0.01));
            if ecs.get::<Collider>(entity).is_some() {
                *ecs.get_mut::<Collider>(entity).unwrap() = col;
            } else {
                ecs.add_component(entity, col);
            }
        } else if let Some(bc) = runtime.world.GetComponent::<BoxCollider>(go) {
            let h = bc.size * 0.5;
            let col = Collider::cuboid(h.x.max(0.01), h.y.max(0.01), h.z.max(0.01));
            if ecs.get::<Collider>(entity).is_some() {
                *ecs.get_mut::<Collider>(entity).unwrap() = col;
            } else {
                ecs.add_component(entity, col);
            }
        }
        synced += 1;
    }
    synced
}

/// Write simulated ECS world pose **and velocity** back onto Unity World.
///
/// Pose: physics stores world position; convert to parent-local before
/// `SetLocalPosition` so parented bodies stay coherent. Velocity is written
/// back onto Unity `Rigidbody` so the next `from_unity` does not zero the
/// simulation state.
pub fn sync_physics_to_unity(runtime: &mut SceneRuntime, ecs: &EcsWorld) -> usize {
    let handles = collect_unity_handles(&runtime.world);
    let mut written = 0usize;
    for go in handles {
        let Some(entity) = runtime.entity_for(go) else {
            continue;
        };
        if ecs.get::<RigidBody>(entity).is_none() {
            continue;
        }
        let Some(ecs_t) = ecs.get::<CoreTransform>(entity) else {
            continue;
        };
        let world_pos = ecs_t.Position();
        let local = world_to_local_position(&runtime.world, go, world_pos);
        let world_rot = ecs_t.Rotation();
        let local_rot = world_to_local_rotation(&runtime.world, go, world_rot);
        let _ = runtime.world.with_ecs_transform_mut(go, |t| {
            // Keep both local and world fields coherent for dual-read / next from_unity.
            t.SetLocalPosition(local);
            t.SetPosition(world_pos);
            t.SetLocalRotation(local_rot);
            t.SetRotation(world_rot);
        });

        // Round-trip velocity + sleep onto Unity Rigidbody.
        if let Some(body) = ecs.get::<RigidBody>(entity)
            && let Some(rb) = runtime.world.GetComponentMut::<Rigidbody>(go)
        {
            rb.velocity = body.linear_velocity;
            rb.angular_velocity = body.angular_velocity;
            rb.is_sleeping = body.is_sleeping;
        }
        written += 1;
    }
    runtime.world.sync_transforms();
    written
}

/// Convert a world-space point into `go` local space using the parent's inverse.
fn world_to_local_position(
    unity: &engine_core::world::World,
    go: GameObjectHandle,
    world_pos: engine_math::Vec3,
) -> engine_math::Vec3 {
    if let Some(parent) = unity.GetParent(go)
        && let Some(pt) = unity.GetTransform(parent)
    {
        return pt.InverseTransformPoint(world_pos);
    }
    world_pos
}

/// Convert world rotation into `go` local space (parent inverse * world).
fn world_to_local_rotation(
    unity: &engine_core::world::World,
    go: GameObjectHandle,
    world_rot: engine_math::Quat,
) -> engine_math::Quat {
    if let Some(parent) = unity.GetParent(go)
        && let Some(pt) = unity.GetTransform(parent)
    {
        return pt.Rotation().inverse() * world_rot;
    }
    world_rot
}

/// Fixed-step: Unity → physics, step, physics → Unity.
pub fn unity_physics_fixed_step(runtime: &mut SceneRuntime, ecs: &mut EcsWorld, fixed_dt: f32) {
    sync_physics_from_unity(runtime, ecs);
    if let Some(mut pw) = ecs.remove_resource::<PhysicsWorld>() {
        pw.delta_time = fixed_dt;
        pw.step(ecs);
        ecs.insert_resource(pw);
    }
    sync_physics_to_unity(runtime, ecs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::components::Rigidbody as UnityRb;
    use engine_math::Vec3;

    #[test]
    fn test_unity_physics_gravity_writes_world_pose() {
        let mut runtime = SceneRuntime::new();
        let go = runtime.world.CreateGameObject("Falling");
        runtime.world.SetLocalPosition(go, Vec3::new(0.0, 8.0, 0.0));
        runtime.world.AddComponent(
            go,
            UnityRb {
                use_gravity: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            go,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.5,
                is_trigger: false,
            },
        );

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());

        // ~0.5 s free-fall with velocity accumulation (not n·g·dt² zero-reset).
        for _ in 0..25 {
            unity_physics_fixed_step(&mut runtime, &mut ecs, 0.02);
        }

        let y1 = runtime.world.GetTransform(go).unwrap().LocalPosition().y;
        assert!(
            y1 < 8.0 - 0.5,
            "velocity must accumulate under gravity; end Y={y1}"
        );
        let rb = runtime
            .world
            .GetComponent::<UnityRb>(go)
            .expect("Rigidbody");
        assert!(
            rb.velocity.y < -0.5,
            "to_unity should write back downward linear_velocity; got {:?}",
            rb.velocity
        );
    }

    #[test]
    fn test_sync_physics_to_unity_skips_without_rigidbody() {
        let mut runtime = SceneRuntime::new();
        let go = runtime.world.CreateGameObject("StaticGO");
        runtime.world.SetLocalPosition(go, Vec3::new(1.0, 2.0, 3.0));
        let ecs = EcsWorld::new();
        let n = sync_physics_to_unity(&mut runtime, &ecs);
        assert_eq!(n, 0);
        let p = runtime.world.GetTransform(go).unwrap().LocalPosition();
        assert_eq!(p, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_to_unity_parented_body_uses_local_space() {
        let mut runtime = SceneRuntime::new();
        let parent = runtime.world.CreateGameObject("Parent");
        runtime
            .world
            .SetLocalPosition(parent, Vec3::new(10.0, 0.0, 0.0));
        let child = runtime.world.CreateGameObject("Child");
        runtime.world.SetParent(child, Some(parent));
        runtime
            .world
            .SetLocalPosition(child, Vec3::new(0.0, 1.0, 0.0));
        runtime.world.AddComponent(
            child,
            UnityRb {
                use_gravity: false,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.sync_transforms();

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        assert!(sync_physics_from_unity(&mut runtime, &mut ecs) >= 1);

        let entity = runtime.entity_for(child).expect("child entity");
        let mut t = CoreTransform::from_position_rotation_scale(
            Vec3::new(10.0, 4.0, 0.0),
            engine_math::Quat::IDENTITY,
            Vec3::ONE,
        );
        t.SetPosition(Vec3::new(10.0, 4.0, 0.0));
        if ecs.get::<CoreTransform>(entity).is_some() {
            *ecs.get_mut::<CoreTransform>(entity).unwrap() = t;
        } else {
            ecs.add_component(entity, t);
        }

        assert!(sync_physics_to_unity(&mut runtime, &ecs) >= 1);
        let local = runtime.world.GetTransform(child).unwrap().LocalPosition();
        assert!(
            (local - Vec3::new(0.0, 4.0, 0.0)).length() < 0.05,
            "parented writeback must convert world→local; got {local:?}"
        );
    }

    #[test]
    fn test_physics_rotation_and_sleep() {
        let mut runtime = SceneRuntime::new();
        let go = runtime.world.CreateGameObject("Spinner");
        runtime.world.SetLocalPosition(go, Vec3::ZERO);
        runtime.world.AddComponent(
            go,
            UnityRb {
                use_gravity: false,
                mass: 1.0,
                angular_velocity: Vec3::new(0.0, 3.0, 0.0),
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            go,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.5,
                is_trigger: false,
            },
        );
        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());

        let rot0 = runtime.world.GetTransform(go).unwrap().LocalRotation();
        for _ in 0..10 {
            unity_physics_fixed_step(&mut runtime, &mut ecs, 0.02);
        }
        let rot1 = runtime.world.GetTransform(go).unwrap().LocalRotation();
        assert!(
            (rot1.x - rot0.x).abs() > 1e-4
                || (rot1.y - rot0.y).abs() > 1e-4
                || (rot1.z - rot0.z).abs() > 1e-4
                || (rot1.w - rot0.w).abs() > 1e-4,
            "angular velocity should rotate World; rot0={rot0:?} rot1={rot1:?}"
        );

        if let Some(rb) = runtime.world.GetComponentMut::<UnityRb>(go) {
            rb.velocity = Vec3::new(0.0, -50.0, 0.0);
            rb.Sleep();
        }
        let pos_before = runtime.world.GetTransform(go).unwrap().LocalPosition();
        for _ in 0..10 {
            unity_physics_fixed_step(&mut runtime, &mut ecs, 0.02);
        }
        let pos_after = runtime.world.GetTransform(go).unwrap().LocalPosition();
        assert!(
            (pos_after - pos_before).length() < 0.05,
            "sleeping body must not move; {pos_before:?} -> {pos_after:?}"
        );
    }

    #[test]
    fn test_invoke_collision_enter_counts() {
        use engine_core::events::Collision;
        use engine_core::time::Time;
        use engine_math::Quat;
        use std::any::Any;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        #[derive(Default)]
        struct HitCounter {
            go: Option<GameObjectHandle>,
            hits: Arc<AtomicU32>,
        }
        impl engine_core::component::Component for HitCounter {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl engine_core::behaviour::Behaviour for HitCounter {
            fn Enabled(&self) -> bool {
                true
            }
            fn SetEnabled(&mut self, _e: bool) {}
            fn IsActiveAndEnabled(&self) -> bool {
                true
            }
            fn set_gameobject(&mut self, h: GameObjectHandle) {
                self.go = Some(h);
            }
            fn gameobject_handle(&self) -> Option<GameObjectHandle> {
                self.go
            }
        }
        impl engine_core::monobehaviour::MonoBehaviour for HitCounter {
            fn OnCollisionEnter(
                &mut self,
                _ctx: &mut engine_core::context::Context,
                _c: &Collision,
            ) {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut runtime = SceneRuntime::new();
        let a = runtime.world.CreateGameObject("A");
        let b = runtime.world.CreateGameObject("B");
        let hits = Arc::new(AtomicU32::new(0));
        runtime.world.AddMonoBehaviour(
            a,
            HitCounter {
                go: None,
                hits: hits.clone(),
            },
        );
        let mut bus = engine_core::event::EventBus::new();
        runtime.world.invoke_collision_enter(
            a,
            Collision {
                other: b,
                normal: Vec3::new(0.0, 1.0, 0.0),
                point: Vec3::ZERO,
                relative_velocity: Vec3::ZERO,
            },
            Time::default(),
            1,
            &mut bus,
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        let _ = Quat::IDENTITY;
    }
}
