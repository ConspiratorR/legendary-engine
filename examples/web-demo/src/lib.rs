use std::sync::Arc;
use wasm_bindgen::prelude::*;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowAttributes,
};

/// Phase 36 — SceneRuntime JSON smoke on wasm (no file I/O).
///
/// Phase 41: loads SceneData with `AnimationClipPlayer` (position track 0→6 @1s),
/// multi-ticks Time + SceneRuntime, returns start/end world X for overlay.
#[wasm_bindgen]
pub fn scene_runtime_json_smoke() -> Result<String, JsValue> {
    use engine_core::event::EventBus;
    use engine_core::sample_scripts::register_sample_scripts;
    use engine_core::scene_management::LoadSceneMode;
    use engine_core::scene_runtime::SceneRuntime;
    use engine_core::time::Time;
    use engine_core::world::World;

    register_sample_scripts();
    // SceneData TransformData keys: local_position / local_rotation / local_scale
    // ComponentData: type_name + properties.{script_type,enabled,props}
    let json = r#"{
      "name": "WasmSmoke",
      "version": 1,
      "game_objects": [{
        "name": "AnimProbe",
        "tag": "Untagged",
        "layer": 0,
        "active": true,
        "transform": {
          "local_position": [0.0, 0.0, 0.0],
          "local_rotation": [0.0, 0.0, 0.0, 1.0],
          "local_scale": [1.0, 1.0, 1.0]
        },
        "components": [{
          "type_name": "AnimationClipPlayer",
          "properties": {
            "script_type": "AnimationClipPlayer",
            "enabled": true,
            "props": {
              "clip": {
                "name": "wasm_move",
                "duration": 1.0,
                "looping": false,
                "position_track": [
                  {
                    "time": 0.0,
                    "value": [0.0, 0.0, 0.0],
                    "interpolation": "Linear",
                    "tangent_in": [0.0, 0.0, 0.0],
                    "tangent_out": [0.0, 0.0, 0.0]
                  },
                  {
                    "time": 1.0,
                    "value": [6.0, 0.0, 0.0],
                    "interpolation": "Linear",
                    "tangent_in": [0.0, 0.0, 0.0],
                    "tangent_out": [0.0, 0.0, 0.0]
                  }
                ]
              },
              "time": 0.0,
              "speed": 1.0,
              "playing": true
            }
          }
        }],
        "children": []
      }]
    }"#;

    let mut rt = SceneRuntime::new();
    rt.load_scene_json("WasmSmoke", json, LoadSceneMode::Single)
        .map_err(|e| JsValue::from_str(&e))?;
    rt.mark_needs_awake();

    let go = rt
        .world
        .Find("AnimProbe")
        .ok_or_else(|| JsValue::from_str("AnimProbe missing"))?;
    let start = rt
        .world
        .GetTransform(go)
        .ok_or_else(|| JsValue::from_str("transform missing"))?
        .LocalPosition();

    let mut time = Time::default();
    let mut bus = EventBus::new();
    let frames = 20u32;
    for _ in 0..frames {
        time.update(0.05);
        rt.tick(&time, time.frameCount(), &mut bus);
    }

    let end = rt
        .world
        .GetTransform(go)
        .ok_or_else(|| JsValue::from_str("transform missing after ticks"))?
        .LocalPosition();
    let flag = World::unity_world_primary_feature();
    Ok(format!(
        "anim tick ok; unity_world_primary={flag}; frames={frames}; start_x={:.2} end_x={:.2}",
        start.x, end.x
    ))
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(run());
}

async fn run() {
    let doc = web_sys::window().unwrap().document().unwrap();
    let status = doc.get_element_by_id("status").unwrap();

    status.set_text_content(Some("Creating window..."));

    // Phase 39: SceneRuntime JSON smoke **before** wgpu — evidence even if GPU fails.
    {
        let smoke_el = doc.get_element_by_id("scene-smoke");
        match scene_runtime_json_smoke() {
            Ok(msg) => {
                log::info!("{msg}");
                if let Some(el) = &smoke_el {
                    el.set_text_content(Some(&format!("SceneRuntime smoke: {msg}")));
                }
            }
            Err(e) => {
                log::warn!("scene_runtime_json_smoke failed: {e:?}");
                if let Some(el) = &smoke_el {
                    el.set_text_content(Some(&format!("SceneRuntime smoke FAILED: {e:?}")));
                    el.set_class_name("err");
                }
            }
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(
        event_loop
            .create_window(WindowAttributes::default().with_title("RustEngine Web"))
            .unwrap(),
    );

    status.set_text_content(Some("Initializing wgpu..."));

    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance.create_surface(window.clone()).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Failed to create device");

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps.formats[0];
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    status.set_text_content(Some("Ready! Rendering..."));

    // Smoke already written before wgpu (phase 39); hide centered loader only.
    let style = status
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .style();
    style.set_property("display", "none").unwrap();

    log::info!("RustEngine Web Demo initialized");

    let mut config = config;
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::RedrawRequested => {
                        let frame = surface.get_current_texture().unwrap();
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder = device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        {
                            let _render_pass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("clear_pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                                r: 0.1,
                                                g: 0.1,
                                                b: 0.18,
                                                a: 1.0,
                                            }),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });
                        }

                        queue.submit(Some(encoder.finish()));
                        frame.present();
                    }
                    WindowEvent::Resized(size) => {
                        if size.width > 0 && size.height > 0 {
                            config.width = size.width;
                            config.height = size.height;
                            surface.configure(&device, &config);
                        }
                    }
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}
