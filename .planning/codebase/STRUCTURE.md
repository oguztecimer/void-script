# Codebase Structure

**Analysis Date:** 2026-03-14

## Directory Layout

```
void-script/
├── crates/                           # Rust workspace crates
│   ├── voidscript-lang/             # Language: lexer, parser, interpreter
│   │   └── src/
│   │       ├── lib.rs               # Public API: run_script, debug_script
│   │       ├── lexer.rs             # Tokenization (indent-aware)
│   │       ├── parser.rs            # Token → AST conversion
│   │       ├── interpreter.rs       # AST execution with debug support
│   │       ├── ast.rs               # Program, Statement, Expression definitions
│   │       ├── value.rs             # Runtime value types
│   │       ├── environment.rs       # Variable scoping
│   │       ├── builtins.rs          # Game API functions
│   │       ├── error.rs             # Error types
│   │       ├── token.rs             # Token enum definitions
│   │       └── debug.rs             # Debug events, stepping, variables
│   │
│   ├── voidscript-editor/           # Desktop editor UI host
│   │   └── src/
│   │       ├── lib.rs               # Module declarations
│   │       ├── plugin.rs            # Bevy EditorPlugin, system registration
│   │       ├── window.rs            # EditorWindow spawning, webview attachment
│   │       ├── ipc.rs               # Message types: RustToJs, JsToRust
│   │       ├── execution.rs         # Script execution threading, event polling
│   │       ├── scripts.rs           # Script file I/O, ScriptStore resource
│   │       ├── tabs.rs              # Tab events (TabChanged, etc.)
│   │       └── embedded_assets.rs   # Asset serving for webview
│   │
│   ├── voidscript-game/             # Game application binary
│   │   └── src/
│   │       └── main.rs              # Bevy app with EditorPlugin
│   │
│   └── voidscript-sim/              # Game simulation (unused in editor)
│       └── src/
│           └── lib.rs
│
├── editor-ui/                        # TypeScript/React frontend
│   ├── src/
│   │   ├── main.tsx                 # React entry point, mounts root
│   │   ├── App.tsx                  # Layout: 3 panels, tool strips, tabs
│   │   ├── state/
│   │   │   └── store.ts             # Zustand store for all UI state
│   │   ├── components/              # React UI components
│   │   │   ├── Editor.tsx           # CodeMirror editor with breakpoints, gutter
│   │   │   ├── Console.tsx          # Script output display
│   │   │   ├── DebugPanel.tsx       # Variables, call stack, step buttons
│   │   │   ├── Header.tsx           # Title bar and window controls
│   │   │   ├── TabBar.tsx           # Open script tabs
│   │   │   ├── ScriptList.tsx       # Left panel: available scripts
│   │   │   ├── ToolStrip.tsx        # Left/right sidebar buttons
│   │   │   └── StatusBar.tsx        # Bottom status information
│   │   ├── ipc/
│   │   │   ├── bridge.ts            # IPC message handler, init
│   │   │   ├── types.ts             # TypeScript IPC message types
│   │   │   └── sender.ts            # sendToRust() helper
│   │   └── codemirror/
│   │       ├── voidscript-lang.ts   # Language grammar for syntax highlighting
│   │       ├── voidscript-completion.ts  # Autocomplete suggestions
│   │       └── voidscript-theme.ts  # Editor theming and colors
│   │
│   ├── package.json                 # npm dependencies: React, CodeMirror, Zustand
│   ├── vite.config.ts               # Build config
│   ├── tsconfig.json                # TypeScript config
│   └── dist/                        # Built output (embedded in Rust binary)
│
├── scripts/                         # User script directory (created at runtime)
│   └── *.vs                         # VOID//SCRIPT files
│
├── .planning/                       # GSD planning documents
│   └── codebase/
│       ├── ARCHITECTURE.md
│       └── STRUCTURE.md
│
├── Cargo.toml                       # Workspace root
├── Cargo.lock                       # Dependency lock
├── target/                          # Rust build output
└── .git/
```

## Directory Purposes

**`crates/voidscript-lang/src/`:**
- Purpose: Language implementation—tokenization, parsing, execution
- Contains: Lexer, Parser, Interpreter, AST definitions, error types, debug support
- Key files:
  - `lib.rs`: Entry point with public `run_script()` and `debug_script()` functions
  - `lexer.rs`: Converts source text → tokens (handles indentation-based syntax)
  - `parser.rs`: Converts tokens → AST (recursive descent parser)
  - `interpreter.rs`: Executes AST, manages environment and call stack
  - `builtins.rs`: Built-in game API functions (e.g., `move()`, `scan()`)

**`crates/voidscript-editor/src/`:**
- Purpose: Rust backend for editor—window management, IPC, execution threading
- Contains: Bevy plugin, webview integration, script storage, execution manager
- Key files:
  - `plugin.rs`: EditorPlugin that registers all systems and events
  - `window.rs`: Window creation and webview attachment via wry
  - `ipc.rs`: IPC message enums (RustToJs, JsToRust) with Serde serialization
  - `execution.rs`: Spawns threads for script execution, polls events, sends to UI
  - `scripts.rs`: ScriptStore resource for loading/managing script files

**`editor-ui/src/`:**
- Purpose: React frontend for code editor UI
- Contains: Components, state management, IPC bridge, CodeMirror integration
- Key files:
  - `App.tsx`: Root layout with 3-panel design (left panel, center editor, right debug)
  - `state/store.ts`: Zustand store holding all UI state (tabs, console, debug, panels)
  - `ipc/bridge.ts`: Initializes IPC handler, declares window.ipc interface

**`editor-ui/src/components/`:**
- Purpose: Modular React components for UI features
- Files:
  - `Editor.tsx`: CodeMirror integration, breakpoint gutter, debug line highlight
  - `TabBar.tsx`: Rendering open script tabs
  - `Console.tsx`: Displaying script output and errors
  - `DebugPanel.tsx`: Variables, call stack, step/continue buttons
  - `ScriptList.tsx`: Left panel showing available scripts
  - `Header.tsx`: Title bar, window controls (minimize, close)
  - `ToolStrip.tsx`: Sidebar buttons for toggling panels
  - `StatusBar.tsx`: Cursor position, script type display

**`editor-ui/src/ipc/`:**
- Purpose: Inter-process communication with Rust backend
- Files:
  - `bridge.ts`: Sets up `window.__IPC_RECEIVE` handler, message dispatching to store
  - `types.ts`: TypeScript types for IPC messages (RustToJsMessage, JsToRustMessage)

**`editor-ui/src/codemirror/`:**
- Purpose: CodeMirror 6 customization for VOID//SCRIPT language
- Files:
  - `voidscript-lang.ts`: Syntax highlighting grammar (Lezer parser)
  - `voidscript-completion.ts`: Autocomplete provider (builtins, keywords)
  - `voidscript-theme.ts`: Editor theme colors (dark mode)

**`scripts/`:**
- Purpose: User script storage directory
- Created at: Runtime by ScriptStore when editor loads
- Contains: `.vs` files (VOID//SCRIPT source)
- Pattern: Scripts auto-loaded on editor startup

## Key File Locations

**Entry Points:**

- `crates/voidscript-game/src/main.rs`: Game binary entry point (loads EditorPlugin)
- `editor-ui/src/main.tsx`: React root, mounts App component
- `editor-ui/src/App.tsx`: React root component, layout structure

**Configuration:**

- `Cargo.toml`: Workspace definition, version, dependencies (Bevy 0.15, serde)
- `crates/voidscript-editor/Cargo.toml`: Editor crate deps (bevy, wry, crossbeam)
- `editor-ui/package.json`: Frontend deps (React, CodeMirror, Zustand, Vite)
- `editor-ui/tsconfig.json`: TypeScript compiler options
- `editor-ui/vite.config.ts`: Vite bundler config

**Core Logic:**

- `crates/voidscript-lang/src/lib.rs`: Public API for script execution
- `crates/voidscript-editor/src/execution.rs`: Thread spawning, event polling loop
- `crates/voidscript-editor/src/plugin.rs`: Bevy system scheduling
- `editor-ui/src/state/store.ts`: Centralized state mutations
- `editor-ui/src/ipc/bridge.ts`: Message routing from Rust to React

**Testing:**

- Not detected (no test files found)

## Naming Conventions

**Files:**

- Rust crate names: kebab-case (`voidscript-editor`, `voidscript-lang`)
- Rust source files: snake_case (`lexer.rs`, `interpreter.rs`, `ipc.rs`)
- React components: PascalCase (`Editor.tsx`, `DebugPanel.tsx`)
- Utility files: camelCase (`bridge.ts`, `types.ts`)
- Config files: lowercase or camelCase (`Cargo.toml`, `vite.config.ts`, `tsconfig.json`)

**Directories:**

- Rust packages: kebab-case (`crates/voidscript-editor`)
- Feature directories: lowercase (`components/`, `ipc/`, `state/`, `codemirror/`)
- Built output: `dist/`, `target/`

**Code Identifiers:**

- Rust types: PascalCase (`EditorWindow`, `ScriptStore`, `RustToJs`)
- Rust functions: snake_case (`run_script`, `handle_run_script`, `toggle_breakpoint`)
- React components: PascalCase (`Editor`, `DebugPanel`, `ScriptList`)
- Zustand store: `useStore` (naming convention for hooks)
- TypeScript types: PascalCase (`Tab`, `ScriptInfo`, `DebugVariable`)
- Variables: camelCase (`activeTabId`, `isDebugging`, `scriptList`)

## Where to Add New Code

**New Feature (e.g., new game API function):**
- Language builtins: `crates/voidscript-lang/src/builtins.rs`
- Interpreter support: Add to `crates/voidscript-lang/src/interpreter.rs` evaluation
- Frontend UI: Add component in `editor-ui/src/components/` if UI needed
- Tests: N/A (not structured)

**New Component/Panel:**
- Implementation: Create file in `editor-ui/src/components/Component.tsx`
- State: Add properties to `EditorState` interface in `editor-ui/src/state/store.ts`
- Integration: Import in `editor-ui/src/App.tsx` and add to layout
- IPC messages: Add message type to `crates/voidscript-editor/src/ipc.rs` if backend coordination needed

**Utilities or Helpers:**
- Shared Rust helpers: Add module to `crates/voidscript-lang/src/` or `crates/voidscript-editor/src/`
- Shared TypeScript helpers: Create file in `editor-ui/src/` or subdirectory (e.g., `utils/`)
- CodeMirror customization: Add to `editor-ui/src/codemirror/`

**IPC Message Types:**
- Rust side: Add variant to `RustToJs` or `JsToRust` enum in `crates/voidscript-editor/src/ipc.rs`
- TypeScript side: Add type to `crates/voidscript-editor/src/ipc/types.ts` (generated or mirrored)
- Handler: Add case in IPC bridge (`editor-ui/src/ipc/bridge.ts`)
- State: Update Zustand store as needed

**Backend System (Bevy):**
- Create function following pattern: `fn handle_event_name(...)` in appropriate file
- Register in `EditorPlugin::build()` under `add_systems(Update, (...))`
- Define event type in same file or `ipc.rs` for IPC events

## Special Directories

**`target/`:**
- Purpose: Rust build artifacts
- Generated: Yes (by `cargo build`)
- Committed: No (in .gitignore)

**`editor-ui/node_modules/`:**
- Purpose: npm dependencies
- Generated: Yes (by `npm install`)
- Committed: No (in .gitignore)

**`editor-ui/dist/`:**
- Purpose: Built frontend bundle
- Generated: Yes (by `npm run build`)
- Committed: No
- Usage: Served via wry custom protocol from Rust

**`scripts/`:**
- Purpose: User-editable script files
- Generated: No (user creates via editor)
- Committed: No (in .gitignore)
- Loaded by: `ScriptStore::load_all()` on editor startup

**`.planning/codebase/`:**
- Purpose: GSD analysis documents
- Generated: Yes (by map-codebase agent)
- Committed: Yes (reference for future work)

---

*Structure analysis: 2026-03-14*
