use crate::ir::Instruction;

/// Classification of a builtin function call.
pub enum BuiltinKind {
    /// Game query — instant, does not consume tick.
    Query(QueryBuiltin),
    /// Game action — consumes tick, executor yields.
    Action(ActionBuiltin),
    /// Standard library function.
    Stdlib(StdlibBuiltin),
    /// Not a builtin.
    NotBuiltin,
}

pub enum QueryBuiltin {
    Scan,
    Nearest,
    Distance,
    GetPos,
    GetHealth,
    GetEnergy,
    GetShield,
    GetTarget,
    HasTarget,
    GetType,
    GetName,
    GetOwner,
}

pub enum ActionBuiltin {
    Move,
    Attack,
    Flee,
    Wait,
    SetTarget,
    Consult,
    Raise,
    Harvest,
    Pact,
}

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
}

/// Classify a function name as a builtin.
pub fn classify(name: &str) -> BuiltinKind {
    match name {
        // Queries
        "scan" => BuiltinKind::Query(QueryBuiltin::Scan),
        "nearest" => BuiltinKind::Query(QueryBuiltin::Nearest),
        "distance" => BuiltinKind::Query(QueryBuiltin::Distance),
        "get_pos" => BuiltinKind::Query(QueryBuiltin::GetPos),
        "get_health" => BuiltinKind::Query(QueryBuiltin::GetHealth),
        "get_energy" => BuiltinKind::Query(QueryBuiltin::GetEnergy),
        "get_shield" => BuiltinKind::Query(QueryBuiltin::GetShield),
        "get_target" => BuiltinKind::Query(QueryBuiltin::GetTarget),
        "has_target" => BuiltinKind::Query(QueryBuiltin::HasTarget),
        "get_type" => BuiltinKind::Query(QueryBuiltin::GetType),
        "get_name" => BuiltinKind::Query(QueryBuiltin::GetName),
        "get_owner" => BuiltinKind::Query(QueryBuiltin::GetOwner),
        // Actions
        "move" => BuiltinKind::Action(ActionBuiltin::Move),
        "attack" => BuiltinKind::Action(ActionBuiltin::Attack),
        "flee" => BuiltinKind::Action(ActionBuiltin::Flee),
        "wait" => BuiltinKind::Action(ActionBuiltin::Wait),
        "set_target" => BuiltinKind::Action(ActionBuiltin::SetTarget),
        "consult" => BuiltinKind::Action(ActionBuiltin::Consult),
        "raise" => BuiltinKind::Action(ActionBuiltin::Raise),
        "harvest" => BuiltinKind::Action(ActionBuiltin::Harvest),
        "pact" => BuiltinKind::Action(ActionBuiltin::Pact),
        // Stdlib
        "print" => BuiltinKind::Stdlib(StdlibBuiltin::Print),
        "len" => BuiltinKind::Stdlib(StdlibBuiltin::Len),
        "range" => BuiltinKind::Stdlib(StdlibBuiltin::Range),
        "abs" => BuiltinKind::Stdlib(StdlibBuiltin::Abs),
        "min" => BuiltinKind::Stdlib(StdlibBuiltin::Min),
        "max" => BuiltinKind::Stdlib(StdlibBuiltin::Max),
        "int" => BuiltinKind::Stdlib(StdlibBuiltin::Int),
        "str" => BuiltinKind::Stdlib(StdlibBuiltin::Str),
        "type" => BuiltinKind::Stdlib(StdlibBuiltin::Type),
        "float" => BuiltinKind::Stdlib(StdlibBuiltin::Float),
        _ => BuiltinKind::NotBuiltin,
    }
}

/// Get the IR instruction for a query builtin.
pub fn query_instruction(q: &QueryBuiltin) -> Instruction {
    match q {
        QueryBuiltin::Scan => Instruction::QueryScan,
        QueryBuiltin::Nearest => Instruction::QueryNearest,
        QueryBuiltin::Distance => Instruction::QueryDistance,
        QueryBuiltin::GetPos => Instruction::QueryGetPos,
        QueryBuiltin::GetHealth => Instruction::QueryGetHealth,
        QueryBuiltin::GetEnergy => Instruction::QueryGetEnergy,
        QueryBuiltin::GetShield => Instruction::QueryGetShield,
        QueryBuiltin::GetTarget => Instruction::QueryGetTarget,
        QueryBuiltin::HasTarget => Instruction::QueryHasTarget,
        QueryBuiltin::GetType => Instruction::QueryGetType,
        QueryBuiltin::GetName => Instruction::QueryGetName,
        QueryBuiltin::GetOwner => Instruction::QueryGetOwner,
    }
}

/// Get the IR instruction for an action builtin.
pub fn action_instruction(a: &ActionBuiltin) -> Instruction {
    match a {
        ActionBuiltin::Move => Instruction::ActionMove,
        ActionBuiltin::Attack => Instruction::ActionAttack,
        ActionBuiltin::Flee => Instruction::ActionFlee,
        ActionBuiltin::Wait => Instruction::ActionWait,
        ActionBuiltin::SetTarget => Instruction::ActionSetTarget,
        ActionBuiltin::Consult => Instruction::ActionConsult,
        ActionBuiltin::Raise => Instruction::ActionRaise,
        ActionBuiltin::Harvest => Instruction::ActionHarvest,
        ActionBuiltin::Pact => Instruction::ActionPact,
    }
}

/// Whether a query takes an implicit `self` argument when called with 0 args.
pub fn query_takes_implicit_self(q: &QueryBuiltin) -> bool {
    matches!(
        q,
        QueryBuiltin::GetPos
            | QueryBuiltin::GetHealth
            | QueryBuiltin::GetEnergy
            | QueryBuiltin::GetShield
            | QueryBuiltin::GetTarget
            | QueryBuiltin::HasTarget
    )
}

/// Expected number of explicit arguments for a query.
pub fn query_expected_args(q: &QueryBuiltin) -> usize {
    match q {
        QueryBuiltin::Scan | QueryBuiltin::Nearest => 1,
        QueryBuiltin::Distance => 2,
        QueryBuiltin::GetPos
        | QueryBuiltin::GetHealth
        | QueryBuiltin::GetEnergy
        | QueryBuiltin::GetShield
        | QueryBuiltin::GetTarget
        | QueryBuiltin::HasTarget
        | QueryBuiltin::GetType
        | QueryBuiltin::GetName
        | QueryBuiltin::GetOwner => 1,
    }
}

/// Whether an action is a "yields" action that the expression statement
/// should NOT emit Pop after (the action instruction already consumed the args).
pub fn action_is_void(_a: &ActionBuiltin) -> bool {
    true // All actions yield and don't push a result.
}

/// Expected number of arguments for an action.
pub fn action_expected_args(a: &ActionBuiltin) -> usize {
    match a {
        ActionBuiltin::Move => 1,
        ActionBuiltin::Attack => 1,
        ActionBuiltin::Flee => 1,
        ActionBuiltin::Wait => 0,
        ActionBuiltin::SetTarget => 1,
        ActionBuiltin::Consult => 0,
        ActionBuiltin::Raise => 0,
        ActionBuiltin::Harvest => 0,
        ActionBuiltin::Pact => 0,
    }
}
