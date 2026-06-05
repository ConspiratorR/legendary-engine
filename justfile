# RustEngine Build Automation
# Usage: just <command>

# Default recipe - show available commands
default:
    @just --list

# Build the project in debug mode
build:
    cargo build

# Build with minimal features (no audio, physics, network, script)
build-minimal:
    cargo build --no-default-features

# Build the project in release mode
build-release:
    cargo build --release

# Run all tests
test:
    cargo test --all --no-fail-fast

# Run clippy linting
lint:
    cargo clippy --all -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run all CI checks (fmt, clippy, build, test)
ci: fmt-check lint build test

# Quick local validation
check:
    cargo fmt --check
    cargo clippy --all -- -D warnings
    cargo build --all
    cargo test --all

# Run a specific example
run-example name:
    cargo run --example {{name}} -p engine-core

# List available examples
list-examples:
    @echo "Available examples:"
    @ls crates/engine-core/examples/*.rs | ForEach-Object { [System.IO.Path]::GetFileNameWithoutExtension($_) }

# Clean build artifacts
clean:
    cargo clean

# Generate documentation
doc:
    cargo doc --all --no-deps --open

# Run the editor
editor:
    cargo run -p engine-editor

# Cross-compile for Linux (requires cross-compilation toolchain)
build-linux:
    cargo build --target x86_64-unknown-linux-gnu

# Cross-compile for macOS (requires cross-compilation toolchain)
build-macos:
    cargo build --target x86_64-apple-darwin

# Cross-compile for Android (requires Android NDK)
build-android:
    cargo build --target aarch64-linux-android

# Run all platform builds (CI simulation)
build-all-platforms: build build-linux build-macos

# Check for security vulnerabilities
audit:
    cargo audit

# Update dependencies
update:
    cargo update

# Show project info
info:
    @echo "RustEngine - Cross-platform Game Engine"
    @echo "========================================"
    @echo "Workspace members:"
    @cargo metadata --no-deps --format-version 1 | ConvertFrom-Json | Select-Object -ExpandProperty packages | ForEach-Object { $_.name }

# Show dependency tree
deps:
    cargo tree --depth 1

# Check for duplicate dependencies
dedup:
    cargo tree --duplicates
