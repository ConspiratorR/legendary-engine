## WASM 构建状态 (更新于 2026-07-13, phase 21)

### 编译状态

| Crate | 状态 | 命令 |
|-------|------|------|
| engine-math | ✅ | `cargo build -p engine-math --target wasm32-unknown-unknown` |
| engine-ecs | ✅ | `cargo build -p engine-ecs --target wasm32-unknown-unknown` |
| engine-render | ✅ | `cargo build -p engine-render --target wasm32-unknown-unknown` |
| **engine-core (lib)** | ✅ **phase 21** | `cargo build -p engine-core --target wasm32-unknown-unknown` |
| engine-editor (lib) | ✅ | `cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib` |
| engine-editor (bin) | ❌ | 需要原生事件循环, WASM 使用 `start_wasm()` 入口点 |
| web-demo | ✅ | `wasm-pack build --target web --release` |

### Phase 21 — engine-core WASM 门控

- `libloading` 仅在 **非 wasm32** 目标依赖（`cfg(not(target_arch = "wasm32"))`）。
- `DynamicPlugin::load` / `PluginLoader::load_all` / `AppBuilder::load_dynamic_plugins`：wasm 返回 `PluginLoadError::UnsupportedPlatform`。
- `PluginLoader::register_all`：wasm **no-op**（不返回错误）；manifest/registry 类型仍可用。

### Phase 22 — WASM SceneRuntime API 面

| 路径 | WASM | 说明 |
|------|------|------|
| `SceneRuntime::new` / spawn / `LoadSceneJson` / `tick` | ✅ 推荐 | **JSON 字符串**场景；无文件系统 |
| `SceneManager::LoadSceneFromFile` | ❌ | 返回 `Err`，提示改用 `LoadSceneJson` |
| `AssetDatabase::poll_hot_reload` | ⭕ no-op | 恒返回空事件 |
| `save_scriptable_object` / `load_scriptable_object` | ❌ | 返回错误（native 不变） |
| 浏览器 WebGL 实机 SceneRuntime | ❌ | 后续切片 |

**未做**：SceneRuntime 全量浏览器运行 / 生命周期实机验证。

### 实际运行测试

- ✅ wasm-pack 构建成功
- ✅ HTTP 服务器运行 (`python -m http.server 8080`)
- ✅ 浏览器访问 `http://localhost:8080` 可见 wgpu 渲染窗口
- ✅ 标题 "RustEngine Web", 深色背景正常显示

### 已完成的修复

1. **Send/Sync 问题** — 在 WASM 上为 wgpu 包装类型添加 `unsafe impl Send/Sync` (Mesh, MaterialStore, GpuDevice, GpuQueue, Renderer)
2. **并行迭代** — 将 `par_iter` 替换为顺序 `for` 循环 (WASM 单线程)
3. **渲染器初始化** — 添加 `Renderer::new_async()` 用于异步初始化
4. **插件模块** — 使用 `not(wasm32)` 条件编译
5. **默认运行** — 在 engine-core 中使用 `not(wasm32)` 条件编译
6. **脚本系统** — 通过 `scripting` feature flag 可选
7. **文件对话框** — 通过 `native-dialogs` feature flag 可选
8. **随机数生成** — 添加 `getrandom` 的 `wasm_js` feature
9. **入口点** — `start_wasm()` 异步函数 + `wasm_bindgen_futures::spawn_local`
10. **libloading / 动态插件** — phase 21：wasm 门控，`UnsupportedPlatform`

### 构建命令

```bash
# 安装 WASM 目标
rustup target add wasm32-unknown-unknown

# 构建 engine-core（phase 21 起可编译）
cargo build -p engine-core --target wasm32-unknown-unknown

# 构建渲染器
cargo build -p engine-render --target wasm32-unknown-unknown

# 构建编辑器库 (不含二进制)
cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib

# 构建并运行 Web Demo
cd examples/web-demo
wasm-pack build --target web --release
python -m http.server 8080
# 浏览器访问 http://localhost:8080
```

### Feature Flags

engine-editor 的 feature flags:
- `default = ["native", "scripting", "native-dialogs"]`
- `native` — 原生平台支持
- `web` — Web/WASM 平台支持
- `scripting` — Lua 脚本支持 (mlua + engine-script)
- `native-dialogs` — 原生文件对话框 (rfd)

engine-core 默认 features（native）：`["audio", "unity-world-primary"]`。WASM 构建建议：

```bash
cargo build -p engine-core --target wasm32-unknown-unknown --no-default-features
# 或仅 unity-world-primary：--no-default-features --features unity-world-primary
```

### 关键文件

- `crates/engine-editor/src/lib.rs` — `start_wasm()` 异步入口点
- `crates/engine-render/src/renderer.rs` — `Renderer::new_async()` 异步初始化
- `crates/engine-render/src/resource/mesh.rs` — WASM 上的 `unsafe impl Send/Sync`
- `crates/engine-render/src/resource/material.rs` — WASM 上的 `unsafe impl Send/Sync`
- `crates/engine-editor/Cargo.toml` — Feature flags 定义
- `crates/engine-editor/src/layout.rs` — 文件对话框条件编译
- `crates/engine-editor/src/main.rs` — 脚本系统条件编译
- `crates/engine-core/src/engine.rs` — `run_default()` 条件编译
- `examples/web-demo/` — 完整 Web Demo (wasm-pack + index.html)
- `build-web.sh` — WASM 构建脚本
