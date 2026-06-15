# Contributing Guide

How to contribute to RustEngine.

## Prerequisites

- Rust 2024 edition (toolchain 1.95.0+)
- Windows: Visual Studio Build Tools with C++ workload

## Code Style

- **Formatting**: Always run `cargo fmt` before committing. The CI enforces `cargo fmt --check`.
- **Linting**: Run `cargo clippy` and fix all warnings before committing.
- **No unsafe**: Avoid `unsafe` blocks. If absolutely necessary, document why with a `// SAFETY:` comment.
- **Naming**: Follow Rust conventions — `snake_case` for functions/variables, `PascalCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants.

## Error Handling

- Use `anyhow::Result` for application-level errors (examples, binary entry points).
- Use `thiserror` for library error types that callers may want to match on.
- Never `.unwrap()` in library code — propagate errors with `?`.
- `.unwrap()` is acceptable in examples and tests where failure means the test should panic.

## Testing

- Unit tests go in the same file as the code, inside `#[cfg(test)] mod tests { ... }`.
- Integration tests go in `tests/` directories within each crate.
- Run tests per crate: `cargo test -p engine-ecs` (workspace-wide `cargo test` has known issues).
- Known test failures are documented in `AGENTS.md` — do not attempt to fix those unless specifically tasked.

## Git Workflow

### Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `test`: Adding or updating tests
- `chore`: Build, CI, tooling changes

Scopes are crate names without the `engine-` prefix (e.g., `ecs`, `render`, `physics`).

Examples:
```
feat(ecs): add query caching for read-only systems
fix(render): correct sprite batch sorting by depth
docs: add contributing guide
```

### Branches

- `main` — stable, always builds and passes tests.
- Feature branches — `feat/<short-description>` or `fix/<short-description>`.
- Rebase or squash-merge into `main`.

### Pre-commit Checklist

```bash
cargo fmt --check
cargo clippy
cargo test -p <affected-crate>
```

## Project Structure

```
crates/
├── engine-math/       # Linear algebra (Vec3, Mat4, Quat)
├── engine-ecs/        # Entity-Component-System core
├── engine-window/     # Window creation and management
├── engine-input/      # Keyboard, mouse, gamepad input
├── engine-audio/      # Sound playback
├── engine-asset/      # Asset loading, hot-reload, .meta files
├── engine-scene/      # Scene serialization, prefabs
├── engine-render/     # wgpu rendering pipeline (deferred shading)
├── engine-physics/    # Physics simulation (rigid bodies, colliders, joints)
├── engine-framework/  # Game states, app lifecycle
├── engine-script/     # Lua/WASM scripting
├── engine-terrain/    # Terrain system
├── engine-ui/         # UI framework (egui integration)
├── engine-editor/     # Editor application
├── engine-network/    # Client/server networking
├── engine-jobs/       # Thread pool and job graphs
└── engine-core/       # Re-exports and integration
```

## Building for Different Platforms

### Native (Windows/macOS/Linux)

```bash
cargo build
cargo run -p engine-editor
```

### WASM/Web

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Build renderer for WASM
cargo build -p engine-render --target wasm32-unknown-unknown

# Build editor library for WASM (no binary)
cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib
```

## Adding a New Crate

1. Create the crate directory with `cargo new crates/engine-<name> --lib`.
2. Add it to the workspace `Cargo.toml` members list.
3. Add dependencies to the new crate's `Cargo.toml`.
4. Export the crate from `engine-core` if it should be part of the public API.

## Documentation

- Rustdoc comments (`///`) on all public items.
- Module-level `//!` docs explaining the module's purpose.
- Examples in doc comments where behavior is non-obvious.
- Larger guides go in `docs/` as markdown files.
