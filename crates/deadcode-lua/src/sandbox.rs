//! Lua sandbox: strip unsafe globals, inject deterministic replacements.

use mlua::Lua;

use crate::error::LuaModError;

/// Apply sandbox restrictions to the Lua state.
///
/// Removes:
/// - `os` (filesystem, clock, system calls)
/// - `io` (file I/O)
/// - `debug` (introspection, can escape sandbox)
/// - `dofile`, `loadfile` (filesystem access)
/// - `package` (module loading from filesystem)
///
/// Keeps:
/// - `math`, `string`, `table`, `pairs`, `ipairs`, `type`, `tostring`, `tonumber`,
///   `pcall`, `xpcall`, `error`, `select`, `unpack`, `next`, `rawget`, `rawset`,
///   `setmetatable`, `getmetatable`, `coroutine`
pub fn apply_sandbox(lua: &Lua) -> Result<(), LuaModError> {
    lua.load(
        r#"
        os = nil
        io = nil
        debug = nil
        dofile = nil
        loadfile = nil
        package = nil
        "#,
    )
    .exec()
    .map_err(|e| LuaModError::Runtime(format!("sandbox setup failed: {e}")))?;

    Ok(())
}
