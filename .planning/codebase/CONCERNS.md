# Codebase Concerns

**Analysis Date:** 2026-03-14

## Tech Debt

**Invalid Rust Edition:**
- Issue: Workspace configured with `edition = "2024"` which is not a valid Rust edition (valid editions are 2015, 2018, 2021)
- Files: `/Users/dakmor/Projects/Other/void-script/Cargo.toml`
- Impact: Crate may fail to compile. This is a blocking build issue.
- Fix approach: Change `edition = "2024"` to `edition = "2021"` in the workspace configuration

**Unsafe Unwrap Calls in Core Paths:**
- Issue: Multiple `.unwrap()` and `.expect()` calls in critical code paths without fallback error handling
- Files:
  - `crates/voidscript-editor/src/window.rs:22` - IPC message serialization with `.expect("serialize IPC")`
  - `crates/voidscript-editor/src/window.rs:78,82` - HTTP response building with `.unwrap()`
  - `crates/voidscript-lang/src/interpreter.rs:749,773,807` - List operations with `.unwrap()`
  - `crates/voidscript-lang/src/lexer.rs:34` - Indent stack access with `.unwrap()`
  - `crates/voidscript-lang/src/parser.rs` - Multiple `.expect()` calls in parsing (lines 80, 141, 160-228)
- Impact: Panics on malformed input, malformed IPC messages, or edge cases in list operations. Can crash the editor and interpreter.
- Fix approach: Replace with proper `Result` types or handle edge cases before unwrapping. For list operations, check bounds before `.pop()`. For HTTP responses, handle build failures gracefully.

**Unbounded Script Execution:**
- Issue: Interpreter has a 100,000-step limit (`max_steps: 100_000` at line 83), which may be too generous or too restrictive depending on script complexity
- Files: `crates/voidscript-lang/src/interpreter.rs:83`
- Impact: Long-running scripts could timeout unexpectedly or, conversely, inefficient infinite loops aren't caught quickly enough. Step counting happens per check, but very tight loops may thrash.
- Fix approach: Make step limit configurable per script execution. Add telemetry to understand typical step counts for game scripts. Consider adaptive limits based on script type.

## Known Bugs

**Parser Unreachable Code Path:**
- Symptoms: Parser has `unreachable!()` assertion that claims to be unreachable but may actually execute if parser logic has bugs
- Files: `crates/voidscript-lang/src/parser.rs:498`
- Trigger: Malformed expressions with infix operators in certain combinations
- Workaround: None - will panic if triggered
- Recommendation: Replace with proper error handling that returns a `VoidScriptError` instead of panicking

**Potential Indent Stack Underflow:**
- Symptoms: Lexer panics when accessing indent stack without checking if it's empty (though logic suggests it shouldn't be)
- Files: `crates/voidscript-lang/src/lexer.rs:34`
- Trigger: Extremely malformed indentation in input (dedent before any indent)
- Workaround: Avoid manual indentation; use an IDE with automatic indent management
- Recommendation: Guard with `.last().ok_or()` and return proper error

## Security Considerations

**IPC Message Serialization Without Validation:**
- Risk: JSON serialization errors in IPC could leak or corrupt message state without proper bounds checking
- Files: `crates/voidscript-editor/src/window.rs:22-26`, `crates/voidscript-editor/src/ipc.rs`
- Current mitigation: Basic try_recv loop with error logging (line 92 of window.rs); Bevy event system guards against some message types
- Recommendations:
  - Add message size limits before serialization
  - Validate all incoming JSON from JS before deserializing (already done at line 88-92 of window.rs)
  - Add rate limiting on IPC message processing

**Script Content Validation Missing:**
- Risk: Script content is loaded from disk and executed directly without AST validation or sandbox containment
- Files: `crates/voidscript-editor/src/scripts.rs:56`, `crates/voidscript-lang/src/interpreter.rs`
- Current mitigation: None - scripts execute with full interpreter privileges
- Recommendations:
  - Validate script syntax before storing or execution
  - Consider a capabilities system for game scripts (e.g., which entities can be accessed)
  - Implement resource quotas (time, memory) per script

**File System Access Without Permission Checks:**
- Risk: Script store reads/writes `.vs` files without validating paths
- Files: `crates/voidscript-editor/src/scripts.rs:52-76` (read), lines 92-93 (write)
- Current mitigation: Path construction uses `.join()` which prevents directory traversal somewhat
- Recommendations:
  - Canonicalize script directory path and verify all loaded paths are within it
  - Use explicit allowlists for script names
  - Add audit logging for script modifications

## Performance Bottlenecks

**Excessive Cloning in Interpreter:**
- Problem: Values are cloned repeatedly during list/dict operations and method calls
- Files: `crates/voidscript-lang/src/interpreter.rs` (52 clone() calls across file), `crates/voidscript-lang/src/value.rs`
- Cause: Rust requires owned values for many operations; no use of references where possible
- Improvement path:
  - Use `Cow<Value>` for read-only operations
  - Implement value interning for strings/small values
  - Profile hot paths with large collections (lists of 1000+ items)

**String Formatting in Display Loop:**
- Problem: `Value::display()` and `Value::repr()` allocate new strings for every value in lists/dicts (lines 59-60, 63-64)
- Files: `crates/voidscript-lang/src/value.rs:50-80`
- Cause: Nested loops with `.collect()` and `.join()` for complex values
- Improvement path:
  - Implement `Display` trait properly to avoid intermediate allocations
  - Cache repr() for frequently displayed values
  - Benchmark with large nested structures

**IPC Broadcasting Without Backpressure:**
- Problem: `send_to_all()` broadcasts to all webviews even if one is slow/unresponsive
- Files: `crates/voidscript-editor/src/window.rs:21-27`
- Cause: Ignores `.evaluate_script()` result; no retry or backoff
- Improvement path:
  - Add return value checking to `send_to_all()`
  - Implement exponential backoff for failed sends
  - Consider per-window message queues

## Fragile Areas

**Lexer Indent Handling:**
- Files: `crates/voidscript-lang/src/lexer.rs:16-87`
- Why fragile: Relies on mutable stack tracking with no validation at key points. Edge cases: mixed tabs/spaces, dedent without matching indent, EOF during indent
- Safe modification: Test against malformed indentation patterns before changing. Add comprehensive error tests.
- Test coverage: None detected (no test files found in codebase)

**Parser Token Consumption:**
- Files: `crates/voidscript-lang/src/parser.rs`
- Why fragile: Position tracking via `self.pos` with no bounds assertions except on current(). Advance() can go past EOF.
- Safe modification: Add invariants (e.g., `self.pos <= self.tokens.len()`). Create test cases for EOF handling.
- Test coverage: None detected

**Script Execution Thread Management:**
- Files: `crates/voidscript-editor/src/execution.rs:49-59`
- Why fragile: JoinHandle stored but never explicitly joined; relies on Drop impl. Thread panics would kill execution thread but UI might not notice.
- Safe modification: Wrap in explicit join with timeout. Add panic hook to channel a "script crashed" event.
- Test coverage: None detected

**IPC Type Enum Conversion:**
- Files: `crates/voidscript-editor/src/ipc.rs:220-271`
- Why fragile: Large match statement in `poll_ipc_messages()` must be kept in sync with `JsToRust` enum. Adding new message types is error-prone.
- Safe modification: Use derive macros or macro-generated dispatch where possible. Test that all variants are handled.
- Test coverage: None detected

## Scaling Limits

**Single Active Script Execution:**
- Current capacity: Only 1 script can execute at a time (stored in `Option<RunningScript>`)
- Limit: Cannot run parallel scripts or concurrent tasks; blocks UI on long computations
- Files: `crates/voidscript-editor/src/execution.rs:23`
- Scaling path:
  - Replace `Option<RunningScript>` with `HashMap<String, RunningScript>` to allow multiple concurrent executions
  - Use a bounded queue or spawning limit to prevent runaway thread creation
  - Add per-script resource limits

**Interpreter Step Limit Prevents Complex Algorithms:**
- Current capacity: 100,000 steps
- Limit: Complex pathfinding, sorting on large lists, or recursive algorithms may hit limit
- Files: `crates/voidscript-lang/src/interpreter.rs:83`
- Scaling path:
  - Profile real game scripts to determine realistic step budgets
  - Make limit configurable per script execution context
  - Consider dynamic reallocation based on script type

**Memory Growth in Environment Scopes:**
- Current capacity: Unbounded scope stack in environment
- Limit: Deep function call chains or tight loops that spawn scopes could cause memory pressure
- Files: `crates/voidscript-lang/src/environment.rs:6-67`
- Scaling path:
  - Add maximum depth limit for scope stack
  - Profile typical scope depth in game scripts
  - Consider scope pooling or reuse

## Dependencies at Risk

**Bevy 0.15 - Pinned Major Version:**
- Risk: Bevy is a young, rapidly-evolving framework; 0.15 may be outdated or have unpatched issues
- Impact: Security vulnerabilities, performance regressions, or API incompatibilities with plugins
- Files: `Cargo.toml` (workspace dependency)
- Migration plan:
  - Track Bevy releases; plan to update annually or on major version bumps
  - Test thoroughly after upgrades (no test suite currently)
  - Consider vendoring critical dependencies if updates become risky

**wry 0.50 - WebView Framework:**
- Risk: Early 0.x version; may have platform-specific bugs or security issues
- Impact: Editor UI could crash, hang, or be exploited via JavaScript injection
- Files: `crates/voidscript-editor/Cargo.toml:10`
- Migration plan:
  - Monitor wry releases for security patches
  - Add validation/sanitization for any user content injected into WebView
  - Have fallback plan (e.g., native UI layer) if wry becomes unmaintained

**crossbeam-channel 0.5 - Channel Implementation:**
- Risk: IPC and execution rely on unbounded channels; if sender/receiver are held across async boundaries, could deadlock
- Impact: Editor could hang on IPC message processing
- Files: Used throughout (`voidscript-lang/Cargo.toml`, `voidscript-editor/Cargo.toml`)
- Migration plan:
  - Ensure channel lifetimes are bounded to single thread
  - Add timeouts to all `try_recv()` calls
  - Consider tokio-based channels for async IPC in future

## Missing Critical Features

**No Test Suite:**
- Problem: Zero automated tests detected in codebase
- Blocks: Cannot safely refactor, cannot catch regressions, cannot validate correctness
- Files: No `*.test.rs` or `*.spec.rs` files found
- Recommendation: Start with interpreter unit tests (lexer, parser, basic execution), then expand to integration tests (IPC, script execution)

**No Error Recovery in Lexer/Parser:**
- Problem: Errors halt parsing immediately; no partial recovery or multi-error reporting
- Blocks: Editor cannot provide useful diagnostics for multiple syntax errors in one file
- Files: `crates/voidscript-lang/src/lexer.rs`, `crates/voidscript-lang/src/parser.rs`
- Recommendation: Implement error recovery at statement boundaries; collect errors and return all to UI

**No Script Debugging Info Storage:**
- Problem: Debug info (line mappings, variable scopes) not persisted
- Blocks: Cannot show source-to-bytecode correlation in debugger; stack traces are basic
- Files: `crates/voidscript-lang/src/debug.rs` (stub), interpreter
- Recommendation: Generate and serialize debug symbols during parsing; attach to scripts

**No Persistence for Editor State:**
- Problem: Open tabs, breakpoints, scroll position not saved between sessions
- Blocks: Users lose work context on restart
- Files: UI state in `editor-ui/src/App.tsx`
- Recommendation: Store editor state in localStorage or file system

## Test Coverage Gaps

**Lexer Not Tested:**
- What's not tested: Indentation handling, edge cases (empty file, only comments, mixed tabs/spaces), malformed tokens
- Files: `crates/voidscript-lang/src/lexer.rs`
- Risk: Indent-related panics or infinite loops could be hiding
- Priority: High (lexer is first line of defense)

**Parser Not Tested:**
- What's not tested: Error recovery, all statement types, expression precedence, EOF handling
- Files: `crates/voidscript-lang/src/parser.rs`
- Risk: Unreachable code paths, subtle bugs in precedence, crashes on malformed input
- Priority: High

**Interpreter Not Tested:**
- What's not tested: List/dict mutations, function scoping, loop control flow, error propagation
- Files: `crates/voidscript-lang/src/interpreter.rs`
- Risk: Unwrap panics, incorrect variable shadowing, off-by-one in list operations
- Priority: High

**IPC Round-Trip Not Tested:**
- What's not tested: Message serialization/deserialization, webview communication, event ordering
- Files: `crates/voidscript-editor/src/window.rs`, `crates/voidscript-editor/src/ipc.rs`
- Risk: Race conditions, message loss, UI hangs
- Priority: Medium

**Script Execution Not Tested:**
- What's not tested: Thread spawning, cleanup on error, breakpoint handling, step limits
- Files: `crates/voidscript-editor/src/execution.rs`
- Risk: Thread leaks, dangling state, debugger hang
- Priority: Medium

---

*Concerns audit: 2026-03-14*
