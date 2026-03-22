use crate::action::CommandKind;

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
    Random,
    Wait,
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
        "random" => Some(StdlibBuiltin::Random),
        "wait" => Some(StdlibBuiltin::Wait),
        _ => None,
    }
}
