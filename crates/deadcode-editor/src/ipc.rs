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
}

pub enum WindowControlEvent {
    Minimize,
    Maximize,
    Close,
}
