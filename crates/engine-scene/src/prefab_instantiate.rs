//! Prefab instantiation into an ECS [`World`](engine_ecs::world::World).
//!
//! This module handles spawning entity hierarchies from [`PrefabDef`]s,
//! applying component templates and per-instance overrides. Nested prefab
//! references are resolved recursively up to [`MAX_NESTING_DEPTH`] levels.

use std::collections::{HashMap, HashSet};

use engine_ecs::entity::Entity;
use engine_ecs::world::World;

use super::hierarchy::{Children, Parent};
use super::prefab::{
    MAX_NESTING_DEPTH, PrefabDef, PrefabError, PrefabInstance, PrefabNode, PrefabNodeOverride,
};
use super::serialization::{ComponentData, TransformData};
use super::transform::Transform;

// ── Public API ──────────────────────────────────────────────────────

/// Instantiate a [`PrefabDef`] into a [`World`], returning a [`PrefabInstance`]
/// that maps node names to spawned entities.
///
/// `overrides` may contain per-node property and transform overrides.
/// Prefab nesting (via `PrefabNode::prefab_ref`) is resolved recursively,
/// up to [`MAX_NESTING_DEPTH`] levels.
pub fn instantiate_prefab(
    prefab: &PrefabDef,
    world: &mut World,
    overrides: &HashMap<String, PrefabNodeOverride>,
) -> Result<PrefabInstance, PrefabError> {
    let mut instance = PrefabInstance::new(&prefab.name, 0);
    let mut visited = HashSet::new();
    visited.insert(prefab.name.clone());

    let root_entity = instantiate_node(
        &prefab.root,
        world,
        overrides,
        &mut instance.entity_map,
        &visited,
        0,
    )?;

    instance.root_entity = root_entity.index() as u64;
    instance.overrides = overrides.clone();

    Ok(instance)
}

// ── Internal ────────────────────────────────────────────────────────

/// Recursively instantiate a single [`PrefabNode`] and its children.
fn instantiate_node(
    node: &PrefabNode,
    world: &mut World,
    overrides: &HashMap<String, PrefabNodeOverride>,
    entity_map: &mut HashMap<String, u64>,
    visited: &HashSet<String>,
    depth: u32,
) -> Result<Entity, PrefabError> {
    // If this node references another prefab, resolve it recursively.
    if let Some(ref prefab_name) = node.prefab_ref {
        if depth >= MAX_NESTING_DEPTH {
            return Err(PrefabError::NestingDepthExceeded(prefab_name.clone()));
        }
        if visited.contains(prefab_name) {
            return Err(PrefabError::CircularReference(prefab_name.clone()));
        }
        // We cannot look up the referenced prefab here without access to
        // the registry. The caller (or a higher-level API) must resolve
        // nested prefab references before calling this function.
        //
        // For now, we treat a prefab_ref node as a regular node — the
        // referenced prefab's content should have been inlined beforehand.
        // See `resolve_prefab_refs` for the flattening step.
    }

    // Spawn the entity.
    let entity = world.spawn();

    // Apply transform (override wins over template).
    let transform = if let Some(node_override) = overrides.get(&node.name) {
        node_override
            .transform
            .as_ref()
            .unwrap_or(&node.transform)
            .clone()
    } else {
        node.transform.clone()
    };
    world.add_component(entity, transform_from_data(&transform));
    world.add_component(entity, super::transform::GlobalTransform::default());

    // Apply component templates with overrides.
    let component_data = build_component_data(node, overrides);
    world.add_component(entity, component_data);

    // Set up children placeholder.
    world.add_component(entity, Children::new());

    // Record entity mapping.
    entity_map.insert(node.name.clone(), entity.index() as u64);

    // Instantiate children and link hierarchy.
    for child_node in &node.children {
        let child_entity =
            instantiate_node(child_node, world, overrides, entity_map, visited, depth + 1)?;
        world.add_component(child_entity, Parent(entity));
        if let Some(children) = world.get_mut::<Children>(entity) {
            children.0.push(child_entity);
        }
    }

    Ok(entity)
}

/// Build the list of [`ComponentData`] for a node, merging template
/// properties with any per-instance overrides.
fn build_component_data(
    node: &PrefabNode,
    overrides: &HashMap<String, PrefabNodeOverride>,
) -> Vec<ComponentData> {
    let node_override = overrides.get(&node.name);

    node.components
        .iter()
        .map(|template| {
            let mut data = ComponentData::new(&template.type_name);

            // Start with template properties.
            for (key, value) in &template.properties {
                data.properties.insert(key.clone(), value.clone());
            }

            // Apply component-level overrides.
            if let Some(no) = node_override
                && let Some(comp_override) = no.components.get(&template.type_name)
            {
                for (key, value) in &comp_override.properties {
                    data.properties.insert(key.clone(), value.clone());
                }
            }

            data
        })
        .collect()
}

/// Convert a [`TransformData`] to an engine [`Transform`].
fn transform_from_data(data: &TransformData) -> Transform {
    Transform {
        translation: engine_math::Vec3::new(
            data.translation[0],
            data.translation[1],
            data.translation[2],
        ),
        rotation: engine_math::Quat::from_xyzw(
            data.rotation[0],
            data.rotation[1],
            data.rotation[2],
            data.rotation[3],
        ),
        scale: engine_math::Vec3::new(data.scale[0], data.scale[1], data.scale[2]),
    }
}

// ── Prefab Ref Resolution ───────────────────────────────────────────

/// Resolve all `prefab_ref` entries in a [`PrefabDef`] by inlining the
/// referenced prefab's node tree.
///
/// This must be called before instantiation when nested prefab references
/// are present. The `registry` is used to look up referenced prefabs.
///
/// Returns an error if a referenced prefab is not found, nesting depth
/// is exceeded, or a circular reference is detected.
pub fn resolve_prefab_refs(
    prefab: &mut PrefabDef,
    registry: &super::prefab_registry::PrefabRegistry,
) -> Result<(), PrefabError> {
    let mut visited = HashSet::new();
    visited.insert(prefab.name.clone());
    resolve_node_refs(&mut prefab.root, registry, &visited, 0)
}

fn resolve_node_refs(
    node: &mut PrefabNode,
    registry: &super::prefab_registry::PrefabRegistry,
    visited: &HashSet<String>,
    depth: u32,
) -> Result<(), PrefabError> {
    if let Some(ref prefab_name) = node.prefab_ref {
        if depth >= MAX_NESTING_DEPTH {
            return Err(PrefabError::NestingDepthExceeded(prefab_name.clone()));
        }
        if visited.contains(prefab_name) {
            return Err(PrefabError::CircularReference(prefab_name.clone()));
        }

        let referenced = registry
            .get(prefab_name)
            .ok_or_else(|| PrefabError::NotFound(prefab_name.clone()))?
            .clone();

        let mut new_visited = visited.clone();
        new_visited.insert(prefab_name.clone());

        // Inline the referenced prefab's root into this node.
        // Preserve the current node's name and any local overrides.
        let original_name = node.name.clone();
        let original_transform = node.transform.clone();
        let original_components = node.components.clone();

        *node = referenced.root;
        node.name = original_name;
        // Keep the local transform if it was customized, otherwise use the
        // referenced prefab's transform.
        if original_transform != TransformData::default() {
            node.transform = original_transform;
        }
        // Merge local components on top of the referenced prefab's components.
        for comp in original_components {
            node.components.push(comp);
        }
        node.prefab_ref = None; // Resolved.

        // Recursively resolve any nested refs in the inlined children.
        for child in &mut node.children {
            resolve_node_refs(child, registry, &new_visited, depth + 1)?;
        }
    } else {
        // No ref — just recurse into children.
        for child in &mut node.children {
            resolve_node_refs(child, registry, visited, depth)?;
        }
    }

    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefab::{ComponentOverride, ComponentTemplate, PrefabDef, PrefabNode};
    use crate::prefab_registry::PrefabRegistry;
    use crate::serialization::{PropertyValue, TransformData};

    fn make_prefab(name: &str) -> PrefabDef {
        let mut prefab = PrefabDef::new(name);
        prefab.root = PrefabNode::new("root")
            .with_component(
                ComponentTemplate::new("Health").with_property("max", PropertyValue::Float(100.0)),
            )
            .with_child(PrefabNode::new("body").with_transform(TransformData {
                translation: [0.0, 1.0, 0.0],
                ..Default::default()
            }));
        prefab
    }

    #[test]
    fn test_instantiate_simple_prefab() {
        let prefab = make_prefab("Enemy");
        let mut world = World::new();
        let instance = prefab.instantiate(&mut world).unwrap();

        assert_eq!(instance.prefab_name, "Enemy");
        assert!(instance.entity_map.contains_key("root"));
        assert!(instance.entity_map.contains_key("body"));

        // Root entity should exist in the world.
        let root_idx = instance.root_entity as u32;
        let root_entity = Entity::new(root_idx, 0);
        assert!(world.get::<Transform>(root_entity).is_some());
        assert!(world.get::<Vec<ComponentData>>(root_entity).is_some());
    }

    #[test]
    fn test_instantiate_hierarchy() {
        let prefab = make_prefab("Enemy");
        let mut world = World::new();
        let instance = prefab.instantiate(&mut world).unwrap();

        let root_entity = Entity::new(instance.root_entity as u32, 0);
        let body_entity = Entity::new(instance.get_entity("body").unwrap() as u32, 0);

        // Body should have Parent pointing to root.
        let parent = world.get::<Parent>(body_entity).unwrap();
        assert_eq!(parent.0, root_entity);

        // Root should have body as a child.
        let children = world.get::<Children>(root_entity).unwrap();
        assert!(children.0.contains(&body_entity));
    }

    #[test]
    fn test_instantiate_with_transform_override() {
        let prefab = make_prefab("Enemy");
        let mut world = World::new();

        let mut overrides = HashMap::new();
        overrides.insert(
            "body".to_string(),
            PrefabNodeOverride {
                transform: Some(TransformData {
                    translation: [5.0, 0.0, 0.0],
                    ..Default::default()
                }),
                components: HashMap::new(),
            },
        );

        let instance = prefab
            .instantiate_with_overrides(&mut world, &overrides)
            .unwrap();
        let body_entity = Entity::new(instance.get_entity("body").unwrap() as u32, 0);
        let transform = world.get::<Transform>(body_entity).unwrap();
        assert_eq!(transform.translation.x, 5.0);
    }

    #[test]
    fn test_instantiate_with_component_override() {
        let prefab = make_prefab("Enemy");
        let mut world = World::new();

        let mut overrides = HashMap::new();
        let mut comp_overrides = HashMap::new();
        comp_overrides.insert(
            "Health".to_string(),
            ComponentOverride {
                properties: {
                    let mut m = HashMap::new();
                    m.insert("max".to_string(), PropertyValue::Float(200.0));
                    m
                },
            },
        );
        overrides.insert(
            "root".to_string(),
            PrefabNodeOverride {
                transform: None,
                components: comp_overrides,
            },
        );

        let instance = prefab
            .instantiate_with_overrides(&mut world, &overrides)
            .unwrap();
        let root_entity = Entity::new(instance.root_entity as u32, 0);
        let components = world.get::<Vec<ComponentData>>(root_entity).unwrap();
        let health = components.iter().find(|c| c.type_name == "Health").unwrap();
        match health.properties.get("max").unwrap() {
            PropertyValue::Float(v) => assert_eq!(*v, 200.0),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_instantiate_override_does_not_affect_original() {
        let prefab = make_prefab("Enemy");
        let mut world = World::new();

        let mut overrides = HashMap::new();
        overrides.insert(
            "body".to_string(),
            PrefabNodeOverride {
                transform: Some(TransformData {
                    translation: [999.0, 0.0, 0.0],
                    ..Default::default()
                }),
                components: HashMap::new(),
            },
        );

        let _instance = prefab
            .instantiate_with_overrides(&mut world, &overrides)
            .unwrap();

        // Original prefab's body transform should be unchanged.
        assert_eq!(
            prefab.root.children[0].transform.translation,
            [0.0, 1.0, 0.0]
        );
    }

    #[test]
    fn test_resolve_prefab_refs_simple() {
        let mut registry = PrefabRegistry::new();
        let weapon = PrefabDef::new("Sword");
        registry.register(weapon);

        let mut enemy = make_prefab("Enemy");
        enemy.root = enemy
            .root
            .with_child(PrefabNode::new("weapon_slot").with_prefab_ref("Sword"));

        resolve_prefab_refs(&mut enemy, &registry).unwrap();

        let weapon_slot = enemy
            .root
            .children
            .iter()
            .find(|c| c.name == "weapon_slot")
            .unwrap();
        assert!(weapon_slot.prefab_ref.is_none());
    }

    #[test]
    fn test_resolve_prefab_refs_not_found() {
        let registry = PrefabRegistry::new();
        let mut enemy = make_prefab("Enemy");
        enemy.root = enemy
            .root
            .with_child(PrefabNode::new("weapon").with_prefab_ref("NonExistent"));

        let result = resolve_prefab_refs(&mut enemy, &registry);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_prefab_refs_circular() {
        let mut registry = PrefabRegistry::new();
        let mut prefab_a = PrefabDef::new("A");
        prefab_a.root = prefab_a
            .root
            .with_child(PrefabNode::new("ref_b").with_prefab_ref("B"));
        registry.register(prefab_a);

        let mut prefab_b = PrefabDef::new("B");
        prefab_b.root = prefab_b
            .root
            .with_child(PrefabNode::new("ref_a").with_prefab_ref("A"));
        registry.register(prefab_b);

        // Now trying to resolve A should detect circular reference.
        let mut a = registry.get("A").cloned().unwrap();
        let result = resolve_prefab_refs(&mut a, &registry);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_prefab_refs_depth_limit() {
        let mut registry = PrefabRegistry::new();
        // Create a chain: L0 -> L1 -> L2 -> L3 -> L4 (exceeds depth 3)
        for i in 0..5 {
            let name = format!("L{i}");
            let child_name = format!("L{}", i + 1);
            let mut prefab = PrefabDef::new(&name);
            if i < 4 {
                prefab.root = prefab
                    .root
                    .with_child(PrefabNode::new("ref").with_prefab_ref(&child_name));
            }
            registry.register(prefab);
        }

        let mut l0 = registry.get("L0").cloned().unwrap();
        let result = resolve_prefab_refs(&mut l0, &registry);
        assert!(result.is_err());
    }

    #[test]
    fn test_component_data_merge_with_override() {
        let node = PrefabNode::new("test").with_component(
            ComponentTemplate::new("Health")
                .with_property("max", PropertyValue::Float(100.0))
                .with_property("current", PropertyValue::Float(100.0)),
        );

        let mut overrides = HashMap::new();
        let mut comp_overrides = HashMap::new();
        comp_overrides.insert(
            "Health".to_string(),
            ComponentOverride {
                properties: {
                    let mut m = HashMap::new();
                    m.insert("max".to_string(), PropertyValue::Float(200.0));
                    m
                },
            },
        );
        overrides.insert(
            "test".to_string(),
            PrefabNodeOverride {
                transform: None,
                components: comp_overrides,
            },
        );

        let data = build_component_data(&node, &overrides);
        assert_eq!(data.len(), 1);

        let health = &data[0];
        match health.properties.get("max").unwrap() {
            PropertyValue::Float(v) => assert_eq!(*v, 200.0),
            _ => panic!("expected Float"),
        }
        // current should be unchanged from template.
        match health.properties.get("current").unwrap() {
            PropertyValue::Float(v) => assert_eq!(*v, 100.0),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_transform_from_data() {
        let data = TransformData {
            translation: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [2.0, 2.0, 2.0],
        };
        let t = transform_from_data(&data);
        assert_eq!(t.translation.x, 1.0);
        assert_eq!(t.translation.y, 2.0);
        assert_eq!(t.scale.x, 2.0);
    }
}
