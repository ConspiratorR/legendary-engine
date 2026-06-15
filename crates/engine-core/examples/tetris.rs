use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;
use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::{Mat4, Vec2, Vec3};
use engine_render::camera::{Camera, Color};
use engine_render::plugin::RenderPlugin2D;
use engine_render::sprite::Sprite;
use engine_render::texture_bridge::TextureBridge;
use engine_window::{window::WindowConfig, window::create_window};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

// ── Grid ──────────────────────────────────────────────────
const COLS: usize = 10;
const ROWS: usize = 22;
const VISIBLE_ROWS: usize = 20;
const CELL: f32 = 34.0;
const BLK: f32 = 32.0;

// ── Window & Layout ──────────────────────────────────────
const WIN_W: u32 = 1100;
const WIN_H: u32 = 800;
const GX: f32 = 350.0;
const GY_TOP: f32 = 60.0;
const HOLD_X: f32 = 100.0;
const HOLD_Y: f32 = 180.0;
const HOLD_CELL: f32 = 30.0;
const HOLD_BLK: f32 = 28.0;
const NEXT_X: f32 = 730.0;
const NEXT_Y: f32 = 100.0;
const NEXT_CELL: f32 = 30.0;
const NEXT_BLK: f32 = 28.0;
const NEXT_GAP: f32 = 24.0;
const SCORE_X: f32 = 730.0;
const SCORE_Y: f32 = 530.0;
const LEVEL_X: f32 = 730.0;
const LEVEL_Y: f32 = 600.0;
const LINES_X: f32 = 730.0;
const LINES_Y: f32 = 670.0;

// ── Pixel Font ───────────────────────────────────────────
const PX_SZ: f32 = 8.0;
const PX_GAP: f32 = 1.0;
const DIGIT_W: f32 = 3.0 * PX_SZ + 2.0 * PX_GAP;
const DIGIT_SP: f32 = 5.0;

// ── Entity Budget ────────────────────────────────────────
const GRID_ENT: usize = COLS * VISIBLE_ROWS;
const HOLD_ENT: usize = 16;
const NEXT_ENT: usize = 48;
const BG_ENT: usize = 10;
const SCORE_DIGITS: usize = 7;
const LEVEL_DIGITS: usize = 2;
const LINES_DIGITS: usize = 3;
const FONT_ENT: usize = (SCORE_DIGITS + LEVEL_DIGITS + LINES_DIGITS) * 15;
const TOTAL_ENTITIES: usize = GRID_ENT + HOLD_ENT + NEXT_ENT + BG_ENT + FONT_ENT;
const GRID_OFF: usize = 0;
const HOLD_OFF: usize = GRID_OFF + GRID_ENT;
const NEXT_OFF: usize = HOLD_OFF + HOLD_ENT;
const BG_OFF: usize = NEXT_OFF + NEXT_ENT;
const SCORE_OFF: usize = BG_OFF + BG_ENT;
const LEVEL_OFF: usize = SCORE_OFF + SCORE_DIGITS * 15;
const LINES_OFF: usize = LEVEL_OFF + LEVEL_DIGITS * 15;

// ── Timing ───────────────────────────────────────────────
const DAS_DELAY: f32 = 0.167;
const DAS_REPEAT: f32 = 0.033;
const LOCK_DELAY: f32 = 0.5;
const MAX_LOCK_RESETS: u32 = 15;
const SOFT_DROP_SPEED: f32 = 0.04;

// ── Colors ───────────────────────────────────────────────
const COLORS: [[f32; 4]; 8] = [
    [0.06, 0.06, 0.12, 1.0],
    [0.0, 0.95, 0.95, 1.0],
    [0.98, 0.95, 0.0, 1.0],
    [0.78, 0.0, 1.0, 1.0],
    [0.0, 0.95, 0.3, 1.0],
    [1.0, 0.15, 0.15, 1.0],
    [0.15, 0.4, 1.0, 1.0],
    [1.0, 0.6, 0.0, 1.0],
];
const BORDER_CLR: [f32; 4] = [0.28, 0.28, 0.45, 1.0];
const PANEL_BG_CLR: [f32; 4] = [0.05, 0.05, 0.10, 1.0];
const SCORE_CLR: [f32; 4] = [0.92, 0.92, 0.96, 1.0];
const LEVEL_CLR: [f32; 4] = [0.0, 0.85, 0.85, 1.0];
const LINES_CLR: [f32; 4] = [0.95, 0.9, 0.0, 1.0];

// ── Pixel Font Digits (3x5 bitmaps, row-major) ──────────
const DIGITS: [[u8; 15]; 10] = [
    [1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1],
    [0, 1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 1],
    [1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1],
    [1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1],
    [1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1],
    [1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1],
    [1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1],
    [1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1],
    [1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1],
    [1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1],
];

// ── Piece Definitions ────────────────────────────────────
const PIECES: [[[(i32, i32); 4]; 4]; 7] = [
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ],
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];

// ── SRS Kick Tables ──────────────────────────────────────
const SRS_KICKS_NORMAL: [[[(i32, i32); 5]; 4]; 4] = [
    [
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    ],
    [
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    ],
    [
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    ],
    [
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    ],
];
const SRS_KICKS_I: [[[(i32, i32); 5]; 4]; 4] = [
    [
        [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
        [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
        [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
        [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
    ],
    [
        [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
        [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
        [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
        [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
    ],
    [
        [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
        [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
        [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
        [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
    ],
    [
        [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
        [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
        [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
        [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
    ],
];

// ── 7-Bag Randomizer ────────────────────────────────────
struct Bag {
    pieces: VecDeque<usize>,
}
impl Bag {
    fn new() -> Self {
        let mut b = Bag {
            pieces: VecDeque::new(),
        };
        b.fill();
        b
    }
    fn next(&mut self) -> usize {
        if self.pieces.is_empty() {
            self.fill();
        }
        self.pieces.pop_front().unwrap()
    }
    fn fill(&mut self) {
        let mut order: Vec<usize> = (0..7).collect();
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as usize;
        for i in (1..7).rev() {
            let j = seed.wrapping_mul(31).wrapping_add(i) % (i + 1);
            order.swap(i, j);
        }
        self.pieces.extend(order);
    }
}

// ── Game State ───────────────────────────────────────────
struct Game {
    grid: [[u8; COLS]; ROWS],
    piece: usize,
    px: i32,
    py: i32,
    rot: usize,
    hold: Option<usize>,
    hold_used: bool,
    next_queue: VecDeque<usize>,
    bag: Bag,
    score: u32,
    level: u32,
    lines: u32,
    combo: i32,
    over: bool,
    paused: bool,
    fall_acc: f32,
    fall_speed: f32,
    lock_acc: f32,
    lock_resets: u32,
    lock_active: bool,
    das_dir: i32,
    das_acc: f32,
    das_charged: bool,
    soft_drop: bool,
    clear_flash: f32,
    clear_rows: Vec<usize>,
}

// ── Main ─────────────────────────────────────────────────
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    println!(
        "\n========================================\n           T E T R I S\n========================================\n\n  Left/Right/A/D  Move\n  Up/X            Rotate CW\n  Z               Rotate CCW\n  Down/S          Soft drop\n  Space           Hard drop\n  C               Hold\n  P               Pause\n  Esc             Restart\n\n========================================\n"
    );

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = Arc::new(
        create_window(
            &WindowConfig {
                title: "Tetris".to_string(),
                width: WIN_W,
                height: WIN_H,
                vsync: true,
            },
            &event_loop,
        )
        .unwrap(),
    );

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);

    let tex = Texture {
        id: "w".into(),
        width: 1,
        height: 1,
        data: vec![255, 255, 255, 255],
        channels: 4,
        asset_path: PathBuf::new(),
    };
    let tex_handle = Handle::new(tex);

    let world = builder.world_mut();
    let cam = world.spawn();
    let mut cam_comp = Camera::orthographic(0.0, WIN_W as f32, WIN_H as f32, 0.0);
    cam_comp.clear_color = Some(Color::new(0.02, 0.02, 0.04, 1.0));
    world.add_component(cam, cam_comp);

    let mut entities = Vec::with_capacity(TOTAL_ENTITIES);
    for _ in 0..TOTAL_ENTITIES {
        let e = world.spawn();
        world.add_component(
            e,
            Sprite {
                texture: tex_handle.clone(),
                color: [0.0; 4],
                size: Vec2::new(BLK, BLK),
                transform: Mat4::from_translation(Vec3::new(-300.0, -300.0, 0.0)),
                flip_x: false,
                flip_y: false,
                uv_region: [0.0, 0.0, 1.0, 1.0],
            },
        );
        entities.push(e);
    }

    let mut bag = Bag::new();
    let mut nq = VecDeque::new();
    for _ in 0..4 {
        nq.push_back(bag.next());
    }
    let first = nq.pop_front().unwrap();
    nq.push_back(bag.next());

    let mut game = Game {
        grid: [[0; COLS]; ROWS],
        piece: first,
        px: 3,
        py: 0,
        rot: 0,
        hold: None,
        hold_used: false,
        next_queue: nq,
        bag,
        score: 0,
        level: 1,
        lines: 0,
        combo: -1,
        over: false,
        paused: false,
        fall_acc: 0.0,
        fall_speed: 0.8,
        lock_acc: 0.0,
        lock_resets: 0,
        lock_active: false,
        das_dir: 0,
        das_acc: 0.0,
        das_charged: false,
        soft_drop: false,
        clear_flash: 0.0,
        clear_rows: Vec::new(),
    };

    let mut plugin = RenderPlugin2D::new(window.clone());
    plugin.build(builder.world_mut());
    {
        builder
            .world_mut()
            .get_resource_mut::<TextureBridge>()
            .unwrap()
            .request(&tex_handle, "");
    }
    let renderer = plugin.take_renderer().unwrap();
    let mut app = builder.build();
    app.set_renderer(renderer);

    #[allow(deprecated)]
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);
            match &event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::Resized(s),
                    ..
                } => {
                    if let Some(r) = app.renderer_mut() {
                        r.resize(s.width, s.height);
                    }
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::KeyboardInput { event: ke, .. },
                    ..
                } => {
                    let input = app.input_mut();
                    if let winit::keyboard::PhysicalKey::Code(key) = ke.physical_key {
                        if ke.state == winit::event::ElementState::Pressed {
                            input.press(key);
                        } else {
                            input.release(key);
                        }
                    }
                }
                _ => {}
            }
            if let winit::event::Event::AboutToWait = event {
                let dt = app
                    .world
                    .get_resource::<Time>()
                    .map(|t| t.delta_seconds().min(0.05))
                    .unwrap_or(0.016);
                let input_ptr = app.world.get_resource::<InputManager>().unwrap() as *const _;
                let input = unsafe { &*input_ptr };
                process_input(input, &mut game, dt);
                update(&mut game, dt);
                redraw(&mut app.world, &game, &entities);
                window.set_title(&format!(
                    "Tetris | Score {} | Lv {} | Lines {}",
                    game.score, game.level, game.lines
                ));
                app.run();
                app.render_phase();
            }
        })
        .unwrap();
}

// ── Input ────────────────────────────────────────────────
fn process_input(input: &InputManager, g: &mut Game, dt: f32) {
    if input.key_just_pressed(KeyCode::Escape) {
        if g.over {
            reset(g);
        } else {
            g.over = true;
        }
        return;
    }
    if input.key_just_pressed(KeyCode::KeyP) {
        g.paused = !g.paused;
        return;
    }
    if g.over || g.paused {
        return;
    }
    if input.key_just_pressed(KeyCode::ArrowUp) || input.key_just_pressed(KeyCode::KeyX) {
        rotate(g, 1);
    }
    if input.key_just_pressed(KeyCode::KeyZ) {
        rotate(g, 3);
    }
    if input.key_just_pressed(KeyCode::Space) {
        let mut dist = 0;
        while fits(g, g.piece, g.rot, g.px, g.py + 1) {
            g.py += 1;
            dist += 1;
        }
        g.score += dist * 2;
        lock_piece(g);
        return;
    }
    if input.key_just_pressed(KeyCode::KeyC) && !g.hold_used {
        let cur = g.piece;
        if let Some(h) = g.hold {
            g.piece = h;
        } else {
            spawn_next(g);
        }
        g.hold = Some(cur);
        g.hold_used = true;
        g.px = 3;
        g.py = 0;
        g.rot = 0;
        g.lock_acc = 0.0;
        g.lock_resets = 0;
        g.lock_active = false;
    }
    g.soft_drop = input.key_down(KeyCode::ArrowDown) || input.key_down(KeyCode::KeyS);
    if (input.key_just_pressed(KeyCode::ArrowDown) || input.key_just_pressed(KeyCode::KeyS))
        && try_move(g, 0, 1)
    {
        g.score += 1;
    }
    let left = input.key_down(KeyCode::ArrowLeft) || input.key_down(KeyCode::KeyA);
    let right = input.key_down(KeyCode::ArrowRight) || input.key_down(KeyCode::KeyD);
    let left_just =
        input.key_just_pressed(KeyCode::ArrowLeft) || input.key_just_pressed(KeyCode::KeyA);
    let right_just =
        input.key_just_pressed(KeyCode::ArrowRight) || input.key_just_pressed(KeyCode::KeyD);
    if left_just {
        try_move(g, -1, 0);
        g.das_dir = -1;
        g.das_acc = 0.0;
        g.das_charged = false;
    } else if right_just {
        try_move(g, 1, 0);
        g.das_dir = 1;
        g.das_acc = 0.0;
        g.das_charged = false;
    } else if left && g.das_dir == -1 {
        g.das_acc += dt;
        if !g.das_charged {
            if g.das_acc >= DAS_DELAY {
                g.das_charged = true;
                g.das_acc = 0.0;
                try_move(g, -1, 0);
            }
        } else {
            while g.das_acc >= DAS_REPEAT {
                g.das_acc -= DAS_REPEAT;
                try_move(g, -1, 0);
            }
        }
    } else if right && g.das_dir == 1 {
        g.das_acc += dt;
        if !g.das_charged {
            if g.das_acc >= DAS_DELAY {
                g.das_charged = true;
                g.das_acc = 0.0;
                try_move(g, 1, 0);
            }
        } else {
            while g.das_acc >= DAS_REPEAT {
                g.das_acc -= DAS_REPEAT;
                try_move(g, 1, 0);
            }
        }
    }
    if !left && !right {
        g.das_dir = 0;
        g.das_acc = 0.0;
        g.das_charged = false;
    }
}

// ── Update ───────────────────────────────────────────────
fn update(g: &mut Game, dt: f32) {
    if g.over || g.paused {
        return;
    }
    if g.clear_flash > 0.0 {
        g.clear_flash -= dt * 8.0;
        if g.clear_flash <= 0.0 {
            g.clear_flash = 0.0;
            actually_clear(g);
        }
        return;
    }
    let speed = if g.soft_drop {
        SOFT_DROP_SPEED
    } else {
        g.fall_speed
    };
    g.fall_acc += dt;
    if g.fall_acc >= speed {
        g.fall_acc -= speed;
        if try_move(g, 0, 1) {
            if g.soft_drop {
                g.score += 1;
            }
            if g.lock_active {
                g.lock_acc = 0.0;
            }
        } else if !g.lock_active {
            g.lock_active = true;
            g.lock_acc = 0.0;
        }
    }
    if g.lock_active {
        g.lock_acc += dt;
        if g.lock_acc >= LOCK_DELAY {
            lock_piece(g);
        }
    }
}

// ── Rendering ────────────────────────────────────────────
fn sprite_center(left: f32, top: f32, sz: f32) -> (f32, f32) {
    (left + sz * 0.5, top + sz * 0.5)
}

fn redraw(world: &mut World, g: &Game, entities: &[Entity]) {
    let ghost_y = ghost_y(g);
    let over_dim = if g.over { 0.3 } else { 1.0 };
    let grid_w = COLS as f32 * CELL;
    let grid_h = VISIBLE_ROWS as f32 * CELL;

    // ── Background sprites ───────────────────────────────
    set_bg(
        world,
        entities,
        BG_OFF,
        BORDER_CLR,
        Vec2::new(grid_w + 8.0, grid_h + 8.0),
        GX + grid_w * 0.5,
        GY_TOP + grid_h * 0.5,
    );
    set_bg(
        world,
        entities,
        BG_OFF + 1,
        PANEL_BG_CLR,
        Vec2::new(4.0 * HOLD_CELL + 24.0, 4.0 * HOLD_CELL + 50.0),
        HOLD_X + 2.0 * HOLD_CELL,
        HOLD_Y + 2.0 * HOLD_CELL + 7.0,
    );
    let nh = 3.0 * (4.0 * NEXT_CELL) + 2.0 * NEXT_GAP + 28.0;
    set_bg(
        world,
        entities,
        BG_OFF + 2,
        PANEL_BG_CLR,
        Vec2::new(4.0 * NEXT_CELL + 24.0, nh),
        NEXT_X + 2.0 * NEXT_CELL,
        NEXT_Y + nh * 0.5 - 14.0,
    );
    let sph = LINES_Y - SCORE_Y + 44.0 + 20.0;
    set_bg(
        world,
        entities,
        BG_OFF + 3,
        PANEL_BG_CLR,
        Vec2::new(SCORE_DIGITS as f32 * (DIGIT_W + DIGIT_SP) + 20.0, sph),
        SCORE_X + SCORE_DIGITS as f32 * (DIGIT_W + DIGIT_SP) * 0.5,
        SCORE_Y + sph * 0.5 - 10.0,
    );
    // Score indicators (colored dots)
    set_bg(
        world,
        entities,
        BG_OFF + 4,
        SCORE_CLR,
        Vec2::new(10.0, 10.0),
        SCORE_X - 16.0,
        SCORE_Y + 22.0,
    );
    set_bg(
        world,
        entities,
        BG_OFF + 5,
        LEVEL_CLR,
        Vec2::new(10.0, 10.0),
        LEVEL_X - 16.0,
        LEVEL_Y + 22.0,
    );
    set_bg(
        world,
        entities,
        BG_OFF + 6,
        LINES_CLR,
        Vec2::new(10.0, 10.0),
        LINES_X - 16.0,
        LINES_Y + 22.0,
    );
    for i in 7..BG_ENT {
        hide(world, entities, BG_OFF + i);
    }

    // ── Grid cells ───────────────────────────────────────
    for row in 0..VISIBLE_ROWS {
        let grid_row = row + (ROWS - VISIBLE_ROWS);
        for col in 0..COLS {
            let idx = GRID_OFF + row * COLS + col;
            let e = entities[idx];
            let cell = g.grid[grid_row][col];
            let is_cur = !g.over
                && cell == 0
                && piece_at(g, col as i32, grid_row as i32, g.piece, g.rot, g.px, g.py);
            let is_ghost = !g.over
                && cell == 0
                && !is_cur
                && piece_at(
                    g,
                    col as i32,
                    grid_row as i32,
                    g.piece,
                    g.rot,
                    g.px,
                    ghost_y,
                );
            let is_clearing = g.clear_rows.contains(&grid_row);
            let cl = GX + col as f32 * CELL;
            let ct = GY_TOP + row as f32 * CELL;

            let (color, size) = if is_clearing {
                let progress = 1.0 - g.clear_flash;
                let sweep = progress * COLS as f32;
                if (col as f32) < sweep {
                    let a = g.clear_flash;
                    ([a, a, a * 1.05, 1.0], Vec2::new(CELL, CELL))
                } else {
                    let c = COLORS[cell as usize];
                    let b = 1.0 + g.clear_flash * 0.6;
                    (
                        [
                            (c[0] * b).min(1.0),
                            (c[1] * b).min(1.0),
                            (c[2] * b).min(1.0),
                            1.0,
                        ],
                        Vec2::new(BLK, BLK),
                    )
                }
            } else if cell != 0 {
                let c = COLORS[cell as usize];
                (
                    [c[0] * over_dim, c[1] * over_dim, c[2] * over_dim, 1.0],
                    Vec2::new(BLK, BLK),
                )
            } else if is_cur {
                (COLORS[g.piece + 1], Vec2::new(BLK, BLK))
            } else if is_ghost {
                let c = COLORS[g.piece + 1];
                ([c[0], c[1], c[2], 0.18], Vec2::new(CELL - 2.0, CELL - 2.0))
            } else {
                (COLORS[0], Vec2::new(CELL, CELL))
            };
            let (cx, cy) = sprite_center(cl, ct, CELL);
            if let Some(sp) = world.get_by_index_mut::<Sprite>(e.index()) {
                sp.color = color;
                sp.size = size;
                sp.transform = Mat4::from_translation(Vec3::new(cx, cy, 0.0));
            }
        }
    }

    // ── Next pieces ──────────────────────────────────────
    for qi in 0..3 {
        let blocks = PIECES[g.next_queue[qi]][0];
        let color = COLORS[g.next_queue[qi] + 1];
        for i in 0..16 {
            let ei = NEXT_OFF + qi * 16 + i;
            let e = entities[ei];
            let col = i % 4;
            let row = i / 4;
            let is_block = blocks
                .iter()
                .any(|&(bx, by)| bx as usize == col && by as usize == row);
            if let Some(sp) = world.get_by_index_mut::<Sprite>(e.index()) {
                if is_block {
                    let px = NEXT_X + col as f32 * NEXT_CELL;
                    let py =
                        NEXT_Y + qi as f32 * (4.0 * NEXT_CELL + NEXT_GAP) + row as f32 * NEXT_CELL;
                    sp.color = color;
                    sp.size = Vec2::new(NEXT_BLK, NEXT_BLK);
                    let (cx, cy) = sprite_center(px, py, NEXT_CELL);
                    sp.transform = Mat4::from_translation(Vec3::new(cx, cy, 0.0));
                } else {
                    sp.color = [0.0; 4];
                    sp.transform = Mat4::from_translation(Vec3::new(-300.0, -300.0, 0.0));
                }
            }
        }
    }

    // ── Hold piece ───────────────────────────────────────
    let hold_blocks = g.hold.map(|h| PIECES[h][0]);
    let hold_color = g.hold.map(|h| {
        if g.hold_used {
            [0.3, 0.3, 0.35, 1.0]
        } else {
            COLORS[h + 1]
        }
    });
    for i in 0..16 {
        let ei = HOLD_OFF + i;
        let e = entities[ei];
        let col = i % 4;
        let row = i / 4;
        let is_block = hold_blocks.is_some_and(|b| {
            b.iter()
                .any(|&(bx, by)| bx as usize == col && by as usize == row)
        });
        if let Some(sp) = world.get_by_index_mut::<Sprite>(e.index()) {
            if is_block {
                let px = HOLD_X + col as f32 * HOLD_CELL;
                let py = HOLD_Y + row as f32 * HOLD_CELL;
                sp.color = hold_color.unwrap();
                sp.size = Vec2::new(HOLD_BLK, HOLD_BLK);
                let (cx, cy) = sprite_center(px, py, HOLD_CELL);
                sp.transform = Mat4::from_translation(Vec3::new(cx, cy, 0.0));
            } else {
                sp.color = [0.0; 4];
                sp.transform = Mat4::from_translation(Vec3::new(-300.0, -300.0, 0.0));
            }
        }
    }

    // ── Score / Level / Lines (pixel font) ───────────────
    render_number(
        world,
        entities,
        SCORE_OFF,
        g.score.min(9999999),
        SCORE_DIGITS,
        SCORE_X,
        SCORE_Y,
        SCORE_CLR,
    );
    render_number(
        world,
        entities,
        LEVEL_OFF,
        g.level.min(99),
        LEVEL_DIGITS,
        LEVEL_X,
        LEVEL_Y,
        LEVEL_CLR,
    );
    render_number(
        world,
        entities,
        LINES_OFF,
        g.lines.min(999),
        LINES_DIGITS,
        LINES_X,
        LINES_Y,
        LINES_CLR,
    );

    // ── Pause overlay ────────────────────────────────────
    if g.paused {
        for row in 0..VISIBLE_ROWS {
            for col in 0..COLS {
                let idx = GRID_OFF + row * COLS + col;
                let e = entities[idx];
                let cl = GX + col as f32 * CELL;
                let ct = GY_TOP + row as f32 * CELL;
                if let Some(sp) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sp.color = [0.02, 0.02, 0.05, 0.75];
                    sp.size = Vec2::new(CELL, CELL);
                    let (cx, cy) = sprite_center(cl, ct, CELL);
                    sp.transform = Mat4::from_translation(Vec3::new(cx, cy, 0.0));
                }
            }
        }
    }
}

// ── Pixel Font Helpers ───────────────────────────────────
fn set_bg(
    world: &mut World,
    entities: &[Entity],
    idx: usize,
    color: [f32; 4],
    size: Vec2,
    cx: f32,
    cy: f32,
) {
    let e = entities[idx];
    if let Some(sp) = world.get_by_index_mut::<Sprite>(e.index()) {
        sp.color = color;
        sp.size = size;
        sp.transform = Mat4::from_translation(Vec3::new(cx, cy, 0.0));
    }
}

fn hide(world: &mut World, entities: &[Entity], idx: usize) {
    let e = entities[idx];
    if let Some(sp) = world.get_by_index_mut::<Sprite>(e.index()) {
        sp.color = [0.0; 4];
        sp.transform = Mat4::from_translation(Vec3::new(-300.0, -300.0, 0.0));
    }
}

fn render_digit(
    world: &mut World,
    entities: &[Entity],
    offset: usize,
    digit: usize,
    x: f32,
    y: f32,
    color: [f32; 4],
) {
    let pat = &DIGITS[digit];
    for i in 0..15 {
        let e = entities[offset + i];
        if pat[i] != 0 {
            let col = i % 3;
            let row = i / 3;
            let px = x + col as f32 * (PX_SZ + PX_GAP) + PX_SZ * 0.5;
            let py = y + row as f32 * (PX_SZ + PX_GAP) + PX_SZ * 0.5;
            if let Some(sp) = world.get_by_index_mut::<Sprite>(e.index()) {
                sp.color = color;
                sp.size = Vec2::new(PX_SZ, PX_SZ);
                sp.transform = Mat4::from_translation(Vec3::new(px, py, 0.0));
            }
        } else {
            hide(world, entities, offset + i);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_number(
    world: &mut World,
    entities: &[Entity],
    offset: usize,
    value: u32,
    max_digits: usize,
    x: f32,
    y: f32,
    color: [f32; 4],
) {
    let mut digits = Vec::new();
    let mut v = value;
    if v == 0 {
        digits.push(0);
    } else {
        while v > 0 {
            digits.push((v % 10) as usize);
            v /= 10;
        }
    }
    digits.reverse();
    let n = digits.len().min(max_digits);
    let blanks = max_digits - n;
    for i in 0..blanks {
        for j in 0..15 {
            hide(world, entities, offset + i * 15 + j);
        }
    }
    for (i, &d) in digits.iter().enumerate().take(n) {
        let dx = x + (blanks + i) as f32 * (DIGIT_W + DIGIT_SP);
        render_digit(world, entities, offset + (blanks + i) * 15, d, dx, y, color);
    }
    for i in (blanks + n)..max_digits {
        for j in 0..15 {
            hide(world, entities, offset + i * 15 + j);
        }
    }
}

// ── Game Logic ───────────────────────────────────────────
fn piece_at(_g: &Game, gx: i32, gy: i32, piece: usize, rot: usize, px: i32, py: i32) -> bool {
    PIECES[piece][rot]
        .iter()
        .any(|&(bx, by)| px + bx == gx && py + by == gy)
}
fn ghost_y(g: &Game) -> i32 {
    let mut gy = g.py;
    while fits(g, g.piece, g.rot, g.px, gy + 1) {
        gy += 1;
    }
    gy
}
fn try_move(g: &mut Game, dx: i32, dy: i32) -> bool {
    if fits(g, g.piece, g.rot, g.px + dx, g.py + dy) {
        g.px += dx;
        g.py += dy;
        if g.lock_active && g.lock_resets < MAX_LOCK_RESETS {
            g.lock_acc = 0.0;
            g.lock_resets += 1;
        }
        true
    } else {
        false
    }
}
fn rotate(g: &mut Game, dir: usize) {
    let nr = (g.rot + dir) % 4;
    let kicks = if g.piece == 0 {
        &SRS_KICKS_I[g.rot]
    } else {
        &SRS_KICKS_NORMAL[g.rot]
    };
    let kick_idx = match dir {
        1 => 0,
        3 => 1,
        _ => 0,
    };
    for &(dx, dy) in &kicks[kick_idx] {
        if fits(g, g.piece, nr, g.px + dx, g.py + dy) {
            g.rot = nr;
            g.px += dx;
            g.py += dy;
            if g.lock_active && g.lock_resets < MAX_LOCK_RESETS {
                g.lock_acc = 0.0;
                g.lock_resets += 1;
            }
            return;
        }
    }
}
fn lock_piece(g: &mut Game) {
    let blocks = PIECES[g.piece][g.rot];
    let cid = (g.piece + 1) as u8;
    for &(bx, by) in &blocks {
        let gx = g.px + bx;
        let gy = g.py + by;
        if gx >= 0 && gx < COLS as i32 && gy >= 0 && gy < ROWS as i32 {
            g.grid[gy as usize][gx as usize] = cid;
        }
    }
    let mut cleared = Vec::new();
    for row in 0..ROWS {
        if g.grid[row].iter().all(|&c| c != 0) {
            cleared.push(row);
        }
    }
    if !cleared.is_empty() {
        g.clear_rows = cleared;
        g.clear_flash = 1.0;
        g.combo += 1;
    } else {
        g.combo = -1;
    }
    g.hold_used = false;
    g.lock_active = false;
    g.lock_acc = 0.0;
    g.lock_resets = 0;
    spawn_next(g);
}
fn actually_clear(g: &mut Game) {
    let n = g.clear_rows.len() as u32;
    let mut write = ROWS;
    for read in (0..ROWS).rev() {
        if g.clear_rows.contains(&read) {
            continue;
        }
        write -= 1;
        if write != read {
            g.grid[write] = g.grid[read];
        }
    }
    for row in g.grid.iter_mut().take(g.clear_rows.len()) {
        *row = [0; COLS];
    }
    g.clear_rows.clear();
    let base = match n {
        1 => 100,
        2 => 300,
        3 => 500,
        4 => 800,
        _ => 0,
    };
    let combo_bonus = if g.combo > 0 { 50 * g.combo as u32 } else { 0 };
    g.score += (base + combo_bonus) * g.level;
    g.lines += n;
    g.level = g.lines / 10 + 1;
    g.fall_speed = (0.8 - (g.level as f32 - 1.0) * 0.07).max(0.05);
}
fn spawn_next(g: &mut Game) {
    g.piece = g.next_queue.pop_front().unwrap();
    g.next_queue.push_back(g.bag.next());
    g.px = 3;
    g.py = 0;
    g.rot = 0;
    g.fall_acc = 0.0;
    g.lock_active = false;
    g.lock_acc = 0.0;
    g.lock_resets = 0;
    if !fits(g, g.piece, 0, g.px, g.py) {
        g.over = true;
    }
}
fn fits(g: &Game, piece: usize, rot: usize, px: i32, py: i32) -> bool {
    PIECES[piece][rot].iter().all(|&(bx, by)| {
        let gx = px + bx;
        let gy = py + by;
        gx >= 0
            && gx < COLS as i32
            && gy < ROWS as i32
            && (gy < 0 || g.grid[gy as usize][gx as usize] == 0)
    })
}
fn reset(g: &mut Game) {
    g.grid = [[0; COLS]; ROWS];
    g.bag = Bag::new();
    g.next_queue.clear();
    for _ in 0..4 {
        g.next_queue.push_back(g.bag.next());
    }
    g.piece = g.next_queue.pop_front().unwrap();
    g.next_queue.push_back(g.bag.next());
    g.px = 3;
    g.py = 0;
    g.rot = 0;
    g.hold = None;
    g.hold_used = false;
    g.score = 0;
    g.level = 1;
    g.lines = 0;
    g.combo = -1;
    g.over = false;
    g.paused = false;
    g.fall_acc = 0.0;
    g.fall_speed = 0.8;
    g.lock_acc = 0.0;
    g.lock_resets = 0;
    g.lock_active = false;
    g.das_dir = 0;
    g.das_acc = 0.0;
    g.das_charged = false;
    g.soft_drop = false;
    g.clear_flash = 0.0;
    g.clear_rows.clear();
}
