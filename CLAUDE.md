# CLAUDE.md — VOID//SCRIPT

## Project Overview

Desktop automation game with a necromancer theme (the base mod is called "core"). Players write GrimScript (a custom Python-like language) to control units on a 1D strip rendered above the taskbar/dock. Scripts compile to a deterministic stack-based IR and execute in a tick-based simulation engine.

Hybrid Rust + TypeScript/React desktop app targeting Windows, macOS, and Linux.

## Tech Stack

- **Core:** Rust (edition 2024), Cargo workspace
- **UI:** React 19, TypeScript 5.7, Vite 6, CodeMirror 6
- **Desktop:** winit 0.30 (windowing), wry 0.50 (webview), softbuffer 0.4 + tiny-skia 0.12 (2D CPU rendering)
- **State:** zustand (frontend), crossbeam-channel (Rust IPC)
- **Data:** indexmap 2 (deterministic ordered dicts in sim)
- **Modding:** mlua 0.10 + Lua 5.4 vendored (mod logic scripting)
- **Platform:** objc2 (macOS), windows crate (Windows), x11rb (Linux)

## Repository Structure

```
crates/
  deadcode-app/        # Entry point — winit event loop, App state machine, sim integration
  deadcode-desktop/    # 2D rendering, window mgmt, units, animations, sprites, save/load
  deadcode-editor/     # WebView editor windows, IPC bridge, tab/script management
  deadcode-lua/        # Lua 5.4 runtime for mod logic (commands, triggers, buff callbacks)
  grimscript-lang/     # Custom scripting language (lexer → parser → AST → tree-walking interpreter)
  deadcode-sim/        # Deterministic simulation engine + GrimScript→IR compiler
editor-ui/             # React/TS editor UI — CodeMirror, panels, toolbar, debug UI
scripts/               # User scripts directory (created at runtime)
```

## Build & Run

```bash
# Full application (editor UI must be built first)
cd editor-ui && npm install && npm run build && cd ..
cargo build
cargo run

# Editor UI dev server (for UI-only iteration)
cd editor-ui && npm install && npm run dev   # port 5173

# Release
cargo build --release
```

Editor UI is embedded into the Rust binary via `rust-embed` in `deadcode-editor/build.rs`. Run `npm run build` in `editor-ui/` before `cargo build` when changing frontend code.

## Testing

```bash
cargo test                          # All Rust tests (177 tests)
cargo test -p deadcode-sim          # Sim engine + compiler tests
cargo test -p grimscript-lang       # Language crate only
cargo test -p deadcode-app --test interpreter_compiler_parity  # Parity tests
cd editor-ui && npx tsc --noEmit    # TypeScript type check
```

## Architecture

**Five layers:**

1. **Desktop** (`deadcode-app` + `deadcode-desktop`) — winit event loop, softbuffer+tiny-skia rendering, sprite-based units on a transparent always-on-top strip window, system tray, per-pixel hit testing, save/load, fullscreen detection
2. **Editor** (`deadcode-editor` + `editor-ui/`) — wry WebView hosting React UI, JSON IPC between Rust and JS, multi-tab script editing with CodeMirror 6, debug panel
3. **Language** (`grimscript-lang`) — lexer, Pratt parser, tree-walking interpreter with debugger (breakpoints, step over/into/out), dynamic types (int, float, string, bool, None, list, dict, tuple, entity), enum definitions (auto-incrementing integer members), match/case statements (literal, enum member, wildcard, OR patterns)
4. **Simulation** (`deadcode-sim`) — deterministic tick-based engine. GrimScript compiles to stack-based IR; executor steps each unit's program counter until an action yields. 1D integer positions, no floats, seeded RNG for determinism.
5. **Lua Runtime** (`deadcode-lua`) — Lua 5.4 scripting for mod logic. Implements the `CommandHandler` trait from `deadcode-sim`. Commands use Lua coroutines for multi-tick behavior (`ctx:yield_ticks(N)`). TOML is data-only (types, entities, resources, buff stats); Lua handles all behavior (commands, triggers, buff callbacks, init).

### Simulation Engine (`deadcode-sim`)

```
src/
  lib.rs          — Module declarations, re-exports
  rng.rs          — SplitMix64 PRNG + Fisher-Yates shuffle (deterministic)
  value.rs        — SimValue: Int, Bool, Str, None, List, Dict(IndexMap), EntityRef (no floats)
  error.rs        — SimError types (SimErrorKind: TypeError, DivisionByZero, IndexOutOfBounds, KeyNotFound, EntityNotFound, StackUnderflow, InvalidVariable, StackOverflow, Overflow, StepLimitExceeded, Runtime)
  entity.rs       — SimEntity (unified stats HashMap, types: Vec<String>, owner: Option<EntityId>), EntityId, EntityConfig, ScriptState (incl. step_limit_hit, error recovery), CallFrame, spawn_ticks_remaining
  ir.rs           — 48 stack-based Instruction variants, CompiledScript, FunctionEntry
  executor.rs     — Stack machine: steps IR until action/halt/error, 10k step limit per tick (warns on limit hit)
  world.rs        — SimWorld: entity storage, tick() loop (main brain + entity shuffle), event collection, snapshots, global resources, WorldAccess API, entity_types_registry
  action.rs       — UnitAction enum, resolve_action(), CommandDef, BuffDef, CommandHandler trait, CommandHandlerResult, CoroutineHandle, BuffCallbackType
  query.rs        — get_entity_attr() — entity attribute access for GetAttr instruction
  compiler/       — GrimScript AST → IR compiler (feature-gated behind "compiler")
    mod.rs        — compile(), compile_source(), compile_source_with(), compile_source_full(source, available_commands, custom_commands, enable_brain_loop), source_defines_function(), initial_variables()
    emit.rs       — AST walk, instruction emission, jump patching, function compilation, available command gating
    symbol_table.rs — Scope tracking, global slots vs function-local offsets
    builtins.rs   — CommandMeta struct, classify_stdlib() for stdlib classification
    error.rs      — CompileError
```

**Feature flag:** `deadcode-sim` has an optional `compiler` feature that enables the `grimscript-lang` dependency. Without it, the sim crate is standalone (IR types + executor only). `deadcode-app` enables this feature.

**Execution model:**
- GrimScript source → lexer → parser → AST → compiler → `CompiledScript` (flat instruction vec + function table)
- Each entity has a `ScriptState` (program counter, value stack, variable slots, call stack)
- Brain scripts loop via a `brain()` function: top-level code runs once (init), `brain()` is auto-called each tick with global variables preserved. `CompiledScript.brain_entry_pc` stores the PC of the auto-generated `Call brain()` instruction. `ScriptState::reset_for_brain_loop(brain_pc)` jumps to `brain_entry_pc`, clears stack/call_stack, but preserves global variables. Scripts without `brain()` run once and halt.
- Per tick: seeded shuffle entity order, execute each until an action yields
- Custom mod commands consume the tick — the executor yields after ActionCustom
- `self` is pre-allocated at variable slot 0 as `EntityRef` for the executing entity
- Error recovery uses `reset_for_restart(entity_id)` (PC=0, clears all vars) — the script re-runs init + brain from scratch

**Mod dependencies:** `depends_on` and `conflicts_with` fields in `[mod]` are enforced at load time. Mods are topologically sorted by dependency graph (Kahn's algorithm, alphabetical tie-breaking). Missing deps → mod skipped with warning (cascading). Conflicts → second-loaded mod skipped. Cycles → fallback to alphabetical with error log.

**Library files:** Mods can provide `.grim` files via `commands.libraries` in `mod.toml`. Library source is loaded at mod time, concatenated across mods (in load order), and prepended to player scripts before compilation. Functions defined in libraries are available in player scripts as if defined at the top. Subject to the same command gating. Flat namespace, first-loaded-wins.

**Available commands:** Stdlib functions (`print`, `len`, `range`, `abs`, `min`, `max`, `int`, `str`, `type`, `percent`, `scale`, `random`) are always available. Note: `float()` is classified as stdlib but deliberately produces a compile error in the sim ("float() is not supported in simulation mode"). All other commands are defined in Lua via `mod.command()` in `mod.lua`. The compiler receives command metadata via `HashMap<String, CommandMeta>` and an `available_commands: Option<HashSet<String>>` for type-based gating. In **dev mode**, all commands are available (gate bypassed).

**Custom commands:** Mods define commands in Lua via `mod.command(name, opts, handler)`. Commands compile to `ActionCustom(name)` IR instructions. At runtime, `resolve_action()` dispatches to the Lua `CommandHandler`. Multi-tick commands use `ctx:yield_ticks(N)` which suspends the Lua coroutine; a `LuaCoroutineState` is stored on the entity and resumed after N ticks. See `docs/modding.md` for the full Lua API reference.

**Global resources:** World-level integer resources (e.g. `mana`, `bones`) stored in `SimWorld.resources: IndexMap<String, i64>`. Defined by mods via `[resources]` table in `mod.toml`, merged at load time (first-defined wins). Accessible via Lua `ctx:use_resource()`, `ctx:modify_resource()`, `ctx:get_resource()`. **Resource availability:** Resources have an available/unavailable mechanic. `[initial].resources` in `mod.toml` lists initially available resource names; if omitted, all defined resources are available. In dev mode, `available_resources` is `None` (all available). Lua `ctx:set_available_resources()` can set this at runtime.

**Buff system:** Mods define temporary stat modifiers via `[[buffs]]` in `mod.toml`. Each `BuffDef` has `name`, `duration`, `modifiers` (stat→amount), `stackable`, and `max_stacks`. Buff lifecycle callbacks (`on_apply`, `per_tick`, `on_expire`) are defined in Lua via `mod.buff()`. Active buffs tracked per-entity via `SimEntity.active_buffs`. Modifiers directly modify stats on apply and reverse on expire. Stackable buffs accumulate stacks; non-stackable refresh duration. Buff tick processing (step 6b) calls Lua callbacks, decrements durations, and handles expiry (reverse modifiers, run on_expire). Buff definitions stored in `SimWorld.buff_registry`.

**Multi-type entity system:** Entities have a `types: Vec<String>` field containing composable type tags for queries and filtering. The `entity_type: String` field serves as the unique entity definition ID for registry lookups (sprites, configs). `has_type(&self, tag) -> bool` checks membership. `SimWorld.entity_types_registry` stores def ID → types mapping for spawn effects.

**Type definitions (`[[types]]` in mod.toml):** Types are composable tags with optional stats, commands, and brain scripts. Each `TypeDef` has `name`, `brain: bool`, `stats: IndexMap`, `commands: Vec<String>`, and optional `script` path. Entity `[[entities]]` definitions reference types via `types = ["undead", "melee"]`. Stats are merged in type order, then entity-level stats override. Type `.gs` scripts loaded from `grimscript/` directory. Entity definitions require `id` field (unique key); `types` defaults to `[id]` if absent.

**Unified entity stats:** All entity stats live in a single `SimEntity.stats: IndexMap<String, i64>` (deterministic iteration order), accessed via `stat(&self, name) -> i64` (returns 0 if unset), `set_stat(&mut self, name, value)`, and `clamp_stat(&mut self, name)` (clamps to `[0, max_{name}]` if a max exists). There are **no built-in default stats** — entities start with an empty stats map; all stats come from `EntityConfig` applied by mods. `EntityConfig` contains `stats: IndexMap<String, i64>`; `apply_config()` auto-sets `max_health`/`max_shield` when health/shield are defined without explicit max values. In `mod.toml`, entity stats merge from types (in type order) then entity-level overrides. Stats are accessible via entity attribute access (e.g. `entity.armor` via the GetAttr instruction) and Lua `ctx:get_stat()`/`ctx:modify_stat()`.

**Event triggers:** Mods define reactive rules in Lua via `mod.on(event, opts, handler)`. Events include `entity_died`, `entity_spawned`, `entity_damaged`, `command_used`, `channel_completed`. The Lua handler receives event data (entity_id, killer_id, etc.) and a `ctx` for world access. Trigger processing occurs at the end of each tick (step 8) via `CommandHandler::process_triggers()`.

**Unified execution:** The sim runs continuously from game open. Brain scripts are compiled and assigned via `compile_and_assign_all_brains()` at startup (after script store init and initial effects flush) and per-entity via `compile_and_assign_entity_brain()` when entities spawn during gameplay. The caller pre-scans the brain type's source for a `brain()` function via `source_defines_function()` and passes `enable_brain_loop` to the compiler. Saving a type `.gs` file triggers auto-reload: recompiles and hot-swaps all affected entities' `ScriptState` (full reset: PC, stack, variables discarded; entity keeps position/health/world state). Saving an empty brain script clears the entity's `script_state` so it stops executing. The main brain (`main.gs`) runs with a special "main" entity before the entity shuffle each tick. Terminal commands execute against the main brain. The "main" type is always treated as a brain regardless of the `brain` flag in `mod.toml`.

**Main brain:** Script stored as `SimWorld.main_brain: Option<ScriptState>`, backed by a real "main" entity spawned via `spawn_main_brain_entity()`. Runs first every tick (step 2c, before entity shuffle). Can call print and custom commands. Terminal commands execute against the main brain.

**Error recovery:** When a script hits a runtime error, it stores the error on `ScriptState.error`. On the next tick, error recovery kicks in: the error is cleared, script state is fully reset (PC=0, stack/call stack cleared, variables re-initialized with slot 0 = self EntityRef), the entity yields `wait()` for that tick, and a `[error recovery]` message is emitted. The script re-executes from the beginning on the following tick. This prevents permanent script death from transient errors. Applies to entity scripts, main brain, and channel interruptible scripts.

**Command gating (type-based):** Command availability is determined solely by type capability (`commands` on `[[types]]`). An entity's effective commands = union of all its types' `commands` lists. If no types define `commands`, all commands are available (backward compat — `compute_effective_commands()` returns `None`). In dev mode, all commands are available (gate bypassed). Computed via `modding::compute_effective_commands()`.

**Auto-reload on save:** Saving a type `.gs` file in the editor triggers `handle_type_script_reload()`: brain type changed → recompile all entities with that brain; non-brain type changed → recompile all entities including that type (library changed); `main.gs` changed → recompile main brain. Script composition per entity: non-brain type `.gs` (library) + mod libraries + brain `.gs` (execution logic).

**Spawn state:** Dynamically spawned entities (from effects) have `spawn_ticks_remaining > 0` — they play their spawn animation and can't act or be targeted by queries until the timer reaches 0. Duration is computed from the entity type's atlas JSON spawn animation. Initial mod spawns start ready (`spawn_ticks_remaining = 0`). Brain scripts are assigned to spawned entities immediately (via `EntitySpawned` event handling in `do_tick()`), but execution is gated by `is_ready()` — the brain loop doesn't start until spawning completes.

**Death lifecycle:** When an entity dies (`alive = false`), a `SimEvent::EntityDied { entity_id, name, killer_id, owner_id }` is emitted. The sim removes the entity at end-of-tick via `flush_pending()`. The game loop handles the event by calling `UnitManager::kill(uid)`, which plays the "death" animation (if the atlas has one) and marks the unit `pending_destroy`. Units with `pending_destroy` are reaped by `reap_dead()` after their death animation finishes. If no death animation exists, the unit is destroyed immediately.

**Fixed timestep:** The sim runs at exactly 30 TPS via an accumulator in `do_tick()`. Wall-clock delta is accumulated; `sim.tick()` fires once per 33ms. Capped at 4 sim ticks per frame to prevent spiral of death. Animations are sim-driven (advanced once per sim tick via `UnitManager::tick_animations()`), movement interpolation remains render-driven.

**Tick loop** (`SimWorld::tick()`):
1. Derive per-tick RNG: `SimRng::new(seed ^ tick)`. Snapshot entity types and resource values for trigger processing.
2. Decrement spawn timers on spawning entities
2c. Execute main brain (backed by real "main" entity, instant actions only)
3. Shuffle ready entity IDs (excludes spawning entities, includes entities with active channels)
4. For each: check error recovery (if script errored last tick, full reset via `reset_for_restart()` and yield wait), then process active channel if present (phase effects, interruption check), otherwise take script state out, execute, handle instant actions via `try_handle_instant()` (Print — re-enter executor), collect tick-consuming action, put state back. Brain scripts that halt are reset via `reset_for_brain_loop(brain_pc)` to re-enter the `brain()` function next tick (global vars preserved).
5. Resolve all actions against world state
6. Tick passive systems (cooldowns, behavior cooldowns)
6b. Tick buffs: run per_tick effects, decrement durations, handle expiry (reverse modifiers, fire on_expire)
7. Flush pending spawns/despawns
8. Process triggers: match events collected during the tick against registered triggers, check filters and conditions, fire effects. Trigger effects do not re-trigger within the same tick.

### Lua Runtime (`deadcode-lua`)

```
src/
  lib.rs          — LuaModRuntime struct, CommandHandler impl
  sandbox.rs      — Strip unsafe globals (os/io/debug/package)
  api.rs          — ctx userdata methods (damage, heal, spawn, etc.) + mod registration API
  coroutine.rs    — Coroutine lifecycle: create, resume, cancel
  convert.rs      — SimValue ↔ mlua::Value bidirectional conversion
  triggers.rs     — Lua trigger dispatch and buff callbacks
  error.rs        — Error wrapping with file/line info
  tests.rs        — Integration tests for Lua runtime
```

**Dependency graph (no cycles):**
```
deadcode-sim     — defines CommandHandler trait, stores Option<Box<dyn CommandHandler>>
deadcode-lua     — depends on deadcode-sim, implements CommandHandler
deadcode-app     — depends on both, wires LuaModRuntime into SimWorld
```

**Mod directory structure:**
```
mods/core/
  mod.toml        # Data: metadata, types, entities, resources, buff stats
  mod.lua         # Logic: commands, triggers, buff callbacks, init
  grimscript/     # Player brain scripts (unchanged)
  sprites/        # Assets (unchanged)
```

**Command coroutine lifecycle:** `mod.command("name", handler)` → handler wrapped in Lua coroutine → `ctx:yield_ticks(N)` yields coroutine → `LuaCoroutineState` stored on entity → sim ticks down `remaining_ticks` → resume coroutine → repeat or complete. Interruptible yields check entity script for interrupting actions.

**Lua-only logic:** All command logic, triggers, buff callbacks, and init effects are defined in Lua. TOML is data-only (types, entities, resources, buff stats). If no Lua handler exists for a command, the sim emits a "no handler" message.

### IPC

JSON messages as serde-tagged enums in `deadcode-editor/src/ipc.rs` (`JsToRust`/`RustToJs`). JS side types in `editor-ui/src/ipc/types.ts`, handler in `editor-ui/src/ipc/bridge.ts`.

Message categories:
- **Script ops:** ScriptSave, ScriptRequest, ScriptList, ScriptReloaded
- **Debug:** DebugStart, DebugContinue, StepOver/Into/Out, ToggleBreakpoint, DebugPaused/Resumed
- **Simulation:** StartSimulation, StopSimulation, PauseSimulation → SimulationStarted, SimulationStopped, SimulationTick
- **Window:** Minimize, Maximize, Close, DragStart, ResizeStart, Shake, SetSize
- **Console:** ConsoleOutput, ConsoleCommand
- **Game state:** AvailableCommands (Rust→JS, sent on EditorReady; includes commands, resources, command_info, dev_mode)
- **Diagnostics:** ScriptErrorDetail (Rust→JS, variable state dump on script error; entity_id, error, variables, stack, pc)

### Game Loop

`App::do_tick()` in `deadcode-app/src/app.rs`:
1. Unit movement tick (render-driven, uses wall-clock delta)
2. Simulation tick (fixed 30 TPS via accumulator) → animations tick → reap dead units → forward events (spawn, death, output, animation) to editor → assign brain scripts to newly spawned entities → snapshot → sync positions to UnitManager
3. Auto-save timer
4. Fullscreen detection
5. Per-pixel hit testing
6. Editor IPC polling
7. Script execution polling

Render: 30 FPS active / 10 FPS idle. Sim: fixed 30 TPS regardless of render rate.

## Key Conventions

- Workspace deps in root `Cargo.toml`, referenced with `{ workspace = true }`
- Platform code uses `#[cfg(target_os = "...")]`
- Editor UI uses CSS modules
- IPC enums use `#[serde(tag = "type")]`
- Sprite atlases are JSON + PNG pairs; frame durations use `ticks` (sim ticks at 30 TPS, not milliseconds)
- Simulation uses only `i64` (no floats) for determinism; `Dict` uses `IndexMap<String, SimValue>` for deterministic insertion-order iteration with O(1) lookup
- Division/modulo use Python-style floor semantics in both executor and interpreter: `-7 // 2 = -4`, `-7 % 2 = 1` (not C truncating or Euclidean)
- `Lexer::tokenize()` returns `Result<Vec<SpannedToken>, GrimScriptError>` — callers must handle lex errors
- Compiler is feature-gated: `deadcode-sim` stays independent without `grimscript-lang`
- The summoner is defined by the core mod (`mods/core/mod.toml`) like any other entity — entity type, stats, sprite, and pivot are in the mod manifest; initial spawning is handled via `mod.on_init()` in `mod.lua`. If no mods are found, nothing loads (no fallback). Script execution methods find the summoner by entity type `"summoner"`.
- Theme-agnostic sim: no baked-in entity type constants, entity types are runtime strings

## Common Tasks

| Task | Where |
|------|-------|
| Add language builtin | `crates/grimscript-lang/src/builtins.rs` |
| Add sim IR instruction | `crates/deadcode-sim/src/ir.rs` + `executor.rs` |
| Add sim builtin mapping | `crates/deadcode-sim/src/compiler/builtins.rs` |
| Add IPC message | `crates/deadcode-editor/src/ipc.rs` + `editor-ui/src/ipc/types.ts` + `bridge.ts` |
| Add unit/sprite | `crates/deadcode-desktop/src/unit.rs` + atlas in `src/assets/` |
| Game loop logic | `crates/deadcode-app/src/app.rs` → `do_tick()` |
| Rendering | `crates/deadcode-desktop/src/renderer.rs` |
| Editor UI components | `editor-ui/src/components/` |
| Editor state | `editor-ui/src/state/` (zustand) |
| Script storage | `crates/deadcode-editor/src/scripts.rs` |
| Save/load | `crates/deadcode-desktop/src/save.rs` |
| Add Lua mod command | `mods/<mod>/mod.lua` → `mod.command("name", { description = "..." }, function(ctx) ... end)` |
| Add Lua trigger | `mods/<mod>/mod.lua` → `mod.on("event", { filter = {...} }, function(ctx, event) ... end)` |
| Add Lua buff callbacks | `mods/<mod>/mod.lua` → `mod.buff("name", { on_apply = ..., per_tick = ..., on_expire = ... })` |
| Add Lua init handler | `mods/<mod>/mod.lua` → `mod.on_init(function(ctx) ... end)` |
| Add new ctx method for Lua | `crates/deadcode-lua/src/api.rs` → add method on `CtxUserData`, update `__void_cmd_wrapper` method list |
| Define mod resources | `mods/<mod>/mod.toml` → `[resources]` table (name = initial_value), `[initial].resources` list for availability |
| Define buff | `mods/<mod>/mod.toml` → `[[buffs]]` with name, duration, modifiers, per_tick/on_apply/on_expire effects |
| Add trigger event type | `crates/deadcode-sim/src/world.rs` → `SimEvent` enum + emit in tick loop |
| Define entity type | `mods/<mod>/mod.toml` → `[[types]]` with name, brain, stats, commands |
| Add entity with types | `mods/<mod>/mod.toml` → `[[entities]]` with id, types, sprite, stats (entity stats override type stats) |

## Documentation Maintenance

Every change to the project must be reflected in the related `.md` files. When making code changes:

- **`CHANGELOG.md`** — Always update. Add an entry under the appropriate section (`### Simulation Engine`, `### Modding System`, etc.) describing what changed, why, and under which item ID (S-xx, M-xx).
- **`CLAUDE.md`** — Update if the change affects architecture, conventions, types, the common tasks table, or introduces new patterns. Keep descriptions concise and accurate to current code.
- **`docs/modding.md`** — Update if the change affects mod.toml schema, loading/validation behavior, custom command flow, or any modder-facing API.
- **`TODO.md`** — Update if items are resolved or partially resolved, or if new issues are discovered.
- **`bugs&issues.md`** — Add entries for newly discovered bugs. Update status when bugs are fixed.

When in doubt, update the doc. Stale documentation is worse than missing documentation because it misleads.
