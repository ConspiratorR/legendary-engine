use std::any::TypeId;
use std::marker::PhantomData;

/// Describes which resources/components a system reads.
///
/// Used by the parallel scheduler to detect conflicts between systems.
pub struct Read<T: 'static> {
    _marker: PhantomData<T>,
}

impl<T: 'static> Default for Read<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

/// Describes which resources/components a system writes.
///
/// Used by the parallel scheduler to detect conflicts between systems.
pub struct Write<T: 'static> {
    _marker: PhantomData<T>,
}

impl<T: 'static> Default for Write<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

/// The set of component/resource types a system reads or writes.
///
/// The scheduler uses this to determine which systems can run concurrently:
/// - Two systems with overlapping writes cannot run in parallel.
/// - A read and a write to the same type cannot run in parallel.
/// - Two systems that only read the same type CAN run in parallel.
#[derive(Debug, Clone, Default)]
pub struct SystemAccess {
    /// Types this system reads (shared access).
    pub reads: Vec<TypeId>,
    /// Types this system writes (exclusive access).
    pub writes: Vec<TypeId>,
}

impl SystemAccess {
    /// Create an empty access descriptor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a read dependency on type `T`.
    pub fn read<T: 'static>(&mut self) -> &mut Self {
        let tid = TypeId::of::<T>();
        if !self.reads.contains(&tid) {
            self.reads.push(tid);
        }
        self
    }

    /// Add a write dependency on type `T`.
    pub fn write<T: 'static>(&mut self) -> &mut Self {
        let tid = TypeId::of::<T>();
        if !self.writes.contains(&tid) {
            self.writes.push(tid);
        }
        self
    }

    /// Check if this access conflicts with another.
    ///
    /// A conflict exists when:
    /// - Both write to the same type, OR
    /// - One writes to a type the other reads.
    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        for w in &self.writes {
            if other.writes.contains(w) {
                return true;
            }
        }
        for w in &self.writes {
            if other.reads.contains(w) {
                return true;
            }
        }
        for r in &self.reads {
            if other.writes.contains(r) {
                return true;
            }
        }
        false
    }
}
