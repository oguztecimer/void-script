use serde::{Deserialize, Serialize};

use crate::ir::CompiledScript;
use crate::value::SimValue;

/// Unique entity identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// What kind of entity this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Miner,
    Fighter,
    Scout,
    Hauler,
    Mothership,
    Asteroid,
    Station,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Miner => "miner",
            EntityType::Fighter => "fighter",
            EntityType::Scout => "scout",
            EntityType::Hauler => "hauler",
            EntityType::Mothership => "mothership",
            EntityType::Asteroid => "asteroid",
            EntityType::Station => "station",
        }
    }

    /// Whether this entity type can run scripts.
    pub fn is_scriptable(&self) -> bool {
        matches!(
            self,
            EntityType::Miner
                | EntityType::Fighter
                | EntityType::Scout
                | EntityType::Hauler
                | EntityType::Mothership
        )
    }
}

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

/// A game entity: unit, asteroid, station, etc.
#[derive(Debug, Clone)]
pub struct SimEntity {
    pub id: EntityId,
    pub entity_type: EntityType,
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

    // Cargo
    pub cargo: Vec<(String, i64)>,
    pub cargo_capacity: i64,

    // Combat
    pub speed: i64,
    pub attack_damage: i64,
    pub attack_range: i64,
    pub attack_cooldown: i64,
    pub cooldown_remaining: i64,

    // Mining
    pub mine_range: i64,
    pub mine_amount: i64,

    // State
    pub target: Option<EntityId>,
    pub alive: bool,

    // Script (None for non-scriptable entities like asteroids)
    pub script_state: Option<ScriptState>,
}

impl SimEntity {
    pub fn new(id: EntityId, entity_type: EntityType, name: String, position: i64) -> Self {
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
            cargo: Vec::new(),
            cargo_capacity: 0,
            speed: 1,
            attack_damage: 10,
            attack_range: 5,
            attack_cooldown: 3,
            cooldown_remaining: 0,
            mine_range: 3,
            mine_amount: 5,
            target: None,
            alive: true,
            script_state: None,
        }
    }

    /// Total cargo currently held.
    pub fn cargo_total(&self) -> i64 {
        self.cargo.iter().map(|(_, amt)| amt).sum()
    }

    /// Whether cargo is at capacity.
    pub fn cargo_full(&self) -> bool {
        self.cargo_total() >= self.cargo_capacity
    }

    /// Add cargo, clamped to capacity. Returns amount actually added.
    pub fn add_cargo(&mut self, resource: &str, amount: i64) -> i64 {
        let space = (self.cargo_capacity - self.cargo_total()).max(0);
        let added = amount.min(space);
        if added <= 0 {
            return 0;
        }
        if let Some(entry) = self.cargo.iter_mut().find(|(r, _)| r == resource) {
            entry.1 += added;
        } else {
            self.cargo.push((resource.to_string(), added));
        }
        added
    }

    /// Remove cargo. Returns amount actually removed.
    pub fn remove_cargo(&mut self, resource: &str, amount: i64) -> i64 {
        if let Some(entry) = self.cargo.iter_mut().find(|(r, _)| r == resource) {
            let removed = amount.min(entry.1);
            entry.1 -= removed;
            if entry.1 <= 0 {
                self.cargo.retain(|(r, _)| r != resource);
            }
            removed
        } else {
            0
        }
    }
}
