use engine_editor::state::EditorState;
use engine_render::renderer::Renderer;
use engine_ui::{EguiState, GuiSkin};
use engine_window::{WindowConfig, create_window};
use log::info;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting RustEngine Editor");

    // Create event loop and window
    let event_loop = EventLoop::new()?;
    let window = std::sync::Arc::new(create_window(
        &WindowConfig {
            title: "RustEngine Editor".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        },
        &event_loop,
    ));

    // Pollster (for async)
    let mut render = None;
    let mut egui_state = None;
    let mut editor_state = EditorState::new();

    pollster::block_on(async {
        // Initialize renderer
        let renderer =
            Renderer::new(std::sync::Arc::clone(&window)).expect("Failed to create renderer");
        let scale_factor = window.scale_factor() as f32;
        let mut egui_state_local = EguiState::new(&renderer.device, &renderer.config, scale_factor);

        render = Some(renderer);
        egui_state = Some(egui_state_local);

        // Run the event loop
        let mut last_time = std::time::Instant::now();
        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        WindowEvent::Resized(size) => {
                            if let (Some(r), Some(e)) = (&mut render, &mut egui_state) {
                                r.resize(size.width, size.height);
                                let scale = window.scale_factor() as f32;
                                e.resize(size.width, size.height, scale);
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            if let (Some(r), Some(e)) = (&mut render, &mut egui_state) {
                                let now = std::time::Instant::now();
                                let dt = (now - last_time).as_secs_f64();
                                last_time = now;

                                // Begin frame
                                e.begin_frame(dt);

                                // Update editor
                                editor_state.frame(&e.ctx(), &GuiSkin::default());

                                // End frame and render
                                let (paint_jobs, textures_delta) = e.end_frame();

                                // Render
                                if let Ok(output) = r.surface.get_current_texture() {
                                    e.paint(
                                        &r.device,
                                        &r.queue,
                                        &output,
                                        &paint_jobs,
                                        &textures_delta,
                                    );
                                    output.present();
                                }
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            if let Some(e) = &mut egui_state {
                                e.handle_mouse_move(position.x, position.y);
                            }
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            if let Some(e) = &mut egui_state {
                                let idx = match button {
                                    winit::event::MouseButton::Left => 0,
                                    winit::event::MouseButton::Right => 1,
                                    winit::event::MouseButton::Middle => 2,
                                    _ => 0,
                                };
                                match state {
                                    winit::event::ElementState::Pressed => e.press_button(idx),
                                    winit::event::ElementState::Released => e.release_button(idx),
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
    })?;

    Ok(())
}
