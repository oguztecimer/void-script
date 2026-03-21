use serde::{Deserialize, Serialize};

use crate::entity::EntityId;
use crate::value::SimValue;
use crate::world::{SimEvent, SimWorld};

/// The kind of a command: how it compiles and executes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    /// Instant query — returns a value, does not consume tick. Supports method call syntax and implicit self.
    Query,
    /// Built-in action — consumes tick, executor yields.
    Action,
    /// Instant effect — returns a value, does not consume tick, but mutates world state via tick loop.
    Instant,
    /// Data-driven custom command — consumes tick.
    Custom,
}

impl Default for CommandKind {
    fn default() -> Self {
        CommandKind::Custom
    }
}

/// An action a unit wants to perform this tick.
#[derive(Debug, Clone)]
pub enum UnitAction {
    /// Do nothing for one tick.
    Wait,
    /// Print a value (not really a game action, but uses the same yield path).
    Print { text: String },
    /// Custom mod-defined command with resolved arguments.
    Custom { name: String, args: Vec<SimValue> },
}

/// Definition of a command (parsed from mod.toml).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandDef {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// If true, the command is hidden from `list_commands` output.
    #[serde(default)]
    pub unlisted: bool,
    /// The kind of command: query, action, instant, or custom (default).
    #[serde(default)]
    pub kind: CommandKind,
    /// If true, 0-arg calls auto-push `self` as first argument (queries only).
    #[serde(default)]
    pub implicit_self: bool,
}

/// A buff definition that can be applied to entities.
///
/// Defined in `[[buffs]]` sections in `mod.toml`. Buffs provide temporary
/// stat modifications with automatic expiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuffDef {
    pub name: String,
    /// Default duration in ticks.
    #[serde(default)]
    pub duration: i64,
    /// Stat modifiers applied while active (stat → amount).
    #[serde(default)]
    pub modifiers: indexmap::IndexMap<String, i64>,
    /// Whether multiple applications stack (true) or refresh duration (false).
    #[serde(default)]
    pub stackable: bool,
    /// Maximum stack count (0 = unlimited). Only relevant if stackable.
    #[serde(default)]
    pub max_stacks: i64,
}

/// Resolve a unit's action against the world state.
/// Mutates the world directly. Returns events generated.
pub fn resolve_action(
    world: &mut SimWorld,
    entity_id: EntityId,
    action: UnitAction,
) -> Vec<SimEvent> {
    let mut events = Vec::new();

    match action {
        UnitAction::Wait => {}

        UnitAction::Print { text } => {
            events.push(SimEvent::ScriptOutput { entity_id, text });
        }

        UnitAction::Custom { name, args } => {
            events.push(SimEvent::CommandUsed {
                entity_id,
                command: name.clone(),
            });

            if world.command_handler.is_some() {
                let mut handler = world.command_handler.take().unwrap();
                let mut access = crate::world::WorldAccess::new_from_world_ptr(world, entity_id);
                let result = handler.resolve_command(&mut access, entity_id, &name, &args);
                let lua_events = std::mem::take(&mut access.events);
                events.extend(lua_events);
                match result {
                    CommandHandlerResult::Completed { events: cmd_events } => {
                        events.extend(cmd_events);
                    }
                    CommandHandlerResult::Yielded { events: cmd_events, handle, remaining_ticks, interruptible } => {
                        events.extend(cmd_events);
                        if let Some(entity) = world.get_entity_mut(entity_id) {
                            entity.active_channel = Some(crate::entity::LuaCoroutineState {
                                handle,
                                command_name: name.clone(),
                                remaining_ticks,
                                interruptible,
                            });
                        }
                    }
                    CommandHandlerResult::Error(msg) => {
                        events.push(SimEvent::ScriptOutput {
                            entity_id,
                            text: format!("[lua error] {msg}"),
                        });
                    }
                    CommandHandlerResult::NotHandled => {
                        events.push(SimEvent::ScriptOutput {
                            entity_id,
                            text: format!("[{name}] (no handler)"),
                        });
                    }
                }
                world.command_handler = Some(handler);
            } else {
                events.push(SimEvent::ScriptOutput {
                    entity_id,
                    text: format!("[{name}] (no handler)"),
                });
            }
        }
    }

    events
}

/// Apply a buff's stat modifiers to an entity (one stack worth).
pub(crate) fn apply_buff_modifiers(world: &mut SimWorld, entity_id: EntityId, buff_def: &BuffDef) {
    if let Some(entity) = world.get_entity_mut(entity_id) {
        for (stat, amount) in &buff_def.modifiers {
            match stat.as_str() {
                "health" => {
                    let new_max = entity.stat("max_health").saturating_add(*amount).max(1);
                    entity.set_stat("max_health", new_max);
                    let new_health = entity.stat("health").saturating_add(*amount).max(1).min(new_max);
                    entity.set_stat("health", new_health);
                }
                "shield" => {
                    let new_max = entity.stat("max_shield").saturating_add(*amount).max(0);
                    entity.set_stat("max_shield", new_max);
                    let new_shield = entity.stat("shield").saturating_add(*amount).max(0).min(new_max);
                    entity.set_stat("shield", new_shield);
                }
                _ => {
                    let new_val = entity.stat(stat).saturating_add(*amount).max(0);
                    entity.set_stat(stat, new_val);
                }
            }
        }
    }
}

/// Reverse a buff's stat modifiers on an entity (one stack worth).
pub(crate) fn reverse_buff_modifiers(world: &mut SimWorld, entity_id: EntityId, buff_def: &BuffDef) {
    if let Some(entity) = world.get_entity_mut(entity_id) {
        for (stat, amount) in &buff_def.modifiers {
            match stat.as_str() {
                "health" => {
                    let new_max = entity.stat("max_health").saturating_sub(*amount).max(1);
                    entity.set_stat("max_health", new_max);
                    let clamped = entity.stat("health").min(new_max).max(1);
                    entity.set_stat("health", clamped);
                }
                "shield" => {
                    let new_max = entity.stat("max_shield").saturating_sub(*amount).max(0);
                    entity.set_stat("max_shield", new_max);
                    let clamped = entity.stat("shield").min(new_max).max(0);
                    entity.set_stat("shield", clamped);
                }
                _ => {
                    let new_val = entity.stat(stat).saturating_sub(*amount).max(0);
                    entity.set_stat(stat, new_val);
                }
            }
        }
    }
}

/// Handle for a suspended Lua coroutine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoroutineHandle(pub u64);

/// Metadata about a command (for the compiler and list_commands).
#[derive(Debug, Clone)]
pub struct CommandMeta {
    pub description: String,
    pub args: Vec<String>,
    pub unlisted: bool,
}

/// Which buff callback to invoke.
#[derive(Debug, Clone, Copy)]
pub enum BuffCallbackType {
    OnApply,
    PerTick,
    OnExpire,
}

/// Result of a command handler invocation.
pub enum CommandHandlerResult {
    /// Command completed in a single tick.
    Completed { events: Vec<crate::world::SimEvent> },
    /// Command yielded — coroutine is suspended, resume after `remaining_ticks`.
    Yielded {
        events: Vec<crate::world::SimEvent>,
        handle: CoroutineHandle,
        remaining_ticks: i64,
        interruptible: bool,
    },
    /// Command name not handled by this handler.
    NotHandled,
    /// Error during execution.
    Error(String),
}

/// Trait for external command/trigger/buff handlers (implemented by `deadcode-lua`).
///
/// The sim engine calls into this trait at specific points in the tick loop.
/// The handler receives a `WorldAccess` that provides safe, controlled access
/// to the simulation state.
pub trait CommandHandler {
    /// Resolve a custom command. Returns events and optional coroutine state.
    fn resolve_command(
        &mut self,
        world: &mut crate::world::WorldAccess,
        entity_id: EntityId,
        command_name: &str,
        args: &[SimValue],
    ) -> CommandHandlerResult;

    /// Resume a suspended coroutine. Called each tick when remaining_ticks reaches 0.
    fn resume_coroutine(
        &mut self,
        world: &mut crate::world::WorldAccess,
        entity_id: EntityId,
        handle: CoroutineHandle,
    ) -> CommandHandlerResult;

    /// Cancel an active coroutine (interruption or hot-reload).
    fn cancel_coroutine(&mut self, handle: CoroutineHandle);

    /// Process triggers for collected events.
    fn process_triggers(
        &mut self,
        world: &mut crate::world::WorldAccess,
        events: &[crate::world::SimEvent],
    ) -> Vec<crate::world::SimEvent>;

    /// Run buff callbacks (on_apply, per_tick, on_expire).
    fn buff_callback(
        &mut self,
        world: &mut crate::world::WorldAccess,
        entity_id: EntityId,
        buff_name: &str,
        callback_type: BuffCallbackType,
    ) -> Vec<crate::world::SimEvent>;

    /// Run initialization effects.
    fn run_init(&mut self, world: &mut crate::world::WorldAccess) -> Vec<crate::world::SimEvent>;

    /// Get command metadata for the compiler.
    fn command_metadata(&self) -> Vec<(String, CommandMeta)>;

    /// Hot-reload a mod's Lua scripts.
    fn reload_mod(&mut self, mod_id: &str, source: &str) -> Result<(), String>;
}
