//! Spatial hash grid broadphase for collision detection.

use engine_math::Vec3;
use std::collections::{HashMap, HashSet};

/// Cell coordinates in the spatial hash grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CellCoord(i32, i32, i32);

/// An entry in the broadphase: entity index + AABB bounds.
#[derive(Debug, Clone, Copy)]
pub struct BroadphaseEntry {
    pub entity_index: u32,
    pub center: Vec3,
    pub half_extents: Vec3,
}

impl BroadphaseEntry {
    /// Compute the AABB min corner.
    pub fn aabb_min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    /// Compute the AABB max corner.
    pub fn aabb_max(&self) -> Vec3 {
        self.center + self.half_extents
    }
}

/// A candidate pair from the broadphase.
#[derive(Debug, Clone, Copy)]
pub struct BroadphasePair {
    pub index_a: u32,
    pub index_b: u32,
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
    /// Returns unique pairs — deduplication is handled internally.
    pub fn compute_pairs(&self) -> Vec<BroadphasePair> {
        let mut seen = HashSet::new();
        let mut pairs = Vec::new();

        for cell_entries in self.grid.values() {
            for i in 0..cell_entries.len() {
                for j in (i + 1)..cell_entries.len() {
                    let a = self.entries[cell_entries[i]].entity_index;
                    let b = self.entries[cell_entries[j]].entity_index;

                    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                    if seen.insert((lo, hi)) {
                        pairs.push(BroadphasePair {
                            index_a: lo,
                            index_b: hi,
                        });
                    }
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

    #[test]
    fn test_broadphase_no_overlap() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        bp.insert(BroadphaseEntry {
            entity_index: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            half_extents: Vec3::new(0.5, 0.5, 0.5),
        });
        bp.insert(BroadphaseEntry {
            entity_index: 1,
            center: Vec3::new(10.0, 0.0, 0.0),
            half_extents: Vec3::new(0.5, 0.5, 0.5),
        });

        let pairs = bp.compute_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_broadphase_same_cell() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        bp.insert(BroadphaseEntry {
            entity_index: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            half_extents: Vec3::new(0.5, 0.5, 0.5),
        });
        bp.insert(BroadphaseEntry {
            entity_index: 1,
            center: Vec3::new(0.5, 0.0, 0.0),
            half_extents: Vec3::new(0.5, 0.5, 0.5),
        });

        let pairs = bp.compute_pairs();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].index_a, 0);
        assert_eq!(pairs[0].index_b, 1);
    }

    #[test]
    fn test_broadphase_dedup() {
        let mut bp = SpatialHashBroadphase::new(10.0);
        // Both entries span the entire cell — should produce exactly 1 pair
        bp.insert(BroadphaseEntry {
            entity_index: 0,
            center: Vec3::new(0.0, 0.0, 0.0),
            half_extents: Vec3::new(5.0, 5.0, 5.0),
        });
        bp.insert(BroadphaseEntry {
            entity_index: 1,
            center: Vec3::new(1.0, 0.0, 0.0),
            half_extents: Vec3::new(5.0, 5.0, 5.0),
        });

        let pairs = bp.compute_pairs();
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_broadphase_multiple_entries() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        for i in 0..10 {
            bp.insert(BroadphaseEntry {
                entity_index: i,
                center: Vec3::new(i as f32 * 0.5, 0.0, 0.0),
                half_extents: Vec3::new(0.5, 0.5, 0.5),
            });
        }

        let pairs = bp.compute_pairs();
        // Adjacent entries should produce pairs
        assert!(!pairs.is_empty());
    }

    #[test]
    fn test_broadphase_clear() {
        let mut bp = SpatialHashBroadphase::new(2.0);
        bp.insert(BroadphaseEntry {
            entity_index: 0,
            center: Vec3::ZERO,
            half_extents: Vec3::ONE,
        });
        bp.clear();
        assert_eq!(bp.entry_count(), 0);
        assert_eq!(bp.cell_count(), 0);
    }
}
