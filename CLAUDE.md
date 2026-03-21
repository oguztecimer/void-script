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
- **Platform:** objc2 (macOS), windows crate (Windows), x11rb (Linux)

## Repository Structure

```
crates/
  deadcode-app/        # Entry point — winit event loop, App state machine, sim integration
  deadcode-desktop/    # 2D rendering, window mgmt, units, animations, sprites, save/load
  deadcode-editor/     # WebView editor windows, IPC bridge, tab/script management
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
cargo test                          # All Rust tests (182 tests)
cargo test -p deadcode-sim          # Sim engine + compiler tests
cargo test -p grimscript-lang       # Language crate only
cargo test -p deadcode-app --test interpreter_compiler_parity  # Parity tests
cd editor-ui && npx tsc --noEmit    # TypeScript type check
```

## Architecture

**Four layers:**

1. **Desktop** (`deadcode-app` + `deadcode-desktop`) — winit event loop, softbuffer+tiny-skia rendering, sprite-based units on a transparent always-on-top strip window, system tray, per-pixel hit testing, save/load, fullscreen detection
2. **Editor** (`deadcode-editor` + `editor-ui/`) — wry WebView hosting React UI, JSON IPC between Rust and JS, multi-tab script editing with CodeMirror 6, debug panel
3. **Language** (`grimscript-lang`) — lexer, Pratt parser, tree-walking interpreter with debugger (breakpoints, step over/into/out), dynamic types (int, float, string, bool, None, list, dict, tuple, entity)
4. **Simulation** (`deadcode-sim`) — deterministic tick-based engine. GrimScript compiles to stack-based IR; executor steps each unit's program counter until an action yields. 1D integer positions, no floats, seeded RNG for determinism.

### Simulation Engine (`deadcode-sim`)

```
src/
  lib.rs          — Module declarations, re-exports
  rng.rs          — SplitMix64 PRNG + Fisher-Yates shuffle (deterministic)
  value.rs        — SimValue: Int, Bool, Str, None, List, Dict(IndexMap), EntityRef (no floats)
  error.rs        — SimError types (SimErrorKind: TypeError, DivisionByZero, IndexOutOfBounds, KeyNotFound, EntityNotFound, StackUnderflow, InvalidVariable, StackOverflow, Overflow, StepLimitExceeded, Runtime)
  entity.rs       — SimEntity (unified stats HashMap, types: Vec<String>, owner: Option<EntityId>), EntityId, EntityConfig, ScriptState (incl. step_limit_hit, error recovery), CallFrame, spawn_ticks_remaining
  ir.rs           — 60+ stack-based Instruction variants, CompiledScript, FunctionEntry
  executor.rs     — Stack machine: steps IR until action/halt/error, 10k step limit per tick (warns on limit hit)
  world.rs        — SimWorld: entity storage, tick() loop (main brain + entity shuffle), event collection, snapshots, global resources, trigger processing, entity_types_registry
  action.rs       — UnitAction enum, resolve_action(), CommandDef/CommandEffect/DynInt/CompareOp/Condition/EffectOutcome/TriggerDef/TriggerFilter/EffectContext types for mod-defined commands and triggers
  query.rs        — scan(), nearest(), distance() — linear scan over entities
  compiler/       — GrimScript AST → IR compiler (feature-gated behind "compiler")
    mod.rs        — compile(), compile_source(), compile_source_with(), compile_source_full(), initial_variables()
    emit.rs       — AST walk, instruction emission, jump patching, function compilation, available command gating
    symbol_table.rs — Scope tracking, global slots vs function-local offsets
    builtins.rs   — Maps 30+ game builtins to IR instructions (queries, actions, stdlib incl. percent/scale)
    error.rs      — CompileError
```

**Feature flag:** `deadcode-sim` has an optional `compiler` feature that enables the `grimscript-lang` dependency. Without it, the sim crate is standalone (IR types + executor only). `deadcode-app` enables this feature.

**Execution model:**
- GrimScript source → lexer → parser → AST → compiler → `CompiledScript` (flat instruction vec + function table)
- Each entity has a `ScriptState` (program counter, value stack, variable slots, call stack)
- Per tick: seeded shuffle entity order, execute each until an action yields
- Queries (scan, get_health, get_resource, etc.) are instant; actions (move, attack, wait, custom mod commands) consume the tick
- Instant effects (gain_resource, try_spend_resource) return to the executor without consuming the tick — the tick loop handles mutation and pushes the return value onto the script's stack
- `self` is pre-allocated at variable slot 0 as `EntityRef` for the executing entity

**Mod dependencies:** `depends_on` and `conflicts_with` fields in `[mod]` are enforced at load time. Mods are topologically sorted by dependency graph (Kahn's algorithm, alphabetical tie-breaking). Missing deps → mod skipped with warning (cascading). Conflicts → second-loaded mod skipped. Cycles → fallback to alphabetical with error log.

**Library files:** Mods can provide `.grim` files via `commands.libraries` in `mod.toml`. Library source is loaded at mod time, concatenated across mods (in load order), and prepended to player scripts before compilation. Functions defined in libraries are available in player scripts as if defined at the top. Subject to the same command gating. Flat namespace, first-loaded-wins.

**Available commands:** Not all builtins are available from the start. Stdlib functions (`print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`, `percent`, `scale`) are always available. Game commands (queries/actions) and custom mod commands are gated by an `available_commands: Option<HashSet<String>>` passed to both the interpreter and the IR compiler. Initial set defined in `[initial].commands` in `mod.toml`. In **dev mode** (`--features dev-mode`), all commands are available (gate bypassed entirely). The frontend dynamically filters completions and syntax highlighting based on the available set + command info received via IPC.

**Custom commands:** Mods define new commands via `[[commands.definitions]]` in `mod.toml` with data-driven effects (damage, heal, spawn, modify_stat, use_resource, output, animate, list_commands, sacrifice, modify_resource, use_global_resource, if, start_channel, apply_buff, remove_buff). `modify_stat` works with all stats; `use_resource` checks and deducts any stat, aborting remaining effects if insufficient. Integer fields in effects use `DynInt`: plain integers, `"rand(min,max)"`, or computed values. These compile to `ActionCustom(name)` IR instructions. The executor yields `UnitAction::Custom { name, args }`, then effects are resolved in order against world state. The `animate` effect triggers sprite animations on target entities via `PlayAnimation` sim events. The `if` effect evaluates a condition (resource, entity_count, stat) and runs one of two effect lists (then/else), with full nesting support. The `start_channel` effect initiates a phased channel from within an effect list, enabling conditional phase branching when combined with `if`. Duplicate command names across mods are logged as warnings; first-loaded wins. See `docs/modding.md` for the full reference.

**Phased commands:** Commands can use `phases` instead of `effects` for multi-tick abilities (mutually exclusive, validated at load). Each `PhaseDef` has `ticks`, `interruptible`, `on_start`, `per_update` effect lists, and `update_interval` (default 1). `per_update` effects fire after every `update_interval` ticks within a phase (interval=1: every tick; interval=2: ticks 1,3,5; interval=3: ticks 2,5,8). On initiation, a `ChannelState` is stored on the entity. The tick loop processes channels before script execution: interruptible phases run the script and cancel if it yields a real action; non-interruptible phases skip script execution. `use_resource` failure mid-phase cancels the channel. Hot-reload clears active channels.

**Global resources:** World-level integer resources (e.g. `souls`, `gold`) stored in `SimWorld.resources: IndexMap<String, i64>`. Defined by mods via `[resources]` table in `mod.toml`, merged at load time (first-defined wins). Three builtins: `get_resource(name)` → Int (query, instant), `gain_resource(name, amount)` → Int (instant effect), `try_spend_resource(name, amount)` → Bool (instant effect). Instant effects use the `try_handle_instant()` pattern: the executor returns a `UnitAction` variant without yielding, the tick loop handles mutation and pushes the return value onto the stack before re-entering the executor. **Resource availability:** Resources have an available/unavailable mechanic mirroring commands. `[initial].resources` in `mod.toml` lists initially available resource names; if omitted, all defined resources are available. The executor checks `SimWorld.available_resources: Option<HashSet<String>>` at runtime — unavailable resources produce a runtime error. In dev mode, `available_resources` is `None` (all available).

**Buff system:** Mods define temporary stat modifiers via `[[buffs]]` in `mod.toml`. Each `BuffDef` has `name`, `duration`, `modifiers` (stat→amount), `per_tick`/`on_apply`/`on_expire` effect lists, `stackable`, and `max_stacks`. Two new effect types: `apply_buff { target, buff, duration? }` and `remove_buff { target, buff }`. Active buffs tracked per-entity via `SimEntity.active_buffs`. Modifiers directly modify stats on apply and reverse on expire. Stackable buffs accumulate stacks; non-stackable refresh duration. Buff tick processing (step 6b) runs per_tick effects, decrements durations, and handles expiry (reverse modifiers, run on_expire effects). Buff definitions stored in `SimWorld.buff_registry`.

**Multi-type entity system:** Entities have a `types: Vec<String>` field containing composable type tags for queries and filtering. The `entity_type: String` field serves as the unique entity definition ID for registry lookups (sprites, configs). `SimEntity::new()` auto-populates `types = [entity_type]` for backward compat; `SimEntity::new_with_types()` allows explicit type tags. `has_type(&self, tag) -> bool` checks membership. Query functions `scan()` and `nearest()` filter by `has_type()` instead of exact match. New GrimScript builtins: `get_types(entity) -> List`, `has_type(entity, name) -> Bool`. Entity attribute `entity.types` returns the type list. `DynInt::EntityCount`, `Condition::EntityCount`, `Sacrifice` effects, and trigger filters all use `has_type()` for matching. `SimWorld.entity_types_registry` stores def ID → types mapping for spawn effects.

**Type definitions (`[[types]]` in mod.toml):** Types are composable tags with optional stats, commands, and brain scripts. Each `TypeDef` has `name`, `brain: bool`, `stats: IndexMap`, `commands: Vec<String>`, and optional `script` path. Entity `[[entities]]` definitions reference types via `types = ["undead", "melee"]`. Stats are merged in type order, then entity-level stats override. Type `.gs` scripts loaded from `grimscript/` directory. Entity definitions use `id` field (unique key) with `type` as backward-compat fallback; `types` defaults to `[id]` if absent.

**Unified entity stats:** All entity stats live in a single `SimEntity.stats: IndexMap<String, i64>` (deterministic iteration order), accessed via `stat(&self, name) -> i64` (returns 0 if unset), `set_stat(&mut self, name, value)`, and `clamp_stat(&mut self, name)` (clamps to `[0, max_{name}]` if a max exists). There are **no built-in default stats** — entities start with an empty stats map; all stats come from `EntityConfig` applied by mods. `EntityConfig` contains `stats: IndexMap<String, i64>`; `apply_config()` auto-sets `max_health`/`max_shield` when health/shield are defined without explicit max values. In `mod.toml`, entity stats merge from types (in type order) then entity-level overrides. `modify_stat` and `use_resource` effects work with all stats generically. Stats are accessible via entity attribute access (`entity.armor`) and the `get_stat(entity, "name")` GrimScript builtin (query, instant, gated by available_commands; `get_custom_stat` kept as alias).

**Computed values (DynInt):** `DynInt` extended with three game-state variants: `EntityCount { entity_type, multiplier }` resolves to the count of alive entities of a type, `ResourceValue { resource, multiplier }` resolves to a global resource's current value, `CasterStat { stat, multiplier }` resolves to the caster's stat value. TOML format: `"entity_count(skeleton)"`, `"resource(mana)"`, `"stat(health)"`, with optional multiplier `"entity_count(skeleton)*2"`. All effect `amount`/`offset` fields use `resolve_with_world()` for game-state access. Backward compatible — plain integers and `"rand(min,max)"` continue to work.

**Extended conditions:** `Condition::Stat` handles all stats generically. Plus: `has_buff { buff }` checks if the caster has an active buff, `random_chance { percent }` uses deterministic RNG (roll < percent), `and { conditions }` requires all sub-conditions true, `or { conditions }` requires at least one true. `is_alive { target }` resolves a target entity via `resolve_target_from_args()` and returns true if it exists and is alive (false if unresolvable). `distance { target, compare, amount }` computes absolute integer distance between caster and target positions and compares via `CompareOp` + `DynInt` amount (false if unresolvable). Both new conditions accept all scoped target strings (`"self"`, `"arg:name"`, `"source"`, `"owner"`, `"attacker"`, `"killer"`). Compound conditions support nesting. `evaluate_condition_with_ctx(caster, world, args, ctx)` is the canonical evaluator; `evaluate_condition(caster, world)` is a backward-compatible wrapper passing empty args and default context. `resolve_effects_inner()` calls `evaluate_condition_with_ctx()` so scoped targets work inside `if` condition fields. Serde alias `custom_stat` → `stat` for backward compat.

**Event triggers:** Mods define reactive rules via `[[triggers]]` in `mod.toml`. Each `TriggerDef` has an `event` (entity_died, entity_spawned, entity_damaged, resource_changed, command_used, tick_interval, channel_completed, channel_interrupted), a `TriggerFilter` (entity_type, resource, command, interval), optional `conditions` (reuse `Condition` enum), and `effects` (reuse `CommandEffect`). Triggers are registered on `SimWorld` at load time. Processing occurs at the end of each tick (step 8): events from the tick are matched against triggers, filters narrow matches, conditions gate firing, effects resolve against the first alive entity (summoner). Trigger effects do not re-trigger within the same tick. `resource_changed` uses snapshot comparison (resources at tick start vs current). `tick_interval` fires when `tick % interval == 0`. Validated at load time by `validate_triggers()`.

**Scoped targets (trigger effects):** Trigger effect resolution passes an `EffectContext` struct through `resolve_effects_inner()` that carries event-participant IDs. Four scoped target strings are available in trigger effect `target` fields: `"source"` (event subject), `"owner"` (source entity's owner, resolved via `SimEntity.owner: Option<EntityId>` with fallback to entity field), `"attacker"` (damage dealer, available in `entity_damaged` triggers), `"killer"` (killing-blow dealer, available in `entity_died` triggers). `resolve_target_from_args()` checks context first; unresolvable scoped targets silently no-op the effect. `SimEvent` variants enriched: `EntityDamaged` carries `attacker_id`, `EntityDied` carries `killer_id`/`owner_id`, `EntitySpawned` carries `spawner_id`. `get_owner()` returns `EntityRef` or `None` (previously `Int(0)`). Scoped targets are accepted by `validate_target()` in trigger contexts.

**Unified execution:** The sim runs continuously from game open. Entities with brain types get compiled scripts assigned at startup via `compile_and_assign_all_brains()`. Saving a type `.gs` file triggers auto-reload: recompiles and hot-swaps all affected entities' `ScriptState` (full reset: PC, stack, variables discarded; entity keeps position/health/world state). The main brain (`main.gs`) runs entity-less before the entity shuffle each tick. Terminal commands execute against the main brain / first alive entity. The `handle_run_script_sim` path still exists for legacy `SummonerBrain` scripts.

**Main brain:** Entity-less script stored as `SimWorld.main_brain: Option<ScriptState>`. Runs first every tick (step 2c, before entity shuffle) using sentinel `EntityId(0)`. Can call resource ops, queries, print, custom commands. Cannot perform entity actions (move, attack, flee — silently ignored). Terminal commands execute against the main brain.

**Error recovery:** When a script hits a runtime error, it stores the error on `ScriptState.error`. On the next tick, error recovery kicks in: the error is cleared, script state is fully reset (PC=0, stack/call stack cleared, variables re-initialized with slot 0 = self EntityRef), the entity yields `wait()` for that tick, and a `[error recovery]` message is emitted. The script re-executes from the beginning on the following tick. This prevents permanent script death from transient errors. Applies to entity scripts, main brain, and channel interruptible scripts.

**Command gating (two layers):** A command must pass both gates: (1) Global unlock (`[initial].commands`) — progression gate; (2) Type capability (`commands` on `[[types]]`) — per-entity capability gate. An entity's effective commands = union of its types' `commands` ∩ globally unlocked. If no types define commands, all globally unlocked commands are available (backward compat). In dev mode, all commands available (gates bypassed). Computed via `modding::compute_effective_commands()`.

**Auto-reload on save:** Saving a type `.gs` file in the editor triggers `handle_type_script_reload()`: brain type changed → recompile all entities with that brain; non-brain type changed → recompile all entities including that type (library changed); `main.gs` changed → recompile main brain. Script composition per entity: non-brain type `.gs` (library) + mod libraries + brain `.gs` (execution logic).

**Spawn state:** Dynamically spawned entities (from effects) have `spawn_ticks_remaining > 0` — they play their spawn animation and can't act or be targeted by queries until the timer reaches 0. Duration is computed from the entity type's atlas JSON spawn animation. Initial mod spawns start ready (`spawn_ticks_remaining = 0`).

**Death lifecycle:** When an entity dies (`alive = false`), a `SimEvent::EntityDied { entity_id, name, killer_id, owner_id }` is emitted. The sim removes the entity at end-of-tick via `flush_pending()`. The game loop handles the event by calling `UnitManager::kill(uid)`, which plays the "death" animation (if the atlas has one) and marks the unit `pending_destroy`. Units with `pending_destroy` are reaped by `reap_dead()` after their death animation finishes. If no death animation exists, the unit is destroyed immediately.

**Fixed timestep:** The sim runs at exactly 30 TPS via an accumulator in `do_tick()`. Wall-clock delta is accumulated; `sim.tick()` fires once per 33ms. Capped at 4 sim ticks per frame to prevent spiral of death. Animations are sim-driven (advanced once per sim tick via `UnitManager::tick_animations()`), movement interpolation remains render-driven.

**Tick loop** (`SimWorld::tick()`):
1. Derive per-tick RNG: `SimRng::new(seed ^ tick)`. Snapshot entity types and resource values for trigger processing.
2. Decrement spawn timers on spawning entities
2c. Execute main brain (entity-less, sentinel EntityId(0), instant actions only)
3. Shuffle ready entity IDs (excludes spawning entities, includes entities with active channels)
4. For each: check error recovery (if script errored last tick, reset and yield wait), then process active channel if present (phase effects, interruption check), otherwise take script state out, execute, handle instant actions via `try_handle_instant()` (Print, resource ops — re-enter executor), collect tick-consuming action, put state back
5. Resolve all actions against world state
6. Tick passive systems (cooldowns, behavior cooldowns)
6b. Tick buffs: run per_tick effects, decrement durations, handle expiry (reverse modifiers, fire on_expire)
7. Flush pending spawns/despawns
8. Process triggers: match events collected during the tick against registered triggers, check filters and conditions, fire effects. Trigger effects do not re-trigger within the same tick.

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
2. Simulation tick (fixed 30 TPS via accumulator) → animations tick → reap dead units → snapshot → sync positions to UnitManager → forward events (spawn, death, output, animation) to editor
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
- The summoner is defined by the core mod (`mods/core/mod.toml`) like any other entity — entity type, stats, sprite, and pivot are in the mod manifest; initial spawning is handled via `[initial].effects` with a `spawn` effect. If no mods are found, nothing loads (no fallback). Script execution methods find the summoner by entity type `"summoner"`.
- Theme-agnostic sim: no baked-in entity type constants, entity types are runtime strings

## Common Tasks

| Task | Where |
|------|-------|
| Add language builtin | `crates/grimscript-lang/src/builtins.rs` |
| Add sim IR instruction | `crates/deadcode-sim/src/ir.rs` + `executor.rs` |
| Add sim builtin mapping | `crates/deadcode-sim/src/compiler/builtins.rs` |
| Add sim query | `crates/deadcode-sim/src/query.rs` + `ir.rs` (QueryXxx) + `executor.rs` |
| Add sim action | `crates/deadcode-sim/src/action.rs` + `ir.rs` (ActionXxx) + `executor.rs` |
| Add IPC message | `crates/deadcode-editor/src/ipc.rs` + `editor-ui/src/ipc/types.ts` + `bridge.ts` |
| Add unit/sprite | `crates/deadcode-desktop/src/unit.rs` + atlas in `src/assets/` |
| Game loop logic | `crates/deadcode-app/src/app.rs` → `do_tick()` |
| Rendering | `crates/deadcode-desktop/src/renderer.rs` |
| Editor UI components | `editor-ui/src/components/` |
| Editor state | `editor-ui/src/state/` (zustand) |
| Script storage | `crates/deadcode-editor/src/scripts.rs` |
| Save/load | `crates/deadcode-desktop/src/save.rs` |
| Unlock a game command | `crates/deadcode-app/src/app.rs` → `available_commands` set |
| Gate a new game builtin | Add to `is_builtin()`, `is_game_builtin()` returns true, `is_stdlib()` returns false |
| Add custom mod command | `mods/<mod>/mod.toml` → `[[commands.definitions]]` with name, args, effects or phases |
| Add phased mod command | `mods/<mod>/mod.toml` → `[[commands.definitions]]` with name, args, phases (see `docs/modding.md`) |
| Add new effect type | `crates/deadcode-sim/src/action.rs` → `CommandEffect` enum + handler in `resolve_effects_inner()` |
| Add conditional effect logic | `crates/deadcode-sim/src/action.rs` → `Condition` enum + `evaluate_condition_with_ctx()` (target-bearing conditions need args/ctx); update `evaluate_condition()` wrapper if needed |
| Add instant effect builtin | `ir.rs` (InstantXxx) + `executor.rs` + `action.rs` (UnitAction) + `builtins.rs` (InstantEffectBuiltin) + `world.rs` (try_handle_instant) |
| Define mod resources | `mods/<mod>/mod.toml` → `[resources]` table (name = initial_value), `[initial] resources` list for availability |
| Define buff | `mods/<mod>/mod.toml` → `[[buffs]]` with name, duration, modifiers, per_tick/on_apply/on_expire effects |
| Add event trigger | `mods/<mod>/mod.toml` → `[[triggers]]` with event, filter, conditions, effects |
| Add trigger event type | `crates/deadcode-sim/src/world.rs` → `SimEvent` enum + emit in tick loop + match in `process_triggers()` |
| Add scoped target to new trigger event | Add participant IDs to `SimEvent` variant + populate `EffectContext` fields in `process_triggers()` match arm in `world.rs` |
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
