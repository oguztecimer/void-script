use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    None,
    List(Vec<Value>),
    Dict(HashMap<String, Value>),
    /// Reference to a game entity (stub)
    Entity {
        id: u64,
        name: String,
        entity_type: String,
    },
    /// Tuple (used for positions)
    Tuple(Vec<Value>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::None => false,
            Value::List(l) => !l.is_empty(),
            Value::Dict(d) => !d.is_empty(),
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "str",
            Value::Bool(_) => "bool",
            Value::None => "NoneType",
            Value::List(_) => "list",
            Value::Dict(_) => "dict",
            Value::Entity { .. } => "Entity",
            Value::Tuple(_) => "tuple",
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => format!("{f}"),
            Value::String(s) => s.clone(),
            Value::Bool(true) => "True".to_string(),
            Value::Bool(false) => "False".to_string(),
            Value::None => "None".to_string(),
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.repr()).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Dict(map) => {
                let inner: Vec<String> =
                    map.iter().map(|(k, v)| format!("{}: {}", k, v.repr())).collect();
                format!("{{{}}}", inner.join(", "))
            }
            Value::Entity { name, .. } => format!("<Entity:{name}>"),
            Value::Tuple(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.repr()).collect();
                format!("({})", inner.join(", "))
            }
        }
    }

    pub fn repr(&self) -> String {
        match self {
            Value::String(s) => format!("\"{s}\""),
            other => other.display(),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Entity { id: a, .. }, Value::Entity { id: b, .. }) => a == b,
            _ => false,
        }
    }
}
