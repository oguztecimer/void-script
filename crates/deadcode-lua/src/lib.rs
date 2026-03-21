//! Lua scripting runtime for mod logic (commands, triggers, buff callbacks, init).
//!
//! Implements the `CommandHandler` trait from `deadcode-sim`, providing a Lua-based
//! alternative to TOML-defined effects/phases/triggers.

mod api;
mod convert;
mod coroutine;
mod error;
mod sandbox;
#[cfg(test)]
mod tests;
mod triggers;

// Re-export CoroutineHandle from the sim crate (canonical type).
pub use deadcode_sim::action::CoroutineHandle;
pub use crate::error::LuaModError;

use std::collections::HashMap;
use std::path::Path;

use deadcode_sim::action::{BuffCallbackType, CommandHandler, CommandHandlerResult, CommandMeta};
use deadcode_sim::entity::EntityId;
use deadcode_sim::value::SimValue;
use deadcode_sim::world::{SimEvent, WorldAccess};

use mlua::Lua;

/// Runtime for all loaded Lua mods.
pub struct LuaModRuntime {
    lua: Lua,
    /// mod_id → list of registered command names
    mod_commands: HashMap<String, Vec<String>>,
    /// command_name → mod_id (for dispatch)
    command_to_mod: HashMap<String, String>,
    /// Next coroutine handle ID.
    next_coroutine_id: u64,
    /// Active coroutines keyed by handle.
    active_coroutines: HashMap<u64, coroutine::LuaCoroutineEntry>,
}

impl LuaModRuntime {
    /// Create a new Lua runtime with a sandboxed environment.
    pub fn new() -> Result<Self, LuaModError> {
        let lua = Lua::new();
        sandbox::apply_sandbox(&lua)?;
        Ok(Self {
            lua,
            mod_commands: HashMap::new(),
            command_to_mod: HashMap::new(),
            next_coroutine_id: 1,
            active_coroutines: HashMap::new(),
        })
    }

    /// Load a mod's `mod.lua` file. Executes the script which registers
    /// commands, triggers, buff callbacks, and init handlers.
    pub fn load_mod(&mut self, mod_id: &str, mod_dir: &Path) -> Result<(), LuaModError> {
        let lua_path = mod_dir.join("mod.lua");
        if !lua_path.exists() {
            return Ok(()); // No Lua file — mod uses TOML-only
        }
        let source = std::fs::read_to_string(&lua_path)
            .map_err(|e| LuaModError::Io(format!("{}: {e}", lua_path.display())))?;
        self.load_mod_source(mod_id, &source)
    }

    /// Load a mod from a Lua source string (for hot-reload and testing).
    pub fn load_mod_source(&mut self, mod_id: &str, source: &str) -> Result<(), LuaModError> {
        // Set up per-mod registry tables
        api::setup_mod_tables(&self.lua, mod_id)?;

        // Execute the mod script
        self.lua.load(source)
            .set_name(format!("mod:{mod_id}"))
            .exec()
            .map_err(|e| LuaModError::Runtime(format!("[mod:{mod_id}] {e}")))?;

        // Collect registered command names
        let commands = api::get_registered_commands(&self.lua, mod_id)?;
        for name in &commands {
            self.command_to_mod.insert(name.clone(), mod_id.to_string());
        }
        self.mod_commands.insert(mod_id.to_string(), commands);

        Ok(())
    }

    /// Check if a command has a Lua handler registered.
    pub fn has_command(&self, name: &str) -> bool {
        self.command_to_mod.contains_key(name)
    }

    /// Allocate a new coroutine handle.
    fn alloc_handle(&mut self) -> CoroutineHandle {
        let id = self.next_coroutine_id;
        self.next_coroutine_id += 1;
        CoroutineHandle(id)
    }
}

impl CommandHandler for LuaModRuntime {
    fn resolve_command(
        &mut self,
        world: &mut WorldAccess,
        entity_id: EntityId,
        command_name: &str,
        args: &[SimValue],
    ) -> CommandHandlerResult {
        let Some(mod_id) = self.command_to_mod.get(command_name).cloned() else {
            return CommandHandlerResult::NotHandled;
        };

        let handle = self.alloc_handle();

        match coroutine::create_and_resume_command(
            &self.lua,
            &mod_id,
            command_name,
            entity_id,
            args,
            world,
            handle,
        ) {
            Ok(outcome) => {
                if let Some(entry) = outcome.coroutine_entry {
                    self.active_coroutines.insert(handle.0, entry);
                    CommandHandlerResult::Yielded {
                        events: outcome.events,
                        handle,
                        remaining_ticks: outcome.remaining_ticks,
                        interruptible: outcome.interruptible,
                    }
                } else {
                    CommandHandlerResult::Completed {
                        events: outcome.events,
                    }
                }
            }
            Err(e) => CommandHandlerResult::Error(format!("{e}")),
        }
    }

    fn resume_coroutine(
        &mut self,
        world: &mut WorldAccess,
        entity_id: EntityId,
        handle: CoroutineHandle,
    ) -> CommandHandlerResult {
        let Some(entry) = self.active_coroutines.remove(&handle.0) else {
            return CommandHandlerResult::Error("coroutine not found".into());
        };

        match coroutine::resume_coroutine(
            &self.lua,
            entry,
            entity_id,
            world,
            handle,
        ) {
            Ok(outcome) => {
                if let Some(entry) = outcome.coroutine_entry {
                    self.active_coroutines.insert(handle.0, entry);
                    CommandHandlerResult::Yielded {
                        events: outcome.events,
                        handle,
                        remaining_ticks: outcome.remaining_ticks,
                        interruptible: outcome.interruptible,
                    }
                } else {
                    CommandHandlerResult::Completed {
                        events: outcome.events,
                    }
                }
            }
            Err(e) => CommandHandlerResult::Error(format!("{e}")),
        }
    }

    fn cancel_coroutine(&mut self, handle: CoroutineHandle) {
        self.active_coroutines.remove(&handle.0);
    }

    fn process_triggers(
        &mut self,
        world: &mut WorldAccess,
        events: &[SimEvent],
    ) -> Vec<SimEvent> {
        triggers::process_lua_triggers(&self.lua, world, events)
    }

    fn buff_callback(
        &mut self,
        world: &mut WorldAccess,
        entity_id: EntityId,
        buff_name: &str,
        callback_type: BuffCallbackType,
    ) -> Vec<SimEvent> {
        triggers::run_buff_callback(&self.lua, world, entity_id, buff_name, callback_type)
    }

    fn run_init(&mut self, world: &mut WorldAccess) -> Vec<SimEvent> {
        api::run_init_handlers(&self.lua, world)
    }

    fn command_metadata(&self) -> Vec<(String, CommandMeta)> {
        api::collect_command_metadata(&self.lua)
    }

    fn reload_mod(&mut self, mod_id: &str, source: &str) -> Result<(), String> {
        // Cancel all coroutines for this mod
        let to_remove: Vec<u64> = self.active_coroutines.keys()
            .copied()
            .collect();
        for id in to_remove {
            // In a full implementation we'd track which mod owns which coroutine
            self.active_coroutines.remove(&id);
        }

        // Unregister old commands
        if let Some(old_cmds) = self.mod_commands.remove(mod_id) {
            for name in old_cmds {
                self.command_to_mod.remove(&name);
            }
        }

        // Clear old registrations
        api::clear_mod_tables(&self.lua, mod_id);

        // Re-execute
        self.load_mod_source(mod_id, source)
            .map_err(|e| format!("{e}"))
    }
}
