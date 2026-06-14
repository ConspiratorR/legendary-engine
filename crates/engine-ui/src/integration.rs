//! egui ↔ wgpu bridge.
//!
//! [`EguiState`] manages the `egui` context, the `egui_wgpu::Renderer`,
//! and input forwarding (mouse position, button presses). Call
//! [`begin_frame`](EguiState::begin_frame) at the start of each frame,
//! use [`ctx`](EguiState::ctx) to build UI, then
//! [`end_frame`](EguiState::end_frame) and [`paint`](EguiState::paint)
//! to render via wgpu.

use std::any::TypeId;
use std::collections::HashMap;

use egui::Context;
use egui_wgpu::{Renderer, ScreenDescriptor};

/// Manages the `egui` context, renderer, and input forwarding.
///
/// Call [`begin_frame`](Self::begin_frame) at the start of each frame,
/// use [`ctx`](Self::ctx) to build UI, then [`end_frame`](Self::end_frame)
/// and [`paint`](Self::paint) to render.
pub struct EguiState {
    /// The egui context for building UI.
    pub ctx: Context,
    renderer: Option<Renderer>,
    screen_descriptor: ScreenDescriptor,
    width: u32,
    height: u32,
    scale_factor: f32,
    mouse_pos: (f64, f64),
    mouse_buttons: [bool; 3],
    just_pressed: [bool; 3],
    just_released: [bool; 3],
    callback_resources: HashMap<TypeId, Box<dyn std::any::Any>>,
}

impl EguiState {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        scale_factor: f32,
    ) -> Self {
        let ctx = Context::default();
        ctx.set_fonts(Self::load_fonts());
        let renderer = Renderer::new(device, config.format, None, 1, false);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: scale_factor,
        };
        Self {
            ctx,
            renderer: Some(renderer),
            screen_descriptor,
            width: config.width,
            height: config.height,
            scale_factor,
            mouse_pos: (0.0, 0.0),
            mouse_buttons: [false; 3],
            just_pressed: [false; 3],
            just_released: [false; 3],
            callback_resources: HashMap::new(),
        }
    }

    /// Insert a custom callback resource for use with PaintCallback.
    pub fn insert_callback_resource<T: 'static>(&mut self, resource: T) {
        self.callback_resources
            .insert(TypeId::of::<T>(), Box::new(resource));
    }

    /// Get a reference to a custom callback resource.
    pub fn callback_resource<T: 'static>(&self) -> Option<&T> {
        self.callback_resources
            .get(&TypeId::of::<T>())
            .and_then(|r| r.downcast_ref())
    }

    /// Get a mutable reference to a custom callback resource.
    pub fn callback_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.callback_resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|r| r.downcast_mut())
    }

    fn load_fonts() -> egui::FontDefinitions {
        let mut fonts = egui::FontDefinitions::default();
        let cjk_candidates = [
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\msyhbd.ttc",
            "C:\\Windows\\Fonts\\simsun.ttc",
        ];
        for path in &cjk_candidates {
            if let Ok(data) = std::fs::read(path) {
                let name = format!("cjk_{}", path.rsplit('\\').next().unwrap_or("font"));
                fonts.font_data.insert(
                    name.clone(),
                    std::sync::Arc::new(egui::FontData::from_owned(data)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, name.clone());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, name);
                break;
            }
        }
        fonts
    }

    #[allow(deprecated)]
    pub fn begin_frame(&mut self, time: f64) {
        let mut events = Vec::new();
        let pos = egui::pos2(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);

        events.push(egui::Event::PointerMoved(pos));

        let buttons = [
            egui::PointerButton::Primary,
            egui::PointerButton::Secondary,
            egui::PointerButton::Middle,
        ];
        for (i, &button) in buttons.iter().enumerate() {
            if self.just_pressed[i] {
                events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                });
                self.just_pressed[i] = false;
            }
            if self.just_released[i] {
                events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                });
                self.just_released[i] = false;
            }
        }

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.width as f32, self.height as f32),
            )),
            time: Some(time),
            events,
            ..Default::default()
        };
        self.ctx.begin_frame(raw_input);
    }

    #[allow(deprecated)]
    pub fn end_frame(&mut self) -> (Vec<egui::ClippedPrimitive>, egui::TexturesDelta) {
        let full_output = self.ctx.end_frame();
        let paint_jobs = self.ctx.tessellate(full_output.shapes, self.scale_factor);
        (paint_jobs, full_output.textures_delta)
    }

    pub fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output: &wgpu::SurfaceTexture,
        paint_jobs: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
    ) {
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui_encoder"),
        });

        let renderer = self
            .renderer
            .as_mut()
            .expect("egui renderer must be initialized");

        for (id, delta) in &textures_delta.set {
            renderer.update_texture(device, queue, *id, delta);
        }

        renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            paint_jobs,
            &self.screen_descriptor,
        );

        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let mut rpass_static = rpass.forget_lifetime();
            renderer.render(&mut rpass_static, paint_jobs, &self.screen_descriptor);
        }

        queue.submit([encoder.finish()]);

        for id in &textures_delta.free {
            renderer.free_texture(id);
        }
    }

    pub fn handle_mouse_move(&mut self, x: f64, y: f64) {
        self.mouse_pos = (x, y);
    }

    pub fn press_button(&mut self, button: usize) {
        if !self.mouse_buttons[button] {
            self.just_pressed[button] = true;
            self.mouse_buttons[button] = true;
        }
    }

    pub fn release_button(&mut self, button: usize) {
        if self.mouse_buttons[button] {
            self.just_released[button] = true;
            self.mouse_buttons[button] = false;
        }
    }

    pub fn press_left(&mut self) {
        self.press_button(0);
    }

    pub fn release_left(&mut self) {
        self.release_button(0);
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f32) {
        self.width = width;
        self.height = height;
        self.scale_factor = scale_factor;
        self.screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale_factor,
        };
    }

    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::EguiState;

    fn dummy_state(width: u32, height: u32, scale_factor: f32) -> EguiState {
        EguiState {
            ctx: egui::Context::default(),
            renderer: None,
            screen_descriptor: egui_wgpu::ScreenDescriptor {
                size_in_pixels: [width, height],
                pixels_per_point: scale_factor,
            },
            width,
            height,
            scale_factor,
            mouse_pos: (0.0, 0.0),
            mouse_buttons: [false; 3],
            just_pressed: [false; 3],
            just_released: [false; 3],
            callback_resources: HashMap::new(),
        }
    }

    #[test]
    fn test_mouse_tracking() {
        let mut state = dummy_state(1024, 768, 1.0);
        assert_eq!(state.mouse_pos, (0.0, 0.0));
        assert!(!state.mouse_buttons[0]);

        state.handle_mouse_move(100.0, 200.0);
        assert_eq!(state.mouse_pos, (100.0, 200.0));

        state.press_button(0);
        assert!(state.mouse_buttons[0]);
        assert!(state.just_pressed[0]);

        state.press_button(0);
        assert!(state.mouse_buttons[0]);
        assert!(state.just_pressed[0]);

        state.release_button(0);
        assert!(!state.mouse_buttons[0]);
        assert!(state.just_released[0]);
    }

    #[test]
    fn test_resize() {
        let mut state = dummy_state(800, 600, 1.0);
        assert_eq!(state.width, 800);
        assert_eq!(state.height, 600);

        state.resize(1920, 1080, 2.0);
        assert_eq!(state.width, 1920);
        assert_eq!(state.height, 1080);
        assert_eq!(state.scale_factor, 2.0);
    }

    #[test]
    fn test_new_dummy_creation() {
        let state = dummy_state(1024, 768, 1.0);
        assert_eq!(state.width, 1024);
        assert_eq!(state.height, 768);
        assert_eq!(state.scale_factor, 1.0);
    }
}
