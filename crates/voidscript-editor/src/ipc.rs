use serde::{Deserialize, Serialize};
use crossbeam_channel::{Receiver, Sender};
use bevy::prelude::*;

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
}

// Channel resource for sending IPC messages from wry thread to Bevy
#[derive(Resource)]
pub struct IpcChannelReceiver(pub Receiver<JsToRust>);

pub struct IpcChannelSender(pub Sender<JsToRust>);

// Bevy events
#[derive(Event)]
pub struct EditorReadyEvent;

#[derive(Event)]
pub struct ScriptSaveEvent {
    pub script_id: String,
    pub content: String,
}

#[derive(Event)]
pub struct ScriptRequestEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct ScriptListRequestEvent;

#[derive(Event)]
pub struct TabChangedEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct RunScriptEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct StopScriptEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct DebugStartEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct DebugContinueEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct DebugStepOverEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct DebugStepIntoEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct DebugStepOutEvent {
    pub script_id: String,
}

#[derive(Event)]
pub struct ToggleBreakpointEvent {
    pub script_id: String,
    pub line: u32,
}

pub fn poll_ipc_messages(
    receiver: Res<IpcChannelReceiver>,
    mut ready_events: EventWriter<EditorReadyEvent>,
    mut save_events: EventWriter<ScriptSaveEvent>,
    mut request_events: EventWriter<ScriptRequestEvent>,
    mut list_events: EventWriter<ScriptListRequestEvent>,
    mut tab_events: EventWriter<TabChangedEvent>,
    mut run_events: EventWriter<RunScriptEvent>,
    mut stop_events: EventWriter<StopScriptEvent>,
    mut debug_start_events: EventWriter<DebugStartEvent>,
    mut debug_continue_events: EventWriter<DebugContinueEvent>,
    mut debug_step_over_events: EventWriter<DebugStepOverEvent>,
    mut debug_step_into_events: EventWriter<DebugStepIntoEvent>,
    mut debug_step_out_events: EventWriter<DebugStepOutEvent>,
    mut toggle_bp_events: EventWriter<ToggleBreakpointEvent>,
) {
    while let Ok(msg) = receiver.0.try_recv() {
        match msg {
            JsToRust::EditorReady => {
                ready_events.send(EditorReadyEvent);
            }
            JsToRust::ScriptSave { script_id, content } => {
                save_events.send(ScriptSaveEvent { script_id, content });
            }
            JsToRust::ScriptRequest { script_id } => {
                request_events.send(ScriptRequestEvent { script_id });
            }
            JsToRust::ScriptListRequest => {
                list_events.send(ScriptListRequestEvent);
            }
            JsToRust::TabChanged { script_id } => {
                tab_events.send(TabChangedEvent { script_id });
            }
            JsToRust::RunScript { script_id } => {
                run_events.send(RunScriptEvent { script_id });
            }
            JsToRust::StopScript { script_id } => {
                stop_events.send(StopScriptEvent { script_id });
            }
            JsToRust::DebugStart { script_id } => {
                debug_start_events.send(DebugStartEvent { script_id });
            }
            JsToRust::DebugContinue { script_id } => {
                debug_continue_events.send(DebugContinueEvent { script_id });
            }
            JsToRust::DebugStepOver { script_id } => {
                debug_step_over_events.send(DebugStepOverEvent { script_id });
            }
            JsToRust::DebugStepInto { script_id } => {
                debug_step_into_events.send(DebugStepIntoEvent { script_id });
            }
            JsToRust::DebugStepOut { script_id } => {
                debug_step_out_events.send(DebugStepOutEvent { script_id });
            }
            JsToRust::ToggleBreakpoint { script_id, line } => {
                toggle_bp_events.send(ToggleBreakpointEvent { script_id, line });
            }
        }
    }
}
