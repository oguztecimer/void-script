# Testing Patterns

**Analysis Date:** 2026-03-14

## Test Framework

**Runner:**
- No test runner currently configured or in use
- TypeScript: Vite as build tool (not test runner); no Vitest, Jest, or similar configured in `package.json`
- Rust: Cargo test framework available (not explicitly configured; no test crates found)

**Assertion Library:**
- Not applicable - no tests currently exist

**Run Commands:**
```bash
# TypeScript/React
npm run build    # Compiles TypeScript with `tsc && vite build`
npm run dev      # Runs Vite dev server

# Rust
cargo test       # Runs all tests (none currently present)
cargo build      # Compiles the workspace
```

## Test File Organization

**Current Status:**
- No test files found in the repository
- No `*.test.ts`, `*.spec.ts`, `*.test.tsx`, or `*.spec.tsx` files detected
- No Rust `#[test]` modules or `#[cfg(test)]` attributes in any `.rs` files

**Recommended Structure (for future tests):**

**TypeScript/React:**
- Co-located pattern: Tests next to source files
- Naming: `ComponentName.test.tsx` or `componentName.test.ts`
- Example structure:
  ```
  editor-ui/src/
  ├── components/
  │   ├── Editor.tsx
  │   ├── Editor.test.tsx
  │   ├── Header.tsx
  │   └── Header.test.tsx
  ├── state/
  │   ├── store.ts
  │   └── store.test.ts
  └── ipc/
      ├── bridge.ts
      └── bridge.test.ts
  ```

**Rust:**
- Module test submodules: `#[cfg(test)] mod tests { ... }`
- Located at bottom of each `.rs` file
- Example structure:
  ```rust
  // src/interpreter.rs
  pub struct Interpreter { ... }
  impl Interpreter { ... }

  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_execute_simple_program() { ... }
  }
  ```

## Test Structure

**TypeScript (Not implemented - recommended pattern):**

For React components, recommended structure using testing-library:
```typescript
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Editor } from './Editor';

describe('Editor Component', () => {
  it('should render editor container when active tab exists', () => {
    // Setup
    // Act
    // Assert
  });

  it('should show placeholder text when no tab is active', () => {
    // Setup
    // Act
    // Assert
  });
});
```

For Zustand store, recommended pattern:
```typescript
import { useStore } from './store';

describe('Editor Store', () => {
  beforeEach(() => {
    useStore.setState(initialState);
  });

  it('should add console output', () => {
    // Setup
    useStore.getState().addConsoleOutput('test', 'info');
    // Assert
    expect(useStore.getState().consoleOutput).toHaveLength(1);
  });
});
```

**Rust (Not implemented - recommended pattern):**

For interpreter execution:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_assignment() {
        let mut interpreter = Interpreter::new(/* ... */);
        let program = Program {
            statements: vec![
                Statement {
                    kind: StmtKind::Assign {
                        target: AssignTarget::Name("x".to_string()),
                        value: Expr { kind: ExprKind::Integer(42), line: 1 },
                    },
                    line: 1,
                }
            ],
        };
        assert!(interpreter.execute(&program).is_ok());
    }
}
```

## Mocking

**Framework:** Not configured

**Recommended approach for future tests:**

**TypeScript:**
- Mock Rust IPC: Mock `window.ipc` or `window.__IPC_RECEIVE` for bridge testing
- Mock Zustand store: Use `useStore.setState()` to set test state
- Mock CodeMirror: Mock `EditorView` and related APIs if testing Editor component
- Example mock pattern:
  ```typescript
  const mockSendToRust = jest.fn();
  jest.mock('./bridge', () => ({
    sendToRust: mockSendToRust,
  }));
  ```

**Rust:**
- Mock channels: Create test `Sender`/`Receiver` for output and commands
- Mock Environment: Create fresh `Environment::new()` for each test
- Example:
  ```rust
  #[test]
  fn test_interpreter_initialization() {
      let (tx, _) = crossbeam_channel::unbounded();
      let (_, rx) = crossbeam_channel::unbounded();
      let interp = Interpreter::new(tx, rx, false);
      assert_eq!(interp.step_count, 0);
  }
  ```

**What to Mock:**
- External IPC communication (window.ipc API)
- Channels (crossbeam_channel for Rust interpreter)
- User input (for UI component tests)

**What NOT to Mock:**
- Business logic (interpreter execution, lexer tokenization)
- Data structures (Program, AST, Value types)
- Core language features

## Fixtures and Factories

**Current Status:** None present in codebase

**Recommended Fixtures (TypeScript):**

Create `editor-ui/src/__fixtures__/mockStore.ts`:
```typescript
import type { Tab } from '../state/store';

export const mockTab: Tab = {
  scriptId: 'script-1',
  name: 'test.vs',
  content: 'x = 42',
  scriptType: 'script',
  isModified: false,
  diagnostics: [],
};

export const mockDiagnostic = {
  line: 1,
  col_start: 0,
  col_end: 1,
  severity: 'error' as const,
  message: 'Syntax error',
};
```

**Recommended Factories (Rust):**

Create helper in test modules:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_interpreter() -> Interpreter {
        let (tx, _) = crossbeam_channel::unbounded();
        let (_, rx) = crossbeam_channel::unbounded();
        Interpreter::new(tx, rx, false)
    }

    fn create_simple_program(code: &str) -> Program {
        // Parse code into Program
        let lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        // ... parse tokens into Program
    }
}
```

**Location:**
- TypeScript: `editor-ui/src/__fixtures__/` directory
- Rust: At bottom of each test module within `#[cfg(test)]` block

## Coverage

**Requirements:** None enforced

**Recommended Target:** Minimum 80% for critical paths (interpreter, parser, lexer)

**View Coverage (future):**
```bash
# TypeScript with coverage
npm test -- --coverage

# Rust with coverage (requires tarpaulin)
cargo tarpaulin --out Html --output-dir coverage
```

## Test Types

**Unit Tests (Not implemented - recommended):**

**Scope:** Individual functions and methods
- Interpreter `execute()` method with simple programs
- Lexer tokenization of various input
- Parser statement/expression parsing
- Zustand store action side effects
- React component rendering with specific props

**Integration Tests (Not implemented - recommended):**

**Scope:** Multi-component interactions
- Full script execution from code string to output
- Editor component with IPC bridge
- Store state changes across multiple actions
- CodeMirror breakpoint interaction with store

**E2E Tests:** Not applicable for this codebase (would be handled separately in Bevy game)

## Common Patterns

**Async Testing:**

TypeScript (with CodeMirror/IPC):
```typescript
it('should send script to rust after debounce', async () => {
  jest.useFakeTimers();

  // Trigger editor update
  // Fast-forward debounce timer
  jest.runAllTimers();

  expect(mockSendToRust).toHaveBeenCalledWith({
    type: 'script_save',
    script_id: 'script-1',
    content: '...',
  });

  jest.useRealTimers();
});
```

Rust (with channels):
```rust
#[test]
fn test_debug_output_sent_on_pause() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let (_, cmd_rx) = crossbeam_channel::unbounded();
    let mut interpreter = Interpreter::new(tx, cmd_rx, true);

    interpreter.execute(&program).unwrap();

    let event = rx.recv().unwrap();
    assert!(matches!(event, ScriptEvent::DebugPaused { .. }));
}
```

**Error Testing:**

TypeScript:
```typescript
it('should handle invalid message type gracefully', () => {
  expect(() => {
    bridge.handleMessage({ type: 'unknown' });
  }).not.toThrow();
});
```

Rust:
```rust
#[test]
fn test_syntax_error_returns_correct_kind() {
    let err = VoidScriptError::syntax(5, "expected 'def'");
    assert_eq!(err.line, 5);
    assert!(matches!(err.kind, ErrorKind::SyntaxError));
    assert_eq!(err.message, "expected 'def'");
}

#[test]
fn test_undefined_variable_error() {
    let mut interpreter = create_test_interpreter();
    let program = Program { /* references undefined 'x' */ };
    let result = interpreter.execute(&program);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err().kind, ErrorKind::NameError));
}
```

## Critical Code Paths to Test

**Priority: High**

1. **Interpreter execution** (`crate/voidscript-lang/src/interpreter.rs`)
   - Simple assignment and retrieval
   - Function definitions and calls
   - Control flow (if/elif/else, while, for)
   - Breakpoint handling and debug state
   - Step limiting and stop conditions

2. **Parser** (`crate/voidscript-lang/src/parser.rs`)
   - Valid statement parsing
   - Expression precedence
   - Indentation-based blocks
   - Error recovery and reporting

3. **Lexer** (`crate/voidscript-lang/src/lexer.rs`)
   - Token recognition
   - Indent/dedent handling
   - String and number parsing
   - Comment handling

4. **Store state management** (`editor-ui/src/state/store.ts`)
   - Tab open/close/switch
   - Console output accumulation
   - Debug state transitions
   - Breakpoint toggle

**Priority: Medium**

5. **IPC Bridge** (`editor-ui/src/ipc/bridge.ts`)
   - Message routing to store
   - State consistency after messages
   - Error message handling

6. **Editor Component** (`editor-ui/src/components/Editor.tsx`)
   - Content update debouncing
   - Breakpoint gutter interaction
   - Debug line highlighting
   - Diagnostic display

**Priority: Low (UI-heavy)**

7. **UI Components** (Header, Console, TabBar, etc.)
   - Rendering with store state
   - Button click handlers
   - Panel toggle state

---

*Testing analysis: 2026-03-14*
