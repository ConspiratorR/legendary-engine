## WASM Build Status

**Compiles for WASM:**
- ✅ engine-math
- ✅ engine-ecs
- ❌ engine-render (wgpu types not Send/Sync on WASM)
- ❌ engine-editor (depends on engine-render)

**Remaining issues:**
1. `wgpu::Buffer` is not `Send`/`Sync` on WASM (DynContext not thread-safe)
2. ECS `World::insert_resource` requires `Send + Sync`
3. `Renderer::new` only exists on native (needs `new_async` for WASM)
4. `getrandom` needs `wasm_js` feature flag

**Solutions:**
- Use `unsafe impl Send/Sync` wrapper for wgpu types on WASM
- Or use a separate ECS resource type for WASM that doesn't require Send
- Or use `wasm-bindgen`'s single-threaded model

**Next steps:**
1. Create a `WgpuWrapper<T>` type that implements Send+Sync unsafely on WASM
2. Use cfg(target_arch) to conditionally compile ECS resource insertion
3. Test with actual WASM runtime (wasm-pack or trunk)
