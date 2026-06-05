use engine_math::Mat4;

/// Unique key for grouping identical meshes into instanced batches.
///
/// Entities sharing the same `InstanceKey` are rendered with a single
/// `draw_indexed_indirect` call using multiple instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceKey {
    /// Mesh asset identifier (high-detail LOD mesh).
    pub mesh_id: u64,
    /// Material asset identifier.
    pub material_id: u64,
}

impl InstanceKey {
    pub fn new(mesh_id: u64, material_id: u64) -> Self {
        Self {
            mesh_id,
            material_id,
        }
    }
}

/// A batch of instances for the same mesh + material combination.
///
/// Each entry in `transforms` is a world-space model matrix for one instance.
/// The renderer uploads these as an instance buffer and issues a single
/// indirect draw call per batch.
#[derive(Debug, Clone)]
pub struct InstanceBatch {
    pub key: InstanceKey,
    pub transforms: Vec<Mat4>,
}

impl InstanceBatch {
    pub fn new(key: InstanceKey) -> Self {
        Self {
            key,
            transforms: Vec::new(),
        }
    }

    /// Number of instances in this batch.
    pub fn instance_count(&self) -> u32 {
        self.transforms.len() as u32
    }

    /// Add an instance transform.
    pub fn push(&mut self, transform: Mat4) {
        self.transforms.push(transform);
    }
}

/// Groups entities into instanced batches by mesh + material.
///
/// After culling and LOD selection, call this to collect visible entities
/// into [`InstanceBatch`]es that minimize draw calls.
///
/// # Arguments
///
/// * `keys` — per-entity `InstanceKey` (mesh + material after LOD selection)
/// * `transforms` — per-entity world transform matrix
///
/// Returns a `Vec<InstanceBatch>` with one entry per unique `InstanceKey`.
pub fn collect_instance_batches(keys: &[InstanceKey], transforms: &[Mat4]) -> Vec<InstanceBatch> {
    use std::collections::HashMap;

    let mut batch_map: HashMap<InstanceKey, Vec<Mat4>> = HashMap::new();
    let mut order: Vec<InstanceKey> = Vec::new();

    for (key, transform) in keys.iter().zip(transforms.iter()) {
        let entry = batch_map.entry(*key);
        if matches!(entry, std::collections::hash_map::Entry::Vacant(_)) {
            order.push(*key);
        }
        entry.or_default().push(*transform);
    }

    order
        .into_iter()
        .map(|key| {
            let transforms = batch_map.remove(&key).expect("key must exist in batch_map");
            InstanceBatch { key, transforms }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(mesh: u64, mat: u64) -> InstanceKey {
        InstanceKey::new(mesh, mat)
    }

    #[test]
    fn test_single_instance() {
        let keys = vec![key(1, 0)];
        let transforms = vec![Mat4::IDENTITY];
        let batches = collect_instance_batches(&keys, &transforms);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].instance_count(), 1);
    }

    #[test]
    fn test_grouping_identical_mesh() {
        let keys = vec![key(1, 0), key(1, 0), key(1, 0)];
        let transforms = vec![Mat4::IDENTITY; 3];
        let batches = collect_instance_batches(&keys, &transforms);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].instance_count(), 3);
    }

    #[test]
    fn test_different_materials_separate_batches() {
        let keys = vec![key(1, 0), key(1, 1)];
        let transforms = vec![Mat4::IDENTITY; 2];
        let batches = collect_instance_batches(&keys, &transforms);
        assert_eq!(batches.len(), 2);
    }

    #[test]
    fn test_different_meshes_separate_batches() {
        let keys = vec![key(1, 0), key(2, 0)];
        let transforms = vec![Mat4::IDENTITY; 2];
        let batches = collect_instance_batches(&keys, &transforms);
        assert_eq!(batches.len(), 2);
    }

    #[test]
    fn test_preserves_insertion_order() {
        let keys = vec![key(3, 0), key(1, 0), key(3, 0), key(1, 0)];
        let transforms = vec![Mat4::IDENTITY; 4];
        let batches = collect_instance_batches(&keys, &transforms);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].key.mesh_id, 3);
        assert_eq!(batches[1].key.mesh_id, 1);
    }

    #[test]
    fn test_empty_input() {
        let batches = collect_instance_batches(&[], &[]);
        assert!(batches.is_empty());
    }

    #[test]
    fn test_mixed_batch_sizes() {
        let keys = vec![key(1, 0), key(2, 0), key(1, 0), key(1, 0)];
        let transforms = vec![Mat4::IDENTITY; 4];
        let batches = collect_instance_batches(&keys, &transforms);
        assert_eq!(batches.len(), 2);
        // mesh 1 has 3 instances, mesh 2 has 1
        let b1 = batches.iter().find(|b| b.key.mesh_id == 1).unwrap();
        let b2 = batches.iter().find(|b| b.key.mesh_id == 2).unwrap();
        assert_eq!(b1.instance_count(), 3);
        assert_eq!(b2.instance_count(), 1);
    }
}
