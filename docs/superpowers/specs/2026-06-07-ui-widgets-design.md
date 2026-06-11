# UI 控件系统设计

**日期**: 2026-06-07
**状态**: 已批准
**模块**: engine-ui

## 概述

为 RustEngine 添加 Immediate Mode UI 控件系统。支持 Label、Button、Checkbox、Slider、TextInput、Panel。使用 TextPainter 画文字 + ShapePainter 画背景/边框，纯引擎内部实现。

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| UI 范式 | Immediate Mode + 内部缓存 | API 简洁，适合游戏 HUD |
| 渲染方式 | TextPainter + ShapePainter | 复用已有子系统 |
| 模块归属 | engine-ui | UI 相关逻辑集中 |
| 状态管理 | 内部 hover/pressed/focus 缓存 | 跨帧持久化交互状态 |

## 架构

```
用户代码
    │ UiContext::new(text_painter, shape_painter, input, device, queue)
    │ ui.label("Score: 1000")
    │ if ui.button("Play") { ... }
    ▼
Widget 内部
    │ 计算布局（位置/大小）
    │ ShapePainter 画背景/边框
    │ TextPainter 画文字
    │ 处理输入（hover/click/focus）
    ▼
屏幕上的 UI
```

## 新增文件

```
crates/engine-ui/src/widgets/
├── mod.rs          # 模块入口
├── context.rs      # UiContext
├── style.rs        # UiStyle
├── label.rs        # Label 控件
├── button.rs       # Button 控件
├── panel.rs        # Panel 容器
├── input_field.rs  # TextInput 控件
├── checkbox.rs     # Checkbox 控件
└── slider.rs       # Slider 控件
```

## 模块详细设计

### 1. UiStyle（widgets/style.rs）

```rust
pub struct UiStyle {
    pub font_name: String,
    pub font_size: f32,
    pub text_color: Color,
    pub bg_color: Color,
    pub hover_color: Color,
    pub active_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub padding: [f32; 4], // [top, right, bottom, left]
    pub spacing: f32,
    pub corner_radius: f32,
    pub accent_color: Color,      // checkbox 勾选、slider 滑块
    pub input_bg_color: Color,    // 输入框背景
    pub cursor_color: Color,      // 输入光标
}
```

### 2. UiContext（widgets/context.rs）

```rust
pub struct UiContext<'a> {
    text_painter: &'a mut TextPainter,
    shape_painter: &'a mut ShapePainter,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    style: UiStyle,
    cursor: [f32; 2],
    mouse_pos: [f32; 2],
    mouse_down: bool,
    mouse_clicked: bool,
    focused_id: Option<u64>,
    next_id: u64,
}
```

方法：
- `new(text_painter, shape_painter, device, queue, style, mouse_pos, mouse_down, mouse_clicked)` — 创建
- `label(text)` — 显示文字标签
- `button(label) -> bool` — 显示按钮，返回是否点击
- `checkbox(label, checked: &mut bool)` — 复选框
- `slider(label, value: &mut f32, min, max)` — 滑块
- `text_input(label, buffer: &mut String)` — 文本输入
- `panel(size, closure)` — 面板容器
- `set_cursor(pos)` — 设置布局光标
- `advance(dy)` — 向下移动光标
- `style() -> &UiStyle` — 获取当前样式

每个控件自动分配 ID（next_id += 1），用于状态追踪。

### 3. Label（widgets/label.rs）

无交互，纯文字显示。
- 调用 TextPainter::draw_text 在 cursor 位置
- 自动 advance(dy = line_height + spacing)

### 4. Button（widgets/button.rs）

文字 + 圆角矩形背景。
- 测量文字宽度，计算按钮大小（文字 + padding）
- ShapePainter::rounded_rect 画背景（hover/active 时换颜色）
- TextPainter::draw_text 居中显示
- 检测 mouse 在矩形内 + mouse_clicked → 返回 true

### 5. Checkbox（widgets/checkbox.rs）

勾选框 + 文字标签。
- ShapePainter::rect 画外框
- 如果 checked: ShapePainter::rect 画内部填充（accent_color）
- TextPainter::draw_text 画标签
- 点击切换 checked

### 6. Slider（widgets/slider.rs）

滑轨 + 滑块 + 数值显示。
- ShapePainter::rect 画轨道
- ShapePainter::circle 画滑块（位置根据 value 计算）
- TextPainter::draw_text 画标签 + 当前值
- 拖拽更新 value（clamp 到 min..max）

### 7. TextInput（widgets/input_field.rs）

输入框 + 光标。
- ShapePainter::rect 画背景
- ShapePainter::rect_stroked 画边框
- TextPainter::draw_text 画 buffer 内容
- 如果 focused: ShapePainter::rect 画闪烁光标
- 键盘输入追加到 buffer（Backspace 删除，Enter 提交）

### 8. Panel（widgets/panel.rs）

容器，设置子控件布局区域。
- ShapePainter::rect 画面板背景
- 保存/恢复 cursor
- 子控件在面板内布局（从面板顶部开始）

## 与渲染管线集成

UiContext 在每帧渲染时创建：
```rust
let text_painter = world.get_resource_mut::<TextPainter>().unwrap();
let shape_painter = world.get_resource_mut::<ShapePainter>().unwrap();
let input = world.get_resource::<InputManager>().unwrap();

let mut ui = UiContext::new(
    &mut text_painter, &mut shape_painter,
    &device, &queue,
    UiStyle::default(),
    input.mouse_position(),
    input.mouse_down(MouseButton::Left),
    input.mouse_just_pressed(MouseButton::Left),
);

ui.label("Score: 1000");
if ui.button("Play") { start_game(); }
```

渲染顺序：UI 在所有 Sprite/Shape 之后渲染（最上层）。

## 依赖

- `engine-render::font::TextPainter` — 文字渲染
- `engine-render::shape::ShapePainter` — Shape 绘制
- `engine-input::InputManager` — 输入状态

## 测试策略

1. **Style 测试**：默认值验证
2. **Layout 测试**：cursor 自动推进
3. **Button 交互测试**：hover/click 状态变化
4. **Slider 测试**：value clamp 和拖拽
5. **TextInput 测试**：字符追加和删除

## 性能考量

- Immediate Mode 每帧重建所有控件，但游戏 UI 通常 <100 个控件，开销可忽略
- TextPainter 和 ShapePainter 内部有缓存，相同文字/形状不重复光栅化

## 后续优化（不在本次范围）

- 滚动容器
- 动画/过渡效果
- 布局算法（flexbox）
- 主题系统
