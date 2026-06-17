use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::{Mat4, Vec2, Vec3};
use engine_render::camera::{Camera, Color};
use engine_render::plugin::RenderPlugin2D;
use engine_render::sprite::Sprite;
use engine_window::{window::WindowConfig, window::create_window};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

const COLS: usize = 10;
const ROWS: usize = 20;
const CELL: f32 = 32.0;
const WIN_W: u32 = 480;
const WIN_H: u32 = 640;
const GRID_X: f32 = 40.0;
const GRID_Y: f32 = 40.0;

const COLORS: [[f32; 4]; 8] = [
    [0.08, 0.08, 0.14, 1.0], // background
    [0.0, 0.9, 0.9, 1.0],    // I - cyan
    [0.9, 0.9, 0.0, 1.0],    // O - yellow
    [0.6, 0.0, 0.9, 1.0],    // T - purple
    [0.0, 0.9, 0.3, 1.0],    // S - green
    [0.9, 0.1, 0.1, 1.0],    // Z - red
    [0.1, 0.3, 0.9, 1.0],    // J - blue
    [0.9, 0.5, 0.0, 1.0],    // L - orange
];

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

struct Bag {
    queue: VecDeque<usize>,
}
impl Bag {
    fn new() -> Self {
        let mut b = Bag {
            queue: VecDeque::new(),
        };
        b.fill();
        b
    }
    fn next(&mut self) -> usize {
        if self.queue.is_empty() {
            self.fill();
        }
        self.queue.pop_front().unwrap()
    }
    fn fill(&mut self) {
        let mut order: Vec<usize> = (0..7).collect();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as usize;
        for i in (1..7).rev() {
            let j = seed.wrapping_mul(31).wrapping_add(i) % (i + 1);
            order.swap(i, j);
        }
        self.queue.extend(order);
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
    next: VecDeque<usize>,
    bag: Bag,
    score: u32,
    level: u32,
    lines: u32,
    over: bool,
    paused: bool,
    fall_acc: f32,
    fall_speed: f32,
    lock_acc: f32,
    lock_active: bool,
    soft_drop: bool,
    das_dir: i32,
    das_acc: f32,
    das_charged: bool,
}

impl Game {
    fn new() -> Self {
        let mut bag = Bag::new();
        let mut next = VecDeque::new();
        for _ in 0..4 {
            next.push_back(bag.next());
        }
        let piece = next.pop_front().unwrap();
        next.push_back(bag.next());
        Game {
            grid: [[0; COLS]; ROWS],
            piece,
            px: 3,
            py: 0,
            rot: 0,
            hold: None,
            hold_used: false,
            next,
            bag,
            score: 0,
            level: 1,
            lines: 0,
            over: false,
            paused: false,
            fall_acc: 0.0,
            fall_speed: 0.8,
            lock_acc: 0.0,
            lock_active: false,
            soft_drop: false,
            das_dir: 0,
            das_acc: 0.0,
            das_charged: false,
        }
    }

    fn fits(&self, piece: usize, rot: usize, px: i32, py: i32) -> bool {
        PIECES[piece][rot].iter().all(|&(bx, by)| {
            let (gx, gy) = (px + bx, py + by);
            gx >= 0
                && gx < COLS as i32
                && gy < ROWS as i32
                && (gy < 0 || self.grid[gy as usize][gx as usize] == 0)
        })
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        if self.fits(self.piece, self.rot, self.px + dx, self.py + dy) {
            self.px += dx;
            self.py += dy;
            if self.lock_active {
                self.lock_acc = 0.0;
            }
            true
        } else {
            false
        }
    }

    fn rotate(&mut self, dir: usize) {
        let nr = (self.rot + dir) % 4;
        for &(dx, dy) in &[(0, 0), (-1, 0), (1, 0), (0, -1), (-1, -1), (1, -1)] {
            if self.fits(self.piece, nr, self.px + dx, self.py + dy) {
                self.rot = nr;
                self.px += dx;
                self.py += dy;
                if self.lock_active {
                    self.lock_acc = 0.0;
                }
                return;
            }
        }
    }

    fn ghost_y(&self) -> i32 {
        let mut gy = self.py;
        while self.fits(self.piece, self.rot, self.px, gy + 1) {
            gy += 1;
        }
        gy
    }

    fn lock(&mut self) {
        for &(bx, by) in &PIECES[self.piece][self.rot] {
            let (gx, gy) = (self.px + bx, self.py + by);
            if gx >= 0 && gx < COLS as i32 && gy >= 0 && gy < ROWS as i32 {
                self.grid[gy as usize][gx as usize] = (self.piece + 1) as u8;
            }
        }
        let mut cleared = 0;
        let mut write = ROWS;
        for read in (0..ROWS).rev() {
            if self.grid[read].iter().all(|&c| c != 0) {
                cleared += 1;
                continue;
            }
            write -= 1;
            if write != read {
                self.grid[write] = self.grid[read];
            }
        }
        for row in self.grid.iter_mut().take(cleared) {
            *row = [0; COLS];
        }
        if cleared > 0 {
            self.lines += cleared as u32;
            self.score += match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            } * self.level;
            self.level = self.lines / 10 + 1;
            self.fall_speed = (0.8 - (self.level as f32 - 1.0) * 0.07).max(0.05);
        }
        self.hold_used = false;
        self.lock_active = false;
        self.lock_acc = 0.0;
        self.spawn();
    }

    fn spawn(&mut self) {
        self.piece = self.next.pop_front().unwrap();
        self.next.push_back(self.bag.next());
        self.px = 3;
        self.py = 0;
        self.rot = 0;
        self.fall_acc = 0.0;
        self.lock_active = false;
        self.lock_acc = 0.0;
        if !self.fits(self.piece, 0, self.px, self.py) {
            self.over = true;
        }
    }

    fn hard_drop(&mut self) {
        while self.try_move(0, 1) {
            self.score += 2;
        }
        self.lock();
    }

    fn hold_piece(&mut self) {
        if self.hold_used {
            return;
        }
        let cur = self.piece;
        if let Some(h) = self.hold {
            self.piece = h;
        } else {
            self.spawn();
            return;
        }
        self.hold = Some(cur);
        self.hold_used = true;
        self.px = 3;
        self.py = 0;
        self.rot = 0;
        self.lock_active = false;
        self.lock_acc = 0.0;
    }

    fn reset(&mut self) {
        *self = Game::new();
    }
}

fn main() {
    println!(
        "\n  TETRIS\n  Arrows/AD: Move  Up/X: Rotate  Z: Rotate CCW\n  Down/S: Soft drop  Space: Hard drop  C: Hold  P: Pause  Esc: Restart\n"
    );

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = Arc::new(
        create_window(
            &WindowConfig {
                title: "Tetris".into(),
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

    let tex = Handle::new(Texture {
        id: "w".into(),
        width: 1,
        height: 1,
        data: vec![255, 255, 255, 255],
        channels: 4,
        asset_path: PathBuf::new(),
    });

    let world = builder.world_mut();
    let cam = world.spawn();
    let mut cam_comp = Camera::orthographic(0.0, WIN_W as f32, WIN_H as f32, 0.0);
    cam_comp.clear_color = Some(Color::new(0.04, 0.04, 0.08, 1.0));
    world.add_component(cam, cam_comp);

    let total = COLS * ROWS + 16 + 48 + 3;
    let mut entities = Vec::with_capacity(total);
    for _ in 0..total {
        let e = world.spawn();
        world.add_component(
            e,
            Sprite {
                texture: tex.clone(),
                color: [0.0; 4],
                size: Vec2::new(CELL, CELL),
                transform: Mat4::from_translation(Vec3::new(-200.0, -200.0, 0.0)),
                flip_x: false,
                flip_y: false,
                uv_region: [0.0, 0.0, 1.0, 1.0],
            },
        );
        entities.push(e);
    }

    let mut plugin = RenderPlugin2D::new(window.clone());
    plugin.build(builder.world_mut());
    {
        builder
            .world_mut()
            .get_resource_mut::<engine_render::texture_bridge::TextureBridge>()
            .unwrap()
            .request(&tex, "");
    }
    let renderer = plugin.take_renderer().unwrap();
    let mut app = builder.build();
    app.set_renderer(renderer);

    let mut game = Game::new();
    let grid_off = 0;
    let hold_off = COLS * ROWS;
    let next_off = hold_off + 16;

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
                    if let winit::keyboard::PhysicalKey::Code(key) = ke.physical_key {
                        if ke.state == winit::event::ElementState::Pressed {
                            app.input_mut().press(key);
                        } else {
                            app.input_mut().release(key);
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
                let input_ref = app.world.get_resource::<InputManager>().unwrap();
                let input: &InputManager = unsafe { &*(input_ref as *const InputManager) };

                // Input
                if input.key_just_pressed(KeyCode::Escape) {
                    if game.over {
                        game.reset();
                    } else {
                        game.over = true;
                    }
                }
                if input.key_just_pressed(KeyCode::KeyP) && !game.over {
                    game.paused = !game.paused;
                }
                if !game.over && !game.paused {
                    if input.key_just_pressed(KeyCode::ArrowUp)
                        || input.key_just_pressed(KeyCode::KeyX)
                    {
                        game.rotate(1);
                    }
                    if input.key_just_pressed(KeyCode::KeyZ) {
                        game.rotate(3);
                    }
                    if input.key_just_pressed(KeyCode::Space) {
                        game.hard_drop();
                    }
                    if input.key_just_pressed(KeyCode::KeyC) {
                        game.hold_piece();
                    }
                    game.soft_drop =
                        input.key_down(KeyCode::ArrowDown) || input.key_down(KeyCode::KeyS);
                    if (input.key_just_pressed(KeyCode::ArrowDown)
                        || input.key_just_pressed(KeyCode::KeyS))
                        && game.try_move(0, 1)
                    {
                        game.score += 1;
                    }
                    let left = input.key_down(KeyCode::ArrowLeft) || input.key_down(KeyCode::KeyA);
                    let right =
                        input.key_down(KeyCode::ArrowRight) || input.key_down(KeyCode::KeyD);
                    let lj = input.key_just_pressed(KeyCode::ArrowLeft)
                        || input.key_just_pressed(KeyCode::KeyA);
                    let rj = input.key_just_pressed(KeyCode::ArrowRight)
                        || input.key_just_pressed(KeyCode::KeyD);
                    if lj {
                        game.try_move(-1, 0);
                        game.das_dir = -1;
                        game.das_acc = 0.0;
                        game.das_charged = false;
                    } else if rj {
                        game.try_move(1, 0);
                        game.das_dir = 1;
                        game.das_acc = 0.0;
                        game.das_charged = false;
                    } else if left && game.das_dir == -1 {
                        game.das_acc += dt;
                        if !game.das_charged {
                            if game.das_acc >= 0.167 {
                                game.das_charged = true;
                                game.das_acc = 0.0;
                                game.try_move(-1, 0);
                            }
                        } else {
                            while game.das_acc >= 0.033 {
                                game.das_acc -= 0.033;
                                game.try_move(-1, 0);
                            }
                        }
                    } else if right && game.das_dir == 1 {
                        game.das_acc += dt;
                        if !game.das_charged {
                            if game.das_acc >= 0.167 {
                                game.das_charged = true;
                                game.das_acc = 0.0;
                                game.try_move(1, 0);
                            }
                        } else {
                            while game.das_acc >= 0.033 {
                                game.das_acc -= 0.033;
                                game.try_move(1, 0);
                            }
                        }
                    }
                    if !left && !right {
                        game.das_dir = 0;
                        game.das_acc = 0.0;
                        game.das_charged = false;
                    }
                }

                // Update
                if !game.over && !game.paused {
                    let speed = if game.soft_drop {
                        0.04
                    } else {
                        game.fall_speed
                    };
                    game.fall_acc += dt;
                    if game.fall_acc >= speed {
                        game.fall_acc -= speed;
                        if game.try_move(0, 1) {
                            if game.soft_drop {
                                game.score += 1;
                            }
                            if game.lock_active {
                                game.lock_acc = 0.0;
                            }
                        } else if !game.lock_active {
                            game.lock_active = true;
                            game.lock_acc = 0.0;
                        }
                    }
                    if game.lock_active {
                        game.lock_acc += dt;
                        if game.lock_acc >= 0.5 {
                            game.lock();
                        }
                    }
                }

                // Render
                let ghost = game.ghost_y();
                let dim = if game.over { 0.3 } else { 1.0 };
                for r in 0..ROWS {
                    for c in 0..COLS {
                        let idx = grid_off + r * COLS + c;
                        let cell = game.grid[r][c];
                        let is_cur = !game.over
                            && cell == 0
                            && PIECES[game.piece][game.rot].iter().any(|&(bx, by)| {
                                game.px + bx == c as i32 && game.py + by == r as i32
                            });
                        let is_ghost = !game.over
                            && cell == 0
                            && !is_cur
                            && PIECES[game.piece][game.rot].iter().any(|&(bx, by)| {
                                game.px + bx == c as i32 && ghost + by == r as i32
                            });
                        let (color, sz) = if cell != 0 {
                            let cl = COLORS[cell as usize];
                            ([cl[0] * dim, cl[1] * dim, cl[2] * dim, 1.0], CELL)
                        } else if is_cur {
                            (COLORS[game.piece + 1], CELL)
                        } else if is_ghost {
                            let cl = COLORS[game.piece + 1];
                            ([cl[0], cl[1], cl[2], 0.15], CELL - 2.0)
                        } else {
                            (COLORS[0], CELL)
                        };
                        let cx = GRID_X + c as f32 * CELL + CELL * 0.5;
                        let cy = GRID_Y + r as f32 * CELL + CELL * 0.5;
                        if let Some(sp) =
                            app.world.get_by_index_mut::<Sprite>(entities[idx].index())
                        {
                            sp.color = color;
                            sp.size = Vec2::new(sz, sz);
                            sp.transform = Mat4::from_translation(Vec3::new(cx, cy, 0.0));
                        }
                    }
                }

                // Hold
                for i in 0..16 {
                    let e = entities[hold_off + i];
                    let (c, r) = (i % 4, i / 4);
                    let show = game.hold.is_some_and(|h| {
                        PIECES[h][0]
                            .iter()
                            .any(|&(bx, by)| bx as usize == c && by as usize == r)
                    });
                    if let Some(sp) = app.world.get_by_index_mut::<Sprite>(e.index()) {
                        if show {
                            let h = game.hold.unwrap();
                            let cl = if game.hold_used {
                                [0.25, 0.25, 0.3, 1.0]
                            } else {
                                COLORS[h + 1]
                            };
                            sp.color = cl;
                            sp.size = Vec2::new(CELL - 2.0, CELL - 2.0);
                            sp.transform = Mat4::from_translation(Vec3::new(
                                380.0 + c as f32 * CELL + CELL * 0.5,
                                60.0 + r as f32 * CELL + CELL * 0.5,
                                0.0,
                            ));
                        } else {
                            sp.color = [0.0; 4];
                            sp.transform = Mat4::from_translation(Vec3::new(-200.0, -200.0, 0.0));
                        }
                    }
                }

                // Next
                for qi in 0..3 {
                    for i in 0..16 {
                        let e = entities[next_off + qi * 16 + i];
                        let (c, r) = (i % 4, i / 4);
                        let show = PIECES[game.next[qi]][0]
                            .iter()
                            .any(|&(bx, by)| bx as usize == c && by as usize == r);
                        if let Some(sp) = app.world.get_by_index_mut::<Sprite>(e.index()) {
                            if show {
                                sp.color = COLORS[game.next[qi] + 1];
                                sp.size = Vec2::new(CELL - 2.0, CELL - 2.0);
                                sp.transform = Mat4::from_translation(Vec3::new(
                                    380.0 + c as f32 * CELL + CELL * 0.5,
                                    220.0 + qi as f32 * 140.0 + r as f32 * CELL + CELL * 0.5,
                                    0.0,
                                ));
                            } else {
                                sp.color = [0.0; 4];
                                sp.transform =
                                    Mat4::from_translation(Vec3::new(-200.0, -200.0, 0.0));
                            }
                        }
                    }
                }

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
