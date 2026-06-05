//! Spatial hash grid broadphase for collision detection.

use engine_math::Vec3;
use std::collections::{HashMap, HashSet};

/// Cell coordinates in the spatial hash grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CellCoord(i32, i32, i32);

/// An entry in the broadphase: entity index + AABB bounds + collision layer info.
#[derive(Debug, Clone, Copy)]
pub struct BroadphaseEntry {
    pub entity_index: u32,
    pub center: Vec3,
    pub half_extents: Vec3,
    /// Which collision layers this entity belongs to (bitmask).
    pub collision_layers: u32,
    /// Which collision layers this entity can collide with (bitmask).
    pub collision_mask: u32,
}

impl BroadphaseEntry {
    /// Compute the AABB min corner (center - half_extents).
    pub fn aabb_min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    /// Compute the AABB max corner.
    pub fn aabb_max(&self) -> Vec3 {
        self.center + self.half_extents
    }

    /// Check if this entry can collide with another based on layer masks.
    pub fn can_collide_with(&self, other: &BroadphaseEntry) -> bool {
        (self.collision_layers & other.collision_mask) != 0
            && (other.collision_layers & self.collision_mask) != 0
    }
}

/// A candidate pair from the broadphase.
#[derive(Debug, Clone, Copy)]
pub struct BroadphasePair {
    pub index_a: u32,
    pub index_b: u32,
}

/// Check if two AABBs overlap (tight AABB refinement test).
///
/// Used internally by the broadphase to filter false positives from shared cells.
pub fn aabb_overlap(a: &BroadphaseEntry, b: &BroadphaseEntry) -> bool {
    let a_min = a.aabb_min();
    let a_max = a.aabb_max();
    let b_min = b.aabb_min();
    let b_max = b.aabb_max();

    a_min.x <= b_max.x
        && a_max.x >= b_min.x
        && a_min.y <= b_max.y
        && a_max.y >= b_min.y
        && a_min.z <= b_max.z
        && a_max.z >= b_min.z
}

/// Spatial hash grid broadphase for collision detection.
///
/// Divides space into uniform cells and only tests pairs that share
/// a cell, reducing O(n²) to O(n) for uniformly distributed objects.
pub struct SpatialHashBroadphase {
    /// Cell size (should be >= largest object diameter).
    cell_size: f32,
    /// The hash grid: cell coordinates → list of entry indices.
    grid: HashMap<CellCoord, Vec<usize>>,
    /// Entries for the current frame.
    entries: Vec<BroadphaseEntry>,
}

impl SpatialHashBroadphase {
    /// Create a new spatial hash broadphase with the given cell size.
    ///
    /// Cell size should be >= the largest collider diameter for optimal performance.
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size: cell_size.max(0.1),
            grid: HashMap::new(),
            entries: Vec::new(),
        }
    }

    /// Clear all entries and prepare for a new frame.
    pub fn clear(&mut self) {
        self.grid.clear();
        self.entries.clear();
    }

    /// Insert an entry into the broadphase.
    pub fn insert(&mut self, entry: BroadphaseEntry) {
        let idx = self.entries.len();
        let inv = 1.0 / self.cell_size;

        let min = entry.aabb_min();
        let max = entry.aabb_max();

        let x0 = (min.x * inv).floor() as i32;
        let y0 = (min.y * inv).floor() as i32;
        let z0 = (min.z * inv).floor() as i32;
        let x1 = (max.x * inv).floor() as i32;
        let y1 = (max.y * inv).floor() as i32;
        let z1 = (max.z * inv).floor() as i32;

        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    self.grid.entry(CellCoord(x, y, z)).or_default().push(idx);
                }
            }
        }

        self.entries.push(entry);
    }

    /// Compute candidate pairs (entities that share at least one cell).
    ///
    /// Applies layer mask filtering and AABB refinement to reduce false positives.
    /// Returns unique pairs — deduplication is handled internally.
    pub fn compute_pairs(&self) -> Vec<BroadphasePair> {
        let mut seen = HashSet::new();
        let mut pairs = Vec::new();

        for cell_entries in self.grid.values() {
            for i in 0..cell_entries.len() {
                for j in (i + 1)..cell_entries.len() {
                    let entry_a = &self.entries[cell_entries[i]];
                    let entry_b = &self.entries[cell_entries[j]];

                    let a = entry_a.entity_index;
                    let b = entry_b.entity_index;

                    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                    if seen.contains(&(lo, hi)) {
                        continue;
                    }

                    // Layer mask filtering: skip pairs that can never collide
                    if !entry_a.can_collide_with(entry_b) {
                        continue;
                    }

                    // AABB refinement: tight overlap test before narrowphase
                    if !aabb_overlap(entry_a, entry_b) {
                        continue;
                    }

                    seen.insert((lo, hi));
                    pairs.push(BroadphasePair {
                        index_a: lo,
                        index_b: hi,
                    });
                }
            }
        }

        pairs
    }

    /// Return the number of entries inserted.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Return the number of cells occupied.
    pub fn cell_count(&self) -> usize {
        self.grid.len()
    }

    /// Set the cell size.
    pub fn set_cell_size(&mut self, cell_size: f32) {
        self.cell_size = cell_size.max(0.1);
    }

    /// Get the cell size.
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(idx: u32, center: Vec3, half: Vec3) -> BroadphaseEntry {
        BroadphaseEntry {
            entity_index: idx,
            center,
            half_extents: half,
            collision_layers: 0xFFFF_FFFF,
            collision_mask: 0xFFFF_FFFF,
        }
    }

    fn entry_with_layers(
        idx: u32,
        center: Vec3,
        half: Vec3,
        layers: u32,
        mask: u32,
    ) -> BroadphaseEntry {
        BroadphaseEntry {
            entity_index: idx,
            center,
            half_extents: half,
            collision_layers: layers,
            collision_mask: mask,
        }
    }

    #[test]
    fn test_broadphase_no_overlap() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        bp.insert(entry(0, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.5, 0.5)));
        bp.insert(entry(
            1,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
        ));

        let pairs = bp.compute_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_broadphase_same_cell() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        bp.insert(entry(0, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.5, 0.5)));
        bp.insert(entry(1, Vec3::new(0.5, 0.0, 0.0), Vec3::new(0.5, 0.5, 0.5)));

        let pairs = bp.compute_pairs();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].index_a, 0);
        assert_eq!(pairs[0].index_b, 1);
    }

    #[test]
    fn test_broadphase_dedup() {
        let mut bp = SpatialHashBroadphase::new(10.0);
        bp.insert(entry(0, Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0)));
        bp.insert(entry(1, Vec3::new(1.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0)));

        let pairs = bp.compute_pairs();
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_broadphase_multiple_entries() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        for i in 0..10 {
            bp.insert(entry(
                i,
                Vec3::new(i as f32 * 0.5, 0.0, 0.0),
                Vec3::new(0.5, 0.5, 0.5),
            ));
        }

        let pairs = bp.compute_pairs();
        assert!(!pairs.is_empty());
    }

    #[test]
    fn test_broadphase_clear() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        bp.insert(entry(0, Vec3::ZERO, Vec3::ONE));
        bp.clear();
        assert_eq!(bp.entry_count(), 0);
        assert_eq!(bp.cell_count(), 0);
    }

    #[test]
    fn test_layer_mask_filtering() {
        let mut bp = SpatialHashBroadphase::new(10.0);
        // Layer 1 (bit 0) vs layer 2 (bit 1) — masks don't match
        bp.insert(entry_with_layers(0, Vec3::ZERO, Vec3::ONE, 0x01, 0x01));
        bp.insert(entry_with_layers(
            1,
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::ONE,
            0x02,
            0x02,
        ));

        let pairs = bp.compute_pairs();
        assert!(pairs.is_empty(), "Layer mismatch should filter the pair");
    }

    #[test]
    fn test_layer_mask_allows_collision() {
        let mut bp = SpatialHashBroadphase::new(10.0);
        // Both on layer 1, mask includes each other
        bp.insert(entry_with_layers(0, Vec3::ZERO, Vec3::ONE, 0x01, 0x01));
        bp.insert(entry_with_layers(
            1,
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::ONE,
            0x01,
            0x01,
        ));

        let pairs = bp.compute_pairs();
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_aabb_refinement_same_cell_no_overlap() {
        let mut bp = SpatialHashBroadphase::new(10.0);
        // Large cell size forces same-cell grouping, but AABBs don't overlap
        bp.insert(entry(
            0,
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        ));
        bp.insert(entry(
            1,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        ));

        let pairs = bp.compute_pairs();
        assert!(
            pairs.is_empty(),
            "AABB refinement should filter non-overlapping pairs"
        );
    }

    #[test]
    fn test_aabb_refinement_partial_overlap() {
        let mut bp = SpatialHashBroadphase::new(10.0);
        bp.insert(entry(0, Vec3::ZERO, Vec3::ONE));
        bp.insert(entry(1, Vec3::new(1.5, 0.0, 0.0), Vec3::ONE));

        let pairs = bp.compute_pairs();
        assert_eq!(pairs.len(), 1, "Overlapping AABBs should produce a pair");
    }
}
