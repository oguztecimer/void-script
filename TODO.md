# TODO

> **Resolved items** are tracked in `CHANGELOG.md`. Completed: S-01 (parity tests), S-02 (IndexMap), S-03 (print tick — already correct), S-04 (error recovery — implicit fallback), S-05 (step limit warning), S-06 (hot-reload surface), S-09 (percent/scale), M-01 (deterministic load order), M-02 (collision warnings + reserved fields), M-03 (command def validation), M-04 (spawn validation), M-05 (mod dependencies + library system), M-06 (conditional effects — `if` effect type with conditions). Modding extended conditions (Phase 2 of modding roadmap) and scoped targets are complete. S-07 Phase 1 (variable dump on error) complete. **MOD-02 (Lua Scripting Layer)** fully implemented — see CHANGELOG M-01 through M-10.

## S-07: No Debugging Tools Beyond Run (Phase 1 Complete)

**Priority: Medium**

The interpreter has basic debug infrastructure (breakpoints, step over/into/out via IPC — see `DebugStart`, `DebugContinue`, `StepOver/Into/Out` messages), but debugging support for the **compiler/executor path** (which is the real game execution path) is missing. Variable inspection and call stack visualization are not yet surfaced in the editor UI. For a game where writing and debugging code is the gameplay, rich debugging tools are a core feature, not a nice-to-have. Players will spend significant time figuring out why their scripts behave unexpectedly. Note: S-05 (step limit auto-yield warning) is now resolved — scripts that exceed 10k instructions per tick emit a console warning.

### Suggested fix (incremental)

- **Phase 1:** ~~On error, display the full variable state and call stack in the editor output panel.~~ **Resolved.** `SimEvent::ScriptError` now includes variable/stack snapshots. `ScriptErrorDetail` IPC message forwards to editor UI.
- **Phase 2:** Breakpoints. The interpreter checks a breakpoint set (line numbers) before executing each AST node. When hit, it pauses and sends current state to the editor. The editor highlights the line and shows a variable inspector panel. Resume/step buttons send IPC messages back.
- **Phase 3:** Sim-side debugging. The executor emits a trace log of actions and queries per tick, viewable in the editor as a "tick replay" timeline. This bridges the gap between the interpreter (where you debug logic) and the sim (where the logic actually runs).

---

## S-08: No Coroutine or Multi-Tick Planning Primitive

**Priority: Low — Deferred**

If a player wants a unit to execute a multi-step plan (move to position X, then harvest, then move to position Y, then raise), they must manage that state manually with variables and conditionals — tracking which step they're on, resetting state between steps, handling interruptions. This is fine for experienced programmers but is a significant complexity wall for the target audience.

### Suggested fix

Consider a yield-style coroutine system where a script can perform an action and then resume where it left off on the next tick. The executor already supports yielding on actions (see `ScriptState` docs in `entity.rs` and `handle_run_script_sim` in `app.rs`) — the difference is that currently `pc` advances past the action instruction, so the script continues from the next instruction on resume. A coroutine model would be a higher-level abstraction, potentially a `plan()` or `sequence()` block in GrimScript that desugars into the existing yield mechanics with auto-generated state tracking. This is a game design decision as much as an engineering one — it makes scripts easier to write but potentially less interesting. Worth prototyping to see how it feels.

---

## C-01: Removed Compiler Scaffolding — Re-add When Needed

**Priority: Low**

Several compiler scaffolding items were removed to eliminate warnings. Re-add them when their features are implemented:

- **`action_is_void()` in `builtins.rs`** — Indicated whether an action is void (all actions are). Useful if non-void actions are added (e.g., queries that also consume a tick).
- **`FuncDef.line` field in `emit.rs`** — Stored the source line of function definitions. Needed for compiler error messages that reference function definition locations.
- **`Scope.local_base` field in `symbol_table.rs`** — Tracked the offset base for local variable numbering in function scopes. Needed for nested function scopes or closure support.
- **`SymbolTable::in_function()` in `symbol_table.rs`** — Checked whether the compiler is currently inside a function scope. Useful for scope-dependent compilation logic (e.g., disallowing certain statements at module level vs function level).

---

## MOD-01: Entity Behaviors

**Priority: Medium**

Entities now have soul scripts (type `.gs` files) that give them autonomous behavior. The remaining work is adding data-driven behaviors in `mod.toml` for modders who want simple AI without writing GrimScript.

### Planned work

- Data-driven behavior system in `mod.toml` as an alternative to soul scripts
- `[[entities.behaviors]]` section in mod.toml for simple AI patterns
- Built-in behaviors: attack_nearest, flee_when_low, move_toward, idle
- Data-driven periodic behaviors — interval + effects (reuse effect engine)
- Behavior cooldowns — per-entity cooldown tracking
- Behavior conditions — only activate when condition met
- Target resolution for behaviors — "nearest_enemy", "nearest_ally", "owner"
- Behavior validation at load time

---

## MOD-02: Lua Scripting Layer

**Priority: N/A — Resolved**
**Status: Complete** — See CHANGELOG entries M-01 through M-10.

The `deadcode-lua` crate provides a full Lua 5.4 runtime (mlua 0.10) for mod logic. All command logic, triggers, buff callbacks, and init effects are now Lua-only. TOML is data-only (types, entities, resources, buff stats). The old TOML effect system was completely removed (M-08, M-09).

Implemented: sandbox (os/io/debug stripped), deterministic RNG via SimRng, full ctx API (damage, heal, spawn, modify_stat, resources, buffs, queries), event triggers (`mod.on()`), multi-tick coroutines (`ctx:yield_ticks()`), hot-reload (`mod.lua` re-execution), error wrapping with file/line info.

---

## MOD-03: Mod UI

**Priority: Low — Future**

Config panels and HUD elements for mods.

### Planned work

- `[[settings]]` in mod.toml — define mod config options (toggle, slider, dropdown)
- Settings panel in editor UI — renders mod settings, persists to save file
- IPC: ModSettings message (Rust<->JS) for setting sync
- Custom HUD widgets — mod-defined resource bars, counters, timers (rendered on strip or editor panel)

---

## MOD-04: Backlog Ideas

**Priority: Low**

- Prototype inheritance — entity types extending base types
- Mod browser/manager UI — list installed mods, enable/disable, load order
- Hot-reload for mod.toml — detect file changes, reload without restart
