use crate::bridge::ComponentBridge;
use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use mlua::prelude::*;
use std::sync::{Arc, RwLock};

/// Wrapper around `Lua` that is `Send + Sync`.
struct SendLua(Lua);
unsafe impl Send for SendLua {}
unsafe impl Sync for SendLua {}

/// Send-safe wrapper for `*mut World`.
struct WorldPtr(*mut World);
unsafe impl Send for WorldPtr {}
unsafe impl Sync for WorldPtr {}

/// Send-safe wrapper for `*const ComponentBridge`.
struct BridgePtr(*const ComponentBridge);
unsafe impl Send for BridgePtr {}
unsafe impl Sync for BridgePtr {}

/// A system that executes a Lua script each tick.
///
/// The script's `update(dt)` function is called with a `world` table
/// providing entity/component operations.
///
/// # Lua API
///
/// - `world:spawn()` → entity_index (u32)
/// - `world:despawn(entity_index)`
/// - `world:get(entity_index, "ComponentName")` → value | nil
/// - `world:set(entity_index, "ComponentName", value)`
/// - `world:add(entity_index, "ComponentName", value)`
/// - `world:has(entity_index, "ComponentName")` → boolean
/// - `world:entities("ComponentName")` → {entity_index, ...}
/// - `world:delta_time()` → float
pub struct ScriptSystem {
    name: String,
    lua: SendLua,
    bridge: Arc<RwLock<ComponentBridge>>,
}

impl ScriptSystem {
    /// Create a new Lua script system by compiling `source` inline.
    ///
    /// The source is loaded and executed once at creation time. The global
    /// `update(dt)` function (if defined) will be called each tick via
    /// the [`System`](engine_ecs::system::System) trait.
    pub fn new(
        name: impl Into<String>,
        source: &str,
        bridge: Arc<RwLock<ComponentBridge>>,
    ) -> LuaResult<Self> {
        let lua = Lua::new();
        Self::setup_globals(&lua)?;
        lua.load(source).exec()?;
        Ok(Self {
            name: name.into(),
            lua: SendLua(lua),
            bridge,
        })
    }

    /// Create a new Lua script system by reading source from a file path.
    ///
    /// This is a convenience wrapper around [`new`](Self::new) that reads
    /// the file contents for you.
    pub fn from_file(
        name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
        bridge: Arc<RwLock<ComponentBridge>>,
    ) -> LuaResult<Self> {
        let source = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            LuaError::runtime(format!("Failed to read {}: {}", path.as_ref().display(), e))
        })?;
        Self::new(name, &source, bridge)
    }

    /// Replace the Lua state with freshly compiled `source`.
    ///
    /// The previous Lua state is discarded. This is used by the
    /// [`HotReloader`](crate::hot_reload::HotReloader) to apply file changes.
    pub fn reload(&mut self, source: &str) -> LuaResult<()> {
        let lua = Lua::new();
        Self::setup_globals(&lua)?;
        lua.load(source).exec()?;
        self.lua = SendLua(lua);
        Ok(())
    }

    /// Reload the Lua state from a file path.
    ///
    /// Convenience wrapper around [`reload`](Self::reload).
    pub fn reload_from_file(&mut self, path: impl AsRef<std::path::Path>) -> LuaResult<()> {
        let source = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            LuaError::runtime(format!("Failed to read {}: {}", path.as_ref().display(), e))
        })?;
        self.reload(&source)
    }

    /// Return the human-readable name assigned to this system.
    pub fn script_name(&self) -> &str {
        &self.name
    }

    fn setup_globals(lua: &Lua) -> LuaResult<()> {
        lua.globals().set(
            "print",
            lua.create_function(|_, args: LuaMultiValue| {
                let parts: Vec<String> = args.iter().map(|a| format!("{:?}", a)).collect();
                println!("[Lua] {}", parts.join("\t"));
                Ok(())
            })?,
        )?;
        Ok(())
    }

    fn execute(&self, world: &mut World, dt: f32) -> LuaResult<()> {
        let bridge = self.bridge.read().unwrap_or_else(|e| e.into_inner());
        let lua = &self.lua.0;

        // Store Send-wrapped pointers as app data
        lua.set_app_data(WorldPtr(world as *mut World));
        lua.set_app_data(BridgePtr(&*bridge as *const ComponentBridge));

        let world_table = lua.create_table()?;

        // world:spawn() → entity_index
        world_table.set(
            "spawn",
            lua.create_function_mut(|lua, ()| {
                let wp = lua
                    .app_data_ref::<WorldPtr>()
                    .ok_or_else(|| LuaError::runtime("World not available"))?;
                let world = unsafe { &mut *wp.0 };
                Ok(world.spawn().index())
            })?,
        )?;

        // world:despawn(entity_index)
        world_table.set(
            "despawn",
            lua.create_function_mut(|lua, idx: u32| {
                let wp = lua
                    .app_data_ref::<WorldPtr>()
                    .ok_or_else(|| LuaError::runtime("World not available"))?;
                let world = unsafe { &mut *wp.0 };
                world.despawn(Entity::new(idx, 0));
                Ok(())
            })?,
        )?;

        // world:get(entity_index, "ComponentName") → value | nil
        world_table.set(
            "get",
            lua.create_function(|lua, (idx, name): (u32, String)| {
                let wp = lua
                    .app_data_ref::<WorldPtr>()
                    .ok_or_else(|| LuaError::runtime("World not available"))?;
                let bp = lua
                    .app_data_ref::<BridgePtr>()
                    .ok_or_else(|| LuaError::runtime("Bridge not available"))?;
                let world = unsafe { &*wp.0 };
                let bridge = unsafe { &*bp.0 };
                match bridge.get(lua, world, &name, idx) {
                    Ok(Some(val)) => Ok(val),
                    Ok(None) => Ok(LuaValue::Nil),
                    Err(e) => Err(e),
                }
            })?,
        )?;

        // world:set(entity_index, "ComponentName", value)
        world_table.set(
            "set",
            lua.create_function_mut(|lua, (idx, name, value): (u32, String, LuaValue)| {
                let wp = lua
                    .app_data_ref::<WorldPtr>()
                    .ok_or_else(|| LuaError::runtime("World not available"))?;
                let bp = lua
                    .app_data_ref::<BridgePtr>()
                    .ok_or_else(|| LuaError::runtime("Bridge not available"))?;
                let world = unsafe { &mut *wp.0 };
                let bridge = unsafe { &*bp.0 };
                bridge.set(lua, world, &name, idx, &value)
            })?,
        )?;

        // world:add(entity_index, "ComponentName", value)
        world_table.set(
            "add",
            lua.create_function_mut(|lua, (idx, name, value): (u32, String, LuaValue)| {
                let wp = lua
                    .app_data_ref::<WorldPtr>()
                    .ok_or_else(|| LuaError::runtime("World not available"))?;
                let bp = lua
                    .app_data_ref::<BridgePtr>()
                    .ok_or_else(|| LuaError::runtime("Bridge not available"))?;
                let world = unsafe { &mut *wp.0 };
                let bridge = unsafe { &*bp.0 };
                bridge.add(lua, world, &name, idx, &value)
            })?,
        )?;

        // world:has(entity_index, "ComponentName") → boolean
        world_table.set(
            "has",
            lua.create_function(|lua, (idx, name): (u32, String)| {
                let wp = lua
                    .app_data_ref::<WorldPtr>()
                    .ok_or_else(|| LuaError::runtime("World not available"))?;
                let bp = lua
                    .app_data_ref::<BridgePtr>()
                    .ok_or_else(|| LuaError::runtime("Bridge not available"))?;
                let world = unsafe { &*wp.0 };
                let bridge = unsafe { &*bp.0 };
                match bridge.get(lua, world, &name, idx) {
                    Ok(Some(_)) => Ok(true),
                    Ok(None) => Ok(false),
                    Err(_) => Ok(false),
                }
            })?,
        )?;

        // world:entities("ComponentName") → {entity_index, ...}
        world_table.set(
            "entities",
            lua.create_function(|lua, name: String| {
                let wp = lua
                    .app_data_ref::<WorldPtr>()
                    .ok_or_else(|| LuaError::runtime("World not available"))?;
                let bp = lua
                    .app_data_ref::<BridgePtr>()
                    .ok_or_else(|| LuaError::runtime("Bridge not available"))?;
                let world = unsafe { &*wp.0 };
                let bridge = unsafe { &*bp.0 };
                let table = lua.create_table()?;
                let mut count = 0;
                for idx in 0..1024u32 {
                    if bridge.get(lua, world, &name, idx)?.is_some() {
                        count += 1;
                        table.set(count, idx)?;
                    }
                }
                Ok(table)
            })?,
        )?;

        // world:delta_time() → float
        world_table.set("delta_time", lua.create_function(move |_, ()| Ok(dt))?)?;

        lua.globals().set("world", world_table)?;

        // Call update(dt) if defined
        let result: LuaResult<LuaFunction> = lua.globals().get("update");
        if let Ok(update_fn) = result {
            update_fn.call::<()>(dt)?;
        }

        // Clean up
        lua.remove_app_data::<WorldPtr>();
        lua.remove_app_data::<BridgePtr>();

        Ok(())
    }
}

impl engine_ecs::system::System for ScriptSystem {
    fn run(&self, world: &mut World) {
        if let Err(e) = self.execute(world, 0.016) {
            eprintln!("[ScriptSystem:{}] Lua error: {}", self.name, e);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
