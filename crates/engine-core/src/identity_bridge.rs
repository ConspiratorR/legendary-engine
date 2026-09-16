//! Identity bridge: `GameObjectHandle` ↔ ECS `Entity` mapping.
//!
//! # Why
//! The engine currently has two identities for “the same object”:
//! - **Unity World** (`GameObjectHandle`) — editor hierarchy, MonoBehaviours, scenes
//! - **ECS World** (`Entity`) — render/physics systems
//!
//! This module keeps a bidirectional map so both systems can refer to one
//! object without merging storage (see roadmap P2).
//!
//! ```text
//! GameObjectHandle  ←→  IdentityBridge  ←→  Entity
//!         │                                        │
//!   SceneRuntime / editor                    render / physics
//! ```

use std::collections::HashMap;

use engine_ecs::entity::Entity;
use engine_ecs::world::World as EcsWorld;

use crate::components::{Material, SpriteRenderer};
use crate::gameobject::GameObjectHandle;
use crate::world::World as UnityWorld;

/// ECS proxy of a Unity object's transform (updated by [`IdentityBridge::sync_transforms`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformProxy {
    pub position: engine_math::Vec3,
    pub rotation: engine_math::Quat,
    pub scale: engine_math::Vec3,
}

impl Default for TransformProxy {
    fn default() -> Self {
        Self {
            position: engine_math::Vec3::ZERO,
            rotation: engine_math::Quat::IDENTITY,
            scale: engine_math::Vec3::ONE,
        }
    }
}

/// ECS proxy of Unity render-related components for the render phase.
#[derive(Debug, Clone, Default)]
pub struct RenderProxy {
    /// Base color from Unity `Material`.
    pub color: [f32; 4],
    /// Texture/path from Unity `SpriteRenderer` (empty if none).
    pub sprite: String,
    pub flip_x: bool,
    pub flip_y: bool,
    pub sorting_order: i32,
    /// Whether this entity has any visual (material or sprite).
    pub visible: bool,
}

/// Bidirectional GameObject ↔ Entity map, stored as an ECS resource.
#[derive(Debug, Default)]
pub struct IdentityBridge {
    go_to_entity: HashMap<GameObjectHandle, Entity>,
    entity_to_go: HashMap<Entity, GameObjectHandle>,
}

impl IdentityBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Link an existing GameObject and Entity.
    pub fn link(&mut self, go: GameObjectHandle, entity: Entity) {
        // Drop stale reverse entries if overwriting
        if let Some(old_e) = self.go_to_entity.insert(go, entity) {
            if old_e != entity {
                self.entity_to_go.remove(&old_e);
            }
        }
        if let Some(old_go) = self.entity_to_go.insert(entity, go) {
            if old_go != go {
                self.go_to_entity.remove(&old_go);
            }
        }
    }

    /// Unlink by GameObject (e.g. on destroy).
    pub fn unlink_go(&mut self, go: GameObjectHandle) -> Option<Entity> {
        let e = self.go_to_entity.remove(&go)?;
        self.entity_to_go.remove(&e);
        Some(e)
    }

    /// Unlink by Entity.
    pub fn unlink_entity(&mut self, entity: Entity) -> Option<GameObjectHandle> {
        let go = self.entity_to_go.remove(&entity)?;
        self.go_to_entity.remove(&go);
        Some(go)
    }

    pub fn entity_for(&self, go: GameObjectHandle) -> Option<Entity> {
        self.go_to_entity.get(&go).copied()
    }

    pub fn gameobject_for(&self, entity: Entity) -> Option<GameObjectHandle> {
        self.entity_to_go.get(&entity).copied()
    }

    pub fn is_linked(&self, go: GameObjectHandle) -> bool {
        self.go_to_entity.contains_key(&go)
    }

    pub fn len(&self) -> usize {
        self.go_to_entity.len()
    }

    pub fn is_empty(&self) -> bool {
        self.go_to_entity.is_empty()
    }

    /// Ensure `go` has an ECS entity; spawn one if missing.
    pub fn ensure_entity(&mut self, go: GameObjectHandle, ecs: &mut EcsWorld) -> Entity {
        if let Some(e) = self.entity_for(go) {
            return e;
        }
        let e = ecs.spawn();
        self.link(go, e);
        e
    }

    /// Remove links whose GameObject is no longer valid.
    pub fn prune_invalid(&mut self, unity: &UnityWorld) {
        let stale: Vec<GameObjectHandle> = self
            .go_to_entity
            .keys()
            .copied()
            .filter(|&h| !unity.is_valid(h))
            .collect();
        for h in stale {
            self.unlink_go(h);
        }
    }

    /// Link every valid GameObject in the Unity hierarchy that is not yet mapped.
    ///
    /// Walks roots and all descendants so scene loads / editor trees become
    /// bridge-visible without calling `spawn_linked` per object.
    pub fn ensure_all_linked(&mut self, unity: &UnityWorld, ecs: &mut EcsWorld) {
        let mut stack: Vec<GameObjectHandle> = unity.GetRootGameObjects();
        while let Some(go) = stack.pop() {
            if !unity.is_valid(go) {
                continue;
            }
            if !self.is_linked(go) {
                self.ensure_entity(go, ecs);
            }
            stack.extend(unity.GetChildren(go));
        }
    }

    /// Copy Unity Transform into ECS `TransformProxy` for each linked pair.
    pub fn sync_transforms(&self, unity: &UnityWorld, ecs: &mut EcsWorld) {
        for (&go, &entity) in &self.go_to_entity {
            if !unity.is_valid(go) {
                continue;
            }
            let Some(t) = unity.GetTransform(go) else {
                continue;
            };
            let proxy = TransformProxy {
                position: t.Position(),
                rotation: t.Rotation(),
                scale: t.LossyScale(),
            };
            // add or update
            if ecs.get::<TransformProxy>(entity).is_some() {
                *ecs.get_mut::<TransformProxy>(entity).unwrap() = proxy;
            } else {
                ecs.add_component(entity, proxy);
            }
        }
    }

    /// Copy Unity Material / SpriteRenderer into ECS `RenderProxy`.
    pub fn sync_render_proxies(&self, unity: &UnityWorld, ecs: &mut EcsWorld) {
        for (&go, &entity) in &self.go_to_entity {
            if !unity.is_valid(go) {
                continue;
            }

            let mut proxy = RenderProxy::default();
            if let Some(mat) = unity.GetComponent::<Material>(go) {
                proxy.color = mat.base_color;
                proxy.visible = true;
            }
            if let Some(sr) = unity.GetComponent::<SpriteRenderer>(go) {
                proxy.sprite = sr.sprite.clone();
                proxy.color = sr.color;
                proxy.flip_x = sr.flip_x;
                proxy.flip_y = sr.flip_y;
                proxy.sorting_order = sr.sorting_order;
                proxy.visible = true;
            }

            if ecs.get::<RenderProxy>(entity).is_some() {
                *ecs.get_mut::<RenderProxy>(entity).unwrap() = proxy;
            } else {
                ecs.add_component(entity, proxy);
            }
        }
    }

    /// Run all sync steps (prune + auto-link + transforms + render proxies).
    pub fn sync_all(&mut self, unity: &UnityWorld, ecs: &mut EcsWorld) {
        self.prune_invalid(unity);
        self.ensure_all_linked(unity, ecs);
        self.sync_transforms(unity, ecs);
        self.sync_render_proxies(unity, ecs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_and_lookup() {
        let mut bridge = IdentityBridge::new();
        let mut ecs = EcsWorld::new();
        let e = ecs.spawn();
        let go = GameObjectHandle::new(0, 0);
        bridge.link(go, e);
        assert_eq!(bridge.entity_for(go), Some(e));
        assert_eq!(bridge.gameobject_for(e), Some(go));
        assert_eq!(bridge.len(), 1);
    }

    #[test]
    fn test_ensure_entity_spawns_once() {
        let mut bridge = IdentityBridge::new();
        let mut ecs = EcsWorld::new();
        let go = GameObjectHandle::new(1, 0);
        let e1 = bridge.ensure_entity(go, &mut ecs);
        let e2 = bridge.ensure_entity(go, &mut ecs);
        assert_eq!(e1, e2);
        assert_eq!(bridge.len(), 1);
    }

    #[test]
    fn test_unlink() {
        let mut bridge = IdentityBridge::new();
        let mut ecs = EcsWorld::new();
        let e = ecs.spawn();
        let go = GameObjectHandle::new(2, 0);
        bridge.link(go, e);
        assert_eq!(bridge.unlink_go(go), Some(e));
        assert!(bridge.is_empty());
    }

    #[test]
    fn test_sync_transforms() {
        let mut unity = UnityWorld::new();
        let go = unity.CreateGameObject("Obj");
        if let Some(t) = unity.GetTransformMut(go) {
            t.SetLocalPosition(engine_math::Vec3::new(1.0, 2.0, 3.0));
        }
        unity.sync_transforms();

        let mut ecs = EcsWorld::new();
        let mut bridge = IdentityBridge::new();
        let e = bridge.ensure_entity(go, &mut ecs);
        bridge.sync_transforms(&unity, &mut ecs);

        let proxy = ecs.get::<TransformProxy>(e).unwrap();
        assert_eq!(proxy.position, engine_math::Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_sync_render_proxy_from_material() {
        let mut unity = UnityWorld::new();
        let go = unity.CreateGameObject("Mat");
        unity.AddComponent(go, Material::new_with_color([1.0, 0.0, 0.0, 1.0]));

        let mut ecs = EcsWorld::new();
        let mut bridge = IdentityBridge::new();
        let e = bridge.ensure_entity(go, &mut ecs);
        bridge.sync_render_proxies(&unity, &mut ecs);

        let proxy = ecs.get::<RenderProxy>(e).unwrap();
        assert!(proxy.visible);
        assert_eq!(proxy.color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_prune_invalid() {
        let mut unity = UnityWorld::new();
        let go = unity.CreateGameObject("Temp");
        let mut ecs = EcsWorld::new();
        let mut bridge = IdentityBridge::new();
        bridge.ensure_entity(go, &mut ecs);
        unity.DestroyImmediate(go);
        bridge.prune_invalid(&unity);
        assert!(bridge.is_empty());
    }

    #[test]
    fn test_ensure_all_linked_walks_hierarchy() {
        let mut unity = UnityWorld::new();
        let mut ecs = EcsWorld::new();
        let mut bridge = IdentityBridge::new();

        let root = unity.CreateGameObject("Root");
        let child = unity.CreateGameObject("Child");
        unity.SetParent(child, Some(root));
        let orphan = unity.CreateGameObject("Orphan");

        bridge.ensure_all_linked(&unity, &mut ecs);
        assert_eq!(bridge.len(), 3);
        assert!(bridge.entity_for(root).is_some());
        assert!(bridge.entity_for(child).is_some());
        assert!(bridge.entity_for(orphan).is_some());

        bridge.ensure_all_linked(&unity, &mut ecs);
        assert_eq!(bridge.len(), 3);
    }

    #[test]
    fn test_sync_all_auto_links_unlinked() {
        let mut unity = UnityWorld::new();
        let mut ecs = EcsWorld::new();
        let mut bridge = IdentityBridge::new();

        let go = unity.CreateGameObject("Auto");
        assert!(bridge.is_empty());
        bridge.sync_all(&unity, &mut ecs);
        assert!(bridge.is_linked(go));
        assert!(
            ecs.get::<TransformProxy>(bridge.entity_for(go).unwrap())
                .is_some()
        );
    }
}
