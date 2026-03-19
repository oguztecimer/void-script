# TODO

> **Resolved items** are tracked in `CHANGELOG.md`. Completed: S-01 (parity tests), S-02 (IndexMap), S-03 (print tick — already correct), S-05 (step limit warning), S-06 (hot-reload surface), S-09 (percent/scale), M-01 (deterministic load order), M-02 (collision warnings + reserved fields), M-03 (command def validation), M-04 (spawn validation). Partially resolved: M-05 (Phase 2 design sketch + reserved field), M-06 (command cost system — conditional effects deferred).

## S-04: No Error Recovery in Scripts

**Priority: High**

When a script hits a runtime error (type mismatch, invalid operation, accessing a dead entity), the script halts permanently. The entity stands there doing nothing for the rest of the run. In a game where coding is the core loop, one unhandled edge case can brick a minion. This is especially bad in an idle game where the player might not be watching when it happens.

### Suggested fix (pick one or layer them) WARDING

- **try/except blocks in GrimScript** — let players write defensive code: `try: attack(target) except: wait()`. Requires new AST nodes, compiler support for exception tables or jump-on-error, and executor support for catching errors and jumping to handler blocks.
- **Implicit fallback behavior** — on error, the entity performs `wait()` instead of halting, and an error indicator appears in the editor UI (red badge on the script tab, error log in the output panel). The script resets to `pc = 0` next tick and tries again. This is the more forgiving option and works even for players who don't know about try/except.
- **Both** — implicit fallback as the safety net, try/except for advanced players who want fine-grained control.

---

## S-07: No Debugging Tools Beyond Run

**Priority: Medium**

The interpreter has basic debug infrastructure (breakpoints, step over/into/out via IPC — see `DebugStart`, `DebugContinue`, `StepOver/Into/Out` messages), but debugging support for the **compiler/executor path** (which is the real game execution path) is missing. Variable inspection and call stack visualization are not yet surfaced in the editor UI. For a game where writing and debugging code is the gameplay, rich debugging tools are a core feature, not a nice-to-have. Players will spend significant time figuring out why their scripts behave unexpectedly. Note: S-05 (step limit auto-yield warning) is now resolved — scripts that exceed 10k instructions per tick emit a console warning.

### Suggested fix (incremental)

- **Phase 1:** On error, display the full variable state and call stack in the editor output panel. This is relatively cheap — the interpreter already has this data, it just needs to be serialized and sent via IPC.
- **Phase 2:** Breakpoints. The interpreter checks a breakpoint set (line numbers) before executing each AST node. When hit, it pauses and sends current state to the editor. The editor highlights the line and shows a variable inspector panel. Resume/step buttons send IPC messages back.
- **Phase 3:** Sim-side debugging. The executor emits a trace log of actions and queries per tick, viewable in the editor as a "tick replay" timeline. This bridges the gap between the interpreter (where you debug logic) and the sim (where the logic actually runs).

---

## M-05: Phase 2 Mod API Design Needs Early Planning

**Priority: Medium**

The `.grim` library file system (Phase 2) has architectural implications for namespace strategy, compilation order, and how library functions interact with custom commands. A design sketch has been written in `docs/modding.md` and the `libraries` field reserved in `CommandsDef`, but the actual loading and compilation of `.grim` files is not yet implemented.

### Remaining work

- Parse and load `.grim` files listed in `commands.libraries`
- Compile library functions before player scripts
- Inject library function definitions into player script compilation
- Handle cross-mod library function collisions

---

## M-06: Custom Command Effect System Is Fixed and Non-Composable

**Priority: Medium**

Custom commands now support resource costs (`cost` field in `CommandDef`), but the effect system is still limited to 5 fixed types. More advanced modding needs:

### Remaining work

- **Conditional effects** — effects that only trigger if a condition is met (e.g., "if target health < 50%, deal double damage")
- **Custom queries** — mod-defined queries that return values (not just actions that consume a tick)
- **Composable effects** — reference other commands or chain effects dynamically

---

## C-01: Removed Compiler Scaffolding — Re-add When Needed

**Priority: Low**

Several compiler scaffolding items were removed to eliminate warnings. Re-add them when their features are implemented:

- **`action_is_void()` in `builtins.rs`** — Indicated whether an action is void (all actions are). Useful if non-void actions are added (e.g., queries that also consume a tick).
- **`FuncDef.line` field in `emit.rs`** — Stored the source line of function definitions. Needed for compiler error messages that reference function definition locations.
- **`Scope.local_base` field in `symbol_table.rs`** — Tracked the offset base for local variable numbering in function scopes. Needed for nested function scopes or closure support.
- **`SymbolTable::in_function()` in `symbol_table.rs`** — Checked whether the compiler is currently inside a function scope. Useful for scope-dependent compilation logic (e.g., disallowing certain statements at module level vs function level).

---

## S-08: No Coroutine or Multi-Tick Planning Primitive

**Priority: Low — Deferred**

If a player wants a unit to execute a multi-step plan (move to position X, then harvest, then move to position Y, then raise), they must manage that state manually with variables and conditionals — tracking which step they're on, resetting state between steps, handling interruptions. This is fine for experienced programmers but is a significant complexity wall for the target audience.

### Suggested fix

Consider a yield-style coroutine system where a script can perform an action and then resume where it left off on the next tick. The executor already supports yielding on actions (see `ScriptState` docs in `entity.rs` and `handle_run_script_sim` in `app.rs`) — the difference is that currently `pc` advances past the action instruction, so the script continues from the next instruction on resume. A coroutine model would be a higher-level abstraction, potentially a `plan()` or `sequence()` block in GrimScript that desugars into the existing yield mechanics with auto-generated state tracking. This is a game design decision as much as an engineering one — it makes scripts easier to write but potentially less interesting. Worth prototyping to see how it feels.
