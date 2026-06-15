## WASM Build Status (Updated 2026-06-16)

**Compiles for WASM:**
- ✅ engine-math
- ✅ engine-ecs
- ✅ engine-render (`cargo build -p engine-render --target wasm32-unknown-unknown`)
- ✅ engine-editor lib (`cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib`)
- ❌ engine-editor bin (requires native event loop, WASM uses `start_wasm()` entry point)

**What was fixed:**
1. `unsafe impl Send/Sync` for wgpu wrapper types on WASM (Mesh, MaterialStore, GpuDevice, GpuQueue, Renderer)
2. cfg-gated `par_iter` → sequential `for` loop on WASM
3. cfg-gated `Renderer::new` (native-only), added `Renderer::new_async` (WASM)
4. cfg-gated `plugin` module behind `not(wasm32)`
5. cfg-gated `run_default()` in engine-core behind `not(wasm32)`
6. Made `mlua`/`engine-script` optional via `scripting` feature
7. Made `rfd` optional via `native-dialogs` feature
8. cfg-gated file dialog functions
9. cfg-gated script initialization/stepping code
10. Added `getrandom` with `wasm_js` feature

**How to build for WASM:**
```bash
# Full render crate
cargo build -p engine-render --target wasm32-unknown-unknown

# Editor library only (no binary)
cargo build -p engine-editor --target wasm32-unknown-unknown --no-default-features --lib
```

**Next steps for full WASM runtime:**
1. Create a `wasm_main` entry point using `wasm_bindgen_futures::spawn_local`
2. Implement async renderer initialization in the editor
3. Use `requestAnimationFrame` instead of blocking event loop
4. Test with actual WASM runtime (wasm-pack or trunk)
