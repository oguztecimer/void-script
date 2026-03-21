//! Deterministic simulation engine for VOID//SCRIPT.
//!
//! Runs GrimScript IR at 30 TPS. Each unit has a program counter and executes
//! instructions until hitting an action (which consumes the tick) or halting.
//!
//! Key design: no floats in simulation. All values are `i64`. Dicts use
//! `IndexMap<String, SimValue>` for deterministic insertion-order iteration
//! with O(1) amortized lookup. World positions are 1D integers.

#[cfg(feature = "compiler")]
pub mod compiler;

pub mod action;
pub mod entity;
pub mod error;
pub mod executor;
pub mod ir;
pub mod query;
pub mod rng;
pub mod value;
pub mod world;

pub use action::{BuffCallbackType, BuffDef, CommandDef, CommandHandler, CommandHandlerResult, CommandKind, CommandMeta, CoroutineHandle};
pub use entity::{EntityId, LuaCoroutineState, SimEntity};
pub use ir::{CompiledScript, Instruction};
pub use value::SimValue;
pub use world::{DEFAULT_WORLD_WIDTH, SimEvent, SimSnapshot, SimWorld, WorldAccess};

// Re-export indexmap for crates that need to work with SimWorld.resources.
pub use indexmap::IndexMap;
