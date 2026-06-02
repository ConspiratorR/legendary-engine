//! Event system integration for script ↔ Rust interop.
//!
//! The [`ScriptEventBus`] bridges the engine's [`EventChannel`] system with
//! scripts. Scripts can:
//!
//! - **Subscribe** to named event channels
//! - **Emit** events into named channels (which Rust listeners also receive)
//! - **Receive** events emitted from Rust code
//!
//! # Lua Usage
//!
//! ```lua
//! -- Subscribe to an event channel
//! local listener_id = events:subscribe("on_collision", function(data)
//!     print("Collision: " .. data.entity_a .. " <-> " .. data.entity_b)
//! end)
//!
//! -- Emit an event
//! events:emit("on_collision", { entity_a = 1, entity_b = 2, damage = 50 })
//!
//! -- Unsubscribe
//! events:unsubscribe("on_collision", listener_id)
//! ```

use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Identifier for a script-side event listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScriptListenerId(usize);

/// A pending event waiting to be dispatched to script listeners.
#[derive(Debug, Clone)]
pub struct PendingEvent {
    /// The channel this event was emitted on.
    pub channel: String,
    /// The event data as a Lua table (serialized).
    pub data: EventData,
}

/// Event data that can cross the script boundary.
#[derive(Debug, Clone)]
pub enum EventData {
    /// No data.
    None,
    /// A single boolean.
    Bool(bool),
    /// A single i64.
    I64(i64),
    /// A single f64.
    F64(f64),
    /// A string.
    String(String),
    /// Key-value pairs (for Lua tables).
    Map(Vec<(String, EventData)>),
}

impl EventData {
    /// Convert this event data to a Lua value.
    pub fn to_lua_value(&self, lua: &Lua) -> LuaResult<LuaValue> {
        match self {
            EventData::None => Ok(LuaValue::Nil),
            EventData::Bool(v) => Ok(LuaValue::Boolean(*v)),
            EventData::I64(v) => Ok(LuaValue::Integer(*v)),
            EventData::F64(v) => Ok(LuaValue::Number(*v)),
            EventData::String(v) => lua.create_string(v).map(LuaValue::String),
            EventData::Map(pairs) => {
                let table = lua.create_table()?;
                for (key, val) in pairs {
                    table.set(key.as_str(), val.to_lua_value(lua)?)?;
                }
                Ok(LuaValue::Table(table))
            }
        }
    }

    /// Convert a Lua value to `EventData`.
    pub fn from_lua_value(value: &LuaValue) -> LuaResult<Self> {
        match value {
            LuaValue::Nil => Ok(EventData::None),
            LuaValue::Boolean(b) => Ok(EventData::Bool(*b)),
            LuaValue::Integer(i) => Ok(EventData::I64(*i)),
            LuaValue::Number(n) => Ok(EventData::F64(*n)),
            LuaValue::String(s) => Ok(EventData::String(s.to_str()?.to_string())),
            LuaValue::Table(t) => {
                let mut pairs = Vec::new();
                for pair in t.clone().pairs::<String, LuaValue>() {
                    let (key, val) = pair?;
                    pairs.push((key, Self::from_lua_value(&val)?));
                }
                Ok(EventData::Map(pairs))
            }
            _ => Err(LuaError::runtime(format!(
                "unsupported event data type: {}",
                value.type_name()
            ))),
        }
    }
}

/// A script-side listener entry.
struct ScriptListener {
    _id: ScriptListenerId,
    /// The Lua callback function (stored as a registry reference).
    callback_ref: LuaRegistryKey,
}

/// A named event channel that bridges Rust and script listeners.
struct EventChannelState {
    /// Script listeners for this channel.
    listeners: Vec<ScriptListener>,
    /// Next listener ID counter.
    next_id: usize,
    /// Pending events queued for script dispatch.
    pending: Vec<EventData>,
}

/// Bus bridging engine events to scripts.
///
/// Manages named event channels. Scripts subscribe via Lua functions,
/// and events can be emitted from either Rust or script code.
///
/// # Example
///
/// ```rust
/// use engine_script::event_bridge::{ScriptEventBus, EventData};
///
/// let bus = ScriptEventBus::new();
/// bus.queue_event("on_tick", EventData::F64(0.016));
///
/// let pending = bus.drain_pending();
/// assert_eq!(pending.len(), 1);
/// assert_eq!(pending[0].channel, "on_tick");
/// ```
pub struct ScriptEventBus {
    channels: Mutex<HashMap<String, EventChannelState>>,
}

impl ScriptEventBus {
    /// Create a new empty event bus.
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// Queue an event on a named channel for script dispatch.
    ///
    /// This is the Rust-side entry point: scripts will receive the event
    /// the next time [`drain_pending`] is called.
    pub fn queue_event(&self, channel: impl Into<String>, data: EventData) {
        let mut channels = self.channels.lock().unwrap();
        let channel_name = channel.into();
        let ch = channels
            .entry(channel_name.clone())
            .or_insert_with(|| EventChannelState {
                listeners: Vec::new(),
                next_id: 0,
                pending: Vec::new(),
            });
        ch.pending.push(data);
    }

    /// Drain all pending events from all channels.
    pub fn drain_pending(&self) -> Vec<PendingEvent> {
        let mut channels = self.channels.lock().unwrap();
        let mut result = Vec::new();
        for (name, ch) in channels.iter_mut() {
            for data in ch.pending.drain(..) {
                result.push(PendingEvent {
                    channel: name.clone(),
                    data,
                });
            }
        }
        result
    }

    /// Subscribe a Lua function to a named channel.
    ///
    /// Returns a [`ScriptListenerId`] for later unsubscription.
    pub fn subscribe(
        &self,
        channel: impl Into<String>,
        lua: &Lua,
        callback: LuaFunction,
    ) -> LuaResult<ScriptListenerId> {
        let ref_key = lua.create_registry_value(&callback)?;
        let mut channels = self.channels.lock().unwrap();
        let ch = channels
            .entry(channel.into())
            .or_insert_with(|| EventChannelState {
                listeners: Vec::new(),
                next_id: 0,
                pending: Vec::new(),
            });
        let id = ScriptListenerId(ch.next_id);
        ch.next_id += 1;
        ch.listeners.push(ScriptListener {
            _id: id,
            callback_ref: ref_key,
        });
        Ok(id)
    }

    /// Unsubscribe a listener from a channel.
    pub fn unsubscribe(&self, channel: &str, id: ScriptListenerId) {
        let mut channels = self.channels.lock().unwrap();
        if let Some(ch) = channels.get_mut(channel) {
            ch.listeners.retain(|l| l._id != id);
        }
    }

    /// Dispatch all pending events to Lua listeners.
    ///
    /// This is called by the script system each frame before running
    /// user scripts. Events are delivered as calls to the registered
    /// callback functions.
    pub fn dispatch_to_lua(&self, lua: &Lua) -> LuaResult<()> {
        let pending = self.drain_pending();
        let channels = self.channels.lock().unwrap();

        for event in pending {
            if let Some(ch) = channels.get(&event.channel) {
                let lua_data = event.data.to_lua_value(lua)?;
                for listener in &ch.listeners {
                    let cb: LuaFunction = lua.registry_value(&listener.callback_ref)?;
                    let _ = cb.call::<()>(&lua_data); // ignore errors in listeners
                }
            }
        }

        Ok(())
    }

    /// Create a Lua `events` table with `subscribe`, `emit`, and `unsubscribe` methods.
    pub fn create_lua_table(self: &Arc<Self>, lua: &Lua) -> LuaResult<LuaTable> {
        let table = lua.create_table()?;

        // events:subscribe(channel, callback) -> listener_id
        let bus = Arc::clone(self);
        table.set(
            "subscribe",
            lua.create_function(move |lua, args: LuaMultiValue| {
                // Handle method call syntax: first arg may be the table itself
                let (channel, callback) = match args.len() {
                    2 => {
                        // Direct call: events.subscribe("channel", cb)
                        let channel: String =
                            lua.unpack(args.front().cloned().unwrap_or(LuaValue::Nil))?;
                        let callback: LuaFunction =
                            lua.unpack(args.get(1).cloned().unwrap_or(LuaValue::Nil))?;
                        (channel, callback)
                    }
                    3 => {
                        // Method call: events:subscribe("channel", cb) passes self as first arg
                        let channel: String =
                            lua.unpack(args.get(1).cloned().unwrap_or(LuaValue::Nil))?;
                        let callback: LuaFunction =
                            lua.unpack(args.get(2).cloned().unwrap_or(LuaValue::Nil))?;
                        (channel, callback)
                    }
                    _ => {
                        return Err(LuaError::runtime(
                            "subscribe() requires (channel, callback) arguments",
                        ));
                    }
                };
                let id = bus.subscribe(&channel, lua, callback)?;
                Ok(id.0 as i64)
            })?,
        )?;

        // events:emit(channel, data)
        let bus = Arc::clone(self);
        table.set(
            "emit",
            lua.create_function(move |_lua, args: LuaMultiValue| {
                let (channel, data) = match args.len() {
                    2 => {
                        let channel: String =
                            _lua.unpack(args.front().cloned().unwrap_or(LuaValue::Nil))?;
                        let data = args.get(1).cloned().unwrap_or(LuaValue::Nil);
                        (channel, data)
                    }
                    3 => {
                        let channel: String =
                            _lua.unpack(args.get(1).cloned().unwrap_or(LuaValue::Nil))?;
                        let data = args.get(2).cloned().unwrap_or(LuaValue::Nil);
                        (channel, data)
                    }
                    _ => {
                        return Err(LuaError::runtime(
                            "emit() requires (channel, data) arguments",
                        ));
                    }
                };
                let event_data = EventData::from_lua_value(&data)?;
                bus.queue_event(&channel, event_data);
                Ok(())
            })?,
        )?;

        // events:unsubscribe(channel, listener_id)
        let bus = Arc::clone(self);
        table.set(
            "unsubscribe",
            lua.create_function(move |_, args: LuaMultiValue| {
                let (channel, id) = match args.len() {
                    2 => {
                        let channel: String = match &args[0] {
                            LuaValue::String(s) => s.to_str()?.to_string(),
                            _ => return Err(LuaError::runtime("expected string for channel")),
                        };
                        let id: i64 = match &args[1] {
                            LuaValue::Integer(i) => *i,
                            _ => return Err(LuaError::runtime("expected integer for listener_id")),
                        };
                        (channel, id)
                    }
                    3 => {
                        let channel: String = match &args[1] {
                            LuaValue::String(s) => s.to_str()?.to_string(),
                            _ => return Err(LuaError::runtime("expected string for channel")),
                        };
                        let id: i64 = match &args[2] {
                            LuaValue::Integer(i) => *i,
                            _ => return Err(LuaError::runtime("expected integer for listener_id")),
                        };
                        (channel, id)
                    }
                    _ => {
                        return Err(LuaError::runtime(
                            "unsubscribe() requires (channel, listener_id) arguments",
                        ));
                    }
                };
                bus.unsubscribe(&channel, ScriptListenerId(id as usize));
                Ok(())
            })?,
        )?;

        Ok(table)
    }

    /// Check if a channel has any listeners.
    pub fn has_listeners(&self, channel: &str) -> bool {
        let channels = self.channels.lock().unwrap();
        channels
            .get(channel)
            .map(|ch| !ch.listeners.is_empty())
            .unwrap_or(false)
    }

    /// Get the number of listeners on a channel.
    pub fn listener_count(&self, channel: &str) -> usize {
        let channels = self.channels.lock().unwrap();
        channels
            .get(channel)
            .map(|ch| ch.listeners.len())
            .unwrap_or(0)
    }
}

impl Default for ScriptEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_and_drain() {
        let bus = ScriptEventBus::new();
        bus.queue_event("test", EventData::I64(42));
        bus.queue_event("test", EventData::F64(3.14));
        bus.queue_event("other", EventData::Bool(true));

        let pending = bus.drain_pending();
        assert_eq!(pending.len(), 3);

        let test_events: Vec<_> = pending.iter().filter(|e| e.channel == "test").collect();
        let other_events: Vec<_> = pending.iter().filter(|e| e.channel == "other").collect();
        assert_eq!(test_events.len(), 2);
        assert_eq!(other_events.len(), 1);

        // Second drain should be empty
        let pending = bus.drain_pending();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_event_data_lua_roundtrip() {
        let lua = Lua::new();

        let data = EventData::None;
        let lv = data.to_lua_value(&lua).unwrap();
        assert!(matches!(lv, LuaValue::Nil));
        let back = EventData::from_lua_value(&lv).unwrap();
        assert!(matches!(back, EventData::None));

        let data = EventData::Bool(true);
        let lv = data.to_lua_value(&lua).unwrap();
        assert!(matches!(lv, LuaValue::Boolean(true)));

        let data = EventData::I64(42);
        let lv = data.to_lua_value(&lua).unwrap();
        assert!(matches!(lv, LuaValue::Integer(42)));

        let data = EventData::F64(3.14);
        let lv = data.to_lua_value(&lua).unwrap();
        match lv {
            LuaValue::Number(n) => assert!((n - 3.14).abs() < 1e-10),
            _ => panic!("expected Number"),
        }

        let data = EventData::String("hello".to_string());
        let lv = data.to_lua_value(&lua).unwrap();
        assert!(matches!(lv, LuaValue::String(_)));
    }

    #[test]
    fn test_event_data_map_roundtrip() {
        let lua = Lua::new();
        let data = EventData::Map(vec![
            ("x".to_string(), EventData::F64(1.0)),
            ("y".to_string(), EventData::F64(2.0)),
        ]);
        let lv = data.to_lua_value(&lua).unwrap();
        let back = EventData::from_lua_value(&lv).unwrap();
        match back {
            EventData::Map(pairs) => {
                assert_eq!(pairs.len(), 2);
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn test_subscribe_and_emit() {
        let bus = ScriptEventBus::new();
        let lua = Lua::new();

        let received = Arc::new(Mutex::new(Vec::new()));
        let r = received.clone();

        let cb = lua
            .create_function(move |_, data: LuaValue| {
                if let LuaValue::Number(n) = data {
                    r.lock().unwrap().push(n);
                }
                Ok(())
            })
            .unwrap();

        bus.subscribe("test", &lua, cb).unwrap();
        assert!(bus.has_listeners("test"));
        assert_eq!(bus.listener_count("test"), 1);
    }

    #[test]
    fn test_unsubscribe() {
        let bus = ScriptEventBus::new();
        let lua = Lua::new();

        let cb = lua.create_function(|_, _: LuaValue| Ok(())).unwrap();
        let id = bus.subscribe("test", &lua, cb).unwrap();
        assert_eq!(bus.listener_count("test"), 1);

        bus.unsubscribe("test", id);
        assert_eq!(bus.listener_count("test"), 0);
        assert!(!bus.has_listeners("test"));
    }

    #[test]
    fn test_dispatch_to_lua() {
        let bus = ScriptEventBus::new();
        let lua = Lua::new();
        let bus = Arc::new(bus);

        // Register a Lua function that stores received values
        lua.load(
            r#"
            received = {}
            function on_test(data)
                table.insert(received, data)
            end
            "#,
        )
        .exec()
        .unwrap();

        let cb: LuaFunction = lua.globals().get("on_test").unwrap();
        bus.subscribe("test", &lua, cb).unwrap();

        // Queue events from Rust side
        bus.queue_event("test", EventData::I64(10));
        bus.queue_event("test", EventData::I64(20));

        // Dispatch to Lua
        bus.dispatch_to_lua(&lua).unwrap();

        // Verify Lua received them
        let received: LuaTable = lua.globals().get("received").unwrap();
        assert_eq!(received.len().unwrap(), 2);
        let v1: i64 = received.get(1).unwrap();
        let v2: i64 = received.get(2).unwrap();
        assert_eq!(v1, 10);
        assert_eq!(v2, 20);
    }

    #[test]
    fn test_create_lua_table() {
        let bus = Arc::new(ScriptEventBus::new());
        let lua = Lua::new();

        let events_table = bus.create_lua_table(&lua).unwrap();
        lua.globals().set("events", events_table).unwrap();

        // Should have subscribe, emit, unsubscribe
        assert!(lua.load("type(events.subscribe)").eval::<String>().unwrap() == "function");
        assert!(lua.load("type(events.emit)").eval::<String>().unwrap() == "function");
        assert!(
            lua.load("type(events.unsubscribe)")
                .eval::<String>()
                .unwrap()
                == "function"
        );
    }

    #[test]
    fn test_full_lua_integration() {
        let bus = Arc::new(ScriptEventBus::new());
        let lua = Lua::new();

        let events_table = bus.create_lua_table(&lua).unwrap();
        lua.globals().set("events", events_table).unwrap();

        // Subscribe and emit from Lua
        lua.load(
            r#"
            results = {}
            events:subscribe("ping", function(data)
                table.insert(results, data)
            end)
            events:emit("ping", 42)
            events:emit("ping", 99)
            "#,
        )
        .exec()
        .unwrap();

        // Dispatch queued events
        bus.dispatch_to_lua(&lua).unwrap();

        let results: LuaTable = lua.globals().get("results").unwrap();
        assert_eq!(results.len().unwrap(), 2);
    }
}
