use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ScriptEvent {
    Output {
        line: String,
        level: OutputLevel,
    },
    Paused {
        line: u32,
        variables: Vec<VariableInfo>,
        call_stack: Vec<String>,
    },
    Finished {
        success: bool,
        error: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum OutputLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub value: String,
    pub var_type: String,
}

#[derive(Debug, Clone)]
pub enum DebugCommand {
    Continue,
    StepOver,
    StepInto,
    StepOut,
    Stop,
    SetBreakpoints(HashSet<u32>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepMode {
    Run,
    StepOver { depth: usize },
    StepInto,
    StepOut { target_depth: usize },
}
