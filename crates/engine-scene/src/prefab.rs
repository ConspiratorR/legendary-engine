//! Prefab system for reusable entity templates.
//!
//! A [`PrefabDef`] defines a tree of [`PrefabNode`]s, each carrying a name,
//! local transform, and component templates. Prefabs can be instantiated into
//! an ECS [`World`](engine_ecs::world::World) via
//! [`instantiate`](PrefabDef::instantiate), producing a hierarchy of entities
//! with optional per-instance overrides.
//!
//! Prefabs may reference other prefabs by name (nesting), up to
//! [`MAX_NESTING_DEPTH`] levels deep.
//!
//! # Example
//!
//! ```rust
//! use engine_scene::prefab::{PrefabDef, PrefabNode, ComponentTemplate};
//! use engine_scene::serialization::PropertyValue;
//!
//! let mut prefab = PrefabDef::new("Enemy");
//! prefab.root.components.push(
//!     ComponentTemplate::new("Health")
//!         .with_property("max", PropertyValue::Float(100.0)),
//! );
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::serialization::{PropertyValue, TransformData};

// ── Constants ───────────────────────────────────────────────────────

/// Maximum nesting depth when one prefab references another.
pub const MAX_NESTING_DEPTH: u32 = 3;

// ── Error Type ──────────────────────────────────────────────────────

/// Errors that can occur during prefab operations.
#[derive(Error, Debug)]
pub enum PrefabError {
    #[error("prefab not found: '{0}'")]
    NotFound(String),

    #[error("nesting depth exceeded ({MAX_NESTING_DEPTH} levels) at prefab '{0}'")]
    NestingDepthExceeded(String),

    #[error("circular prefab reference detected: '{0}'")]
    CircularReference(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] crate::serialization::SceneError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RON error: {0}")]
    Ron(#[from] ron::error::Error),

    #[error("RON deserialization error: {0}")]
    RonDe(#[from] ron::error::SpannedError),

    #[error("bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Prefab Definition ───────────────────────────────────────────────

/// A reusable prefab template consisting of a tree of nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabDef {
    /// Unique prefab name (used as the lookup key).
    pub name: String,
    /// Root node of the prefab hierarchy.
    pub root: PrefabNode,
}

/// A single node within a prefab definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabNode {
    /// Human-readable node name.
    pub name: String,
    /// Local transform relative to the parent node.
    pub transform: TransformData,
    /// Component templates attached to this node.
    pub components: Vec<ComponentTemplate>,
    /// Child nodes.
    pub children: Vec<PrefabNode>,
    /// If set, this node is itself a reference to another prefab by name.
    pub prefab_ref: Option<String>,
}

/// A component template within a prefab node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTemplate {
    /// The component type name (e.g. `"MeshRenderer"`).
    pub type_name: String,
    /// Key-value properties.
    pub properties: HashMap<String, PropertyValue>,
}

// ── Prefab Instance ─────────────────────────────────────────────────

/// A live instance of a prefab in the ECS world.
///
/// Tracks which entities were spawned from which prefab nodes and stores
/// any per-instance property overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabInstance {
    /// The name of the source [`PrefabDef`].
    pub prefab_name: String,
    /// The root entity of the instantiated hierarchy.
    pub root_entity: u64,
    /// Maps `"node_name"` → entity index for every node in the prefab tree.
    pub entity_map: HashMap<String, u64>,
    /// Per-instance overrides keyed by node name.
    pub overrides: HashMap<String, PrefabNodeOverride>,
}

/// Overrides for a single node within a prefab instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabNodeOverride {
    /// Transform override (if [`Some`]).
    pub transform: Option<TransformData>,
    /// Per-component property overrides, keyed by component type name.
    pub components: HashMap<String, ComponentOverride>,
}

/// Override for a single component on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOverride {
    /// Property value overrides.
    pub properties: HashMap<String, PropertyValue>,
}

// ── Constructors ────────────────────────────────────────────────────

impl PrefabDef {
    /// Create a new prefab with a single root node.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            root: PrefabNode::new(&name),
            name,
        }
    }

    /// Instantiate this prefab into an ECS world.
    ///
    /// Delegates to [`super::prefab_instantiate::instantiate_prefab`].
    pub fn instantiate(
        &self,
        world: &mut engine_ecs::world::World,
    ) -> Result<PrefabInstance, PrefabError> {
        super::prefab_instantiate::instantiate_prefab(self, world, &HashMap::new())
    }

    /// Instantiate with per-instance overrides.
    pub fn instantiate_with_overrides(
        &self,
        world: &mut engine_ecs::world::World,
        overrides: &HashMap<String, PrefabNodeOverride>,
    ) -> Result<PrefabInstance, PrefabError> {
        super::prefab_instantiate::instantiate_prefab(self, world, overrides)
    }
}

impl PrefabNode {
    /// Create a new node with the given name and default transform.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: TransformData::default(),
            components: Vec::new(),
            children: Vec::new(),
            prefab_ref: None,
        }
    }

    /// Set the local transform of this node.
    pub fn with_transform(mut self, transform: TransformData) -> Self {
        self.transform = transform;
        self
    }

    /// Add a component template to this node.
    pub fn with_component(mut self, component: ComponentTemplate) -> Self {
        self.components.push(component);
        self
    }

    /// Add a child node.
    pub fn with_child(mut self, child: PrefabNode) -> Self {
        self.children.push(child);
        self
    }

    /// Mark this node as a reference to another prefab.
    pub fn with_prefab_ref(mut self, prefab_name: impl Into<String>) -> Self {
        self.prefab_ref = Some(prefab_name.into());
        self
    }
}

impl ComponentTemplate {
    /// Create a new component template.
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            properties: HashMap::new(),
        }
    }

    /// Add a property to this component template.
    pub fn with_property(mut self, key: impl Into<String>, value: PropertyValue) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
}

impl PrefabInstance {
    /// Create a new empty prefab instance.
    pub fn new(prefab_name: impl Into<String>, root_entity: u64) -> Self {
        Self {
            prefab_name: prefab_name.into(),
            root_entity,
            entity_map: HashMap::new(),
            overrides: HashMap::new(),
        }
    }

    /// Get the entity index for a given node name.
    pub fn get_entity(&self, node_name: &str) -> Option<u64> {
        self.entity_map.get(node_name).copied()
    }

    /// Apply a transform override to a node.
    pub fn set_transform_override(&mut self, node_name: &str, transform: TransformData) {
        let node_override = self
            .overrides
            .entry(node_name.to_string())
            .or_insert_with(PrefabNodeOverride::empty);
        node_override.transform = Some(transform);
    }

    /// Apply a component property override to a node.
    pub fn set_component_override(
        &mut self,
        node_name: &str,
        component_type: &str,
        property: &str,
        value: PropertyValue,
    ) {
        let node_override = self
            .overrides
            .entry(node_name.to_string())
            .or_insert_with(PrefabNodeOverride::empty);
        let comp_override = node_override
            .components
            .entry(component_type.to_string())
            .or_insert_with(ComponentOverride::empty);
        comp_override.properties.insert(property.to_string(), value);
    }
}

impl PrefabNodeOverride {
    /// Create an empty override (no changes).
    pub fn empty() -> Self {
        Self {
            transform: None,
            components: HashMap::new(),
        }
    }
}

impl ComponentOverride {
    /// Create an empty component override.
    pub fn empty() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefab_def_creation() {
        let prefab = PrefabDef::new("TestPrefab");
        assert_eq!(prefab.name, "TestPrefab");
        assert_eq!(prefab.root.name, "TestPrefab");
        assert!(prefab.root.components.is_empty());
        assert!(prefab.root.children.is_empty());
    }

    #[test]
    fn test_prefab_node_builder() {
        let node = PrefabNode::new("body")
            .with_transform(TransformData {
                translation: [1.0, 2.0, 3.0],
                ..Default::default()
            })
            .with_component(
                ComponentTemplate::new("MeshRenderer")
                    .with_property("mesh", PropertyValue::String("cube.obj".into())),
            )
            .with_child(PrefabNode::new("head"));

        assert_eq!(node.name, "body");
        assert_eq!(node.transform.translation, [1.0, 2.0, 3.0]);
        assert_eq!(node.components.len(), 1);
        assert_eq!(node.components[0].type_name, "MeshRenderer");
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].name, "head");
    }

    #[test]
    fn test_component_template_builder() {
        let comp = ComponentTemplate::new("Health")
            .with_property("max", PropertyValue::Float(100.0))
            .with_property("current", PropertyValue::Float(100.0));

        assert_eq!(comp.type_name, "Health");
        assert_eq!(comp.properties.len(), 2);
    }

    #[test]
    fn test_prefab_with_nested_prefab_ref() {
        let node = PrefabNode::new("weapon").with_prefab_ref("Sword");
        assert_eq!(node.prefab_ref, Some("Sword".to_string()));
    }

    #[test]
    fn test_prefab_instance_entity_map() {
        let mut instance = PrefabInstance::new("Enemy", 0);
        instance.entity_map.insert("root".to_string(), 0);
        instance.entity_map.insert("body".to_string(), 1);

        assert_eq!(instance.get_entity("root"), Some(0));
        assert_eq!(instance.get_entity("body"), Some(1));
        assert_eq!(instance.get_entity("missing"), None);
    }

    #[test]
    fn test_prefab_instance_overrides() {
        let mut instance = PrefabInstance::new("Enemy", 0);
        instance.set_transform_override(
            "body",
            TransformData {
                translation: [10.0, 0.0, 0.0],
                ..Default::default()
            },
        );
        instance.set_component_override("body", "Health", "max", PropertyValue::Float(200.0));

        let body_override = instance.overrides.get("body").unwrap();
        assert!(body_override.transform.is_some());
        assert_eq!(
            body_override.transform.as_ref().unwrap().translation,
            [10.0, 0.0, 0.0]
        );

        let health_override = body_override.components.get("Health").unwrap();
        match health_override.properties.get("max").unwrap() {
            PropertyValue::Float(v) => assert_eq!(*v, 200.0),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_max_nesting_depth_constant() {
        assert_eq!(MAX_NESTING_DEPTH, 3);
    }
}
