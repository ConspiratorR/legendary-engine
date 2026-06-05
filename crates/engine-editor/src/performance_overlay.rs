//! Real-time performance overlay — FPS counter, frame time graph, and draw
//! call stats rendered as a HUD on top of the viewport.

use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Real-time performance overlay configuration.
#[derive(Debug, Clone)]
pub struct PerformanceOverlayConfig {
    /// Show the overlay.
    pub visible: bool,
    /// Position anchor (top-left, top-right, bottom-left, bottom-right).
    pub anchor: OverlayAnchor,
    /// Background opacity (0.0 - 1.0).
    pub bg_opacity: f32,
    /// Show FPS counter.
    pub show_fps: bool,
    /// Show frame time graph.
    pub show_frame_time_graph: bool,
    /// Show draw call count.
    pub show_draw_calls: bool,
    /// Show memory usage.
    pub show_memory: bool,
    /// Show GPU time.
    pub show_gpu_time: bool,
}

impl Default for PerformanceOverlayConfig {
    fn default() -> Self {
        Self {
            visible: false,
            anchor: OverlayAnchor::TopLeft,
            bg_opacity: 0.7,
            show_fps: true,
            show_frame_time_graph: true,
            show_draw_calls: true,
            show_memory: true,
            show_gpu_time: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Performance overlay state — tracks FPS, frame times, and render stats.
#[derive(Debug, Clone)]
pub struct PerformanceOverlay {
    pub config: PerformanceOverlayConfig,
    /// Rolling FPS samples.
    fps_samples: VecDeque<f32>,
    /// Rolling frame time samples (ms).
    frame_time_samples: VecDeque<f32>,
    /// Last frame instant for delta calculation.
    last_frame: Option<Instant>,
    /// Current FPS (smoothed).
    current_fps: f32,
    /// Current frame time (ms).
    current_frame_time_ms: f32,
    /// Current draw call count.
    current_draw_calls: u32,
    /// Current triangle count.
    current_triangles: u64,
    /// Current GPU time (ms).
    current_gpu_time_ms: f32,
    /// Estimated memory usage (bytes).
    current_memory_bytes: u64,
    /// Max samples to keep.
    max_samples: usize,
}

impl Default for PerformanceOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceOverlay {
    pub fn new() -> Self {
        Self {
            config: PerformanceOverlayConfig::default(),
            fps_samples: VecDeque::with_capacity(120),
            frame_time_samples: VecDeque::with_capacity(120),
            last_frame: None,
            current_fps: 0.0,
            current_frame_time_ms: 0.0,
            current_draw_calls: 0,
            current_triangles: 0,
            current_gpu_time_ms: 0.0,
            current_memory_bytes: 0,
            max_samples: 120,
        }
    }

    /// Called once per frame to update FPS and frame time.
    pub fn tick(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_frame {
            let delta = now.duration_since(last);
            let dt = delta.as_secs_f32();
            if dt > 0.0 {
                let fps = 1.0 / dt;
                self.fps_samples.push_back(fps);
                if self.fps_samples.len() > self.max_samples {
                    self.fps_samples.pop_front();
                }
                self.current_fps =
                    self.fps_samples.iter().sum::<f32>() / self.fps_samples.len() as f32;

                let ms = dt * 1000.0;
                self.frame_time_samples.push_back(ms);
                if self.frame_time_samples.len() > self.max_samples {
                    self.frame_time_samples.pop_front();
                }
                self.current_frame_time_ms = ms;
            }
        }
        self.last_frame = Some(now);
    }

    /// Update render stats from the profiler.
    pub fn update_stats(&mut self, draw_calls: u32, triangles: u64) {
        self.current_draw_calls = draw_calls;
        self.current_triangles = triangles;
    }

    /// Update GPU time.
    pub fn update_gpu_time(&mut self, gpu_time: Duration) {
        self.current_gpu_time_ms = gpu_time.as_secs_f32() * 1000.0;
    }

    /// Update memory usage estimate.
    pub fn update_memory(&mut self, bytes: u64) {
        self.current_memory_bytes = bytes;
    }

    /// Current FPS.
    pub fn fps(&self) -> f32 {
        self.current_fps
    }

    /// Current frame time in milliseconds.
    pub fn frame_time_ms(&self) -> f32 {
        self.current_frame_time_ms
    }

    /// Draw the overlay on screen.
    pub fn draw(&self, ctx: &egui::Context) {
        if !self.config.visible {
            return;
        }

        let screen = ctx.screen_rect();
        let scale = screen.height() / 1080.0;
        let overlay_w = 220.0 * scale;
        let overlay_h = self.calculate_height(scale);

        let pos = match self.config.anchor {
            OverlayAnchor::TopLeft => Pos2::new(8.0 * scale, 8.0 * scale),
            OverlayAnchor::TopRight => {
                Pos2::new(screen.width() - overlay_w - 8.0 * scale, 8.0 * scale)
            }
            OverlayAnchor::BottomLeft => {
                Pos2::new(8.0 * scale, screen.height() - overlay_h - 8.0 * scale)
            }
            OverlayAnchor::BottomRight => Pos2::new(
                screen.width() - overlay_w - 8.0 * scale,
                screen.height() - overlay_h - 8.0 * scale,
            ),
        };

        let overlay_rect = Rect::from_min_size(pos, Vec2::new(overlay_w, overlay_h));

        egui::Area::new(egui::Id::new("perf_overlay"))
            .fixed_pos(pos)
            .interactable(false)
            .show(ctx, |ui| {
                let painter = ui.painter_at(overlay_rect);

                // Background
                let bg_color = Color32::from_rgba_premultiplied(
                    15,
                    15,
                    18,
                    (self.config.bg_opacity * 255.0) as u8,
                );
                painter.add(Shape::rect_filled(
                    overlay_rect,
                    Rounding::same(6.0 * scale),
                    bg_color,
                ));
                painter.add(Shape::rect_stroke(
                    overlay_rect,
                    Rounding::same(6.0 * scale),
                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(60, 60, 70, 180)),
                ));

                let mut y = overlay_rect.top() + 8.0 * scale;
                let x = overlay_rect.left() + 10.0 * scale;
                let row_h = 18.0 * scale;
                let label_font = egui::FontId::proportional(10.0 * scale);
                let value_font = egui::FontId::proportional(12.0 * scale);

                // FPS
                if self.config.show_fps {
                    let fps_color = fps_color(self.current_fps);
                    painter.text(
                        Pos2::new(x, y),
                        egui::Align2::LEFT_CENTER,
                        "FPS",
                        label_font.clone(),
                        Color32::from_gray(120),
                    );
                    painter.text(
                        Pos2::new(overlay_rect.right() - 10.0 * scale, y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{:.0}", self.current_fps),
                        value_font.clone(),
                        fps_color,
                    );
                    y += row_h;
                }

                // Frame time
                if self.config.show_fps {
                    painter.text(
                        Pos2::new(x, y),
                        egui::Align2::LEFT_CENTER,
                        "帧时间",
                        label_font.clone(),
                        Color32::from_gray(120),
                    );
                    painter.text(
                        Pos2::new(overlay_rect.right() - 10.0 * scale, y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{:.1}ms", self.current_frame_time_ms),
                        value_font.clone(),
                        Color32::from_rgb(232, 232, 236),
                    );
                    y += row_h;
                }

                // GPU time
                if self.config.show_gpu_time {
                    painter.text(
                        Pos2::new(x, y),
                        egui::Align2::LEFT_CENTER,
                        "GPU",
                        label_font.clone(),
                        Color32::from_gray(120),
                    );
                    painter.text(
                        Pos2::new(overlay_rect.right() - 10.0 * scale, y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{:.1}ms", self.current_gpu_time_ms),
                        value_font.clone(),
                        Color32::from_rgb(0, 212, 170),
                    );
                    y += row_h;
                }

                // Draw calls
                if self.config.show_draw_calls {
                    painter.text(
                        Pos2::new(x, y),
                        egui::Align2::LEFT_CENTER,
                        "Draw Calls",
                        label_font.clone(),
                        Color32::from_gray(120),
                    );
                    painter.text(
                        Pos2::new(overlay_rect.right() - 10.0 * scale, y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{}", self.current_draw_calls),
                        value_font.clone(),
                        Color32::from_rgb(0, 150, 255),
                    );
                    y += row_h;

                    painter.text(
                        Pos2::new(x, y),
                        egui::Align2::LEFT_CENTER,
                        "三角形",
                        label_font.clone(),
                        Color32::from_gray(120),
                    );
                    painter.text(
                        Pos2::new(overlay_rect.right() - 10.0 * scale, y),
                        egui::Align2::RIGHT_CENTER,
                        format_triangles(self.current_triangles),
                        value_font.clone(),
                        Color32::from_rgb(0, 150, 255),
                    );
                    y += row_h;
                }

                // Memory
                if self.config.show_memory {
                    painter.text(
                        Pos2::new(x, y),
                        egui::Align2::LEFT_CENTER,
                        "内存",
                        label_font.clone(),
                        Color32::from_gray(120),
                    );
                    painter.text(
                        Pos2::new(overlay_rect.right() - 10.0 * scale, y),
                        egui::Align2::RIGHT_CENTER,
                        format_memory(self.current_memory_bytes),
                        value_font.clone(),
                        Color32::from_rgb(161, 107, 255),
                    );
                    y += row_h;
                }

                // Frame time graph
                if self.config.show_frame_time_graph && !self.frame_time_samples.is_empty() {
                    y += 4.0 * scale;
                    let graph_h = 30.0 * scale;
                    let graph_rect = Rect::from_min_size(
                        Pos2::new(x, y),
                        Vec2::new(overlay_rect.width() - 20.0 * scale, graph_h),
                    );

                    // Background
                    painter.add(Shape::rect_filled(
                        graph_rect,
                        Rounding::same(2.0 * scale),
                        Color32::from_rgba_premultiplied(0, 0, 0, 60),
                    ));

                    // Draw frame time bars
                    let max_ft = self
                        .frame_time_samples
                        .iter()
                        .copied()
                        .fold(0.0_f32, f32::max)
                        .max(1.0);

                    let bar_count = self.frame_time_samples.len();
                    let bar_w = graph_rect.width() / self.max_samples as f32;
                    let start_x = graph_rect.right() - bar_count as f32 * bar_w;

                    for (i, &ft) in self.frame_time_samples.iter().enumerate() {
                        let frac = ft / max_ft;
                        let bar_h = frac * graph_rect.height();
                        let bx = start_x + i as f32 * bar_w;

                        let color = if ft > 16.0 {
                            Color32::from_rgb(255, 107, 107)
                        } else if ft > 8.0 {
                            Color32::from_rgb(255, 184, 0)
                        } else {
                            Color32::from_rgb(0, 212, 170)
                        };

                        let bar_rect = Rect::from_min_size(
                            Pos2::new(bx, graph_rect.bottom() - bar_h),
                            Vec2::new(bar_w.max(1.0), bar_h),
                        );
                        painter.add(Shape::rect_filled(bar_rect, Rounding::ZERO, color));
                    }

                    // 16ms target line
                    let target_frac = 16.0 / max_ft;
                    let target_y = graph_rect.bottom() - target_frac * graph_rect.height();
                    if target_y > graph_rect.top() && target_y < graph_rect.bottom() {
                        painter.add(Shape::line(
                            vec![
                                Pos2::new(graph_rect.left(), target_y),
                                Pos2::new(graph_rect.right(), target_y),
                            ],
                            Stroke::new(
                                0.5_f32,
                                Color32::from_rgba_premultiplied(255, 107, 107, 100),
                            ),
                        ));
                    }
                }
            });
    }

    fn calculate_height(&self, scale: f32) -> f32 {
        let row_h = 18.0 * scale;
        let mut h = 8.0 * scale; // top padding
        if self.config.show_fps {
            h += row_h * 2.0; // FPS + frame time
        }
        if self.config.show_gpu_time {
            h += row_h;
        }
        if self.config.show_draw_calls {
            h += row_h * 2.0; // draw calls + triangles
        }
        if self.config.show_memory {
            h += row_h;
        }
        if self.config.show_frame_time_graph {
            h += 4.0 * scale + 30.0 * scale; // graph
        }
        h += 8.0 * scale; // bottom padding
        h
    }
}

fn fps_color(fps: f32) -> Color32 {
    if fps >= 55.0 {
        Color32::from_rgb(46, 213, 115) // green
    } else if fps >= 30.0 {
        Color32::from_rgb(255, 184, 0) // amber
    } else {
        Color32::from_rgb(255, 107, 107) // red
    }
}

fn format_triangles(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}

fn format_memory(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.0} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_tick_calculates_fps() {
        let mut overlay = PerformanceOverlay::new();
        overlay.config.visible = true;

        // First tick — no FPS yet (no previous frame)
        overlay.tick();
        assert_eq!(overlay.fps(), 0.0);

        // Simulate some delay
        std::thread::sleep(Duration::from_millis(16));
        overlay.tick();
        assert!(overlay.fps() > 0.0);
        assert!(overlay.frame_time_ms() > 0.0);
    }

    #[test]
    fn test_overlay_update_stats() {
        let mut overlay = PerformanceOverlay::new();
        overlay.update_stats(128, 50000);
        assert_eq!(overlay.current_draw_calls, 128);
        assert_eq!(overlay.current_triangles, 50000);
    }

    #[test]
    fn test_format_memory() {
        assert_eq!(format_memory(512), "512 B");
        assert_eq!(format_memory(2048), "2 KB");
        assert_eq!(format_memory(5 * 1048576), "5 MB");
        assert_eq!(format_memory(2 * 1073741824), "2.0 GB");
    }

    #[test]
    fn test_fps_color() {
        assert_eq!(fps_color(60.0), Color32::from_rgb(46, 213, 115));
        assert_eq!(fps_color(45.0), Color32::from_rgb(255, 184, 0));
        assert_eq!(fps_color(15.0), Color32::from_rgb(255, 107, 107));
    }

    #[test]
    fn test_format_triangles() {
        assert_eq!(format_triangles(500), "500");
        assert_eq!(format_triangles(15000), "15.0K");
        assert_eq!(format_triangles(2_500_000), "2.5M");
    }

    #[test]
    fn test_overlay_config_default() {
        let overlay = PerformanceOverlay::new();
        assert!(!overlay.config.visible);
        assert_eq!(overlay.config.anchor, OverlayAnchor::TopLeft);
        assert!(overlay.config.show_fps);
        assert!(overlay.config.show_draw_calls);
        assert!(overlay.config.show_memory);
    }
}
