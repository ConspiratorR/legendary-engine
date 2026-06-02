use engine_ecs::world::World;
use mlua::prelude::*;
use std::any::TypeId;
use std::collections::HashMap;

/// Type-erased getter: reads a component from the world by entity index
/// and converts it into a Lua value.
pub type GetFn = Box<dyn Fn(&Lua, &World, u32) -> LuaResult<Option<LuaValue>> + Send + Sync>;

/// Type-erased setter: reads a Lua value and writes it into the component
/// storage for the given entity index.
pub type SetFn = Box<dyn Fn(&Lua, &mut World, u32, &LuaValue) -> LuaResult<()> + Send + Sync>;

/// Type-erased factory: creates a default component from a Lua table
/// and inserts it into the world for the given entity index.
pub type AddFn = Box<dyn Fn(&Lua, &mut World, u32, &LuaValue) -> LuaResult<()> + Send + Sync>;

/// Registry mapping human-readable component names to their Lua bridge closures.
///
/// Register each component type you want to expose to Lua with
/// [`register`], [`register_set`], and optionally [`register_add`].
pub struct ComponentBridge {
    getters: HashMap<String, GetFn>,
    setters: HashMap<String, SetFn>,
    adders: HashMap<String, AddFn>,
    type_ids: HashMap<String, TypeId>,
}

impl ComponentBridge {
    pub fn new() -> Self {
        Self {
            getters: HashMap::new(),
            setters: HashMap::new(),
            adders: HashMap::new(),
            type_ids: HashMap::new(),
        }
    }

    /// Register a read-only component binding.
    ///
    /// `getter` converts `&T` → `LuaValue`.
    pub fn register_get<T: Send + Sync + 'static>(
        &mut self,
        name: impl Into<String>,
        getter: impl Fn(&Lua, &T) -> LuaResult<LuaValue> + Send + Sync + 'static,
    ) {
        let name = name.into();
        self.type_ids.insert(name.clone(), TypeId::of::<T>());
        self.getters.insert(
            name,
            Box::new(move |lua, world, idx| {
                world
                    .get_by_index::<T>(idx)
                    .map(|c| getter(lua, c))
                    .transpose()
            }),
        );
    }

    /// Register a writable component binding.
    ///
    /// `setter` applies a `LuaValue` to `&mut T`.
    pub fn register_set<T: Send + Sync + 'static>(
        &mut self,
        name: impl Into<String>,
        setter: impl Fn(&Lua, &mut T, &LuaValue) -> LuaResult<()> + Send + Sync + 'static,
    ) {
        let name = name.into();
        self.setters.insert(
            name,
            Box::new(move |lua, world, idx, val| {
                if let Some(c) = world.get_by_index_mut::<T>(idx) {
                    setter(lua, c, val)?;
                }
                Ok(())
            }),
        );
    }

    /// Register a factory for creating components from Lua values.
    ///
    /// `factory` creates a `T` from a Lua value (usually a table).
    pub fn register_add<T: Send + Sync + 'static>(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(&Lua, &LuaValue) -> LuaResult<T> + Send + Sync + 'static,
    ) {
        let name = name.into();
        self.type_ids.insert(name.clone(), TypeId::of::<T>());
        self.adders.insert(
            name,
            Box::new(move |lua, world, idx, val| {
                let component = factory(lua, val)?;
                // We need to convert the entity index back to an Entity handle.
                // Since we only have the index, we'll use add_component by index.
                // The world's add_component takes an Entity, but we can construct one.
                let entity = engine_ecs::entity::Entity::new(idx, 0);
                world.add_component(entity, component);
                Ok(())
            }),
        );
    }

    /// Get a component value as a Lua value.
    pub fn get(
        &self,
        lua: &Lua,
        world: &World,
        name: &str,
        idx: u32,
    ) -> LuaResult<Option<LuaValue>> {
        match self.getters.get(name) {
            Some(f) => f(lua, world, idx),
            None => Ok(None),
        }
    }

    /// Set a component from a Lua value.
    pub fn set(
        &self,
        lua: &Lua,
        world: &mut World,
        name: &str,
        idx: u32,
        val: &LuaValue,
    ) -> LuaResult<()> {
        if let Some(f) = self.setters.get(name) {
            f(lua, world, idx, val)?;
        }
        Ok(())
    }

    /// Add a component from a Lua value.
    pub fn add(
        &self,
        lua: &Lua,
        world: &mut World,
        name: &str,
        idx: u32,
        val: &LuaValue,
    ) -> LuaResult<()> {
        if let Some(f) = self.adders.get(name) {
            f(lua, world, idx, val)?;
        }
        Ok(())
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

impl Default for ComponentBridge {
    fn default() -> Self {
        Self::new()
    }
}
