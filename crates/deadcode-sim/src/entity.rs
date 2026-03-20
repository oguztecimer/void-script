use std::collections::HashMap;

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

/// Optional stat overrides applied at spawn time.
#[derive(Debug, Clone, Default)]
pub struct EntityConfig {
    pub health: Option<i64>,
    pub speed: Option<i64>,
    pub attack_damage: Option<i64>,
    pub attack_range: Option<i64>,
    pub attack_cooldown: Option<i64>,
    pub shield: Option<i64>,
    /// Mod-defined custom stats.
    pub custom_stats: HashMap<String, i64>,
}

/// A game entity — theme-agnostic. Entity type is a free-form string.
#[derive(Debug, Clone)]
pub struct SimEntity {
    pub id: EntityId,
    /// Free-form type string (e.g., "skeleton", "summoner", "grave").
    pub entity_type: String,
    pub name: String,
    pub owner: u64,

    // Position (1D)
    pub position: i64,

    // Stats
    pub health: i64,
    pub max_health: i64,
    pub shield: i64,
    pub max_shield: i64,

    // Combat
    pub speed: i64,
    pub attack_damage: i64,
    pub attack_range: i64,
    pub attack_cooldown: i64,
    pub cooldown_remaining: i64,

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

    /// Mod-defined custom stats (e.g., armor, crit_chance).
    pub custom_stats: HashMap<String, i64>,
}

impl SimEntity {
    pub fn new(id: EntityId, entity_type: String, name: String, position: i64) -> Self {
        Self {
            id,
            entity_type,
            name,
            owner: 0,
            position,
            health: 100,
            max_health: 100,
            shield: 0,
            max_shield: 0,
            speed: 1,
            attack_damage: 10,
            attack_range: 5,
            attack_cooldown: 3,
            cooldown_remaining: 0,
            target: None,
            alive: true,
            spawn_ticks_remaining: 0,
            script_state: None,
            active_channel: None,
            active_buffs: Vec::new(),
            custom_stats: HashMap::new(),
        }
    }

    /// Whether this entity is fully spawned and can act/be targeted.
    pub fn is_ready(&self) -> bool {
        self.alive && self.spawn_ticks_remaining <= 0
    }

    /// Apply optional stat overrides from an `EntityConfig`.
    pub fn apply_config(&mut self, config: &EntityConfig) {
        if let Some(h) = config.health {
            self.health = h;
            self.max_health = h;
        }
        if let Some(s) = config.speed {
            self.speed = s;
        }
        if let Some(d) = config.attack_damage {
            self.attack_damage = d;
        }
        if let Some(r) = config.attack_range {
            self.attack_range = r;
        }
        if let Some(c) = config.attack_cooldown {
            self.attack_cooldown = c;
        }
        if let Some(s) = config.shield {
            self.shield = s;
            self.max_shield = s;
        }
        for (name, value) in &config.custom_stats {
            self.custom_stats.insert(name.clone(), *value);
        }
    }
}
