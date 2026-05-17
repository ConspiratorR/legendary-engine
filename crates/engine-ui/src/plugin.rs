use crate::EguiState;
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;
use engine_input::input_manager::InputManager;
use engine_render::renderer::{GpuDevice, GpuQueue};

pub struct EguiPlugin;

impl Plugin for EguiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(EguiInitFlag(false));
        app.add_pre_update_hook(Box::new(|app: &mut App| {
            let is_init = { app.resources.get::<EguiInitFlag>().unwrap().0 };
            if !is_init {
                let (renderer, resources) = app.split_renderer_ref();
                if let Some(r) = renderer {
                    resources.insert(EguiState::new(&r.device, &r.config, 1.0));
                }
                resources.get_mut::<EguiInitFlag>().unwrap().0 = true;
            }

            let (mouse_x, mouse_y, left_down, right_down, middle_down) = {
                let input = app.resources.get::<InputManager>().unwrap();
                (
                    input.mouse().position.0,
                    input.mouse().position.1,
                    input.mouse().left_button,
                    input.mouse().right_button,
                    input.mouse().middle_button,
                )
            };
            {
                let egui_state = app.resources.get_mut::<EguiState>().unwrap();
                egui_state.handle_mouse_move(mouse_x, mouse_y);
                if left_down {
                    egui_state.press_button(0);
                } else {
                    egui_state.release_button(0);
                }
                if right_down {
                    egui_state.press_button(1);
                } else {
                    egui_state.release_button(1);
                }
                if middle_down {
                    egui_state.press_button(2);
                } else {
                    egui_state.release_button(2);
                }
            }

            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            {
                let egui_state = app.resources.get_mut::<EguiState>().unwrap();
                egui_state.begin_frame(time);
            }

            {
                let (renderer, resources) = app.split_renderer_ref();
                if let Some(r) = renderer
                    && let Some(egui_state) = resources.get_mut::<EguiState>()
                {
                    egui_state.resize(r.config.width, r.config.height, 1.0);
                }
            }
        }));
        app.add_post_render_hook(Box::new(|app: &mut App| {
            let (renderer, resources) = app.split_renderer_mut();
            if resources.get::<EguiState>().is_none() {
                return;
            }

            let (paint_jobs, textures_delta) = {
                let egui_state = resources.get_mut::<EguiState>().unwrap();
                egui_state.end_frame()
            };

            let device = resources.get::<GpuDevice>().unwrap().clone();
            let queue = resources.get::<GpuQueue>().unwrap().clone();

            let renderer_ref = match renderer {
                Some(r) => r,
                None => return,
            };

            let output = match renderer_ref.surface.get_current_texture() {
                Ok(o) => o,
                Err(wgpu::SurfaceError::Timeout) => return,
                Err(wgpu::SurfaceError::Outdated) => {
                    renderer_ref
                        .surface
                        .configure(&device, &renderer_ref.config);
                    return;
                }
                Err(wgpu::SurfaceError::Lost) => {
                    renderer_ref
                        .surface
                        .configure(&device, &renderer_ref.config);
                    return;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => return,
            };

            resources.get_mut::<EguiState>().unwrap().paint(
                &device,
                &queue,
                &output,
                &paint_jobs,
                &textures_delta,
            );

            output.present();
        }));
    }
}

struct EguiInitFlag(bool);
