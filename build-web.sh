#!/bin/bash
# Build the editor for WASM/Web target
# Requires: wasm32-unknown-unknown target installed
# Install: rustup target add wasm32-unknown-unknown

set -e

echo "Building RustEngine Editor for WASM..."

cargo build -p engine-editor --target wasm32-unknown-unknown --release 2>&1

echo "Build complete. Output at: target/wasm32-unknown-unknown/release/engine-editor.wasm"

# Generate JS bindings with wasm-bindgen
if command -v wasm-bindgen &> /dev/null; then
    wasm-bindgen target/wasm32-unknown-unknown/release/engine-editor.wasm \
        --out-dir dist/web \
        --target web
    echo "JS bindings generated at: dist/web/"
else
    echo "wasm-bindgen not found. Install with: cargo install wasm-bindgen-cli"
fi
