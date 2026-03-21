//! Error types for the Lua mod runtime.

use std::fmt;

#[derive(Debug)]
pub enum LuaModError {
    /// Lua runtime error (syntax, runtime, etc.)
    Runtime(String),
    /// I/O error loading mod files
    Io(String),
}

impl fmt::Display for LuaModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuaModError::Runtime(msg) => write!(f, "{msg}"),
            LuaModError::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for LuaModError {}

impl From<mlua::Error> for LuaModError {
    fn from(err: mlua::Error) -> Self {
        LuaModError::Runtime(err.to_string())
    }
}
