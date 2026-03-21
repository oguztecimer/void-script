use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

use crate::action::{CoroutineHandle, PhaseDef};
use crate::ir::CompiledScript;
use crate::value::SimValue;

/// Unique entity identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Call frame for the executor's call stack.
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Program counter to return to.
    pub return_pc: usize,
    /// Value stack depth at call time (for cleanup on return).
    pub stack_base: usize,
    /// Variable slot base (locals start here).
    pub var_base: usize,
}

/// Per-entity script execution state.
///
/// Taken out of the entity during execution, then put back.
/// This avoids borrow conflicts when the executor needs `&SimWorld`.
#[derive(Debug, Clone)]
pub struct ScriptState {
    pub program: CompiledScript,
    pub pc: usize,
    pub stack: Vec<SimValue>,
    pub variables: Vec<SimValue>,
    pub call_stack: Vec<CallFrame>,
    /// True if the unit yielded (action consumed the tick).
    pub yielded: bool,
    /// True if the unit hit the step limit and was auto-yielded.
    pub step_limit_hit: bool,
    /// Set on unrecoverable error — unit stops executing.
    pub error: Option<String>,
    /// Brain scripts implicitly loop (restart from top when they halt).
    pub is_brain: bool,
}

impl ScriptState {
    pub fn new(program: CompiledScript, num_variables: usize) -> Self {
        Self {
            program,
            pc: 0,
            stack: Vec::with_capacity(64),
            variables: vec![SimValue::None; num_variables],
            call_stack: Vec::new(),
            yielded: false,
            step_limit_hit: false,
            error: None,
            is_brain: false,
        }
    }

    /// Reset script to restart from the beginning on the next tick.
    /// Preserves the program and variable slot count; clears execution state.
    /// Sets variables[0] = EntityRef(entity_id) if variables exist.
    pub fn reset_for_restart(&mut self, entity_id: EntityId) {
        self.pc = 0;
        self.stack.clear();
        self.call_stack.clear();
        self.yielded = false;
        self.step_limit_hit = false;
        let num_vars = self.variables.len();
        self.variables = vec![SimValue::None; num_vars];
        if num_vars > 0 {
            self.variables[0] = SimValue::EntityRef(entity_id);
        }
    }
}

/// An active buff on an entity.
#[derive(Debug, Clone)]
pub struct ActiveBuff {
    pub name: String,
    pub remaining_ticks: i64,
    pub stacks: i64,
}

/// Active channel state for a multi-tick phased command.
#[derive(Debug, Clone)]
pub struct ChannelState {
    pub command_name: String,
    pub args: Vec<SimValue>,
    pub phases: Vec<PhaseDef>,
    pub phase_index: usize,
    pub ticks_elapsed_in_phase: i64,
}

/// Active Lua coroutine state for a yielded command.
#[derive(Debug, Clone)]
pub struct LuaCoroutineState {
    pub handle: CoroutineHandle,
    pub command_name: String,
    pub remaining_ticks: i64,
    pub interruptible: bool,
}

/// Union of TOML channel and Lua coroutine active states.
#[derive(Debug, Clone)]
pub enum ActiveChannel {
    /// Legacy TOML-based phased command.
    Toml(ChannelState),
    /// Lua coroutine-based yielded command.
    Lua(LuaCoroutineState),
}

/// Stat overrides applied at spawn time. All stats live in a single HashMap.
#[derive(Debug, Clone, Default)]
pub struct EntityConfig {
    pub stats: IndexMap<String, i64>,
}

/// A game entity — theme-agnostic. Entity type is a free-form string.
#[derive(Debug, Clone)]
pub struct SimEntity {
    pub id: EntityId,
    /// Free-form type string (e.g., "skeleton", "summoner", "grave").
    /// Used as the unique entity definition ID for sprite/config registry lookups.
    pub entity_type: String,
    /// Composable type tags for queries and filtering.
    /// An entity can have multiple types (e.g., ["undead", "melee", "skeleton_ai"]).
    /// If empty, behaves as if it contains just `entity_type`.
    pub types: Vec<String>,
    pub name: String,
    pub owner: Option<EntityId>,

    // Position (1D)
    pub position: i64,

    /// All entity stats in a single HashMap (health, max_health, shield, speed, etc.).
    pub stats: IndexMap<String, i64>,

    // State
    pub target: Option<EntityId>,
    pub alive: bool,
    /// Ticks remaining in spawn animation. While > 0, entity can't act or be targeted.
    pub spawn_ticks_remaining: i64,

    // Script (None for non-scriptable entities)
    pub script_state: Option<ScriptState>,

    /// Active channel for a multi-tick phased command or Lua coroutine (None when idle).
    pub active_channel: Option<ActiveChannel>,

    /// Active buffs on this entity.
    pub active_buffs: Vec<ActiveBuff>,
}

impl SimEntity {
    pub fn new(id: EntityId, entity_type: String, name: String, position: i64) -> Self {
        let types = vec![entity_type.clone()];
        let stats = IndexMap::new();
        Self {
            id,
            entity_type,
            types,
            name,
            owner: None,
            position,
            stats,
            target: None,
            alive: true,
            spawn_ticks_remaining: 0,
            script_state: None,
            active_channel: None,
            active_buffs: Vec::new(),
        }
    }

    /// Create a new entity with explicit type tags.
    pub fn new_with_types(id: EntityId, entity_type: String, types: Vec<String>, name: String, position: i64) -> Self {
        let stats = IndexMap::new();
        Self {
            id,
            entity_type,
            types,
            name,
            owner: None,
            position,
            stats,
            target: None,
            alive: true,
            spawn_ticks_remaining: 0,
            script_state: None,
            active_channel: None,
            active_buffs: Vec::new(),
        }
    }

    /// Check if this entity has a given type tag.
    pub fn has_type(&self, tag: &str) -> bool {
        self.types.iter().any(|t| t == tag)
    }

    /// Get a stat value (returns 0 if not set).
    pub fn stat(&self, name: &str) -> i64 {
        self.stats.get(name).copied().unwrap_or(0)
    }

    /// Set a stat value.
    pub fn set_stat(&mut self, name: &str, value: i64) {
        self.stats.insert(name.to_string(), value);
    }

    /// Clamp a stat to `[0, max_{name}]` if a max exists, else `[0, +inf)`.
    pub fn clamp_stat(&mut self, name: &str) {
        let max_key = format!("max_{name}");
        let value = self.stat(name);
        let clamped = if let Some(&max) = self.stats.get(&max_key) {
            value.max(0).min(max)
        } else {
            value.max(0)
        };
        self.set_stat(name, clamped);
    }

    /// Whether this entity is fully spawned and can act/be targeted.
    pub fn is_ready(&self) -> bool {
        self.alive && self.spawn_ticks_remaining <= 0
    }

    /// Apply stat overrides from an `EntityConfig`.
    pub fn apply_config(&mut self, config: &EntityConfig) {
        for (name, &value) in &config.stats {
            self.stats.insert(name.clone(), value);
        }
        // Auto-set max_health from health if health is set but max_health isn't in config.
        if config.stats.contains_key("health") && !config.stats.contains_key("max_health") {
            let h = self.stat("health");
            self.set_stat("max_health", h);
        }
        // Auto-set max_shield from shield if shield is set but max_shield isn't in config.
        if config.stats.contains_key("shield") && !config.stats.contains_key("max_shield") {
            let s = self.stat("shield");
            self.set_stat("max_shield", s);
        }
    }
}
