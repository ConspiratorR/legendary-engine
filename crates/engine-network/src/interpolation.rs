//! State snapshot interpolation, client-side prediction, and reconciliation.
//!
//! Provides smooth visual interpolation between server snapshots, local
//! prediction of player inputs for responsiveness, and reconciliation
//! when the server sends corrections.

use crate::message::EntityComponentData;
use crate::snapshot::WorldSnapshot;

/// A ring buffer of snapshots for interpolation.
#[derive(Debug)]
pub struct SnapshotBuffer {
    /// Stored snapshots ordered by tick.
    snapshots: Vec<WorldSnapshot>,
    /// Maximum number of snapshots to store.
    capacity: usize,
}

impl SnapshotBuffer {
    /// Create a new snapshot buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            snapshots: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a snapshot into the buffer. Drops oldest if at capacity.
    pub fn push(&mut self, snapshot: WorldSnapshot) {
        if self.snapshots.len() >= self.capacity {
            self.snapshots.remove(0);
        }
        self.snapshots.push(snapshot);
    }

    /// Get the two snapshots that bracket the given interpolation tick.
    ///
    /// Returns `(older, newer, t)` where `t` is the interpolation factor [0, 1].
    /// Returns `None` if fewer than 2 snapshots are available.
    pub fn get_interpolation_pair(
        &self,
        tick: f64,
    ) -> Option<(&WorldSnapshot, &WorldSnapshot, f32)> {
        if self.snapshots.len() < 2 {
            return None;
        }

        // Find the pair of snapshots that bracket the tick
        for i in 0..self.snapshots.len() - 1 {
            let older = &self.snapshots[i];
            let newer = &self.snapshots[i + 1];

            let older_tick = older.tick as f64;
            let newer_tick = newer.tick as f64;

            if tick >= older_tick && tick <= newer_tick {
                let range = newer_tick - older_tick;
                let t = if range > 0.0 {
                    ((tick - older_tick) / range) as f32
                } else {
                    0.0
                };
                return Some((older, newer, t.clamp(0.0, 1.0)));
            }
        }

        // If tick is past all snapshots, return the last pair with t=1.0
        let len = self.snapshots.len();
        Some((&self.snapshots[len - 2], &self.snapshots[len - 1], 1.0))
    }

    /// Get the latest snapshot.
    pub fn latest(&self) -> Option<&WorldSnapshot> {
        self.snapshots.last()
    }

    /// Get the number of stored snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Clear all stored snapshots.
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}

/// Trait for types that can be linearly interpolated.
///
/// Implement this on component types that should be smoothly interpolated
/// between snapshot states on the client.
pub trait Interpolatable: Clone {
    /// Linearly interpolate between `self` and `other` by factor `t` in [0, 1].
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

/// Helper to interpolate raw component bytes using nearest-neighbor.
///
/// Returns `a` when `t < 0.5`, `b` otherwise. For smooth interpolation,
/// implement `Interpolatable` on your component type and use typed systems.
pub fn interpolate_nearest<T: Clone>(a: &T, b: &T, t: f32) -> T {
    if t < 0.5 { a.clone() } else { b.clone() }
}

/// Interpolation engine that smooths entity state between snapshots.
#[derive(Debug)]
pub struct Interpolator {
    /// Configurable interpolation delay in ticks (default: 2).
    pub interpolation_delay: u64,
    /// Snapshot buffer for interpolation.
    buffer: SnapshotBuffer,
    /// Current render tick (behind server tick by `interpolation_delay`).
    render_tick: f64,
}

impl Interpolator {
    /// Create a new interpolator with the given delay and buffer capacity.
    pub fn new(interpolation_delay: u64, buffer_capacity: usize) -> Self {
        Self {
            interpolation_delay,
            buffer: SnapshotBuffer::new(buffer_capacity),
            render_tick: 0.0,
        }
    }

    /// Push a new server snapshot.
    pub fn push_snapshot(&mut self, snapshot: WorldSnapshot) {
        self.buffer.push(snapshot);
    }

    /// Update the render tick based on the latest server tick.
    ///
    /// Call this each frame. The render tick lags behind the server tick
    /// by `interpolation_delay` ticks.
    pub fn update(&mut self) {
        if let Some(latest) = self.buffer.latest() {
            let target = (latest.tick as f64) - (self.interpolation_delay as f64);
            if target > self.render_tick {
                self.render_tick = target;
            }
        }
    }

    /// Get the interpolated state for a given entity and component type.
    ///
    /// Returns the interpolated component data bytes, or `None` if
    /// interpolation data is not available.
    pub fn interpolate_entity(&self, entity_index: u32, type_hash: u64) -> Option<Vec<u8>> {
        let (older, newer, t) = self.buffer.get_interpolation_pair(self.render_tick)?;

        let older_data = older.entities.get(&entity_index)?.get(&type_hash)?;
        let newer_data = newer.entities.get(&entity_index)?.get(&type_hash)?;

        if older_data == newer_data {
            return Some(older_data.clone());
        }

        // For raw bytes, we do nearest-neighbor; implementors should use
        // typed interpolation via `Interpolatable` for smooth results.
        if t < 0.5 {
            Some(older_data.clone())
        } else {
            Some(newer_data.clone())
        }
    }

    /// Interpolate all entities in the current frame into an `EntityComponentData`.
    pub fn interpolate_all(&self) -> EntityComponentData {
        let (older, newer, t) = match self.buffer.get_interpolation_pair(self.render_tick) {
            Some(pair) => pair,
            None => return Vec::new(),
        };

        let mut result = Vec::new();

        // Collect all entity indices from both snapshots
        let mut all_entities: Vec<u32> = older.entities.keys().copied().collect();
        for key in newer.entities.keys() {
            if !all_entities.contains(key) {
                all_entities.push(*key);
            }
        }

        for entity_idx in all_entities {
            let mut components = Vec::new();
            let older_comps = older.entities.get(&entity_idx);
            let newer_comps = newer.entities.get(&entity_idx);

            let mut all_hashes: Vec<u64> = older_comps
                .map(|c| c.keys().copied().collect())
                .unwrap_or_default();
            if let Some(nc) = newer_comps {
                for h in nc.keys() {
                    if !all_hashes.contains(h) {
                        all_hashes.push(*h);
                    }
                }
            }

            for type_hash in all_hashes {
                let older_data = older_comps.and_then(|c| c.get(&type_hash));
                let newer_data = newer_comps.and_then(|c| c.get(&type_hash));

                let data = match (older_data, newer_data) {
                    (Some(a), Some(b)) => {
                        if a == b || t < 0.5 {
                            a.clone()
                        } else {
                            b.clone()
                        }
                    }
                    (Some(a), None) => a.clone(),
                    (None, Some(b)) => b.clone(),
                    (None, None) => continue,
                };

                components.push((type_hash, data));
            }

            if !components.is_empty() {
                result.push((entity_idx, components));
            }
        }

        result
    }

    /// Get the current render tick.
    pub fn render_tick(&self) -> f64 {
        self.render_tick
    }

    /// Get the snapshot buffer.
    pub fn buffer(&self) -> &SnapshotBuffer {
        &self.buffer
    }

    /// Get a mutable reference to the snapshot buffer.
    pub fn buffer_mut(&mut self) -> &mut SnapshotBuffer {
        &mut self.buffer
    }
}

impl Default for Interpolator {
    fn default() -> Self {
        Self::new(2, 10)
    }
}

/// A predicted state entry for client-side prediction.
#[derive(Debug, Clone)]
pub struct PredictedState {
    /// Client tick when this prediction was made.
    pub client_tick: u64,
    /// The predicted entity component data.
    pub entities: EntityComponentData,
}

/// Client-side prediction and server reconciliation manager.
#[derive(Debug)]
pub struct PredictionManager {
    /// Buffer of predicted states.
    predictions: Vec<PredictedState>,
    /// Maximum number of predictions to store.
    max_predictions: usize,
    /// Threshold for snapping vs smoothing corrections.
    pub snap_threshold: f32,
    /// Number of frames to smooth a small correction over.
    pub smooth_frames: u32,
    /// Current smoothing state.
    smoothing: Option<SmoothingState>,
}

/// State for smooth correction interpolation.
#[derive(Debug)]
struct SmoothingState {
    /// The target corrected state.
    target: EntityComponentData,
    /// The state at the start of smoothing.
    start: EntityComponentData,
    /// Current frame in the smoothing process.
    current_frame: u32,
    /// Total frames for smoothing.
    total_frames: u32,
}

impl PredictionManager {
    /// Create a new prediction manager.
    pub fn new(max_predictions: usize) -> Self {
        Self {
            predictions: Vec::with_capacity(max_predictions),
            max_predictions,
            snap_threshold: 10.0,
            smooth_frames: 5,
            smoothing: None,
        }
    }

    /// Store a predicted state.
    pub fn store_prediction(&mut self, client_tick: u64, entities: EntityComponentData) {
        let state = PredictedState {
            client_tick,
            entities,
        };
        if self.predictions.len() >= self.max_predictions {
            self.predictions.remove(0);
        }
        self.predictions.push(state);
    }

    /// Reconcile with a server correction.
    ///
    /// When the server sends a correction for a specific tick, find the
    /// corresponding predicted state, compute the difference, and decide
    /// whether to snap or smooth.
    ///
    /// Returns the corrected entity data to apply.
    pub fn reconcile(
        &mut self,
        correction_tick: u64,
        server_state: &EntityComponentData,
        _pending_inputs: &[(u64, Vec<u8>)],
    ) -> EntityComponentData {
        // Remove predictions up to and including the correction tick
        self.predictions.retain(|p| p.client_tick > correction_tick);

        // Compute diff magnitude between prediction and server state
        let diff = self.compute_diff_magnitude(server_state);

        if diff > self.snap_threshold {
            // Large diff: snap immediately
            self.smoothing = None;
            server_state.clone()
        } else {
            // Small diff: smooth over several frames
            let start = self
                .get_current_state()
                .unwrap_or_else(|| server_state.clone());
            self.smoothing = Some(SmoothingState {
                target: server_state.clone(),
                start,
                current_frame: 0,
                total_frames: self.smooth_frames,
            });
            // Return the start of smoothing; subsequent calls to `update_smoothing`
            // will interpolate toward the target.
            server_state.clone()
        }
    }

    /// Update smoothing interpolation. Call each frame during smooth correction.
    ///
    /// Returns the interpolated entity data, or `None` if not smoothing.
    pub fn update_smoothing(&mut self) -> Option<EntityComponentData> {
        let (current_frame, total_frames, start, target) = {
            let state = self.smoothing.as_mut()?;
            state.current_frame += 1;
            if state.current_frame >= state.total_frames {
                let result = state.target.clone();
                self.smoothing = None;
                return Some(result);
            }
            (
                state.current_frame,
                state.total_frames,
                state.start.clone(),
                state.target.clone(),
            )
        };

        let t = current_frame as f32 / total_frames as f32;
        Some(self.interpolate_entities(&start, &target, t))
    }

    /// Check if currently smoothing a correction.
    pub fn is_smoothing(&self) -> bool {
        self.smoothing.is_some()
    }

    /// Get the number of pending predictions.
    pub fn pending_count(&self) -> usize {
        self.predictions.len()
    }

    /// Clear all predictions.
    pub fn clear(&mut self) {
        self.predictions.clear();
        self.smoothing = None;
    }

    fn compute_diff_magnitude(&self, server_state: &EntityComponentData) -> f32 {
        // Simple diff: count differing bytes as a rough magnitude
        let predicted = match self.predictions.last() {
            Some(p) => &p.entities,
            None => return 0.0,
        };

        let mut diff_count = 0usize;
        for (entity_idx, server_comps) in server_state {
            if let Some(pred_comps) = predicted.iter().find(|(eid, _)| eid == entity_idx) {
                for (type_hash, server_data) in server_comps {
                    if let Some(pred_data) = pred_comps.1.iter().find(|(th, _)| th == type_hash) {
                        for (a, b) in pred_data.1.iter().zip(server_data.iter()) {
                            if a != b {
                                diff_count += 1;
                            }
                        }
                        // Account for length differences
                        diff_count += pred_data.1.len().abs_diff(server_data.len());
                    } else {
                        diff_count += server_data.len();
                    }
                }
            } else {
                diff_count += server_comps.iter().map(|(_, d)| d.len()).sum::<usize>();
            }
        }
        diff_count as f32
    }

    fn get_current_state(&self) -> Option<EntityComponentData> {
        self.predictions.last().map(|p| p.entities.clone())
    }

    fn interpolate_entities(
        &self,
        a: &EntityComponentData,
        b: &EntityComponentData,
        t: f32,
    ) -> EntityComponentData {
        let mut result = Vec::new();

        for (entity_idx, b_comps) in b {
            let a_comps = a.iter().find(|(eid, _)| eid == entity_idx);

            let mut components = Vec::new();
            for (type_hash, b_data) in b_comps {
                let data = if let Some((_, a_data)) =
                    a_comps.and_then(|(_, c)| c.iter().find(|(th, _)| th == type_hash))
                {
                    // Byte-level interpolation
                    interpolate_bytes(a_data, b_data, t)
                } else {
                    b_data.clone()
                };
                components.push((*type_hash, data));
            }
            result.push((*entity_idx, components));
        }

        result
    }
}

impl Default for PredictionManager {
    fn default() -> Self {
        Self::new(120)
    }
}

/// Interpolate between two byte arrays by factor `t`.
fn interpolate_bytes(a: &[u8], b: &[u8], t: f32) -> Vec<u8> {
    let len = a.len().min(b.len());
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let val = a[i] as f32 * (1.0 - t) + b[i] as f32 * t;
        result.push(val.round() as u8);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(tick: u64) -> WorldSnapshot {
        let mut snap = WorldSnapshot::new(tick);
        snap.add_component(0, 1, vec![0, 0, 0, 64]); // 2.0f32 LE
        snap.add_component(1, 1, vec![0, 0, 0, 65]);
        snap
    }

    fn make_snapshot_with(tick: u64, entity: u32, data: Vec<u8>) -> WorldSnapshot {
        let mut snap = WorldSnapshot::new(tick);
        snap.add_component(entity, 1, data);
        snap
    }

    #[test]
    fn test_snapshot_buffer_push_and_len() {
        let mut buf = SnapshotBuffer::new(3);
        assert!(buf.is_empty());

        buf.push(make_snapshot(1));
        buf.push(make_snapshot(2));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_snapshot_buffer_capacity() {
        let mut buf = SnapshotBuffer::new(3);
        buf.push(make_snapshot(1));
        buf.push(make_snapshot(2));
        buf.push(make_snapshot(3));
        buf.push(make_snapshot(4)); // drops tick 1

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.latest().unwrap().tick, 4);
    }

    #[test]
    fn test_snapshot_buffer_interpolation_pair() {
        let mut buf = SnapshotBuffer::new(10);
        buf.push(make_snapshot(10));
        buf.push(make_snapshot(20));
        buf.push(make_snapshot(30));

        let (older, newer, t) = buf.get_interpolation_pair(15.0).unwrap();
        assert_eq!(older.tick, 10);
        assert_eq!(newer.tick, 20);
        assert!((t - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_snapshot_buffer_interpolation_pair_exact() {
        let mut buf = SnapshotBuffer::new(10);
        buf.push(make_snapshot(10));
        buf.push(make_snapshot(20));

        let (older, newer, t) = buf.get_interpolation_pair(10.0).unwrap();
        assert_eq!(older.tick, 10);
        assert_eq!(newer.tick, 20);
        assert!((t - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_snapshot_buffer_too_few() {
        let mut buf = SnapshotBuffer::new(10);
        buf.push(make_snapshot(10));
        assert!(buf.get_interpolation_pair(10.0).is_none());
    }

    #[test]
    fn test_interpolator_push_and_update() {
        let mut interp = Interpolator::new(2, 10);
        interp.push_snapshot(make_snapshot(10));
        interp.push_snapshot(make_snapshot(12));
        interp.update();

        // Render tick should be 12 - 2 = 10
        assert!((interp.render_tick() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_interpolator_interpolate_entity() {
        let mut interp = Interpolator::new(0, 10);
        interp.push_snapshot(make_snapshot_with(10, 0, vec![10, 20, 30]));
        interp.push_snapshot(make_snapshot_with(20, 0, vec![40, 50, 60]));
        interp.update();

        let result = interp.interpolate_entity(0, 1);
        assert!(result.is_some());
    }

    #[test]
    fn test_interpolator_interpolate_entity_missing() {
        let mut interp = Interpolator::new(0, 10);
        interp.push_snapshot(make_snapshot(10));
        interp.push_snapshot(make_snapshot(20));
        interp.update();

        // Entity 999 doesn't exist
        assert!(interp.interpolate_entity(999, 1).is_none());
    }

    #[test]
    fn test_interpolator_interpolate_all() {
        let mut interp = Interpolator::new(0, 10);
        interp.push_snapshot(make_snapshot(10));
        interp.push_snapshot(make_snapshot(20));
        interp.update();

        let all = interp.interpolate_all();
        assert!(!all.is_empty());
    }

    #[test]
    fn test_interpolator_empty() {
        let interp = Interpolator::default();
        assert!(interp.interpolate_all().is_empty());
    }

    #[test]
    fn test_prediction_manager_store_and_count() {
        let mut pm = PredictionManager::new(10);
        pm.store_prediction(1, vec![(0, vec![(1, vec![1, 2, 3])])]);
        pm.store_prediction(2, vec![(0, vec![(1, vec![4, 5, 6])])]);

        assert_eq!(pm.pending_count(), 2);
    }

    #[test]
    fn test_prediction_manager_reconcile_snap() {
        let mut pm = PredictionManager::new(10);
        pm.snap_threshold = 0.0; // always snap
        pm.store_prediction(1, vec![(0, vec![(1, vec![1, 2, 3])])]);

        let server_state = vec![(0, vec![(1, vec![100, 200, 255])])];
        let result = pm.reconcile(1, &server_state, &[]);
        assert_eq!(result, server_state);
    }

    #[test]
    fn test_prediction_manager_reconcile_smooth() {
        let mut pm = PredictionManager::new(10);
        pm.snap_threshold = 1000.0; // always smooth
        pm.smooth_frames = 3;
        pm.store_prediction(1, vec![(0, vec![(1, vec![1, 2, 3])])]);

        let server_state = vec![(0, vec![(1, vec![4, 5, 6])])];
        pm.reconcile(1, &server_state, &[]);

        assert!(pm.is_smoothing());
    }

    #[test]
    fn test_prediction_manager_update_smoothing() {
        let mut pm = PredictionManager::new(10);
        pm.snap_threshold = 1000.0;
        pm.smooth_frames = 2;
        pm.store_prediction(1, vec![(0, vec![(1, vec![0, 0, 0])])]);

        let server_state = vec![(0, vec![(1, vec![100, 100, 100])])];
        pm.reconcile(1, &server_state, &[]);

        let r1 = pm.update_smoothing();
        assert!(r1.is_some());

        let r2 = pm.update_smoothing();
        assert!(r2.is_some());
        assert!(!pm.is_smoothing()); // done after 2 frames
    }

    #[test]
    fn test_prediction_manager_clear() {
        let mut pm = PredictionManager::new(10);
        pm.store_prediction(1, vec![(0, vec![(1, vec![1])])]);
        pm.clear();
        assert_eq!(pm.pending_count(), 0);
    }

    #[test]
    fn test_interpolate_bytes() {
        let a = vec![0, 0, 0, 0];
        let b = vec![100, 100, 100, 100];
        let mid = interpolate_bytes(&a, &b, 0.5);
        assert_eq!(mid, vec![50, 50, 50, 50]);
    }

    #[test]
    fn test_interpolate_bytes_zero() {
        let a = vec![10, 20, 30];
        let b = vec![40, 50, 60];
        let result = interpolate_bytes(&a, &b, 0.0);
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn test_interpolate_bytes_one() {
        let a = vec![10, 20, 30];
        let b = vec![40, 50, 60];
        let result = interpolate_bytes(&a, &b, 1.0);
        assert_eq!(result, vec![40, 50, 60]);
    }
}
