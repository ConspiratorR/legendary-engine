# Sprite 批量绘制性能优化设计

**日期**: 2026-05-27  
**状态**: 已批准  
**目标**: 大规模精灵渲染（上万精灵）性能优化

## 背景

当前 `SpriteBatch` 实现每帧调用 `create_buffer_init` 创建新的顶点/索引缓冲，没有复用 GPU 内存。对于上万精灵场景，这会导致：

- 频繁的 GPU 内存分配
- 不必要的内存拷贝
- Draw call 数量未优化

## 设计目标

1. **零拷贝上传** — 持久映射缓冲，CPU 直接写入 GPU 内存
2. **间接绘制** — 减少 draw call 开销
3. **双缓冲** — 避免 CPU/GPU 竞争
4. **自动扩容** — 动态适应精灵数量变化

## 架构设计

### 新增结构体

```rust
// crates/engine-render/src/sprite_renderer.rs

pub struct PersistentBuffer {
    buffer: wgpu::Buffer,
    size: usize,
    mapped_ptr: *mut u8,
    frame_offset: [usize; 2],  // 双缓冲当前偏移
}

pub struct SpriteRenderer {
    instance_buffers: [PersistentBuffer; 2],  // 双缓冲实例缓冲
    indirect_buffer: wgpu::Buffer,            // 间接绘制命令缓冲
    current_frame: usize,                      // 当前帧索引 (0 or 1)
    sprite_capacity: usize,                    // 精灵容量上限
    pipeline: Arc<SpritePipeline>,            // 复用现有管线
}
```

### 修改现有结构

```rust
// crates/engine-render/src/sprite.rs

pub struct SpriteBatch {
    // 现有字段保留
    pub texture_id: u64,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
    
    // 新增字段
    pub instance_data: Vec<Mat4>,                      // 实例变换矩阵
    pub indirect_cmd: DrawIndexedIndirectArgs,         // 间接绘制命令
}
```

## 数据流

### 每帧执行流程

```
1. 收集阶段
   all_sprites → collect_batches() → Vec<SpriteBatch>
   每个 SpriteBatch.push() 收集 Mat4 到 instance_data

2. 剔除阶段 (已有)
   Frustum::test_aabb() 过滤不可见精灵

3. 上传阶段 (新)
   SpriteRenderer.begin_frame()
   ├── 切换到当前帧的持久缓冲
   └── 重置偏移量

   for each batch:
   ├── 写入顶点数据到持久缓冲 (顶点区域)
   ├── 写入实例数据到持久缓冲 (实例区域)
   ├── 构建 DrawIndexedIndirect 命令
   └── 写入间接缓冲

   SpriteRenderer.end_frame()
   └── fence 同步 (可选，依赖 wgpu 内部同步)

4. 绘制阶段 (新)
   for each batch:
   ├── pass.set_vertex_buffer(0, 顶点缓冲)
   ├── pass.set_vertex_buffer(1, 实例缓冲)
   └── pass.draw_indexed_indirect(间接缓冲, offset)
```

### 内存布局（单个持久缓冲）

```
[顶点区域: N bytes] [实例区域: M bytes] [填充/对齐]
```

- 顶点区域：存放所有批次的 SpriteVertex 数据
- 实例区域：存放所有批次的 Mat4 实例数据
- 每帧从头开始写入（覆盖上一帧数据）

## 同步机制

### 双缓冲策略

```
帧 0: CPU 写入 buffer[0], GPU 读取 buffer[1]
帧 1: CPU 写入 buffer[1], GPU 读取 buffer[0]
帧 2: CPU 写入 buffer[0], GPU 读取 buffer[1]
...
```

### 同步点

```rust
SpriteRenderer::begin_frame()
├── current_frame = 1 - current_frame  // 切换缓冲
├── 等待 GPU 完成上上帧 (如果需要)
│   └── 使用 wgpu::Maintain::Wait 或 fence
└── 重置当前帧偏移量

SpriteRenderer::end_frame()
└── 无需显式同步，wgpu 内部处理
```

### 容量管理

```rust
初始化时：
├── 计算 sprite_capacity = 10000 (可配置)
├── 分配 buffer_size = sprite_capacity * (vertex_size + instance_size)
└── 创建持久映射缓冲

运行时：
├── 如果当前帧精灵数 > sprite_capacity
│   ├── 重新分配更大缓冲 (2x)
│   └── 重建持久映射
└── 否则正常写入
```

## 错误处理

- **缓冲分配失败** → 回退到常规 `write_buffer` 上传
- **容量超限** → 自动扩容或警告
- **同步超时** → 跳过帧，记录警告

## 实现计划

### 新增文件

```
crates/engine-render/src/
├── sprite_renderer.rs    // SpriteRenderer + PersistentBuffer
└── indirect.rs           // DrawIndexedIndirect 相关类型
```

### 修改文件

```
crates/engine-render/src/
├── sprite.rs             // SpriteBatch 增加 instance_data
├── renderer.rs           // 集成 SpriteRenderer
├── pipeline/sprite.rs    // 修改顶点布局，支持实例化
└── pipeline/sprite.wgsl  // 修改着色器，读取实例矩阵
```

### 着色器修改

```wgsl
// pipeline/sprite.wgsl

// 顶点输入增加实例矩阵
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    // ... 使用 model 矩阵变换
}
```

## 测试策略

- **单元测试**：`PersistentBuffer` 写入/读取
- **集成测试**：10000 精灵渲染帧率
- **边界测试**：容量超限、空批次、单精灵
- **性能基准**：对比优化前后 draw call 数量和帧时间

## 依赖

- wgpu 23（已有）
- bytemuck 1（已有）

## 风险

1. **持久映射缓冲支持** — 某些后端可能不支持，需要回退方案
2. **内存对齐** — 需要正确处理 GPU 缓冲对齐要求
3. **同步复杂性** — 双缓冲需要正确实现，避免竞争条件

## 成功标准

- 同屏 10000 精灵稳定 60fps
- Draw call 数量减少（按纹理批次分组）
- CPU 端内存拷贝减少 90%+
