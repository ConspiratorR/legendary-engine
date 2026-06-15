## WASM 构建状态 (更新于 2026-06-16)

### 编译状态

| Crate | 状态 | 命令 |
|-------|------|------|
| engine-math | ✅ | `cargo build -p engine-math --target wasm32-unknown-unknown` |
| engine-ecs | ✅ | `cargo build -p engine-ecs --target wasm32-unknown-unknown` |
| engine-render | ✅ | `cargo build -p engine-render --target wasm32-unknown-unknown` |
| engine-editor (lib) | ✅ | `cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib` |
| engine-editor (bin) | ❌ | 需要原生事件循环, WASM 使用 `start_wasm()` 入口点 |

### 已完成的修复

1. **Send/Sync 问题** — 在 WASM 上为 wgpu 包装类型添加 `unsafe impl Send/Sync` (Mesh, MaterialStore, GpuDevice, GpuQueue, Renderer)
2. **并行迭代** — 将 `par_iter` 替换为顺序 `for` 循环 (WASM 单线程)
3. **渲染器初始化** — 添加 `Renderer::new_async()` 用于异步初始化
4. **插件模块** — 使用 `not(wasm32)` 条件编译
5. **默认运行** — 在 engine-core 中使用 `not(wasm32)` 条件编译
6. **脚本系统** — 通过 `scripting` feature flag 可选
7. **文件对话框** — 通过 `native-dialogs` feature flag 可选
8. **随机数生成** — 添加 `getrandom` 的 `wasm_js` feature

### 构建命令

```bash
# 安装 WASM 目标
rustup target add wasm32-unknown-unknown

# 构建渲染器
cargo build -p engine-render --target wasm32-unknown-unknown

# 构建编辑器库 (不含二进制)
cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib

# 使用 build-web.sh 脚本
./build-web.sh
```

### Feature Flags

engine-editor 的 feature flags:
- `default = ["native", "scripting", "native-dialogs"]`
- `native` — 原生平台支持
- `web` — Web/WASM 平台支持
- `scripting` — Lua 脚本支持 (mlua + engine-script)
- `native-dialogs` — 原生文件对话框 (rfd)

### 下一步

1. 创建 `wasm_main` 入口点 (使用 `wasm_bindgen_futures::spawn_local`)
2. 实现编辑器的异步渲染器初始化
3. 使用 `requestAnimationFrame` 替代阻塞事件循环
4. 使用实际 WASM 运行时测试 (wasm-pack 或 trunk)

### 关键文件

- `crates/engine-render/src/renderer.rs` — `Renderer::new_async()` 异步初始化
- `crates/engine-render/src/resource/mesh.rs` — WASM 上的 `unsafe impl Send/Sync`
- `crates/engine-render/src/resource/material.rs` — WASM 上的 `unsafe impl Send/Sync`
- `crates/engine-editor/Cargo.toml` — Feature flags 定义
- `crates/engine-editor/src/layout.rs` — 文件对话框条件编译
- `crates/engine-editor/src/main.rs` — 脚本系统条件编译
- `crates/engine-core/src/engine.rs` — `run_default()` 条件编译
- `build-web.sh` — WASM 构建脚本
