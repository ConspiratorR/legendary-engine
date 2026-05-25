# AGENTS.md — RustEngine (legendary-engine)

Rust game engine (MIT license, author: ConspiratorR).

## Current state

14 crates with real implementation (~9k+ lines non-test). Core infrastructure (ECS, app, input, scene, asset) complete. Rendering pipeline (wgpu render graph, sprite pipeline, camera) in progress. Physics/network partially implemented (types real, runtime stubbed). Editor has extensive UI scaffolding.

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
- `engine-asset` tests fail — missing `tempfile` dev-dep
- `engine-core` examples with outdated `KeyCode` variants

Expected order: `cargo clippy && cargo fmt --check && cargo test`.

## Style

- Follow `cargo fmt` formatting.
- No `unsafe` unless unavoidable and documented.
- Prefer `anyhow`/`thiserror` for error handling.
- Rust 2024 edition (toolchain: 1.95.0).

## Notes

- `debug/` and `target/` are gitignored (Cargo defaults).
- `.idea/` is gitignored by convention but not committed to `.gitignore`.
- Add CI (`.github/workflows/`) as a future task (see roadmap Stage 9).
