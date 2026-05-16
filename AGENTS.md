# AGENTS.md — RustEngine (legendary-engine)

Rust game engine (MIT license, author: ConspiratorR).

## Current state

Repository is a skeleton — single initial commit, no `Cargo.toml` or source code yet.

## Commands (when project matures)

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run                # run the engine/demo
cargo test               # all tests
cargo clippy             # lint (run before committing)
cargo fmt                # format (run before committing)
```

Expected order: `cargo clippy && cargo fmt --check && cargo test`.

## Style

- Follow `cargo fmt` formatting.
- No `unsafe` unless unavoidable and documented.
- Prefer `anyhow`/`thiserror` for error handling.
- Rust 2024 edition (toolchain: 1.95.0).

## Notes

- `debug/` and `target/` are gitignored (Cargo defaults).
- `.idea/` is gitignored by convention but not committed to `.gitignore`.
- Add CI (`.github/workflows/`) and a `Cargo.toml` workspace once source modules are introduced.
