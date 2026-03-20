use serde::{Deserialize, Serialize};

// Messages sent from Rust to JavaScript
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum RustToJs {
    #[serde(rename = "script_load")]
    ScriptLoad {
        script_id: String,
        name: String,
        content: String,
        script_type: String,
    },
    #[serde(rename = "error_update")]
    ErrorUpdate {
        script_id: String,
        diagnostics: Vec<Diagnostic>,
    },
    #[serde(rename = "script_list")]
    ScriptList {
        scripts: Vec<ScriptInfo>,
    },
    #[serde(rename = "tab_close")]
    TabClose {
        script_id: String,
    },
    #[serde(rename = "console_output")]
    ConsoleOutput {
        text: String,
        level: String, // "info", "warn", "error"
    },
    #[serde(rename = "script_started")]
    ScriptStarted {
        script_id: String,
    },
    #[serde(rename = "script_finished")]
    ScriptFinished {
        script_id: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "debug_paused")]
    DebugPaused {
        script_id: String,
        line: u32,
        variables: Vec<DebugVariable>,
        call_stack: Vec<String>,
    },
    #[serde(rename = "debug_resumed")]
    DebugResumed {
        script_id: String,
    },
    #[serde(rename = "terminal_finished")]
    TerminalFinished {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "simulation_started")]
    SimulationStarted,
    #[serde(rename = "simulation_stopped")]
    SimulationStopped,
    #[serde(rename = "simulation_tick")]
    SimulationTick {
        tick: u64,
    },
    #[serde(rename = "resource_update")]
    ResourceUpdate {
        resources: Vec<ResourceValue>,
    },
    #[serde(rename = "available_commands")]
    AvailableCommands {
        commands: Vec<String>,
        dev_mode: bool,
        #[serde(default)]
        command_info: Vec<CommandInfo>,
        #[serde(default)]
        resources: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceValue {
    pub name: String,
    pub value: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<i64>,
}

/// Metadata about a command (for editor autocomplete/syntax highlighting).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugVariable {
    pub name: String,
    pub value: String,
    pub var_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub line: u32,
    pub col_start: u32,
    pub col_end: u32,
    pub severity: String, // "error", "warning", "info"
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub script_type: String,
}

// Messages sent from JavaScript to Rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum JsToRust {
    #[serde(rename = "editor_ready")]
    EditorReady,
    #[serde(rename = "script_save")]
    ScriptSave {
        script_id: String,
        content: String,
    },
    #[serde(rename = "script_request")]
    ScriptRequest {
        script_id: String,
    },
    #[serde(rename = "script_list_request")]
    ScriptListRequest,
    #[serde(rename = "tab_changed")]
    TabChanged {
        script_id: String,
    },
    #[serde(rename = "run_script")]
    RunScript { script_id: String },
    #[serde(rename = "stop_script")]
    StopScript { script_id: String },
    #[serde(rename = "debug_start")]
    DebugStart { script_id: String },
    #[serde(rename = "debug_continue")]
    DebugContinue { script_id: String },
    #[serde(rename = "debug_step_over")]
    DebugStepOver { script_id: String },
    #[serde(rename = "debug_step_into")]
    DebugStepInto { script_id: String },
    #[serde(rename = "debug_step_out")]
    DebugStepOut { script_id: String },
    #[serde(rename = "toggle_breakpoint")]
    ToggleBreakpoint { script_id: String, line: u32 },
    #[serde(rename = "window_minimize")]
    WindowMinimize,
    #[serde(rename = "window_maximize")]
    WindowMaximize,
    #[serde(rename = "window_close")]
    WindowClose,
    #[serde(rename = "window_drag_start")]
    WindowDragStart,
    #[serde(rename = "window_resize_start")]
    WindowResizeStart { direction: String },
    #[serde(rename = "window_shake")]
    WindowShake,
    #[serde(rename = "window_set_size")]
    WindowSetSize { width: u32, height: u32, resizable: bool },
    #[serde(rename = "console_command")]
    ConsoleCommand { command: String },
    #[serde(rename = "start_simulation")]
    StartSimulation,
    #[serde(rename = "stop_simulation")]
    StopSimulation,
    #[serde(rename = "pause_simulation")]
    PauseSimulation,
}

pub enum WindowControlEvent {
    Minimize,
    Maximize,
    Close,
}
