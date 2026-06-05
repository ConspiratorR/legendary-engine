# 用 RustEngine 构建 3D 地牢探索游戏

本教程带你用 RustEngine 从零构建一个完整的 3D 地牢探索游戏。
完成后你将拥有一个可玩的游戏：程序化地牢、PBR 光照、物理碰撞、敌人 AI、收集品和游戏状态管理。

## 前置条件

- Rust 1.95.0+
- RustEngine 已克隆并可编译

## 构建步骤

我们采用增量构建，每一步都产出可运行的程序：

| 步骤 | 内容 | 验证的系统 |
|------|------|-----------|
| 1 | App 搭建 + 摄像机 + 地面 | engine-core, engine-render (Camera) |
| 2 | 程序化地牢生成 | ECS 实体批量生成 |
| 3 | PBR 材质 + 光照 | engine-render (PbrMaterial, PointLight) |
| 4 | 第一人称控制 | engine-input (键盘 + 鼠标) |
| 5 | 物理碰撞 | engine-physics (Collider, RigidBody) |
| 6 | 收集品 | CollisionEvent, trigger 碰撞 |
| 7 | 敌人 AI | ECS 系统, 状态机 |
| 8 | 游戏状态 | engine-framework (StateStack) |

---

## 第一步：App 搭建和摄像机

最小程序需要 `AppBuilder` + `CorePlugins` + 一个摄像机实体。

```rust
use engine_core::app::AppBuilder;
use engine_core::plugins::CorePlugins;
use engine_core::transform::Transform;
use engine_render::camera::Camera;

fn main() {
    let mut builder = AppBuilder::new();
    builder.add_plugin(CorePlugins);

    // 摄像机：第一人称透视投影
    let camera = builder.world_mut().spawn();
    builder.world_mut().add_component(camera, Transform::from_xyz(0.0, 5.0, 10.0));
    builder.world_mut().add_component(camera, Camera::perspective(
        std::f32::consts::FRAC_PI_4, 0.1, 200.0
    ));

    let mut app = builder.build();
    println!("App 搭建完成，摄像机已创建");
}
```

**关键 API：**
- `AppBuilder::new()` — 创建应用构建器，自动插入 `InputManager`
- `CorePlugins` — 注册 `TimePlugin` + `ActionPlugin`
- `Camera::perspective(fov_y, near, far)` — 透视摄像机

---

## 第二步：程序化地牢生成

地牢用二维数组表示，0=空、1=墙、2=地板、3=走廊。

```rust
const DUNGEON_WIDTH: usize = 64;
const DUNGEON_HEIGHT: usize = 64;
const TILE_SIZE: f32 = 2.0;

struct Dungeon {
    tiles: [[u8; DUNGEON_WIDTH]; DUNGEON_HEIGHT],
    rooms: Vec<Room>,
}

struct Room {
    x: usize, y: usize,
    width: usize, height: usize,
}

impl Dungeon {
    fn generate(&mut self) {
        // 1. 放置 6 个不重叠的房间
        // 2. 用 L 形走廊连接相邻房间
        // 3. 在非空瓦片周围填充墙壁
    }

    fn room_center(&self, room: &Room) -> (usize, usize) {
        (room.x + room.width / 2, room.y + room.height / 2)
    }
}
```

**生成实体：**

```rust
fn spawn_dungeon(world: &mut World, dungeon: &Dungeon) {
    for y in 0..DUNGEON_HEIGHT {
        for x in 0..DUNGEON_WIDTH {
            let tile = dungeon.tiles[y][x];
            if tile == 0 { continue; } // TILE_EMPTY

            let px = x as f32 * TILE_SIZE;
            let pz = y as f32 * TILE_SIZE;

            if tile == 2 || tile == 3 {
                // 地板：扁平方块 + 静态碰撞体
                let e = world.spawn();
                world.add_component(e, Transform::from_xyz(px, -0.5, pz)
                    .with_scale(Vec3::new(TILE_SIZE, 0.5, TILE_SIZE)));
                world.add_component(e, RigidBody::new_static());
                world.add_component(e, Collider::cuboid(
                    TILE_SIZE * 0.5, 0.5, TILE_SIZE * 0.5
                ));
            } else if tile == 1 {
                // 墙壁：高方块 + 静态碰撞体
                let e = world.spawn();
                world.add_component(e, Transform::from_xyz(px, 1.5, pz)
                    .with_scale(Vec3::new(TILE_SIZE, 3.0, TILE_SIZE)));
                world.add_component(e, RigidBody::new_static());
                world.add_component(e, Collider::cuboid(
                    TILE_SIZE * 0.5, 1.5, TILE_SIZE * 0.5
                ));
            }
        }
    }
}
```

**关键 API：**
- `World::spawn()` — 创建新实体
- `World::add_component(entity, component)` — 添加组件
- `Transform::from_xyz(x, y, z)` — 位置变换
- `.with_scale(Vec3)` — 链式设置缩放

---

## 第三步：PBR 材质和光照

为墙壁和地板添加 PBR 材质，在每个房间放置点光源（火把）。

```rust
use engine_render::resource::material::PbrMaterial;
use engine_render::mesh_bridge::MeshRenderer;
use engine_render::light::{DirectionalLight, PointLight};

// 地板材质：深灰色，粗糙
world.add_component(e, PbrMaterial::new([0.25, 0.25, 0.3, 1.0], 0.0, 0.9));

// 墙壁材质：棕色，略粗糙
world.add_component(e, PbrMaterial::new([0.45, 0.38, 0.32, 1.0], 0.0, 0.85));

// MeshRenderer：mesh_id=0 使用引擎内置立方体
world.add_component(e, MeshRenderer {
    mesh_id: 0, material_id: 0, cast_shadow: true,
});

// 方向光（环境填充）
world.add_component(sun, DirectionalLight {
    direction: [0.0, -1.0, 0.0],
    color: [0.15, 0.12, 0.1],
    intensity: 0.3,
    enabled: true,
});

// 点光源（火把）
world.add_component(torch, PointLight {
    color: [1.0, 0.75, 0.4],  // 暖橙色
    intensity: 4.0,
    range: 18.0,
    enabled: true,
});
```

**关键 API：**
- `PbrMaterial::new(base_color, metallic, roughness)` — PBR 材质
  - `metallic`: 0.0=非金属, 1.0=金属
  - `roughness`: 0.0=光滑, 1.0=粗糙
- `DirectionalLight` — 方向光（模拟太阳/环境光）
- `PointLight` — 点光源（模拟火把/灯）

---

## 第四步：第一人称摄像机控制

WASD 移动 + 鼠标旋转视角。

```rust
use engine_input::input_manager::InputManager;
use engine_input::keyboard::KeyCode;

const MOUSE_SENSITIVITY: f32 = 0.003;
const MOVE_SPEED: f32 = 8.0;

fn player_control_system(world: &mut World) {
    // 读取输入状态（避免借用冲突）
    let (fwd, back, left, right, mdx, mdy) = {
        let input = world.get_resource::<InputManager>().unwrap();
        (
            input.key_down(KeyCode::KeyW),
            input.key_down(KeyCode::KeyS),
            input.key_down(KeyCode::KeyA),
            input.key_down(KeyCode::KeyD),
            input.mouse().delta.0 as f32,
            input.mouse().delta.1 as f32,
        )
    };

    let entities = world.component_entities::<PlayerState>();
    for &eid in &entities {
        // 鼠标旋转
        if let Some(player) = world.get_by_index_mut::<PlayerState>(eid) {
            player.yaw -= mdx * MOUSE_SENSITIVITY;
            player.pitch = (player.pitch - mdy * MOUSE_SENSITIVITY).clamp(-1.4, 1.4);
        }

        // 计算移动方向
        let yaw = world.get_by_index::<PlayerState>(eid)
            .map(|p| p.yaw).unwrap_or(0.0);
        let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let right_dir = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

        let mut move_dir = Vec3::ZERO;
        if fwd { move_dir += forward; }
        if back { move_dir -= forward; }
        if right { move_dir += right_dir; }
        if left { move_dir -= right_dir; }
        let len = move_dir.length();
        if len > 1e-6 { move_dir = move_dir / len; }

        // 应用速度（保留 Y 轴给重力）
        if let Some(body) = world.get_by_index_mut::<RigidBody>(eid) {
            body.linear_velocity.x = move_dir.x * MOVE_SPEED;
            body.linear_velocity.z = move_dir.z * MOVE_SPEED;
        }

        // 更新摄像机朝向
        if let Some(transform) = world.get_by_index_mut::<Transform>(eid) {
            transform.rotation = Vec3::new(pitch, yaw, 0.0);
        }
    }
}
```

**关键 API：**
- `InputManager::key_down(KeyCode)` — 键是否按下
- `InputManager::mouse()` — 鼠标状态（位置、增量、按钮）
- `RigidBody::linear_velocity` — 线速度（物理引擎控制 Y 轴）

**借用模式：** 先在代码块中读取输入值，再修改组件。这是 ECS 系统的标准模式。

---

## 第五步：物理碰撞

确保玩家不能穿墙、站在地板上。

```rust
use engine_physics::{PhysicsPlugin, PhysicsWorld};
use engine_physics::body::RigidBody;
use engine_physics::collider::Collider;

// 注册物理插件
builder.add_plugin(PhysicsPlugin);

// 配置重力
{
    let pw = builder.world_mut().get_resource_mut::<PhysicsWorld>().unwrap();
    pw.gravity = Vec3::new(0.0, -20.0, 0.0);
}

// 玩家：动态刚体 + 胶囊碰撞体
world.add_component(player, RigidBody::new_dynamic());
world.add_component(player, Collider::capsule(0.3, 1.4));

// 墙壁/地板：静态刚体 + 立方体碰撞体
world.add_component(wall, RigidBody::new_static());
world.add_component(wall, Collider::cuboid(1.0, 1.5, 1.0));
```

**关键 API：**
- `PhysicsPlugin` — 自动注册 `physics_step_system`
- `PhysicsWorld::gravity` — 重力向量
- `RigidBody::new_dynamic()` — 动态刚体（受力和碰撞影响）
- `RigidBody::new_static()` — 静态刚体（不可移动）
- `Collider::capsule(radius, height)` — 胶囊碰撞体（适合角色）
- `Collider::cuboid(half_x, half_y, half_z)` — 立方体碰撞体

**重要：** 地板和墙壁必须有 `RigidBody::new_static()` + `Collider`，否则物理引擎不会检测碰撞。

---

## 第六步：收集品

在房间中放置宝箱和钥匙，玩家靠近时触发拾取。

```rust
#[derive(Debug, Clone)]
struct Collectible {
    kind: CollectibleKind,
    collected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CollectibleKind { Key, Treasure }

fn collectible_system(world: &mut World) {
    // 读取物理碰撞事件
    let pairs: Vec<(u32, u32)> = {
        let pw = world.get_resource::<PhysicsWorld>().unwrap();
        pw.collision_events.iter()
            .filter(|e| e.is_enter)
            .map(|e| (e.entity_a, e.entity_b))
            .collect()
    };

    for (a, b) in pairs {
        // 判断哪个是玩家、哪个是收集品
        let (player_eid, item_eid) = if
            world.get_by_index::<PlayerState>(a).is_some()
            && world.get_by_index::<Collectible>(b).is_some()
        { (a, b) } else if
            world.get_by_index::<PlayerState>(b).is_some()
            && world.get_by_index::<Collectible>(a).is_some()
        { (b, a) } else { continue };

        if let Some(col) = world.get_by_index_mut::<Collectible>(item_eid) {
            if !col.collected {
                col.collected = true;
                // 处理拾取逻辑...
            }
        }
    }
}
```

**关键 API：**
- `PhysicsWorld::collision_events` — 本帧碰撞事件列表
- `CollisionEvent::is_enter` — 是否为新碰撞（vs 持续碰撞）
- `CollisionEvent::entity_a/b` — 碰撞双方的实体索引

---

## 第七步：敌人 AI

巡逻 + 追踪双状态 AI。

```rust
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
enum EnemyState { Patrol, Chase }

fn enemy_ai_system(world: &mut World) {
    // 获取玩家位置
    let player_pos = { /* ... */ };

    let entities = world.component_entities::<EnemyAI>();
    for &eid in &entities {
        let (state, detection_range, ...) = { /* 读取 AI 组件 */ };

        let dist_to_player = (current_pos - player_pos).length();

        // 状态切换
        let new_state = match state {
            EnemyState::Patrol if dist_to_player < detection_range => EnemyState::Chase,
            EnemyState::Chase if dist_to_player > detection_range * 1.5 => EnemyState::Patrol,
            s => s,
        };

        // 根据状态计算目标位置和速度
        let (goal_x, goal_z, move_speed) = match new_state {
            EnemyState::Patrol => (target_x, target_z, speed),
            EnemyState::Chase => (player_pos.x, player_pos.z, chase_speed),
        };

        // 移动（Kinematic 刚体）
        if let Some(body) = world.get_by_index_mut::<RigidBody>(eid) {
            body.linear_velocity = Vec3::new(vx, body.linear_velocity.y, vz);
        }
    }
}
```

**设计要点：**
- `Kinematic` 刚体：不受重力影响，由代码控制移动
- 追踪范围 > 脱离范围：防止状态抖动
- 巡逻路径点：到达后自动切换到下一个

---

## 第八步：游戏状态管理

暂停菜单和游戏结束。

```rust
use engine_framework::{FrameworkPlugin, GameStateAction};

// 注册框架插件
builder.add_plugin(FrameworkPlugin);

fn pause_system(world: &mut World) {
    let should_pause = {
        let input = world.get_resource::<InputManager>().unwrap();
        input.key_just_pressed(KeyCode::Escape)
    };
    if should_pause {
        if let Some(action) = world.get_resource_mut::<GameStateAction>() {
            *action = GameStateAction::PushPause;
        }
    }
}

fn death_check_system(world: &mut World) {
    // 检查玩家生命值
    if lives <= 0 {
        if let Some(action) = world.get_resource_mut::<GameStateAction>() {
            *action = GameStateAction::PushGameOver { score };
        }
    }
}
```

**关键 API：**
- `FrameworkPlugin` — 注册 `StateStack`，管理游戏状态生命周期
- `GameStateAction::PushPause` — 暂停
- `GameStateAction::PushGameOver { score }` — 游戏结束（显示分数）
- `GameStateAction::Pop` — 恢复上一个状态

---

## 完整代码

见 `crates/engine-core/examples/dungeon_demo.rs`。

运行：
```bash
cargo run --example dungeon_demo -p engine-core
```

## 总结

本教程验证了 RustEngine 的以下子系统：

| 子系统 | 验证内容 |
|--------|---------|
| engine-core | AppBuilder, Plugin, Time, Transform |
| engine-ecs | World, 组件, 资源, 系统 |
| engine-render | Camera, PbrMaterial, MeshRenderer, DirectionalLight, PointLight |
| engine-physics | PhysicsWorld, RigidBody, Collider, CollisionEvent |
| engine-input | InputManager, KeyCode, MouseState |
| engine-framework | FrameworkPlugin, GameStateAction, StateStack |

通过构建真实游戏暴露了 API 问题（TimePlugin hook 修复），验证了各子系统协同工作。
