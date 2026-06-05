use crate::error::EcsError;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Type-erased component storage trait.
///
/// Implementors store components for a single concrete type and expose
/// them through [`Any`] downcasting.
pub trait Storage: Any + Send + Sync {
    /// Remove the component at the given entity `index`.
    fn remove_index(&mut self, index: u32);
    /// Borrow as `&dyn Any` for downcasting.
    fn as_any_ref(&self) -> &dyn Any;
    /// Borrow as `&mut dyn Any` for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Compact the sparse storage, returning the number of freed slots.
    fn compact(&mut self) -> usize;
    /// Return the number of occupied entries in this storage.
    fn len(&self) -> usize;
    /// Return `true` if this storage has no entries.
    fn is_empty(&self) -> bool;
    /// Return the number of wasted (empty) sparse slots.
    fn wasted_slots(&self) -> usize;
}

/// A sparse-set backed component storage.
///
/// Provides O(1) insert, remove, and lookup by entity index while
/// keeping a dense list of occupied indices for fast iteration.
pub struct SparseSet<T> {
    sparse: Vec<Option<T>>,
    entities: Vec<u32>,
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SparseSet<T> {
    /// Create an empty sparse set.
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            entities: Vec::new(),
        }
    }

    /// Insert a component `value` for the entity at `index`.
    ///
    /// If a component already exists at `index` it is overwritten.
    pub fn insert(&mut self, index: u32, value: T) {
        if index as usize >= self.sparse.len() {
            self.sparse.resize_with(index as usize + 1, || None);
        }
        if self.sparse[index as usize].is_none() {
            self.entities.push(index);
        }
        self.sparse[index as usize] = Some(value);
    }

    /// Remove and return the component at `index`, or `None` if absent.
    pub fn remove(&mut self, index: u32) -> Option<T> {
        if (index as usize) < self.sparse.len() {
            let val = self.sparse[index as usize].take();
            if val.is_some() {
                self.entities.retain(|&i| i != index);
            }
            val
        } else {
            None
        }
    }

    /// Get a shared reference to the component at `index`.
    pub fn get(&self, index: u32) -> Option<&T> {
        if (index as usize) < self.sparse.len() {
            self.sparse[index as usize].as_ref()
        } else {
            None
        }
    }

    /// Get an exclusive reference to the component at `index`.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        if (index as usize) < self.sparse.len() {
            self.sparse[index as usize].as_mut()
        } else {
            None
        }
    }

    /// Return the list of entity indices that have a component in this storage.
    pub fn entities(&self) -> &[u32] {
        &self.entities
    }

    /// Defragment the sparse array by shrinking it to fit only occupied slots.
    ///
    /// After many spawn/despawn cycles the sparse Vec can grow large with
    /// mostly `None` slots. This method rebuilds it to the minimum size
    /// needed, reclaiming memory.
    ///
    /// Returns the number of `None` slots that were removed.
    pub fn compact(&mut self) -> usize {
        if self.entities.is_empty() {
            let freed = self.sparse.len();
            self.sparse.clear();
            self.sparse.shrink_to_fit();
            return freed;
        }

        let max_index = *self
            .entities
            .iter()
            .max()
            .expect("entities non-empty: guarded by is_empty above")
            as usize;
        let old_len = self.sparse.len();

        if max_index + 1 >= old_len {
            return 0; // already minimal
        }

        self.sparse.truncate(max_index + 1);
        self.sparse.shrink_to_fit();
        old_len - self.sparse.len()
    }

    /// Return the length of the sparse array (including empty slots).
    pub fn sparse_len(&self) -> usize {
        self.sparse.len()
    }

    /// Return the number of occupied entries.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Return `true` if this storage has no entries.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Return the number of empty slots in the sparse array.
    pub fn wasted_slots(&self) -> usize {
        self.sparse.len().saturating_sub(self.entities.len())
    }
}

impl<T: Send + Sync + 'static> Storage for SparseSet<T> {
    fn remove_index(&mut self, index: u32) {
        self.entities.retain(|e| *e != index);
        if let Some(slot) = self.sparse.get_mut(index as usize) {
            *slot = None;
        }
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn compact(&mut self) -> usize {
        SparseSet::compact(self)
    }

    fn len(&self) -> usize {
        SparseSet::len(self)
    }

    fn is_empty(&self) -> bool {
        SparseSet::is_empty(self)
    }

    fn wasted_slots(&self) -> usize {
        SparseSet::wasted_slots(self)
    }
}

/// Registry that holds one [`SparseSet`] per component type.
///
/// Component types are identified by [`TypeId`]; storage is created lazily
/// on first access.
pub struct ComponentRegistry {
    storages: HashMap<TypeId, Box<dyn Storage>>,
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
        }
    }

    /// Try to get (or create) the sparse-set storage for type `T`.
    pub fn try_storage<T: Send + Sync + 'static>(&mut self) -> Result<&mut SparseSet<T>, EcsError> {
        let tid = TypeId::of::<T>();
        self.storages
            .entry(tid)
            .or_insert_with(|| Box::new(SparseSet::<T>::new()))
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .ok_or_else(|| EcsError::ComponentNotRegistered(std::any::type_name::<T>().to_string()))
    }

    /// Get (or create) the sparse-set storage for type `T`.
    ///
    /// # Panics
    /// Panics if the internal type-id to storage mapping is corrupted
    /// (should be impossible under normal operation).
    pub fn storage<T: Send + Sync + 'static>(&mut self) -> &mut SparseSet<T> {
        self.try_storage::<T>()
            .expect("TypeId-to-SparseSet invariant violated")
    }

    /// Try to get shared access to the storage for type `T`.
    pub fn try_get_storage<T: 'static>(&self) -> Option<&SparseSet<T>> {
        let tid = TypeId::of::<T>();
        self.storages
            .get(&tid)?
            .as_any_ref()
            .downcast_ref::<SparseSet<T>>()
    }

    /// Try to get exclusive access to the storage for type `T`.
    pub fn try_get_storage_mut<T: 'static>(&mut self) -> Option<&mut SparseSet<T>> {
        let tid = TypeId::of::<T>();
        self.storages
            .get_mut(&tid)?
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
    }

    /// Remove all components for the entity at `index` across every storage.
    pub fn remove_entity(&mut self, index: u32) {
        for storage in self.storages.values_mut() {
            storage.remove_index(index);
        }
    }

    /// Compact all storages, returning total freed slots.
    pub fn compact_all(&mut self) -> usize {
        self.storages.values_mut().map(|s| s.compact()).sum()
    }

    /// Return a summary of memory usage per component type.
    pub fn memory_summary(&self) -> Vec<(std::any::TypeId, usize, usize)> {
        self.storages
            .iter()
            .map(|(&tid, s)| (tid, s.len(), s.wasted_slots()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::SparseSet;

    #[test]
    fn test_sparse_set_insert_and_get() {
        let mut set = SparseSet::new();
        set.insert(0, 42);
        assert_eq!(set.get(0), Some(&42));
    }

    #[test]
    fn test_sparse_set_get_nonexistent() {
        let set = SparseSet::<i32>::new();
        assert_eq!(set.get(0), None);
    }

    #[test]
    fn test_sparse_set_remove() {
        let mut set = SparseSet::new();
        set.insert(0, 42);
        assert_eq!(set.remove(0), Some(42));
        assert_eq!(set.get(0), None);
    }

    #[test]
    fn test_sparse_set_entities_tracking() {
        let mut set = SparseSet::new();
        set.insert(5, "a");
        set.insert(3, "b");
        assert_eq!(set.entities(), &[5, 3]);
    }

    #[test]
    fn test_sparse_set_compact() {
        let mut set = SparseSet::<i32>::new();
        // Insert at high index to grow sparse vec
        set.insert(100, 1);
        set.insert(200, 2);
        assert!(set.sparse_len() > 200);

        // Remove the high-index entry
        set.remove(200);
        assert!(set.wasted_slots() > 0);

        let freed = set.compact();
        assert!(freed > 0);
        // After compaction, sparse should shrink to fit index 100
        assert!(set.sparse_len() <= 101);
        // The remaining entry should still be accessible
        assert_eq!(set.get(100), Some(&1));
    }

    #[test]
    fn test_sparse_set_compact_empty() {
        let mut set = SparseSet::<i32>::new();
        set.insert(50, 1);
        set.remove(50);
        let freed = set.compact();
        assert!(freed > 0);
        assert_eq!(set.sparse_len(), 0);
    }

    #[test]
    fn test_sparse_set_len() {
        let mut set = SparseSet::<i32>::new();
        assert_eq!(set.len(), 0);
        set.insert(0, 1);
        set.insert(1, 2);
        assert_eq!(set.len(), 2);
        set.remove(0);
        assert_eq!(set.len(), 1);
    }
}
