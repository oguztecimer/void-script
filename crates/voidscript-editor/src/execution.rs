use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::{HashMap, HashSet};
use std::thread::JoinHandle;

use voidscript_lang::{ScriptEvent, DebugCommand, OutputLevel};
use crate::ipc::*;
use crate::scripts::ScriptStore;
use crate::window::WebViewManager;

/// Tracks a running script execution
struct RunningScript {
    script_id: String,
    event_rx: Receiver<ScriptEvent>,
    command_tx: Sender<DebugCommand>,
    _handle: JoinHandle<()>,
    is_debug: bool,
}

/// Resource managing active script executions
#[derive(Resource, Default)]
pub struct ScriptExecutionManager {
    active: Option<RunningScript>,
    breakpoints: HashMap<String, HashSet<u32>>,
}

/// System: handle run_script events
pub fn handle_run_script(
    mut events: EventReader<RunScriptEvent>,
    script_store: Res<ScriptStore>,
    mut exec_manager: ResMut<ScriptExecutionManager>,
    webview_manager: NonSend<WebViewManager>,
) {
    for event in events.read() {
        // Stop any existing execution
        if let Some(active) = exec_manager.active.take() {
            let _ = active.command_tx.send(DebugCommand::Stop);
        }

        let Some(script) = script_store.scripts.get(&event.script_id) else {
            continue;
        };

        let source = script.content.clone();
        let script_id = event.script_id.clone();
        let (event_tx, event_rx) = unbounded();
        let (command_tx, command_rx) = unbounded();

        let handle = std::thread::spawn(move || {
            voidscript_lang::run_script(&source, event_tx, command_rx);
        });

        exec_manager.active = Some(RunningScript {
            script_id: script_id.clone(),
            event_rx,
            command_tx,
            _handle: handle,
            is_debug: false,
        });

        // Notify JS
        webview_manager.send_to_all(&RustToJs::ScriptStarted {
            script_id,
        });
    }
}

/// System: handle debug_start events
pub fn handle_debug_start(
    mut events: EventReader<DebugStartEvent>,
    script_store: Res<ScriptStore>,
    mut exec_manager: ResMut<ScriptExecutionManager>,
    webview_manager: NonSend<WebViewManager>,
) {
    for event in events.read() {
        // Stop any existing execution
        if let Some(active) = exec_manager.active.take() {
            let _ = active.command_tx.send(DebugCommand::Stop);
        }

        let Some(script) = script_store.scripts.get(&event.script_id) else {
            continue;
        };

        let source = script.content.clone();
        let script_id = event.script_id.clone();
        let breakpoints = exec_manager.breakpoints
            .get(&script_id)
            .cloned()
            .unwrap_or_default();
        let (event_tx, event_rx) = unbounded();
        let (command_tx, command_rx) = unbounded();

        let handle = std::thread::spawn(move || {
            voidscript_lang::debug_script(&source, event_tx, command_rx, breakpoints);
        });

        exec_manager.active = Some(RunningScript {
            script_id: script_id.clone(),
            event_rx,
            command_tx,
            _handle: handle,
            is_debug: true,
        });

        webview_manager.send_to_all(&RustToJs::ScriptStarted {
            script_id,
        });
    }
}

/// System: handle stop events
pub fn handle_stop_script(
    mut events: EventReader<StopScriptEvent>,
    exec_manager: Res<ScriptExecutionManager>,
) {
    for _event in events.read() {
        if let Some(active) = exec_manager.active.as_ref() {
            let _ = active.command_tx.send(DebugCommand::Stop);
        }
    }
}

/// System: handle debug continue/step events
pub fn handle_debug_commands(
    mut continue_events: EventReader<DebugContinueEvent>,
    mut step_over_events: EventReader<DebugStepOverEvent>,
    mut step_into_events: EventReader<DebugStepIntoEvent>,
    mut step_out_events: EventReader<DebugStepOutEvent>,
    exec_manager: Res<ScriptExecutionManager>,
    webview_manager: NonSend<WebViewManager>,
) {
    let Some(active) = exec_manager.active.as_ref() else { return };
    if !active.is_debug { return }

    for _event in continue_events.read() {
        let _ = active.command_tx.send(DebugCommand::Continue);
        webview_manager.send_to_all(&RustToJs::DebugResumed {
            script_id: active.script_id.clone(),
        });
    }
    for _event in step_over_events.read() {
        let _ = active.command_tx.send(DebugCommand::StepOver);
        webview_manager.send_to_all(&RustToJs::DebugResumed {
            script_id: active.script_id.clone(),
        });
    }
    for _event in step_into_events.read() {
        let _ = active.command_tx.send(DebugCommand::StepInto);
        webview_manager.send_to_all(&RustToJs::DebugResumed {
            script_id: active.script_id.clone(),
        });
    }
    for _event in step_out_events.read() {
        let _ = active.command_tx.send(DebugCommand::StepOut);
        webview_manager.send_to_all(&RustToJs::DebugResumed {
            script_id: active.script_id.clone(),
        });
    }
}

/// System: handle breakpoint toggles
pub fn handle_toggle_breakpoint(
    mut events: EventReader<ToggleBreakpointEvent>,
    mut exec_manager: ResMut<ScriptExecutionManager>,
) {
    for event in events.read() {
        let bps = exec_manager.breakpoints
            .entry(event.script_id.clone())
            .or_default();
        if bps.contains(&event.line) {
            bps.remove(&event.line);
        } else {
            bps.insert(event.line);
        }
        // Clone the breakpoint set before releasing the mutable borrow
        let bps_snapshot = bps.clone();
        // If actively debugging, send updated breakpoints to interpreter
        if let Some(active) = exec_manager.active.as_ref() {
            if active.script_id == event.script_id {
                let _ = active.command_tx.send(
                    DebugCommand::SetBreakpoints(bps_snapshot)
                );
            }
        }
    }
}

/// System: poll script execution events and forward to JS
pub fn poll_script_events(
    mut exec_manager: ResMut<ScriptExecutionManager>,
    webview_manager: NonSend<WebViewManager>,
) {
    let Some(active) = exec_manager.active.as_ref() else { return };

    // Process up to 100 events per frame to avoid blocking
    for _ in 0..100 {
        match active.event_rx.try_recv() {
            Ok(event) => {
                match event {
                    ScriptEvent::Output { line, level } => {
                        let level_str = match level {
                            OutputLevel::Info => "info",
                            OutputLevel::Warn => "warn",
                            OutputLevel::Error => "error",
                        };
                        webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                            text: line,
                            level: level_str.to_string(),
                        });
                    }
                    ScriptEvent::Paused { line, variables, call_stack } => {
                        let debug_vars: Vec<DebugVariable> = variables.into_iter().map(|v| {
                            DebugVariable {
                                name: v.name,
                                value: v.value,
                                var_type: v.var_type,
                            }
                        }).collect();
                        webview_manager.send_to_all(&RustToJs::DebugPaused {
                            script_id: active.script_id.clone(),
                            line,
                            variables: debug_vars,
                            call_stack,
                        });
                    }
                    ScriptEvent::Finished { success, error } => {
                        webview_manager.send_to_all(&RustToJs::ScriptFinished {
                            script_id: active.script_id.clone(),
                            success,
                            error,
                        });
                        // Can't clear active here due to borrow; handled below
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Check if finished (channel disconnected means thread ended)
    if exec_manager.active.as_ref().is_some_and(|a| a.event_rx.is_empty() && a._handle.is_finished()) {
        exec_manager.active = None;
    }
}
