//! Bidirectional conversion between SimValue and mlua::Value.

use mlua::{Lua, Value, IntoLua};

use deadcode_sim::entity::EntityId;
use deadcode_sim::value::SimValue;

/// Convert a SimValue to a Lua value.
#[allow(dead_code)]
pub fn sim_to_lua(lua: &Lua, val: &SimValue) -> mlua::Result<Value> {
    match val {
        SimValue::Int(n) => Ok(Value::Integer(*n)),
        SimValue::Bool(b) => Ok(Value::Boolean(*b)),
        SimValue::Str(s) => s.as_str().into_lua(lua),
        SimValue::None => Ok(Value::Nil),
        SimValue::EntityRef(eid) => {
            // EntityRef is represented as a Lua integer (the raw ID).
            Ok(Value::Integer(eid.0 as i64))
        }
        SimValue::List(items) => {
            let table = lua.create_table()?;
            for (i, item) in items.iter().enumerate() {
                table.set(i as i64 + 1, sim_to_lua(lua, item)?)?;
            }
            Ok(Value::Table(table))
        }
        SimValue::Dict(map) => {
            let table = lua.create_table()?;
            for (k, v) in map.iter() {
                table.set(k.as_str(), sim_to_lua(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}

/// Convert a Lua value to a SimValue.
#[allow(dead_code)]
pub fn lua_to_sim(val: &Value) -> SimValue {
    match val {
        Value::Integer(n) => SimValue::Int(*n),
        Value::Number(f) => SimValue::Int(*f as i64), // Sim is integer-only
        Value::Boolean(b) => SimValue::Bool(*b),
        Value::String(s) => SimValue::Str(s.to_str().map(|s| s.to_string()).unwrap_or_default()),
        Value::Nil => SimValue::None,
        Value::Table(t) => {
            // Check if it looks like an array (sequential integer keys starting at 1).
            let len = t.raw_len();
            if len > 0 {
                let mut items = Vec::with_capacity(len);
                for i in 1..=len as i64 {
                    if let Ok(v) = t.raw_get::<Value>(i) {
                        items.push(lua_to_sim(&v));
                    }
                }
                SimValue::List(items)
            } else {
                // Treat as dict.
                let mut map = indexmap::IndexMap::new();
                if let Ok(pairs) = t.clone().pairs::<String, Value>().collect::<Result<Vec<_>, _>>() {
                    for (k, v) in pairs {
                        map.insert(k, lua_to_sim(&v));
                    }
                }
                SimValue::Dict(map)
            }
        }
        _ => SimValue::None,
    }
}

/// Helper: extract an EntityId from a Lua value.
/// Accepts integer (raw ID) or the string "self" resolved via caster_id.
pub fn lua_to_entity_id(val: &Value, caster_id: EntityId) -> Option<EntityId> {
    match val {
        Value::Integer(n) => Some(EntityId(*n as u64)),
        Value::String(s) => {
            let s = s.to_str().ok()?;
            if s == "self" {
                Some(caster_id)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper: extract a string target, resolving "self" to caster_id.
#[allow(dead_code)]
pub fn resolve_target(val: Value, caster_id: EntityId) -> Option<EntityId> {
    lua_to_entity_id(&val, caster_id)
}
