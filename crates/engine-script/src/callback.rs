//! Callback registration system for script ↔ Rust interop.
//!
//! The [`CallbackRegistry`] allows Rust code to register named callbacks
//! that scripts can invoke by name. This bridges the gap between
//! script-initiated actions and Rust-side logic.
//!
//! # Lua Usage
//!
//! ```lua
//! -- Call a registered Rust callback with arguments
//! local result = callback("on_damage", entity_id, 50.0)
//! ```
//!
//! # WASM Usage
//!
//! WASM modules call the `invoke_callback` host function which routes
//! through the registry.

use crate::error::{BridgeError, BridgeResult};
use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A type-erased callback that can be invoked from scripts.
///
/// The callback receives a slice of `CallbackArg` values and returns
/// an optional result.
pub type CallbackFn = Box<dyn Fn(&[CallbackArg]) -> BridgeResult<CallbackResult> + Send + Sync>;

/// Arguments passed to a callback from a script.
#[derive(Debug, Clone)]
pub enum CallbackArg {
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    F64(f64),
    String(String),
}

impl CallbackArg {
    /// Try to interpret this argument as a `bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CallbackArg::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to interpret this argument as an `i32`.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            CallbackArg::I32(v) => Some(*v),
            CallbackArg::U32(v) => i32::try_from(*v).ok(),
            _ => None,
        }
    }

    /// Try to interpret this argument as a `u32`.
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            CallbackArg::U32(v) => Some(*v),
            CallbackArg::I32(v) => u32::try_from(*v).ok(),
            _ => None,
        }
    }

    /// Try to interpret this argument as an `f32`.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            CallbackArg::F32(v) => Some(*v),
            CallbackArg::F64(v) => Some(*v as f32),
            CallbackArg::I32(v) => Some(*v as f32),
            CallbackArg::U32(v) => Some(*v as f32),
            _ => None,
        }
    }

    /// Try to interpret this argument as an `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            CallbackArg::F64(v) => Some(*v),
            CallbackArg::F32(v) => Some(*v as f64),
            CallbackArg::I32(v) => Some(*v as f64),
            CallbackArg::U32(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Try to interpret this argument as a string slice.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            CallbackArg::String(v) => Some(v),
            _ => None,
        }
    }
}

impl From<bool> for CallbackArg {
    fn from(v: bool) -> Self {
        CallbackArg::Bool(v)
    }
}

impl From<i32> for CallbackArg {
    fn from(v: i32) -> Self {
        CallbackArg::I32(v)
    }
}

impl From<u32> for CallbackArg {
    fn from(v: u32) -> Self {
        CallbackArg::U32(v)
    }
}

impl From<f32> for CallbackArg {
    fn from(v: f32) -> Self {
        CallbackArg::F32(v)
    }
}

impl From<f64> for CallbackArg {
    fn from(v: f64) -> Self {
        CallbackArg::F64(v)
    }
}

impl From<String> for CallbackArg {
    fn from(v: String) -> Self {
        CallbackArg::String(v)
    }
}

impl<'a> From<&'a str> for CallbackArg {
    fn from(v: &'a str) -> Self {
        CallbackArg::String(v.to_string())
    }
}

/// Result returned by a callback.
#[derive(Debug, Clone)]
pub enum CallbackResult {
    None,
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    F64(f64),
    String(String),
}

impl CallbackResult {
    /// Convert to a Lua value.
    pub fn to_lua_value(&self, lua: &Lua) -> LuaResult<LuaValue> {
        match self {
            CallbackResult::None => Ok(LuaValue::Nil),
            CallbackResult::Bool(v) => Ok(LuaValue::Boolean(*v)),
            CallbackResult::I32(v) => Ok(LuaValue::Integer(*v as i64)),
            CallbackResult::U32(v) => Ok(LuaValue::Integer(*v as i64)),
            CallbackResult::F32(v) => Ok(LuaValue::Number(*v as f64)),
            CallbackResult::F64(v) => Ok(LuaValue::Number(*v)),
            CallbackResult::String(v) => lua.create_string(v).map(LuaValue::String),
        }
    }
}

impl From<()> for CallbackResult {
    fn from(_: ()) -> Self {
        CallbackResult::None
    }
}

impl From<bool> for CallbackResult {
    fn from(v: bool) -> Self {
        CallbackResult::Bool(v)
    }
}

impl From<i32> for CallbackResult {
    fn from(v: i32) -> Self {
        CallbackResult::I32(v)
    }
}

impl From<u32> for CallbackResult {
    fn from(v: u32) -> Self {
        CallbackResult::U32(v)
    }
}

impl From<f32> for CallbackResult {
    fn from(v: f32) -> Self {
        CallbackResult::F32(v)
    }
}

impl From<f64> for CallbackResult {
    fn from(v: f64) -> Self {
        CallbackResult::F64(v)
    }
}

impl From<String> for CallbackResult {
    fn from(v: String) -> Self {
        CallbackResult::String(v)
    }
}

/// Registry of named Rust callbacks that scripts can invoke.
///
/// # Example
///
/// ```rust
/// use engine_script::callback::{CallbackRegistry, CallbackArg, CallbackResult};
///
/// let mut registry = CallbackRegistry::new();
/// registry.register("on_damage", |args| {
///     let entity = args[0].as_u32().unwrap_or(0);
///     let amount = args[1].as_f32().unwrap_or(0.0);
///     println!("Entity {} took {} damage", entity, amount);
///     Ok(CallbackResult::None)
/// });
///
/// // Later, invoke from Rust or expose to Lua/WASM
/// let result = registry.invoke("on_damage", &[
///     CallbackArg::U32(42),
///     CallbackArg::F32(25.0),
/// ]).unwrap();
/// ```
pub struct CallbackRegistry {
    callbacks: HashMap<String, CallbackFn>,
}

impl CallbackRegistry {
    /// Create an empty callback registry.
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
        }
    }

    /// Register a named callback.
    ///
    /// If a callback with the same name already exists, it is replaced.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        callback: impl Fn(&[CallbackArg]) -> BridgeResult<CallbackResult> + Send + Sync + 'static,
    ) {
        self.callbacks.insert(name.into(), Box::new(callback));
    }

    /// Invoke a named callback with the given arguments.
    pub fn invoke(&self, name: &str, args: &[CallbackArg]) -> BridgeResult<CallbackResult> {
        match self.callbacks.get(name) {
            Some(cb) => cb(args),
            None => Err(BridgeError::CallbackNotFound(name.to_string())),
        }
    }

    /// Check if a callback is registered.
    pub fn has(&self, name: &str) -> bool {
        self.callbacks.contains_key(name)
    }

    /// Get all registered callback names.
    pub fn registered_names(&self) -> Vec<&str> {
        self.callbacks.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a callback by name. Returns `true` if it existed.
    pub fn unregister(&mut self, name: &str) -> bool {
        self.callbacks.remove(name).is_some()
    }

    /// Create a Lua-compatible `callback(name, ...)` function that routes
    /// through this registry.
    pub fn create_lua_function(
        registry: Arc<RwLock<CallbackRegistry>>,
    ) -> impl Fn(&Lua, LuaMultiValue) -> LuaResult<LuaValue> + Send + Sync + 'static {
        move |lua, args: LuaMultiValue| {
            if args.is_empty() {
                return Err(LuaError::runtime(
                    "callback() requires at least a name argument",
                ));
            }

            let name: String = match &args[0] {
                LuaValue::String(s) => s.to_str()?.to_string(),
                _ => {
                    return Err(LuaError::runtime(
                        "first argument to callback() must be a string",
                    ));
                }
            };

            let mut cb_args = Vec::with_capacity(args.len() - 1);
            for arg in args.iter().skip(1) {
                match arg {
                    LuaValue::Nil => cb_args.push(CallbackArg::F64(0.0)),
                    LuaValue::Boolean(b) => cb_args.push(CallbackArg::Bool(*b)),
                    LuaValue::Integer(i) => cb_args.push(CallbackArg::I32(*i as i32)),
                    LuaValue::Number(n) => cb_args.push(CallbackArg::F64(*n)),
                    LuaValue::String(s) => {
                        cb_args.push(CallbackArg::String(s.to_str()?.to_string()));
                    }
                    _ => {
                        return Err(LuaError::runtime(format!(
                            "unsupported callback argument type: {:?}",
                            arg.type_name()
                        )));
                    }
                }
            }

            let reg = registry.read().unwrap_or_else(|e| e.into_inner());
            match reg.invoke(&name, &cb_args) {
                Ok(result) => result.to_lua_value(lua),
                Err(e) => Err(LuaError::runtime(format!("callback error: {}", e))),
            }
        }
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_invoke() {
        let mut reg = CallbackRegistry::new();
        reg.register("add", |args| {
            let a = args[0].as_f64().unwrap_or(0.0);
            let b = args[1].as_f64().unwrap_or(0.0);
            Ok(CallbackResult::F64(a + b))
        });

        let result = reg
            .invoke("add", &[CallbackArg::F64(3.0), CallbackArg::F64(4.0)])
            .unwrap();
        match result {
            CallbackResult::F64(v) => assert!((v - 7.0).abs() < 1e-10),
            _ => panic!("expected F64"),
        }
    }

    #[test]
    fn test_invoke_not_found() {
        let reg = CallbackRegistry::new();
        let result = reg.invoke("missing", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_and_names() {
        let mut reg = CallbackRegistry::new();
        reg.register("a", |_| Ok(CallbackResult::None));
        reg.register("b", |_| Ok(CallbackResult::None));

        assert!(reg.has("a"));
        assert!(reg.has("b"));
        assert!(!reg.has("c"));

        let mut names = reg.registered_names();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn test_unregister() {
        let mut reg = CallbackRegistry::new();
        reg.register("temp", |_| Ok(CallbackResult::None));
        assert!(reg.has("temp"));

        assert!(reg.unregister("temp"));
        assert!(!reg.has("temp"));
        assert!(!reg.unregister("nonexistent"));
    }

    #[test]
    fn test_callback_arg_conversions() {
        let arg = CallbackArg::from(42u32);
        assert_eq!(arg.as_u32(), Some(42));
        assert_eq!(arg.as_i32(), Some(42));
        assert_eq!(arg.as_f32(), Some(42.0));

        let arg = CallbackArg::from(3.14f32);
        assert_eq!(arg.as_f32(), Some(3.14));
        assert_eq!(arg.as_f64(), Some(3.140000104904175));

        let arg = CallbackArg::from("hello");
        assert_eq!(arg.as_str(), Some("hello"));
    }

    #[test]
    fn test_callback_result_to_lua() {
        let lua = Lua::new();

        let r = CallbackResult::None;
        assert!(matches!(r.to_lua_value(&lua).unwrap(), LuaValue::Nil));

        let r = CallbackResult::Bool(true);
        assert!(matches!(
            r.to_lua_value(&lua).unwrap(),
            LuaValue::Boolean(true)
        ));

        let r = CallbackResult::I32(42);
        assert!(matches!(
            r.to_lua_value(&lua).unwrap(),
            LuaValue::Integer(42)
        ));

        let r = CallbackResult::F32(1.5);
        match r.to_lua_value(&lua).unwrap() {
            LuaValue::Number(n) => assert!((n - 1.5).abs() < 1e-6),
            _ => panic!("expected Number"),
        }
    }

    #[test]
    fn test_lua_callback_function() {
        let mut reg = CallbackRegistry::new();
        reg.register("double", |args| {
            let v = args[0].as_f64().unwrap_or(0.0);
            Ok(CallbackResult::F64(v * 2.0))
        });
        let reg = Arc::new(RwLock::new(reg));

        let lua = Lua::new();
        let cb_fn = CallbackRegistry::create_lua_function(reg);
        lua.globals()
            .set("callback", lua.create_function(cb_fn).unwrap())
            .unwrap();

        let result: f64 = lua.load("return callback('double', 21.0)").eval().unwrap();
        assert!((result - 42.0).abs() < 1e-10);
    }
}
