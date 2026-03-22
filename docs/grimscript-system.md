# GrimScript System Guide

How the scripting and simulation system works, and how to extend it.

## Architecture Overview

```
GrimScript source (.gs files)
    │
    ▼
┌─────────────────────────┐
│  grimscript-lang crate  │
│  Lexer → Parser → AST   │
└────────┬────────────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
 Tree-walk   Compiler
 Interpreter (deadcode-sim/compiler)
 (terminal   │
  one-liners ▼
  only)   CompiledScript (flat IR)
              │
              ▼
          Executor (deadcode-sim/executor)
          Stack machine, 10k step limit
              │
              ▼
          SimWorld tick loop
          Deterministic, seeded RNG
          (always running from game open)
```

Two execution paths exist for the same source code:

- **Interpreter** (`grimscript-lang`) — tree-walking, used only for **terminal one-liners**. Runs in a thread, no sim connection.
- **Compiler + Executor** (`deadcode-sim`) — compiles AST to stack-based IR, runs deterministically inside the sim tick loop. This is the execution path for **Run/Debug buttons** and the real game.

The sim runs continuously from game open. The Run button compiles the script to IR and hot-swaps the summoner's `ScriptState` (full reset: PC, stack, variables discarded; entity keeps position, health, world state). A `[reload] Script recompiled and loaded` console message is emitted on successful hot-swap. On the next tick, the executor picks up the new script. The Stop button clears the entity's script state.

Both paths share the same parser and AST. Builtins need to be registered in both (static builtins in both, custom mod commands handled dynamically). Both paths also share the same **available commands gating** — an `Option<HashSet<String>>` that restricts which game builtins can be used (stdlib is always allowed).

**Parity testing:** Integration tests in `crates/deadcode-app/tests/interpreter_compiler_parity.rs` run identical GrimScript through both paths and compare outputs. Known intentional divergences are documented there: `float()` (interpreter supports, compiler rejects), game builtin stubs (interpreter returns dummy values), and string display formatting in list contexts.

## Script Lifecycle

1. Player writes `.gs` file in the editor (CodeMirror)
2. Editor sends source to Rust via IPC
3. **Run/Debug** (unified path): `deadcode_sim::compiler::compile_source_full()` lexes, parses, compiles to IR (with available commands gating + custom command definitions). The `CompiledScript` is assigned to the summoner's `ScriptState`. On the next sim tick, the executor picks it up and runs it.
4. **Terminal one-liners**: `grimscript_lang::run_script()` lexes, parses, interprets directly in a thread (with available commands gating). No sim connection.
5. **Stop**: Clears the entity's `ScriptState`. The sim keeps running, but the entity stops executing.

## Simulation Execution Model

Each entity with a script has a `ScriptState`:
- `pc` — program counter (index into instruction vec)
- `stack` — value stack for computation
- `variables` — slot-indexed variable storage
- `call_stack` — function call frames
- `step_limit_hit` — true if the entity hit the 10k step limit this tick (triggers warning event)

Soul scripts loop via a `soul()` function. `CompiledScript.soul_entry_pc` stores the PC of the auto-generated `Call soul()` instruction. When the script halts:
- If `soul_entry_pc` is `Some(pc)`: `reset_for_soul_loop(pc)` — jump to soul(), preserve global variables
- If `soul_entry_pc` is `None`: script halts (no looping)
- On error: `reset_for_restart(entity_id)` — full reset to PC=0, clear all variables

Per tick, the world:
1. Decrements `spawn_ticks_remaining` on spawning entities
2. Shuffles ready entity IDs — excludes those still spawning (seeded RNG for determinism)
3. For each entity: processes active channel (if present) or takes script state out, calls `executor::execute_unit()`
4. Executor steps instructions until one of:
   - **Action instruction** (move, attack, etc.) → yields, tick consumed
   - **Instant action** (print, query commands) → handled by `try_handle_instant()`, re-enters executor without consuming tick
   - **Halt** → script finished
   - **Error** → script stops
   - **10,000 steps** → auto-yields with implicit `wait()` and emits a warning: `[warning] Script exceeded step limit (10000 instructions) — auto-yielded`
5. Collects actions, resolves them against world state
6. Ticks passive systems (cooldowns)
7. Ticks buffs (per_tick effects, duration decrement, expiry handling)
8. Flushes pending spawns/despawns
9. Processes triggers (match events against registered triggers, check filters/conditions, fire effects)

## IR Instruction Categories

| Category | Consumes tick? | Examples |
|----------|---------------|----------|
| Stack ops | No | LoadConst, LoadVar, StoreVar, Pop, Dup |
| Arithmetic | No | Add, Sub, Mul, Div, Mod, Negate |
| Comparison | No | CmpEq, CmpNe, CmpLt, CmpGt, CmpLe, CmpGe, Contains, NotContains |
| Boolean | No | Not, IsNone, IsNotNone |
| Control flow | No | Jump, JumpIfFalse, JumpIfTrue |
| Functions | No | Call, Return |
| Data structures | No | BuildList, BuildDict, Index, StoreIndex, GetAttr |
| Stdlib | No | Len, Abs, Range, IntCast, StrCast, TypeOf, Min2, Max2, ListAppend, DictKeys/Values/Items/Get, Percent, Scale, Random |
| Locals | No | LoadLocal, StoreLocal (function-scoped variables) |
| **Actions** | **Yes** | ActionCustom(name), Wait |
| **Queries** | **No** | QueryCustom(name) |
| Misc | No | Print, Halt |

48 total instruction variants. All hardcoded game builtins (queries, actions, instant effects) were removed in S-36. All game commands now compile to `ActionCustom(name)` and are dispatched to Lua via the `CommandHandler` trait.

**Key distinctions**:
- `ActionCustom` is the only action instruction — it consumes the tick and dispatches to Lua.
- Print emits output without consuming the tick (handled as an instant action by `try_handle_instant()` in the tick loop).
- Stdlib instructions (Len, Abs, etc.) execute inline without consuming the tick.

## Variable Model

- **Global variables**: absolute slot indices, accessed via `LoadVar(slot)` / `StoreVar(slot)`
- **Function locals**: offset from runtime `var_base`, accessed via `LoadLocal(offset)` / `StoreLocal(offset)`
- **Slot 0**: always `self` (EntityRef for the executing entity)
- The compiler's `SymbolTable` tracks which scope each variable belongs to

## Compiler Two-Pass Strategy

1. **Pass 1**: Walk top-level statements, collect `FunctionDef` names/params/bodies
2. **Pass 2**: Emit global code (non-function statements) → auto-call `main()` if defined → `Halt` → emit function bodies
3. **Fixup**: Patch forward-reference `Call` instructions with resolved function PCs

## Available Commands Gating

Command availability is controlled by type capability gating.

**Stdlib classification** (`grimscript-lang/src/builtins.rs`):
- `is_stdlib(name)` — 14 stdlib functions: `print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`, `percent`, `scale`, `random`, `wait`. Always available, bypass the gate entirely. Note: `float()` is classified as stdlib but deliberately produces a compile error in the sim.

**All non-stdlib commands** are defined in Lua via `mod.command()` in `mod.lua`. All hardcoded game builtins (queries, actions, instant effects) have been removed (S-36).

**Gating mechanism**: The compiler accepts `available_commands: Option<HashSet<String>>`:
- `None` → all commands available (used in dev mode and tests)
- `Some(set)` → only commands in the set are allowed; others produce `"'name' is not available yet"` error

**Type-based gating**: Command availability is determined solely by type capability (`commands` on `[[types]]` in `mod.toml`). An entity's effective commands = union of all its types' `commands` lists. If no types define `commands`, all commands are available (`compute_effective_commands()` returns `None`). Computed via `modding::compute_effective_commands()`.

**Dev mode**: When compiled with `--features dev-mode`, the gate is bypassed entirely (`None` passed to compiler, all commands sent to frontend as available).

**Frontend integration**: Rust sends `AvailableCommands { commands, resources, command_info, dev_mode }` via IPC on EditorReady. `command_info` includes metadata (name, description, args) for Lua-defined commands. The frontend uses this to:
- Filter autocomplete: show available commands + build dynamic entries from `command_info` (`grimscript-completion.ts`)
- Filter syntax highlighting: highlight available command names (`grimscript-lang.ts`)

**Where the gate is checked**:
- Compiler: before emitting `ActionCustom` instructions (`emit.rs`). The compiler checks the `custom_commands` map for known commands.

## Value Types (SimValue)

| Type | Description |
|------|-------------|
| `Int(i64)` | Integer (no floats in sim — determinism) |
| `Bool(bool)` | True / False |
| `Str(String)` | String |
| `None` | Null value |
| `List(Vec<SimValue>)` | Ordered list |
| `Dict(IndexMap<String, SimValue>)` | Ordered key-value map (IndexMap — deterministic insertion-order iteration, O(1) lookup) |
| `EntityRef(EntityId)` | Lightweight reference to an entity, resolved via queries |

---

# How to Add New Things

## Adding a New Command via Lua (Recommended)

The simplest way to add a new command is in `mod.lua`. No Rust code needed:

```lua
local mod = require("void")

mod.command("summon", { description = "Summon a skeleton", args = {} }, function(ctx)
  ctx:spawn("skeleton", { offset = ctx:rand(-300, 300) })
  ctx:output("[summon] A skeleton rises!")
end)
```

This creates a command that:
- Is recognized by the compiler and emits `ActionCustom("summon")` IR
- Consumes a tick when executed (dispatched to Lua via CommandHandler)
- Executes the Lua handler which mutates world state (spawns a skeleton, prints output)
- Shows up in editor autocomplete with the description and args from IPC
- Is syntax-highlighted when available

For multi-tick commands, use `ctx:yield_ticks()`:

```lua
mod.command("channel", { description = "Channel power" }, function(ctx)
  ctx:output("Channeling...")
  ctx:yield_ticks(10, { interruptible = true })
  ctx:output("Power released!")
end)
```

See [Lua Scripting (mod.lua)](modding.md#lua-scripting-modlua) for the full API reference.

## Adding a New Lua ctx Method (Rust)

To add a new operation available to Lua command handlers:

1. **`crates/deadcode-lua/src/api.rs`** — add a method on `CtxUserData`
2. Update the `__void_cmd_wrapper` method list in the same file
3. Use it in `mod.lua`: `ctx:my_method(args)`

## Adding a New Stdlib Builtin (Rust, Advanced)

For new functions callable directly from GrimScript (not Lua commands):

1. **`crates/deadcode-sim/src/ir.rs`** — add `Instruction` variant
2. **`crates/deadcode-sim/src/executor.rs`** — execute the instruction
3. **`crates/deadcode-sim/src/compiler/builtins.rs`** — add mapping so the compiler emits the new instruction
4. **`crates/grimscript-lang/src/builtins.rs`** — add to `is_builtin()` and `call_builtin()` for interpreter path
5. **`editor-ui/src/codemirror/`** — add to autocomplete and highlighting

Note: All hardcoded game builtins (queries, actions, instant effects) were removed in S-36. Currently only stdlib functions and `ActionCustom` exist as IR instructions. New game commands should be Lua-based, not IR builtins.

## Adding a New Entity Attribute

Entity attributes are accessed via dot notation: `entity.health`, `self.position`.

One file: **`crates/deadcode-sim/src/query.rs`** — add a match arm in `get_entity_attr()`:

```rust
"my_attr" => Ok(SimValue::Int(e.my_field)),
```

If the attribute requires a new field on `SimEntity`, also update `crates/deadcode-sim/src/entity.rs`.

## Adding a New IPC Message

Three files:

1. **`crates/deadcode-editor/src/ipc.rs`** — add variant to `JsToRust` or `RustToJs` enum with `#[serde(rename = "my_message")]`
2. **`editor-ui/src/ipc/types.ts`** — add the message shape to `RustToJsMessage` or `JsToRustMessage` union type
3. **`editor-ui/src/ipc/bridge.ts`** — add a `case 'my_message':` handler in the switch (for RustToJs), or use `sendToRust()` (for JsToRust)

Then handle the message in `crates/deadcode-app/src/app.rs` → `poll_editor_ipc()`.

## Adding a New SimEvent

Events are how the sim communicates state changes to the rendering layer.

1. **`crates/deadcode-sim/src/world.rs`** — add variant to `SimEvent` enum
2. **`crates/deadcode-sim/src/action.rs`** — emit the event in `resolve_action()`
3. **`crates/deadcode-app/src/app.rs`** — handle the event in the sim tick block (forward to UI, sync to UnitManager, etc.)

## Adding a New Script Type

1. **`crates/deadcode-editor/src/scripts.rs`** — add variant to `ScriptType` enum, update `as_str()` and `infer_type()`
2. **`editor-ui/src/state/scriptTypes.ts`** — add entry to `TYPE_LABELS` and `TYPE_ORDER`

## Key Files Reference

| Area | File | Purpose |
|------|------|---------|
| Language | `crates/grimscript-lang/src/builtins.rs` | Interpreter builtin functions, `is_stdlib()` classification |
| Language | `crates/grimscript-lang/src/interpreter.rs` | Tree-walking interpreter |
| Language | `crates/grimscript-lang/src/ast.rs` | AST node types |
| Language | `crates/grimscript-lang/src/parser.rs` | Pratt parser |
| Sim IR | `crates/deadcode-sim/src/ir.rs` | Instruction variants, CompiledScript |
| Sim exec | `crates/deadcode-sim/src/executor.rs` | Stack machine execution |
| Sim world | `crates/deadcode-sim/src/world.rs` | SimWorld, tick loop, events, trigger processing |
| Sim actions | `crates/deadcode-sim/src/action.rs` | UnitAction enum (Wait/Print/Custom/Query), resolve_action(), CommandDef, BuffDef, CommandHandler trait |
| Sim queries | `crates/deadcode-sim/src/query.rs` | Entity attribute access (GetAttr instruction) |
| Sim entities | `crates/deadcode-sim/src/entity.rs` | SimEntity, EntityConfig, ScriptState, ActiveBuff |
| Compiler | `crates/deadcode-sim/src/compiler/builtins.rs` | CommandMeta, stdlib classification |
| Compiler | `crates/deadcode-sim/src/compiler/emit.rs` | AST → IR emission, available commands gate |
| Compiler | `crates/deadcode-sim/src/compiler/symbol_table.rs` | Variable scope tracking |
| Lua runtime | `crates/deadcode-lua/src/lib.rs` | LuaModRuntime, CommandHandler impl |
| Lua runtime | `crates/deadcode-lua/src/api.rs` | ctx methods (damage, heal, spawn, etc.) + mod registration API |
| Lua runtime | `crates/deadcode-lua/src/coroutine.rs` | Coroutine lifecycle (create, resume, cancel) |
| App | `crates/deadcode-app/src/app.rs` | Game loop, sim integration, IPC dispatch |
| Modding | `crates/deadcode-app/src/modding.rs` | Mod manifest types, loading, sprite registries |
| Mod data | `mods/core/mod.toml` | Base game mod: types, entities, resources, buff stats |
| Mod logic | `mods/core/mod.lua` | Base game mod: commands, triggers, init, buff callbacks |
| Entity config | `crates/deadcode-sim/src/entity.rs` | `EntityConfig` for stat overrides at spawn |
| Execution | `crates/deadcode-editor/src/execution.rs` | Script execution manager |
| IPC | `crates/deadcode-editor/src/ipc.rs` | Rust-side message enums |
| IPC | `editor-ui/src/ipc/types.ts` | TypeScript message types |
| IPC | `editor-ui/src/ipc/bridge.ts` | JS-side message handler |
| Editor | `editor-ui/src/codemirror/grimscript-completion.ts` | Autocomplete (stdlib + Lua commands filtered by available set) |
| Editor | `editor-ui/src/codemirror/grimscript-lang.ts` | Syntax highlighting (stdlib + commands filtered by available set) |
| Editor state | `editor-ui/src/state/store.ts` | `availableCommands` + `commandInfo` state (set via IPC) |
| Scripts | `crates/deadcode-editor/src/scripts.rs` | Script types, file storage |
| Parity tests | `crates/deadcode-app/tests/interpreter_compiler_parity.rs` | Interpreter vs compiler output comparison (44 tests) |
