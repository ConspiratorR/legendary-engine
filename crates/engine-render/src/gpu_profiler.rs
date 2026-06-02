use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// ---------------------------------------------------------------------------
// GPU Profiler
// ---------------------------------------------------------------------------

/// GPU profiler using wgpu timestamp queries for measuring render pass
/// and pipeline execution times on the GPU.
///
/// # Timestamp Query Workflow
///
/// 1. Call [`begin_frame`](GpuProfiler::begin_frame) to get a query set index.
/// 2. Write timestamps at the start/end of each render pass using
///    [`write_timestamp`](GpuProfiler::write_timestamp).
/// 3. Call [`end_frame`](GpuProfiler::end_frame) after submitting commands.
/// 4. Resolve queries and read back timing data via [`resolve`](GpuProfiler::resolve).
///
/// The profiler manages a ring buffer of query sets and staging buffers
/// so the GPU pipeline is never stalled waiting for readback.
pub struct GpuProfiler {
    device: Arc<wgpu::Device>,
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    max_queries: u32,
    next_query: u32,
    current_frame_queries: Vec<QueryRegion>,
    frame_history: Vec<GpuFrameRecord>,
    max_frames: usize,
    frame_number: u64,
    timestamp_period: f32,
    pending_resolve: bool,
}

/// A named region of GPU time between two timestamp queries.
#[derive(Debug, Clone)]
struct QueryRegion {
    name: String,
    start_query: u32,
    end_query: u32,
}

/// GPU timing data for a single frame.
#[derive(Debug, Clone)]
pub struct GpuFrameRecord {
    pub frame_number: u64,
    pub pass_timings: HashMap<String, Duration>,
}

/// Accumulated GPU timing statistics for a named pass.
#[derive(Debug, Clone)]
pub struct GpuPassStats {
    pub name: String,
    pub total_time: Duration,
    pub sample_count: u64,
    pub peak_time: Duration,
}

impl GpuPassStats {
    pub fn average_time(&self) -> Duration {
        if self.sample_count == 0 {
            Duration::ZERO
        } else {
            self.total_time / self.sample_count as u32
        }
    }
}

impl GpuProfiler {
    /// Create a new GPU profiler.
    ///
    /// `timestamp_period` is the nanoseconds per timestamp tick, obtained from
    /// [`wgpu::Queue::get_timestamp_period`].
    pub fn new(
        device: Arc<wgpu::Device>,
        timestamp_period: f32,
        max_queries: u32,
        max_frames: usize,
    ) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpu_profiler_queries"),
            ty: wgpu::QueryType::Timestamp,
            count: max_queries,
        });

        let buffer_size = (max_queries as u64) * 8; // u64 per timestamp
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu_profiler_resolve"),
            size: buffer_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu_profiler_staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            query_set,
            resolve_buffer,
            staging_buffer,
            max_queries,
            next_query: 0,
            current_frame_queries: Vec::new(),
            frame_history: Vec::with_capacity(max_frames),
            max_frames,
            frame_number: 0,
            timestamp_period,
            pending_resolve: false,
        }
    }

    /// Begin a new GPU frame. Resets the query counter for this frame.
    pub fn begin_frame(&mut self) {
        self.next_query = 0;
        self.current_frame_queries.clear();
    }

    /// Write a timestamp at the current point in a command encoder.
    ///
    /// Returns the query index used. Call this at the start and end of
    /// a render pass, then use [`end_pass`](GpuProfiler::end_pass) to
    /// register the named region.
    pub fn write_timestamp(&mut self, encoder: &mut wgpu::CommandEncoder) -> u32 {
        let idx = self.next_query;
        self.next_query += 1;
        if idx < self.max_queries {
            encoder.write_timestamp(&self.query_set, idx);
        }
        idx
    }

    /// Register a named pass with start/end query indices.
    pub fn end_pass(&mut self, name: &str, start_query: u32, end_query: u32) {
        self.current_frame_queries.push(QueryRegion {
            name: name.to_string(),
            start_query,
            end_query,
        });
    }

    /// End the current frame and schedule query resolution.
    ///
    /// Call this after submitting the command buffers that contain
    /// the timestamp writes.
    pub fn end_frame(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.next_query == 0 {
            self.frame_number += 1;
            return;
        }

        let query_count = self.next_query.min(self.max_queries);
        encoder.resolve_query_set(&self.query_set, 0..query_count, &self.resolve_buffer, 0);
        self.pending_resolve = true;
    }

    /// Read back resolved GPU timestamps and compute pass durations.
    ///
    /// This should be called after the GPU work from [`end_frame`](GpuProfiler::end_frame)
    /// has completed. The `queue` is used to copy from the resolve buffer
    /// to the mappable staging buffer.
    pub fn resolve(&mut self, queue: &wgpu::Queue) {
        if !self.pending_resolve || self.next_query == 0 {
            return;
        }

        let buffer_size = (self.next_query as u64).min(self.max_queries as u64) * 8;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpu_profiler_resolve_encoder"),
            });

        encoder.copy_buffer_to_buffer(
            &self.resolve_buffer,
            0,
            &self.staging_buffer,
            0,
            buffer_size,
        );
        queue.submit([encoder.finish()]);

        self.pending_resolve = false;
    }

    /// Map the staging buffer and extract timing data.
    ///
    /// Returns a buffer mapping guard. After the GPU work has completed,
    /// call [`GpuProfiler::read_timestamps`] to extract the raw u64 timestamps.
    pub fn map_staging(&self) -> wgpu::BufferSlice<'_> {
        self.staging_buffer.slice(..)
    }

    /// Read resolved timestamps from a mapped staging buffer.
    ///
    /// Call this after the staging buffer has been mapped. The returned
    /// slice contains raw u64 timestamp values. Use [`ticks_to_duration`](GpuProfiler::ticks_to_duration)
    /// to convert deltas to wall-clock time.
    pub fn read_timestamps(data: &[u8]) -> &[u64] {
        bytemuck::cast_slice(data)
    }

    /// Compute pass timings from raw timestamps and registered query regions.
    pub fn compute_timings(&self, timestamps: &[u64]) -> HashMap<String, Duration> {
        let mut timings = HashMap::new();
        for region in &self.current_frame_queries {
            if (region.end_query as usize) < timestamps.len()
                && (region.start_query as usize) < timestamps.len()
            {
                let start = timestamps[region.start_query as usize];
                let end = timestamps[region.end_query as usize];
                if end > start {
                    let duration = self.ticks_to_duration(end - start);
                    *timings.entry(region.name.clone()).or_insert(Duration::ZERO) += duration;
                }
            }
        }
        timings
    }

    /// Get the maximum frame history size.
    pub fn max_frames(&self) -> usize {
        self.max_frames
    }

    /// Get the frame history.
    pub fn frame_history(&self) -> &[GpuFrameRecord] {
        &self.frame_history
    }

    /// Get the current frame number.
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Get the query set for direct use in render passes.
    pub fn query_set(&self) -> &wgpu::QuerySet {
        &self.query_set
    }

    /// Get the timestamp period (nanoseconds per tick).
    pub fn timestamp_period(&self) -> f32 {
        self.timestamp_period
    }

    /// Convert a raw timestamp delta to a [`Duration`].
    pub fn ticks_to_duration(&self, ticks: u64) -> Duration {
        Duration::from_nanos((ticks as f32 * self.timestamp_period) as u64)
    }

    /// Get accumulated stats for all named passes across frame history.
    pub fn pass_stats(&self) -> HashMap<String, GpuPassStats> {
        let mut stats: HashMap<String, GpuPassStats> = HashMap::new();
        for frame in &self.frame_history {
            for (name, &duration) in &frame.pass_timings {
                let entry = stats.entry(name.clone()).or_insert_with(|| GpuPassStats {
                    name: name.clone(),
                    total_time: Duration::ZERO,
                    sample_count: 0,
                    peak_time: Duration::ZERO,
                });
                entry.total_time += duration;
                entry.sample_count += 1;
                if duration > entry.peak_time {
                    entry.peak_time = duration;
                }
            }
        }
        stats
    }

    /// Print a formatted summary of GPU performance to stdout.
    pub fn print_stats(&self) {
        println!("\n=== GPU Performance Statistics ===");
        println!("Frames profiled: {}", self.frame_history.len());

        let stats = self.pass_stats();
        if stats.is_empty() {
            println!("No GPU pass data collected.");
            return;
        }

        let mut sorted: Vec<_> = stats.values().collect();
        sorted.sort_by_key(|s| std::cmp::Reverse(s.total_time));

        println!("\nRender Pass Breakdown:");
        for stat in &sorted {
            println!(
                "  {}: avg {:.3}ms, peak {:.3}ms ({} samples)",
                stat.name,
                stat.average_time().as_secs_f32() * 1000.0,
                stat.peak_time.as_secs_f32() * 1000.0,
                stat.sample_count,
            );
        }
        println!();
    }
}

// ---------------------------------------------------------------------------
// GPU Frame Timestamp Helper
// ---------------------------------------------------------------------------

/// Helper for embedding GPU timestamps into a render pass.
///
/// Use [`GpuPassTimer::begin`] to start timing a pass, then
/// [`GpuPassTimer::end`] when the pass is finished.
pub struct GpuPassTimer {
    name: String,
    start_query: u32,
}

impl GpuPassTimer {
    /// Begin timing a named GPU pass.
    pub fn begin(
        profiler: &mut GpuProfiler,
        encoder: &mut wgpu::CommandEncoder,
        name: &str,
    ) -> Self {
        let start = profiler.write_timestamp(encoder);
        Self {
            name: name.to_string(),
            start_query: start,
        }
    }

    /// End timing and register the pass with the profiler.
    pub fn end(self, profiler: &mut GpuProfiler, encoder: &mut wgpu::CommandEncoder) {
        let end = profiler.write_timestamp(encoder);
        profiler.end_pass(&self.name, self.start_query, end);
    }
}

// ---------------------------------------------------------------------------
// Draw Call / Pipeline Statistics
// ---------------------------------------------------------------------------

/// Statistics collected per frame about the rendering workload.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    /// Total number of draw calls issued.
    pub draw_calls: u32,
    /// Total number of triangles rendered.
    pub triangles: u64,
    /// Number of render passes.
    pub render_passes: u32,
    /// Number of texture binds.
    pub texture_binds: u32,
    /// Number of pipeline switches.
    pub pipeline_switches: u32,
    /// Number of vertices processed.
    pub vertices: u64,
}

impl RenderStats {
    /// Reset all counters for a new frame.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl std::fmt::Display for RenderStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Render Statistics:")?;
        writeln!(f, "  Draw calls: {}", self.draw_calls)?;
        writeln!(f, "  Triangles: {}", self.triangles)?;
        writeln!(f, "  Vertices: {}", self.vertices)?;
        writeln!(f, "  Render passes: {}", self.render_passes)?;
        writeln!(f, "  Texture binds: {}", self.texture_binds)?;
        writeln!(f, "  Pipeline switches: {}", self.pipeline_switches)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_stats() {
        let mut stats = RenderStats::default();
        stats.draw_calls = 100;
        stats.triangles = 50000;
        stats.vertices = 150000;
        stats.render_passes = 3;
        stats.texture_binds = 50;
        stats.pipeline_switches = 10;

        let display = format!("{stats}");
        assert!(display.contains("Draw calls: 100"));
        assert!(display.contains("Triangles: 50000"));
        assert!(display.contains("Vertices: 150000"));
    }

    #[test]
    fn test_render_stats_reset() {
        let mut stats = RenderStats::default();
        stats.draw_calls = 100;
        stats.triangles = 50000;
        stats.reset();

        assert_eq!(stats.draw_calls, 0);
        assert_eq!(stats.triangles, 0);
    }

    #[test]
    fn test_gpu_pass_stats_average() {
        let stats = GpuPassStats {
            name: "test".to_string(),
            total_time: Duration::from_millis(100),
            sample_count: 10,
            peak_time: Duration::from_millis(15),
        };
        assert_eq!(stats.average_time(), Duration::from_millis(10));
    }

    #[test]
    fn test_gpu_pass_stats_zero_samples() {
        let stats = GpuPassStats {
            name: "test".to_string(),
            total_time: Duration::ZERO,
            sample_count: 0,
            peak_time: Duration::ZERO,
        };
        assert_eq!(stats.average_time(), Duration::ZERO);
    }
}
