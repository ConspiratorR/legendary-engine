use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Profiler {
    timers: HashMap<String, Timer>,
    frame_times: Vec<f32>,
    max_frames: usize,
}

pub struct Timer {
    name: String,
    start: Instant,
    elapsed: Duration,
    count: u64,
    children: HashMap<String, Duration>,
}

impl Profiler {
    pub fn new(max_frames: usize) -> Self {
        Self {
            timers: HashMap::new(),
            frame_times: Vec::with_capacity(max_frames),
            max_frames,
        }
    }

    pub fn start(&mut self, name: &str) {
        self.timers.insert(
            name.to_string(),
            Timer {
                name: name.to_string(),
                start: Instant::now(),
                elapsed: Duration::from_secs(0),
                count: 0,
                children: HashMap::new(),
            },
        );
    }

    pub fn end(&mut self, name: &str) {
        if let Some(timer) = self.timers.get_mut(name) {
            timer.elapsed = timer.start.elapsed();
            timer.count += 1;
        }
    }

    pub fn record_frame(&mut self) {
        let total: Duration = self.timers.values().map(|t| t.elapsed).sum();

        self.frame_times.push(total.as_secs_f32());

        if self.frame_times.len() > self.max_frames {
            self.frame_times.remove(0);
        }

        self.timers.clear();
    }

    pub fn average_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        let avg_frame_time: f32 =
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;

        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    pub fn min_fps(&self) -> f32 {
        self.frame_times
            .iter()
            .filter(|&&t| t > 0.0)
            .map(|&t| 1.0 / t)
            .fold(f32::INFINITY, f32::min)
    }

    pub fn max_fps(&self) -> f32 {
        self.frame_times
            .iter()
            .filter(|&&t| t > 0.0)
            .map(|&t| 1.0 / t)
            .fold(0.0, f32::max)
    }

    pub fn average_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }

    pub fn get_timers(&self) -> &HashMap<String, Timer> {
        &self.timers
    }

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
            timers.sort_by(|a, b| b.elapsed.cmp(&a.elapsed));

            for timer in timers {
                println!(
                    "  {}: {:.3}ms (x{})",
                    timer.name,
                    timer.elapsed.as_secs_f32() * 1000.0,
                    timer.count
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
