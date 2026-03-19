use serde::{Deserialize, Serialize};

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
            error: None,
        }
    }
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
    pub energy: i64,
    pub max_energy: i64,
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

    // Script (None for non-scriptable entities)
    pub script_state: Option<ScriptState>,
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
            energy: 100,
            max_energy: 100,
            shield: 0,
            max_shield: 0,
            speed: 1,
            attack_damage: 10,
            attack_range: 5,
            attack_cooldown: 3,
            cooldown_remaining: 0,
            target: None,
            alive: true,
            script_state: None,
        }
    }
}
