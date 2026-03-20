use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

use crate::action::PhaseDef;
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
    pub entity_type: String,
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

    /// Active channel for a multi-tick phased command (None when idle).
    pub active_channel: Option<ChannelState>,

    /// Active buffs on this entity.
    pub active_buffs: Vec<ActiveBuff>,
}

impl SimEntity {
    pub fn new(id: EntityId, entity_type: String, name: String, position: i64) -> Self {
        let stats = IndexMap::new();
        Self {
            id,
            entity_type,
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
