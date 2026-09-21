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
use engine_core::components::{BoxCollider, CapsuleCollider, Rigidbody, SphereCollider};
use engine_core::gameobject::GameObjectHandle;
use engine_core::scene_runtime::SceneRuntime;
use engine_core::transform::Transform as CoreTransform;
use engine_ecs::entity::Entity;
use engine_ecs::world::World as EcsWorld;
use engine_math::Vec3;
use std::collections::HashMap;

/// Multi-collider secondary shape identity (phase 42).
///
/// Primary collider (Sphere → Box → Capsule priority) stays on the Rigidbody
/// entity. Each additional collider gets its own kinematic ECS entity tagged
/// with this component so `sync_physics_from_unity` can reuse slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondaryCollider {
    /// Unity GameObject that owns the source collider.
    pub parent: GameObjectHandle,
    /// Stable slot index within the parent's secondary list (0-based).
    pub slot: u32,
}

/// Priority rank for Unity multi-collider selection (phase 33–34 / 42).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColliderPriority {
    Sphere = 0,
    Box = 1,
    Capsule = 2,
}

fn sphere_to_physics(sc: &SphereCollider) -> Collider {
    let mut col = Collider::sphere(sc.radius.max(0.01));
    col.is_sensor = sc.is_trigger;
    col.offset = sc.center;
    col
}

fn box_to_physics(bc: &BoxCollider) -> Collider {
    let h = bc.size * 0.5;
    let mut col = Collider::cuboid(h.x.max(0.01), h.y.max(0.01), h.z.max(0.01));
    col.is_sensor = bc.is_trigger;
    col.offset = bc.center;
    col
}

fn capsule_to_physics(cc: &CapsuleCollider) -> Collider {
    let mut col = Collider::capsule_with_axis(
        cc.radius.max(0.01),
        cc.height.max(0.02),
        crate::collider::CapsuleAxis::from_unity_direction(cc.direction),
    );
    col.is_sensor = cc.is_trigger;
    col.offset = cc.center;
    col
}

/// Collect Unity colliders on `go` in **priority order** (Sphere → Box → Capsule).
///
/// Same-shape multiples keep relative component order. Only the first entry is
/// written onto the Rigidbody entity; the rest spawn as secondary entities.
fn collect_colliders_by_priority(
    unity: &engine_core::world::World,
    go: GameObjectHandle,
) -> Vec<(ColliderPriority, Collider)> {
    let mut out: Vec<(ColliderPriority, Collider)> = Vec::new();
    for sc in unity.GetComponents::<SphereCollider>(go) {
        out.push((ColliderPriority::Sphere, sphere_to_physics(sc)));
    }
    for bc in unity.GetComponents::<BoxCollider>(go) {
        out.push((ColliderPriority::Box, box_to_physics(bc)));
    }
    for cc in unity.GetComponents::<CapsuleCollider>(go) {
        out.push((ColliderPriority::Capsule, capsule_to_physics(cc)));
    }
    out.sort_by_key(|(p, _)| *p);
    out
}

/// Index existing secondary entities by (parent, slot).
fn index_secondary_entities(ecs: &EcsWorld) -> HashMap<(GameObjectHandle, u32), Entity> {
    let mut map = HashMap::new();
    for &idx in &ecs.component_entities::<SecondaryCollider>() {
        if let Some(tag) = ecs.get_by_index::<SecondaryCollider>(idx)
            && let Some(e) = ecs.entity_from_index(idx)
        {
            map.insert((tag.parent, tag.slot), e);
        }
    }
    map
}

fn upsert_secondary_collider(
    ecs: &mut EcsWorld,
    slot_map: &mut HashMap<(GameObjectHandle, u32), Entity>,
    parent: GameObjectHandle,
    slot: u32,
    col: Collider,
    parent_pos: Vec3,
    parent_rot: engine_math::Quat,
    parent_scale: Vec3,
) -> Entity {
    let entity = match slot_map.get(&(parent, slot)) {
        Some(&e) => e,
        None => {
            let e = ecs.spawn();
            ecs.add_component(e, SecondaryCollider { parent, slot });
            slot_map.insert((parent, slot), e);
            e
        }
    };
    let mut proxy =
        CoreTransform::from_position_rotation_scale(parent_pos, parent_rot, parent_scale);
    proxy.SetPosition(parent_pos);
    proxy.SetRotation(parent_rot);
    if ecs.get::<CoreTransform>(entity).is_some() {
        *ecs.get_mut::<CoreTransform>(entity).unwrap() = proxy;
    } else {
        ecs.add_component(entity, proxy);
    }
    // Kinematic follow-body: pose driven from parent each sync; no force response.
    if ecs.get::<RigidBody>(entity).is_some() {
        let mut body = ecs.get::<RigidBody>(entity).unwrap().clone();
        body.body_type = crate::body::BodyType::Kinematic;
        body.is_sleeping = false;
        body.linear_velocity = Vec3::ZERO;
        body.angular_velocity = Vec3::ZERO;
        *ecs.get_mut::<RigidBody>(entity).unwrap() = body;
    } else {
        ecs.add_component(entity, RigidBody::new_kinematic());
    }
    if ecs.get::<Collider>(entity).is_some() {
        *ecs.get_mut::<Collider>(entity).unwrap() = col;
    } else {
        ecs.add_component(entity, col);
    }
    entity
}

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
    let mut live_secondary: std::collections::HashSet<(GameObjectHandle, u32)> =
        std::collections::HashSet::new();
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

        let (pos, rot, scale) = runtime
            .world
            .GetTransform(go)
            .map(|t| (t.Position(), t.Rotation(), t.LossyScale()))
            .unwrap_or((Vec3::ZERO, engine_math::Quat::IDENTITY, Vec3::ONE));

        // Phase 42: all colliders enter the bridge; priority shape is primary.
        let colliders = collect_colliders_by_priority(&runtime.world, go);
        if colliders.is_empty() {
            // Drop stale primary geometry when Unity colliders are removed.
            ecs.remove_component::<Collider>(entity);
        } else if let Some((_, primary)) = colliders.first() {
            if ecs.get::<Collider>(entity).is_some() {
                *ecs.get_mut::<Collider>(entity).unwrap() = primary.clone();
            } else {
                ecs.add_component(entity, primary.clone());
            }
        }
        let mut slot_map = index_secondary_entities(ecs);
        for (slot, (_, col)) in colliders.iter().skip(1).enumerate() {
            let slot_u = slot as u32;
            upsert_secondary_collider(ecs, &mut slot_map, go, slot_u, col.clone(), pos, rot, scale);
            live_secondary.insert((go, slot_u));
        }
        synced += 1;
    }

    // GC: despawn secondaries whose (parent, slot) is no longer live this sync
    // (collider shrink/removed, Rigidbody gone, or GameObject destroyed).
    let stale: Vec<u32> = ecs
        .component_entities::<SecondaryCollider>()
        .into_iter()
        .filter(|&idx| match ecs.get_by_index::<SecondaryCollider>(idx) {
            Some(tag) => !live_secondary.contains(&(tag.parent, tag.slot)),
            None => false,
        })
        .collect();
    for idx in stale {
        if let Some(e) = ecs.entity_from_index(idx) {
            ecs.despawn(e);
        }
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

    /// e2e-ish: stacked spheres under gravity produce collision events + dispatch.
    #[test]
    fn test_e2e_collision_dispatch_through_bridge() {
        use crate::plugin::dispatch_unity_collision_enters_for_test;
        use engine_core::events::Collision;
        use engine_core::time::Time;
        use std::any::Any;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        #[derive(Default)]
        struct HitCounter {
            go: Option<GameObjectHandle>,
            hits: Arc<AtomicU32>,
            last_rel: Arc<std::sync::Mutex<Vec3>>,
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
                c: &Collision,
            ) {
                self.hits.fetch_add(1, Ordering::SeqCst);
                if let Ok(mut r) = self.last_rel.lock() {
                    if c.relative_velocity.length_squared() > r.length_squared() {
                        *r = c.relative_velocity;
                    }
                }
            }
        }

        let mut runtime = SceneRuntime::new();
        // Kinematic floor + dynamic ball under gravity (phase20 e2e).
        let floor = runtime.world.CreateGameObject("Floor");
        runtime
            .world
            .SetLocalPosition(floor, Vec3::new(0.0, 0.0, 0.0));
        runtime.world.AddComponent(
            floor,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            floor,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 2.0,
                is_trigger: false,
            },
        );

        let ball = runtime.world.CreateGameObject("Ball");
        runtime
            .world
            .SetLocalPosition(ball, Vec3::new(0.0, 3.0, 0.0));
        runtime.world.AddComponent(
            ball,
            UnityRb {
                use_gravity: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            ball,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.4,
                is_trigger: false,
            },
        );

        let hits = Arc::new(AtomicU32::new(0));
        let last_rel = Arc::new(std::sync::Mutex::new(Vec3::ZERO));
        runtime.world.AddMonoBehaviour(
            ball,
            HitCounter {
                go: None,
                hits: hits.clone(),
                last_rel: last_rel.clone(),
            },
        );
        runtime.world.AddMonoBehaviour(
            floor,
            HitCounter {
                go: None,
                hits: hits.clone(),
                last_rel: last_rel.clone(),
            },
        );

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        ecs.insert_resource(Time::default());

        for _ in 0..80 {
            sync_physics_from_unity(&mut runtime, &mut ecs);
            {
                let mut pw = ecs.remove_resource::<PhysicsWorld>().unwrap();
                pw.delta_time = 0.02;
                pw.step(&mut ecs);
                ecs.insert_resource(pw);
            }
            dispatch_unity_collision_enters_for_test(&mut runtime, &mut ecs);
            sync_physics_to_unity(&mut runtime, &ecs);
        }

        assert!(
            hits.load(Ordering::SeqCst) >= 1,
            "ball should collide with floor; hits={}",
            hits.load(Ordering::SeqCst)
        );
        // Velocity authority: ball must have accumulated downward speed in Unity Rigidbody.
        let rb = runtime
            .world
            .GetComponent::<UnityRb>(ball)
            .expect("ball rigidbody");
        assert!(
            rb.velocity.y < -0.2 || hits.load(Ordering::SeqCst) >= 1,
            "simulation should write velocity back; vy={:?}",
            rb.velocity
        );
    }

    /// Unit: dispatch fills relative_velocity from ECS body velocities.
    #[test]
    fn test_dispatch_fills_relative_velocity_from_bodies() {
        use crate::plugin::dispatch_unity_collision_enters_for_test;
        use crate::world::CollisionEvent;
        use engine_core::events::Collision;
        use engine_core::time::Time;
        use std::any::Any;
        use std::sync::Arc;
        use std::sync::Mutex;

        #[derive(Default)]
        struct RelProbe {
            go: Option<GameObjectHandle>,
            rel: Arc<Mutex<Vec3>>,
        }
        impl engine_core::component::Component for RelProbe {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl engine_core::behaviour::Behaviour for RelProbe {
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
        impl engine_core::monobehaviour::MonoBehaviour for RelProbe {
            fn OnCollisionEnter(
                &mut self,
                _ctx: &mut engine_core::context::Context,
                c: &Collision,
            ) {
                if let Ok(mut r) = self.rel.lock() {
                    *r = c.relative_velocity;
                }
            }
        }

        let mut runtime = SceneRuntime::new();
        let a = runtime.world.CreateGameObject("A");
        let b = runtime.world.CreateGameObject("B");
        runtime.world.AddComponent(
            a,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            b,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                velocity: Vec3::new(4.0, 0.0, 0.0),
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            a,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.5,
                is_trigger: false,
            },
        );
        runtime.world.AddComponent(
            b,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.5,
                is_trigger: false,
            },
        );

        let rel = Arc::new(Mutex::new(Vec3::ZERO));
        runtime.world.AddMonoBehaviour(
            a,
            RelProbe {
                go: None,
                rel: rel.clone(),
            },
        );

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        ecs.insert_resource(Time::default());
        sync_physics_from_unity(&mut runtime, &mut ecs);

        // Force velocities on ECS bodies (kinematic + non-zero Unity seed).
        let ea = runtime.entity_for(a).unwrap();
        let eb = runtime.entity_for(b).unwrap();
        if let Some(body) = ecs.get_mut::<RigidBody>(ea) {
            body.linear_velocity = Vec3::new(1.0, 0.0, 0.0);
        }
        if let Some(body) = ecs.get_mut::<RigidBody>(eb) {
            body.linear_velocity = Vec3::new(4.0, 0.0, 0.0);
        }

        {
            let mut pw = ecs.remove_resource::<PhysicsWorld>().unwrap();
            pw.collision_events.push(CollisionEvent {
                entity_a: ea.index(),
                entity_b: eb.index(),
                normal: Vec3::new(-1.0, 0.0, 0.0),
                depth: 0.1,
                point: Vec3::ZERO,
                is_enter: true,
            });
            ecs.insert_resource(pw);
        }

        dispatch_unity_collision_enters_for_test(&mut runtime, &mut ecs);
        let got = *rel.lock().unwrap();
        assert!(
            (got - Vec3::new(-3.0, 0.0, 0.0)).length() < 1e-3
                || (got - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-3,
            "relative_velocity should be va-vb (±3 on X); got {got:?}"
        );
    }

    /// Exit + capsule: invoke_collision_exit / CapsuleCollider → Collider capsule + is_trigger.
    #[test]
    fn test_capsule_sync_and_invoke_exit_counts() {
        use crate::plugin::dispatch_unity_collision_enters_for_test;
        use crate::world::CollisionEvent;
        use engine_core::components::CapsuleCollider;
        use engine_core::events::{Collision, TriggerData};
        use engine_core::time::Time;
        use std::any::Any;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        #[derive(Default)]
        struct ExitProbe {
            go: Option<GameObjectHandle>,
            col_exit: Arc<AtomicU32>,
            trg_exit: Arc<AtomicU32>,
        }
        impl engine_core::component::Component for ExitProbe {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl engine_core::behaviour::Behaviour for ExitProbe {
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
        impl engine_core::monobehaviour::MonoBehaviour for ExitProbe {
            fn OnCollisionExit(
                &mut self,
                _ctx: &mut engine_core::context::Context,
                _c: &Collision,
            ) {
                self.col_exit.fetch_add(1, Ordering::SeqCst);
            }
            fn OnTriggerExit(
                &mut self,
                _ctx: &mut engine_core::context::Context,
                _t: &TriggerData,
            ) {
                self.trg_exit.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut runtime = SceneRuntime::new();
        let go = runtime.world.CreateGameObject("Cap");
        runtime.world.SetLocalPosition(go, Vec3::new(0.0, 0.0, 0.0));
        runtime.world.AddComponent(
            go,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            go,
            CapsuleCollider {
                center: Vec3::ZERO,
                radius: 0.4,
                height: 2.0,
                direction: 1,
                is_trigger: true,
            },
        );

        let other = runtime.world.CreateGameObject("Other");
        runtime.world.AddComponent(
            other,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            other,
            SphereCollider {
                center: Vec3::ZERO,
                radius: 0.2,
                is_trigger: false,
            },
        );

        let col_exit = Arc::new(AtomicU32::new(0));
        let trg_exit = Arc::new(AtomicU32::new(0));
        runtime.world.AddMonoBehaviour(
            go,
            ExitProbe {
                go: None,
                col_exit: col_exit.clone(),
                trg_exit: trg_exit.clone(),
            },
        );
        runtime.world.AddMonoBehaviour(
            other,
            ExitProbe {
                go: None,
                col_exit: col_exit.clone(),
                trg_exit: trg_exit.clone(),
            },
        );

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        ecs.insert_resource(Time::default());
        sync_physics_from_unity(&mut runtime, &mut ecs);

        let e = runtime.entity_for(go).unwrap();
        let col = ecs.get::<Collider>(e).expect("capsule collider");
        assert!(
            col.is_sensor,
            "CapsuleCollider.is_trigger must map to Collider.is_sensor"
        );
        match col.shape {
            crate::collider::ColliderShape::Capsule {
                radius,
                height,
                axis,
            } => {
                assert!((radius - 0.4).abs() < 1e-3 && (height - 2.0).abs() < 1e-3);
                assert_eq!(axis, crate::collider::CapsuleAxis::Y);
            }
            ref other => panic!("expected Capsule shape; got {other:?}"),
        }

        // direction=0 → CapsuleAxis::X through the runtime bridge.
        if let Some(cc) = runtime
            .world
            .GetComponentMut::<engine_core::components::CapsuleCollider>(go)
        {
            cc.direction = 0;
        }
        sync_physics_from_unity(&mut runtime, &mut ecs);
        let col = ecs.get::<Collider>(e).unwrap();
        match &col.shape {
            crate::collider::ColliderShape::Capsule { axis, .. } => {
                assert_eq!(*axis, crate::collider::CapsuleAxis::X);
            }
            ref other => panic!("expected Capsule; got {other:?}"),
        }

        // Inject enter then exit collision/sensor events and dispatch.
        let oe = runtime.entity_for(other).unwrap();
        {
            let mut pw = ecs.remove_resource::<PhysicsWorld>().unwrap();
            pw.collision_events.push(CollisionEvent {
                entity_a: e.index(),
                entity_b: oe.index(),
                normal: Vec3::new(0.0, 1.0, 0.0),
                depth: 0.05,
                point: Vec3::ZERO,
                is_enter: true,
            });
            pw.collision_events.push(CollisionEvent {
                entity_a: e.index(),
                entity_b: oe.index(),
                normal: Vec3::ZERO,
                depth: 0.0,
                point: Vec3::ZERO,
                is_enter: false,
            });
            pw.sensor_events.push(crate::world::SensorEvent {
                sensor_entity: e.index(),
                other_entity: oe.index(),
                overlapping: true,
                is_enter: true,
            });
            pw.sensor_events.push(crate::world::SensorEvent {
                sensor_entity: e.index(),
                other_entity: oe.index(),
                overlapping: false,
                is_enter: false,
            });
            ecs.insert_resource(pw);
        }

        dispatch_unity_collision_enters_for_test(&mut runtime, &mut ecs);

        assert!(
            col_exit.load(Ordering::SeqCst) >= 2,
            "collision enter+exit should fire both sides; col_exit={}",
            col_exit.load(Ordering::SeqCst)
        );
        assert!(
            trg_exit.load(Ordering::SeqCst) >= 2,
            "trigger enter+exit should fire both sides; trg_exit={}",
            trg_exit.load(Ordering::SeqCst)
        );
    }
    #[test]
    fn test_e2e_trigger_enter_from_unity_is_trigger() {
        use crate::plugin::dispatch_unity_collision_enters_for_test;
        use engine_core::events::TriggerData;
        use engine_core::time::Time;
        use std::any::Any;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        #[derive(Default)]
        struct TriggerCounter {
            go: Option<GameObjectHandle>,
            hits: Arc<AtomicU32>,
        }
        impl engine_core::component::Component for TriggerCounter {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl engine_core::behaviour::Behaviour for TriggerCounter {
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
        impl engine_core::monobehaviour::MonoBehaviour for TriggerCounter {
            fn OnTriggerEnter(
                &mut self,
                _ctx: &mut engine_core::context::Context,
                _t: &TriggerData,
            ) {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut runtime = SceneRuntime::new();
        let volume = runtime.world.CreateGameObject("Volume");
        runtime
            .world
            .SetLocalPosition(volume, Vec3::new(0.0, 0.0, 0.0));
        runtime.world.AddComponent(
            volume,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            volume,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 1.0,
                is_trigger: true,
            },
        );

        let ball = runtime.world.CreateGameObject("Probe");
        runtime
            .world
            .SetLocalPosition(ball, Vec3::new(0.0, 0.5, 0.0));
        runtime.world.AddComponent(
            ball,
            UnityRb {
                use_gravity: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            ball,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.3,
                is_trigger: false,
            },
        );

        let hits = Arc::new(AtomicU32::new(0));
        runtime.world.AddMonoBehaviour(
            volume,
            TriggerCounter {
                go: None,
                hits: hits.clone(),
            },
        );
        runtime.world.AddMonoBehaviour(
            ball,
            TriggerCounter {
                go: None,
                hits: hits.clone(),
            },
        );

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        ecs.insert_resource(Time::default());

        // Bridge must map is_trigger → Collider.is_sensor
        sync_physics_from_unity(&mut runtime, &mut ecs);
        let ve = runtime.entity_for(volume).unwrap();
        let col = ecs.get::<Collider>(ve).expect("volume collider");
        assert!(
            col.is_sensor,
            "Unity is_trigger must map to Collider.is_sensor"
        );

        for _ in 0..40 {
            sync_physics_from_unity(&mut runtime, &mut ecs);
            {
                let mut pw = ecs.remove_resource::<PhysicsWorld>().unwrap();
                pw.delta_time = 0.02;
                pw.step(&mut ecs);
                ecs.insert_resource(pw);
            }
            dispatch_unity_collision_enters_for_test(&mut runtime, &mut ecs);
            sync_physics_to_unity(&mut runtime, &ecs);
        }

        assert!(
            hits.load(Ordering::SeqCst) >= 1,
            "trigger enter should fire; hits={}",
            hits.load(Ordering::SeqCst)
        );
    }

    /// Phase 42 — multi-collider: all shapes enter bridge; priority is primary.
    #[test]
    fn test_multi_collider_bridge_primary_and_secondary() {
        use crate::body::BodyType;
        use crate::collider::ColliderShape;
        use engine_core::components::{BoxCollider, Rigidbody as UnityRb, SphereCollider};

        let mut runtime = SceneRuntime::new();
        let go = runtime.world.CreateGameObject("MultiCol");
        runtime.world.SetLocalPosition(go, Vec3::new(3.0, 4.0, 5.0));
        runtime.world.AddComponent(
            go,
            UnityRb {
                use_gravity: true,
                mass: 2.0,
                ..Default::default()
            },
        );
        // Box first in component list — priority still prefers Sphere.
        runtime.world.AddComponent(
            go,
            BoxCollider {
                center: Vec3::new(0.0, 1.0, 0.0),
                size: Vec3::new(2.0, 2.0, 2.0),
                is_trigger: false,
            },
        );
        runtime.world.AddComponent(
            go,
            SphereCollider {
                center: Vec3::ZERO,
                radius: 0.4,
                is_trigger: false,
            },
        );

        let mut ecs = EcsWorld::new();
        sync_physics_from_unity(&mut runtime, &mut ecs);

        let primary_e = runtime.entity_for(go).expect("primary entity");
        let primary_col = ecs.get::<Collider>(primary_e).expect("primary collider");
        assert!(
            matches!(primary_col.shape, ColliderShape::Sphere { .. }),
            "priority primary must be Sphere; got {:?}",
            primary_col.shape
        );
        let primary_body = ecs.get::<RigidBody>(primary_e).expect("primary body");
        assert_eq!(primary_body.body_type, BodyType::Dynamic);
        assert!((primary_body.mass - 2.0).abs() < 1e-4);

        let mut secondaries = Vec::new();
        for &idx in &ecs.component_entities::<SecondaryCollider>() {
            let tag = ecs.get_by_index::<SecondaryCollider>(idx).unwrap().clone();
            if tag.parent == go {
                let e = ecs.entity_from_index(idx).expect("entity handle");
                secondaries.push((tag.slot, e));
            }
        }
        assert_eq!(
            secondaries.len(),
            1,
            "exactly one secondary collider expected; got {secondaries:?}"
        );
        secondaries.sort_by_key(|(s, _)| *s);
        let (slot, sec_e) = secondaries[0];
        assert_eq!(slot, 0);
        let sec_col = ecs.get::<Collider>(sec_e).expect("secondary collider");
        assert!(
            matches!(sec_col.shape, ColliderShape::Box { .. }),
            "secondary should be Box; got {:?}",
            sec_col.shape
        );
        assert!((sec_col.offset - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-4);
        let sec_body = ecs.get::<RigidBody>(sec_e).expect("secondary body");
        assert_eq!(sec_body.body_type, BodyType::Kinematic);
        let sec_t = ecs
            .get::<CoreTransform>(sec_e)
            .expect("secondary transform");
        assert!(
            (sec_t.Position() - Vec3::new(3.0, 4.0, 5.0)).length() < 1e-3,
            "secondary transform must follow parent world pose; got {:?}",
            sec_t.Position()
        );

        // Slot reuse: second sync must not spawn another secondary entity.
        runtime
            .world
            .SetLocalPosition(go, Vec3::new(10.0, 0.0, 0.0));
        sync_physics_from_unity(&mut runtime, &mut ecs);
        let mut count = 0usize;
        let mut seen_e = None;
        for &idx in &ecs.component_entities::<SecondaryCollider>() {
            let tag = ecs.get_by_index::<SecondaryCollider>(idx).unwrap();
            if tag.parent == go {
                count += 1;
                seen_e = Some(ecs.entity_from_index(idx).unwrap());
            }
        }
        assert_eq!(count, 1, "secondary slot must be reused, not duplicated");
        let sec_t2 = ecs
            .get::<CoreTransform>(seen_e.expect("secondary"))
            .expect("secondary transform after follow");
        assert!(
            (sec_t2.Position() - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-3,
            "secondary must track parent; got {:?}",
            sec_t2.Position()
        );

        // Phase 42 critical: shrink — remove Box; secondary must despawn (no phantom).
        assert!(
            runtime.world.RemoveComponent::<BoxCollider>(go),
            "BoxCollider should be present before shrink"
        );
        sync_physics_from_unity(&mut runtime, &mut ecs);
        let mut stale_count = 0usize;
        for &idx in &ecs.component_entities::<SecondaryCollider>() {
            let tag = ecs.get_by_index::<SecondaryCollider>(idx).unwrap();
            if tag.parent == go {
                stale_count += 1;
            }
        }
        assert_eq!(
            stale_count, 0,
            "secondary entities must despawn when colliders shrink"
        );
        // Primary remains Sphere.
        let pc = ecs
            .get::<Collider>(primary_e)
            .expect("primary after shrink");
        assert!(matches!(pc.shape, ColliderShape::Sphere { .. }));
    }

    /// Phase 43 — secondary trigger events must map to parent GameObject MB.
    #[test]
    fn test_secondary_trigger_maps_to_parent_monobehaviour() {
        use crate::plugin::dispatch_unity_collision_enters_for_test;
        use engine_core::events::TriggerData;
        use engine_core::time::Time;
        use std::any::Any;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        struct ParentHits {
            go: Option<GameObjectHandle>,
            hits: Arc<AtomicU32>,
        }
        impl engine_core::component::Component for ParentHits {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl engine_core::behaviour::Behaviour for ParentHits {
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
        impl engine_core::monobehaviour::MonoBehaviour for ParentHits {
            fn OnTriggerEnter(
                &mut self,
                _ctx: &mut engine_core::context::Context,
                _t: &TriggerData,
            ) {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut runtime = SceneRuntime::new();
        // Multi: primary small non-trigger Sphere; secondary large trigger Box.
        let multi = runtime.world.CreateGameObject("Multi");
        runtime
            .world
            .SetLocalPosition(multi, Vec3::new(0.0, 0.0, 0.0));
        runtime.world.AddComponent(
            multi,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            multi,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.2,
                is_trigger: false,
            },
        );
        runtime.world.AddComponent(
            multi,
            engine_core::components::BoxCollider {
                center: Vec3::ZERO,
                size: Vec3::new(2.0, 2.0, 2.0),
                is_trigger: true,
            },
        );

        // Probe overlaps Box half-extent 1.0 but not primary sphere (0.2+0.3 < 0.8).
        let probe = runtime.world.CreateGameObject("Probe");
        runtime
            .world
            .SetLocalPosition(probe, Vec3::new(0.8, 0.0, 0.0));
        runtime.world.AddComponent(
            probe,
            UnityRb {
                use_gravity: false,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            probe,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.3,
                is_trigger: false,
            },
        );

        let parent_hits = Arc::new(AtomicU32::new(0));
        let probe_hits = Arc::new(AtomicU32::new(0));
        runtime.world.AddMonoBehaviour(
            multi,
            ParentHits {
                go: None,
                hits: parent_hits.clone(),
            },
        );
        runtime.world.AddMonoBehaviour(
            probe,
            ParentHits {
                go: None,
                hits: probe_hits.clone(),
            },
        );

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        ecs.insert_resource(Time::default());
        sync_physics_from_unity(&mut runtime, &mut ecs);

        let mut found_sec = false;
        for &idx in &ecs.component_entities::<SecondaryCollider>() {
            let tag = ecs.get_by_index::<SecondaryCollider>(idx).unwrap();
            if tag.parent == multi {
                found_sec = true;
                let e = ecs.entity_from_index(idx).unwrap();
                let c = ecs.get::<Collider>(e).expect("sec col");
                assert!(c.is_sensor, "box secondary must be sensor/trigger");
            }
        }
        assert!(found_sec, "secondary box must exist on Multi");

        for _ in 0..20 {
            sync_physics_from_unity(&mut runtime, &mut ecs);
            {
                let mut pw = ecs.remove_resource::<PhysicsWorld>().unwrap();
                pw.delta_time = 0.02;
                pw.step(&mut ecs);
                ecs.insert_resource(pw);
            }
            dispatch_unity_collision_enters_for_test(&mut runtime, &mut ecs);
            sync_physics_to_unity(&mut runtime, &ecs);
        }

        assert!(
            parent_hits.load(Ordering::SeqCst) >= 1,
            "parent Multi must receive OnTriggerEnter from secondary box; parent_hits={}",
            parent_hits.load(Ordering::SeqCst)
        );
        assert!(
            probe_hits.load(Ordering::SeqCst) >= 1,
            "probe must also receive trigger enter; probe_hits={}",
            probe_hits.load(Ordering::SeqCst)
        );
    }

    /// Phase 44 — same-GO primary↔secondary overlap must not self-dispatch.
    #[test]
    fn test_same_go_collider_pair_does_not_self_dispatch() {
        use crate::plugin::dispatch_unity_collision_enters_for_test;
        use engine_core::events::{Collision, TriggerData};
        use engine_core::time::Time;
        use std::any::Any;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

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
            fn OnTriggerEnter(
                &mut self,
                _ctx: &mut engine_core::context::Context,
                _t: &TriggerData,
            ) {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut runtime = SceneRuntime::new();
        // Same GO: primary sphere + secondary box fully overlapping at origin.
        let multi = runtime.world.CreateGameObject("SelfOverlap");
        runtime
            .world
            .SetLocalPosition(multi, Vec3::new(0.0, 0.0, 0.0));
        runtime.world.AddComponent(
            multi,
            UnityRb {
                use_gravity: false,
                is_kinematic: true,
                mass: 1.0,
                ..Default::default()
            },
        );
        runtime.world.AddComponent(
            multi,
            engine_core::components::SphereCollider {
                center: Vec3::ZERO,
                radius: 0.5,
                is_trigger: false,
            },
        );
        runtime.world.AddComponent(
            multi,
            engine_core::components::BoxCollider {
                center: Vec3::ZERO,
                size: Vec3::new(2.0, 2.0, 2.0),
                is_trigger: true,
            },
        );

        let hits = Arc::new(AtomicU32::new(0));
        runtime.world.AddMonoBehaviour(
            multi,
            HitCounter {
                go: None,
                hits: hits.clone(),
            },
        );

        let mut ecs = EcsWorld::new();
        ecs.insert_resource(PhysicsWorld::default());
        ecs.insert_resource(Time::default());

        for _ in 0..20 {
            sync_physics_from_unity(&mut runtime, &mut ecs);
            {
                let mut pw = ecs.remove_resource::<PhysicsWorld>().unwrap();
                pw.delta_time = 0.02;
                pw.step(&mut ecs);
                ecs.insert_resource(pw);
            }
            dispatch_unity_collision_enters_for_test(&mut runtime, &mut ecs);
            sync_physics_to_unity(&mut runtime, &ecs);
        }

        assert_eq!(
            hits.load(Ordering::SeqCst),
            0,
            "same-GO primary↔secondary must not dispatch self-callbacks; hits={}",
            hits.load(Ordering::SeqCst)
        );
    }
}
