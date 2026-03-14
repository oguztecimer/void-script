# Architecture

**Analysis Date:** 2026-03-14

## Pattern Overview

**Overall:** Multi-process Desktop Application with IPC Bridge

**Key Characteristics:**
- Rust backend (Bevy game engine as host)
- Embedded WebView for UI layer (Wry)
- TypeScript/React frontend decoupled from Rust via message-passing IPC
- Language interpreter (Lexer → Parser → Interpreter pipeline)
- Thread-based script execution for concurrent running

## Layers

**Backend (Rust):**
- Purpose: Game engine host, script management, execution, IPC coordination
- Location: `crates/voidscript-editor/src/`, `crates/voidscript-lang/src/`
- Contains: Plugin systems, window management, script execution threading, IPC marshaling
- Depends on: Bevy 0.15, wry (WebView), voidscript-lang crate
- Used by: Embedded React frontend via IPC bridge

**Language Layer (Rust):**
- Purpose: Parse and interpret VOID//SCRIPT language
- Location: `crates/voidscript-lang/src/`
- Contains: Lexer, Parser, AST, Interpreter, builtins, debug infrastructure
- Depends on: crossbeam-channel (for event/command channels)
- Used by: Editor execution module, game simulation

**Frontend UI (TypeScript/React):**
- Purpose: Code editor interface with tabs, console, debug panel, script management
- Location: `editor-ui/src/`
- Contains: React components, CodeMirror integration, Zustand state, IPC bridge
- Depends on: React 19, CodeMirror 6, Zustand, Vite build system
- Used by: Browser/WebView rendered by Wry

## Data Flow

**Script Execution Pipeline:**

1. User edits script in CodeMirror editor (`editor-ui/src/components/Editor.tsx`)
2. User clicks "Run" button → sends `RunScriptEvent` via IPC
3. Rust receives `JsToRust::RunScript` in IPC channel (`crates/voidscript-editor/src/ipc.rs`)
4. `handle_run_script` in `execution.rs` spawns thread with `voidscript_lang::run_script()`
5. Lexer tokenizes source (`voidscript-lang/src/lexer.rs`)
6. Parser builds AST (`voidscript-lang/src/parser.rs`)
7. Interpreter executes AST, emits `ScriptEvent`s through crossbeam channel
8. `poll_script_events` system reads events and sends `RustToJs::ConsoleOutput` via IPC
9. JavaScript bridge receives and updates Zustand state (`editor-ui/src/state/store.ts`)
10. React components re-render with new console output

**Debug Mode Pipeline:**

1. User sets breakpoint by clicking in gutter (`editor-ui/src/components/Editor.tsx` breakpoint gutter)
2. UI sends `toggle_breakpoint` message with line number
3. Rust stores in `ScriptExecutionManager::breakpoints`
4. `handle_debug_start` creates interpreter with `debug_mode=true`
5. Interpreter sends `ScriptEvent::Paused` when breakpoint hit
6. `poll_script_events` sends `RustToJs::DebugPaused` with variables and call stack
7. UI updates debug panel (`editor-ui/src/components/DebugPanel.tsx`)
8. User sends debug command (step, continue, etc.)
9. Rust sends `DebugCommand` to interpreter via channel
10. Interpreter responds with new `ScriptEvent::Paused` or `Finished`

**State Management:**

- Zustand store (`editor-ui/src/state/store.ts`) is source of truth for UI state
- Rust sends state updates via IPC messages
- IPC bridge handler updates store, components react
- Breakpoints are bidirectional: UI stores locally, synced to Rust via IPC

## Key Abstractions

**IPC Message Types:**
- Purpose: Decouple Rust backend from React frontend
- Examples: `src/ipc.rs` defines `RustToJs` and `JsToRust` enums
- Pattern: Tagged Serde serialization for JSON interchange
- Implementation: `window.__IPC_RECEIVE` in JavaScript, `webview.evaluate_script()` in Rust

**Script Events:**
- Purpose: Communicate script execution state from interpreter to UI
- Examples: `ScriptEvent::Output`, `ScriptEvent::Paused`, `ScriptEvent::Finished`
- Pattern: crossbeam-channel sender/receiver
- Location: `voidscript-lang/src/debug.rs`

**AST (Abstract Syntax Tree):**
- Purpose: Represent parsed VOID//SCRIPT program structure
- Examples: `StmtKind` for statements, `ExprKind` for expressions
- Pattern: Recursive enum structure with line number tracking
- Location: `crates/voidscript-lang/src/ast.rs`

**Tab Management:**
- Purpose: Track open scripts and active editor state
- Pattern: Redux-like reducer functions in Zustand
- Location: `editor-ui/src/state/store.ts` (Tab interface and state)
- Implementation: Each tab holds script ID, content, diagnostics, modification flag

## Entry Points

**Rust (Editor Plugin):**
- Location: `crates/voidscript-editor/src/plugin.rs`
- Triggers: Loaded as Bevy plugin in game binary
- Responsibilities: Register systems, events, resources; coordinate IPC and execution

**Rust (Window Creation):**
- Location: `crates/voidscript-editor/src/window.rs`
- Function: `create_editor_window()`
- Triggers: `OpenEditorEvent` (from game)
- Responsibilities: Spawn Bevy window, attach webview, initialize wry

**JavaScript (App Root):**
- Location: `editor-ui/src/App.tsx`
- Triggers: React root render
- Responsibilities: Layout 3-panel UI (left scripts, center editor, right debug), register IPC bridge

**JavaScript (IPC Bridge):**
- Location: `editor-ui/src/ipc/bridge.ts`
- Function: `initIpcBridge()`
- Triggers: Called in `App.tsx` useEffect
- Responsibilities: Set up `window.__IPC_RECEIVE` handler, declare `window.ipc`, request initial script list

## Error Handling

**Strategy:** Multi-layer error propagation with user feedback

**Patterns:**

- **Lexer/Parser Errors:** Caught in `voidscript-lang/src/lib.rs` run_script/debug_script
  - Sent as `ScriptEvent::Output { level: OutputLevel::Error }`
  - Result: Error message displayed in console

- **Interpreter Errors:** Caught in interpreter execution loop
  - VoidScriptError includes line number and message
  - Sent as `ScriptEvent::Finished { success: false, error: Some(msg) }`
  - UI displays in console and shows status

- **IPC Errors:** Handled with fallback logging
  - `sendToRust()` in bridge logs if `window.ipc` unavailable (dev fallback)
  - Webview script evaluation uses `let _ =` to ignore failures

- **Execution Errors:** Thread panics caught implicitly
  - Thread spawned in `handle_run_script` with `JoinHandle` stored
  - If thread panics, execution stops and resource cleaned up on next run

## Cross-Cutting Concerns

**Logging:**
- Rust: Uses Bevy's `info!()` macro (debug output)
- JavaScript: Uses `console.log()` with `[IPC Mock]` prefixes in dev mode

**Validation:**
- **Script syntax:** Handled by Lexer/Parser before execution
- **Breakpoints:** Validated by line number existence during debug
- **Messages:** Serde handles validation on IPC deserialization

**Authentication:**
- Not implemented (single-user desktop app)
- IPC messages unprotected (local only, no network exposure)

**Script Lifecycle:**
- Load: User selects script from left panel
- Edit: Live in editor tab
- Save: Sent to Rust via `script_save` message
- Run: Spawned in thread with own environment
- Debug: Paused at breakpoints with variable inspection

---

*Architecture analysis: 2026-03-14*
