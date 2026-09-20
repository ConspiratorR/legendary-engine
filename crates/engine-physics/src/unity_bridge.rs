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

        // Full Transform for physics integration (world pose).
        if let Some(t) = runtime.world.GetTransform(go) {
            let pos = t.Position();
            let rot = t.Rotation();
            let scale = t.LossyScale();
            let proxy = CoreTransform::from_position_rotation_scale(pos, rot, scale);
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
        } else if unity_rb.use_gravity {
            RigidBody::new_dynamic()
        } else {
            RigidBody::new_static()
        };
        body.mass = unity_rb.mass.max(0.001);
        body.linear_damping = unity_rb.drag;
        body.angular_damping = unity_rb.angular_drag;
        // Velocity authority: simulation owns velocity on existing bodies.
        // Only seed from Unity on first insert (or when Unity publishes a non-zero velocity).
        let is_new = ecs.get::<RigidBody>(entity).is_none();
        if is_new || unity_rb.velocity != Vec3::ZERO {
            body.linear_velocity = unity_rb.velocity;
        }
        if is_new || unity_rb.angular_velocity != Vec3::ZERO {
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
        runtime.world.SetLocalPosition(go, local);

        // Round-trip velocity onto Unity Rigidbody (simulation → gameplay view).
        if let Some(body) = ecs.get::<RigidBody>(entity)
            && let Some(rb) = runtime.world.GetComponentMut::<Rigidbody>(go)
        {
            rb.velocity = body.linear_velocity;
            rb.angular_velocity = body.angular_velocity;
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

    /// Parented body: world sim pose → local via parent inverse (phase18 residual).
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
            .SetLocalPosition(child, Vec3::new(0.0, 1.0, 0.0)); // world (10,1,0)
        runtime.world.AddComponent(
            child,
            UnityRb {
                use_gravity: false,
                is_kinematic: false,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.sync_transforms();

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        let n = sync_physics_from_unity(&mut runtime, &mut ecs);
        assert!(n >= 1);

        // Put simulation world position at (10, 4, 0) → local should be (0,4,0).
        let entity = runtime.entity_for(child).expect("child entity");
        if let Some(t) = ecs.get_mut::<CoreTransform>(entity) {
            t.SetPosition(Vec3::new(10.0, 4.0, 0.0));
        } else {
            let mut t = CoreTransform::from_position_rotation_scale(
                Vec3::new(10.0, 4.0, 0.0),
                engine_math::Quat::IDENTITY,
                Vec3::ONE,
            );
            t.SetPosition(Vec3::new(10.0, 4.0, 0.0));
            ecs.add_component(entity, t);
        }

        let w = sync_physics_to_unity(&mut runtime, &ecs);
        assert!(w >= 1);
        let local = runtime.world.GetTransform(child).unwrap().LocalPosition();
        assert!(
            (local - Vec3::new(0.0, 4.0, 0.0)).length() < 0.05,
            "parented writeback must convert world→local; got {local:?}"
        );
        let world_pos = runtime.world.GetTransform(child).unwrap().Position();
        assert!(
            (world_pos - Vec3::new(10.0, 4.0, 0.0)).length() < 0.05,
            "world pose after sync_transforms; got {world_pos:?}"
        );
    }
}
