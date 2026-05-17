use egui::Context;
use egui_wgpu::{Renderer, ScreenDescriptor};

pub struct EguiState {
    pub ctx: Context,
    renderer: Option<Renderer>,
    screen_descriptor: ScreenDescriptor,
    width: u32,
    height: u32,
    scale_factor: f32,
    mouse_pos: (f64, f64),
    mouse_down: bool,
    just_pressed: bool,
    just_released: bool,
}

impl EguiState {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        scale_factor: f32,
    ) -> Self {
        let ctx = Context::default();
        let renderer = Renderer::new(device, config.format, None, 0, false);
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
            mouse_down: false,
            just_pressed: false,
            just_released: false,
        }
    }

    #[allow(deprecated)]
    pub fn begin_frame(&mut self, time: f64) {
        let mut events = Vec::new();
        let pos = egui::pos2(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);

        events.push(egui::Event::PointerMoved(pos));

        if self.just_pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            });
            self.just_pressed = false;
        }
        if self.just_released {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            });
            self.just_released = false;
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

        let renderer = self.renderer.as_mut().unwrap();

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

        queue.submit([encoder.finish()]);

        for id in &textures_delta.free {
            renderer.free_texture(id);
        }
    }

    pub fn handle_mouse_move(&mut self, x: f64, y: f64) {
        self.mouse_pos = (x, y);
    }

    pub fn press(&mut self) {
        if !self.mouse_down {
            self.just_pressed = true;
            self.mouse_down = true;
        }
    }

    pub fn release(&mut self) {
        if self.mouse_down {
            self.just_released = true;
            self.mouse_down = false;
        }
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
            mouse_down: false,
            just_pressed: false,
            just_released: false,
        }
    }

    #[test]
    fn test_mouse_tracking() {
        let mut state = dummy_state(1024, 768, 1.0);
        assert_eq!(state.mouse_pos, (0.0, 0.0));
        assert!(!state.mouse_down);

        state.handle_mouse_move(100.0, 200.0);
        assert_eq!(state.mouse_pos, (100.0, 200.0));

        state.press();
        assert!(state.mouse_down);
        assert!(state.just_pressed);

        state.press();
        assert!(state.mouse_down);
        assert!(state.just_pressed);

        state.release();
        assert!(!state.mouse_down);
        assert!(state.just_released);
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
