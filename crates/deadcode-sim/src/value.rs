use std::fmt;

use serde::{Deserialize, Serialize};

use crate::entity::EntityId;

/// Simulation value type — separate from `grimscript_lang::Value`.
///
/// Key differences:
/// - No `f64` (determinism).
/// - `Dict` uses `Vec<(String, SimValue)>` for deterministic iteration order.
/// - `EntityRef` is a lightweight ID reference; attribute access goes through queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimValue {
    Int(i64),
    Bool(bool),
    Str(String),
    None,
    List(Vec<SimValue>),
    /// Ordered key-value pairs — deterministic iteration.
    Dict(Vec<(String, SimValue)>),
    /// Lightweight entity reference resolved via world queries.
    EntityRef(EntityId),
}

impl SimValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            SimValue::Int(_) => "int",
            SimValue::Bool(_) => "bool",
            SimValue::Str(_) => "str",
            SimValue::None => "NoneType",
            SimValue::List(_) => "list",
            SimValue::Dict(_) => "dict",
            SimValue::EntityRef(_) => "entity",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            SimValue::Int(n) => *n != 0,
            SimValue::Bool(b) => *b,
            SimValue::Str(s) => !s.is_empty(),
            SimValue::None => false,
            SimValue::List(v) => !v.is_empty(),
            SimValue::Dict(v) => !v.is_empty(),
            SimValue::EntityRef(_) => true,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            SimValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SimValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            SimValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_entity_ref(&self) -> Option<EntityId> {
        match self {
            SimValue::EntityRef(id) => Some(*id),
            _ => None,
        }
    }
}

impl fmt::Display for SimValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimValue::Int(n) => write!(f, "{n}"),
            SimValue::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            SimValue::Str(s) => write!(f, "{s}"),
            SimValue::None => write!(f, "None"),
            SimValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            SimValue::Dict(pairs) => {
                write!(f, "{{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{k}\": {v}")?;
                }
                write!(f, "}}")
            }
            SimValue::EntityRef(id) => write!(f, "<entity {}>", id.0),
        }
    }
}

impl PartialEq for SimValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SimValue::Int(a), SimValue::Int(b)) => a == b,
            (SimValue::Bool(a), SimValue::Bool(b)) => a == b,
            (SimValue::Str(a), SimValue::Str(b)) => a == b,
            (SimValue::None, SimValue::None) => true,
            (SimValue::EntityRef(a), SimValue::EntityRef(b)) => a == b,
            (SimValue::List(a), SimValue::List(b)) => a == b,
            (SimValue::Dict(a), SimValue::Dict(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for SimValue {}
