# RustEngine 编辑器架构设计

## 目标
创建一个类似 Unity 的可视化编辑器引擎，提供实时场景编辑、游戏运行、调试等完整工具链。

## 核心架构

### 编辑器子系统

```
Editor (编辑器主窗口)
├── MenuBar (菜单栏)
├── Toolbar (工具栏 - 变换工具选择)
├── HierarchyPanel (层级视图)
│   └── SceneTree (场景树)
├── Viewport (视口)
│   ├── EditorCamera (编辑器相机)
│   ├── GizmosRenderer (变换工具渲染)
│   └── GridRenderer (网格渲染)
├── Inspector (属性检查器)
│   ├── TransformEditor
│   ├── ComponentList
│   └── AddComponentMenu
├── ProjectBrowser (资源浏览器)
│   ├── FolderTree
│   └── AssetGrid
├── Console (控制台)
│   └── LogViewer
└── StatusBar (状态栏)
```

### 编辑器状态

```rust
pub enum EditorMode {
    Edit,          // 编辑模式
    Playing,       // 运行模式
    Paused,        // 暂停
    Simulating,    // 模拟（物理）
}
```

## 核心功能模块

### 1. 场景管理
- 多场景支持
- 场景序列化/反序列化（JSON/YAML）
- 场景切换
- 运行时场景修改

### 2. 层级视图（Hierarchy）
- 树形结构显示
- 拖拽重排序
- 展开/折叠
- 搜索过滤
- 多选支持
- 拖拽创建父子关系

### 3. 属性检查器（Inspector）
- 变换组件编辑
  - Position (Vec3)
  - Rotation (Vec3 - 欧拉角)
  - Scale (Vec3)
- 组件添加/删除
- 组件启用/禁用
- 数组/列表编辑
- 资源引用选择器

### 4. 变换工具（Gizmos）
- 移动工具 (Translate)
  - 显示 XYZ 轴拖柄
  - 显示平面拖柄 (XY, XZ, YZ)
- 旋转工具 (Rotate)
  - 显示 XYZ 旋转环
- 缩放工具 (Scale)
  - 显示 XYZ 缩放手柄
  - 显示统一缩放手柄

### 5. 视口系统
- 正交/透视切换
- 相机旋转 (右键拖拽)
- 相机平移 (中键拖拽)
- 相机缩放 (滚轮)
- 焦点到选中物体 (F键)
- 网格显示
- 轴向指示器

### 6. 撤销/重做系统
- 命令模式实现
- 支持操作:
  - 创建实体
  - 删除实体
  - 移动实体
  - 修改组件
  - 重命名实体
  - 修改变换

### 7. 资源管理
- 资源导入 (图像, 音频, 模型)
- 资源引用追踪
- 资源预览
- 文件夹管理
- 搜索过滤

### 8. 快捷键系统
```
Ctrl+S - 保存场景
Ctrl+Z - 撤销
Ctrl+Y - 重做
Ctrl+C - 复制
Ctrl+V - 粘贴
Ctrl+D - 复制并创建
Delete - 删除选中
F - 焦点到选中
Q - 移动工具
W - 移动工具
E - 旋转工具
R - 缩放工具
T - 矩形选择
```

## 运行时集成

### Play/Stop 流程

```
Edit Mode -> [Play Button] -> Playing Mode
                                    |
                                    v
                              [Stop Button]
                                    |
                                    v
Edit Mode (恢复之前状态)
```

### 运行时修改追踪
- 记录运行时的组件修改
- 支持在 Play 模式下编辑
- Pause 时可以检查状态

## 技术实现

### 渲染集成
- 复用 engine-render 的渲染器
- 编辑器 UI 使用 engine-ui (ImGui)
- Gizmos 使用自定义渲染通道

### 事件系统
```rust
pub enum EditorEvent {
    SelectionChanged(Vec<Entity>),
    SceneModified,
    PlayModeChanged(EditorMode),
    AssetImported(AssetId),
    UndoPerformed,
    RedoPerformed,
}
```

### 序列化格式
```json
{
  "scene": {
    "name": "MainScene",
    "entities": [
      {
        "id": 1,
        "name": "Player",
        "components": [
          {
            "type": "Transform",
            "position": [0, 0, 0],
            "rotation": [0, 0, 0],
            "scale": [1, 1, 1]
          },
          {
            "type": "Sprite",
            "texture": "player.png",
            "color": "#FFFFFF"
          }
        ]
      }
    ]
  }
}
```

## 性能优化

### 编辑器专用优化
- 视口分辨率动态调整
- 场景大时禁用实时预览
- 延迟刷新非活动面板
- 对象池复用临时对象

### 层级视图优化
- 虚拟化列表（大量对象时）
- 展开状态缓存
- 搜索结果缓存

## 扩展性

### 组件编辑器注册
```rust
pub trait ComponentEditor {
    fn draw(&mut self, component: &mut dyn Component);
}

pub struct EditorRegistry {
    component_editors: HashMap<TypeId, Box<dyn ComponentEditor>>,
}
```

### 自定义工具栏
```rust
pub trait EditorTool {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn shortcut(&self) -> KeyCode;
    fn draw(&mut self, ctx: &mut EditorContext);
}
```

## 开发进度

- [x] 基本编辑器布局
- [x] 层级视图
- [x] Inspector (基础)
- [x] Viewport (基础)
- [x] 场景树
- [ ] Gizmos
- [ ] 完整 Inspector
- [ ] 撤销/重做
- [ ] 场景保存/加载
- [ ] 资源浏览器
- [ ] 快捷键系统
- [ ] 运行时调试

## 参考Unity功能

1. Scene View - ✅ 基础完成
2. Game View - 待实现
3. Hierarchy - ✅ 基础完成
4. Inspector - ⚠️ 部分完成
5. Project Browser - 待实现
6. Console - 待实现
7. Gizmos - ⚠️ 部分完成
8. Play Mode - 待实现
9. Undo/Redo - 待实现
10. Prefabs - 待实现
