use serde::{Deserialize, Serialize};

use crate::value::SimValue;

/// Stack-based IR instruction for the simulation executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    // --- Stack operations ---
    /// Push a constant value onto the stack.
    LoadConst(SimValue),
    /// Push variable from slot onto the stack.
    LoadVar(usize),
    /// Pop top of stack into variable slot.
    StoreVar(usize),
    /// Discard top of stack.
    Pop,
    /// Duplicate top of stack.
    Dup,

    // --- Arithmetic (pop 2, push 1) ---
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    /// Unary negate (pop 1, push 1).
    Negate,

    // --- Comparison (pop 2, push Bool) ---
    CmpEq,
    CmpNe,
    CmpLt,
    CmpGt,
    CmpLe,
    CmpGe,
    /// `in` operator: pop item and container, push Bool (item in container).
    Contains,
    /// `not in` operator: pop item and container, push Bool (item not in container).
    NotContains,

    // --- Boolean (pop 1, push Bool) ---
    Not,
    IsNone,
    IsNotNone,

    // --- Control flow ---
    /// Unconditional jump to instruction index.
    Jump(usize),
    /// Pop top; jump if falsy.
    JumpIfFalse(usize),
    /// Pop top; jump if truthy.
    JumpIfTrue(usize),

    // --- Functions ---
    /// Call function: jump to target, push frame. Arg = (target_pc, num_args).
    Call(usize, usize),
    /// Return from function; pops frame, pushes return value.
    Return,

    // --- Data structures ---
    /// Build a list from top N stack values.
    BuildList(usize),
    /// Build a dict from top N*2 stack values (key, value pairs).
    BuildDict(usize),
    /// Pop index, pop collection, push element.
    Index,
    /// Pop value, pop index, pop collection, store value at index, push collection.
    StoreIndex,
    /// Pop string attr name, pop value, get attribute.
    GetAttr,

    // --- Action instructions (consume tick — executor yields after these) ---
    /// Custom action defined by mods. Args are already on the stack.
    /// The String is the command name; arg count is looked up from the command registry.
    ActionCustom(String),

    // --- Local variable access (var_base-relative for function params/locals) ---
    /// Load function-local variable at var_base + offset.
    LoadLocal(usize),
    /// Store to function-local variable at var_base + offset.
    StoreLocal(usize),

    // --- Standard library builtins ---
    /// Pop collection (list/str/dict), push length as Int.
    Len,
    /// Pop Int, push absolute value.
    Abs,
    /// Pop value, push Int conversion (Bool→0/1, Str→parse, Int→identity).
    IntCast,
    /// Pop value, push String representation.
    StrCast,
    /// Pop value, push type name as Str.
    TypeOf,
    /// Build range list. Pop `nargs` Int values, push List. nargs = 1/2/3.
    Range(u8),
    /// Pop value, pop list, push list with value appended.
    ListAppend,
    /// Pop 2 Ints, push minimum.
    Min2,
    /// Pop 2 Ints, push maximum.
    Max2,
    /// Pop dict, push list of keys.
    DictKeys,
    /// Pop dict, push list of values.
    DictValues,
    /// Pop dict, push list of [key, value] pairs (as lists).
    DictItems,
    /// Pop default, pop key (Str), pop dict, push value or default.
    DictGet,
    /// Pop pct (Int), pop value (Int), push `value * pct / 100` with banker's rounding.
    Percent,
    /// Pop den (Int), pop num (Int), pop value (Int), push `value * num / den` with banker's rounding.
    Scale,

    // --- Misc ---
    /// Pop value, emit as script output.
    Print,
    /// Halt execution.
    Halt,
}

/// A function entry point within a compiled script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionEntry {
    pub name: String,
    pub pc: usize,
    pub num_params: usize,
    pub num_locals: usize,
}

/// A compiled script: a flat instruction sequence plus function table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledScript {
    pub instructions: Vec<Instruction>,
    pub functions: Vec<FunctionEntry>,
    /// Total number of variable slots needed (params + locals + temporaries).
    pub num_variables: usize,
    /// PC of the auto-generated `Call brain()` instruction.
    /// When present, the script loops by jumping to this PC instead of PC=0,
    /// preserving global variables across ticks.
    pub brain_entry_pc: Option<usize>,
}

impl CompiledScript {
    pub fn new(instructions: Vec<Instruction>, num_variables: usize) -> Self {
        Self {
            instructions,
            functions: Vec::new(),
            num_variables,
            brain_entry_pc: None,
        }
    }
}
