# RustEngine 品牌设计规范

## 产品定位
专业游戏引擎IDE，支持3D/2D游戏开发、实时渲染、资产管理和多平台发布。

## 视觉风格
游戏/创意工具风格 - 色彩鲜明、富有科技感、专业的开发者工具

## 色彩系统

### 主色调 (深色编辑器)
```css
--bg-primary:    #0d0d0f;      /* 深邃黑 - 主背景 */
--bg-secondary:   #161619;      /* 略亮黑 - 面板背景 */
--bg-tertiary:    #1e1e22;      /* 卡片/悬停背景 */
--bg-elevated:    #252529;      /* 浮层/对话框 */
--surface:        #2a2a30;      /* 输入框/控件 */
--border:         #3a3a42;      /* 边框 */
--border-subtle:  #2d2d35;      /* 细分边框 */
```

### 文字颜色
```css
--text-primary:   #e8e8ec;      /* 主要文字 */
--text-secondary: #9898a0;      /* 次要文字 */
--text-muted:     #5a5a66;      /* 禁用/提示 */
--text-accent:    #00d4aa;      /* 强调文字（青色）*/
```

### 强调色系统
```css
--accent-primary: #00d4aa;      /* 主强调 - 青绿 (rust橙改为青绿) */
--accent-secondary: #ff6b35;    /* 次强调 - 活力橙 */
--accent-tertiary: #7c5cff;     /* 第三强调 - 紫色 */
--accent-warning: #ffb800;      /* 警告 - 黄色 */
--accent-error:   #ff4757;      /* 错误 - 红 */
--accent-success: #2ed573;      /* 成功 - 绿 */
```

### 功能色
```css
--tool-red:       #ff6b6b;
--tool-orange:    #ffa502;
--tool-yellow:    #ffd43b;
--tool-green:     #26de81;
--tool-cyan:      #00d4aa;
--tool-blue:      #4dabf7;
--tool-purple:    #9775fa;
--tool-pink:      #ff6b9d;
```

## 字体系统
```css
--font-display: 'JetBrains Mono', 'Fira Code', monospace;
--font-body:    -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
--font-mono:     'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
```

### 字号
- H1: 32px / 700
- H2: 24px / 600
- H3: 18px / 600
- Body: 14px / 400
- Caption: 12px / 400
- Code: 13px / 400

## 间距系统
- xs: 4px
- sm: 8px
- md: 12px
- lg: 16px
- xl: 24px
- 2xl: 32px

## 圆角
- sm: 4px
- md: 6px
- lg: 8px
- full: 9999px

## 阴影
```css
--shadow-sm: 0 2px 4px rgba(0,0,0,0.3);
--shadow-md: 0 4px 12px rgba(0,0,0,0.4);
--shadow-lg: 0 8px 24px rgba(0,0,0,0.5);
--shadow-glow: 0 0 20px rgba(0,212,170,0.3); /* 青色发光 */
```

## 组件样式

### 按钮
- Primary: bg #00d4aa, text #0d0d0f, hover 亮度+10%
- Secondary: bg transparent, border #3a3a42, text #e8e8ec
- Danger: bg #ff4757

### 输入框
- bg: #1e1e22
- border: #3a3a42
- focus: border #00d4aa + shadow-glow

### 面板
- bg: #161619
- border-right: 1px solid #2d2d35

### 标签页
- inactive: text #5a5a66
- active: text #00d4aa, border-bottom 2px solid #00d4aa

### 树形列表
- 缩进: 16px/级
- 展开/收起图标: 12px chevron
- hover: bg #1e1e22
- selected: bg #00d4aa20, text #00d4aa

### 视口3D
- 背景: 渐变 #0a0a0c 到 #141418
- 网格: #252530
- 坐标轴: X红 Y绿 Z蓝

## 动画
- transition: 150ms ease
- hover scale: 1.02
- panel slide: 200ms ease-out