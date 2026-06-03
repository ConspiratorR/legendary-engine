use egui::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_render::gpu_profiler::{FrameHistory, FrameSnapshot, RenderStats};
use std::collections::HashMap;
use std::time::Duration;

/// State for the performance profiler panel.
#[derive(Debug, Clone)]
pub struct PerformanceProfilerState {
    pub visible: bool,
    pub active_sub_tab: usize,
    frame_history: FrameHistory,
    /// Accumulated render stats for the current frame.
    current_stats: RenderStats,
    /// Accumulated GPU pass timings for the current frame.
    current_pass_timings: HashMap<String, Duration>,
    /// CPU frame time for the current frame.
    current_cpu_time: Duration,
    /// GPU frame time for the current frame.
    current_gpu_time: Duration,
    /// Frame counter.
    frame_number: u64,
}

impl Default for PerformanceProfilerState {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceProfilerState {
    pub fn new() -> Self {
        Self {
            visible: false,
            active_sub_tab: 0,
            frame_history: FrameHistory::new(300),
            current_stats: RenderStats::default(),
            current_pass_timings: HashMap::new(),
            current_cpu_time: Duration::ZERO,
            current_gpu_time: Duration::ZERO,
            frame_number: 0,
        }
    }

    /// Begin a new frame — resets current-frame accumulators.
    pub fn begin_frame(&mut self) {
        self.current_stats.reset();
        self.current_pass_timings.clear();
        self.current_cpu_time = Duration::ZERO;
        self.current_gpu_time = Duration::ZERO;
    }

    /// Record draw call stats for the current frame.
    pub fn record_draw_calls(&mut self, draw_calls: u32, triangles: u64, vertices: u64) {
        self.current_stats.draw_calls += draw_calls;
        self.current_stats.triangles += triangles;
        self.current_stats.vertices += vertices;
    }

    /// Record state change counts for the current frame.
    pub fn record_state_changes(
        &mut self,
        pipeline_switches: u32,
        texture_binds: u32,
        buffer_binds: u32,
        shader_switches: u32,
    ) {
        self.current_stats.pipeline_switches += pipeline_switches;
        self.current_stats.texture_binds += texture_binds;
        self.current_stats.buffer_binds += buffer_binds;
        self.current_stats.shader_switches += shader_switches;
    }

    /// Record a named GPU pass timing for the current frame.
    pub fn record_pass_timing(&mut self, name: &str, duration: Duration) {
        *self
            .current_pass_timings
            .entry(name.to_string())
            .or_insert(Duration::ZERO) += duration;
    }

    /// Set the CPU frame time for the current frame.
    pub fn set_cpu_frame_time(&mut self, duration: Duration) {
        self.current_cpu_time = duration;
    }

    /// Set the GPU frame time for the current frame.
    pub fn set_gpu_frame_time(&mut self, duration: Duration) {
        self.current_gpu_time = duration;
    }

    /// End the current frame — pushes snapshot into history.
    pub fn end_frame(&mut self) {
        self.frame_number += 1;
        self.frame_history.push(FrameSnapshot {
            frame_number: self.frame_number,
            cpu_frame_time: self.current_cpu_time,
            gpu_frame_time: self.current_gpu_time,
            render_stats: self.current_stats.clone(),
            pass_timings: self.current_pass_timings.clone(),
        });
    }

    /// Get the frame history.
    pub fn frame_history(&self) -> &FrameHistory {
        &self.frame_history
    }

    /// Get the current frame's render stats.
    pub fn current_stats(&self) -> &RenderStats {
        &self.current_stats
    }

    /// Get the current frame's pass timings.
    pub fn current_pass_timings(&self) -> &HashMap<String, Duration> {
        &self.current_pass_timings
    }
}

/// Color palette for GPU pass visualization.
const PASS_COLORS: &[Color32] = &[
    Color32::from_rgb(0, 150, 255),   // blue
    Color32::from_rgb(0, 212, 170),   // teal
    Color32::from_rgb(255, 184, 0),   // amber
    Color32::from_rgb(255, 107, 107), // red
    Color32::from_rgb(161, 107, 255), // purple
    Color32::from_rgb(46, 213, 115),  // green
    Color32::from_rgb(255, 159, 67),  // orange
    Color32::from_rgb(77, 171, 247),  // light blue
];

fn pass_color(index: usize) -> Color32 {
    PASS_COLORS[index % PASS_COLORS.len()]
}

/// Draw the performance profiler panel content.
pub fn draw(state: &mut PerformanceProfilerState, ui: &mut egui::Ui, rect: Rect) {
    let h_scale = ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = ui.ctx().screen_rect().width() / 1920.0;

    let tab_h = 28.0 * h_scale;
    let tab_bar_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), tab_h));
    let sub_tabs = &["Draw Call 统计", "时间线", "GPU Pass"];
    let tab_font = 11.0 * h_scale;
    let char_w = 7.0 * w_scale;
    let mut tx = rect.left() + 8.0 * w_scale;

    let painter = ui.painter_at(rect);

    for (i, label) in sub_tabs.iter().enumerate() {
        let text_w = label.len() as f32 * char_w;
        let tab_rect = Rect::from_min_size(
            Pos2::new(tx, rect.top()),
            Vec2::new(text_w + 20.0 * w_scale, tab_h),
        );
        let id = egui::Id::new("perf_sub_tab").with(i as u64);
        let response = ui.interact(tab_rect, id, egui::Sense::click());

        if state.active_sub_tab == i {
            let line_rect = Rect::from_min_size(
                Pos2::new(tab_rect.left(), tab_rect.bottom() - 2.0 * h_scale),
                Vec2::new(tab_rect.width(), 2.0 * h_scale),
            );
            painter.add(Shape::rect_filled(
                line_rect,
                Rounding::ZERO,
                Color32::from_rgb(0, 212, 170),
            ));
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_rgb(0, 212, 170),
            );
        } else {
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                FontId::proportional(tab_font),
                Color32::from_gray(90),
            );
        }
        if response.clicked() {
            state.active_sub_tab = i;
        }
        tx += text_w + 20.0 * w_scale;
    }

    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), tab_bar_rect.bottom() + 4.0 * h_scale),
        Vec2::new(
            rect.width() - 16.0 * w_scale,
            rect.bottom() - tab_bar_rect.bottom() - 8.0 * h_scale,
        ),
    );

    match state.active_sub_tab {
        0 => draw_call_stats(state, &painter, content_rect, h_scale, w_scale),
        1 => draw_timeline(state, &painter, content_rect, h_scale, w_scale),
        2 => draw_gpu_passes(state, &painter, content_rect, h_scale, w_scale),
        _ => {}
    }
}

use egui::FontId;

/// Draw Call 统计子面板
fn draw_call_stats(
    state: &PerformanceProfilerState,
    painter: &egui::Painter,
    rect: Rect,
    h_scale: f32,
    w_scale: f32,
) {
    let label_font = FontId::proportional(11.0 * h_scale);
    let value_font = FontId::proportional(13.0 * h_scale);
    let row_h = 20.0 * h_scale;
    let col_w = rect.width() / 3.0;
    let pad = 12.0 * w_scale;

    let stats = state.current_stats();
    let latest = state.frame_history().latest();

    // Column headers
    let header_color = Color32::from_gray(120);
    let columns = [("当前帧", 0.0), ("平均值", col_w), ("峰值", col_w * 2.0)];

    let mut y = rect.top();

    // Header row
    for (label, x_off) in &columns {
        painter.text(
            Pos2::new(rect.left() + pad + x_off, y),
            egui::Align2::LEFT_CENTER,
            *label,
            FontId::proportional(10.0 * h_scale),
            header_color,
        );
    }
    y += row_h + 4.0 * h_scale;

    // Stats rows
    let stat_rows = [
        ("Draw Calls", format!("{}", stats.draw_calls)),
        ("三角形", format_number(stats.triangles)),
        ("顶点", format_number(stats.vertices)),
        ("渲染 Pass", format!("{}", stats.render_passes)),
        ("纹理绑定", format!("{}", stats.texture_binds)),
        ("管线切换", format!("{}", stats.pipeline_switches)),
        ("Buffer 绑定", format!("{}", stats.buffer_binds)),
        ("Shader 切换", format!("{}", stats.shader_switches)),
    ];

    let avg_stats = latest.map(|s| &s.render_stats);
    let peak_stats = find_peak_stats(state);

    for (i, (label, current_val)) in stat_rows.iter().enumerate() {
        let row_y = y + i as f32 * row_h;

        // Label
        painter.text(
            Pos2::new(rect.left() + pad, row_y),
            egui::Align2::LEFT_CENTER,
            *label,
            label_font.clone(),
            Color32::from_gray(152),
        );

        // Current value
        painter.text(
            Pos2::new(rect.left() + pad, row_y),
            egui::Align2::LEFT_CENTER,
            current_val.as_str(),
            value_font.clone(),
            Color32::from_rgb(232, 232, 236),
        );

        // Average value
        if let Some(avg) = avg_stats {
            let avg_val = get_stat_value(avg, i);
            painter.text(
                Pos2::new(rect.left() + pad + col_w, row_y),
                egui::Align2::LEFT_CENTER,
                avg_val.as_str(),
                value_font.clone(),
                Color32::from_gray(152),
            );
        }

        // Peak value
        if let Some(peak) = &peak_stats {
            let peak_val = get_stat_value(peak, i);
            painter.text(
                Pos2::new(rect.left() + pad + col_w * 2.0, row_y),
                egui::Align2::LEFT_CENTER,
                peak_val.as_str(),
                value_font.clone(),
                Color32::from_rgb(255, 184, 0),
            );
        }
    }
}

/// 时间线子面板 — CPU/GPU 帧时间柱状图
fn draw_timeline(
    state: &PerformanceProfilerState,
    painter: &egui::Painter,
    rect: Rect,
    h_scale: f32,
    _w_scale: f32,
) {
    let frames = state.frame_history().frames();
    if frames.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "暂无帧数据",
            FontId::proportional(12.0 * h_scale),
            Color32::from_gray(90),
        );
        return;
    }

    let label_font = FontId::proportional(10.0 * h_scale);
    let legend_h = 20.0 * h_scale;
    let chart_rect = Rect::from_min_size(
        rect.left_top(),
        Vec2::new(rect.width(), rect.height() - legend_h),
    );

    // Find max frame time for scaling
    let max_time = frames
        .iter()
        .map(|f| f.cpu_frame_time.max(f.gpu_frame_time))
        .max()
        .unwrap_or(Duration::from_millis(16))
        .as_secs_f32()
        .max(0.001);

    let bar_count = frames.len().min(chart_rect.width() as usize / 3);
    let bar_w = chart_rect.width() / bar_count as f32;
    let start_idx = frames.len().saturating_sub(bar_count);

    // Draw bars
    for (i, frame) in frames[start_idx..].iter().enumerate() {
        let x = chart_rect.left() + i as f32 * bar_w;

        // CPU bar (full height)
        let cpu_h = (frame.cpu_frame_time.as_secs_f32() / max_time) * chart_rect.height();
        let cpu_rect = Rect::from_min_size(
            Pos2::new(x + 1.0, chart_rect.bottom() - cpu_h),
            Vec2::new(bar_w * 0.45, cpu_h),
        );
        painter.add(Shape::rect_filled(
            cpu_rect,
            Rounding::same(1.0),
            Color32::from_rgb(0, 150, 255),
        ));

        // GPU bar
        let gpu_h = (frame.gpu_frame_time.as_secs_f32() / max_time) * chart_rect.height();
        let gpu_rect = Rect::from_min_size(
            Pos2::new(x + bar_w * 0.5, chart_rect.bottom() - gpu_h),
            Vec2::new(bar_w * 0.45, gpu_h),
        );
        painter.add(Shape::rect_filled(
            gpu_rect,
            Rounding::same(1.0),
            Color32::from_rgb(0, 212, 170),
        ));
    }

    // 16ms target line
    let target_h = (0.016 / max_time) * chart_rect.height();
    let target_y = chart_rect.bottom() - target_h;
    if target_y > chart_rect.top() {
        painter.add(Shape::line(
            vec![
                Pos2::new(chart_rect.left(), target_y),
                Pos2::new(chart_rect.right(), target_y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(255, 107, 107)),
        ));
        painter.text(
            Pos2::new(chart_rect.right() - 4.0, target_y - 4.0),
            egui::Align2::RIGHT_BOTTOM,
            "16ms",
            FontId::proportional(9.0 * h_scale),
            Color32::from_rgb(255, 107, 107),
        );
    }

    // Legend
    let legend_y = chart_rect.bottom() + 4.0 * h_scale;
    let legend_items = [
        ("CPU", Color32::from_rgb(0, 150, 255)),
        ("GPU", Color32::from_rgb(0, 212, 170)),
    ];
    let mut lx = rect.left() + 12.0;
    for (label, color) in &legend_items {
        painter.add(Shape::rect_filled(
            Rect::from_min_size(Pos2::new(lx, legend_y), Vec2::new(10.0, 10.0)),
            Rounding::same(2.0),
            *color,
        ));
        painter.text(
            Pos2::new(lx + 14.0, legend_y + 5.0),
            egui::Align2::LEFT_CENTER,
            *label,
            label_font.clone(),
            Color32::from_gray(152),
        );
        lx += 60.0;
    }

    // Stats summary
    let avg_cpu = state.frame_history().avg_cpu_frame_time();
    let avg_gpu = state.frame_history().avg_gpu_frame_time();
    let summary = format!(
        "CPU: {:.1}ms  GPU: {:.1}ms  (avg over {} frames)",
        avg_cpu.as_secs_f32() * 1000.0,
        avg_gpu.as_secs_f32() * 1000.0,
        frames.len(),
    );
    painter.text(
        Pos2::new(rect.right() - 12.0, legend_y + 5.0),
        egui::Align2::RIGHT_CENTER,
        summary,
        label_font,
        Color32::from_gray(120),
    );
}

/// GPU Pass 色块图子面板
fn draw_gpu_passes(
    state: &PerformanceProfilerState,
    painter: &egui::Painter,
    rect: Rect,
    h_scale: f32,
    _w_scale: f32,
) {
    let pass_timings = state.current_pass_timings();
    if pass_timings.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "暂无 GPU Pass 数据",
            FontId::proportional(12.0 * h_scale),
            Color32::from_gray(90),
        );
        return;
    }

    let label_font = FontId::proportional(10.0 * h_scale);
    let value_font = FontId::proportional(11.0 * h_scale);
    let row_h = 24.0 * h_scale;
    let bar_h = 14.0 * h_scale;

    // Sort passes by duration descending
    let mut sorted_passes: Vec<_> = pass_timings.iter().collect();
    sorted_passes.sort_by(|a, b| b.1.cmp(a.1));

    let max_duration = sorted_passes
        .first()
        .map(|(_, d)| d.as_secs_f32())
        .unwrap_or(0.001)
        .max(0.001);

    let total_duration: f32 = sorted_passes.iter().map(|(_, d)| d.as_secs_f32()).sum();

    // Stacked bar at the top
    let bar_rect = Rect::from_min_size(
        rect.left_top(),
        Vec2::new(rect.width(), bar_h + 8.0 * h_scale),
    );
    let mut bx = bar_rect.left();
    for (i, (_, duration)) in sorted_passes.iter().enumerate() {
        let fraction = duration.as_secs_f32() / total_duration;
        let seg_w = fraction * bar_rect.width();
        let seg_rect = Rect::from_min_size(
            Pos2::new(bx, bar_rect.top() + 4.0 * h_scale),
            Vec2::new(seg_w.max(1.0), bar_h),
        );
        painter.add(Shape::rect_filled(seg_rect, Rounding::ZERO, pass_color(i)));
        bx += seg_w;
    }

    // Pass list
    let list_top = bar_rect.bottom() + 8.0 * h_scale;
    let bar_max_w = rect.width() * 0.4;

    for (i, (name, duration)) in sorted_passes.iter().enumerate() {
        let y = list_top + i as f32 * row_h;
        if y + row_h > rect.bottom() {
            break;
        }

        // Color swatch
        let swatch_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 8.0, y + (row_h - bar_h) * 0.5),
            Vec2::new(bar_h, bar_h),
        );
        painter.add(Shape::rect_filled(
            swatch_rect,
            Rounding::same(2.0),
            pass_color(i),
        ));

        // Name
        painter.text(
            Pos2::new(rect.left() + 28.0, y + row_h * 0.5),
            egui::Align2::LEFT_CENTER,
            name.as_str(),
            label_font.clone(),
            Color32::from_rgb(232, 232, 236),
        );

        // Duration bar
        let frac = duration.as_secs_f32() / max_duration;
        let dw = frac * bar_max_w;
        let dur_bar_rect = Rect::from_min_size(
            Pos2::new(rect.left() + rect.width() * 0.5, y + (row_h - bar_h) * 0.5),
            Vec2::new(dw.max(2.0), bar_h),
        );
        painter.add(Shape::rect_filled(
            dur_bar_rect,
            Rounding::same(2.0),
            pass_color(i),
        ));

        // Duration value
        let ms = duration.as_secs_f32() * 1000.0;
        let pct = if total_duration > 0.0 {
            (duration.as_secs_f32() / total_duration) * 100.0
        } else {
            0.0
        };
        painter.text(
            Pos2::new(rect.right() - 8.0, y + row_h * 0.5),
            egui::Align2::RIGHT_CENTER,
            format!("{:.2}ms ({:.0}%)", ms, pct),
            value_font.clone(),
            Color32::from_gray(152),
        );
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}

fn get_stat_value(stats: &RenderStats, index: usize) -> String {
    match index {
        0 => format!("{}", stats.draw_calls),
        1 => format_number(stats.triangles),
        2 => format_number(stats.vertices),
        3 => format!("{}", stats.render_passes),
        4 => format!("{}", stats.texture_binds),
        5 => format!("{}", stats.pipeline_switches),
        6 => format!("{}", stats.buffer_binds),
        7 => format!("{}", stats.shader_switches),
        _ => String::new(),
    }
}

fn find_peak_stats(state: &PerformanceProfilerState) -> Option<RenderStats> {
    let frames = state.frame_history().frames();
    if frames.is_empty() {
        return None;
    }
    Some(
        frames
            .iter()
            .max_by_key(|f| f.render_stats.draw_calls)
            .unwrap()
            .render_stats
            .clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_state_record_and_snapshot() {
        let mut state = PerformanceProfilerState::new();
        state.begin_frame();
        state.record_draw_calls(50, 10000, 30000);
        state.record_state_changes(5, 10, 8, 3);
        state.record_pass_timing("shadow", Duration::from_millis(2));
        state.record_pass_timing("main", Duration::from_millis(8));
        state.set_cpu_frame_time(Duration::from_millis(16));
        state.set_gpu_frame_time(Duration::from_millis(10));
        state.end_frame();

        assert_eq!(state.frame_history().frames().len(), 1);
        let latest = state.frame_history().latest().unwrap();
        assert_eq!(latest.render_stats.draw_calls, 50);
        assert_eq!(latest.render_stats.triangles, 10000);
        assert_eq!(latest.pass_timings.len(), 2);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1500), "1.5K");
        assert_eq!(format_number(2_500_000), "2.5M");
    }

    #[test]
    fn test_profiler_state_begin_resets() {
        let mut state = PerformanceProfilerState::new();
        state.begin_frame();
        state.record_draw_calls(100, 50000, 150000);
        state.end_frame();

        state.begin_frame();
        assert_eq!(state.current_stats().draw_calls, 0);
        state.end_frame();

        assert_eq!(state.frame_history().frames().len(), 2);
    }
}
