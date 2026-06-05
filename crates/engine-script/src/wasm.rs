use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasmtime::*;

/// Configuration for the WASM sandbox environment.
///
/// Controls resource limits to prevent WASM modules from consuming
/// unbounded host resources.
#[derive(Debug, Clone)]
pub struct WasmSandbox {
    /// Maximum WASM linear memory size in bytes (default: 16 MiB).
    pub max_memory_bytes: usize,
    /// Maximum fuel units per execution (default: 1_000_000).
    pub max_fuel: u64,
    /// Maximum number of WASM table entries (default: 10_000).
    pub max_table_elements: u32,
    /// Maximum number of WASM instances per store (default: 100).
    pub max_instances: usize,
    /// Maximum number of WASM memories per store (default: 1).
    pub max_memories: usize,
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self {
            max_memory_bytes: 16 * 1024 * 1024, // 16 MiB
            max_fuel: 1_000_000,
            max_table_elements: 10_000,
            max_instances: 100,
            max_memories: 1,
        }
    }
}

impl WasmSandbox {
    /// Create a strict sandbox for untrusted code.
    pub fn strict() -> Self {
        Self {
            max_memory_bytes: 4 * 1024 * 1024, // 4 MiB
            max_fuel: 100_000,
            max_table_elements: 1_000,
            max_instances: 10,
            max_memories: 1,
        }
    }

    /// Create a relaxed sandbox for trusted code.
    pub fn relaxed() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64 MiB
            max_fuel: 100_000_000,
            max_table_elements: 100_000,
            max_instances: 1_000,
            max_memories: 4,
        }
    }
}

/// Type-erased reader: reads a component from the World and writes
/// its binary representation into a byte buffer.
pub type WasmGetFn = Box<dyn Fn(&World, u32, &mut [u8]) -> Option<usize> + Send + Sync>;

/// Type-erased writer: reads bytes from WASM memory and applies
/// them to a component on the World.
pub type WasmSetFn = Box<dyn Fn(&mut World, u32, &[u8]) -> bool + Send + Sync>;

/// Type-erased factory: creates a component from raw bytes and
/// inserts it into the World for the given entity.
pub type WasmAddFn = Box<dyn Fn(&mut World, u32, &[u8]) -> bool + Send + Sync>;

/// Registry mapping component names to their WASM binary bridge closures.
///
/// Each component type is registered with a fixed byte size and
/// closures that serialize/deserialize between Rust types and raw bytes.
pub struct WasmComponentBridge {
    getters: HashMap<String, WasmGetFn>,
    setters: HashMap<String, WasmSetFn>,
    adders: HashMap<String, WasmAddFn>,
    sizes: HashMap<String, usize>,
}

impl WasmComponentBridge {
    /// Create an empty WASM component bridge with no registered types.
    pub fn new() -> Self {
        Self {
            getters: HashMap::new(),
            setters: HashMap::new(),
            adders: HashMap::new(),
            sizes: HashMap::new(),
        }
    }

    /// Register a component type for WASM access.
    ///
    /// `size` is the byte size of the component in WASM linear memory.
    /// `to_bytes` converts `&T` to a byte vector.
    /// `from_bytes` converts a byte slice to `T`.
    pub fn register<T: Send + Sync + 'static>(
        &mut self,
        name: impl Into<String>,
        size: usize,
        to_bytes: fn(&T) -> Vec<u8>,
        from_bytes: fn(&[u8]) -> T,
    ) {
        let name = name.into();
        self.sizes.insert(name.clone(), size);

        self.getters.insert(
            name.clone(),
            Box::new(move |world, idx, buf| {
                let comp = world.get_by_index::<T>(idx)?;
                let bytes = to_bytes(comp);
                let len = bytes.len().min(buf.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Some(len)
            }),
        );

        self.setters.insert(
            name.clone(),
            Box::new(move |world, idx, bytes| {
                if let Some(c) = world.get_by_index_mut::<T>(idx) {
                    *c = from_bytes(bytes);
                    true
                } else {
                    false
                }
            }),
        );

        self.adders.insert(
            name,
            Box::new(move |world, idx, bytes| {
                let component = from_bytes(bytes);
                let entity = Entity::new(idx, 0);
                world.add_component(entity, component);
                true
            }),
        );
    }

    /// Get the byte size for a registered component.
    pub fn size_of(&self, name: &str) -> Option<usize> {
        self.sizes.get(name).copied()
    }

    /// Read a component value into a byte buffer.
    pub fn get(&self, world: &World, name: &str, idx: u32, buf: &mut [u8]) -> Option<usize> {
        match self.getters.get(name) {
            Some(f) => f(world, idx, buf),
            None => None,
        }
    }

    /// Write a component value from a byte slice.
    pub fn set(&self, world: &mut World, name: &str, idx: u32, bytes: &[u8]) -> bool {
        match self.setters.get(name) {
            Some(f) => f(world, idx, bytes),
            None => false,
        }
    }

    /// Add a component from a byte slice.
    pub fn add(&self, world: &mut World, name: &str, idx: u32, bytes: &[u8]) -> bool {
        match self.adders.get(name) {
            Some(f) => f(world, idx, bytes),
            None => false,
        }
    }

    /// Check if a component name is registered.
    pub fn has(&self, name: &str) -> bool {
        self.getters.contains_key(name)
    }

    /// Get all registered component names.
    pub fn registered_names(&self) -> Vec<&str> {
        self.getters.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for WasmComponentBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Send-safe wrapper for `*mut World`.
struct WorldPtr(*mut World);
unsafe impl Send for WorldPtr {}
unsafe impl Sync for WorldPtr {}

/// Send-safe wrapper for `*const WasmComponentBridge`.
struct BridgePtr(*const WasmComponentBridge);
unsafe impl Send for BridgePtr {}
unsafe impl Sync for BridgePtr {}

/// Host-side state stored in the wasmtime Store.
struct HostState {
    world: WorldPtr,
    bridge: BridgePtr,
    delta_time: f32,
    component_buffer: Vec<u8>,
    limiter: SandboxLimiter,
}

/// Resource limiter that enforces sandbox memory/table limits.
struct SandboxLimiter {
    max_memory_bytes: usize,
    max_table_elements: u32,
}

impl ResourceLimiter for SandboxLimiter {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        if desired > self.max_memory_bytes {
            Err(wasmtime::Error::msg(format!(
                "WASM memory limit exceeded: requested {} bytes, limit is {} bytes",
                desired, self.max_memory_bytes
            )))
        } else {
            Ok(true)
        }
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        if desired > self.max_table_elements as usize {
            Err(wasmtime::Error::msg(format!(
                "WASM table limit exceeded: requested {} elements, limit is {}",
                desired, self.max_table_elements
            )))
        } else {
            Ok(true)
        }
    }
}

/// A runtime for loading and executing WASM modules with ECS integration.
///
/// Wraps a wasmtime `Engine` with sandbox configuration and component bridge.
pub struct WasmRuntime {
    engine: Engine,
    sandbox: WasmSandbox,
}

impl WasmRuntime {
    /// Create a new WASM runtime with default sandbox settings.
    pub fn new() -> anyhow::Result<Self> {
        Self::with_sandbox(WasmSandbox::default())
    }

    /// Create a new WASM runtime with custom sandbox settings.
    pub fn with_sandbox(sandbox: WasmSandbox) -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(false);
        config.consume_fuel(true);
        config.max_wasm_stack(1024 * 1024); // 1 MiB stack

        let engine = Engine::new(&config).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(Self { engine, sandbox })
    }

    /// Get a reference to the underlying wasmtime Engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get the sandbox configuration.
    pub fn sandbox(&self) -> &WasmSandbox {
        &self.sandbox
    }

    /// Compile a WASM module from bytes.
    pub fn compile(&self, wasm_bytes: &[u8]) -> anyhow::Result<Module> {
        Module::new(&self.engine, wasm_bytes).map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Compile a WASM module from a file path.
    pub fn compile_file(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<Module> {
        Module::from_file(&self.engine, path).map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Create a Store with the sandbox limits applied.
    fn create_store(&self, dt: f32) -> Store<HostState> {
        let limiter = SandboxLimiter {
            max_memory_bytes: self.sandbox.max_memory_bytes,
            max_table_elements: self.sandbox.max_table_elements,
        };

        let mut store = Store::new(
            &self.engine,
            HostState {
                world: WorldPtr(std::ptr::null_mut()),
                bridge: BridgePtr(std::ptr::null()),
                delta_time: dt,
                component_buffer: vec![0u8; 4096],
                limiter,
            },
        );
        store.limiter(|state| &mut state.limiter);
        store
            .set_fuel(self.sandbox.max_fuel)
            .expect("Failed to set WASM fuel");
        store
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default WasmRuntime")
    }
}

/// A system that executes a WASM module each tick.
///
/// The WASM module can call host functions to interact with the ECS world:
///
/// - `spawn() -> i32` — spawn an entity, returns its index
/// - `despawn(entity: i32)` — despawn an entity
/// - `get_component(entity, name_ptr, name_len, result_ptr, result_cap) -> i32` — read component
/// - `set_component(entity, name_ptr, name_len, value_ptr, value_len) -> i32` — write component
/// - `add_component(entity, name_ptr, name_len, value_ptr, value_len) -> i32` — add component
/// - `has_component(entity, name_ptr, name_len) -> i32` — check if entity has component
/// - `component_size(name_ptr, name_len) -> i32` — get byte size of a component type
/// - `log(ptr, len)` — print a string to the host console
/// - `delta_time() -> f32` — get the current delta time
pub struct WasmSystem {
    name: String,
    module: Module,
    runtime: Arc<WasmRuntime>,
    bridge: Arc<RwLock<WasmComponentBridge>>,
}

// SAFETY: WasmSystem stores raw pointers in HostState only during execute(),
// where we ensure exclusive access via &mut World. The pointers never escape.
unsafe impl Send for WasmSystem {}
unsafe impl Sync for WasmSystem {}

impl WasmSystem {
    /// Create a new WASM system from compiled module bytes.
    /// Create a new WASM system by compiling `wasm_bytes` through `runtime`.
    pub fn new(
        name: impl Into<String>,
        wasm_bytes: &[u8],
        runtime: Arc<WasmRuntime>,
        bridge: Arc<RwLock<WasmComponentBridge>>,
    ) -> anyhow::Result<Self> {
        let module = runtime.compile(wasm_bytes)?;
        Ok(Self {
            name: name.into(),
            module,
            runtime,
            bridge,
        })
    }

    /// Create a new WASM system from a file path.
    /// Create a new WASM system by loading a `.wasm` file from disk.
    pub fn from_file(
        name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
        runtime: Arc<WasmRuntime>,
        bridge: Arc<RwLock<WasmComponentBridge>>,
    ) -> anyhow::Result<Self> {
        let module = runtime.compile_file(path)?;
        Ok(Self {
            name: name.into(),
            module,
            runtime,
            bridge,
        })
    }

    /// Reload the WASM module from new bytes.
    /// Replace the WASM module with freshly compiled bytes.
    pub fn reload(&mut self, wasm_bytes: &[u8]) -> anyhow::Result<()> {
        self.module = self.runtime.compile(wasm_bytes)?;
        Ok(())
    }

    /// Reload the WASM module from a file path.
    /// Reload the WASM module from a file path.
    pub fn reload_from_file(&mut self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        self.module = self.runtime.compile_file(path)?;
        Ok(())
    }

    /// Return the human-readable name assigned to this system.
    pub fn script_name(&self) -> &str {
        &self.name
    }

    /// Register host functions (WASM imports) on a Linker.
    fn register_host_functions(linker: &mut Linker<HostState>) -> anyhow::Result<()> {
        // spawn() -> i32
        linker
            .func_wrap("env", "spawn", |caller: Caller<'_, HostState>| -> i32 {
                let state = caller.data();
                let world = unsafe { &mut *state.world.0 };
                world.spawn().index() as i32
            })
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // despawn(entity: i32)
        linker
            .func_wrap(
                "env",
                "despawn",
                |caller: Caller<'_, HostState>, entity: i32| {
                    let state = caller.data();
                    let world = unsafe { &mut *state.world.0 };
                    world.despawn(Entity::new(entity as u32, 0));
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // get_component(entity, name_ptr, name_len, result_ptr, result_cap) -> i32
        linker
            .func_wrap(
                "env",
                "get_component",
                |mut caller: Caller<'_, HostState>,
                 entity: i32,
                 name_ptr: i32,
                 name_len: i32,
                 result_ptr: i32,
                 result_cap: i32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return 0,
                    };

                    // Read component name from WASM memory
                    let name = {
                        let mem_data = memory.data(&caller);
                        let start = name_ptr as usize;
                        let end = start + name_len as usize;
                        if end > mem_data.len() {
                            return 0;
                        }
                        match std::str::from_utf8(&mem_data[start..end]) {
                            Ok(s) => s.to_string(),
                            Err(_) => return 0,
                        }
                    };

                    // Read component into buffer
                    let written = {
                        let state = caller.data_mut();
                        let world = unsafe { &*state.world.0 };
                        let bridge = unsafe { &*state.bridge.0 };
                        let buf = &mut state.component_buffer;
                        match bridge.get(world, &name, entity as u32, buf) {
                            Some(n) => n,
                            None => return 0,
                        }
                    };

                    // Write result to WASM memory
                    let cap = result_cap as usize;
                    let len = written.min(cap);
                    let buf_snapshot = caller.data().component_buffer[..len].to_vec();
                    let mem_data = memory.data_mut(&mut caller);
                    let start = result_ptr as usize;
                    let end = start + len;
                    if end > mem_data.len() {
                        return 0;
                    }
                    mem_data[start..end].copy_from_slice(&buf_snapshot);
                    len as i32
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // set_component(entity, name_ptr, name_len, value_ptr, value_len) -> i32
        linker
            .func_wrap(
                "env",
                "set_component",
                |mut caller: Caller<'_, HostState>,
                 entity: i32,
                 name_ptr: i32,
                 name_len: i32,
                 value_ptr: i32,
                 value_len: i32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return 0,
                    };

                    let (name, value) = {
                        let mem_data = memory.data(&caller);
                        let name_start = name_ptr as usize;
                        let name_end = name_start + name_len as usize;
                        let val_start = value_ptr as usize;
                        let val_end = val_start + value_len as usize;
                        if name_end > mem_data.len() || val_end > mem_data.len() {
                            return 0;
                        }
                        let name = match std::str::from_utf8(&mem_data[name_start..name_end]) {
                            Ok(s) => s.to_string(),
                            Err(_) => return 0,
                        };
                        let value = mem_data[val_start..val_end].to_vec();
                        (name, value)
                    };

                    let state = caller.data_mut();
                    let world = unsafe { &mut *state.world.0 };
                    let bridge = unsafe { &*state.bridge.0 };
                    if bridge.set(world, &name, entity as u32, &value) {
                        1
                    } else {
                        0
                    }
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // add_component(entity, name_ptr, name_len, value_ptr, value_len) -> i32
        linker
            .func_wrap(
                "env",
                "add_component",
                |mut caller: Caller<'_, HostState>,
                 entity: i32,
                 name_ptr: i32,
                 name_len: i32,
                 value_ptr: i32,
                 value_len: i32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return 0,
                    };

                    let (name, value) = {
                        let mem_data = memory.data(&caller);
                        let name_start = name_ptr as usize;
                        let name_end = name_start + name_len as usize;
                        let val_start = value_ptr as usize;
                        let val_end = val_start + value_len as usize;
                        if name_end > mem_data.len() || val_end > mem_data.len() {
                            return 0;
                        }
                        let name = match std::str::from_utf8(&mem_data[name_start..name_end]) {
                            Ok(s) => s.to_string(),
                            Err(_) => return 0,
                        };
                        let value = mem_data[val_start..val_end].to_vec();
                        (name, value)
                    };

                    let state = caller.data_mut();
                    let world = unsafe { &mut *state.world.0 };
                    let bridge = unsafe { &*state.bridge.0 };
                    if bridge.add(world, &name, entity as u32, &value) {
                        1
                    } else {
                        0
                    }
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // has_component(entity, name_ptr, name_len) -> i32
        linker
            .func_wrap(
                "env",
                "has_component",
                |mut caller: Caller<'_, HostState>,
                 entity: i32,
                 name_ptr: i32,
                 name_len: i32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return 0,
                    };

                    let name = {
                        let mem_data = memory.data(&caller);
                        let start = name_ptr as usize;
                        let end = start + name_len as usize;
                        if end > mem_data.len() {
                            return 0;
                        }
                        match std::str::from_utf8(&mem_data[start..end]) {
                            Ok(s) => s.to_string(),
                            Err(_) => return 0,
                        }
                    };

                    let state = caller.data();
                    let world = unsafe { &*state.world.0 };
                    let bridge = unsafe { &*state.bridge.0 };

                    let mut buf = [0u8; 1];
                    if bridge.get(world, &name, entity as u32, &mut buf).is_some() {
                        1
                    } else {
                        0
                    }
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // component_size(name_ptr, name_len) -> i32
        linker
            .func_wrap(
                "env",
                "component_size",
                |mut caller: Caller<'_, HostState>, name_ptr: i32, name_len: i32| -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -1,
                    };

                    let name = {
                        let mem_data = memory.data(&caller);
                        let start = name_ptr as usize;
                        let end = start + name_len as usize;
                        if end > mem_data.len() {
                            return -1;
                        }
                        match std::str::from_utf8(&mem_data[start..end]) {
                            Ok(s) => s.to_string(),
                            Err(_) => return -1,
                        }
                    };

                    let state = caller.data();
                    let bridge = unsafe { &*state.bridge.0 };
                    match bridge.size_of(&name) {
                        Some(size) => size as i32,
                        None => -1,
                    }
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // log(ptr, len)
        linker
            .func_wrap(
                "env",
                "log",
                |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return,
                    };
                    let mem_data = memory.data(&caller);
                    let start = ptr as usize;
                    let end = start + len as usize;
                    if end <= mem_data.len()
                        && let Ok(s) = std::str::from_utf8(&mem_data[start..end])
                    {
                        println!("[WASM] {}", s);
                    }
                },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // delta_time() -> f32
        linker
            .func_wrap(
                "env",
                "delta_time",
                |caller: Caller<'_, HostState>| -> f32 { caller.data().delta_time },
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(())
    }

    fn execute(&self, world: &mut World, dt: f32) -> anyhow::Result<()> {
        let bridge = self.bridge.read().unwrap_or_else(|e| e.into_inner());

        let mut store = self.runtime.create_store(dt);
        store.data_mut().world = WorldPtr(world as *mut World);
        store.data_mut().bridge = BridgePtr(&*bridge as *const WasmComponentBridge);

        let mut linker = Linker::new(self.runtime.engine());
        Self::register_host_functions(&mut linker)?;

        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Call the exported update(dt) function if it exists
        if let Some(update_func) = instance.get_func(&mut store, "update") {
            let update = update_func
                .typed::<f32, ()>(&store)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            update
                .call(&mut store, dt)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }

        Ok(())
    }
}

impl engine_ecs::system::System for WasmSystem {
    fn run(&self, world: &mut World) {
        if let Err(e) = self.execute(world, 0.016) {
            eprintln!("[WasmSystem:{}] Error: {}", self.name, e);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_ecs::system::System as _;

    #[derive(Debug, Clone)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }

    fn position_to_bytes(pos: &Position) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&pos.x.to_le_bytes());
        bytes.extend_from_slice(&pos.y.to_le_bytes());
        bytes.extend_from_slice(&pos.z.to_le_bytes());
        bytes
    }

    fn position_from_bytes(bytes: &[u8]) -> Position {
        Position {
            x: f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            y: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            z: f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    }

    fn make_bridge() -> Arc<RwLock<WasmComponentBridge>> {
        let mut bridge = WasmComponentBridge::new();
        bridge.register::<Position>("Position", 12, position_to_bytes, position_from_bytes);
        Arc::new(RwLock::new(bridge))
    }

    #[test]
    fn test_sandbox_defaults() {
        let sandbox = WasmSandbox::default();
        assert_eq!(sandbox.max_memory_bytes, 16 * 1024 * 1024);
        assert_eq!(sandbox.max_fuel, 1_000_000);
    }

    #[test]
    fn test_sandbox_strict() {
        let sandbox = WasmSandbox::strict();
        assert_eq!(sandbox.max_memory_bytes, 4 * 1024 * 1024);
        assert_eq!(sandbox.max_fuel, 100_000);
    }

    #[test]
    fn test_sandbox_relaxed() {
        let sandbox = WasmSandbox::relaxed();
        assert_eq!(sandbox.max_memory_bytes, 64 * 1024 * 1024);
        assert_eq!(sandbox.max_fuel, 100_000_000);
    }

    #[test]
    fn test_component_bridge_register() {
        let bridge = make_bridge();
        let bridge = bridge.read().unwrap();
        assert!(bridge.has("Position"));
        assert!(!bridge.has("Velocity"));
        assert_eq!(bridge.size_of("Position"), Some(12));
        assert_eq!(bridge.registered_names().len(), 1);
    }

    #[test]
    fn test_component_bridge_get_set() {
        let bridge = make_bridge();
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );

        let bridge = bridge.read().unwrap();
        let mut buf = [0u8; 12];
        let written = bridge.get(&world, "Position", e.index(), &mut buf);
        assert_eq!(written, Some(12));

        let pos = position_from_bytes(&buf);
        assert!((pos.x - 1.0).abs() < 0.001);
        assert!((pos.y - 2.0).abs() < 0.001);
        assert!((pos.z - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_component_bridge_add() {
        let bridge = make_bridge();
        let mut world = World::new();
        let e = world.spawn();

        let bridge = bridge.read().unwrap();
        let bytes = position_to_bytes(&Position {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        });
        assert!(bridge.add(&mut world, "Position", e.index(), &bytes));

        let pos = world.get::<Position>(e).unwrap();
        assert!((pos.x - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_wasm_runtime_creation() {
        let runtime = WasmRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_wasm_runtime_with_sandbox() {
        let sandbox = WasmSandbox::strict();
        let runtime = WasmRuntime::with_sandbox(sandbox);
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_wasm_compile_invalid() {
        let runtime = WasmRuntime::new().unwrap();
        let result = runtime.compile(&[0x00, 0x01, 0x02]);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasm_system_minimal() {
        let bridge = make_bridge();
        let runtime = Arc::new(WasmRuntime::new().unwrap());

        // Minimal WASM module with memory and update export
        let wasm = wat::parse_str(
            r#"
            (module
                (import "env" "log" (func $log (param i32 i32)))
                (import "env" "delta_time" (func $delta_time (result f32)))
                (memory (export "memory") 1 16)
                (data (i32.const 0) "hello from wasm")
                (func (export "update") (param $dt f32)
                    (call $log (i32.const 0) (i32.const 15))
                )
            )
            "#,
        )
        .unwrap();

        let system = WasmSystem::new("test", &wasm, runtime, bridge).unwrap();
        let mut world = World::new();
        // Should not panic
        system.run(&mut world);
    }

    #[test]
    fn test_wasm_system_no_update() {
        let bridge = make_bridge();
        let runtime = Arc::new(WasmRuntime::new().unwrap());

        // WASM module without update export
        let wasm = wat::parse_str(
            r#"
            (module
                (memory (export "memory") 1 1)
            )
            "#,
        )
        .unwrap();

        let system = WasmSystem::new("no_update", &wasm, runtime, bridge).unwrap();
        let mut world = World::new();
        // Should not panic, just silently skip
        system.run(&mut world);
    }

    #[test]
    fn test_wasm_system_ecs_interaction() {
        let bridge = make_bridge();
        let runtime = Arc::new(WasmRuntime::new().unwrap());

        // WASM module that spawns an entity and adds a Position component
        let wasm = wat::parse_str(
            r#"
            (module
                (import "env" "spawn" (func $spawn (result i32)))
                (import "env" "add_component" (func $add_component (param i32 i32 i32 i32 i32) (result i32)))
                (import "env" "log" (func $log (param i32 i32)))
                (import "env" "delta_time" (func $delta_time (result f32)))

                (memory (export "memory") 1 16)

                ;; "Position" at offset 0 (8 bytes)
                (data (i32.const 0) "Position")
                ;; Component data at offset 256: x=1.0, y=2.0, z=3.0 (little-endian f32)
                (data (i32.const 256) "\00\00\80\3F\00\00\00\40\00\00\40\40")

                (func (export "update") (param $dt f32)
                    (local $entity i32)
                    ;; Spawn entity
                    (local.set $entity (call $spawn))
                    ;; Add Position component
                    (drop (call $add_component
                        (local.get $entity)  ;; entity
                        (i32.const 0)        ;; "Position" ptr
                        (i32.const 8)        ;; "Position" len
                        (i32.const 256)      ;; value ptr
                        (i32.const 12)       ;; value len (3 * f32)
                    ))
                )
            )
            "#,
        )
        .unwrap();

        let system = WasmSystem::new("ecs_test", &wasm, runtime, bridge).unwrap();
        let mut world = World::new();
        system.run(&mut world);

        // Verify the entity was created with the Position component
        // Entity 0 should have Position(1.0, 2.0, 3.0)
        let pos = world.get_by_index::<Position>(0);
        assert!(pos.is_some());
        let pos = pos.unwrap();
        assert!((pos.x - 1.0).abs() < 0.001);
        assert!((pos.y - 2.0).abs() < 0.001);
        assert!((pos.z - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_wasm_fuel_exhaustion() {
        let bridge = make_bridge();
        let sandbox = WasmSandbox {
            max_fuel: 100, // Very low fuel
            ..Default::default()
        };
        let runtime = Arc::new(WasmRuntime::with_sandbox(sandbox).unwrap());

        // WASM module with an infinite loop
        let wasm = wat::parse_str(
            r#"
            (module
                (memory (export "memory") 1 1)
                (func (export "update") (param $dt f32)
                    (loop $inf
                        (br $inf)
                    )
                )
            )
            "#,
        )
        .unwrap();

        let system = WasmSystem::new("fuel_test", &wasm, runtime, bridge).unwrap();
        let mut world = World::new();
        // Should return an error (fuel exhausted) but not panic
        let result = system.execute(&mut world, 0.016);
        assert!(result.is_err());
    }
}
