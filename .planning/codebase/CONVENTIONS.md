# Coding Conventions

**Analysis Date:** 2026-03-14

## Naming Patterns

**Files:**
- TypeScript/React components: PascalCase (e.g., `Editor.tsx`, `Header.tsx`, `DebugPanel.tsx`)
- TypeScript utility/module files: camelCase (e.g., `voidscript-theme.ts`, `voidscript-completion.ts`)
- Rust modules: snake_case (e.g., `ipc.rs`, `environment.rs`, `lexer.rs`)
- Rust crates: snake_case with hyphens (e.g., `voidscript-lang`, `voidscript-editor`)

**Functions:**
- JavaScript/TypeScript: camelCase (e.g., `initIpcBridge()`, `sendToRust()`, `handleUpdate()`)
- Rust: snake_case (e.g., `push_scope()`, `all_variables()`, `type_name()`)
- React components: PascalCase (e.g., `Editor()`, `Header()`, `Console()`)

**Variables:**
- TypeScript: camelCase for state variables and let bindings (e.g., `activeTabId`, `consoleOutput`, `debugVariables`)
- TypeScript state: camelCase for Zustand store properties (e.g., `isDebugging`, `bottomPanelOpen`)
- Rust: snake_case (e.g., `output_tx`, `step_mode`, `call_stack`)

**Types:**
- TypeScript interfaces: PascalCase (e.g., `EditorState`, `Tab`, `ConsoleEntry`)
- TypeScript type aliases: PascalCase (e.g., `RustToJsMessage`, `JsToRustMessage`)
- Rust structs: PascalCase (e.g., `Interpreter`, `Lexer`, `VoidScriptError`)
- Rust enums: PascalCase (e.g., `ControlFlow`, `ErrorKind`, `StmtKind`)
- Rust enum variants: PascalCase (e.g., `SyntaxError`, `RuntimeError`, `FunctionDef`)

## Code Style

**Formatting:**
- TypeScript: 2-space indentation (implicit in ESNext config)
- Rust: 4-space indentation (Rust standard)
- No explicit linter/formatter configured in package.json (vite only)
- TypeScript strict mode enabled in `tsconfig.json`

**Linting:**
- TypeScript: `strict: true` in tsconfig.json with `noFallthroughCasesInSwitch` enforced
- No ESLint configuration present
- Rust: Standard `cargo fmt` conventions (implied by workspace edition 2024)

**Line Length:**
- TypeScript components favor inline styles; no hard line limit observed
- Single-line style objects common: `style={{ display: 'flex', ... }}`
- Long function parameters broken across multiple lines when necessary

## Import Organization

**Order:**
1. External libraries/framework imports (React, Zustand, CodeMirror)
2. Type imports (e.g., `import type { ... }`)
3. Local relative imports (components, utils, state)
4. Path aliases (if any) - not used in this codebase

**Path Aliases:**
- Not configured in this codebase
- Imports use relative paths (e.g., `'../state/store'`, `'../ipc/bridge'`)

**Examples:**

TypeScript imports in `Editor.tsx`:
```typescript
import { useEffect, useRef, useCallback } from 'react';
import { EditorView, keymap, lineNumbers, ... } from '@codemirror/view';
import { EditorState, StateField, StateEffect, ... } from '@codemirror/state';
import { voidScriptLanguage } from '../codemirror/voidscript-lang';
import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';
```

Rust imports in `interpreter.rs`:
```rust
use std::collections::{HashMap, HashSet};
use crossbeam_channel::{Receiver, Sender};
use crate::ast::*;
use crate::builtins;
use crate::debug::*;
```

## Error Handling

**Patterns:**

Rust uses custom error types:
- Custom struct: `VoidScriptError` with `ErrorKind` enum in `crate/voidscript-lang/src/error.rs`
- Constructor methods for different error types: `VoidScriptError::syntax()`, `VoidScriptError::runtime()`, `VoidScriptError::type_error()`, etc.
- Example: `VoidScriptError::name_error(line_no, "variable not found")`
- All errors include line number for debugging

TypeScript uses discriminated unions for IPC messages:
- Tagged union types for messages: `RustToJsMessage`, `JsToRustMessage` with `type` field
- Switch statements for handling different message types in `bridge.ts`
- Example in `initIpcBridge()`: exhaustive switch with cases for each message type

Console error output:
- TypeScript: `console.log()` for IPC mocking in development mode
- Rust: `eprintln!()` for stderr output (e.g., IPC parse errors in `window.rs`)

## Logging

**Framework:** No centralized logging library; console logging and stderr

**Patterns:**

TypeScript (UI):
- Uses `console.log()` for development/debugging
- IPC communication logged to console in mock mode: `console.log('[IPC Mock] sendToRust:', msg)`
- No permanent log statements in production code paths

Rust:
- Uses `eprintln!()` for error output to stderr
- Found in `crate/voidscript-editor/src/window.rs`: `eprintln!("IPC parse error: {e}")`
- Build script logging: `println!("cargo:rerun-if-changed=...")` in build.rs

Output collection:
- Scripts emit output via IPC to `ConsoleEntry` which is collected in Zustand store
- `useStore.getState().addConsoleOutput(text, level)` with levels: 'info', 'warn', 'error'

## Comments

**When to Comment:**
- Documentation comments on public types and functions in Rust: triple-slash `///`
- Example in `environment.rs`: `/// Update an existing variable in the scope where it was defined.`
- Inline comments for complex logic (breakpoint gutter logic in `Editor.tsx`)
- Comments used to mark sections: `// --- Breakpoint gutter ---`, `// --- Debug line highlighting ---`

**JSDoc/TSDoc:**
- Not observed in codebase; interfaces have inline type documentation only
- Example from `ipc/types.ts`: plain comments above enum variants
- Rust documentation strings in `environment.rs` and `value.rs`

## Function Design

**Size:**
- Prefer small, focused functions
- Component functions in React are 40-230 lines (Header is 378 lines as a larger example)
- Utility functions typically 10-50 lines
- Interpreter execution methods can be longer (100+ lines) due to match expressions

**Parameters:**
- React components accept destructured props with inline type annotations
- Example: `function Header() { ... }` uses `useStore` selectors instead of props
- Zustand actions use arrow functions with destructured state: `(scriptId, name, content) => ...`
- Rust functions use explicit parameter types: `pub fn execute(&mut self, program: &Program) -> Result<(), VoidScriptError>`

**Return Values:**
- TypeScript components return JSX elements
- Zustand store actions return void (mutation via `set()`)
- Rust functions return `Result<T, VoidScriptError>` for fallible operations
- Example: `pub fn execute(&mut self, program: &Program) -> Result<(), VoidScriptError>`

## Module Design

**Exports:**
- TypeScript: Named exports for components and utilities
- Example: `export function Editor() { ... }` in `components/Editor.tsx`
- Example: `export const useStore = create<EditorState>(...)` in `state/store.ts`
- Rust: Explicit pub visibility on types and functions
- Example: `pub struct Interpreter { ... }` in `interpreter.rs`

**Barrel Files:**
- Not used in this codebase
- Imports are specific: `import { Editor } from './components/Editor'` not from `'./components'`

**Module organization in Rust:**
- `crate/voidscript-editor/src/lib.rs` declares submodules: `pub mod plugin; pub mod window; pub mod ipc; pub mod tabs; pub mod scripts; pub mod execution;`
- Each module is a separate file in the same directory
- Workspace pattern with multiple crates sharing dependencies

## Inline Styles in React

TypeScript/React conventions heavily favor inline style objects:
- All styling is inline via `style` prop
- Colors are hardcoded hex values (e.g., `#1E1F22`, `#DFE1E5`, `#393B40`)
- Flexbox used for layout
- Hover effects applied via `onMouseEnter` and `onMouseLeave` event handlers
- No CSS files or Tailwind used

Example pattern from `Header.tsx`:
```typescript
<button
  style={{
    display: 'flex',
    alignItems: 'center',
    width: '28px',
    height: '28px',
    background: 'none',
    border: 'none',
    borderRadius: '6px',
    color: disabled ? '#5A5D63' : iconColor,
    cursor: disabled ? 'default' : 'pointer',
    padding: 0,
    opacity: disabled ? 0.5 : 1,
  }}
  onMouseEnter={(e) => { if (!disabled) e.currentTarget.style.backgroundColor = hoverBg; }}
  onMouseLeave={(e) => { if (!disabled) e.currentTarget.style.backgroundColor = disabled ? 'transparent' : bgColor; }}
>
```

---

*Convention analysis: 2026-03-14*
