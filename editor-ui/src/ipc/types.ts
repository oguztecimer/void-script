// Messages from Rust to JS
export type RustToJsMessage =
  | { type: 'script_load'; script_id: string; name: string; content: string; script_type: string }
  | { type: 'error_update'; script_id: string; diagnostics: Diagnostic[] }
  | { type: 'script_list'; scripts: ScriptInfo[] }
  | { type: 'tab_close'; script_id: string }
  | { type: 'console_output'; text: string; level: 'info' | 'warn' | 'error' }
  | { type: 'script_started'; script_id: string }
  | { type: 'script_finished'; script_id: string; success: boolean; error?: string }
  | { type: 'debug_paused'; script_id: string; line: number; variables: VariableInfo[]; call_stack: string[] }
  | { type: 'debug_resumed'; script_id: string };

// Messages from JS to Rust
export type JsToRustMessage =
  | { type: 'editor_ready' }
  | { type: 'script_save'; script_id: string; content: string }
  | { type: 'script_request'; script_id: string }
  | { type: 'script_list_request' }
  | { type: 'tab_changed'; script_id: string }
  | { type: 'run_script'; script_id: string }
  | { type: 'stop_script'; script_id: string }
  | { type: 'debug_start'; script_id: string }
  | { type: 'debug_continue'; script_id: string }
  | { type: 'debug_step_over'; script_id: string }
  | { type: 'debug_step_into'; script_id: string }
  | { type: 'debug_step_out'; script_id: string }
  | { type: 'toggle_breakpoint'; script_id: string; line: number }
  | { type: 'create_script' }
  | { type: 'window_minimize' }
  | { type: 'window_maximize' }
  | { type: 'window_close' };

export interface Diagnostic {
  line: number;
  col_start: number;
  col_end: number;
  severity: 'error' | 'warning' | 'info';
  message: string;
}

export interface ScriptInfo {
  id: string;
  name: string;
  script_type: string;
}

export interface VariableInfo {
  name: string;
  value: string;
  var_type: string;
}
