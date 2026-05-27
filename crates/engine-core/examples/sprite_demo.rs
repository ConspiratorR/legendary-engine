use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_math::{Mat4, Vec2, Vec3};
use engine_render::camera::{Camera, Color, Viewport};
use engine_render::renderer::Renderer;
use engine_render::sprite::Sprite;
use engine_render::texture_bridge::TextureBridge;
use engine_window::{window::WindowConfig, window::create_window};
use log::info;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Sprite Demo — TextureBridge");

    let event_loop = EventLoop::new().unwrap();
    let window = std::sync::Arc::new(create_window(
        &WindowConfig {
            title: "Sprite Demo — TextureBridge".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        },
        &event_loop,
    ));

    let renderer = Renderer::new(window).expect("Failed to create renderer");

    let mut bridge = TextureBridge::new(&renderer.device, &renderer.queue);

    let tex_asset = Texture {
        id: "test".into(),
        width: 1,
        height: 1,
        data: vec![0; 4],
        channels: 4,
    };
    let handle = Handle::new(tex_asset);
    bridge.request(&handle, "assets/test.png");

    bridge.on_loaded.subscribe(|e| {
        info!("Texture loaded: {:?} → {:?}", e.handle_id, e.result);
    });

    let sprites = vec![
        Sprite {
            texture: handle.clone(),
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            transform: Mat4::from_translation(Vec3::new(200.0, 300.0, 0.0)),
            flip_x: false,
            flip_y: false,
        },
        Sprite {
            texture: handle.clone(),
            color: [0.0, 1.0, 0.0, 1.0],
            size: Vec2::new(128.0, 128.0),
            transform: Mat4::from_translation(Vec3::new(600.0, 300.0, 0.0)),
            flip_x: false,
            flip_y: false,
        },
    ];

    let mut main_camera = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
    main_camera.priority = 0;
    main_camera.clear_color = Some(Color::new(0.1, 0.1, 0.1, 1.0));

    let mut mini_camera = Camera::orthographic(0.0, 400.0, 300.0, 0.0);
    mini_camera.priority = 1;
    mini_camera.viewport = Viewport::Relative {
        x: 0.6,
        y: 0.0,
        width: 0.4,
        height: 0.4,
    };
    mini_camera.clear_color = Some(Color::new(0.2, 0.2, 0.3, 1.0));

    let mut renderer = renderer;
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match &event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    renderer.resize(size.width, size.height);
                }
                _ => {}
            }

            if let Event::AboutToWait = event {
                let cameras: Vec<&Camera> = vec![&main_camera, &mini_camera];
                let _ = renderer.render_frame(&cameras, &sprites, &mut bridge);
            }
        })
        .unwrap();
}
