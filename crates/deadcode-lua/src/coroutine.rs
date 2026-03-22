//! Lua coroutine lifecycle management.
//!
//! Command handlers are Lua functions wrapped in coroutines. When a handler
//! calls `ctx:yield_ticks(N)`, the coroutine yields and the sim stores
//! a `LuaCoroutineState` on the entity. After N ticks, the coroutine is resumed.

use mlua::{Lua, Function, Thread, Value, ThreadStatus};

use deadcode_sim::action::CoroutineHandle;
use deadcode_sim::entity::EntityId;
use deadcode_sim::value::SimValue;
use deadcode_sim::world::{SimEvent, WorldAccess};

use crate::api::CtxUserData;
use crate::error::LuaModError;

/// Internal storage for a suspended coroutine.
pub struct LuaCoroutineEntry {
    /// Registry key for the Lua thread (coroutine).
    pub thread_key: mlua::RegistryKey,
}

/// Outcome of creating/resuming a coroutine.
pub struct CoroutineOutcome {
    pub events: Vec<SimEvent>,
    /// If Some, the coroutine yielded and needs to be stored.
    pub coroutine_entry: Option<LuaCoroutineEntry>,
    pub remaining_ticks: i64,
    pub interruptible: bool,
}

/// Create a coroutine for a command handler and run it until first yield or completion.
pub fn create_and_resume_command(
    lua: &Lua,
    mod_id: &str,
    command_name: &str,
    entity_id: EntityId,
    args: &[SimValue],
    world: &mut WorldAccess,
    _handle: CoroutineHandle,
) -> Result<CoroutineOutcome, LuaModError> {
    // Look up the command handler function from the mod's registry.
    let registry_key = format!("__mod_{mod_id}_cmd_{command_name}");
    let handler: Function = lua.globals().get(registry_key.as_str())
        .map_err(|e| LuaModError::Runtime(format!("command '{command_name}' not found in mod '{mod_id}': {e}")))?;

    // Create a wrapper function that adds yield_ticks/wait to ctx.
    // This wrapper calls the actual handler with a patched ctx.
    let wrapper: Function = lua.globals().get("__void_cmd_wrapper")
        .map_err(|e| LuaModError::Runtime(format!("command wrapper not found: {e}")))?;

    // Create a coroutine from the wrapper function.
    let thread = lua.create_thread(wrapper)
        .map_err(|e| LuaModError::Runtime(format!("failed to create coroutine: {e}")))?;

    // Create the ctx userdata.
    let ctx = create_ctx(lua, entity_id, world, args)?;

    // Resume the coroutine with (handler, ctx) as arguments.
    let handler_val = Value::Function(handler);
    resume_thread(lua, thread, entity_id, world, Some((handler_val, ctx)))
}

/// Resume an existing coroutine.
pub fn resume_coroutine(
    lua: &Lua,
    entry: LuaCoroutineEntry,
    entity_id: EntityId,
    world: &mut WorldAccess,
    _handle: CoroutineHandle,
) -> Result<CoroutineOutcome, LuaModError> {
    let thread: Thread = lua.registry_value(&entry.thread_key)
        .map_err(|e| LuaModError::Runtime(format!("coroutine not found: {e}")))?;

    // Remove from registry since we're taking ownership.
    lua.remove_registry_value(entry.thread_key)
        .map_err(|e| LuaModError::Runtime(format!("failed to remove registry: {e}")))?;

    resume_thread::<()>(lua, thread, entity_id, world, None)
}

/// Create a `ctx` userdata for the command handler.
fn create_ctx(
    lua: &Lua,
    entity_id: EntityId,
    world: &mut WorldAccess,
    args: &[SimValue],
) -> Result<Value, LuaModError> {
    let ctx = CtxUserData::new(entity_id, world, args);
    let ud = lua.create_userdata(ctx)
        .map_err(|e| LuaModError::Runtime(format!("failed to create ctx: {e}")))?;
    Ok(Value::UserData(ud))
}

/// Resume a thread and process the result.
fn resume_thread<A: mlua::IntoLuaMulti>(
    lua: &Lua,
    thread: Thread,
    entity_id: EntityId,
    world: &mut WorldAccess,
    initial_args: Option<A>,
) -> Result<CoroutineOutcome, LuaModError> {
    // Set up the world access pointer in Lua's app_data so ctx methods can use it.
    // Safety: we ensure the WorldAccess reference is valid for the duration of the resume.
    let world_ptr = world as *mut WorldAccess as usize;
    lua.set_app_data(WorldAccessPtr(world_ptr));
    lua.set_app_data(CasterEntity(entity_id));

    let args = if let Some(a) = initial_args {
        a.into_lua_multi(lua).map_err(|e| LuaModError::Runtime(format!("arg conversion: {e}")))?
    } else {
        mlua::MultiValue::new()
    };

    let result = thread.resume::<mlua::MultiValue>(args);

    // Collect events from ctx.
    let mut events = Vec::new();
    if let Some(collected) = lua.remove_app_data::<CollectedEvents>() {
        events = collected.0;
    }
    // Clean up app_data.
    lua.remove_app_data::<WorldAccessPtr>();
    lua.remove_app_data::<CasterEntity>();

    match result {
        Ok(_values) => {
            let status = thread.status();
            match status {
                ThreadStatus::Resumable => {
                    // Coroutine yielded. Extract yield info from app_data.
                    let yield_info = lua.remove_app_data::<YieldInfo>()
                        .unwrap_or(YieldInfo { ticks: 1, interruptible: false });

                    // Store the coroutine in the registry.
                    let thread_key = lua.create_registry_value(thread)
                        .map_err(|e| LuaModError::Runtime(format!("failed to store coroutine: {e}")))?;

                    Ok(CoroutineOutcome {
                        events,
                        coroutine_entry: Some(LuaCoroutineEntry { thread_key }),
                        remaining_ticks: yield_info.ticks,
                        interruptible: yield_info.interruptible,
                    })
                }
                _ => {
                    // Coroutine completed.
                    Ok(CoroutineOutcome {
                        events,
                        coroutine_entry: None,
                        remaining_ticks: 0,
                        interruptible: false,
                    })
                }
            }
        }
        Err(e) => {
            // Lua error — report and treat as completed.
            events.push(SimEvent::ScriptOutput {
                entity_id,
                text: format!("[lua error] {e}"),
            });
            Ok(CoroutineOutcome {
                events,
                coroutine_entry: None,
                remaining_ticks: 0,
                interruptible: false,
            })
        }
    }
}

/// Call a query handler directly (no coroutine). Returns the handler's return value.
/// Query handlers must not yield.
pub fn call_query_handler(
    lua: &Lua,
    mod_id: &str,
    command_name: &str,
    entity_id: EntityId,
    args: &[SimValue],
    world: &mut WorldAccess,
) -> Result<(SimValue, Vec<SimEvent>), LuaModError> {
    let registry_key = format!("__mod_{mod_id}_cmd_{command_name}");
    let handler: Function = lua.globals().get(registry_key.as_str())
        .map_err(|e| LuaModError::Runtime(format!("query '{command_name}' not found: {e}")))?;

    // Use the query wrapper which sets up the ctx proxy (args, caster, tick)
    // but without coroutine yield support.
    let wrapper: Function = lua.globals().get("__void_query_wrapper")
        .map_err(|e| LuaModError::Runtime(format!("query wrapper not found: {e}")))?;

    let ctx = create_ctx(lua, entity_id, world, args)?;

    // Set up world access for ctx methods.
    let world_ptr = world as *mut WorldAccess as usize;
    lua.set_app_data(WorldAccessPtr(world_ptr));
    lua.set_app_data(CasterEntity(entity_id));

    // Call wrapper(handler, ctx) — returns the handler's return value.
    let handler_val = Value::Function(handler);
    let result = wrapper.call::<Value>((handler_val, ctx));

    // Collect events.
    let events = lua.remove_app_data::<CollectedEvents>()
        .map(|c| c.0)
        .unwrap_or_default();
    lua.remove_app_data::<WorldAccessPtr>();
    lua.remove_app_data::<CasterEntity>();

    match result {
        Ok(val) => {
            let sim_val = crate::convert::lua_to_sim(&val);
            Ok((sim_val, events))
        }
        Err(e) => Err(LuaModError::Runtime(format!("{e}"))),
    }
}

// App-data types stored during coroutine execution.

/// Raw pointer to WorldAccess, stored in Lua's app_data during resume.
pub(crate) struct WorldAccessPtr(pub usize);

/// The entity executing the current command.
#[allow(dead_code)]
pub(crate) struct CasterEntity(pub EntityId);

/// Yield parameters set by ctx:yield_ticks().
pub(crate) struct YieldInfo {
    pub ticks: i64,
    pub interruptible: bool,
}

/// Events collected during a coroutine resume.
pub(crate) struct CollectedEvents(pub Vec<SimEvent>);
