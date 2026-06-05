//! 3D Dungeon Explorer Demo — validates engine 3D subsystems working together.
//!
//! Demonstrates:
//! 1. Procedural dungeon generation
//! 2. PBR materials with directional and point lighting
//! 3. First-person camera controller (WASD + mouse)
//! 4. 3D physics collision with walls
//! 5. Collectibles (treasure chests, keys)
//! 6. Enemy AI (patrol + chase states)
//! 7. Game state management (pause, game over)
//!
//! Controls: WASD to move, Mouse to look, Escape to pause
//! Goal: Collect treasures, avoid enemies, find the exit

use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;
use engine_core::transform::Transform;
use engine_ecs::world::World;
use engine_framework::{FrameworkPlugin, GameStateAction};
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::Vec3;
use engine_physics::collider::Collider;
use engine_physics::body::RigidBody;
use engine_physics::{PhysicsPlugin, PhysicsWorld};
use engine_render::camera::Camera;
use engine_render::light::{DirectionalLight, PointLight};
use engine_render::mesh_bridge::MeshRenderer;
use engine_render::resource::material::PbrMaterial;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TILE_SIZE: f32 = 2.0;
const MOVE_SPEED: f32 = 8.0;
const MOUSE_SENSITIVITY: f32 = 0.003;
const DUNGEON_WIDTH: usize = 64;
const DUNGEON_HEIGHT: usize = 64;

const TILE_EMPTY: u8 = 0;
const TILE_WALL: u8 = 1;
const TILE_FLOOR: u8 = 2;
const TILE_CORRIDOR: u8 = 3;

// ---------------------------------------------------------------------------
// Dungeon data structures
// ---------------------------------------------------------------------------

struct Dungeon {
    tiles: [[u8; DUNGEON_WIDTH]; DUNGEON_HEIGHT],
    rooms: Vec<Room>,
}

#[derive(Clone)]
struct Room {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Dungeon {
    fn new() -> Self {
        Self {
            tiles: [[TILE_EMPTY; DUNGEON_WIDTH]; DUNGEON_HEIGHT],
            rooms: Vec::new(),
        }
    }

    fn generate(&mut self) {
        // Place 6 non-overlapping rooms
        for _ in 0..20 {
            if self.rooms.len() >= 6 {
                break;
            }
            let w = 5 + (rand_usize() % 6);
            let h = 5 + (rand_usize() % 6);
            let x = 1 + (rand_usize() % (DUNGEON_WIDTH.saturating_sub(w + 2)));
            let y = 1 + (rand_usize() % (DUNGEON_HEIGHT.saturating_sub(h + 2)));
            let room = Room { x, y, width: w, height: h };
            if !self.rooms.iter().any(|r| self.rooms_overlap(&room, r)) {
                self.carve_room(&room);
                self.rooms.push(room);
            }
        }

        // Connect adjacent rooms with corridors
        for i in 1..self.rooms.len() {
            let (ax, ay) = self.room_center(&self.rooms[i - 1]);
            let (bx, by) = self.room_center(&self.rooms[i]);
            self.carve_corridor(ax, ay, bx, by);
        }

        // Fill walls around non-empty tiles
        self.fill_walls();

        println!("Dungeon generated: {} rooms", self.rooms.len());
        for (i, room) in self.rooms.iter().enumerate() {
            let (cx, cy) = self.room_center(room);
            println!(
                "  Room {}: pos=({}, {}) size=({}, {}) center=({}, {})",
                i, room.x, room.y, room.width, room.height, cx, cy
            );
        }
    }

    fn rooms_overlap(&self, a: &Room, b: &Room) -> bool {
        a.x < b.x + b.width + 1
            && a.x + a.width + 1 > b.x
            && a.y < b.y + b.height + 1
            && a.y + a.height + 1 > b.y
    }

    fn room_center(&self, room: &Room) -> (usize, usize) {
        (room.x + room.width / 2, room.y + room.height / 2)
    }

    fn carve_room(&mut self, room: &Room) {
        for y in room.y..room.y + room.height {
            for x in room.x..room.x + room.width {
                self.tiles[y][x] = TILE_FLOOR;
            }
        }
    }

    fn carve_corridor(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        let (sx, ex) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        for x in sx..=ex {
            if self.tiles[y0][x] == TILE_EMPTY {
                self.tiles[y0][x] = TILE_CORRIDOR;
            }
        }
        let (sy, ey) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        for y in sy..=ey {
            if self.tiles[y][x1] == TILE_EMPTY {
                self.tiles[y][x1] = TILE_CORRIDOR;
            }
        }
    }

    fn fill_walls(&mut self) {
        let mut wall_map = [[false; DUNGEON_WIDTH]; DUNGEON_HEIGHT];
        for y in 0..DUNGEON_HEIGHT {
            for x in 0..DUNGEON_WIDTH {
                if self.tiles[y][x] != TILE_EMPTY {
                    for dy in [-1i32, 0, 1] {
                        for dx in [-1i32, 0, 1] {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx >= 0
                                && nx < DUNGEON_WIDTH as i32
                                && ny >= 0
                                && ny < DUNGEON_HEIGHT as i32
                            {
                                let (nx, ny) = (nx as usize, ny as usize);
                                if self.tiles[ny][nx] == TILE_EMPTY {
                                    wall_map[ny][nx] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        for y in 0..DUNGEON_HEIGHT {
            for x in 0..DUNGEON_WIDTH {
                if wall_map[y][x] {
                    self.tiles[y][x] = TILE_WALL;
                }
            }
        }
    }
}

/// Simple pseudo-random number generator (no external crate dependency).
fn rand_usize() -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(12345);
    let prev = SEED.load(Ordering::Relaxed);
    let next = prev.wrapping_mul(6364136223846793005).wrapping_add(1);
    SEED.store(next, Ordering::Relaxed);
    let mut h = DefaultHasher::new();
    next.hash(&mut h);
    h.finish() as usize
}

// ---------------------------------------------------------------------------
// ECS Components
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PlayerState {
    lives: i32,
    score: i32,
    yaw: f32,
    pitch: f32,
    has_key: bool,
}

impl PlayerState {
    fn new() -> Self {
        Self {
            lives: 3,
            score: 0,
            yaw: 0.0,
            pitch: 0.0,
            has_key: false,
        }
    }
}

#[derive(Debug, Clone)]
struct Collectible {
    kind: CollectibleKind,
    collected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CollectibleKind {
    Key,
    Treasure,
}

#[derive(Debug, Clone)]
struct EnemyAI {
    state: EnemyState,
    patrol_points: Vec<(f32, f32)>,
    current_point: usize,
    speed: f32,
    chase_speed: f32,
    detection_range: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnemyState {
    Patrol,
    Chase,
}

#[derive(Debug, Clone)]
struct ExitDoor;

#[derive(Debug, Clone)]
struct DungeonTile;

// ---------------------------------------------------------------------------
// Entity spawning
// ---------------------------------------------------------------------------

fn spawn_dungeon(world: &mut World, dungeon: &Dungeon) {
    let mut wall_count = 0u32;
    let mut floor_count = 0u32;

    for y in 0..DUNGEON_HEIGHT {
        for x in 0..DUNGEON_WIDTH {
            let tile = dungeon.tiles[y][x];
            if tile == TILE_EMPTY {
                continue;
            }
            let px = x as f32 * TILE_SIZE;
            let pz = y as f32 * TILE_SIZE;

            if tile == TILE_FLOOR || tile == TILE_CORRIDOR {
                // Floor: flat box
                let e = world.spawn();
                world.add_component(
                    e,
                    Transform::from_xyz(px, -0.5, pz)
                        .with_scale(Vec3::new(TILE_SIZE, 0.5, TILE_SIZE)),
                );
                world.add_component(
                    e,
                    PbrMaterial::new([0.25, 0.25, 0.3, 1.0], 0.0, 0.9),
                );
                world.add_component(
                    e,
                    MeshRenderer {
                        mesh_id: 0,
                        material_id: 0,
                        cast_shadow: false,
                    },
                );
                // Static collider so player can stand on floor
                world.add_component(e, RigidBody::new_static());
                world.add_component(e, Collider::cuboid(TILE_SIZE * 0.5, 0.5, TILE_SIZE * 0.5));
                world.add_component(e, DungeonTile);
                floor_count += 1;
            } else if tile == TILE_WALL {
                // Wall: tall box
                let e = world.spawn();
                world.add_component(
                    e,
                    Transform::from_xyz(px, 1.5, pz)
                        .with_scale(Vec3::new(TILE_SIZE, 3.0, TILE_SIZE)),
                );
                world.add_component(
                    e,
                    PbrMaterial::new([0.45, 0.38, 0.32, 1.0], 0.0, 0.85),
                );
                world.add_component(
                    e,
                    MeshRenderer {
                        mesh_id: 0,
                        material_id: 0,
                        cast_shadow: true,
                    },
                );
                // Static collider for wall
                world.add_component(e, RigidBody::new_static());
                world.add_component(e, Collider::cuboid(TILE_SIZE * 0.5, 1.5, TILE_SIZE * 0.5));
                world.add_component(e, DungeonTile);
                wall_count += 1;
            }
        }
    }

    println!(
        "Spawned dungeon geometry: {} floors, {} walls",
        floor_count, wall_count
    );
}

fn spawn_player(world: &mut World, dungeon: &Dungeon) {
    let (cx, cy) = dungeon.room_center(&dungeon.rooms[0]);
    let px = cx as f32 * TILE_SIZE;
    let pz = cy as f32 * TILE_SIZE;

    let player = world.spawn();
    world.add_component(player, Transform::from_xyz(px, 1.0, pz));
    world.add_component(
        player,
        Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 200.0),
    );
    // Dynamic body for physics (gravity + collision)
    let mut body = RigidBody::new_dynamic();
    body.gravity_scale = 1.0;
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;
    // Lock rotation to prevent the capsule from tipping over
    world.add_component(player, body);
    // Capsule collider: radius 0.3, height 1.4
    world.add_component(player, Collider::capsule(0.3, 1.4));
    world.add_component(player, PlayerState::new());

    println!("Player spawned at ({}, {})", px, pz);
}

fn spawn_lights(world: &mut World, dungeon: &Dungeon) {
    // Dim directional light (ambient fill)
    let sun = world.spawn();
    world.add_component(sun, Transform::from_xyz(0.0, 50.0, 0.0));
    world.add_component(
        sun,
        DirectionalLight {
            direction: [0.0, -1.0, 0.0],
            color: [0.15, 0.12, 0.1],
            intensity: 0.3,
            enabled: true,
        },
    );

    // Point light (torch) in each room center
    for room in &dungeon.rooms {
        let (cx, cy) = dungeon.room_center(room);
        let lx = cx as f32 * TILE_SIZE;
        let lz = cy as f32 * TILE_SIZE;

        let light = world.spawn();
        world.add_component(light, Transform::from_xyz(lx, 2.8, lz));
        world.add_component(
            light,
            PointLight {
                color: [1.0, 0.75, 0.4],
                intensity: 4.0,
                range: 18.0,
                enabled: true,
            },
        );
    }

    println!("Spawned 1 directional light + {} point lights", dungeon.rooms.len());
}

fn spawn_collectibles(world: &mut World, dungeon: &Dungeon) {
    // Treasure in each room except the first (player spawn)
    for room in dungeon.rooms.iter().skip(1) {
        let (cx, cy) = dungeon.room_center(room);
        let px = cx as f32 * TILE_SIZE;
        let pz = cy as f32 * TILE_SIZE;

        let e = world.spawn();
        world.add_component(
            e,
            Transform::from_xyz(px, 0.5, pz).with_scale(Vec3::new(0.5, 0.5, 0.5)),
        );
        world.add_component(e, PbrMaterial::new([1.0, 0.84, 0.0, 1.0], 0.8, 0.2));
        world.add_component(
            e,
            MeshRenderer {
                mesh_id: 0,
                material_id: 0,
                cast_shadow: true,
            },
        );
        world.add_component(e, Collider::cuboid(0.5, 0.5, 0.5));
        world.add_component(e, Collectible { kind: CollectibleKind::Treasure, collected: false });
    }

    // Key in the last room
    if let Some(room) = dungeon.rooms.last() {
        let (cx, cy) = dungeon.room_center(room);
        let px = cx as f32 * TILE_SIZE;
        let pz = cy as f32 * TILE_SIZE;

        let e = world.spawn();
        world.add_component(
            e,
            Transform::from_xyz(px, 0.5, pz).with_scale(Vec3::new(0.3, 0.6, 0.1)),
        );
        world.add_component(e, PbrMaterial::new([0.8, 0.8, 0.1, 1.0], 0.9, 0.1));
        world.add_component(
            e,
            MeshRenderer {
                mesh_id: 0,
                material_id: 0,
                cast_shadow: true,
            },
        );
        world.add_component(e, Collider::cuboid(0.3, 0.6, 0.1));
        world.add_component(e, Collectible { kind: CollectibleKind::Key, collected: false });
    }

    // Exit door in the last room (near the key)
    if let Some(room) = dungeon.rooms.last() {
        let (cx, cy) = dungeon.room_center(room);
        let px = cx as f32 * TILE_SIZE + 2.0;
        let pz = cy as f32 * TILE_SIZE;

        let e = world.spawn();
        world.add_component(
            e,
            Transform::from_xyz(px, 1.5, pz).with_scale(Vec3::new(1.0, 3.0, 0.3)),
        );
        world.add_component(e, PbrMaterial::new([0.4, 0.25, 0.1, 1.0], 0.0, 0.7));
        world.add_component(
            e,
            MeshRenderer {
                mesh_id: 0,
                material_id: 0,
                cast_shadow: true,
            },
        );
        world.add_component(e, Collider::cuboid(1.0, 3.0, 0.3));
        world.add_component(e, ExitDoor);
    }

    println!(
        "Spawned {} treasures + 1 key + 1 exit door",
        dungeon.rooms.len().saturating_sub(1)
    );
}

fn spawn_enemies(world: &mut World, dungeon: &Dungeon) {
    for room in dungeon.rooms.iter().skip(1) {
        let (cx, cy) = dungeon.room_center(room);
        let px = cx as f32 * TILE_SIZE;
        let pz = cy as f32 * TILE_SIZE;

        let half_w = (room.width as f32 / 2.0 - 1.0) * TILE_SIZE;
        let patrol_points = vec![(px - half_w, pz), (px + half_w, pz)];

        let e = world.spawn();
        world.add_component(
            e,
            Transform::from_xyz(px, 0.75, pz).with_scale(Vec3::new(0.6, 1.5, 0.6)),
        );
        world.add_component(e, PbrMaterial::new([0.8, 0.1, 0.1, 1.0], 0.0, 0.6));
        world.add_component(
            e,
            MeshRenderer {
                mesh_id: 0,
                material_id: 0,
                cast_shadow: true,
            },
        );
        let mut body = RigidBody::new_kinematic();
        body.gravity_scale = 0.0;
        world.add_component(e, body);
        world.add_component(e, Collider::cuboid(0.6, 1.5, 0.6));
        world.add_component(
            e,
            EnemyAI {
                state: EnemyState::Patrol,
                patrol_points,
                current_point: 0,
                speed: 3.0,
                chase_speed: 5.0,
                detection_range: 10.0,
            },
        );
    }

    println!(
        "Spawned {} enemies",
        dungeon.rooms.len().saturating_sub(1)
    );
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn player_control_system(world: &mut World) {
    // Read input state in a block to avoid borrow conflicts
    let (fwd, back, left, right, mdx, mdy) = {
        let input = match world.get_resource::<InputManager>() {
            Some(i) => i,
            None => return,
        };
        (
            input.key_down(KeyCode::KeyW) || input.key_down(KeyCode::ArrowUp),
            input.key_down(KeyCode::KeyS) || input.key_down(KeyCode::ArrowDown),
            input.key_down(KeyCode::KeyA) || input.key_down(KeyCode::ArrowLeft),
            input.key_down(KeyCode::KeyD) || input.key_down(KeyCode::ArrowRight),
            input.mouse().delta.0 as f32,
            input.mouse().delta.1 as f32,
        )
    };

    let entities = world.component_entities::<PlayerState>();
    for &eid in &entities {
        // Mouse rotation
        if let Some(player) = world.get_by_index_mut::<PlayerState>(eid) {
            player.yaw -= mdx * MOUSE_SENSITIVITY;
            player.pitch = (player.pitch - mdy * MOUSE_SENSITIVITY).clamp(-1.4, 1.4);
        }

        // Get yaw for movement direction
        let yaw = world
            .get_by_index::<PlayerState>(eid)
            .map(|p| p.yaw)
            .unwrap_or(0.0);

        // Movement direction in XZ plane
        let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let right_dir = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
        let mut move_dir = Vec3::ZERO;
        if fwd {
            move_dir += forward;
        }
        if back {
            move_dir -= forward;
        }
        if right {
            move_dir += right_dir;
        }
        if left {
            move_dir -= right_dir;
        }
        let len = move_dir.length();
        if len > 1e-6 {
            move_dir = move_dir / len;
        }

        // Apply horizontal velocity to rigid body (preserve Y for gravity)
        if let Some(body) = world.get_by_index_mut::<RigidBody>(eid) {
            body.linear_velocity.x = move_dir.x * MOVE_SPEED;
            body.linear_velocity.z = move_dir.z * MOVE_SPEED;
        }

        // Update camera orientation
        let pitch = world
            .get_by_index::<PlayerState>(eid)
            .map(|p| p.pitch)
            .unwrap_or(0.0);
        if let Some(transform) = world.get_by_index_mut::<Transform>(eid) {
            transform.rotation = Vec3::new(pitch, yaw, 0.0);
        }
    }
}

fn enemy_ai_system(world: &mut World) {
    // Get player position
    let player_pos = {
        let players = world.component_entities::<PlayerState>();
        players.first().and_then(|&eid| {
            world
                .get_by_index::<Transform>(eid)
                .map(|t| Vec3::new(t.position.x, t.position.y, t.position.z))
        })
    };
    let player_pos = match player_pos {
        Some(p) => p,
        None => return,
    };

    let entities = world.component_entities::<EnemyAI>();
    for &eid in &entities {
        let (state, detection_range, speed, chase_speed, target_x, target_z) = {
            let ai = world.get_by_index::<EnemyAI>(eid).unwrap();
            let target = ai.patrol_points[ai.current_point];
            (
                ai.state,
                ai.detection_range,
                ai.speed,
                ai.chase_speed,
                target.0,
                target.1,
            )
        };

        let current_pos = world
            .get_by_index::<Transform>(eid)
            .map(|t| Vec3::new(t.position.x, t.position.y, t.position.z))
            .unwrap_or(Vec3::ZERO);

        let dist_to_player = (current_pos - player_pos).length();

        // State transitions
        let new_state = match state {
            EnemyState::Patrol if dist_to_player < detection_range => EnemyState::Chase,
            EnemyState::Chase if dist_to_player > detection_range * 1.5 => EnemyState::Patrol,
            s => s,
        };

        // Update state
        if let Some(ai) = world.get_by_index_mut::<EnemyAI>(eid) {
            ai.state = new_state;
        }

        // Compute movement target
        let (goal_x, goal_z, move_speed) = match new_state {
            EnemyState::Patrol => (target_x, target_z, speed),
            EnemyState::Chase => (player_pos.x, player_pos.z, chase_speed),
        };

        // Move toward target
        let dx = goal_x - current_pos.x;
        let dz = goal_z - current_pos.z;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist > 0.5 {
            let vx = (dx / dist) * move_speed;
            let vz = (dz / dist) * move_speed;
            if let Some(body) = world.get_by_index_mut::<RigidBody>(eid) {
                body.linear_velocity = Vec3::new(vx, body.linear_velocity.y, vz);
            }
        }

        // Advance patrol waypoint when close enough
        if new_state == EnemyState::Patrol && dist < 1.0 {
            if let Some(ai) = world.get_by_index_mut::<EnemyAI>(eid) {
                ai.current_point = (ai.current_point + 1) % ai.patrol_points.len();
            }
        }
    }
}

fn collectible_system(world: &mut World) {
    // Read collision events
    let collision_pairs: Vec<(u32, u32)> = {
        let pw = match world.get_resource::<PhysicsWorld>() {
            Some(pw) => pw,
            None => return,
        };
        pw.collision_events
            .iter()
            .filter(|e| e.is_enter)
            .map(|e| (e.entity_a, e.entity_b))
            .collect()
    };

    for (a, b) in collision_pairs {
        let (player_eid, collectible_eid) =
            if world.get_by_index::<PlayerState>(a).is_some()
                && world.get_by_index::<Collectible>(b).is_some()
            {
                (a, b)
            } else if world.get_by_index::<PlayerState>(b).is_some()
                && world.get_by_index::<Collectible>(a).is_some()
            {
                (b, a)
            } else {
                continue;
            };

        if let Some(col) = world.get_by_index_mut::<Collectible>(collectible_eid) {
            if !col.collected {
                col.collected = true;
                let kind = col.kind;
                if let Some(player) = world.get_by_index_mut::<PlayerState>(player_eid) {
                    match kind {
                        CollectibleKind::Treasure => {
                            player.score += 100;
                            println!(
                                "[Collect] Treasure! Score: {} | Lives: {}",
                                player.score, player.lives
                            );
                        }
                        CollectibleKind::Key => {
                            player.has_key = true;
                            println!("[Collect] Key obtained!");
                        }
                    }
                }
                // Entity stays in world but marked as collected (no further interaction)
            }
        }
    }
}

fn enemy_collision_system(world: &mut World) {
    let collision_pairs: Vec<(u32, u32)> = {
        let pw = match world.get_resource::<PhysicsWorld>() {
            Some(pw) => pw,
            None => return,
        };
        pw.collision_events
            .iter()
            .filter(|e| e.is_enter)
            .map(|e| (e.entity_a, e.entity_b))
            .collect()
    };

    for (a, b) in collision_pairs {
        let (player_eid, _enemy_eid) =
            if world.get_by_index::<PlayerState>(a).is_some()
                && world.get_by_index::<EnemyAI>(b).is_some()
            {
                (a, b)
            } else if world.get_by_index::<PlayerState>(b).is_some()
                && world.get_by_index::<EnemyAI>(a).is_some()
            {
                (b, a)
            } else {
                continue;
            };

        if let Some(player) = world.get_by_index_mut::<PlayerState>(player_eid) {
            player.lives -= 1;
            println!(
                "[Hit] Enemy collision! Lives: {} | Score: {}",
                player.lives, player.score
            );
        }
    }
}

fn exit_door_system(world: &mut World) {
    let collision_pairs: Vec<(u32, u32)> = {
        let pw = match world.get_resource::<PhysicsWorld>() {
            Some(pw) => pw,
            None => return,
        };
        pw.collision_events
            .iter()
            .filter(|e| e.is_enter)
            .map(|e| (e.entity_a, e.entity_b))
            .collect()
    };

    for (a, b) in collision_pairs {
        let has_player = world.get_by_index::<PlayerState>(a).is_some()
            || world.get_by_index::<PlayerState>(b).is_some();
        let has_exit = world.get_by_index::<ExitDoor>(a).is_some()
            || world.get_by_index::<ExitDoor>(b).is_some();

        if has_player && has_exit {
            let player_eid = if world.get_by_index::<PlayerState>(a).is_some() {
                a
            } else {
                b
            };
            if let Some(player) = world.get_by_index::<PlayerState>(player_eid) {
                if player.has_key {
                    println!(
                        "\n*** LEVEL COMPLETE! Final Score: {} ***\n",
                        player.score
                    );
                } else {
                    println!("[Exit] You need the key to open this door!");
                }
            }
        }
    }
}

fn death_check_system(world: &mut World) {
    let entities = world.component_entities::<PlayerState>();
    for &eid in &entities {
        let (lives, score) = world
            .get_by_index::<PlayerState>(eid)
            .map(|p| (p.lives, p.score))
            .unwrap_or((0, 0));
        if lives <= 0 {
            println!("\n*** GAME OVER! Final Score: {} ***\n", score);
            if let Some(action) = world.get_resource_mut::<GameStateAction>() {
                *action = GameStateAction::PushGameOver { score };
            }
        }
    }
}

fn pause_system(world: &mut World) {
    let should_pause = {
        let input = match world.get_resource::<InputManager>() {
            Some(i) => i,
            None => return,
        };
        input.key_just_pressed(KeyCode::Escape)
    };

    if should_pause {
        if let Some(action) = world.get_resource_mut::<GameStateAction>() {
            *action = GameStateAction::PushPause;
            println!("[Pause] Game paused. Press Escape to resume.");
        }
    }
}

fn status_report_system(world: &mut World) {
    // Throttle: only report every ~60 frames using elapsed time
    let elapsed = world
        .get_resource::<Time>()
        .map(|t| t.elapsed_seconds())
        .unwrap_or(0.0);
    // Print at ~0.5s intervals (skip first few frames)
    if elapsed < 0.5 || (elapsed % 1.0) > 0.02 {
        return;
    }

    let entities = world.component_entities::<PlayerState>();
    if let Some(&eid) = entities.first() {
        if let (Some(player), Some(transform)) = (
            world.get_by_index::<PlayerState>(eid),
            world.get_by_index::<Transform>(eid),
        ) {
            println!(
                "[Status] t={:.1}s | Pos: ({:.1}, {:.1}, {:.1}) | Lives: {} | Score: {} | Key: {}",
                elapsed,
                transform.position.x,
                transform.position.y,
                transform.position.z,
                player.lives,
                player.score,
                player.has_key,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== RustEngine 3D Dungeon Explorer Demo ===\n");

    // Build app
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);
    builder.add_plugin(FrameworkPlugin);
    builder.add_plugin(PhysicsPlugin);

    // Configure physics for dungeon scale
    {
        let pw = builder
            .world_mut()
            .get_resource_mut::<PhysicsWorld>()
            .unwrap();
        pw.gravity = Vec3::new(0.0, -20.0, 0.0);
        pw.set_broadphase_cell_size(4.0);
    }

    // Generate dungeon
    let mut dungeon = Dungeon::new();
    dungeon.generate();

    // Spawn entities
    spawn_dungeon(builder.world_mut(), &dungeon);
    spawn_player(builder.world_mut(), &dungeon);
    spawn_lights(builder.world_mut(), &dungeon);
    spawn_collectibles(builder.world_mut(), &dungeon);
    spawn_enemies(builder.world_mut(), &dungeon);

    // Register systems
    builder.add_system(player_control_system);
    builder.add_system(enemy_ai_system);
    builder.add_system(collectible_system);
    builder.add_system(enemy_collision_system);
    builder.add_system(exit_door_system);
    builder.add_system(death_check_system);
    builder.add_system(pause_system);
    builder.add_system(status_report_system);

    let mut app = builder.build();

    let entity_count = app.world.entity_count();
    println!(
        "\nStarting simulation with {} entities...\n",
        entity_count
    );

    // Simulation loop (terminal-based, no window)
    for frame in 0..600u32 {
        // Simulate forward movement for first 200 frames
        {
            let input = app.world.get_resource_mut::<InputManager>().unwrap();
            if frame == 0 {
                input.press(KeyCode::KeyW);
            }
            if frame == 200 {
                input.release(KeyCode::KeyW);
                input.press(KeyCode::KeyD);
            }
            if frame == 350 {
                input.release(KeyCode::KeyD);
                input.press(KeyCode::KeyW);
            }
            if frame == 500 {
                input.release(KeyCode::KeyW);
            }
        }

        // Run all systems
        app.run();

        // Print physics stats every 120 frames
        if frame == 0 || frame == 119 || frame == 299 || frame == 599 {
            let pw = app.world.get_resource::<PhysicsWorld>();
            if let Some(pw) = pw {
                let elapsed = app.world.get_resource::<Time>().map(|t| t.elapsed_seconds()).unwrap_or(0.0);
                println!(
                    "[Physics] frame={} t={:.1}s | Bodies: {} | Colliders: {} | Collisions: {}",
                    frame, elapsed,
                    pw.body_count,
                    pw.collider_count,
                    pw.collision_events.len()
                );
            }
        }
    }

    println!("\nSimulation complete (600 frames).");
}
