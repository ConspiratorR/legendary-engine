//! Tetris — playable game built on the ECS auto-render integration.
//!
//! Controls:
//!   Left/Right  — move piece
//!   Up          — rotate
//!   Down        — soft drop
//!   Space       — hard drop
//!   Escape      — restart (when game over)

use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
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
const BLOCK_SIZE: f32 = 28.0;
const GRID_X: f32 = 220.0;
const GRID_Y: f32 = 560.0;
const PREVIEW_X: f32 = 530.0;
const PREVIEW_Y: f32 = 440.0;
const WINDOW_W: u32 = 680;
const WINDOW_H: u32 = 640;
const NUM_ENTITIES: usize = COLS * ROWS + 16 + 200 + 64;

const TETROMINOES: [[[(i32, i32); 4]; 4]; 7] = [
    // I
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    // O
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    // T
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // S
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // Z
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ],
    // J
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    // L
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];

const PIECE_COLORS: [[f32; 4]; 8] = [
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 1.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [0.6, 0.0, 0.8, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [1.0, 0.5, 0.0, 1.0],
];

struct GameState {
    grid: [[u8; COLS]; ROWS],
    piece_type: usize,
    piece_x: i32,
    piece_y: i32,
    piece_rot: usize,
    next_piece: usize,
    score: u32,
    level: u32,
    lines: u32,
    game_over: bool,
    gravity_counter: u32,
    gravity_interval: u32,
    input_repeat: u32,
    input_dir: i32,
}

fn random_piece() -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 7) as usize
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

    let white_tex = Texture {
        id: "white".into(),
        width: 1,
        height: 1,
        data: vec![255, 255, 255, 255],
        channels: 4,
        asset_path: PathBuf::new(),
    };
    let tex_handle = Handle::new(white_tex);

    let world = builder.world_mut();

    let cam = world.spawn();
    let mut camera = Camera::orthographic(0.0, WINDOW_W as f32, WINDOW_H as f32, 0.0);
    camera.clear_color = Some(Color::new(0.08, 0.08, 0.12, 1.0));
    world.add_component(cam, camera);

    let mut entities = Vec::with_capacity(NUM_ENTITIES);
    for _ in 0..NUM_ENTITIES {
        let e = world.spawn();
        world.add_component(
            e,
            Sprite {
                texture: tex_handle.clone(),
                color: [0.0, 0.0, 0.0, 0.0],
                size: Vec2::new(BLOCK_SIZE, BLOCK_SIZE),
                transform: Mat4::from_translation(Vec3::new(-100.0, -100.0, 0.0)),
                flip_x: false,
                flip_y: false,
                uv_region: [0.0, 0.0, 1.0, 1.0],
            },
        );
        entities.push(e);
    }

    let mut game = GameState {
        grid: [[0; COLS]; ROWS],
        piece_type: random_piece(),
        piece_x: 3,
        piece_y: 0,
        piece_rot: 0,
        next_piece: random_piece(),
        score: 0,
        level: 1,
        lines: 0,
        game_over: false,
        gravity_counter: 0,
        gravity_interval: 45,
        input_repeat: 0,
        input_dir: 0,
    };

    let mut plugin = RenderPlugin2D::new(window.clone());
    plugin.build(builder.world_mut());

    // Register texture handle with bridge
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
                    use winit::keyboard::PhysicalKey;
                    if let PhysicalKey::Code(key) = ke.physical_key {
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
                handle_input(app.world.get_resource::<InputManager>().unwrap(), &mut game);
                update(&mut game);
                redraw(&mut app.world, &game, &entities);
                app.run();
                app.render_phase();
            }
        })
        .unwrap();
}

fn handle_input(input: &InputManager, game: &mut GameState) {
    if game.game_over {
        if input.key_just_pressed(KeyCode::Escape) {
            game.grid = [[0; COLS]; ROWS];
            game.piece_type = random_piece();
            game.piece_x = 3;
            game.piece_y = 0;
            game.piece_rot = 0;
            game.next_piece = random_piece();
            game.score = 0;
            game.level = 1;
            game.lines = 0;
            game.game_over = false;
            game.gravity_counter = 0;
            game.gravity_interval = 45;
        }
        return;
    }

    if input.key_just_pressed(KeyCode::ArrowLeft) || input.key_just_pressed(KeyCode::KeyA) {
        try_move(game, -1, 0);
        game.input_repeat = 0;
        game.input_dir = -1;
    } else if input.key_down(KeyCode::ArrowLeft) || input.key_down(KeyCode::KeyA) {
        game.input_repeat += 1;
        if game.input_dir == -1 && game.input_repeat > 12 && game.input_repeat.is_multiple_of(3) {
            try_move(game, -1, 0);
        }
    }

    if input.key_just_pressed(KeyCode::ArrowRight) || input.key_just_pressed(KeyCode::KeyD) {
        try_move(game, 1, 0);
        game.input_repeat = 0;
        game.input_dir = 1;
    } else if input.key_down(KeyCode::ArrowRight) || input.key_down(KeyCode::KeyD) {
        game.input_repeat += 1;
        if game.input_dir == 1 && game.input_repeat > 12 && game.input_repeat.is_multiple_of(3) {
            try_move(game, 1, 0);
        }
    }

    if !input.key_down(KeyCode::ArrowLeft)
        && !input.key_down(KeyCode::KeyA)
        && !input.key_down(KeyCode::ArrowRight)
        && !input.key_down(KeyCode::KeyD)
    {
        game.input_repeat = 0;
        game.input_dir = 0;
    }

    if input.key_just_pressed(KeyCode::ArrowUp) || input.key_just_pressed(KeyCode::KeyW) {
        try_rotate(game);
    }

    if (input.key_down(KeyCode::ArrowDown) || input.key_down(KeyCode::KeyS)) && try_move(game, 0, 1)
    {
        game.score += 1;
    }

    if input.key_just_pressed(KeyCode::Space) {
        while try_move(game, 0, 1) {
            game.score += 2;
        }
        lock_piece(game);
    }
}

fn update(game: &mut GameState) {
    if game.game_over {
        return;
    }

    game.gravity_counter += 1;
    let interval = game.gravity_interval.max(3);
    if game.gravity_counter >= interval {
        game.gravity_counter = 0;
        if !try_move(game, 0, 1) {
            lock_piece(game);
        }
    }
}

fn redraw(world: &mut World, game: &GameState, entities: &[Entity]) {
    // Grid background + placed blocks
    for row in 0..ROWS {
        for col in 0..COLS {
            let idx = row * COLS + col;
            let e = entities[idx];
            let cell = game.grid[row][col];
            let color = PIECE_COLORS[cell as usize];
            let px = GRID_X + col as f32 * BLOCK_SIZE;
            let py = GRID_Y + row as f32 * BLOCK_SIZE;

            if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                sprite.color = color;
                sprite.transform = Mat4::from_translation(Vec3::new(px, py, 0.0));
            }
        }
    }

    // Current piece
    if !game.game_over {
        let blocks = TETROMINOES[game.piece_type][game.piece_rot];
        let color = PIECE_COLORS[game.piece_type + 1];
        for &(bx, by) in &blocks {
            let gx = game.piece_x + bx;
            let gy = game.piece_y + by;
            if gx >= 0 && gx < COLS as i32 && gy >= 0 && gy < ROWS as i32 {
                let idx = (gy as usize) * COLS + (gx as usize);
                let e = entities[idx];
                let px = GRID_X + gx as f32 * BLOCK_SIZE;
                let py = GRID_Y + gy as f32 * BLOCK_SIZE;
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = color;
                    sprite.transform = Mat4::from_translation(Vec3::new(px, py, 0.0));
                }
            }
        }
    }

    // Next piece preview (entities[240..256])
    let next_blocks = TETROMINOES[game.next_piece][0];
    let next_color = PIECE_COLORS[game.next_piece + 1];
    for i in 0..16 {
        let e = entities[COLS * ROWS + i];
        let col = i % 4;
        let row = i / 4;
        let is_block = next_blocks
            .iter()
            .any(|&(bx, by)| bx as usize == col && by as usize == row);
        if is_block {
            let px = PREVIEW_X + col as f32 * BLOCK_SIZE;
            let py = PREVIEW_Y + row as f32 * BLOCK_SIZE;
            if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                sprite.color = next_color;
                sprite.transform = Mat4::from_translation(Vec3::new(px, py, 0.0));
            }
        } else if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
            sprite.color = [0.0, 0.0, 0.0, 0.0];
            sprite.transform = Mat4::from_translation(Vec3::new(-100.0, -100.0, 0.0));
        }
    }

    // Score digits — simplified bar-style display
    let score_str = format!("{:06}", game.score);
    for (di, ch) in score_str.chars().enumerate() {
        let digit = ch.to_digit(10).unwrap() as usize;
        let digit_map = DIGIT_MAP[digit];
        for (py, row) in digit_map.iter().enumerate() {
            let idx = COLS * ROWS + 16 + di * 4 + py;
            if idx < entities.len() {
                let e = entities[idx];
                let on_count = row.iter().filter(|&&v| v != 0).count();
                let any_on = on_count > 0;
                let color = if any_on {
                    [0.9, 0.9, 0.9, 1.0]
                } else {
                    [0.0, 0.0, 0.0, 0.0]
                };
                let screen_x = 530.0 + di as f32 * 18.0;
                let screen_y = 310.0 + py as f32 * 6.0;
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = color;
                    sprite.size = Vec2::new(on_count as f32 * 5.0, 5.0);
                    sprite.transform = Mat4::from_translation(Vec3::new(screen_x, screen_y, 0.0));
                }
            }
        }
    }

    // Level indicator
    let level_str = format!("{:02}", game.level);
    for (di, ch) in level_str.chars().enumerate() {
        let digit = ch.to_digit(10).unwrap() as usize;
        let digit_map = DIGIT_MAP[digit];
        for (py, row) in digit_map.iter().enumerate() {
            let idx = COLS * ROWS + 16 + 24 + di * 4 + py;
            if idx < entities.len() {
                let e = entities[idx];
                let on_count = row.iter().filter(|&&v| v != 0).count();
                let any_on = on_count > 0;
                let color = if any_on {
                    [0.9, 0.9, 0.5, 1.0]
                } else {
                    [0.0, 0.0, 0.0, 0.0]
                };
                let screen_x = 530.0 + di as f32 * 18.0;
                let screen_y = 370.0 + py as f32 * 6.0;
                if let Some(sprite) = world.get_by_index_mut::<Sprite>(e.index()) {
                    sprite.color = color;
                    sprite.size = Vec2::new(on_count as f32 * 5.0, 5.0);
                    sprite.transform = Mat4::from_translation(Vec3::new(screen_x, screen_y, 0.0));
                }
            }
        }
    }
}

fn try_move(game: &mut GameState, dx: i32, dy: i32) -> bool {
    let nx = game.piece_x + dx;
    let ny = game.piece_y + dy;
    if fits(game, game.piece_type, game.piece_rot, nx, ny) {
        game.piece_x = nx;
        game.piece_y = ny;
        true
    } else {
        false
    }
}

fn try_rotate(game: &mut GameState) {
    let new_rot = (game.piece_rot + 1) % 4;
    if fits(game, game.piece_type, new_rot, game.piece_x, game.piece_y) {
        game.piece_rot = new_rot;
        return;
    }
    for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, -1), (2, 0), (-2, 0)] {
        if fits(
            game,
            game.piece_type,
            new_rot,
            game.piece_x + dx,
            game.piece_y + dy,
        ) {
            game.piece_x += dx;
            game.piece_y += dy;
            game.piece_rot = new_rot;
            return;
        }
    }
}

fn lock_piece(game: &mut GameState) {
    let blocks = TETROMINOES[game.piece_type][game.piece_rot];
    let color_id = (game.piece_type + 1) as u8;
    for &(bx, by) in &blocks {
        let gx = game.piece_x + bx;
        let gy = game.piece_y + by;
        if gx >= 0 && gx < COLS as i32 && gy >= 0 && gy < ROWS as i32 {
            game.grid[gy as usize][gx as usize] = color_id;
        }
    }
    clear_lines(game);
    game.piece_type = game.next_piece;
    game.next_piece = random_piece();
    game.piece_x = 3;
    game.piece_y = 0;
    game.piece_rot = 0;
    game.gravity_counter = 0;
    if !fits(
        game,
        game.piece_type,
        game.piece_rot,
        game.piece_x,
        game.piece_y,
    ) {
        game.game_over = true;
    }
}

fn clear_lines(game: &mut GameState) {
    let mut cleared = 0;
    let mut write = ROWS;
    for read in (0..ROWS).rev() {
        let full = game.grid[read].iter().all(|&c| c != 0);
        if full {
            cleared += 1;
        } else {
            write -= 1;
            if write != read {
                game.grid[write] = game.grid[read];
            }
        }
    }
    for row in game.grid.iter_mut().take(cleared) {
        *row = [0; COLS];
    }
    game.score += match cleared {
        0 => 0,
        1 => 100,
        2 => 300,
        3 => 500,
        _ => 800,
    } * game.level;
    game.lines += cleared as u32;
    game.level = game.lines / 10 + 1;
    game.gravity_interval = 45_u32.saturating_sub(game.level * 3);
}

fn fits(game: &GameState, piece: usize, rot: usize, px: i32, py: i32) -> bool {
    for &(bx, by) in &TETROMINOES[piece][rot] {
        let gx = px + bx;
        let gy = py + by;
        if gx < 0 || gx >= COLS as i32 || gy < 0 || gy >= ROWS as i32 {
            return false;
        }
        if game.grid[gy as usize][gx as usize] != 0 {
            return false;
        }
    }
    true
}

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
