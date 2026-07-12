# AGENTS.md — RustEngine (legendary-engine)

Rust game engine (MIT license, author: ConspiratorR).

## Current state

17 crates, ~84K lines. wgpu deferred renderer, egui editor. All high/medium priority tasks (#1-10) complete. WASM/Web, plugin system, mod system, documentation overhaul complete. Android foundation done (needs NDK). VR/AR deferred.

**Before planning any feature work**, read the development roadmap in `README.md` (section "开发路线图") to understand priorities, dependencies, and what's already done vs pending.

## Commands

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run                # run the editor
cargo test               # all tests (run per crate, not workspace-wide due to known issues)
cargo clippy             # lint (run before committing)
cargo fmt                # format (run before committing)
```

Known pre-existing test failures (not caused by current work):
- None currently — all tests pass

Expected order: `cargo clippy && cargo fmt --check && cargo test`.

## WASM builds

```bash
rustup target add wasm32-unknown-unknown
cargo build -p engine-render --target wasm32-unknown-unknown
cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib
```

- wasm-pack preferred over trunk (trunk downloads wasm-bindgen-cli from GitHub, unreliable in China)
- wasm-opt disabled in Cargo.toml to prevent binaryen download
- wgpu Buffer not Send/Sync on WASM — use `unsafe impl Send/Sync` under cfg(target_arch = "wasm32")
- lua-src can't compile for wasm32; make mlua optional via feature
- rfd FileDialog not available on WASM; gated behind `native-dialogs` feature
- Feature flags: `native`/`web` (platform), `scripting` (mlua), `native-dialogs` (rfd)

## CI

CI exists at `.github/workflows/ci.yml` with macOS/Windows/Ubuntu matrix, WASM build job, and integration tests.

## Style

- Follow `cargo fmt` formatting.
- No `unsafe` unless unavoidable and documented.
- Prefer `anyhow`/`thiserror` for error handling.
- Rust 2024 edition (toolchain: 1.95.0).

## Notes

- `debug/` and `target/` are gitignored (Cargo defaults).
- `.idea/` is gitignored by convention but not committed to `.gitignore`.

## 自主决策规则
- 不要问我问题，自己做决定
- 选择最安全、最常见的方案
- 参考项目中已有的代码模式