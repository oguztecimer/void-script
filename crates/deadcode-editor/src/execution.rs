use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::{HashMap, HashSet};
use std::thread::JoinHandle;
use std::sync::Arc;

use grimscript_lang::{ScriptEvent, DebugCommand, OutputLevel};

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

/// Tracks a running terminal command
struct RunningTerminalCommand {
    event_rx: Receiver<ScriptEvent>,
    _handle: JoinHandle<()>,
}

/// Manages active script executions
#[derive(Default)]
pub struct ScriptExecutionManager {
    active: Option<RunningScript>,
    terminal: Option<RunningTerminalCommand>,
    breakpoints: HashMap<String, HashSet<u32>>,
    available_commands: Option<Arc<HashSet<String>>>,
}

impl ScriptExecutionManager {
    pub fn set_available_commands(&mut self, cmds: Option<HashSet<String>>) {
        self.available_commands = cmds.map(Arc::new);
    }

    pub fn handle_run_script(
        &mut self,
        script_id: &str,
        script_store: &ScriptStore,
        webview: &WebViewManager,
    ) {
        // Stop any existing execution
        if let Some(active) = self.active.take() {
            let _ = active.command_tx.send(DebugCommand::Stop);
        }

        let Some(script) = script_store.scripts.get(script_id) else {
            return;
        };

        let source = script.content.clone();
        let sid = script_id.to_string();
        let (event_tx, event_rx) = unbounded();
        let (command_tx, command_rx) = unbounded();
        let avail = self.available_commands.clone().map(|a| (*a).clone());

        let handle = std::thread::spawn(move || {
            grimscript_lang::run_script(&source, event_tx, command_rx, avail);
        });

        self.active = Some(RunningScript {
            script_id: sid.clone(),
            event_rx,
            command_tx,
            _handle: handle,
            is_debug: false,
        });

        webview.send_to_all(&RustToJs::ScriptStarted {
            script_id: sid,
        });
    }

    pub fn handle_debug_start(
        &mut self,
        script_id: &str,
        script_store: &ScriptStore,
        webview: &WebViewManager,
    ) {
        // Stop any existing execution
        if let Some(active) = self.active.take() {
            let _ = active.command_tx.send(DebugCommand::Stop);
        }

        let Some(script) = script_store.scripts.get(script_id) else {
            return;
        };

        let source = script.content.clone();
        let sid = script_id.to_string();
        let breakpoints = self.breakpoints
            .get(script_id)
            .cloned()
            .unwrap_or_default();
        let (event_tx, event_rx) = unbounded();
        let (command_tx, command_rx) = unbounded();
        let avail = self.available_commands.clone().map(|a| (*a).clone());

        let handle = std::thread::spawn(move || {
            grimscript_lang::debug_script(&source, event_tx, command_rx, breakpoints, avail);
        });

        self.active = Some(RunningScript {
            script_id: sid.clone(),
            event_rx,
            command_tx,
            _handle: handle,
            is_debug: true,
        });

        webview.send_to_all(&RustToJs::ScriptStarted {
            script_id: sid,
        });
    }

    pub fn handle_stop_script(&self) {
        if let Some(active) = self.active.as_ref() {
            let _ = active.command_tx.send(DebugCommand::Stop);
        }
    }

    pub fn handle_debug_command(&self, cmd: DebugCommand, webview: &WebViewManager) {
        let Some(active) = self.active.as_ref() else { return };
        if !active.is_debug { return }

        let _ = active.command_tx.send(cmd);
        webview.send_to_all(&RustToJs::DebugResumed {
            script_id: active.script_id.clone(),
        });
    }

    pub fn handle_toggle_breakpoint(&mut self, script_id: &str, line: u32) {
        let bps = self.breakpoints
            .entry(script_id.to_string())
            .or_default();
        if bps.contains(&line) {
            bps.remove(&line);
        } else {
            bps.insert(line);
        }
        let bps_snapshot = bps.clone();
        if let Some(active) = self.active.as_ref() {
            if active.script_id == script_id {
                let _ = active.command_tx.send(
                    DebugCommand::SetBreakpoints(bps_snapshot)
                );
            }
        }
    }

    pub fn handle_console_command(&mut self, source: &str, _webview: &WebViewManager) {
        // Stop any existing terminal command
        self.terminal = None;

        let source = source.to_string();
        let (event_tx, event_rx) = unbounded();
        let (_, command_rx) = unbounded();
        let avail = self.available_commands.clone().map(|a| (*a).clone());

        let handle = std::thread::spawn(move || {
            grimscript_lang::run_script(&source, event_tx, command_rx, avail);
        });

        self.terminal = Some(RunningTerminalCommand {
            event_rx,
            _handle: handle,
        });

        // Don't send ScriptStarted — terminal commands are separate
    }

    /// Poll terminal command events and forward to JS
    pub fn poll_terminal_events(&mut self, webview: &WebViewManager) {
        let Some(terminal) = self.terminal.as_ref() else { return };

        for _ in 0..100 {
            match terminal.event_rx.try_recv() {
                Ok(event) => {
                    match event {
                        ScriptEvent::Output { line, level } => {
                            let level_str = match level {
                                OutputLevel::Info => "info",
                                OutputLevel::Warn => "warn",
                                OutputLevel::Error => "error",
                            };
                            webview.send_to_all(&RustToJs::ConsoleOutput {
                                text: line,
                                level: level_str.to_string(),
                            });
                        }
                        ScriptEvent::Finished { success, error } => {
                            webview.send_to_all(&RustToJs::TerminalFinished {
                                success,
                                error,
                            });
                        }
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }

        if self.terminal.as_ref().is_some_and(|t| t.event_rx.is_empty() && t._handle.is_finished()) {
            self.terminal = None;
        }
    }

    /// Poll script execution events and forward to JS
    pub fn poll_script_events(&mut self, webview: &WebViewManager) {
        let Some(active) = self.active.as_ref() else { return };

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
                            webview.send_to_all(&RustToJs::ConsoleOutput {
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
                            webview.send_to_all(&RustToJs::DebugPaused {
                                script_id: active.script_id.clone(),
                                line,
                                variables: debug_vars,
                                call_stack,
                            });
                        }
                        ScriptEvent::Finished { success, error } => {
                            webview.send_to_all(&RustToJs::ScriptFinished {
                                script_id: active.script_id.clone(),
                                success,
                                error,
                            });
                        }
                    }
                }
                Err(_) => break,
            }
        }

        // Check if finished (channel disconnected means thread ended)
        if self.active.as_ref().is_some_and(|a| a.event_rx.is_empty() && a._handle.is_finished()) {
            self.active = None;
        }
    }
}
