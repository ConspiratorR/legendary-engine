# 纹理桥接层设计：资产系统 → 渲染管线集成

**日期**: 2026-05-27
**状态**: 待审核

## 目标

将 `engine-asset` 的 `Handle<Texture>` 与 `engine-render` 的 `TextureStore`（u64 id）打通，实现异步纹理加载、GPU 上传、加载完成事件通知。

## 约束

- Rust 2024 edition, toolchain 1.95.0
- wgpu 渲染后端
- 不引入额外异步运行时（用 `std::thread` + crossbeam channel）
- 不改动现有 `TextureStore` 内部实现，通过桥接层复用

## 架构概览

```
游戏代码                     后台线程                    渲染线程
─────────                   ─────────                  ─────────
asset.load("player.png")
  → Handle<Texture>
       │
       ▼
TextureBridge.request(handle)
  → 加入加载队列 ──────→  读取文件
                          解码图片 (image crate)
                          生成 DecodedTexture
                               │
                               ▼
                        写入 completed_queue ──────→ TextureBridge.flush()
                                                      → TextureStore::load_from_bytes()
                                                      → 记录 handle → u64 映射
                                                      → 发送 TextureLoaded 事件
                                                      → Sprite 可用

渲染时：
TextureBridge.resolve(handle)
  → 有映射？返回 u64
  → 无映射？返回 fallback_id (0)
```

## 组件设计

### 1. TextureBridge

位于 `engine-render/src/texture_bridge.rs`。

```rust
pub struct TextureBridge {
    handle_to_id: HashMap<HandleId, u64>,
    states: HashMap<HandleId, LoadState>,
    completed_queue: Receiver<DecodedTexture>,
    load_sender: Sender<LoadRequest>,
    pub on_loaded: EventChannel<TextureLoaded>,
    texture_store: TextureStore,
}

pub enum LoadState {
    Pending,
    Ready(u64),
    Failed(String),
}

struct LoadRequest {
    handle_id: HandleId,
    path: String,
}

struct DecodedTexture {
    handle_id: HandleId,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

pub struct TextureLoaded {
    pub handle_id: HandleId,
    pub texture_id: u64,
}
```

**HandleId 定义：**

```rust
/// Handle<T> 内部 Arc 的指针地址，作为唯一标识。
/// 同一个 Handle 的所有 clone 共享同一个 Arc，因此 ID 相同。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandleId(usize);

impl HandleId {
    pub fn from_handle<T: Asset>(handle: &Handle<T>) -> Self {
        Self(Arc::as_ptr(&handle.inner) as *const () as usize)
    }
}
```

需要在 `engine-asset/src/asset.rs` 中将 `inner` 字段改为 `pub(crate)` 可见性。

**TextureStore 所有权：**

`TextureBridge` 拥有 `TextureStore`，`Renderer` 不再直接持有。`Renderer` 通过 `bridge.texture_store()` 访问。

```rust
impl TextureBridge {
    pub fn texture_store(&self) -> &TextureStore { &self.texture_store }
    pub fn texture_store_mut(&mut self) -> &mut TextureStore { &mut self.texture_store }
}
```

**关键接口：**

- `new(device, queue, texture_layout)` — 创建桥接层，启动后台加载线程，初始化 TextureStore
- `request(handle, path)` — 提交异步加载请求
- `flush(device, queue, layout)` — 每帧调用，上传已完成的纹理到 GPU，触发事件
- `resolve(handle) -> u64` — 查询 texture_id，未就绪返回 fallback_id
- `state(handle) -> &LoadState` — 查询加载状态

**后台加载线程：**

```rust
std::thread::spawn(move || {
    for req in load_rx {
        match std::fs::read(&req.path)
            .and_then(|bytes| image::load_from_memory(&bytes).map_err(|e| std::io::Error::other(e)))
        {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let _ = done_tx.send(DecodedTexture {
                    handle_id: req.handle_id,
                    pixels: rgba.into_raw(),
                    width: w,
                    height: h,
                });
            }
            Err(e) => {
                // 失败时发送带错误信息的完成信号
            }
        }
    }
});
```

**flush 流程：**

```rust
pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) {
    while let Ok(decoded) = self.completed_queue.try_recv() {
        match self.texture_store.load_from_bytes(device, queue, layout,
            &decoded.pixels, decoded.width, decoded.height)
        {
            Ok(texture_id) => {
                self.handle_to_id.insert(decoded.handle_id, texture_id);
                self.states.insert(decoded.handle_id, LoadState::Ready(texture_id));
                self.on_loaded.emit(&TextureLoaded {
                    handle_id: decoded.handle_id,
                    texture_id,
                });
            }
            Err(e) => {
                self.states.insert(decoded.handle_id, LoadState::Failed(e.to_string()));
            }
        }
    }
}
```

### 2. EventChannel<T>

位于 `engine-core/src/event.rs`。通用发布/订阅事件系统，可复用于物理碰撞、输入事件等。

```rust
pub struct EventChannel<T: Send + 'static> {
    listeners: Vec<Box<dyn Fn(&T) + Send + Sync>>,
}

pub struct ListenerId(usize);

impl<T: Send + 'static> EventChannel<T> {
    pub fn new() -> Self;
    pub fn subscribe(&mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> ListenerId;
    pub fn unsubscribe(&mut self, id: ListenerId);
    pub fn emit(&self, event: &T);
}
```

- 同步调用，`emit()` 时立即触发所有监听器
- `Fn(&T)` 而非 `FnMut`，多监听器并行安全

### 3. Sprite 组件改造

```rust
// engine-render/src/sprite.rs

pub struct Sprite {
    pub texture: Handle<Texture>,   // 改：u64 → Handle<Texture>
    pub color: [f32; 4],
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}
```

`SpriteDraw`（渲染内部中间表示）保持 `texture_id: u64`，在渲染前由 `bridge.resolve()` 转换。

### 4. Renderer 集成

`Renderer` 不再直接持有 `TextureStore`，改为通过 `TextureBridge` 访问。

```rust
// Renderer 字段变更
pub struct Renderer {
    // 移除: texture_store: TextureStore,
    // 其余字段不变
}

pub fn render_frame(
    &mut self,
    cameras: &[Camera],
    sprites: &[Sprite],           // 改：接收 Sprite 而非 SpriteDraw
    bridge: &mut TextureBridge,   // 新：传入 bridge
) {
    // flush 新纹理
    bridge.flush(&self.device, &self.queue, &self.sprite_pipeline.texture_bind_group_layout);

    // 1. Sprite → SpriteDraw 转换
    let sprite_draws: Vec<SpriteDraw> = sprites.iter().map(|s| {
        SpriteDraw {
            world_matrix: /* 从 Transform 计算 */,
            color: s.color,
            size: s.size,
            texture_id: bridge.resolve(&s.texture),  // Handle → u64
            flip_x: s.flip_x,
            flip_y: s.flip_y,
        }
    }).collect();

    // 2. 后续不变：裁剪、批处理、上传、绘制
    // 绘制时通过 bridge.texture_store().get_bind_group(id) 获取绑定组
}
```

## 每帧调用顺序

```
1. TextureBridge::flush()   — 上传新纹理到 GPU，触发 TextureLoaded 事件
2. 游戏逻辑                 — 处理事件，更新 Sprite 组件
3. Renderer::render_frame() — resolve(handle) 获取 texture_id，渲染
```

## 依赖

- `crossbeam-channel` — 线程间通信（Cargo.toml 新增依赖）
- `image` crate — 已在 engine-asset 中使用，engine-render 也需引入
- `engine-core::event` — 新模块，EventChannel<T>

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `engine-asset/src/asset.rs` | 修改 | `Handle.inner` 改为 `pub(crate)`，新增 `HandleId` |
| `engine-core/src/event.rs` | 新增 | EventChannel<T> 通用事件系统 |
| `engine-core/src/lib.rs` | 修改 | 导出 event 模块 |
| `engine-render/src/texture_bridge.rs` | 新增 | TextureBridge 桥接层 + HandleId 定义 |
| `engine-render/src/lib.rs` | 修改 | 导出 texture_bridge 模块 |
| `engine-render/src/sprite.rs` | 修改 | Sprite.texture: u64 → Handle<Texture> |
| `engine-render/src/renderer.rs` | 修改 | 移除 texture_store 字段，render_frame 接入 bridge |
| `engine-render/Cargo.toml` | 修改 | 新增 crossbeam-channel、image 依赖 |
| `engine-core/examples/sprite_demo.rs` | 修改 | 适配新 API |

## 不在范围内

- mipmap 生成
- 纹理图集 / sprite sheet 支持
- 纹理压缩格式（ASTC、BC7 等）
- 后台线程池（当前单线程足够）
- ECS 系统自动注册 Sprite 组件
