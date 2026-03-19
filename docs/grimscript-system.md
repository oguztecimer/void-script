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

Per tick, the world:
1. Decrements `spawn_ticks_remaining` on spawning entities
2. Shuffles ready entities — excludes those still spawning (seeded RNG for determinism)
3. For each entity: takes script state out, calls `executor::execute_unit()`
3. Executor steps instructions until one of:
   - **Action instruction** (move, attack, etc.) → yields, tick consumed
   - **Halt** → script finished
   - **Error** → script stops
   - **10,000 steps** → auto-yields with implicit `wait()` and emits a warning: `[warning] Script exceeded step limit (10000 instructions) — auto-yielded`
4. Collects actions, resolves them against world state
5. Emits events (EntityMoved, EntityDamaged, etc.) for the rendering layer

## IR Instruction Categories

| Category | Consumes tick? | Examples |
|----------|---------------|----------|
| Stack ops | No | LoadConst, LoadVar, StoreVar, Pop, Dup |
| Arithmetic | No | Add, Sub, Mul, Div, Mod, Negate |
| Comparison | No | CmpEq, CmpLt, CmpGe, etc. |
| Boolean | No | Not, IsNone, IsNotNone |
| Control flow | No | Jump, JumpIfFalse, JumpIfTrue |
| Functions | No | Call, Return |
| Data structures | No | BuildList, BuildDict, Index, StoreIndex, GetAttr |
| Stdlib | No | Len, Abs, Range, IntCast, StrCast, TypeOf, Min2, Max2, ListAppend, DictKeys/Values/Items/Get, Percent, Scale |
| Locals | No | LoadLocal, StoreLocal (function-scoped variables) |
| **Queries** | **No** | QueryScan, QueryNearest, QueryGetHealth, etc. |
| **Actions** | **Yes** | ActionMove, ActionAttack, ActionFlee, ActionWait, ActionSetTarget, ActionConsult, ActionRaise, ActionHarvest, ActionPact, ActionCustom(name) |
| Misc | Yes (Print) / No (Halt) | Print, Halt |

**Key distinction**: Queries read world state instantly. Actions mutate world state and consume the tick.

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

Not every GrimScript builtin is available from the start. The game progressively unlocks commands.

**Three-tier classification** (`grimscript-lang/src/builtins.rs`):
- `is_stdlib(name)` — 12 stdlib functions: `print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`, `percent`, `scale`. Always available, bypass the gate entirely.
- `is_game_builtin(name)` — all builtins that aren't stdlib: `move`, `scan`, `attack`, `consult`, etc. Subject to gating.
- `is_builtin(name)` — both stdlib + game builtins.

**Gating mechanism**: Both the interpreter and compiler accept `available_commands: Option<HashSet<String>>`:
- `None` → all commands available (used in dev mode and tests)
- `Some(set)` → only game builtins in the set are allowed; others produce `"'name' is not available yet"` error

This applies to both hardcoded game builtins and custom mod-defined commands. Custom commands not in the `initial` set cannot be used until unlocked at runtime.

**Initial available set**: Defined in the mod's `[commands].initial` field (see `mods/core/mod.toml`). The base game starts with `consult`, `raise`, `harvest`, `pact`. Multiple mods' command sets are merged.

**Dev mode**: When compiled with `--features dev-mode`, the gate is bypassed entirely (`None` passed to interpreter/compiler, all game builtins + custom commands sent to frontend as available).

**Frontend integration**: Rust sends `AvailableCommands { commands, command_info }` via IPC on EditorReady. `command_info` includes metadata (name, description, args) for custom mod commands. The frontend uses this to:
- Filter autocomplete: show available game commands + build dynamic entries for custom commands from `command_info` (`grimscript-completion.ts`)
- Filter syntax highlighting: highlight available game functions + custom command names (`grimscript-lang.ts`)

**Where the gate is checked**:
- Interpreter: before `call_builtin()` and before entity method dispatch (`interpreter.rs`)
- Compiler: before emitting Query/Action/CustomAction instructions and before method call fallback (`emit.rs`). The compiler uses `classify_with_custom()` which checks both static builtins and the `custom_commands` map.

**Unlocking commands at runtime**: `App::available_commands` is a `Vec<String>` in `deadcode-app` (preserves insertion order), initially populated from mod manifests (`[commands].initial`). Push a command name, then call `send_available_commands()` to push the updated set to the frontend and `execution_manager.set_available_commands()` to update the interpreter gate. The `help()` command lists commands in this insertion order.

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

## Adding a New Command via Modding (Recommended)

The simplest way to add a new command is via `[[commands.definitions]]` in `mod.toml`. No Rust code needed:

```toml
[[commands.definitions]]
name = "summon"
description = "Summon a skeleton at your position"
args = []
effects = [
  { type = "spawn", entity_type = "skeleton", offset = 1 },
  { type = "output", message = "[summon] A skeleton rises!" },
]

[commands]
initial = ["summon"]   # Make it available at game start
```

This creates a command that:
- Is recognized by the compiler and emits `ActionCustom("summon")` IR
- Consumes a tick when executed (it's an action)
- Resolves effects against the world (spawns a skeleton, prints output)
- Shows up in editor autocomplete with the description and args from IPC
- Is syntax-highlighted when available

See [Custom Command Definitions](modding.md#custom-command-definitions) for the full effect type reference.

## Adding a New Builtin Function (Rust)

For behavior that can't be expressed as data-driven effects, add a new builtin in Rust. There are two paths depending on where the function should work.

### Interpreter-only (terminal one-liners, no sim)

Two files:

1. **`crates/grimscript-lang/src/builtins.rs`**
   - Add the name to `is_builtin()` match list
   - Add an arm in `call_builtin()` that receives `args: Vec<Value>` and returns `Result<Value, GrimScriptError>`

### Sim builtin (full pipeline)

Five files for a query, five for an action:

#### Step 1: IR instruction — `crates/deadcode-sim/src/ir.rs`

Add a variant to the `Instruction` enum:
```rust
// Query (instant):
QueryMyThing,
// Action (yields):
ActionMyThing,
```

#### Step 2: Executor — `crates/deadcode-sim/src/executor.rs`

Add a match arm in the main `execute_unit()` loop:
```rust
// Query example:
Instruction::QueryMyThing => {
    let target = pop_entity_ref(&mut state.stack)?;
    let result = query::my_thing(world, target)?;
    state.stack.push(result);
}

// Action example:
Instruction::ActionMyThing => {
    let arg = pop_int(&mut state.stack)?;
    state.yielded = true;
    return Ok(Some(UnitAction::MyThing { arg }));
}
```

For actions, also add the variant to `UnitAction` in `action.rs` and implement `resolve_action()`.

#### Step 3: Compiler builtin mapping — `crates/deadcode-sim/src/compiler/builtins.rs`

- Add variant to `QueryBuiltin` or `ActionBuiltin` enum
- Add `"my_thing"` arm in `classify()`
- Add mapping in `query_instruction()` or `action_instruction()`
- Set expected arg count in `query_expected_args()` or `action_expected_args()`
- For 0-arg queries that should auto-push `self`, add to `query_takes_implicit_self()`

#### Step 4: Interpreter stub — `crates/grimscript-lang/src/builtins.rs`

Add to `is_builtin()` and `call_builtin()` with a stub return value so the editor's Run button works without the sim.

#### Step 5: Editor autocomplete + highlighting — `editor-ui/src/codemirror/`

- `grimscript-completion.ts` — add `{ label, detail, info }` to `gameCommandCompletions` (filtered by available commands) or `stdlibCompletions` (always shown)
- `grimscript-lang.ts` — add the name to `allGameFunctions` set (highlighted only when available) or `stdlibFunctions` set (always highlighted)

### Concrete example: adding `summon(type_str)` as a hardcoded Rust action

> **Note**: For simple cases, prefer defining custom commands in `mod.toml` instead. This Rust approach is only needed for behavior that can't be expressed as data-driven effects.

```rust
// 1. ir.rs — add instruction
ActionSummon,

// 2a. action.rs — add action variant
pub enum UnitAction {
    // ...existing...
    Summon { unit_type: String },
}

// 2b. action.rs — implement resolution
UnitAction::Summon { unit_type } => {
    // Create a new entity near the summoner
    if let Some(entity) = world.get_entity(entity_id) {
        let pos = entity.position;
        let new_id = world.spawn_entity(unit_type.clone(), unit_type.clone(), pos + 1);
        events.push(SimEvent::EntitySpawned {
            entity_id: new_id,
            entity_type: unit_type,
            position: pos + 1,
        });
    }
}

// 2c. executor.rs — handle instruction
Instruction::ActionSummon => {
    let unit_type = pop_str(&mut state.stack)?;
    state.yielded = true;
    return Ok(Some(UnitAction::Summon { unit_type }));
}

// 3. compiler/builtins.rs
// In ActionBuiltin enum:
Summon,
// In classify():
"summon" => BuiltinKind::Action(ActionBuiltin::Summon),
// In action_instruction():
ActionBuiltin::Summon => Instruction::ActionSummon,
// In action_expected_args():
ActionBuiltin::Summon => 1,

// 4. grimscript-lang/builtins.rs
// In is_builtin():
| "summon"
// In call_builtin():
"summon" => {
    send_output(output_tx, "[summon] Summoning...");
    Ok(Value::None)
}
```

```typescript
// 5. editor-ui/src/codemirror/grimscript-completion.ts
// Add to gameCommandCompletions (shown only when available):
{ label: 'summon', detail: '(type)', info: 'Summon a new unit' },

// 5. editor-ui/src/codemirror/grimscript-lang.ts
// Add to allGameFunctions set (highlighted only when available):
'summon',
```

To make the command initially available, add `"summon"` to the `[commands].initial` list in the mod's `mod.toml` (e.g., `mods/core/mod.toml`). Or at runtime: insert it into `App::available_commands` and call `send_available_commands()`.

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
| Language | `crates/grimscript-lang/src/builtins.rs` | Interpreter builtin functions, `is_stdlib()` / `is_game_builtin()` classification |
| Language | `crates/grimscript-lang/src/interpreter.rs` | Tree-walking interpreter, available commands gate |
| Language | `crates/grimscript-lang/src/ast.rs` | AST node types |
| Language | `crates/grimscript-lang/src/parser.rs` | Pratt parser |
| Sim IR | `crates/deadcode-sim/src/ir.rs` | Instruction enum, CompiledScript |
| Sim exec | `crates/deadcode-sim/src/executor.rs` | Stack machine execution |
| Sim world | `crates/deadcode-sim/src/world.rs` | SimWorld, tick loop, events |
| Sim actions | `crates/deadcode-sim/src/action.rs` | UnitAction enum, resolution, CommandDef/CommandEffect types |
| Sim queries | `crates/deadcode-sim/src/query.rs` | Entity queries, attribute access |
| Sim entities | `crates/deadcode-sim/src/entity.rs` | SimEntity, ScriptState |
| Compiler | `crates/deadcode-sim/src/compiler/builtins.rs` | Builtin → IR mapping |
| Compiler | `crates/deadcode-sim/src/compiler/emit.rs` | AST → IR emission, available commands gate |
| Compiler | `crates/deadcode-sim/src/compiler/symbol_table.rs` | Variable scope tracking |
| App | `crates/deadcode-app/src/app.rs` | Game loop, sim integration, IPC dispatch, available commands state, RunScript→sim compile+assign |
| Modding | `crates/deadcode-app/src/modding.rs` | Mod manifest types, loading, sprite/command registries, embedded fallback |
| Mod manifest | `mods/core/mod.toml` | Base game mod: entity defs, spawns, initial commands, command definitions with effects |
| Entity config | `crates/deadcode-sim/src/entity.rs` | `EntityConfig` for stat overrides at spawn |
| Execution | `crates/deadcode-editor/src/execution.rs` | Script execution manager, threads available commands to interpreter |
| IPC | `crates/deadcode-editor/src/ipc.rs` | Rust-side message enums |
| IPC | `editor-ui/src/ipc/types.ts` | TypeScript message types |
| IPC | `editor-ui/src/ipc/bridge.ts` | JS-side message handler |
| Editor | `editor-ui/src/codemirror/grimscript-completion.ts` | Autocomplete (stdlib always, game commands + custom commands filtered by available set) |
| Editor | `editor-ui/src/codemirror/grimscript-lang.ts` | Syntax highlighting (stdlib always, game + custom functions filtered by available set) |
| Editor state | `editor-ui/src/state/store.ts` | `availableCommands` + `commandInfo` state (set via IPC) |
| Scripts | `crates/deadcode-editor/src/scripts.rs` | Script types, file storage |
| Parity tests | `crates/deadcode-app/tests/interpreter_compiler_parity.rs` | Interpreter vs compiler output comparison (31 tests) |
