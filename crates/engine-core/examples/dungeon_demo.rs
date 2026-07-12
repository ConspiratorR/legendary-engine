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

// AppBuilder bootstraps the engine: registers plugins, systems, and resources.
use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_core::time::Time;
// Transform is the spatial component — position, rotation, scale in world space.
use engine_core::transform::Transform;
// ECS World holds all entities and their components; systems query it each frame.
use engine_ecs::world::World;
// FrameworkPlugin provides game-state stack (pause, game over screens).
use engine_framework::{FrameworkPlugin, GameStateAction};
// InputManager is a resource polled by systems to read keyboard/mouse state.
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;
use engine_math::Vec3;
// RigidBody + Collider are the physics components; PhysicsWorld is the runtime resource.
use engine_physics::body::RigidBody;
use engine_physics::collider::Collider;
use engine_physics::{PhysicsPlugin, PhysicsWorld};
// Camera defines the viewpoint — perspective projection with fov, near/far planes.
use engine_render::camera::Camera;
// Lighting types: DirectionalLight (sun/ambient), PointLight (torches).
use engine_render::light::{DirectionalLight, PointLight};
// MeshRenderer is the bridge component that tells the render pipeline to draw an entity.
// Each entity with Transform + PbrMaterial + MeshRenderer becomes a renderable object.
use engine_render::mesh_bridge::MeshRenderer;
// PBR material: albedo color, metallic, roughness — drives the deferred shading model.
use engine_render::resource::material::PbrMaterial;

// ---------------------------------------------------------------------------
// Constants — tune these to change dungeon scale and player feel
// ---------------------------------------------------------------------------

/// World-space size of one dungeon tile (used for both floor and wall meshes).
const TILE_SIZE: f32 = 2.0;
/// Player movement speed in units/second.
const MOVE_SPEED: f32 = 8.0;
/// Radians of rotation per pixel of mouse movement.
const MOUSE_SENSITIVITY: f32 = 0.003;
/// Dungeon grid dimensions (in tiles).
const DUNGEON_WIDTH: usize = 64;
const DUNGEON_HEIGHT: usize = 64;

// Tile type IDs for the procedural dungeon grid.
const TILE_EMPTY: u8 = 0;
const TILE_WALL: u8 = 1;
const TILE_FLOOR: u8 = 2;
const TILE_CORRIDOR: u8 = 3;

// ---------------------------------------------------------------------------
// Dungeon data structures — procedural generation via room placement + corridors
// ---------------------------------------------------------------------------

/// 2D grid of tile IDs. The generator fills this, then `spawn_dungeon` converts
/// each tile into an ECS entity with mesh + collider.
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

    /// Generate the dungeon: place rooms, connect with corridors, fill walls.
    /// Uses a simple BSP-like approach: try random placements, reject overlaps.
    fn generate(&mut self) {
        // Phase 1: Place up to 6 non-overlapping rooms (random size 5-10 tiles)
        for _ in 0..20 {
            if self.rooms.len() >= 6 {
                break;
            }
            let w = 5 + (rand_usize() % 6);
            let h = 5 + (rand_usize() % 6);
            let x = 1 + (rand_usize() % (DUNGEON_WIDTH.saturating_sub(w + 2)));
            let y = 1 + (rand_usize() % (DUNGEON_HEIGHT.saturating_sub(h + 2)));
            let room = Room {
                x,
                y,
                width: w,
                height: h,
            };
            if !self.rooms.iter().any(|r| self.rooms_overlap(&room, r)) {
                self.carve_room(&room);
                self.rooms.push(room);
            }
        }

        // Phase 2: Connect adjacent rooms with L-shaped corridors
        for i in 1..self.rooms.len() {
            let (ax, ay) = self.room_center(&self.rooms[i - 1]);
            let (bx, by) = self.room_center(&self.rooms[i]);
            self.carve_corridor(ax, ay, bx, by);
        }

        // Phase 3: Auto-tile walls — any empty tile adjacent to a non-empty tile becomes a wall.
        // This ensures the dungeon has proper boundaries for collision and occlusion.
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
// ECS Components — each struct is a component attached to entities via World
// ---------------------------------------------------------------------------

/// Player-specific state: health, score, look direction, inventory.
/// Attached to the player entity alongside Transform, Camera, RigidBody, Collider.
#[derive(Debug, Clone)]
struct PlayerState {
    lives: i32,
    score: i32,
    /// Horizontal look angle (radians, rotates around Y axis).
    yaw: f32,
    /// Vertical look angle (radians, clamped to prevent flipping).
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

/// Marker component for pickup-able items. The `collectible_system` reads
/// collision events to detect when the player touches one.
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

/// Enemy AI state: patrol between waypoints, or chase the player when detected.
/// Uses a finite state machine — transitions are distance-based.
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
// Entity spawning — each tile becomes an ECS entity with rendering + physics
// ---------------------------------------------------------------------------

/// Convert the 2D tile grid into 3D entities.
///
/// Each tile spawns an entity with the **rendering triplet**:
///   Transform (where in world space) + PbrMaterial (how it looks) + MeshRenderer (draw call).
/// Walls also get `cast_shadow: true` so the shadow map includes them.
/// All tiles get static RigidBody + Collider for physics collision.
fn spawn_dungeon(world: &mut World, dungeon: &Dungeon) {
    let mut wall_count = 0u32;
    let mut floor_count = 0u32;

    for y in 0..DUNGEON_HEIGHT {
        for x in 0..DUNGEON_WIDTH {
            let tile = dungeon.tiles[y][x];
            if tile == TILE_EMPTY {
                continue;
            }
            // Convert grid coords to world-space position (XZ plane, Y is up).
            let px = x as f32 * TILE_SIZE;
            let pz = y as f32 * TILE_SIZE;

            if tile == TILE_FLOOR || tile == TILE_CORRIDOR {
                // Floor tile: flat box sitting below the player's feet.
                // PBR params: dark stone-gray albedo, non-metallic, high roughness (matte).
                let e = world.spawn();
                world.add_component(
                    e,
                    Transform::from_xyz(px, -0.5, pz)
                        .with_scale(Vec3::new(TILE_SIZE, 0.5, TILE_SIZE)),
                );
                world.add_component(e, PbrMaterial::new([0.25, 0.25, 0.3, 1.0], 0.0, 0.9));
                // MeshRenderer bridges the ECS entity to the GPU render pipeline.
                // cast_shadow=false for floors — they receive shadows but don't cast them.
                world.add_component(
                    e,
                    MeshRenderer {
                        mesh_id: 0,     // engine default cube mesh
                        material_id: 0, // resolved at render time
                        cast_shadow: false,
                    },
                );
                // Static rigid body: participates in collision but doesn't move.
                world.add_component(e, RigidBody::new_static());
                world.add_component(e, Collider::cuboid(TILE_SIZE * 0.5, 0.5, TILE_SIZE * 0.5));
                world.add_component(e, DungeonTile);
                floor_count += 1;
            } else if tile == TILE_WALL {
                // Wall tile: tall box (3 units high) centered above the floor.
                // PBR params: warm brown stone, non-metallic, slightly rough.
                let e = world.spawn();
                world.add_component(
                    e,
                    Transform::from_xyz(px, 1.5, pz)
                        .with_scale(Vec3::new(TILE_SIZE, 3.0, TILE_SIZE)),
                );
                world.add_component(e, PbrMaterial::new([0.45, 0.38, 0.32, 1.0], 0.0, 0.85));
                // cast_shadow=true: walls occlude light, contributing to the shadow map.
                world.add_component(
                    e,
                    MeshRenderer {
                        mesh_id: 0,
                        material_id: 0,
                        cast_shadow: true,
                    },
                );
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

/// Spawn the player entity in the first room.
///
/// The player entity is a **camera rig**: Transform + Camera + physics body.
/// The Camera component drives the view/projection matrices in the render pipeline.
/// The perspective camera uses a 45° FOV with near=0.1 and far=200 for the depth buffer.
fn spawn_player(world: &mut World, dungeon: &Dungeon) {
    let (cx, cy) = dungeon.room_center(&dungeon.rooms[0]);
    let px = cx as f32 * TILE_SIZE;
    let pz = cy as f32 * TILE_SIZE;

    let player = world.spawn();
    world.add_component(player, Transform::from_xyz(px, 1.0, pz));
    // Perspective camera: fov=π/4 (45°), near clip=0.1, far clip=200.
    // The render pipeline uses these to build the projection matrix for the depth/lighting passes.
    world.add_component(
        player,
        Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 200.0),
    );
    // Dynamic rigid body: affected by gravity, responds to collision forces.
    // linear_damping=0 so the player doesn't slow down in mid-air.
    let mut body = RigidBody::new_dynamic();
    body.gravity_scale = 1.0;
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;
    world.add_component(player, body);
    // Capsule collider for first-person character: radius 0.3, height 1.4.
    // Capsules are standard for FPS controllers — they slide smoothly around corners.
    world.add_component(player, Collider::capsule(0.3, 1.4));
    world.add_component(player, PlayerState::new());

    println!("Player spawned at ({}, {})", px, pz);
}

/// Set up the dungeon lighting.
///
/// The render pipeline processes lights in two passes:
/// 1. **Directional light** — uniform parallel rays, used as ambient fill. No shadow map
///    (too expensive for a dim fill light). Simulates faint skylight leaking underground.
/// 2. **Point lights** — one per room, positioned at ceiling height (y=2.8).
///    Each point light contributes to the deferred lighting pass with its color, intensity,
///    and range. The warm orange color (1.0, 0.75, 0.4) simulates torchlight.
///
/// Both light types are ECS entities with Transform + LightComponent — the render graph
/// queries all light entities each frame to build light lists for the PBR shading pass.
fn spawn_lights(world: &mut World, dungeon: &Dungeon) {
    // Dim directional light — acts as ambient fill so rooms aren't pitch black.
    // Direction [0,-1,0] = pointing straight down. Intensity 0.3 = very dim.
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

    // Point lights (torches) — one per room center, near the ceiling.
    // The deferred renderer accumulates these in the lighting pass over the G-buffer.
    for room in &dungeon.rooms {
        let (cx, cy) = dungeon.room_center(room);
        let lx = cx as f32 * TILE_SIZE;
        let lz = cy as f32 * TILE_SIZE;

        let light = world.spawn();
        world.add_component(light, Transform::from_xyz(lx, 2.8, lz));
        world.add_component(
            light,
            PointLight {
                color: [1.0, 0.75, 0.4], // warm torch orange
                intensity: 4.0,
                range: 18.0, // falloff distance in world units
                enabled: true,
            },
        );
    }

    println!(
        "Spawned 1 directional light + {} point lights",
        dungeon.rooms.len()
    );
}

/// Spawn collectible items: treasures (gold cubes) in each room, a key in the last room,
/// and an exit door that requires the key.
///
/// Each collectible is a renderable entity (Transform + PbrMaterial + MeshRenderer) with
/// a Collider for physics-based pickup detection. The `collectible_system` handles the
/// actual pickup logic by reading collision events from PhysicsWorld.
fn spawn_collectibles(world: &mut World, dungeon: &Dungeon) {
    // Treasure in each room except the first (player spawn room).
    // Gold metallic cubes: high metallic=0.8, low roughness=0.2 = shiny gold.
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
        world.add_component(
            e,
            Collectible {
                kind: CollectibleKind::Treasure,
                collected: false,
            },
        );
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
        world.add_component(
            e,
            Collectible {
                kind: CollectibleKind::Key,
                collected: false,
            },
        );
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

/// Spawn enemies in each room (except the player's starting room).
/// Enemies use kinematic rigid bodies — moved by code, not by physics forces.
/// Red PBR material with low metallic/high roughness = matte red (blood/slime look).
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

    println!("Spawned {} enemies", dungeon.rooms.len().saturating_sub(1));
}

// ---------------------------------------------------------------------------
// Systems — functions called each frame by the ECS scheduler
// ---------------------------------------------------------------------------

/// First-person camera controller.
///
/// Reads WASD + mouse input from InputManager, converts to movement velocity
/// applied to the player's RigidBody, and updates the camera rotation.
///
/// **Borrow-safety pattern**: InputManager is read in a short-lived block to
/// release the borrow before mutating PlayerState/RigidBody/Transform below.
fn player_control_system(world: &mut World) {
    // Read input state in a block to release the InputManager borrow before
    // we mutate player components below (ECS borrow rules).
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

/// Enemy AI — finite state machine with Patrol and Chase states.
///
/// Each frame: get player position, compute distance, transition states,
/// then move the enemy's kinematic body toward the current target.
/// Uses hysteresis on state transitions (detect at range, lose at 1.5× range)
/// to prevent rapid toggling at the boundary.
fn enemy_ai_system(world: &mut World) {
    // Get player position for distance checks
    let player_pos = {
        let players = world.component_entities::<PlayerState>();
        players
            .first()
            .and_then(|&eid| world.get_by_index::<Transform>(eid).map(|t| t.position()))
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

/// Pickup system — reads collision events from PhysicsWorld each frame.
///
/// When the player entity collides with a Collectible entity, the item is
/// marked as collected and the player's score/inventory is updated.
/// This is a standard ECS pattern: event-driven responses to physics overlaps.
fn collectible_system(world: &mut World) {
    // Drain collision events from the physics step
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
        let (player_eid, collectible_eid) = if world.get_by_index::<PlayerState>(a).is_some()
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
        let (player_eid, _enemy_eid) = if world.get_by_index::<PlayerState>(a).is_some()
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
                    println!("\n*** LEVEL COMPLETE! Final Score: {} ***\n", player.score);
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
            let pos = transform.position();
            println!(
                "[Status] t={:.1}s | Pos: ({:.1}, {:.1}, {:.1}) | Lives: {} | Score: {} | Key: {}",
                elapsed, pos.x, pos.y, pos.z, player.lives, player.score, player.has_key,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    // Initialize logging — set RUST_LOG=debug for verbose output.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== RustEngine 3D Dungeon Explorer Demo ===\n");

    // ── App setup ──────────────────────────────────────────────────────
    // AppBuilder is the engine entry point. It creates the ECS World,
    // registers plugins, and manages the system execution order.
    let mut builder = AppBuilder::new();
    // CorePlugins: registers core ECS resources (Time, InputManager, etc.)
    builder.add_plugin(CorePlugins);
    // FrameworkPlugin: game-state stack (play → pause → game over transitions)
    builder.add_plugin(FrameworkPlugin);
    // PhysicsPlugin: registers PhysicsWorld resource + physics step system
    builder.add_plugin(PhysicsPlugin);

    // Configure physics for dungeon scale — heavier gravity for snappier movement.
    {
        let pw = builder
            .world_mut()
            .get_resource_mut::<PhysicsWorld>()
            .unwrap();
        pw.gravity = Vec3::new(0.0, -20.0, 0.0);
        pw.set_broadphase_cell_size(4.0);
    }

    // ── Procedural dungeon generation ──────────────────────────────────
    let mut dungeon = Dungeon::new();
    dungeon.generate();

    // ── Entity spawning ────────────────────────────────────────────────
    // Order matters: dungeon geometry first (floors + walls), then entities
    // that live on top of it (player, lights, collectibles, enemies).
    spawn_dungeon(builder.world_mut(), &dungeon);
    spawn_player(builder.world_mut(), &dungeon);
    spawn_lights(builder.world_mut(), &dungeon);
    spawn_collectibles(builder.world_mut(), &dungeon);
    spawn_enemies(builder.world_mut(), &dungeon);

    // ── System registration ────────────────────────────────────────────
    // Systems run in registration order each frame. Input/physics systems
    // should run before gameplay systems that depend on their results.
    builder.add_system(player_control_system); // input → velocity + camera
    builder.add_system(enemy_ai_system); // AI movement
    builder.add_system(collectible_system); // pickup detection
    builder.add_system(enemy_collision_system); // enemy damage
    builder.add_system(exit_door_system); // win condition
    builder.add_system(death_check_system); // lose condition
    builder.add_system(pause_system); // pause toggle
    builder.add_system(status_report_system); // debug output

    let mut app = builder.build();

    let entity_count = app.world.entity_count();
    println!("\nStarting simulation with {} entities...\n", entity_count);

    // ── Simulation loop (terminal-based, no window) ────────────────────
    // This demo runs headless — no GPU window. Each iteration simulates one
    // frame: physics step → system execution → state updates.
    // In a real game, this would be driven by the window event loop.
    for frame in 0..600u32 {
        // Simulate player input programmatically (no real window/input device).
        // Forward for 200 frames, turn right, forward again — explores the dungeon.
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

        // Run all registered systems for this frame.
        // Internally: physics step → system scheduler → resource updates.
        app.run();

        // Print physics stats periodically for debugging
        if frame == 0 || frame == 119 || frame == 299 || frame == 599 {
            let pw = app.world.get_resource::<PhysicsWorld>();
            if let Some(pw) = pw {
                let elapsed = app
                    .world
                    .get_resource::<Time>()
                    .map(|t| t.elapsed_seconds())
                    .unwrap_or(0.0);
                println!(
                    "[Physics] frame={} t={:.1}s | Bodies: {} | Colliders: {} | Collisions: {}",
                    frame,
                    elapsed,
                    pw.body_count,
                    pw.collider_count,
                    pw.collision_events.len()
                );
            }
        }
    }

    println!("\nSimulation complete (600 frames).");
}
