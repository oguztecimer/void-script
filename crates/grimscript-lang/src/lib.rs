pub mod ast;
pub mod builtins;
pub mod debug;
pub mod environment;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod value;

pub use debug::{DebugCommand, OutputLevel, ScriptEvent, StepMode, VariableInfo};
pub use error::GrimScriptError;
pub use interpreter::Interpreter;
pub use value::Value;

/// Run a script to completion (no debug), returns output events.
pub fn run_script(
    source: &str,
    output_tx: crossbeam_channel::Sender<ScriptEvent>,
    command_rx: crossbeam_channel::Receiver<DebugCommand>,
    available_commands: Option<std::collections::HashSet<String>>,
) {
    let tokens = lexer::Lexer::new(source).tokenize();
    match parser::Parser::new(tokens).parse() {
        Ok(program) => {
            let mut interp = Interpreter::new(output_tx.clone(), command_rx, false);
            if let Some(cmds) = available_commands {
                interp.set_available_commands(cmds);
            }
            if let Err(e) = interp.execute(&program) {
                let _ = output_tx.send(ScriptEvent::Output {
                    line: format!("Error (line {}): {}", e.line, e.message),
                    level: OutputLevel::Error,
                });
                let _ = output_tx.send(ScriptEvent::Finished {
                    success: false,
                    error: Some(e.message),
                });
                return;
            }
            let _ = output_tx.send(ScriptEvent::Finished {
                success: true,
                error: None,
            });
        }
        Err(e) => {
            let _ = output_tx.send(ScriptEvent::Output {
                line: format!("Syntax error (line {}): {}", e.line, e.message),
                level: OutputLevel::Error,
            });
            let _ = output_tx.send(ScriptEvent::Finished {
                success: false,
                error: Some(e.message),
            });
        }
    }
}

/// Run a script in debug mode with breakpoints.
pub fn debug_script(
    source: &str,
    output_tx: crossbeam_channel::Sender<ScriptEvent>,
    command_rx: crossbeam_channel::Receiver<DebugCommand>,
    breakpoints: std::collections::HashSet<u32>,
    available_commands: Option<std::collections::HashSet<String>>,
) {
    let tokens = lexer::Lexer::new(source).tokenize();
    match parser::Parser::new(tokens).parse() {
        Ok(program) => {
            let mut interp = Interpreter::new(output_tx.clone(), command_rx, true);
            interp.set_breakpoints(breakpoints);
            if let Some(cmds) = available_commands {
                interp.set_available_commands(cmds);
            }
            if let Err(e) = interp.execute(&program) {
                let _ = output_tx.send(ScriptEvent::Output {
                    line: format!("Error (line {}): {}", e.line, e.message),
                    level: OutputLevel::Error,
                });
                let _ = output_tx.send(ScriptEvent::Finished {
                    success: false,
                    error: Some(e.message),
                });
                return;
            }
            let _ = output_tx.send(ScriptEvent::Finished {
                success: true,
                error: None,
            });
        }
        Err(e) => {
            let _ = output_tx.send(ScriptEvent::Output {
                line: format!("Syntax error (line {}): {}", e.line, e.message),
                level: OutputLevel::Error,
            });
            let _ = output_tx.send(ScriptEvent::Finished {
                success: false,
                error: Some(e.message),
            });
        }
    }
}
