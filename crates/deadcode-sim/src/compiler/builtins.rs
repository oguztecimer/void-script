use crate::action::CommandKind;
use crate::ir::Instruction;

/// Metadata about a command for the compiler.
#[derive(Clone, Debug)]
pub struct CommandMeta {
    pub num_args: usize,
    pub kind: CommandKind,
    pub implicit_self: bool,
}

/// Standard library builtins (always available, not gated).
pub enum StdlibBuiltin {
    Print,
    Len,
    Range,
    Abs,
    Min,
    Max,
    Int,
    Str,
    Type,
    Float, // compile error
    Percent,
    Scale,
}

/// Classify a function name as a stdlib builtin.
pub fn classify_stdlib(name: &str) -> Option<StdlibBuiltin> {
    match name {
        "print" => Some(StdlibBuiltin::Print),
        "len" => Some(StdlibBuiltin::Len),
        "range" => Some(StdlibBuiltin::Range),
        "abs" => Some(StdlibBuiltin::Abs),
        "min" => Some(StdlibBuiltin::Min),
        "max" => Some(StdlibBuiltin::Max),
        "int" => Some(StdlibBuiltin::Int),
        "str" => Some(StdlibBuiltin::Str),
        "type" => Some(StdlibBuiltin::Type),
        "float" => Some(StdlibBuiltin::Float),
        "percent" => Some(StdlibBuiltin::Percent),
        "scale" => Some(StdlibBuiltin::Scale),
        _ => None,
    }
}

/// Map a builtin command name to its IR instruction.
/// Returns None for custom (data-driven) commands and unknown names.
pub fn builtin_instruction(name: &str) -> Option<Instruction> {
    match name {
        // Queries
        "scan" => Some(Instruction::QueryScan),
        "nearest" => Some(Instruction::QueryNearest),
        "distance" => Some(Instruction::QueryDistance),
        "get_pos" => Some(Instruction::QueryGetPos),
        "get_health" => Some(Instruction::QueryGetHealth),
        "get_shield" => Some(Instruction::QueryGetShield),
        "get_target" => Some(Instruction::QueryGetTarget),
        "has_target" => Some(Instruction::QueryHasTarget),
        "get_type" => Some(Instruction::QueryGetType),
        "get_name" => Some(Instruction::QueryGetName),
        "get_owner" => Some(Instruction::QueryGetOwner),
        "get_resource" => Some(Instruction::QueryGetResource),
        "get_stat" | "get_custom_stat" => Some(Instruction::QueryGetStat),
        "get_types" => Some(Instruction::QueryGetTypes),
        "has_type" => Some(Instruction::QueryHasType),
        // Actions
        "move" => Some(Instruction::ActionMove),
        "attack" => Some(Instruction::ActionAttack),
        "flee" => Some(Instruction::ActionFlee),
        "wait" => Some(Instruction::ActionWait),
        "set_target" => Some(Instruction::ActionSetTarget),
        // Instant effects
        "gain_resource" => Some(Instruction::InstantGainResource),
        "try_spend_resource" => Some(Instruction::InstantTrySpendResource),
        _ => None,
    }
}
