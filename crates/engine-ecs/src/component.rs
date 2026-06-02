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

    /// Get (or create) the sparse-set storage for type `T`.
    pub fn storage<T: Send + Sync + 'static>(&mut self) -> &mut SparseSet<T> {
        let tid = TypeId::of::<T>();
        self.storages
            .entry(tid)
            .or_insert_with(|| Box::new(SparseSet::<T>::new()))
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("Type mismatch in ComponentRegistry")
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
}
