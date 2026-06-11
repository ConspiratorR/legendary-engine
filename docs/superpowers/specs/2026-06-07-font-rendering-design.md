# 字体渲染系统设计

**日期**: 2026-06-07
**状态**: 已批准
**模块**: engine-render

## 概述

为 RustEngine 添加字体渲染能力：加载 TTF/OTF 字体，光栅化字形，上传到 GPU 纹理图集，通过现有 Sprite 管线绘制文字。支持 Unicode BMP（包括 CJK 字符）。

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 光栅化库 | fontdue | API 简洁，性能好，纯 Rust |
| 字形存储 | 动态纹理图集（shelf packing） | GPU 友好，内存效率高 |
| 绘制方式 | 复用 SpriteBatch | 无新 shader，实现最快 |
| 模块归属 | engine-render | 需直接操作 wgpu 纹理 |
| CJK 支持 | 支持 Unicode BMP | 按需光栅化，不预渲染全集 |

## 架构

```
TTF/OTF 文件
    │
    ▼
FontLoader (fontdue)
    │ 加载字体数据，光栅化字形
    ▼
GlyphAtlas
    │ 管理动态纹理图集，shelf packing 字形位图
    │ 多张 1024×1024 RGBA8 纹理
    ▼
TextPainter
    │ 将文字转换为 SpriteDraw[]
    │ 每个字形 = 1 个 Sprite（图集纹理 + UV 偏移）
    ▼
SpriteBatch (已有)
    │ 提交到现有渲染管线
    ▼
屏幕上的文字
```

## 新增文件

```
crates/engine-render/src/font/
├── mod.rs          # 模块入口，pub use
├── error.rs        # FontError 类型
├── loader.rs       # FontLoader - fontdue 封装
├── atlas.rs        # GlyphAtlas - 动态纹理图集
├── painter.rs      # TextPainter - 文字 → SpriteDraw
└── atlas_adapter.rs # FontAtlasAdapter - 适配 engine-ui::text::FontAtlas trait
```

## 模块详细设计

### 1. FontLoader（font/loader.rs）

封装 fontdue 库，负责字体加载和字形光栅化。

```rust
pub struct FontLoader {
    fonts: HashMap<String, fontdue::Font>,
}

pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub pixels: Vec<u8>,  // RGBA8
}

impl FontLoader {
    pub fn new() -> Self;
    pub fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), FontError>;
    pub fn rasterize(&self, font_name: &str, ch: char, size: f32) -> Result<GlyphBitmap, FontError>;
    pub fn has_glyph(&self, font_name: &str, ch: char) -> bool;
    pub fn metrics(&self, font_name: &str, ch: char, size: f32) -> GlyphMetrics;
}
```

fontdue 输出 coverage 值（u8），转换为 RGBA8（白色字形 + alpha 作为 coverage）。

### 2. GlyphAtlas（font/atlas.rs）

管理字形在 GPU 纹理上的打包。使用 shelf packing 算法。

```rust
pub struct GlyphKey {
    pub font_hash: u64,
    pub ch: char,
    pub size: u32,
}

pub struct GlyphEntry {
    pub atlas_index: u32,
    pub uv: [f32; 4],       // [u0, v0, u1, v1]
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

pub struct GlyphAtlas {
    textures: Vec<wgpu::Texture>,
    views: Vec<wgpu::TextureView>,
    bind_groups: Vec<wgpu::BindGroup>,
    packers: Vec<ShelfPacker>,
    cache: HashMap<GlyphKey, GlyphEntry>,
    texture_layout: wgpu::BindGroupLayout,
}

impl GlyphAtlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, texture_layout: wgpu::BindGroupLayout) -> Self;
    
    /// 获取或光栅化字形，返回 GlyphEntry
    pub fn get_or_rasterize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        loader: &FontLoader,
        font_name: &str,
        ch: char,
        size: f32,
    ) -> Result<&GlyphEntry, FontError>;
    
    /// 获取字形的纹理 ID（用于 SpriteDraw.texture_id）
    pub fn texture_id(&self, entry: &GlyphEntry) -> u64;
    
    /// 获取字形的 bind group
    pub fn bind_group(&self, entry: &GlyphEntry) -> &wgpu::BindGroup;
}
```

**Shelf Packing**：每张纹理按行分配字形，行高 = 当前最大字形高度。空间不足时新增纹理。

**纹理规格**：1024×1024 RGBA8，按需扩展。CJK 全集约 20000 字形 × 32×32 ≈ 40MB（约 40 张纹理），实际按需加载。

### 3. TextPainter（font/painter.rs）

将文字转换为 SpriteDraw 数组。

```rust
pub struct TextPainter {
    loader: FontLoader,
    atlas: GlyphAtlas,
}

impl TextPainter {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, texture_layout: wgpu::BindGroupLayout) -> Self;
    pub fn load_font(&mut self, name: &str, data: &[u8]) -> Result<(), FontError>;
    
    /// 将文字转为 SpriteDraw 列表
    pub fn draw_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        font_name: &str,
        size: f32,
        color: [f32; 4],
        position: [f32; 2],
    ) -> Vec<SpriteDraw>;
    
    pub fn loader(&self) -> &FontLoader;
    pub fn atlas(&self) -> &GlyphAtlas;
}
```

**draw_text 流程**：
1. 遍历 text 的每个字符
2. 从 atlas 查缓存，miss 时调用 loader.rasterize() + atlas.pack()
3. 为每个字形创建 SpriteDraw（texture_id = atlas 纹理 ID，uv = atlas UV）
4. 返回 SpriteDraw 数组

### 4. FontAtlasAdapter（font/atlas_adapter.rs）

适配 `engine-ui::text::FontAtlas` trait，让 TextRenderer 能使用 GlyphAtlas。

```rust
pub struct FontAtlasAdapter {
    painter: TextPainter,
}

impl engine_ui::text::FontAtlas for FontAtlasAdapter {
    fn rasterize(&self, family: &FontFamily, ch: char, font_size: f32) -> Result<RasterizedGlyph, TextError>;
    fn advance_width(&self, family: &FontFamily, ch: char, font_size: f32) -> f32;
    fn line_height(&self, family: &FontFamily, font_size: f32) -> f32;
    fn has_glyph(&self, family: &FontFamily, ch: char) -> bool;
}
```

### 5. FontError（font/error.rs）

```rust
#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("font not found: {0}")]
    FontNotFound(String),
    #[error("fontdue error: {0}")]
    Fontdue(String),
    #[error("atlas full: no space for glyph '{0}'")]
    AtlasFull(char),
    #[error("invalid font data")]
    InvalidData,
}
```

## 集成方式

### RenderPlugin2D 集成

在 `plugin.rs` 的 `build()` 中创建 TextPainter 并插入 ECS 资源：

```rust
let text_painter = TextPainter::new(&renderer.device, &renderer.queue, texture_layout.clone());
world.insert_resource(text_painter);
```

### 用户代码使用方式

```rust
// 在 game loop 中
let painter = world.get_resource_mut::<TextPainter>().unwrap();
let sprites = painter.draw_text(
    device, queue,
    "Score: 1000",
    "default",
    24.0,
    [1.0, 1.0, 1.0, 1.0],
    [10.0, 10.0],
);
// sprites 提交到 SpriteBatch 渲染
```

### 与 TextRenderer 协作

```
用户代码 → TextRenderer.layout() → TextLayout → TextPainter.paint() → SpriteDraw[]
```

TextRenderer 负责文字布局（换行、对齐），TextPainter 负责将布局结果转为 GPU 可渲染的 Sprite。

## 依赖

在 `engine-render/Cargo.toml` 添加：

```toml
fontdue = "0.9"
```

## 测试策略

1. **FontLoader 单元测试**：加载字体文件，光栅化 ASCII/CJK 字形，验证尺寸和像素
2. **GlyphAtlas 单元测试**：打包字形，验证 UV 坐标不重叠，验证多纹理扩展
3. **TextPainter 集成测试**：draw_text 输出 SpriteDraw 数量 = 字符数，验证位置计算
4. **渲染验证**：运行 tetris 示例，用 TextPainter 绘制分数文字

## 性能考量

- 字形缓存：GlyphAtlas 内置 HashMap 缓存，相同 (font, char, size) 只光栅化一次
- 纹理上传：只在新增字形时上传，不每帧上传
- Sprite 数量：每个字符 1 个 Sprite，100 个字符 = 100 个 Sprite，对 SpriteBatch 来说很小
- 内存：CJK 按需加载，不预渲染全集

## 后续优化（不在本次范围）

- 文字 Sprite 缓存（整段文字缓存为一张纹理）
- SDF 字形（支持无限缩放）
- GPU instancing 批量绘制
