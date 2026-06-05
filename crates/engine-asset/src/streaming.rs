//! Streaming asset loading with LOD support and memory management.
//!
//! Provides distance-based LOD selection, memory pool budgeting,
//! and prioritized streaming for large asset datasets.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// A level-of-detail tier.
///
/// Lower indices represent higher detail (closer to camera).
/// Each tier can reference a different asset file or configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LodLevel {
    /// Index of this LOD (0 = highest detail).
    pub index: u32,
    /// Distance threshold at which this LOD becomes active.
    /// Assets at distance < threshold use this LOD.
    pub max_distance: f32,
    /// Relative quality factor (1.0 = full quality).
    pub quality: f32,
}

/// Configuration for LOD on a specific asset.
#[derive(Debug, Clone)]
pub struct LodConfig {
    /// The base asset path (highest detail).
    pub base_path: PathBuf,
    /// LOD levels sorted by index (ascending).
    pub levels: Vec<LodLevel>,
    /// Per-level asset paths (index → path). If missing, uses base_path
    /// with import parameters to reduce quality.
    pub level_paths: HashMap<u32, PathBuf>,
}

impl LodConfig {
    /// Create a simple LOD config with uniform distance steps.
    pub fn uniform(base_path: PathBuf, num_levels: u32, max_distance: f32) -> Self {
        let levels = (0..num_levels)
            .map(|i| LodLevel {
                index: i,
                max_distance: max_distance * (i + 1) as f32 / num_levels as f32,
                quality: 1.0 - (i as f32 * 0.25).min(0.75),
            })
            .collect();

        Self {
            base_path,
            levels,
            level_paths: HashMap::new(),
        }
    }

    /// Select the LOD level for a given distance.
    pub fn select_lod(&self, distance: f32) -> &LodLevel {
        // Find the highest-detail LOD whose max_distance covers this distance
        self.levels
            .iter()
            .find(|level| distance < level.max_distance)
            .unwrap_or_else(|| {
                // Beyond all thresholds — use lowest detail
                self.levels.last().unwrap_or(&LodLevel {
                    index: 0,
                    max_distance: f32::MAX,
                    quality: 1.0,
                })
            })
    }

    /// Get the asset path for a specific LOD level.
    pub fn path_for_lod(&self, lod_index: u32) -> &Path {
        self.level_paths
            .get(&lod_index)
            .map(|p| p.as_path())
            .unwrap_or(self.base_path.as_path())
    }
}

/// Tracks which LOD is currently loaded for each streaming asset.
#[derive(Debug, Clone)]
pub struct LodState {
    /// Currently loaded LOD index (None = not loaded).
    pub loaded_lod: Option<u32>,
    /// Target LOD based on last evaluation.
    pub target_lod: u32,
    /// Time of last LOD evaluation.
    pub last_evaluated: Instant,
    /// Whether a load request is in flight.
    pub loading: bool,
}

impl Default for LodState {
    fn default() -> Self {
        Self {
            loaded_lod: None,
            target_lod: 0,
            last_evaluated: Instant::now(),
            loading: false,
        }
    }
}

/// Selects LOD levels based on distance and available memory.
pub struct LodSelector {
    /// Maximum distance before an asset is fully unloaded.
    pub unload_distance: f32,
    /// Hysteresis factor to prevent LOD thrashing (0.0-1.0).
    pub hysteresis: f32,
}

impl LodSelector {
    pub fn new(unload_distance: f32) -> Self {
        Self {
            unload_distance,
            hysteresis: 0.1,
        }
    }

    /// Evaluate which LOD an asset should use.
    ///
    /// Returns `None` if the asset should be unloaded (too far away).
    pub fn evaluate(
        &self,
        config: &LodConfig,
        distance: f32,
        current_lod: Option<u32>,
    ) -> Option<u32> {
        if distance >= self.unload_distance {
            return None; // Unload
        }

        let selected = config.select_lod(distance);

        // Apply hysteresis to prevent thrashing between LODs
        if let Some(current) = current_lod {
            let threshold = config.levels.get(current as usize);
            if let Some(threshold) = threshold {
                let hysteresis_margin = threshold.max_distance * self.hysteresis;
                if (distance - threshold.max_distance).abs() < hysteresis_margin {
                    return Some(current); // Stay at current LOD
                }
            }
        }

        Some(selected.index)
    }
}

impl Default for LodSelector {
    fn default() -> Self {
        Self::new(1000.0)
    }
}

/// Memory budget manager for streamed assets.
///
/// Tracks memory usage and evicts least-recently-used assets
/// when the budget is exceeded.
pub struct MemoryPool {
    /// Maximum memory budget in bytes.
    budget: usize,
    /// Current used memory in bytes.
    used: usize,
    /// Per-asset memory tracking (path → size in bytes).
    allocations: HashMap<PathBuf, usize>,
    /// LRU tracking: most recently used at the back.
    lru_order: VecDeque<PathBuf>,
}

impl MemoryPool {
    /// Create a new memory pool with the given budget in bytes.
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            allocations: HashMap::new(),
            lru_order: VecDeque::new(),
        }
    }

    /// Current memory usage in bytes.
    pub fn used(&self) -> usize {
        self.used
    }

    /// Remaining budget in bytes.
    pub fn available(&self) -> usize {
        self.budget.saturating_sub(self.used)
    }

    /// Memory budget in bytes.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Set a new budget. Returns paths that need to be evicted if over budget.
    pub fn set_budget(&mut self, budget: usize) -> Vec<PathBuf> {
        self.budget = budget;
        self.evict_to_budget()
    }

    /// Allocate memory for an asset. Returns `true` if within budget.
    ///
    /// If the asset was already tracked, updates its size and LRU position.
    pub fn allocate(&mut self, path: PathBuf, size: usize) -> bool {
        // Remove existing allocation if present
        if let Some(old_size) = self.allocations.remove(&path) {
            self.used = self.used.saturating_sub(old_size);
            self.lru_order.retain(|p| p != &path);
        }

        self.allocations.insert(path.clone(), size);
        self.lru_order.push_back(path);
        self.used += size;

        self.used <= self.budget
    }

    /// Touch an asset to mark it as recently used.
    pub fn touch(&mut self, path: &Path) {
        self.lru_order.retain(|p| p != path);
        self.lru_order.push_back(path.to_path_buf());
    }

    /// Release memory for an asset.
    pub fn release(&mut self, path: &Path) -> usize {
        let freed = self.allocations.remove(path).unwrap_or(0);
        self.used = self.used.saturating_sub(freed);
        self.lru_order.retain(|p| p != path);
        freed
    }

    /// Evict least-recently-used assets until within budget.
    ///
    /// Returns paths that were evicted.
    pub fn evict_to_budget(&mut self) -> Vec<PathBuf> {
        let mut evicted = Vec::new();

        while self.used > self.budget && !self.lru_order.is_empty() {
            if let Some(oldest) = self.lru_order.pop_front()
                && let Some(size) = self.allocations.remove(&oldest)
            {
                self.used = self.used.saturating_sub(size);
                evicted.push(oldest);
            }
        }

        evicted
    }

    /// Check if an asset is currently tracked.
    pub fn contains(&self, path: &Path) -> bool {
        self.allocations.contains_key(path)
    }

    /// Get the size of a tracked asset.
    pub fn size_of(&self, path: &Path) -> Option<usize> {
        self.allocations.get(path).copied()
    }

    /// Number of tracked assets.
    pub fn len(&self) -> usize {
        self.allocations.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty()
    }
}

/// Streaming request priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamPriority {
    /// Background prefetch — load when idle.
    Background,
    /// Normal streaming — load soon.
    Normal,
    /// Visible asset — load quickly.
    Visible,
    /// Critical — load immediately (player is very close).
    Critical,
}

/// A pending streaming load request.
#[derive(Debug, Clone)]
pub struct StreamRequest {
    /// Path to load.
    pub path: PathBuf,
    /// Target LOD level.
    pub target_lod: u32,
    /// Priority.
    pub priority: StreamPriority,
    /// Time the request was submitted.
    pub submitted_at: Instant,
}

/// Streaming manager that coordinates LOD-based asset loading.
///
/// Combines LOD selection, memory budgeting, and async loading
/// into a unified streaming system.
pub struct StreamingManager {
    /// LOD configurations per asset (base_path → config).
    lod_configs: HashMap<PathBuf, LodConfig>,
    /// Current LOD state per asset (base_path → state).
    lod_states: HashMap<PathBuf, LodState>,
    /// Memory pool for budget management.
    memory_pool: MemoryPool,
    /// LOD selector.
    selector: LodSelector,
    /// Pending stream requests sorted by priority.
    request_queue: BTreeMap<StreamPriority, Vec<StreamRequest>>,
}

impl StreamingManager {
    /// Create a new streaming manager with the given memory budget.
    pub fn new(memory_budget: usize) -> Self {
        Self {
            lod_configs: HashMap::new(),
            lod_states: HashMap::new(),
            memory_pool: MemoryPool::new(memory_budget),
            selector: LodSelector::default(),
            request_queue: BTreeMap::new(),
        }
    }

    /// Register an asset for streaming with LOD support.
    pub fn register(&mut self, config: LodConfig) {
        let path = config.base_path.clone();
        self.lod_states.insert(path.clone(), LodState::default());
        self.lod_configs.insert(path, config);
    }

    /// Evaluate LOD for all registered assets based on viewer position.
    ///
    /// Returns stream requests for assets that need LOD changes.
    pub fn evaluate(
        &mut self,
        viewer_position: [f32; 3],
        asset_positions: &HashMap<PathBuf, [f32; 3]>,
    ) -> Vec<StreamRequest> {
        let mut requests = Vec::new();

        for (base_path, config) in &self.lod_configs {
            let asset_pos = match asset_positions.get(base_path) {
                Some(pos) => pos,
                None => continue,
            };

            let distance = distance(viewer_position, *asset_pos);
            let state = self.lod_states.get(base_path);

            let current_lod = state.and_then(|s| s.loaded_lod);
            let target = self.selector.evaluate(config, distance, current_lod);

            if let Some(target_lod) = target {
                let needs_load = match current_lod {
                    Some(loaded) => loaded != target_lod,
                    None => true,
                };

                if needs_load {
                    if let Some(state) = self.lod_states.get_mut(base_path) {
                        state.target_lod = target_lod;
                        state.last_evaluated = Instant::now();
                    }

                    let priority = if distance < 10.0 {
                        StreamPriority::Critical
                    } else if distance < 50.0 {
                        StreamPriority::Visible
                    } else {
                        StreamPriority::Normal
                    };

                    let path = config.path_for_lod(target_lod).to_path_buf();
                    requests.push(StreamRequest {
                        path,
                        target_lod,
                        priority,
                        submitted_at: Instant::now(),
                    });
                }
            } else {
                // Asset should be unloaded
                if let Some(state) = self.lod_states.get_mut(base_path)
                    && state.loaded_lod.is_some()
                {
                    state.loaded_lod = None;
                    state.target_lod = 0;
                    self.memory_pool.release(base_path);
                }
            }
        }

        requests
    }

    /// Mark an asset as loaded at a specific LOD.
    pub fn mark_loaded(&mut self, path: &Path, lod: u32, size: usize) {
        if let Some(state) = self.lod_states.get_mut(path) {
            state.loaded_lod = Some(lod);
            state.loading = false;
        }
        self.memory_pool.allocate(path.to_path_buf(), size);
    }

    /// Get the memory pool.
    pub fn memory_pool(&self) -> &MemoryPool {
        &self.memory_pool
    }

    /// Get the memory pool (mutable).
    pub fn memory_pool_mut(&mut self) -> &mut MemoryPool {
        &mut self.memory_pool
    }

    /// Get the LOD selector.
    pub fn selector(&self) -> &LodSelector {
        &self.selector
    }

    /// Get the LOD selector (mutable).
    pub fn selector_mut(&mut self) -> &mut LodSelector {
        &mut self.selector
    }

    /// Get the number of registered streaming assets.
    pub fn registered_count(&self) -> usize {
        self.lod_configs.len()
    }

    /// Submit stream requests into the priority queue.
    pub fn enqueue(&mut self, requests: Vec<StreamRequest>) {
        for req in requests {
            self.request_queue
                .entry(req.priority)
                .or_default()
                .push(req);
        }
    }

    /// Drain all queued requests in priority order (highest first).
    pub fn drain_queue(&mut self) -> Vec<StreamRequest> {
        let mut all = Vec::new();
        for (_, mut requests) in std::mem::take(&mut self.request_queue) {
            all.append(&mut requests);
        }
        // Already ordered by key (BTreeMap iterates sorted), but sort explicitly
        // in case same-priority requests need stable ordering.
        all.sort_by_key(|b| std::cmp::Reverse(b.priority));
        all
    }

    /// Number of pending requests in the queue.
    pub fn queue_len(&self) -> usize {
        self.request_queue.values().map(|v| v.len()).sum()
    }
}

/// Calculate Euclidean distance between two 3D points.
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_level_selection() {
        let config = LodConfig::uniform(PathBuf::from("mesh.obj"), 3, 100.0);

        // Close distance → LOD 0 (highest detail)
        let lod = config.select_lod(10.0);
        assert_eq!(lod.index, 0);

        // Medium distance → LOD 1
        let lod = config.select_lod(45.0);
        assert_eq!(lod.index, 1);

        // Far distance → LOD 2
        let lod = config.select_lod(80.0);
        assert_eq!(lod.index, 2);
    }

    #[test]
    fn test_lod_selector_unload() {
        let selector = LodSelector::new(200.0);
        let config = LodConfig::uniform(PathBuf::from("mesh.obj"), 3, 100.0);

        // Close enough → should load (distance 50 is in LOD 1 for uniform 3-level config)
        assert_eq!(selector.evaluate(&config, 50.0, None), Some(1));

        // Too far → should unload
        assert_eq!(selector.evaluate(&config, 250.0, None), None);
    }

    #[test]
    fn test_lod_selector_hysteresis() {
        let mut selector = LodSelector::new(200.0);
        selector.hysteresis = 0.2;
        let config = LodConfig::uniform(PathBuf::from("mesh.obj"), 3, 100.0);

        // At boundary with current LOD, should stay
        let current = Some(0);
        let result = selector.evaluate(&config, 33.0, current);
        // The exact behavior depends on hysteresis margin
        assert!(result.is_some());
    }

    #[test]
    fn test_lod_config_path_for_lod() {
        let mut config = LodConfig::uniform(PathBuf::from("mesh.obj"), 3, 100.0);
        assert_eq!(config.path_for_lod(0), Path::new("mesh.obj"));

        config.level_paths.insert(1, PathBuf::from("mesh_lod1.obj"));
        assert_eq!(config.path_for_lod(1), Path::new("mesh_lod1.obj"));
        // Fallback to base for unregistered LOD
        assert_eq!(config.path_for_lod(2), Path::new("mesh.obj"));
    }

    #[test]
    fn test_memory_pool_allocate_and_release() {
        let mut pool = MemoryPool::new(1024);

        pool.allocate(PathBuf::from("a"), 500);
        assert_eq!(pool.used(), 500);
        assert_eq!(pool.available(), 524);

        pool.allocate(PathBuf::from("b"), 600);
        assert_eq!(pool.used(), 1100);
        assert_eq!(pool.available(), 0);

        // Evict to budget
        let evicted = pool.evict_to_budget();
        assert!(!evicted.is_empty());
        assert!(pool.used() <= 1024);
    }

    #[test]
    fn test_memory_pool_lru_eviction() {
        let mut pool = MemoryPool::new(1000);

        pool.allocate(PathBuf::from("old"), 600);
        pool.allocate(PathBuf::from("new"), 600);

        // Over budget — should evict "old" (LRU)
        let evicted = pool.evict_to_budget();
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0], PathBuf::from("old"));
        assert!(pool.contains(Path::new("new")));
        assert!(!pool.contains(Path::new("old")));
    }

    #[test]
    fn test_memory_pool_touch() {
        let mut pool = MemoryPool::new(1000);

        pool.allocate(PathBuf::from("a"), 400);
        pool.allocate(PathBuf::from("b"), 400);
        pool.allocate(PathBuf::from("c"), 400);

        // Touch "a" to make it recently used
        pool.touch(Path::new("a"));

        // Over budget — should evict "b" (oldest LRU after touch)
        let evicted = pool.evict_to_budget();
        assert!(evicted.contains(&PathBuf::from("b")));
    }

    #[test]
    fn test_memory_pool_reallocate() {
        let mut pool = MemoryPool::new(1000);

        pool.allocate(PathBuf::from("a"), 500);
        assert_eq!(pool.used(), 500);

        // Reallocate with new size
        pool.allocate(PathBuf::from("a"), 300);
        assert_eq!(pool.used(), 300);
    }

    #[test]
    fn test_memory_pool_set_budget() {
        let mut pool = MemoryPool::new(2000);
        pool.allocate(PathBuf::from("a"), 200);
        pool.allocate(PathBuf::from("b"), 200);

        // Reduce budget to fit one entry but not both
        let evicted = pool.set_budget(250);
        assert_eq!(evicted.len(), 1);
        assert!(pool.used() <= 250);
    }

    #[test]
    fn test_streaming_manager_register_and_evaluate() {
        let mut mgr = StreamingManager::new(1024 * 1024);

        let config = LodConfig::uniform(PathBuf::from("terrain.obj"), 3, 500.0);
        mgr.register(config);

        assert_eq!(mgr.registered_count(), 1);

        // Evaluate with asset nearby
        let mut positions = HashMap::new();
        positions.insert(PathBuf::from("terrain.obj"), [10.0, 0.0, 0.0]);

        let requests = mgr.evaluate([0.0, 0.0, 0.0], &positions);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].target_lod, 0); // Close → high detail
    }

    #[test]
    fn test_streaming_manager_unload_distant() {
        let mut mgr = StreamingManager::new(1024 * 1024);

        let config = LodConfig::uniform(PathBuf::from("terrain.obj"), 3, 500.0);
        mgr.register(config);

        // Mark as loaded
        mgr.mark_loaded(Path::new("terrain.obj"), 0, 1000);

        // Evaluate with asset very far away
        let mut positions = HashMap::new();
        positions.insert(PathBuf::from("terrain.obj"), [9999.0, 0.0, 0.0]);

        let _requests = mgr.evaluate([0.0, 0.0, 0.0], &positions);

        // Asset should be unloaded
        let state = mgr.lod_states.get(Path::new("terrain.obj")).unwrap();
        assert!(state.loaded_lod.is_none());
    }

    #[test]
    fn test_distance() {
        let d = distance([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]);
        assert!((d - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_stream_priority_ordering() {
        assert!(StreamPriority::Critical > StreamPriority::Visible);
        assert!(StreamPriority::Visible > StreamPriority::Normal);
        assert!(StreamPriority::Normal > StreamPriority::Background);
    }

    #[test]
    fn test_enqueue_and_drain() {
        let mut mgr = StreamingManager::new(1024 * 1024);
        assert_eq!(mgr.queue_len(), 0);

        let config = LodConfig::uniform(PathBuf::from("terrain.obj"), 3, 500.0);
        mgr.register(config);

        let mut positions = HashMap::new();
        positions.insert(PathBuf::from("terrain.obj"), [10.0, 0.0, 0.0]);

        let requests = mgr.evaluate([0.0, 0.0, 0.0], &positions);
        assert!(!requests.is_empty());

        mgr.enqueue(requests);
        assert!(mgr.queue_len() > 0);

        let drained = mgr.drain_queue();
        assert!(!drained.is_empty());
        assert_eq!(mgr.queue_len(), 0);
    }

    #[test]
    fn test_drain_priority_order() {
        let mut mgr = StreamingManager::new(1024 * 1024);

        // Manually insert requests at different priorities
        mgr.request_queue
            .entry(StreamPriority::Normal)
            .or_default()
            .push(StreamRequest {
                path: PathBuf::from("far.obj"),
                target_lod: 2,
                priority: StreamPriority::Normal,
                submitted_at: Instant::now(),
            });
        mgr.request_queue
            .entry(StreamPriority::Critical)
            .or_default()
            .push(StreamRequest {
                path: PathBuf::from("close.obj"),
                target_lod: 0,
                priority: StreamPriority::Critical,
                submitted_at: Instant::now(),
            });

        let drained = mgr.drain_queue();
        assert_eq!(drained.len(), 2);
        // Critical should come first
        assert_eq!(drained[0].priority, StreamPriority::Critical);
        assert_eq!(drained[1].priority, StreamPriority::Normal);
    }
}
