# 阶段 1 渲染管线完成设计

日期：2026-05-28
状态：待审核
范围：完成路线图阶段 1 剩余工作

## 背景

RustEngine 阶段 1（渲染管线）已有大量实现。代码分析发现，路线图中部分标记为"待做"的功能实际已完成或接近完成：

| 路线图标记 | 实际状态 | 说明 |
|-----------|---------|------|
| 摄像机系统 🔨 | ✅ 已完成 | 视锥裁剪（`renderer.rs:167-175`）、多摄像机、视口/裁剪区域均已实现 |
| Sprite 批量绘制 ⏳ | ✅ 已完成 | 实例化渲染 + 间接绘制（`SpriteRenderer` + `PersistentBuffer`）已实现 |
| 纹理加载→Sprite ⏳ | 🔨 部分完成 | `TextureBridge` + `TextureStore` 管线存在，但需手动桥接 |

**本设计覆盖的工作：**

- 基础层：4 项架构改进
- 功能层：3 项新功能（精灵动画、粒子系统、Tilemap）

## 实现策略

**先基础后功能** — 先修复架构问题，再在稳固基础上构建新功能。

---

## 基础层

### 1. 资产-渲染自动桥接

**问题：** `Registry`（engine-asset）和 `TextureBridge`（engine-render）完全独立。调用方需手动 `bridge.request(handle, path)` 触发 GPU 上传。

**方案：**

- `Texture` 结构体（`engine-asset/src/types.rs`）新增 `asset_path: String` 字段
- `TextureBridge` 新增 `auto_sync(&mut self, registry: &Registry)` 方法
- `auto_sync` 遍历 Registry 中所有 `Handle<Texture>`，检查 `state(handle)` 是否为 `None`，对未请求的 handle 调用 `request()`
- 调用时机：`Renderer::render_frame()` 开头，`bridge.flush()` 之前

**变更文件：**
- `crates/engine-asset/src/types.rs` — Texture 添加 asset_path
- `crates/engine-render/src/texture_bridge.rs` — 新增 auto_sync 方法
- `crates/engine-render/src/renderer.rs` — render_frame 调用 auto_sync

### 2. BindGroupLayout 统一

**问题：** `TextureBridge::new()`（texture_bridge.rs:94-114）和 `SpritePipeline::new()`（pipeline/sprite.rs:52-73）各自独立创建相同的纹理 BindGroupLayout。

**方案：**

- `SpritePipeline` 拥有纹理 BindGroupLayout，暴露 `texture_layout()` 方法
- `TextureBridge::new()` 接收 `&BindGroupLayout` 参数，不再自行创建
- 删除 `TextureBridge` 中重复的 layout 创建代码

**变更文件：**
- `crates/engine-render/src/pipeline/sprite.rs` — 暴露 texture_layout()
- `crates/engine-render/src/texture_bridge.rs` — 接收外部 layout
- `crates/engine-render/src/renderer.rs` — 创建顺序：先 SpritePipeline，再传 layout 给 TextureBridge

### 3. 深度排序

**问题：** 透明精灵按纹理 ID 排序分批，不保证正确的 alpha 混合顺序。

**方案：**

- `SpriteDraw` 新增 `depth: f32` 字段
- 从 `Sprite.transform` 的 `translation.z` 提取深度值
- `collect_batches()` 中先按 depth 从后到前稳定排序，再按 texture_id 分组

**变更文件：**
- `crates/engine-render/src/sprite.rs` — SpriteDraw 添加 depth，collect_batches 排序逻辑
- `crates/engine-render/src/renderer.rs` — Sprite→SpriteDraw 转换时填充 depth

### 4. 清理孤立代码

- 删除 `crates/engine-render/src/resource/texture.rs`（与 TextureStore 功能重复）
- 清理 `crates/engine-render/src/resource/material.rs`（仅存 `Option<BindGroup>` 占位）
- 删除 `SpriteBatch::upload()` 方法（sprite.rs:87-105，死代码）

---

## 功能层

### 5. 2D 精灵动画

**依赖：** 基础层完成

**数据结构：**

```rust
// 精灵表：一张大图划分成均匀帧
struct SpriteSheet {
    texture: Handle<Texture>,
    frame_width: u32,
    frame_height: u32,
    columns: u32,
    rows: u32,
}

// 帧序列
struct FrameSequence {
    frames: Vec<usize>,       // 帧索引列表
    fps: f32,                 // 播放速度
    mode: PlaybackMode,       // Loop / Once / PingPong
}

enum PlaybackMode {
    Loop,       // 循环播放
    Once,       // 播完停在最后一帧
    PingPong,   // 来回播放
}

// 动画状态（ECS 组件）
struct SpriteAnimation {
    sheet: Handle<SpriteSheet>,
    sequence: String,         // 当前序列名
    current_frame: usize,
    elapsed: f32,
    playing: bool,
}
```

**UV 计算：** 根据帧索引从精灵表计算 UV 坐标：`column = index % columns`，`row = index / columns`，`u_min = column * frame_width / texture_width`，`v_min = row * frame_height / texture_height`，`u_max = (column + 1) * frame_width / texture_width`，`v_max = (row + 1) * frame_height / texture_height`。在 `SpriteBatch::push()` 中设置 UV 范围。

**系统集成：** ECS 系统每帧更新 `SpriteAnimation`，修改关联 `Sprite` 的 UV 区域。

**变更文件：**
- `crates/engine-render/src/animation.rs` — 新文件，SpriteSheet/FrameSequence/SpriteAnimation
- `crates/engine-render/src/sprite.rs` — SpriteBatch UV 支持精灵表区域
- `crates/engine-render/src/lib.rs` — 导出动画模块

### 6. 2D 粒子系统

**依赖：** 基础层完成

**双模式架构：**

```rust
enum ParticleBackend {
    Cpu,        // CPU 更新，适合 <1000 粒子
    Gpu,        // GPU Compute Shader，适合 >1000 粒子
}
```

**数据结构：**

```rust
struct ParticleEmitter {
    backend: ParticleBackend,
    rate: f32,                    // 每秒发射数量
    burst: Option<u32>,           // 一次性爆发数量
    max_particles: u32,
    lifetime: Range<f32>,
    speed: Range<f32>,
    angle: Range<f32>,
    size: Range<f32>,
    color: Range<Color>,
    size_curve: Option<Curve<f32>>,
    color_curve: Option<Curve<Color>>,
    opacity_curve: Option<Curve<f32>>,
    texture: Handle<Texture>,
    active: bool,
    spawn_accumulator: f32,
}

struct Particle {
    position: Vec2,
    velocity: Vec2,
    lifetime: f32,
    age: f32,
    size: f32,
    color: Color,
}

struct Curve<T> {
    points: Vec<(f32, T)>,  // (0.0~1.0, value) 分段线性插值
}

struct ParticleSystem {
    emitters: HashMap<Entity, EmitterState>,
}
```

**实现顺序：**
1. CPU 模式：每帧 Rust 中更新 `Vec<Particle>`，生成 `SpriteDraw` 注入渲染管线
2. GPU 模式：wgpu Compute Shader + Storage Buffer，零 CPU 回读

**变更文件：**
- `crates/engine-render/src/particle.rs` — 新文件，ParticleEmitter/Particle/ParticleSystem/Curve
- `crates/engine-render/src/particle_gpu.rs` — 新文件，GPU compute pipeline（阶段 2）
- `crates/engine-render/src/renderer.rs` — 集成粒子 SpriteDraw 注入

### 7. Tilemap 支持

**依赖：** 基础层完成

**分 5 个阶段：**

#### Phase A — 基础瓦片渲染

```rust
struct Tileset {
    texture: Handle<Texture>,
    tile_width: u32,
    tile_height: u32,
    columns: u32,
    tile_count: u32,
    collision: Vec<TileCollision>,
}

struct TileLayer {
    tileset: Handle<Tileset>,
    width: u32,
    height: u32,
    tiles: Vec<u32>,          // 瓦片索引，0 = 空
    tile_size: Vec2,
    z_order: i32,
    offset: Vec2,             // 视差偏移
}

struct Tilemap {
    layers: Vec<Entity>,      // TileLayer 实体列表
}
```

渲染：将可见瓦片（视锥裁剪后）转换为 `SpriteDraw` 列表注入渲染管线。

#### Phase B — 碰撞系统

```rust
enum TileCollision {
    None,
    Solid,
    Slope { left_y: f32, right_y: f32 },
    OneWay(Direction),
    Custom(Shape),
}
```

物理系统读取 `TileLayer.tiles` + `Tileset.collision` 生成碰撞体。

#### Phase C — 自动贴图规则

```rust
struct AutotileRule {
    bitmask: u16,              // 8 邻域位掩码
    tile_index: u32,
}

struct Autotile {
    rules: Vec<AutotileRule>,
    fallback: u32,
}
```

根据相邻瓦片自动选择正确的贴图变体（如草地边缘、道路连接）。

#### Phase D — 动态修改 + 图层深度排序

```rust
impl TileLayer {
    fn set_tile(&mut self, x: u32, y: u32, tile: u32);
    fn get_tile(&self, x: u32, y: u32) -> u32;
    fn recalculate_autotile(&mut self, x: u32, y: u32);
}
```

运行时修改瓦片，触发邻域自动贴图重算。图层按 `z_order` 排序渲染。

#### Phase E — 瓦片传播

```rust
struct TilePropagation {
    propagate_to: Vec<u32>,    // 可传播到的瓦片类型
    speed: f32,
    max_distance: u32,
}
```

瓦片间的逻辑连接（水流、电路等）。系统每 tick 更新传播状态。

**变更文件：**
- `crates/engine-render/src/tilemap.rs` — 新文件，Tileset/TileLayer/Tilemap
- `crates/engine-render/src/tileset.rs` — 新文件，Autotile/TileCollision/TilePropagation
- `crates/engine-render/src/renderer.rs` — 集成瓦片渲染

---

## 实现顺序

按依赖关系，推荐实现顺序：

```
基础层（按序）：
  1. BindGroupLayout 统一
  2. 资产-渲染自动桥接
  3. 深度排序
  4. 清理孤立代码

功能层（按序，每项独立 spec → plan → 实现）：
  5. 2D 精灵动画
  6. 2D 粒子系统（先 CPU 模式）
  7. Tilemap Phase A（基础渲染）
  8. Tilemap Phase B-E（按需）
  9. 粒子系统 GPU 模式（可选优化）
```

## 范围排除

以下不在本设计范围内：
- Render Graph 集成（当前 sprite 渲染直接编码，Render Graph 独立存在，未来可统一）
- 3D 渲染管线（阶段 3）
- 物理引擎完善（阶段 4，仅 Tilemap 碰撞涉及）
- 编辑器集成（阶段 7）
