use std::collections::HashMap;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Scope Timer (RAII)
// ---------------------------------------------------------------------------

/// RAII guard that automatically records elapsed time when dropped.
///
/// Created by [`Profiler::scope`]. The timer starts immediately and
/// ends when the guard goes out of scope.
///
/// # Example
///
/// ```rust
/// use engine_core::profiler::Profiler;
///
/// let mut profiler = Profiler::new(60);
/// {
///     let _guard = profiler.scope("physics");
///     // ... physics work ...
/// } // automatically recorded here
/// ```
pub struct ScopeTimer<'a> {
    name: String,
    start: Instant,
    profiler: &'a mut Profiler,
}

impl<'a> Drop for ScopeTimer<'a> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        if let Some(timer) = self.profiler.timers.get_mut(&self.name) {
            timer.elapsed += elapsed;
            timer.count += 1;
            timer.current_depth = timer.current_depth.saturating_sub(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Timer
// ---------------------------------------------------------------------------

/// A named timer that records accumulated elapsed time.
pub struct Timer {
    name: String,
    start: Instant,
    elapsed: Duration,
    count: u64,
    current_depth: u32,
    peak_depth: u32,
    children: HashMap<String, Duration>,
}

impl Timer {
    /// Name of this timer.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Total accumulated elapsed time.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Number of times this timer was started/stopped.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Average elapsed per invocation.
    pub fn average(&self) -> Duration {
        if self.count == 0 {
            Duration::ZERO
        } else {
            self.elapsed / self.count as u32
        }
    }

    /// Children timings recorded under this timer.
    pub fn children(&self) -> &HashMap<String, Duration> {
        &self.children
    }
}

// ---------------------------------------------------------------------------
// Frame Record
// ---------------------------------------------------------------------------

/// A snapshot of profiling data for a single frame.
#[derive(Debug, Clone)]
pub struct FrameRecord {
    /// Frame number (sequential).
    pub frame_number: u64,
    /// Total frame time.
    pub total_time: Duration,
    /// Per-system timings for this frame.
    pub systems: HashMap<String, Duration>,
}

// ---------------------------------------------------------------------------
// Profiler
// ---------------------------------------------------------------------------

/// Performance profiler for tracking frame times and named timers.
///
/// Collects frame timing data over a sliding window and provides
/// statistics (average/min/max FPS, frame time). Named timers track
/// per-system execution times with hierarchical nesting support.
///
/// # Example
///
/// ```rust
/// use engine_core::profiler::Profiler;
///
/// let mut profiler = Profiler::new(120);
/// profiler.begin_frame();
/// {
///     let _guard = profiler.scope("physics");
///     // ... physics step ...
/// }
/// profiler.end_frame();
/// println!("Avg FPS: {:.1}", profiler.average_fps());
/// ```
pub struct Profiler {
    timers: HashMap<String, Timer>,
    frame_times: Vec<f32>,
    frame_history: Vec<FrameRecord>,
    max_frames: usize,
    frame_number: u64,
    frame_start: Instant,
    current_frame_systems: HashMap<String, Duration>,
}

impl Profiler {
    /// Create a new profiler that tracks the last `max_frames` frames.
    pub fn new(max_frames: usize) -> Self {
        Self {
            timers: HashMap::new(),
            frame_times: Vec::with_capacity(max_frames),
            frame_history: Vec::with_capacity(max_frames),
            max_frames,
            frame_number: 0,
            frame_start: Instant::now(),
            current_frame_systems: HashMap::new(),
        }
    }

    /// Begin a new frame. Call at the start of each frame.
    pub fn begin_frame(&mut self) {
        self.frame_start = Instant::now();
        self.current_frame_systems.clear();
    }

    /// End the current frame and record its timing data.
    pub fn end_frame(&mut self) {
        let total = self.frame_start.elapsed();

        // Record per-system timings from active timers
        for (name, timer) in &self.timers {
            if timer.count > 0 {
                self.current_frame_systems
                    .insert(name.clone(), timer.elapsed);
            }
        }

        self.frame_times.push(total.as_secs_f32());
        if self.frame_times.len() > self.max_frames {
            self.frame_times.remove(0);
        }

        let record = FrameRecord {
            frame_number: self.frame_number,
            total_time: total,
            systems: self.current_frame_systems.clone(),
        };
        self.frame_history.push(record);
        if self.frame_history.len() > self.max_frames {
            self.frame_history.remove(0);
        }

        self.frame_number += 1;
        self.timers.clear();
    }

    /// Start a named timer. Overwrites any existing timer with the same name.
    pub fn start(&mut self, name: &str) {
        self.timers.insert(
            name.to_string(),
            Timer {
                name: name.to_string(),
                start: Instant::now(),
                elapsed: Duration::ZERO,
                count: 0,
                current_depth: 0,
                peak_depth: 0,
                children: HashMap::new(),
            },
        );
    }

    /// Stop a named timer and record its elapsed time.
    pub fn end(&mut self, name: &str) {
        if let Some(timer) = self.timers.get_mut(name) {
            timer.elapsed += timer.start.elapsed();
            timer.count += 1;
            timer.current_depth = timer.current_depth.saturating_sub(1);
        }
    }

    /// Create a RAII scope timer that records automatically on drop.
    pub fn scope(&mut self, name: &str) -> ScopeTimer<'_> {
        self.timers.insert(
            name.to_string(),
            Timer {
                name: name.to_string(),
                start: Instant::now(),
                elapsed: Duration::ZERO,
                count: 0,
                current_depth: 0,
                peak_depth: 0,
                children: HashMap::new(),
            },
        );
        if let Some(timer) = self.timers.get_mut(name) {
            timer.current_depth += 1;
            if timer.current_depth > timer.peak_depth {
                timer.peak_depth = timer.current_depth;
            }
        }
        ScopeTimer {
            name: name.to_string(),
            start: Instant::now(),
            profiler: self,
        }
    }

    /// Record the current frame's total time and clear timers for the next frame.
    /// Prefer `begin_frame`/`end_frame` for new code.
    pub fn record_frame(&mut self) {
        self.end_frame();
        self.begin_frame();
    }

    /// Calculate average FPS from the tracked frame times.
    pub fn average_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    /// Get the minimum FPS observed in the tracked window.
    pub fn min_fps(&self) -> f32 {
        self.frame_times
            .iter()
            .filter(|&&t| t > 0.0)
            .map(|&t| 1.0 / t)
            .fold(f32::INFINITY, f32::min)
    }

    /// Get the maximum FPS observed in the tracked window.
    pub fn max_fps(&self) -> f32 {
        self.frame_times
            .iter()
            .filter(|&&t| t > 0.0)
            .map(|&t| 1.0 / t)
            .fold(0.0, f32::max)
    }

    /// Get the average frame time in seconds.
    pub fn average_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }

    /// Get a reference to the active timers.
    pub fn get_timers(&self) -> &HashMap<String, Timer> {
        &self.timers
    }

    /// Get the frame history records.
    pub fn frame_history(&self) -> &[FrameRecord] {
        &self.frame_history
    }

    /// Get the current frame number.
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Get the average time for a specific system across recent frames.
    pub fn system_average(&self, name: &str) -> Duration {
        let times: Vec<Duration> = self
            .frame_history
            .iter()
            .filter_map(|f| f.systems.get(name).copied())
            .collect();
        if times.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = times.iter().sum();
        total / times.len() as u32
    }

    /// Get the peak (max) time for a specific system across recent frames.
    pub fn system_peak(&self, name: &str) -> Duration {
        self.frame_history
            .iter()
            .filter_map(|f| f.systems.get(name).copied())
            .max()
            .unwrap_or(Duration::ZERO)
    }

    /// Print a formatted summary of performance statistics to stdout.
    pub fn print_stats(&self) {
        println!("\n=== Performance Statistics ===");
        println!(
            "FPS - Avg: {:.1}, Min: {:.1}, Max: {:.1}",
            self.average_fps(),
            self.min_fps(),
            self.max_fps()
        );
        println!(
            "Frame Time - Avg: {:.3}ms",
            self.average_frame_time() * 1000.0
        );

        if !self.timers.is_empty() {
            println!("\nTimer Breakdown:");
            let mut timers: Vec<_> = self.timers.values().collect();
            timers.sort_by_key(|b| std::cmp::Reverse(b.elapsed));

            for timer in timers {
                println!(
                    "  {}: {:.3}ms (x{})",
                    timer.name,
                    timer.elapsed.as_secs_f32() * 1000.0,
                    timer.count
                );
            }
        }

        if !self.frame_history.is_empty() {
            println!(
                "\nFrame Timeline (last {} frames):",
                self.frame_history.len()
            );
            let mut system_names: Vec<String> = self
                .frame_history
                .iter()
                .flat_map(|f| f.systems.keys().cloned())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            system_names.sort();

            for name in &system_names {
                let avg = self.system_average(name);
                let peak = self.system_peak(name);
                println!(
                    "  {}: avg {:.3}ms, peak {:.3}ms",
                    name,
                    avg.as_secs_f32() * 1000.0,
                    peak.as_secs_f32() * 1000.0
                );
            }
        }
        println!();
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_basic() {
        let mut profiler = Profiler::new(60);
        profiler.start("test");
        std::thread::sleep(Duration::from_millis(1));
        profiler.end("test");
        profiler.record_frame();

        assert_eq!(profiler.frame_number(), 1);
        assert!(profiler.average_fps() > 0.0);
    }

    #[test]
    fn test_profiler_scope_timer() {
        let mut profiler = Profiler::new(60);
        profiler.begin_frame();
        {
            let _guard = profiler.scope("physics");
            std::thread::sleep(Duration::from_millis(1));
        }
        profiler.end_frame();

        assert_eq!(profiler.frame_number(), 1);
        let history = profiler.frame_history();
        assert_eq!(history.len(), 1);
        assert!(history[0].systems.contains_key("physics"));
    }

    #[test]
    fn test_profiler_system_stats() {
        let mut profiler = Profiler::new(10);

        for _ in 0..5 {
            profiler.begin_frame();
            profiler.start("render");
            std::thread::sleep(Duration::from_millis(1));
            profiler.end("render");
            profiler.end_frame();
        }

        let avg = profiler.system_average("render");
        assert!(avg > Duration::ZERO);

        let peak = profiler.system_peak("render");
        assert!(peak >= avg);
    }

    #[test]
    fn test_frame_history_sliding_window() {
        let mut profiler = Profiler::new(3);

        for _ in 0..5 {
            profiler.begin_frame();
            profiler.end_frame();
        }

        assert_eq!(profiler.frame_history().len(), 3);
        assert_eq!(profiler.frame_number(), 5);
    }

    #[test]
    fn test_fps_stats() {
        let mut profiler = Profiler::new(60);
        for _ in 0..10 {
            profiler.begin_frame();
            std::thread::sleep(Duration::from_millis(1));
            profiler.end_frame();
        }

        assert!(profiler.average_fps() > 0.0);
        assert!(profiler.min_fps() > 0.0);
        assert!(profiler.max_fps() >= profiler.min_fps());
    }
}
