//! Memory management utilities: object pools, leak detection, and usage metrics.

use std::alloc::Layout;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Object Pool
// ---------------------------------------------------------------------------

/// A pre-allocated pool for reusing objects of type `T`.
///
/// Avoids repeated heap allocations for frequently created/destroyed objects
/// (entities, components, temporary buffers).
///
/// # Example
///
/// ```
/// use engine_core::memory::Pool;
///
/// let mut pool = Pool::<Vec<u32>>::with_capacity(64);
/// let mut v = pool.acquire();          // reuses or allocates
/// v.push(42);
/// pool.release(v);                     // returns to pool (cleared)
/// let v2 = pool.acquire();            // reuses the returned vec
/// assert!(v2.is_empty());
/// ```
pub struct Pool<T> {
    available: Vec<T>,
    capacity: usize,
}

impl<T: Default> Pool<T> {
    /// Create a pool pre-filled with `capacity` default instances.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut available = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            available.push(T::default());
        }
        Self {
            available,
            capacity,
        }
    }

    /// Acquire an object from the pool. If the pool is empty, a new `T` is
    /// allocated via [`Default`].
    pub fn acquire(&mut self) -> T {
        self.available.pop().unwrap_or_default()
    }

    /// Return an object to the pool. The object is cleared via
    /// [`Default::default()`] before storage so the next user gets a clean
    /// instance. If the pool is at capacity the object is dropped.
    pub fn release(&mut self, _value: T) {
        if self.available.len() < self.capacity {
            self.available.push(T::default());
        }
        // else: drop _value
    }

    /// Number of objects currently available in the pool.
    pub fn len(&self) -> usize {
        self.available.len()
    }

    /// Returns `true` if the pool has no available objects.
    pub fn is_empty(&self) -> bool {
        self.available.is_empty()
    }

    /// Pre-allocate additional objects up to `total` capacity.
    pub fn reserve(&mut self, additional: usize) {
        self.available.reserve(additional);
        for _ in 0..additional {
            self.available.push(T::default());
        }
    }
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self {
            available: Vec::new(),
            capacity: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Memory Tracker (debug-mode allocation tracking)
// ---------------------------------------------------------------------------

static ALLOC_COUNTERS: Mutex<Option<TrackerState>> = Mutex::new(None);

struct TrackerState {
    counters: HashMap<TypeId, AllocCounters>,
    peak_live_objects: usize,
    peak_bytes_in_use: usize,
    frame_snapshots: Vec<MemorySnapshot>,
    max_snapshots: usize,
}

struct AllocCounters {
    allocs: AtomicUsize,
    deallocs: AtomicUsize,
    bytes_allocated: AtomicUsize,
    bytes_deallocated: AtomicUsize,
    peak_bytes_in_use: AtomicUsize,
    peak_live_objects: AtomicUsize,
}

impl AllocCounters {
    fn new() -> Self {
        Self {
            allocs: AtomicUsize::new(0),
            deallocs: AtomicUsize::new(0),
            bytes_allocated: AtomicUsize::new(0),
            bytes_deallocated: AtomicUsize::new(0),
            peak_bytes_in_use: AtomicUsize::new(0),
            peak_live_objects: AtomicUsize::new(0),
        }
    }
}

/// Tracks allocations and deallocations per type for leak detection.
///
/// Only active in debug builds (`cfg(debug_assertions)`). In release builds
/// all methods are no-ops with zero overhead.
pub struct MemoryTracker;

impl MemoryTracker {
    /// Record an allocation of `size` bytes for type `T`.
    pub fn record_alloc<T: 'static>(size: usize) {
        #[cfg(debug_assertions)]
        {
            let tid = TypeId::of::<T>();
            let mut map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            let state = map.get_or_insert_with(|| TrackerState {
                counters: HashMap::new(),
                peak_live_objects: 0,
                peak_bytes_in_use: 0,
                frame_snapshots: Vec::new(),
                max_snapshots: 120,
            });
            let entry = state.counters.entry(tid).or_insert_with(AllocCounters::new);
            entry.allocs.fetch_add(1, Ordering::Relaxed);
            entry.bytes_allocated.fetch_add(size, Ordering::Relaxed);

            // Update per-type peak
            let live = entry
                .allocs
                .load(Ordering::Relaxed)
                .saturating_sub(entry.deallocs.load(Ordering::Relaxed));
            let bytes_in = entry
                .bytes_allocated
                .load(Ordering::Relaxed)
                .saturating_sub(entry.bytes_deallocated.load(Ordering::Relaxed));
            update_peak(&entry.peak_live_objects, live);
            update_peak(&entry.peak_bytes_in_use, bytes_in);

            // Update global peak
            if live > state.peak_live_objects {
                state.peak_live_objects = live;
            }
            if bytes_in > state.peak_bytes_in_use {
                state.peak_bytes_in_use = bytes_in;
            }
        }
        let _ = size;
    }

    /// Record a deallocation of `size` bytes for type `T`.
    pub fn record_dealloc<T: 'static>(size: usize) {
        #[cfg(debug_assertions)]
        {
            let tid = TypeId::of::<T>();
            let mut map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            let state = map.get_or_insert_with(|| TrackerState {
                counters: HashMap::new(),
                peak_live_objects: 0,
                peak_bytes_in_use: 0,
                frame_snapshots: Vec::new(),
                max_snapshots: 120,
            });
            let entry = state.counters.entry(tid).or_insert_with(AllocCounters::new);
            entry.deallocs.fetch_add(1, Ordering::Relaxed);
            entry.bytes_deallocated.fetch_add(size, Ordering::Relaxed);
        }
        let _ = size;
    }

    /// Check for leaks and return a report. Returns `None` if no leaks found.
    ///
    /// A "leak" is any type where `allocs > deallocs`.
    pub fn check_leaks() -> Option<Vec<LeakReport>> {
        #[cfg(debug_assertions)]
        {
            let map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            let state = map.as_ref()?;
            let mut leaks = Vec::new();
            for (&tid, counters) in &state.counters {
                let allocs = counters.allocs.load(Ordering::Relaxed);
                let deallocs = counters.deallocs.load(Ordering::Relaxed);
                if allocs > deallocs {
                    leaks.push(LeakReport {
                        type_id: tid,
                        outstanding_allocs: allocs - deallocs,
                        outstanding_bytes: counters
                            .bytes_allocated
                            .load(Ordering::Relaxed)
                            .saturating_sub(counters.bytes_deallocated.load(Ordering::Relaxed)),
                    });
                }
            }
            if leaks.is_empty() { None } else { Some(leaks) }
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }

    /// Reset all counters. Useful between tests.
    pub fn reset() {
        #[cfg(debug_assertions)]
        {
            let mut map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            *map = None;
        }
    }

    /// Get a snapshot of memory usage metrics per type.
    pub fn snapshot() -> MemorySnapshot {
        #[cfg(debug_assertions)]
        {
            let map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            let mut entries = Vec::new();
            let mut global_peak_live: usize = 0;
            let mut global_peak_bytes: usize = 0;

            if let Some(state) = map.as_ref() {
                global_peak_live = state.peak_live_objects;
                global_peak_bytes = state.peak_bytes_in_use;

                for (&tid, counters) in &state.counters {
                    let allocs = counters.allocs.load(Ordering::Relaxed);
                    let deallocs = counters.deallocs.load(Ordering::Relaxed);
                    let bytes_in = counters.bytes_allocated.load(Ordering::Relaxed);
                    let bytes_out = counters.bytes_deallocated.load(Ordering::Relaxed);
                    entries.push(MemoryEntry {
                        type_id: tid,
                        total_allocs: allocs,
                        total_deallocs: deallocs,
                        live_objects: allocs.saturating_sub(deallocs),
                        bytes_in_use: bytes_in.saturating_sub(bytes_out),
                        bytes_total_allocated: bytes_in,
                        peak_bytes_in_use: counters.peak_bytes_in_use.load(Ordering::Relaxed),
                        peak_live_objects: counters.peak_live_objects.load(Ordering::Relaxed),
                    });
                }
            }
            MemorySnapshot {
                entries,
                peak_live_objects: global_peak_live,
                peak_bytes_in_use: global_peak_bytes,
            }
        }
        #[cfg(not(debug_assertions))]
        {
            MemorySnapshot {
                entries: Vec::new(),
                peak_live_objects: 0,
                peak_bytes_in_use: 0,
            }
        }
    }

    /// Take a frame snapshot and append to history.
    pub fn take_frame_snapshot() {
        #[cfg(debug_assertions)]
        {
            let snapshot = Self::snapshot();
            let mut map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = map.as_mut() {
                state.frame_snapshots.push(snapshot);
                if state.frame_snapshots.len() > state.max_snapshots {
                    state.frame_snapshots.remove(0);
                }
            }
        }
    }

    /// Get the history of frame snapshots.
    pub fn frame_snapshots() -> Vec<MemorySnapshot> {
        #[cfg(debug_assertions)]
        {
            let map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = map.as_ref() {
                state.frame_snapshots.clone()
            } else {
                Vec::new()
            }
        }
        #[cfg(not(debug_assertions))]
        {
            Vec::new()
        }
    }

    /// Get the peak live objects observed since last reset.
    pub fn peak_live_objects() -> usize {
        #[cfg(debug_assertions)]
        {
            let map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            map.as_ref().map(|s| s.peak_live_objects).unwrap_or(0)
        }
        #[cfg(not(debug_assertions))]
        {
            0
        }
    }

    /// Get the peak bytes in use observed since last reset.
    pub fn peak_bytes_in_use() -> usize {
        #[cfg(debug_assertions)]
        {
            let map = ALLOC_COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
            map.as_ref().map(|s| s.peak_bytes_in_use).unwrap_or(0)
        }
        #[cfg(not(debug_assertions))]
        {
            0
        }
    }
}

fn update_peak(peak: &AtomicUsize, value: usize) {
    let mut current = peak.load(Ordering::Relaxed);
    while value > current {
        match peak.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

/// Report for a single leaked type.
#[derive(Debug, Clone)]
pub struct LeakReport {
    pub type_id: TypeId,
    pub outstanding_allocs: usize,
    pub outstanding_bytes: usize,
}

/// A single entry in a memory snapshot.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub type_id: TypeId,
    pub total_allocs: usize,
    pub total_deallocs: usize,
    pub live_objects: usize,
    pub bytes_in_use: usize,
    pub bytes_total_allocated: usize,
    pub peak_bytes_in_use: usize,
    pub peak_live_objects: usize,
}

/// Snapshot of memory usage across all tracked types.
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub entries: Vec<MemoryEntry>,
    pub peak_live_objects: usize,
    pub peak_bytes_in_use: usize,
}

impl MemorySnapshot {
    /// Total live objects across all types.
    pub fn total_live_objects(&self) -> usize {
        self.entries.iter().map(|e| e.live_objects).sum()
    }

    /// Total bytes in use across all types.
    pub fn total_bytes_in_use(&self) -> usize {
        self.entries.iter().map(|e| e.bytes_in_use).sum()
    }

    /// Total bytes ever allocated (including freed).
    pub fn total_bytes_allocated(&self) -> usize {
        self.entries.iter().map(|e| e.bytes_total_allocated).sum()
    }
}

impl std::fmt::Display for MemorySnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Memory Snapshot:")?;
        writeln!(f, "  Total live objects: {}", self.total_live_objects())?;
        writeln!(f, "  Total bytes in use: {}", self.total_bytes_in_use())?;
        writeln!(
            f,
            "  Total bytes allocated (lifetime): {}",
            self.total_bytes_allocated()
        )?;
        writeln!(f, "  Peak live objects: {}", self.peak_live_objects)?;
        writeln!(f, "  Peak bytes in use: {}", self.peak_bytes_in_use)?;
        if !self.entries.is_empty() {
            writeln!(f, "  Per-type breakdown:")?;
            for entry in &self.entries {
                writeln!(
                    f,
                    "    {:?}: {} live (peak {}), {} bytes in use (peak {})",
                    entry.type_id,
                    entry.live_objects,
                    entry.peak_live_objects,
                    entry.bytes_in_use,
                    entry.peak_bytes_in_use,
                )?;
            }
        }
        Ok(())
    }
}

/// Compute the size of a type's layout, or 0 if zero-sized.
pub fn type_size<T: 'static>() -> usize {
    Layout::new::<T>().size()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_acquire_release() {
        let mut pool = Pool::<Vec<i32>>::with_capacity(4);
        assert_eq!(pool.len(), 4);

        let mut v = pool.acquire();
        assert_eq!(pool.len(), 3);
        v.push(1);
        v.push(2);
        pool.release(v);
        assert_eq!(pool.len(), 4);

        let v2 = pool.acquire();
        assert!(v2.is_empty(), "released vec should be cleared");
    }

    #[test]
    fn test_pool_exhaustion_creates_new() {
        let mut pool = Pool::<Vec<u32>>::with_capacity(1);
        let _a = pool.acquire();
        assert!(pool.is_empty());
        let _b = pool.acquire(); // should not panic
    }

    #[test]
    fn test_memory_tracker_basic() {
        MemoryTracker::reset();
        MemoryTracker::record_alloc::<u64>(8);
        MemoryTracker::record_alloc::<u64>(8);
        MemoryTracker::record_dealloc::<u64>(8);

        let snapshot = MemoryTracker::snapshot();
        assert_eq!(snapshot.total_live_objects(), 1);

        let leaks = MemoryTracker::check_leaks();
        assert!(leaks.is_some());
        let leaks = leaks.unwrap();
        assert_eq!(leaks.len(), 1);
        assert_eq!(leaks[0].outstanding_allocs, 1);
    }

    #[test]
    fn test_memory_tracker_no_leak() {
        MemoryTracker::reset();
        MemoryTracker::record_alloc::<u32>(4);
        MemoryTracker::record_dealloc::<u32>(4);

        assert!(MemoryTracker::check_leaks().is_none());
    }

    #[test]
    fn test_snapshot_display() {
        MemoryTracker::reset();
        MemoryTracker::record_alloc::<u64>(8);
        let snapshot = MemoryTracker::snapshot();
        let display = format!("{snapshot}");
        assert!(display.contains("Memory Snapshot"));
        assert!(display.contains("Total live objects"));
        assert!(display.contains("Peak live objects"));
        assert!(display.contains("Peak bytes in use"));
    }

    #[test]
    fn test_peak_tracking() {
        MemoryTracker::reset();
        MemoryTracker::record_alloc::<u64>(100);
        MemoryTracker::record_alloc::<u64>(200);
        assert_eq!(MemoryTracker::peak_live_objects(), 2);
        assert_eq!(MemoryTracker::peak_bytes_in_use(), 300);

        MemoryTracker::record_dealloc::<u64>(100);
        // Peak should still reflect the maximum observed
        assert_eq!(MemoryTracker::peak_live_objects(), 2);
        assert_eq!(MemoryTracker::peak_bytes_in_use(), 300);
    }

    #[test]
    fn test_snapshot_per_type_peaks() {
        MemoryTracker::reset();
        MemoryTracker::record_alloc::<u64>(100);
        MemoryTracker::record_alloc::<u64>(200);
        MemoryTracker::record_dealloc::<u64>(100);

        let snapshot = MemoryTracker::snapshot();
        let entry = &snapshot.entries[0];
        assert_eq!(entry.peak_live_objects, 2);
        assert_eq!(entry.peak_bytes_in_use, 300);
        assert_eq!(entry.live_objects, 1);
        assert_eq!(entry.bytes_in_use, 200);
    }
}
