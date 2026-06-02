//! Scene diffing and incremental serialization.
//!
//! Compares two [`SceneData`] snapshots and produces a [`SceneDiff`] describing
//! only the changes. The diff can be serialized to disk and later applied to
//! reconstruct the modified scene, enabling incremental saves that are
//! significantly smaller than full scene files.
//!
//! # Example
//!
//! ```rust,no_run
//! use engine_scene::serialization::{SceneData, SceneFormat, save_scene, load_scene};
//! use engine_scene::diff::{diff_scenes, apply_diff, save_diff, load_and_apply_diff};
//!
//! let original = SceneData::new("level1");
//! // ... modify scene ...
//! let modified = SceneData::new("level1");
//!
//! let diff = diff_scenes(&original, &modified);
//! save_diff(&diff, "level1_diff.bin", SceneFormat::Bin).unwrap();
//!
//! // Later, load the diff and apply it to the original
//! let base = load_scene("level1.bin").unwrap();
//! let diff = load_and_apply_diff("level1_diff.bin", SceneFormat::Bin).unwrap();
//! let restored = apply_diff(&base, &diff).unwrap();
//! ```

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::serialization::{
    ComponentData, PropertyValue, SceneData, SceneEntityData, SceneError, SceneFormat,
    SceneSettings, TransformData,
};

// ── Error Type ──────────────────────────────────────────────────────

/// Errors that can occur during diff operations.
#[derive(Error, Debug)]
pub enum DiffError {
    #[error("scene serialization error: {0}")]
    Scene(#[from] SceneError),

    #[error("scene name mismatch: expected '{expected}', got '{actual}'")]
    NameMismatch { expected: String, actual: String },

    #[error("entity {0} not found in base scene")]
    EntityNotFound(u64),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RON error: {0}")]
    Ron(#[from] ron::error::SpannedError),

    #[error("BIN error: {0}")]
    Bincode(#[from] bincode::Error),
}

// ── Diff Data Structures ────────────────────────────────────────────

/// A complete diff between two scene snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDiff {
    /// Name of the scene being diffed.
    pub scene_name: String,
    /// Entity-level changes (added, removed, modified).
    pub entity_changes: Vec<EntityChange>,
    /// Changes to global scene settings, if any.
    pub settings_change: Option<SceneSettingsChange>,
}

/// A change to a single entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityChange {
    /// A new entity was added.
    Added(SceneEntityData),
    /// An existing entity was removed.
    Removed(u64),
    /// An existing entity was modified.
    Modified(EntityModification),
}

/// Detailed modification of an existing entity.
///
/// Each field is `Option` — `None` means no change to that field,
/// `Some(v)` means the field changed to the new value `v`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityModification {
    /// The entity ID.
    pub id: u64,
    /// New name, if changed.
    pub name: Option<String>,
    /// New transform, if changed.
    pub transform: Option<TransformData>,
    /// New component list, if changed.
    pub components: Option<Vec<ComponentData>>,
    /// New children list, if changed.
    pub children: Option<Vec<u64>>,
    /// New parent, if changed.
    pub parent: Option<Option<u64>>,
    /// New active state, if changed.
    pub active: Option<bool>,
}

/// Changes to global scene settings.
///
/// Only populated fields are included.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSettingsChange {
    pub ambient_color: Option<[f32; 4]>,
    pub fog_enabled: Option<bool>,
    pub fog_color: Option<[f32; 4]>,
    pub fog_near: Option<f32>,
    pub fog_far: Option<f32>,
}

// ── Diff Computation ────────────────────────────────────────────────

/// Compare two scene snapshots and produce a [`SceneDiff`].
///
/// Entities are matched by their `id` field. Entities present in `modified`
/// but not in `base` are marked as `Added`; those in `base` but not in
/// `modified` are `Removed`; and those in both are checked for modifications.
pub fn diff_scenes(base: &SceneData, modified: &SceneData) -> SceneDiff {
    let base_map: HashMap<u64, &SceneEntityData> =
        base.entities.iter().map(|e| (e.id, e)).collect();
    let mod_map: HashMap<u64, &SceneEntityData> =
        modified.entities.iter().map(|e| (e.id, e)).collect();

    let base_ids: HashSet<u64> = base_map.keys().copied().collect();
    let mod_ids: HashSet<u64> = mod_map.keys().copied().collect();

    let mut entity_changes = Vec::new();

    // Added: in modified but not in base
    for id in mod_ids.difference(&base_ids) {
        entity_changes.push(EntityChange::Added((*mod_map[id]).clone()));
    }

    // Removed: in base but not in modified
    for id in base_ids.difference(&mod_ids) {
        entity_changes.push(EntityChange::Removed(*id));
    }

    // Modified: in both, check for differences
    for id in base_ids.intersection(&mod_ids) {
        let base_entity = base_map[id];
        let mod_entity = mod_map[id];

        let mod_result = diff_entity(base_entity, mod_entity);
        if let Some(modification) = mod_result {
            entity_changes.push(EntityChange::Modified(modification));
        }
    }

    // Settings diff
    let settings_change = diff_settings(&base.settings, &modified.settings);

    SceneDiff {
        scene_name: modified.name.clone(),
        entity_changes,
        settings_change,
    }
}

/// Compare two entities and return an `EntityModification` if they differ.
fn diff_entity(base: &SceneEntityData, modified: &SceneEntityData) -> Option<EntityModification> {
    let name = if base.name != modified.name {
        Some(modified.name.clone())
    } else {
        None
    };

    let transform = if base.transform != modified.transform {
        Some(modified.transform.clone())
    } else {
        None
    };

    let components = if base.components.len() != modified.components.len()
        || !components_equal(&base.components, &modified.components)
    {
        Some(modified.components.clone())
    } else {
        None
    };

    let children = if base.children != modified.children {
        Some(modified.children.clone())
    } else {
        None
    };

    let parent = if base.parent != modified.parent {
        Some(modified.parent)
    } else {
        None
    };

    let active = if base.active != modified.active {
        Some(modified.active)
    } else {
        None
    };

    if name.is_none()
        && transform.is_none()
        && components.is_none()
        && children.is_none()
        && parent.is_none()
        && active.is_none()
    {
        return None;
    }

    Some(EntityModification {
        id: base.id,
        name,
        transform,
        components,
        children,
        parent,
        active,
    })
}

/// Compare two component lists for equality (order-sensitive, keyed by type_name).
fn components_equal(a: &[ComponentData], b: &[ComponentData]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(ca, cb)| {
        ca.type_name == cb.type_name && properties_equal(&ca.properties, &cb.properties)
    })
}

fn properties_equal(
    a: &HashMap<String, PropertyValue>,
    b: &HashMap<String, PropertyValue>,
) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().all(|(key, val_a)| match b.get(key) {
        Some(val_b) => property_value_eq(val_a, val_b),
        None => false,
    })
}

fn property_value_eq(a: &PropertyValue, b: &PropertyValue) -> bool {
    match (a, b) {
        (PropertyValue::Float(a), PropertyValue::Float(b)) => a.to_bits() == b.to_bits(),
        (PropertyValue::Int(a), PropertyValue::Int(b)) => a == b,
        (PropertyValue::Bool(a), PropertyValue::Bool(b)) => a == b,
        (PropertyValue::String(a), PropertyValue::String(b)) => a == b,
        (PropertyValue::Vec2(a), PropertyValue::Vec2(b)) => {
            a[0].to_bits() == b[0].to_bits() && a[1].to_bits() == b[1].to_bits()
        }
        (PropertyValue::Vec3(a), PropertyValue::Vec3(b)) => {
            a[0].to_bits() == b[0].to_bits()
                && a[1].to_bits() == b[1].to_bits()
                && a[2].to_bits() == b[2].to_bits()
        }
        (PropertyValue::Vec4(a), PropertyValue::Vec4(b))
        | (PropertyValue::Color(a), PropertyValue::Color(b)) => {
            a[0].to_bits() == b[0].to_bits()
                && a[1].to_bits() == b[1].to_bits()
                && a[2].to_bits() == b[2].to_bits()
                && a[3].to_bits() == b[3].to_bits()
        }
        _ => false,
    }
}

/// Compare two scene settings and return a `SceneSettingsChange` if they differ.
fn diff_settings(base: &SceneSettings, modified: &SceneSettings) -> Option<SceneSettingsChange> {
    let ambient_color = if base.ambient_color != modified.ambient_color {
        Some(modified.ambient_color)
    } else {
        None
    };
    let fog_enabled = if base.fog_enabled != modified.fog_enabled {
        Some(modified.fog_enabled)
    } else {
        None
    };
    let fog_color = if base.fog_color != modified.fog_color {
        Some(modified.fog_color)
    } else {
        None
    };
    let fog_near = if base.fog_near.to_bits() != modified.fog_near.to_bits() {
        Some(modified.fog_near)
    } else {
        None
    };
    let fog_far = if base.fog_far.to_bits() != modified.fog_far.to_bits() {
        Some(modified.fog_far)
    } else {
        None
    };

    if ambient_color.is_none()
        && fog_enabled.is_none()
        && fog_color.is_none()
        && fog_near.is_none()
        && fog_far.is_none()
    {
        return None;
    }

    Some(SceneSettingsChange {
        ambient_color,
        fog_enabled,
        fog_color,
        fog_near,
        fog_far,
    })
}

// ── Diff Application ────────────────────────────────────────────────

/// Apply a [`SceneDiff`] to a base scene, producing the modified scene.
///
/// Returns an error if the diff references entities not present in the base.
pub fn apply_diff(base: &SceneData, diff: &SceneDiff) -> Result<SceneData, DiffError> {
    if base.name != diff.scene_name {
        return Err(DiffError::NameMismatch {
            expected: base.name.clone(),
            actual: diff.scene_name.clone(),
        });
    }

    let mut result = base.clone();

    for change in &diff.entity_changes {
        match change {
            EntityChange::Added(entity) => {
                result.entities.push(entity.clone());
            }
            EntityChange::Removed(id) => {
                let pos = result
                    .entities
                    .iter()
                    .position(|e| e.id == *id)
                    .ok_or(DiffError::EntityNotFound(*id))?;
                result.entities.remove(pos);
            }
            EntityChange::Modified(modification) => {
                let entity = result
                    .entities
                    .iter_mut()
                    .find(|e| e.id == modification.id)
                    .ok_or(DiffError::EntityNotFound(modification.id))?;

                if let Some(name) = &modification.name {
                    entity.name = name.clone();
                }
                if let Some(transform) = &modification.transform {
                    entity.transform = transform.clone();
                }
                if let Some(components) = &modification.components {
                    entity.components = components.clone();
                }
                if let Some(children) = &modification.children {
                    entity.children = children.clone();
                }
                if let Some(parent) = &modification.parent {
                    entity.parent = *parent;
                }
                if let Some(active) = &modification.active {
                    entity.active = *active;
                }
            }
        }
    }

    if let Some(sc) = &diff.settings_change {
        if let Some(v) = sc.ambient_color {
            result.settings.ambient_color = v;
        }
        if let Some(v) = sc.fog_enabled {
            result.settings.fog_enabled = v;
        }
        if let Some(v) = sc.fog_color {
            result.settings.fog_color = v;
        }
        if let Some(v) = sc.fog_near {
            result.settings.fog_near = v;
        }
        if let Some(v) = sc.fog_far {
            result.settings.fog_far = v;
        }
    }

    Ok(result)
}

// ── Diff Serialization ──────────────────────────────────────────────

/// Serialize a diff to bytes in the given format.
pub fn serialize_diff(diff: &SceneDiff, format: SceneFormat) -> Result<Vec<u8>, DiffError> {
    match format {
        SceneFormat::Json => Ok(serde_json::to_vec_pretty(diff)?),
        SceneFormat::Ron => Ok(
            ron::ser::to_string_pretty(diff, ron::ser::PrettyConfig::default())
                .map_err(SceneError::RonSer)?
                .into_bytes(),
        ),
        SceneFormat::Bin => Ok(bincode::serialize(diff)?),
    }
}

/// Deserialize a diff from bytes in the given format.
pub fn deserialize_diff(data: &[u8], format: SceneFormat) -> Result<SceneDiff, DiffError> {
    match format {
        SceneFormat::Json => Ok(serde_json::from_slice(data)?),
        SceneFormat::Ron => {
            let s = std::str::from_utf8(data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                .map_err(SceneError::Io)?;
            Ok(ron::from_str(s)?)
        }
        SceneFormat::Bin => Ok(bincode::deserialize(data)?),
    }
}

/// Save a diff to a file, auto-detecting format from the file extension.
pub fn save_diff(
    diff: &SceneDiff,
    path: impl AsRef<Path>,
    format: SceneFormat,
) -> Result<(), DiffError> {
    let path = path.as_ref();
    let data = serialize_diff(diff, format)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(SceneError::Io)?;
    }

    fs::write(path, data).map_err(SceneError::Io)?;
    Ok(())
}

/// Load a diff from a file and apply it to the given base scene.
pub fn load_and_apply_diff(
    path: impl AsRef<Path>,
    format: SceneFormat,
) -> Result<SceneDiff, DiffError> {
    let path = path.as_ref();
    let data = fs::read(path).map_err(SceneError::Io)?;
    deserialize_diff(&data, format)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{ComponentData, PropertyValue, SceneData, SceneEntityData};

    fn base_scene() -> SceneData {
        let mut scene = SceneData::new("TestScene");
        scene.settings.fog_enabled = true;

        let mut e1 = SceneEntityData::new(1, "Player");
        e1.transform.translation = [1.0, 2.0, 3.0];
        e1.add_component(
            ComponentData::new("MeshRenderer")
                .with_property("mesh", PropertyValue::String("cube.obj".into()))
                .with_property("visible", PropertyValue::Bool(true)),
        );
        scene.add_entity(e1);

        let mut e2 = SceneEntityData::new(2, "Camera");
        e2.transform.translation = [0.0, 5.0, 10.0];
        scene.add_entity(e2);

        let e3 = SceneEntityData::new(3, "Light");
        scene.add_entity(e3);

        scene
    }

    #[test]
    fn test_no_changes() {
        let scene = base_scene();
        let diff = diff_scenes(&scene, &scene);
        assert!(diff.entity_changes.is_empty());
        assert!(diff.settings_change.is_none());
    }

    #[test]
    fn test_entity_added() {
        let base = base_scene();
        let mut modified = base.clone();
        let new_entity = SceneEntityData::new(4, "Enemy");
        modified.add_entity(new_entity);

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Added(e) => assert_eq!(e.id, 4),
            _ => panic!("expected Added"),
        }
    }

    #[test]
    fn test_entity_removed() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.remove_entity(3);

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Removed(id) => assert_eq!(*id, 3),
            _ => panic!("expected Removed"),
        }
    }

    #[test]
    fn test_entity_transform_modified() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.get_entity_mut(1).unwrap().transform.translation = [10.0, 20.0, 30.0];

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Modified(m) => {
                assert_eq!(m.id, 1);
                assert!(m.transform.is_some());
                assert!(m.name.is_none());
                assert!(m.components.is_none());
            }
            _ => panic!("expected Modified"),
        }
    }

    #[test]
    fn test_entity_name_modified() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.get_entity_mut(2).unwrap().name = "MainCamera".to_string();

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Modified(m) => {
                assert_eq!(m.id, 2);
                assert_eq!(m.name.as_deref(), Some("MainCamera"));
            }
            _ => panic!("expected Modified"),
        }
    }

    #[test]
    fn test_entity_components_modified() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.get_entity_mut(1).unwrap().components.push(
            ComponentData::new("RigidBody").with_property("mass", PropertyValue::Float(10.0)),
        );

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Modified(m) => {
                assert_eq!(m.id, 1);
                assert!(m.components.is_some());
                assert_eq!(m.components.as_ref().unwrap().len(), 2);
            }
            _ => panic!("expected Modified"),
        }
    }

    #[test]
    fn test_entity_parent_modified() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.get_entity_mut(3).unwrap().parent = Some(1);

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Modified(m) => {
                assert_eq!(m.id, 3);
                assert_eq!(m.parent, Some(Some(1)));
            }
            _ => panic!("expected Modified"),
        }
    }

    #[test]
    fn test_entity_active_modified() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.get_entity_mut(2).unwrap().active = false;

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Modified(m) => {
                assert_eq!(m.id, 2);
                assert_eq!(m.active, Some(false));
            }
            _ => panic!("expected Modified"),
        }
    }

    #[test]
    fn test_settings_modified() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.settings.fog_enabled = false;
        modified.settings.fog_near = 5.0;

        let diff = diff_scenes(&base, &modified);
        assert!(diff.entity_changes.is_empty());
        let sc = diff.settings_change.as_ref().unwrap();
        assert_eq!(sc.fog_enabled, Some(false));
        assert_eq!(sc.fog_near, Some(5.0));
        assert!(sc.fog_color.is_none());
    }

    #[test]
    fn test_mixed_changes() {
        let base = base_scene();
        let mut modified = base.clone();

        // Add entity
        modified.add_entity(SceneEntityData::new(4, "Enemy"));

        // Remove entity
        modified.remove_entity(3);

        // Modify entity
        modified.get_entity_mut(1).unwrap().transform.translation = [99.0, 0.0, 0.0];

        // Modify settings
        modified.settings.ambient_color = [1.0, 1.0, 1.0, 1.0];

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 3);
        assert!(diff.settings_change.is_some());

        let added = diff
            .entity_changes
            .iter()
            .filter(|c| matches!(c, EntityChange::Added(_)))
            .count();
        let removed = diff
            .entity_changes
            .iter()
            .filter(|c| matches!(c, EntityChange::Removed(_)))
            .count();
        let modified_count = diff
            .entity_changes
            .iter()
            .filter(|c| matches!(c, EntityChange::Modified(_)))
            .count();
        assert_eq!(added, 1);
        assert_eq!(removed, 1);
        assert_eq!(modified_count, 1);
    }

    #[test]
    fn test_roundtrip_apply_diff() {
        let base = base_scene();
        let mut modified = base.clone();

        // Add
        modified.add_entity(SceneEntityData::new(4, "Enemy"));

        // Remove
        modified.remove_entity(3);

        // Modify transform
        modified.get_entity_mut(1).unwrap().transform.translation = [99.0, 0.0, 0.0];

        // Modify components
        modified.get_entity_mut(2).unwrap().add_component(
            ComponentData::new("AudioSource").with_property("volume", PropertyValue::Float(0.8)),
        );

        // Modify name
        modified.get_entity_mut(2).unwrap().name = "MainCamera".to_string();

        // Modify settings
        modified.settings.fog_enabled = false;

        let diff = diff_scenes(&base, &modified);
        let restored = apply_diff(&base, &diff).unwrap();

        assert_eq!(restored.name, modified.name);
        assert_eq!(restored.entities.len(), modified.entities.len());
        assert_eq!(restored.settings.fog_enabled, modified.settings.fog_enabled);

        // Check entities match (order may differ due to add/remove)
        for mod_entity in &modified.entities {
            let restored_entity = restored.get_entity(mod_entity.id).unwrap();
            assert_eq!(restored_entity.name, mod_entity.name);
            assert_eq!(
                restored_entity.transform.translation,
                mod_entity.transform.translation
            );
            assert_eq!(
                restored_entity.components.len(),
                mod_entity.components.len()
            );
            assert_eq!(restored_entity.active, mod_entity.active);
        }
    }

    #[test]
    fn test_roundtrip_all_formats() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.add_entity(SceneEntityData::new(4, "Enemy"));
        modified.get_entity_mut(1).unwrap().transform.translation = [99.0, 0.0, 0.0];
        modified.settings.fog_enabled = false;

        let diff = diff_scenes(&base, &modified);

        for format in [SceneFormat::Json, SceneFormat::Ron, SceneFormat::Bin] {
            let data = serialize_diff(&diff, format).unwrap();
            let loaded_diff = deserialize_diff(&data, format).unwrap();
            let restored = apply_diff(&base, &loaded_diff).unwrap();

            assert_eq!(restored.entities.len(), modified.entities.len());
            assert_eq!(restored.settings.fog_enabled, modified.settings.fog_enabled);
        }
    }

    #[test]
    fn test_diff_smaller_than_full_scene() {
        let base = base_scene();
        let mut modified = base.clone();

        // Typical modification: change one entity's transform
        modified.get_entity_mut(1).unwrap().transform.translation = [99.0, 0.0, 0.0];

        let diff = diff_scenes(&base, &modified);

        let full_json = serde_json::to_vec_pretty(&modified).unwrap();
        let diff_json = serialize_diff(&diff, SceneFormat::Json).unwrap();

        assert!(
            diff_json.len() < full_json.len(),
            "diff ({}B) should be smaller than full scene ({}B)",
            diff_json.len(),
            full_json.len()
        );

        let full_bin = bincode::serialize(&modified).unwrap();
        let diff_bin = serialize_diff(&diff, SceneFormat::Bin).unwrap();

        assert!(
            diff_bin.len() < full_bin.len(),
            "binary diff ({}B) should be smaller than full binary ({}B)",
            diff_bin.len(),
            full_bin.len()
        );
    }

    #[test]
    fn test_diff_under_30_percent() {
        // Build a larger, more realistic scene to test the 30% ratio.
        let mut base = SceneData::new("LargeScene");
        for i in 0..50 {
            let mut entity = SceneEntityData::new(i, format!("Entity_{}", i));
            entity.transform.translation = [i as f32, i as f32 * 2.0, i as f32 * 3.0];
            entity.add_component(
                ComponentData::new("MeshRenderer")
                    .with_property("mesh", PropertyValue::String(format!("mesh_{}.obj", i)))
                    .with_property("visible", PropertyValue::Bool(true)),
            );
            entity.add_component(
                ComponentData::new("Transform")
                    .with_property("position", PropertyValue::Vec3([i as f32, 0.0, 0.0])),
            );
            base.add_entity(entity);
        }

        let mut modified = base.clone();

        // Typical modification: change one entity's transform and add a component
        modified.get_entity_mut(5).unwrap().transform.translation = [99.0, 0.0, 0.0];
        modified.get_entity_mut(5).unwrap().add_component(
            ComponentData::new("RigidBody").with_property("mass", PropertyValue::Float(10.0)),
        );

        let diff = diff_scenes(&base, &modified);

        let full_bin = bincode::serialize(&modified).unwrap();
        let diff_bin = serialize_diff(&diff, SceneFormat::Bin).unwrap();

        let ratio = diff_bin.len() as f64 / full_bin.len() as f64;
        assert!(
            ratio < 0.30,
            "diff should be <30% of full size, got {:.1}% (diff={}B, full={}B)",
            ratio * 100.0,
            diff_bin.len(),
            full_bin.len()
        );
    }

    #[test]
    fn test_empty_diff_roundtrip() {
        let scene = base_scene();
        let diff = diff_scenes(&scene, &scene);
        let restored = apply_diff(&scene, &diff).unwrap();
        assert_eq!(restored.name, scene.name);
        assert_eq!(restored.entities.len(), scene.entities.len());
    }

    #[test]
    fn test_file_io_diff() {
        let dir = std::env::temp_dir().join("rust_engine_diff_test");
        let path = dir.join("test_diff.json");

        let base = base_scene();
        let mut modified = base.clone();
        modified.get_entity_mut(1).unwrap().transform.translation = [99.0, 0.0, 0.0];

        let diff = diff_scenes(&base, &modified);
        save_diff(&diff, &path, SceneFormat::Json).unwrap();

        let loaded_diff = load_and_apply_diff(&path, SceneFormat::Json).unwrap();
        let restored = apply_diff(&base, &loaded_diff).unwrap();

        assert_eq!(restored.entities.len(), modified.entities.len());
        assert_eq!(
            restored.get_entity(1).unwrap().transform.translation,
            [99.0, 0.0, 0.0]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scene_name_mismatch_error() {
        let base = base_scene();
        let mut modified = base.clone();
        modified.name = "DifferentScene".to_string();
        modified.add_entity(SceneEntityData::new(4, "Enemy"));

        let diff = diff_scenes(&base, &modified);
        let result = apply_diff(&base, &diff);
        assert!(result.is_err());
        match result.unwrap_err() {
            DiffError::NameMismatch { expected, actual } => {
                assert_eq!(expected, "TestScene");
                assert_eq!(actual, "DifferentScene");
            }
            other => panic!("expected NameMismatch, got: {:?}", other),
        }
    }

    #[test]
    fn test_multiple_modifications_same_entity() {
        let base = base_scene();
        let mut modified = base.clone();
        let entity = modified.get_entity_mut(1).unwrap();
        entity.name = "RenamedPlayer".to_string();
        entity.transform.translation = [0.0, 0.0, 0.0];
        entity.active = false;

        let diff = diff_scenes(&base, &modified);
        assert_eq!(diff.entity_changes.len(), 1);
        match &diff.entity_changes[0] {
            EntityChange::Modified(m) => {
                assert_eq!(m.name.as_deref(), Some("RenamedPlayer"));
                assert!(m.transform.is_some());
                assert_eq!(m.active, Some(false));
            }
            _ => panic!("expected Modified"),
        }

        let restored = apply_diff(&base, &diff).unwrap();
        let e = restored.get_entity(1).unwrap();
        assert_eq!(e.name, "RenamedPlayer");
        assert_eq!(e.transform.translation, [0.0, 0.0, 0.0]);
        assert!(!e.active);
    }
}
