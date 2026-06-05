//! Tetris — playable game built on the ECS auto-render integration.
//!
//! Controls:
//!   Left/Right  — move piece
//!   Up          — rotate
//!   Down        — soft drop
//!   Space       — hard drop
//!   P           — pause
//!   Escape      — restart (when game over)

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
use std::path::PathBuf;
use std::sync::Arc;

const COLS: usize = 10;
const ROWS: usize = 20;
const CELL: f32 = 28.0;
const BLOCK: f32 = 26.0;
const GRID_X: f32 = 180.0;
const GRID_Y: f32 = 560.0;
const PREVIEW_X: f32 = 510.0;
const PREVIEW_Y: f32 = 400.0;
const SCORE_X: f32 = 510.0;
const SCORE_Y: f32 = 240.0;
const LEVEL_X: f32 = 510.0;
const LEVEL_Y: f32 = 330.0;
const WINDOW_W: u32 = 660;
const WINDOW_H: u32 = 620;
const NUM_ENTITIES: usize = COLS * ROWS + 16 + 128;

const TETROMINOES: [[[(i32, i32); 4]; 4]; 7] = [
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

const COLORS: [[f32; 4]; 8] = [
    [0.12, 0.12, 0.16, 1.0],
    [0.0, 0.9, 0.9, 1.0],
    [0.9, 0.9, 0.0, 1.0],
    [0.6, 0.0, 0.8, 1.0],
    [0.0, 0.9, 0.0, 1.0],
    [0.9, 0.0, 0.0, 1.0],
    [0.0, 0.0, 0.9, 1.0],
    [0.9, 0.5, 0.0, 1.0],
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

struct Game {
    grid: [[u8; COLS]; ROWS],
    piece: usize,
    px: i32,
    py: i32,
    rot: usize,
    next: usize,
    score: u32,
    level: u32,
    lines: u32,
    over: bool,
    paused: bool,
    fall_acc: f32,
    fall_speed: f32,
    repeat_key: KeyCode,
    repeat_acc: f32,
    repeat_delay: f32,
    repeat_rate: f32,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = Arc::new(
        create_window(
            &WindowConfig {
                title: "Tetris — RustEngine".to_string(),
                width: WINDOW_W,
                height: WINDOW_H,
                vsync: true,
            },
            &event_loop,
        )
        .unwrap(),
    );

    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);

    let tex = Texture {
        id: "white".into(),
        width: 1,
        height: 1,
        data: vec![255, 255, 255, 255],
        channels: 4,
        asset_path: PathBuf::new(),
    };
    let tex_handle = Handle::new(tex);

    let world = builder.world_mut();

    let cam = world.spawn();
    let mut camera = Camera::orthographic(0.0, WINDOW_W as f32, WINDOW_H as f32, 0.0);
    camera.clear_color = Some(Color::new(0.06, 0.06, 0.08, 1.0));
    world.add_component(cam, camera);

    let mut entities = Vec::with_capacity(NUM_ENTITIES);
    for _ in 0..NUM_ENTITIES {
        let e = world.spawn();
        world.add_component(
            e,
            Sprite {
                texture: tex_handle.clone(),
                color: [0.0, 0.0, 0.0, 0.0],
                size: Vec2::new(BLOCK, BLOCK),
                transform: Mat4::from_translation(Vec3::new(-200.0, -200.0, 0.0)),
                flip_x: false,
                flip_y: false,
                uv_region: [0.0, 0.0, 1.0, 1.0],
            },
        );
        entities.push(e);
    }

    let mut game = Game {
        grid: [[0; COLS]; ROWS],
        piece: rand_piece(),
        px: 3,
        py: 0,
        rot: 0,
        next: rand_piece(),
        score: 0,
        level: 1,
        lines: 0,
        over: false,
        paused: false,
        fall_acc: 0.0,
        fall_speed: 0.8,
        repeat_key: KeyCode::Escape,
        repeat_acc: 0.0,
        repeat_delay: 0.17,
        repeat_rate: 0.05,
    };

    let mut plugin = RenderPlugin2D::new(window.clone());
    plugin.build(builder.world_mut());
    {
        let bridge = builder
            .world_mut()
            .get_resource_mut::<TextureBridge>()
            .unwrap();
        bridge.request(&tex_handle, "");
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
                    event: winit::event::WindowEvent::Resized(size),
                    ..
                } => {
                    if let Some(r) = app.renderer_mut() {
                        r.resize(size.width, size.height);
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
                    .map(|t| t.delta_seconds())
                    .unwrap_or(0.016)
                    .min(0.05);

                let input_ref: *const InputManager =
                    app.world.get_resource::<InputManager>().unwrap() as *const _;
                let input = unsafe { &*input_ref };

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
    if input.key_just_pressed(KeyCode::Escape) {
        if g.over {
            reset_game(g);
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

    if input.key_just_pressed(KeyCode::ArrowUp) || input.key_just_pressed(KeyCode::KeyW) {
        try_rotate(g);
    }
    if input.key_just_pressed(KeyCode::Space) {
        while fits(g, g.piece, g.rot, g.px, g.py + 1) {
            g.py += 1;
            g.score += 2;
        }
        lock(g);
        return;
    }

    handle_repeat(input, g, dt, KeyCode::ArrowLeft, KeyCode::KeyA, -1);
    handle_repeat(input, g, dt, KeyCode::ArrowRight, KeyCode::KeyD, 1);

    if input.key_just_pressed(KeyCode::ArrowDown) || input.key_just_pressed(KeyCode::KeyS) {
        if try_move(g, 0, 1) {
            g.score += 1;
        }
        g.repeat_key = KeyCode::ArrowDown;
        g.repeat_acc = 0.0;
    } else if input.key_down(KeyCode::ArrowDown) || input.key_down(KeyCode::KeyS) {
        g.repeat_acc += dt;
        if g.repeat_acc >= g.repeat_rate {
            g.repeat_acc -= g.repeat_rate;
            if try_move(g, 0, 1) {
                g.score += 1;
            }
        }
    } else if g.repeat_key == KeyCode::ArrowDown {
        g.repeat_key = KeyCode::Escape;
    }
}

fn handle_repeat(
    input: &InputManager,
    g: &mut Game,
    dt: f32,
    key1: KeyCode,
    key2: KeyCode,
    dx: i32,
) {
    let just = input.key_just_pressed(key1) || input.key_just_pressed(key2);
    let held = input.key_down(key1) || input.key_down(key2);

    if just {
        try_move(g, dx, 0);
        g.repeat_key = key1;
        g.repeat_acc = 0.0;
    } else if held && g.repeat_key == key1 {
        g.repeat_acc += dt;
        if g.repeat_acc >= g.repeat_delay {
            g.repeat_acc -= g.repeat_rate;
            try_move(g, dx, 0);
        }
    } else if !held && g.repeat_key == key1 {
        g.repeat_key = KeyCode::Escape;
    }
}

fn update(g: &mut Game, dt: f32) {
    if g.over || g.paused {
        return;
    }
    g.fall_acc += dt;
    if g.fall_acc >= g.fall_speed {
        g.fall_acc -= g.fall_speed;
        if !try_move(g, 0, 1) {
            lock(g);
        }
    }
}

fn redraw(world: &mut World, g: &Game, entities: &[Entity]) {
    let ghost_y = ghost_y(g);

    for row in 0..ROWS {
        for col in 0..COLS {
            let idx = row * COLS + col;
            let e = entities[idx];
            let cell = g.grid[row][col];

            let is_ghost = !g.over
                && cell == 0
                && is_piece_cell(g, col as i32, row as i32, g.piece, g.rot, g.px, g.py);
            let is_ghost_below = !g.over
                && cell == 0
                && !is_ghost
                && is_piece_cell(g, col as i32, row as i32, g.piece, g.rot, g.px, ghost_y);

            let (color, size) = if cell != 0 {
                (COLORS[cell as usize], Vec2::new(BLOCK, BLOCK))
            } else if is_ghost {
                let c = COLORS[g.piece + 1];
                ([c[0], c[1], c[2], 0.35], Vec2::new(BLOCK, BLOCK))
            } else if is_ghost_below {
                let c = COLORS[g.piece + 1];
                (
                    [c[0] * 0.5, c[1] * 0.5, c[2] * 0.5, 0.2],
                    Vec2::new(BLOCK, BLOCK),
                )
            } else {
                (COLORS[0], Vec2::new(CELL, CELL))
            };

            let px = GRID_X + col as f32 * CELL;
            let py = GRID_Y + row as f32 * CELL;
            if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                sprite.color = color;
                sprite.size = size;
                sprite.transform = Mat4::from_translation(Vec3::new(px + 1.0, py + 1.0, 0.0));
            }
        }
    }

    // Next piece preview
    let blocks = TETROMINOES[g.next][0];
    let color = COLORS[g.next + 1];
    for i in 0..16 {
        let e = entities[COLS * ROWS + i];
        let col = i % 4;
        let row = i / 4;
        let is_block = blocks
            .iter()
            .any(|&(bx, by)| bx as usize == col && by as usize == row);
        if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
            if is_block {
                sprite.color = color;
                sprite.size = Vec2::new(BLOCK, BLOCK);
                sprite.transform = Mat4::from_translation(Vec3::new(
                    PREVIEW_X + col as f32 * CELL,
                    PREVIEW_Y + row as f32 * CELL,
                    0.0,
                ));
            } else {
                sprite.color = [0.0, 0.0, 0.0, 0.0];
                sprite.transform = Mat4::from_translation(Vec3::new(-200.0, -200.0, 0.0));
            }
        }
    }

    // Score
    draw_number(
        world,
        entities,
        COLS * ROWS + 16,
        SCORE_X,
        SCORE_Y,
        g.score,
        6,
        [0.95, 0.95, 0.95, 1.0],
    );

    // Level
    draw_number(
        world,
        entities,
        COLS * ROWS + 16 + 48,
        LEVEL_X,
        LEVEL_Y,
        g.level,
        2,
        [0.9, 0.9, 0.5, 1.0],
    );

    // Lines
    draw_number(
        world,
        entities,
        COLS * ROWS + 16 + 64,
        LEVEL_X,
        LEVEL_Y - 60.0,
        g.lines,
        4,
        [0.7, 0.7, 0.9, 1.0],
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_number(
    world: &mut World,
    entities: &[Entity],
    start_idx: usize,
    x: f32,
    y: f32,
    value: u32,
    digits: usize,
    color: [f32; 4],
) {
    let s = format!(
        "{:0width$}",
        value.min(10u32.pow(digits as u32) - 1),
        width = digits
    );
    for (di, ch) in s.chars().enumerate() {
        let d = ch.to_digit(10).unwrap() as usize;
        let map = DIGIT_MAP[d];
        for (py, row) in map.iter().enumerate() {
            for (px, &on) in row.iter().enumerate() {
                let ei = start_idx + di * 12 + py * 3 + px;
                if ei >= entities.len() {
                    continue;
                }
                let e = entities[ei];
                let on = on != 0;
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = if on { color } else { [0.0, 0.0, 0.0, 0.0] };
                    sprite.size = Vec2::new(4.0, 4.0);
                    sprite.transform = Mat4::from_translation(Vec3::new(
                        x + di as f32 * 20.0 + px as f32 * 5.0,
                        y + py as f32 * 5.0,
                        0.0,
                    ));
                }
            }
        }
    }
}

fn is_piece_cell(_g: &Game, gx: i32, gy: i32, piece: usize, rot: usize, px: i32, py: i32) -> bool {
    TETROMINOES[piece][rot]
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
        true
    } else {
        false
    }
}

fn try_rotate(g: &mut Game) {
    let nr = (g.rot + 1) % 4;
    for &(dx, dy) in &[(0, 0), (1, 0), (-1, 0), (0, -1), (2, 0), (-2, 0)] {
        if fits(g, g.piece, nr, g.px + dx, g.py + dy) {
            g.rot = nr;
            g.px += dx;
            g.py += dy;
            return;
        }
    }
}

fn lock(g: &mut Game) {
    let blocks = TETROMINOES[g.piece][g.rot];
    let cid = (g.piece + 1) as u8;
    for &(bx, by) in &blocks {
        let gx = g.px + bx;
        let gy = g.py + by;
        if gx >= 0 && gx < COLS as i32 && gy >= 0 && gy < ROWS as i32 {
            g.grid[gy as usize][gx as usize] = cid;
        }
    }
    clear_lines(g);
    g.piece = g.next;
    g.next = rand_piece();
    g.px = 3;
    g.py = 0;
    g.rot = 0;
    g.fall_acc = 0.0;
    if !fits(g, g.piece, g.rot, g.px, g.py) {
        g.over = true;
    }
}

fn clear_lines(g: &mut Game) {
    let mut cleared = 0;
    let mut write = ROWS;
    for read in (0..ROWS).rev() {
        if g.grid[read].iter().all(|&c| c != 0) {
            cleared += 1;
        } else {
            write -= 1;
            if write != read {
                g.grid[write] = g.grid[read];
            }
        }
    }
    for row in g.grid.iter_mut().take(cleared) {
        *row = [0; COLS];
    }
    g.score += match cleared {
        0 => 0,
        1 => 100,
        2 => 300,
        3 => 500,
        _ => 800,
    } * g.level;
    g.lines += cleared as u32;
    g.level = g.lines / 10 + 1;
    g.fall_speed = (0.8 - g.level as f32 * 0.06).max(0.05);
}

fn fits(g: &Game, piece: usize, rot: usize, px: i32, py: i32) -> bool {
    for &(bx, by) in &TETROMINOES[piece][rot] {
        let gx = px + bx;
        let gy = py + by;
        if gx < 0 || gx >= COLS as i32 || gy >= ROWS as i32 {
            return false;
        }
        if gy >= 0 && g.grid[gy as usize][gx as usize] != 0 {
            return false;
        }
    }
    true
}

fn reset_game(g: &mut Game) {
    g.grid = [[0; COLS]; ROWS];
    g.piece = rand_piece();
    g.px = 3;
    g.py = 0;
    g.rot = 0;
    g.next = rand_piece();
    g.score = 0;
    g.level = 1;
    g.lines = 0;
    g.over = false;
    g.paused = false;
    g.fall_acc = 0.0;
    g.fall_speed = 0.8;
}

fn rand_piece() -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (ns % 7) as usize
}
