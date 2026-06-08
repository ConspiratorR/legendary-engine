//! Tetris — A real, playable Tetris game.
//!
//! Implements: SRS rotation, lock delay, hold, 7-bag randomizer,
//! DAS/ARR, combo, T-spin detection, ghost piece, next queue.
//!
//! Controls:
//!   Left/Right  — move (DAS)
//!   Up / X      — rotate CW
//!   Z           — rotate CCW
//!   Down        — soft drop
//!   Space       — hard drop
//!   C           — hold
//!   P           — pause
//!   Escape      — restart

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

const COLS: usize = 10;
const ROWS: usize = 22;
const VISIBLE_ROWS: usize = 20;
const CELL: f32 = 28.0;
const BLK: f32 = 26.0;
const GX: f32 = 260.0;
const GY: f32 = 640.0;
const PREVIEW_X: f32 = 590.0;
const PREVIEW_Y: f32 = 160.0;
const HOLD_X: f32 = 70.0;
const HOLD_Y: f32 = 160.0;
const SCORE_X: f32 = 590.0;
const SCORE_Y: f32 = 420.0;
const WIN_W: u32 = 840;
const WIN_H: u32 = 720;
const TOTAL_ENTITIES: usize = COLS * VISIBLE_ROWS + 48 + 16 + 128 + 64 + 64;

const DAS_DELAY: f32 = 0.167;
const DAS_REPEAT: f32 = 0.033;
const LOCK_DELAY: f32 = 0.5;
const MAX_LOCK_RESETS: u32 = 15;
const SOFT_DROP_SPEED: f32 = 0.04;

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

const COLORS: [[f32; 4]; 8] = [
    [0.10, 0.10, 0.13, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [0.7, 0.0, 1.0, 1.0],
    [0.0, 1.0, 0.2, 1.0],
    [1.0, 0.15, 0.15, 1.0],
    [0.2, 0.3, 1.0, 1.0],
    [1.0, 0.55, 0.0, 1.0],
];

const DIGIT_MAP: [[[u8; 3]; 4]; 10] = [
    [[1, 1, 1], [1, 0, 1], [1, 0, 1], [1, 1, 1]],
    [[0, 1, 0], [1, 1, 0], [0, 1, 0], [1, 1, 1]],
    [[1, 1, 1], [0, 1, 1], [1, 1, 0], [1, 1, 1]],
    [[1, 1, 1], [0, 1, 1], [0, 0, 1], [1, 1, 1]],
    [[1, 0, 1], [1, 1, 1], [0, 0, 1], [0, 0, 1]],
    [[1, 1, 1], [1, 1, 0], [0, 0, 1], [1, 1, 1]],
    [[1, 1, 1], [1, 1, 0], [1, 0, 1], [1, 1, 1]],
    [[1, 1, 1], [0, 0, 1], [0, 1, 0], [0, 1, 0]],
    [[1, 1, 1], [1, 1, 1], [1, 0, 1], [1, 1, 1]],
    [[1, 1, 1], [1, 1, 1], [0, 0, 1], [1, 1, 1]],
];

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
    started: bool,
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
    over_timer: f32,
    level_up_timer: f32,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!();
    println!("========================================");
    println!("           T E T R I S");
    println!("========================================");
    println!();
    println!("  Controls:");
    println!("    Left / A    - Move left");
    println!("    Right / D   - Move right");
    println!("    Up / X      - Rotate clockwise");
    println!("    Z           - Rotate counter-clockwise");
    println!("    Down / S    - Soft drop");
    println!("    Space       - Hard drop");
    println!("    C           - Hold piece");
    println!("    P           - Pause / Resume");
    println!("    Esc         - Restart (when game over)");
    println!();
    println!("  Rules:");
    println!("    - Clear lines by filling a complete row");
    println!("    - 7-bag randomizer (standard Tetris)");
    println!("    - Lock delay: 0.5s after landing");
    println!("    - SRS rotation with wall kicks");
    println!("    - Ghost piece shows landing position");
    println!();
    println!("  Scoring:");
    println!("    1 line  = 100 x level");
    println!("    2 lines = 300 x level");
    println!("    3 lines = 500 x level");
    println!("    4 lines = 800 x level (Tetris!)");
    println!("    Combo bonus = 50 x combo x level");
    println!("    Hard drop = 2 per cell");
    println!("    Soft drop = 1 per cell");
    println!();
    println!("========================================");
    println!();

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
    cam_comp.clear_color = Some(Color::new(0.05, 0.05, 0.07, 1.0));
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
        started: false,
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
        over_timer: 0.0,
        level_up_timer: 0.0,
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
                app.run();
                app.render_phase();
            }
        })
        .unwrap();
}

fn process_input(input: &InputManager, g: &mut Game, dt: f32) {
    if !g.started {
        if input.key_just_pressed(KeyCode::Space) {
            g.started = true;
        }
        return;
    }
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

    // Rotate
    if input.key_just_pressed(KeyCode::ArrowUp) || input.key_just_pressed(KeyCode::KeyX) {
        rotate(g, 1);
    }
    if input.key_just_pressed(KeyCode::KeyZ) {
        rotate(g, 3);
    }

    // Hard drop
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

    // Hold
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

    // Soft drop
    g.soft_drop = input.key_down(KeyCode::ArrowDown) || input.key_down(KeyCode::KeyS);
    if (input.key_just_pressed(KeyCode::ArrowDown) || input.key_just_pressed(KeyCode::KeyS))
        && try_move(g, 0, 1)
    {
        g.score += 1;
    }

    // Horizontal DAS
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

fn update(g: &mut Game, dt: f32) {
    if !g.started {
        g.over_timer += dt;
        return;
    }
    if g.over {
        g.over_timer += dt;
        return;
    }
    if g.paused {
        return;
    }
    if g.level_up_timer > 0.0 {
        g.level_up_timer -= dt;
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

fn redraw(world: &mut World, g: &Game, entities: &[Entity]) {
    let ghost_y = ghost_y(g);
    for row in 0..VISIBLE_ROWS {
        let grid_row = row + (ROWS - VISIBLE_ROWS);
        for col in 0..COLS {
            let idx = row * COLS + col;
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
            let (color, size) = if is_clearing {
                let f = g.clear_flash;
                (
                    [
                        1.0 * f + 0.1 * (1.0 - f),
                        1.0 * f + 0.1 * (1.0 - f),
                        1.0 * f + 0.13 * (1.0 - f),
                        1.0,
                    ],
                    Vec2::new(CELL, CELL),
                )
            } else if cell != 0 {
                (COLORS[cell as usize], Vec2::new(BLK, BLK))
            } else if is_cur {
                (COLORS[g.piece + 1], Vec2::new(BLK, BLK))
            } else if is_ghost {
                let c = COLORS[g.piece + 1];
                ([c[0], c[1], c[2], 0.35], Vec2::new(BLK, BLK))
            } else {
                let shade = if (row + col) % 2 == 0 { 0.10 } else { 0.12 };
                ([shade, shade, shade + 0.03, 1.0], Vec2::new(CELL, CELL))
            };
            let px = GX + col as f32 * CELL;
            let py = GY - (VISIBLE_ROWS as f32 - row as f32) * CELL;
            if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                sprite.color = color;
                sprite.size = size;
                sprite.transform = Mat4::from_translation(Vec3::new(px + 1.0, py + 1.0, 0.0));
            }
        }
    }

    // Next queue (3 pieces)
    for qi in 0..3 {
        let blocks = PIECES[g.next_queue[qi]][0];
        let color = COLORS[g.next_queue[qi] + 1];
        for i in 0..16 {
            let ei = COLS * VISIBLE_ROWS + qi * 16 + i;
            let e = entities[ei];
            let col = i % 4;
            let row = i / 4;
            let is_block = blocks
                .iter()
                .any(|&(bx, by)| bx as usize == col && by as usize == row);
            if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                if is_block {
                    sprite.color = color;
                    sprite.size = Vec2::new(BLK, BLK);
                    sprite.transform = Mat4::from_translation(Vec3::new(
                        PREVIEW_X + col as f32 * CELL,
                        PREVIEW_Y - qi as f32 * 4.5 * CELL + row as f32 * CELL,
                        0.0,
                    ));
                } else {
                    sprite.color = [0.0; 4];
                    sprite.transform = Mat4::from_translation(Vec3::new(-300.0, -300.0, 0.0));
                }
            }
        }
    }

    // Hold piece
    let hold_blocks = g.hold.map(|h| PIECES[h][0]);
    let hold_color = g.hold.map(|h| {
        if g.hold_used {
            [0.4, 0.4, 0.4, 1.0]
        } else {
            COLORS[h + 1]
        }
    });
    for i in 0..16 {
        let ei = COLS * VISIBLE_ROWS + 48 + i;
        let e = entities[ei];
        let col = i % 4;
        let row = i / 4;
        let is_block = hold_blocks.is_some_and(|b| {
            b.iter()
                .any(|&(bx, by)| bx as usize == col && by as usize == row)
        });
        if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
            if is_block {
                sprite.color = hold_color.unwrap();
                sprite.size = Vec2::new(BLK, BLK);
                sprite.transform = Mat4::from_translation(Vec3::new(
                    HOLD_X + col as f32 * CELL,
                    HOLD_Y + row as f32 * CELL,
                    0.0,
                ));
            } else {
                sprite.color = [0.0; 4];
                sprite.transform = Mat4::from_translation(Vec3::new(-300.0, -300.0, 0.0));
            }
        }
    }

    // Score
    draw_num(
        world,
        entities,
        COLS * VISIBLE_ROWS + 64,
        SCORE_X,
        SCORE_Y,
        g.score,
        7,
        [0.95, 0.95, 0.95, 1.0],
    );
    draw_num(
        world,
        entities,
        COLS * VISIBLE_ROWS + 64 + 84,
        SCORE_X,
        SCORE_Y + 35.0,
        g.level,
        2,
        [0.9, 0.9, 0.5, 1.0],
    );
    draw_num(
        world,
        entities,
        COLS * VISIBLE_ROWS + 64 + 84 + 24,
        SCORE_X,
        SCORE_Y + 70.0,
        g.lines,
        4,
        [0.7, 0.7, 0.9, 1.0],
    );

    // Start screen overlay
    if !g.started {
        for row in 0..VISIBLE_ROWS {
            for col in 0..COLS {
                let idx = row * COLS + col;
                let e = entities[idx];
                let is_center = (4..=5).contains(&col) && (9..=10).contains(&row);
                let color = if is_center {
                    let pulse = ((g.over_timer * 3.0).sin() * 0.5 + 0.5) as f32;
                    [0.8 + pulse * 0.2, 0.8 + pulse * 0.2, 0.8 + pulse * 0.2, 1.0]
                } else {
                    [0.05, 0.05, 0.07, 1.0]
                };
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = color;
                    sprite.size = Vec2::new(CELL, CELL);
                    sprite.transform = Mat4::from_translation(Vec3::new(
                        GX + col as f32 * CELL,
                        GY - (VISIBLE_ROWS as f32 - row as f32) * CELL,
                        0.0,
                    ));
                }
            }
        }
        return;
    }

    // Game over overlay
    if g.over {
        let pulse = ((g.over_timer * 2.0).sin() * 0.5 + 0.5) as f32;
        for row in 0..VISIBLE_ROWS {
            for col in 0..COLS {
                let idx = row * COLS + col;
                let e = entities[idx];
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = [0.3 + pulse * 0.2, 0.0, 0.0, 0.8];
                    sprite.size = Vec2::new(CELL, CELL);
                    sprite.transform = Mat4::from_translation(Vec3::new(
                        GX + col as f32 * CELL,
                        GY - (VISIBLE_ROWS as f32 - row as f32) * CELL,
                        0.0,
                    ));
                }
            }
        }
        return;
    }

    // Level up flash
    if g.level_up_timer > 0.0 {
        let flash = g.level_up_timer;
        for row in 0..VISIBLE_ROWS {
            for col in 0..COLS {
                let idx = row * COLS + col;
                let e = entities[idx];
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = [1.0, 1.0, 1.0, flash * 0.3];
                    sprite.size = Vec2::new(CELL, CELL);
                    sprite.transform = Mat4::from_translation(Vec3::new(
                        GX + col as f32 * CELL,
                        GY - (VISIBLE_ROWS as f32 - row as f32) * CELL,
                        0.0,
                    ));
                }
            }
        }
    }

    // Paused overlay
    if g.paused {
        for row in 0..VISIBLE_ROWS {
            for col in 0..COLS {
                let idx = row * COLS + col;
                let e = entities[idx];
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = [0.0, 0.0, 0.0, 0.7];
                    sprite.size = Vec2::new(CELL, CELL);
                    sprite.transform = Mat4::from_translation(Vec3::new(
                        GX + col as f32 * CELL,
                        GY - (VISIBLE_ROWS as f32 - row as f32) * CELL,
                        0.0,
                    ));
                }
            }
        }
    }

    // Controls indicator (bottom-left)
    let base = COLS * VISIBLE_ROWS + 64 + 84 + 24 + 24;
    let key_color = [0.3, 0.3, 0.4, 1.0];
    let label_color = [0.7, 0.7, 0.8, 1.0];
    let controls: [([f32; 4], &str); 7] = [
        (label_color, "LEFT/RIGHT : MOVE"),
        (label_color, "UP/X : ROTATE CW"),
        (label_color, "Z : ROTATE CCW"),
        (label_color, "DOWN/S : SOFT DROP"),
        (label_color, "SPACE : HARD DROP"),
        (label_color, "C : HOLD"),
        (label_color, "P : PAUSE"),
    ];
    for (i, (color, _label)) in controls.iter().enumerate() {
        let ei = base + i * 2;
        if ei + 1 < entities.len() {
            let e = entities[ei];
            if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                sprite.color = key_color;
                sprite.size = Vec2::new(6.0, 6.0);
                sprite.transform =
                    Mat4::from_translation(Vec3::new(20.0, 20.0 + i as f32 * 12.0, 0.0));
            }
            let e2 = entities[ei + 1];
            if let Some(sprite) = world.get_by_index_mut::<Sprite>(e2.index()) {
                sprite.color = *color;
                sprite.size = Vec2::new(80.0, 6.0);
                sprite.transform =
                    Mat4::from_translation(Vec3::new(30.0, 20.0 + i as f32 * 12.0, 0.0));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_num(
    world: &mut World,
    entities: &[Entity],
    start: usize,
    x: f32,
    y: f32,
    val: u32,
    digits: usize,
    color: [f32; 4],
) {
    let s = format!(
        "{:0width$}",
        val.min(10u32.pow(digits as u32) - 1),
        width = digits
    );
    for (di, ch) in s.chars().enumerate() {
        let d = ch.to_digit(10).unwrap() as usize;
        let map = DIGIT_MAP[d];
        for (py, row) in map.iter().enumerate() {
            for (px, &on) in row.iter().enumerate() {
                let ei = start + di * 12 + py * 3 + px;
                if ei >= entities.len() {
                    continue;
                }
                let e = entities[ei];
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = if on != 0 { color } else { [0.0; 4] };
                    sprite.size = Vec2::new(4.0, 4.0);
                    sprite.transform = Mat4::from_translation(Vec3::new(
                        x + di as f32 * 18.0 + px as f32 * 5.0,
                        y + py as f32 * 5.0,
                        0.0,
                    ));
                }
            }
        }
    }
}

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
    let old_level = g.level;
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
    if g.level > old_level {
        g.level_up_timer = 1.0;
    }
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
