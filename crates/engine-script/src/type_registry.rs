//! Unified type mapping layer for Rust ↔ Lua/WASM conversion.
//!
//! The [`TypeRegistry`] provides bidirectional conversion between engine
//! math types and their script-side representations:
//!
//! | Rust Type | Lua | WASM |
//! |-----------|-----|------|
//! | `Vec2`    | `{x, y}` | 8 bytes (2×f32 LE) |
//! | `Vec3`    | `{x, y, z}` | 12 bytes (3×f32 LE) |
//! | `Vec4`    | `{x, y, z, w}` | 16 bytes (4×f32 LE) |
//! | `Quat`    | `{x, y, z, w}` | 16 bytes (4×f32 LE) |
//! | `Color`   | `{r, g, b, a}` | 16 bytes (4×f32 LE) |
//! | `Transform` | `{position, rotation, scale}` | 36 bytes (9×f32 LE) |
//! | `bool`    | boolean | 1 byte |
//! | `i32`     | integer | 4 bytes (i32 LE) |
//! | `u32`     | integer | 4 bytes (u32 LE) |
//! | `f32`     | number | 4 bytes (f32 LE) |
//! | `f64`     | number | 8 bytes (f64 LE) |
//! | `String`  | string | N bytes (length-prefixed) |

use engine_core::color::Color;
use engine_core::transform::Transform;
use engine_math::{Quat, Vec2, Vec3, Vec4};
use mlua::prelude::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Lua conversion helpers
// ---------------------------------------------------------------------------

/// Create a Lua table representing a `Vec2`.
pub fn vec2_to_lua(lua: &Lua, v: Vec2) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("x", v.x)?;
    t.set("y", v.y)?;
    Ok(t)
}

/// Read a `Vec2` from a Lua table.
pub fn lua_to_vec2(t: &LuaTable) -> LuaResult<Vec2> {
    Ok(Vec2::new(t.get("x")?, t.get("y")?))
}

/// Create a Lua table representing a `Vec3`.
pub fn vec3_to_lua(lua: &Lua, v: Vec3) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("x", v.x)?;
    t.set("y", v.y)?;
    t.set("z", v.z)?;
    Ok(t)
}

/// Read a `Vec3` from a Lua table.
pub fn lua_to_vec3(t: &LuaTable) -> LuaResult<Vec3> {
    Ok(Vec3::new(t.get("x")?, t.get("y")?, t.get("z")?))
}

/// Create a Lua table representing a `Vec4`.
pub fn vec4_to_lua(lua: &Lua, v: Vec4) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("x", v.x)?;
    t.set("y", v.y)?;
    t.set("z", v.z)?;
    t.set("w", v.w)?;
    Ok(t)
}

/// Read a `Vec4` from a Lua table.
pub fn lua_to_vec4(t: &LuaTable) -> LuaResult<Vec4> {
    Ok(Vec4::new(
        t.get("x")?,
        t.get("y")?,
        t.get("z")?,
        t.get("w")?,
    ))
}

/// Create a Lua table representing a `Quat`.
pub fn quat_to_lua(lua: &Lua, q: Quat) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("x", q.x)?;
    t.set("y", q.y)?;
    t.set("z", q.z)?;
    t.set("w", q.w)?;
    Ok(t)
}

/// Read a `Quat` from a Lua table.
pub fn lua_to_quat(t: &LuaTable) -> LuaResult<Quat> {
    Ok(Quat::from_xyzw(
        t.get("x")?,
        t.get("y")?,
        t.get("z")?,
        t.get("w")?,
    ))
}

/// Create a Lua table representing a `Color`.
pub fn color_to_lua(lua: &Lua, c: Color) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("r", c.r)?;
    t.set("g", c.g)?;
    t.set("b", c.b)?;
    t.set("a", c.a)?;
    Ok(t)
}

/// Read a `Color` from a Lua table.
pub fn lua_to_color(t: &LuaTable) -> LuaResult<Color> {
    Ok(Color::new(
        t.get("r")?,
        t.get("g")?,
        t.get("b")?,
        t.get("a")?,
    ))
}

/// Create a Lua table representing a `Transform`.
pub fn transform_to_lua(lua: &Lua, tr: Transform) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("position", vec3_to_lua(lua, tr.position)?)?;
    t.set("rotation", vec3_to_lua(lua, tr.rotation)?)?;
    t.set("scale", vec3_to_lua(lua, tr.scale)?)?;
    Ok(t)
}

/// Read a `Transform` from a Lua table.
pub fn lua_to_transform(t: &LuaTable) -> LuaResult<Transform> {
    let pos: LuaTable = t.get("position")?;
    let rot: LuaTable = t.get("rotation")?;
    let scl: LuaTable = t.get("scale")?;
    Ok(Transform {
        position: lua_to_vec3(&pos)?,
        rotation: lua_to_vec3(&rot)?,
        scale: lua_to_vec3(&scl)?,
    })
}

// ---------------------------------------------------------------------------
// WASM byte conversion helpers
// ---------------------------------------------------------------------------

fn write_f32_le(buf: &mut [u8], offset: usize, v: f32) {
    buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

fn read_f32_le(buf: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

/// Serialize a `Vec2` to 8 bytes (little-endian f32×2).
pub fn vec2_to_bytes(v: &Vec2) -> [u8; 8] {
    let mut buf = [0u8; 8];
    write_f32_le(&mut buf, 0, v.x);
    write_f32_le(&mut buf, 4, v.y);
    buf
}

/// Deserialize a `Vec2` from 8 bytes.
pub fn bytes_to_vec2(buf: &[u8]) -> Vec2 {
    Vec2::new(read_f32_le(buf, 0), read_f32_le(buf, 4))
}

/// Serialize a `Vec3` to 12 bytes (little-endian f32×3).
pub fn vec3_to_bytes(v: &Vec3) -> [u8; 12] {
    let mut buf = [0u8; 12];
    write_f32_le(&mut buf, 0, v.x);
    write_f32_le(&mut buf, 4, v.y);
    write_f32_le(&mut buf, 8, v.z);
    buf
}

/// Deserialize a `Vec3` from 12 bytes.
pub fn bytes_to_vec3(buf: &[u8]) -> Vec3 {
    Vec3::new(
        read_f32_le(buf, 0),
        read_f32_le(buf, 4),
        read_f32_le(buf, 8),
    )
}

/// Serialize a `Vec4` to 16 bytes (little-endian f32×4).
pub fn vec4_to_bytes(v: &Vec4) -> [u8; 16] {
    let mut buf = [0u8; 16];
    write_f32_le(&mut buf, 0, v.x);
    write_f32_le(&mut buf, 4, v.y);
    write_f32_le(&mut buf, 8, v.z);
    write_f32_le(&mut buf, 12, v.w);
    buf
}

/// Deserialize a `Vec4` from 16 bytes.
pub fn bytes_to_vec4(buf: &[u8]) -> Vec4 {
    Vec4::new(
        read_f32_le(buf, 0),
        read_f32_le(buf, 4),
        read_f32_le(buf, 8),
        read_f32_le(buf, 12),
    )
}

/// Serialize a `Quat` to 16 bytes (little-endian f32×4).
pub fn quat_to_bytes(q: &Quat) -> [u8; 16] {
    let mut buf = [0u8; 16];
    write_f32_le(&mut buf, 0, q.x);
    write_f32_le(&mut buf, 4, q.y);
    write_f32_le(&mut buf, 8, q.z);
    write_f32_le(&mut buf, 12, q.w);
    buf
}

/// Deserialize a `Quat` from 16 bytes.
pub fn bytes_to_quat(buf: &[u8]) -> Quat {
    Quat::from_xyzw(
        read_f32_le(buf, 0),
        read_f32_le(buf, 4),
        read_f32_le(buf, 8),
        read_f32_le(buf, 12),
    )
}

/// Serialize a `Color` to 16 bytes (little-endian f32×4).
pub fn color_to_bytes(c: &Color) -> [u8; 16] {
    let mut buf = [0u8; 16];
    write_f32_le(&mut buf, 0, c.r);
    write_f32_le(&mut buf, 4, c.g);
    write_f32_le(&mut buf, 8, c.b);
    write_f32_le(&mut buf, 12, c.a);
    buf
}

/// Deserialize a `Color` from 16 bytes.
pub fn bytes_to_color(buf: &[u8]) -> Color {
    Color::new(
        read_f32_le(buf, 0),
        read_f32_le(buf, 4),
        read_f32_le(buf, 8),
        read_f32_le(buf, 12),
    )
}

/// Serialize a `Transform` to 36 bytes (9×f32 LE: position + rotation + scale).
pub fn transform_to_bytes(t: &Transform) -> [u8; 36] {
    let mut buf = [0u8; 36];
    write_f32_le(&mut buf, 0, t.position.x);
    write_f32_le(&mut buf, 4, t.position.y);
    write_f32_le(&mut buf, 8, t.position.z);
    write_f32_le(&mut buf, 12, t.rotation.x);
    write_f32_le(&mut buf, 16, t.rotation.y);
    write_f32_le(&mut buf, 20, t.rotation.z);
    write_f32_le(&mut buf, 24, t.scale.x);
    write_f32_le(&mut buf, 28, t.scale.y);
    write_f32_le(&mut buf, 32, t.scale.z);
    buf
}

/// Deserialize a `Transform` from 36 bytes.
pub fn bytes_to_transform(buf: &[u8]) -> Transform {
    Transform {
        position: Vec3::new(
            read_f32_le(buf, 0),
            read_f32_le(buf, 4),
            read_f32_le(buf, 8),
        ),
        rotation: Vec3::new(
            read_f32_le(buf, 12),
            read_f32_le(buf, 16),
            read_f32_le(buf, 20),
        ),
        scale: Vec3::new(
            read_f32_le(buf, 24),
            read_f32_le(buf, 28),
            read_f32_le(buf, 32),
        ),
    }
}

// ---------------------------------------------------------------------------
// Type registry
// ---------------------------------------------------------------------------

/// Type-erased Lua getter: reads from a World and returns a LuaValue.
type LuaGetFn =
    Box<dyn Fn(&Lua, &engine_ecs::world::World, u32) -> LuaResult<Option<LuaValue>> + Send + Sync>;

/// Type-erased Lua setter: applies a LuaValue to a component in the World.
type LuaSetFn =
    Box<dyn Fn(&Lua, &mut engine_ecs::world::World, u32, &LuaValue) -> LuaResult<()> + Send + Sync>;

/// Type-erased Lua factory: creates a component from a LuaValue.
type LuaAddFn =
    Box<dyn Fn(&Lua, &mut engine_ecs::world::World, u32, &LuaValue) -> LuaResult<()> + Send + Sync>;

/// Type-erased WASM getter: reads a component into a byte buffer.
type WasmGetFn =
    Box<dyn Fn(&engine_ecs::world::World, u32, &mut [u8]) -> Option<usize> + Send + Sync>;

/// Type-erased WASM setter: writes bytes into a component.
type WasmSetFn = Box<dyn Fn(&mut engine_ecs::world::World, u32, &[u8]) -> bool + Send + Sync>;

/// Type-erased WASM factory: creates a component from bytes.
type WasmAddFn = Box<dyn Fn(&mut engine_ecs::world::World, u32, &[u8]) -> bool + Send + Sync>;

/// Entry for a registered type in the registry.
struct TypeEntry {
    lua_get: LuaGetFn,
    lua_set: LuaSetFn,
    lua_add: LuaAddFn,
    wasm_get: WasmGetFn,
    wasm_set: WasmSetFn,
    wasm_add: WasmAddFn,
    wasm_size: usize,
}

/// Unified registry mapping Rust component types to both Lua and WASM bridges.
///
/// Register engine types once, then use the same registry for both Lua
/// [`ScriptSystem`](crate::system::ScriptSystem) and WASM
/// [`WasmSystem`](crate::wasm::WasmSystem) execution.
///
/// # Example
///
/// ```rust
/// use engine_script::type_registry::TypeRegistry;
///
/// let mut registry = TypeRegistry::new();
/// registry.register_all_engine_types();
///
/// assert!(registry.has("Vec3"));
/// assert!(registry.has("Color"));
/// assert_eq!(registry.wasm_size("Vec3"), Some(12));
/// ```
pub struct TypeRegistry {
    entries: HashMap<String, TypeEntry>,
}

impl TypeRegistry {
    /// Create an empty type registry.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register all built-in engine types (Vec2, Vec3, Vec4, Quat, Color, Transform,
    /// plus primitive types bool, i32, u32, f32, f64, String).
    pub fn register_all_engine_types(&mut self) {
        self.register_vec2();
        self.register_vec3();
        self.register_vec4();
        self.register_quat();
        self.register_color();
        self.register_transform();
        self.register_f32();
        self.register_f64();
        self.register_i32();
        self.register_u32();
        self.register_bool();
    }

    /// Check if a type name is registered.
    pub fn has(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Get the WASM byte size for a registered type.
    pub fn wasm_size(&self, name: &str) -> Option<usize> {
        self.entries.get(name).map(|e| e.wasm_size)
    }

    /// Get all registered type names.
    pub fn registered_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// Read a component as a Lua value.
    pub fn lua_get(
        &self,
        lua: &Lua,
        world: &engine_ecs::world::World,
        type_name: &str,
        entity_idx: u32,
    ) -> LuaResult<Option<LuaValue>> {
        match self.entries.get(type_name) {
            Some(entry) => (entry.lua_get)(lua, world, entity_idx),
            None => Ok(None),
        }
    }

    /// Write a component from a Lua value.
    pub fn lua_set(
        &self,
        lua: &Lua,
        world: &mut engine_ecs::world::World,
        type_name: &str,
        entity_idx: u32,
        value: &LuaValue,
    ) -> LuaResult<()> {
        if let Some(entry) = self.entries.get(type_name) {
            (entry.lua_set)(lua, world, entity_idx, value)?;
        }
        Ok(())
    }

    /// Add a component from a Lua value.
    pub fn lua_add(
        &self,
        lua: &Lua,
        world: &mut engine_ecs::world::World,
        type_name: &str,
        entity_idx: u32,
        value: &LuaValue,
    ) -> LuaResult<()> {
        if let Some(entry) = self.entries.get(type_name) {
            (entry.lua_add)(lua, world, entity_idx, value)?;
        }
        Ok(())
    }

    /// Read a component into a WASM byte buffer.
    pub fn wasm_get(
        &self,
        world: &engine_ecs::world::World,
        type_name: &str,
        entity_idx: u32,
        buf: &mut [u8],
    ) -> Option<usize> {
        match self.entries.get(type_name) {
            Some(entry) => (entry.wasm_get)(world, entity_idx, buf),
            None => None,
        }
    }

    /// Write a component from WASM bytes.
    pub fn wasm_set(
        &self,
        world: &mut engine_ecs::world::World,
        type_name: &str,
        entity_idx: u32,
        bytes: &[u8],
    ) -> bool {
        match self.entries.get(type_name) {
            Some(entry) => (entry.wasm_set)(world, entity_idx, bytes),
            None => false,
        }
    }

    /// Add a component from WASM bytes.
    pub fn wasm_add(
        &self,
        world: &mut engine_ecs::world::World,
        type_name: &str,
        entity_idx: u32,
        bytes: &[u8],
    ) -> bool {
        match self.entries.get(type_name) {
            Some(entry) => (entry.wasm_add)(world, entity_idx, bytes),
            None => false,
        }
    }

    // -----------------------------------------------------------------------
    // Internal registration helpers
    // -----------------------------------------------------------------------

    fn register_type<T: Send + Sync + Copy + 'static>(
        &mut self,
        name: impl Into<String>,
        wasm_size: usize,
        to_lua: fn(&Lua, T) -> LuaResult<LuaValue>,
        from_lua: fn(&Lua, &LuaValue) -> LuaResult<T>,
        to_bytes: fn(&T) -> Vec<u8>,
        from_bytes: fn(&[u8]) -> T,
    ) {
        let name = name.into();
        self.entries.insert(
            name,
            TypeEntry {
                lua_get: Box::new(move |lua, world, idx| match world.get_by_index::<T>(idx) {
                    Some(val) => Ok(Some(to_lua(lua, *val)?)),
                    None => Ok(None),
                }),
                lua_set: Box::new(move |lua, world, idx, lua_val| {
                    if let Some(val) = world.get_by_index_mut::<T>(idx) {
                        *val = from_lua(lua, lua_val)?;
                    }
                    Ok(())
                }),
                lua_add: Box::new(move |lua, world, idx, lua_val| {
                    let val = from_lua(lua, lua_val)?;
                    let entity = engine_ecs::entity::Entity::new(idx, 0);
                    world.add_component(entity, val);
                    Ok(())
                }),
                wasm_get: Box::new(move |world, idx, buf| {
                    let comp = world.get_by_index::<T>(idx)?;
                    let bytes = to_bytes(comp);
                    let len = bytes.len().min(buf.len());
                    buf[..len].copy_from_slice(&bytes[..len]);
                    Some(len)
                }),
                wasm_set: Box::new(move |world, idx, bytes| {
                    if let Some(c) = world.get_by_index_mut::<T>(idx) {
                        *c = from_bytes(bytes);
                        true
                    } else {
                        false
                    }
                }),
                wasm_add: Box::new(move |world, idx, bytes| {
                    let comp = from_bytes(bytes);
                    let entity = engine_ecs::entity::Entity::new(idx, 0);
                    world.add_component(entity, comp);
                    true
                }),
                wasm_size,
            },
        );
    }

    fn register_vec2(&mut self) {
        self.register_type::<Vec2>(
            "Vec2",
            8,
            |lua, v| Ok(LuaValue::Table(vec2_to_lua(lua, v)?)),
            |_lua, lv| {
                if let LuaValue::Table(t) = lv {
                    lua_to_vec2(t)
                } else {
                    Err(LuaError::runtime("expected table for Vec2"))
                }
            },
            |v| vec2_to_bytes(v).to_vec(),
            bytes_to_vec2,
        );
    }

    fn register_vec3(&mut self) {
        self.register_type::<Vec3>(
            "Vec3",
            12,
            |lua, v| Ok(LuaValue::Table(vec3_to_lua(lua, v)?)),
            |_lua, lv| {
                if let LuaValue::Table(t) = lv {
                    lua_to_vec3(t)
                } else {
                    Err(LuaError::runtime("expected table for Vec3"))
                }
            },
            |v| vec3_to_bytes(v).to_vec(),
            bytes_to_vec3,
        );
    }

    fn register_vec4(&mut self) {
        self.register_type::<Vec4>(
            "Vec4",
            16,
            |lua, v| Ok(LuaValue::Table(vec4_to_lua(lua, v)?)),
            |_lua, lv| {
                if let LuaValue::Table(t) = lv {
                    lua_to_vec4(t)
                } else {
                    Err(LuaError::runtime("expected table for Vec4"))
                }
            },
            |v| vec4_to_bytes(v).to_vec(),
            bytes_to_vec4,
        );
    }

    fn register_quat(&mut self) {
        self.register_type::<Quat>(
            "Quat",
            16,
            |lua, q| Ok(LuaValue::Table(quat_to_lua(lua, q)?)),
            |_lua, lv| {
                if let LuaValue::Table(t) = lv {
                    lua_to_quat(t)
                } else {
                    Err(LuaError::runtime("expected table for Quat"))
                }
            },
            |q| quat_to_bytes(q).to_vec(),
            bytes_to_quat,
        );
    }

    fn register_color(&mut self) {
        self.register_type::<Color>(
            "Color",
            16,
            |lua, c| Ok(LuaValue::Table(color_to_lua(lua, c)?)),
            |_lua, lv| {
                if let LuaValue::Table(t) = lv {
                    lua_to_color(t)
                } else {
                    Err(LuaError::runtime("expected table for Color"))
                }
            },
            |c| color_to_bytes(c).to_vec(),
            bytes_to_color,
        );
    }

    fn register_transform(&mut self) {
        self.register_type::<Transform>(
            "Transform",
            36,
            |lua, tr| Ok(LuaValue::Table(transform_to_lua(lua, tr)?)),
            |_lua, lv| {
                if let LuaValue::Table(t) = lv {
                    lua_to_transform(t)
                } else {
                    Err(LuaError::runtime("expected table for Transform"))
                }
            },
            |tr| transform_to_bytes(tr).to_vec(),
            bytes_to_transform,
        );
    }

    fn register_f32(&mut self) {
        self.register_type::<f32>(
            "f32",
            4,
            |_, v| Ok(LuaValue::Number(v as f64)),
            |_, lv| {
                if let LuaValue::Number(n) = lv {
                    Ok(*n as f32)
                } else {
                    Err(LuaError::runtime("expected number for f32"))
                }
            },
            |v| v.to_le_bytes().to_vec(),
            |buf| f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        );
    }

    fn register_f64(&mut self) {
        self.register_type::<f64>(
            "f64",
            8,
            |_, v| Ok(LuaValue::Number(v)),
            |_, lv| {
                if let LuaValue::Number(n) = lv {
                    Ok(*n)
                } else {
                    Err(LuaError::runtime("expected number for f64"))
                }
            },
            |v| v.to_le_bytes().to_vec(),
            |buf| {
                f64::from_le_bytes([
                    buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
                ])
            },
        );
    }

    fn register_i32(&mut self) {
        self.register_type::<i32>(
            "i32",
            4,
            |_, v| Ok(LuaValue::Integer(v as i64)),
            |_, lv| {
                if let LuaValue::Integer(i) = lv {
                    Ok(*i as i32)
                } else {
                    Err(LuaError::runtime("expected integer for i32"))
                }
            },
            |v| v.to_le_bytes().to_vec(),
            |buf| i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        );
    }

    fn register_u32(&mut self) {
        self.register_type::<u32>(
            "u32",
            4,
            |_, v| Ok(LuaValue::Integer(v as i64)),
            |_, lv| {
                if let LuaValue::Integer(i) = lv {
                    Ok(*i as u32)
                } else {
                    Err(LuaError::runtime("expected integer for u32"))
                }
            },
            |v| v.to_le_bytes().to_vec(),
            |buf| u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        );
    }

    fn register_bool(&mut self) {
        self.register_type::<bool>(
            "bool",
            1,
            |_, v| Ok(LuaValue::Boolean(v)),
            |_, lv| {
                if let LuaValue::Boolean(b) = lv {
                    Ok(*b)
                } else {
                    Err(LuaError::runtime("expected boolean"))
                }
            },
            |v| vec![if *v { 1u8 } else { 0u8 }],
            |buf| buf[0] != 0,
        );
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register_all_engine_types();
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2_roundtrip_lua() {
        let lua = Lua::new();
        let v = Vec2::new(1.0, 2.0);
        let t = vec2_to_lua(&lua, v).unwrap();
        let v2 = lua_to_vec2(&t).unwrap();
        assert!((v2.x - 1.0).abs() < 1e-6);
        assert!((v2.y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_roundtrip_lua() {
        let lua = Lua::new();
        let v = Vec3::new(1.0, 2.0, 3.0);
        let t = vec3_to_lua(&lua, v).unwrap();
        let v2 = lua_to_vec3(&t).unwrap();
        assert!((v2.x - 1.0).abs() < 1e-6);
        assert!((v2.y - 2.0).abs() < 1e-6);
        assert!((v2.z - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_vec4_roundtrip_lua() {
        let lua = Lua::new();
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let t = vec4_to_lua(&lua, v).unwrap();
        let v2 = lua_to_vec4(&t).unwrap();
        assert!((v2.x - 1.0).abs() < 1e-6);
        assert!((v2.y - 2.0).abs() < 1e-6);
        assert!((v2.z - 3.0).abs() < 1e-6);
        assert!((v2.w - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_quat_roundtrip_lua() {
        let lua = Lua::new();
        let q = Quat::from_xyzw(0.1, 0.2, 0.3, 0.9);
        let t = quat_to_lua(&lua, q).unwrap();
        let q2 = lua_to_quat(&t).unwrap();
        assert!((q2.x - 0.1).abs() < 1e-5);
        assert!((q2.y - 0.2).abs() < 1e-5);
        assert!((q2.z - 0.3).abs() < 1e-5);
        assert!((q2.w - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_color_roundtrip_lua() {
        let lua = Lua::new();
        let c = Color::new(0.5, 0.6, 0.7, 0.8);
        let t = color_to_lua(&lua, c).unwrap();
        let c2 = lua_to_color(&t).unwrap();
        assert!((c2.r - 0.5).abs() < 1e-6);
        assert!((c2.g - 0.6).abs() < 1e-6);
        assert!((c2.b - 0.7).abs() < 1e-6);
        assert!((c2.a - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_transform_roundtrip_lua() {
        let lua = Lua::new();
        let tr = Transform {
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Vec3::new(0.1, 0.2, 0.3),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let t = transform_to_lua(&lua, tr).unwrap();
        let tr2 = lua_to_transform(&t).unwrap();
        assert!((tr2.position.x - 1.0).abs() < 1e-6);
        assert!((tr2.scale.y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_vec2_roundtrip_bytes() {
        let v = Vec2::new(1.5, -2.5);
        let bytes = vec2_to_bytes(&v);
        assert_eq!(bytes.len(), 8);
        let v2 = bytes_to_vec2(&bytes);
        assert!((v2.x - 1.5).abs() < 1e-6);
        assert!((v2.y - (-2.5)).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_roundtrip_bytes() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let bytes = vec3_to_bytes(&v);
        assert_eq!(bytes.len(), 12);
        let v2 = bytes_to_vec3(&bytes);
        assert!((v2.x - 1.0).abs() < 1e-6);
        assert!((v2.y - 2.0).abs() < 1e-6);
        assert!((v2.z - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_vec4_roundtrip_bytes() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let bytes = vec4_to_bytes(&v);
        assert_eq!(bytes.len(), 16);
        let v2 = bytes_to_vec4(&bytes);
        assert!((v2.x - 1.0).abs() < 1e-6);
        assert!((v2.w - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_quat_roundtrip_bytes() {
        let q = Quat::from_xyzw(0.1, 0.2, 0.3, 0.9);
        let bytes = quat_to_bytes(&q);
        assert_eq!(bytes.len(), 16);
        let q2 = bytes_to_quat(&bytes);
        assert!((q2.x - 0.1).abs() < 1e-5);
        assert!((q2.w - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_color_roundtrip_bytes() {
        let c = Color::new(0.5, 0.6, 0.7, 0.8);
        let bytes = color_to_bytes(&c);
        assert_eq!(bytes.len(), 16);
        let c2 = bytes_to_color(&bytes);
        assert!((c2.r - 0.5).abs() < 1e-6);
        assert!((c2.a - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_transform_roundtrip_bytes() {
        let tr = Transform {
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Vec3::new(0.1, 0.2, 0.3),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let bytes = transform_to_bytes(&tr);
        assert_eq!(bytes.len(), 36);
        let tr2 = bytes_to_transform(&bytes);
        assert!((tr2.position.x - 1.0).abs() < 1e-6);
        assert!((tr2.rotation.y - 0.2).abs() < 1e-6);
        assert!((tr2.scale.z - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_registry_has_all_types() {
        let registry = TypeRegistry::default();
        assert!(registry.has("Vec2"));
        assert!(registry.has("Vec3"));
        assert!(registry.has("Vec4"));
        assert!(registry.has("Quat"));
        assert!(registry.has("Color"));
        assert!(registry.has("Transform"));
        assert!(registry.has("f32"));
        assert!(registry.has("f64"));
        assert!(registry.has("i32"));
        assert!(registry.has("u32"));
        assert!(registry.has("bool"));
        assert!(!registry.has("NonExistent"));
    }

    #[test]
    fn test_registry_wasm_sizes() {
        let registry = TypeRegistry::default();
        assert_eq!(registry.wasm_size("Vec2"), Some(8));
        assert_eq!(registry.wasm_size("Vec3"), Some(12));
        assert_eq!(registry.wasm_size("Vec4"), Some(16));
        assert_eq!(registry.wasm_size("Quat"), Some(16));
        assert_eq!(registry.wasm_size("Color"), Some(16));
        assert_eq!(registry.wasm_size("Transform"), Some(36));
        assert_eq!(registry.wasm_size("f32"), Some(4));
        assert_eq!(registry.wasm_size("NonExistent"), None);
    }

    #[test]
    fn test_registry_lua_get_set() {
        let registry = TypeRegistry::default();
        let lua = Lua::new();
        let mut world = engine_ecs::world::World::new();
        let e = world.spawn();
        world.add_component(e, Vec3::new(10.0, 20.0, 30.0));

        let val = registry.lua_get(&lua, &world, "Vec3", e.index()).unwrap();
        assert!(val.is_some());
        if let Some(LuaValue::Table(t)) = val {
            let x: f32 = t.get("x").unwrap();
            assert!((x - 10.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_registry_wasm_get_set() {
        let registry = TypeRegistry::default();
        let mut world = engine_ecs::world::World::new();
        let e = world.spawn();
        world.add_component(e, Vec3::new(5.0, 6.0, 7.0));

        let mut buf = [0u8; 12];
        let written = registry.wasm_get(&world, "Vec3", e.index(), &mut buf);
        assert_eq!(written, Some(12));

        let v = bytes_to_vec3(&buf);
        assert!((v.x - 5.0).abs() < 1e-6);
        assert!((v.y - 6.0).abs() < 1e-6);
        assert!((v.z - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_registry_registered_names() {
        let registry = TypeRegistry::default();
        let names = registry.registered_names();
        assert!(names.contains(&"Vec3"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"Transform"));
        assert_eq!(names.len(), 11); // Vec2, Vec3, Vec4, Quat, Color, Transform, f32, f64, i32, u32, bool
    }
}
