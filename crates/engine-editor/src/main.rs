use engine_editor::state::EditorState;
use engine_render::renderer::Renderer;
use engine_ui::{EguiState, GuiSkin};
use engine_window::{WindowConfig, create_window};
use log::info;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

#[derive(Clone, Copy)]
struct Modifiers {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

#[allow(deprecated, unused_assignments)]
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
    )?);

    // Pollster (for async)
    let mut render = None;
    let mut egui_state = None;
    let mut viewport_renderer_opt = None;
    let mut hot_reload_opt = None;
    let mut editor_state = EditorState::new();
    let mut runtime_world: Option<engine_ecs::world::World> = None;
    let mut _runtime_audio: Option<engine_audio::audio_manager::AudioManager> = None;
    let mut prev_play_state = engine_editor::state::PlayState::Editing;
    let mut window_modifiers = Modifiers { ctrl: false, shift: false, alt: false };
    let start_time = std::time::Instant::now();

    pollster::block_on(async {
        // Initialize renderer
        let renderer =
            Renderer::new(std::sync::Arc::clone(&window)).expect("Failed to create renderer");
        let scale_factor = window.scale_factor() as f32;
        let egui_state_local = EguiState::new(&renderer.device, &renderer.config, scale_factor);

        // Initialize ViewportRenderer
        let vp_renderer = std::sync::Arc::new(std::sync::Mutex::new(
            engine_editor::viewport_renderer::ViewportRenderer::new(
                renderer.device.0.clone(),
                renderer.queue.0.clone(),
            ),
        ));

        // Initialize hot reload manager
        let hot_reload = std::sync::Arc::new(std::sync::Mutex::new(
            engine_editor::hot_reload::ReloadManager::new(std::path::Path::new("assets"))
                .unwrap_or_else(|e| {
                    log::warn!("Failed to init hot reload: {}", e);
                    engine_editor::hot_reload::ReloadManager::new(std::path::Path::new("."))
                        .unwrap()
                }),
        ));

        render = Some(renderer);
        egui_state = Some(egui_state_local);
        viewport_renderer_opt = Some(vp_renderer);
        hot_reload_opt = Some(hot_reload);

        // Log startup messages
        editor_state.log_info("编辑器已启动");
        editor_state.log_info("项目已加载: RustEngine");
        editor_state.log_info("着色器编译完成");
        editor_state.log_info("渲染器初始化成功 (wgpu + 延迟渲染管线)");

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

                                // Step runtime if playing
                                if editor_state.play_state
                                    == engine_editor::state::PlayState::Playing
                                    && let Some(ref mut world) = runtime_world
                                {
                                    editor_state.step_runtime(world, dt as f32);
                                }

                                // Check autosave
                                if editor_state.check_autosave(dt as f32) {
                                    if let Err(e) = editor_state.scene_manager.save_current_scene()
                                    {
                                        log::warn!("Autosave failed: {}", e);
                                    } else {
                                        log::info!("Autosave completed");
                                    }
                                }

                                // Begin frame (pass absolute time, not delta)
                                let elapsed = (now - start_time).as_secs_f64();
                                e.begin_frame(elapsed);

                                // Update editor with renderer context
                                let mut vp_guard = viewport_renderer_opt
                                    .as_ref()
                                    .unwrap()
                                    .lock()
                                    .unwrap();
                                let egui_ctx = e.ctx().clone();
                                editor_state.frame(
                                    &egui_ctx,
                                    &GuiSkin::default(),
                                    r,
                                    &mut vp_guard,
                                    e,
                                );

                                // Manage runtime world on play state transitions
                                use engine_editor::state::PlayState;
                                match (prev_play_state, editor_state.play_state) {
                                    (PlayState::Editing, PlayState::Playing) => {
                                        // Entering play mode: create runtime world
                                        runtime_world =
                                            Some(editor_state.build_runtime_world());
                                        // Create audio manager for runtime
                                        match engine_audio::audio_manager::AudioManager::new() {
                                            Ok(mut am) => {
                                                // Collect audio sources to play
                                                let audio_sources: Vec<(String, bool)> = editor_state.scene_tree.nodes.iter()
                                                    .filter_map(|node| {
                                                        editor_state.node_audio.get(&node.id).map(|a| {
                                                            (a.source.clone(), a.source.contains("music") || a.source.contains("bg"))
                                                        })
                                                    })
                                                    .filter(|(source, _)| !source.is_empty())
                                                    .collect();

                                                // Play audio for nodes with AudioData
                                                for (source, is_music) in audio_sources {
                                                    let channel = if is_music {
                                                        engine_audio::audio_manager::AudioChannel::Music
                                                    } else {
                                                        engine_audio::audio_manager::AudioChannel::Sfx
                                                    };
                                                    match am.play(&source, channel) {
                                                        Ok(handle) => {
                                                            editor_state.log_info(&format!("播放音频: {} (handle: {:?})", source, handle));
                                                        }
                                                        Err(e) => {
                                                            editor_state.log_warn(&format!("音频播放失败 {}: {}", source, e));
                                                        }
                                                    }
                                                }
                                                _runtime_audio = Some(am);
                                                editor_state.log_info("音频系统已初始化");
                                            }
                                            Err(e) => {
                                                editor_state.log_warn(&format!("音频初始化失败: {}", e));
                                            }
                                        }
                                        editor_state.log_info("运行时世界已创建");
                                    }
                                    (_, PlayState::Editing) if prev_play_state != PlayState::Editing => {
                                        // Leaving play mode: destroy runtime world and audio
                                        runtime_world = None;
                                        _runtime_audio = None;
                                        editor_state.log_info("运行时世界已销毁");
                                    }
                                    _ => {}
                                }
                                prev_play_state = editor_state.play_state;

                                // End frame and render
                                let (paint_jobs, textures_delta) = e.end_frame();

                                // Render
                                match r.surface.get_current_texture() {
                                    Ok(output) => {
                                        e.paint(
                                            &r.device,
                                            &r.queue,
                                            &output,
                                            &paint_jobs,
                                            &textures_delta,
                                        );
                                        output.present();
                                    }
                                    Err(wgpu::SurfaceError::Lost) => {
                                        let size = window.inner_size();
                                        r.resize(size.width, size.height);
                                        log::warn!("Surface lost, recreated");
                                    }
                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        log::error!("Out of GPU memory!");
                                        elwt.exit();
                                    }
                                    Err(e) => {
                                        log::warn!("Surface error: {:?}", e);
                                    }
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
                        WindowEvent::KeyboardInput {
                            event:
                                winit::event::KeyEvent {
                                    physical_key: winit::keyboard::PhysicalKey::Code(keycode),
                                    state: winit::event::ElementState::Pressed,
                                    ..
                                },
                            ..
                        } => {
                            // engine_input::KeyCode is a re-export of winit::keyboard::KeyCode
                            let mods = window_modifiers;
                            if let Some(action) = editor_state
                                .shortcuts
                                .get_action(keycode, mods.ctrl, mods.shift, mods.alt)
                            {
                                editor_state.handle_shortcut(action);
                            }
                        }
                        WindowEvent::ModifiersChanged(modifiers) => {
                            window_modifiers = Modifiers {
                                ctrl: modifiers.state().control_key(),
                                shift: modifiers.state().shift_key(),
                                alt: modifiers.state().alt_key(),
                            };
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
