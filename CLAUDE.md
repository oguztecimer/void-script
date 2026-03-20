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
cargo test                          # All Rust tests (132 tests)
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
  error.rs        — SimError types
  entity.rs       — SimEntity, EntityId, EntityType, ScriptState (incl. step_limit_hit), CallFrame, spawn_ticks_remaining
  ir.rs           — 60+ stack-based Instruction variants, CompiledScript, FunctionEntry
  executor.rs     — Stack machine: steps IR until action/halt/error, 10k step limit per tick (warns on limit hit)
  world.rs        — SimWorld: entity storage, tick() loop, event collection, snapshots, global resources
  action.rs       — UnitAction enum, resolve_action(), CommandDef/CommandEffect/DynInt types for mod-defined commands
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

**Available commands:** Not all builtins are available from the start. Stdlib functions (`print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`, `percent`, `scale`) are always available. Game commands (queries/actions) and custom mod commands are gated by an `available_commands: Option<HashSet<String>>` passed to both the interpreter and the IR compiler. Initial set defined in `[initial].commands` in `mod.toml`. In **dev mode** (`--features dev-mode`), all commands are available (gate bypassed entirely). The frontend dynamically filters completions and syntax highlighting based on the available set + command info received via IPC.

**Custom commands:** Mods define new commands via `[[commands.definitions]]` in `mod.toml` with data-driven effects (damage, heal, spawn, modify_stat, use_resource, output, animate, list_commands, sacrifice). Integer fields in effects use `DynInt`: plain integers or `"rand(min,max)"` for deterministic randomness. These compile to `ActionCustom(name)` IR instructions. The executor yields `UnitAction::Custom { name, args }`, then effects are resolved in order against world state. The `use_resource` effect checks and deducts a resource, aborting remaining effects if insufficient. The `animate` effect triggers sprite animations on target entities via `PlayAnimation` sim events. Duplicate command names across mods are logged as warnings; first-loaded wins. See `docs/modding.md` for the full reference.

**Phased commands:** Commands can use `phases` instead of `effects` for multi-tick abilities (mutually exclusive, validated at load). Each `PhaseDef` has `ticks`, `interruptible`, `on_start`, and `per_tick` effect lists. On initiation, a `ChannelState` is stored on the entity. The tick loop processes channels before script execution: interruptible phases run the script and cancel if it yields a real action; non-interruptible phases skip script execution. `use_resource` failure mid-phase cancels the channel. Hot-reload clears active channels.

**Global resources:** World-level integer resources (e.g. `souls`, `gold`) stored in `SimWorld.resources: IndexMap<String, i64>`. Defined by mods via `[resources]` table in `mod.toml`, merged at load time (first-defined wins). Three builtins: `get_resource(name)` → Int (query, instant), `gain_resource(name, amount)` → Int (instant effect), `try_spend_resource(name, amount)` → Bool (instant effect). Instant effects use the `try_handle_instant()` pattern: the executor returns a `UnitAction` variant without yielding, the tick loop handles mutation and pushes the return value onto the stack before re-entering the executor. **Resource availability:** Resources have an available/unavailable mechanic mirroring commands. `[initial].resources` in `mod.toml` lists initially available resource names; if omitted, all defined resources are available. The executor checks `SimWorld.available_resources: Option<HashSet<String>>` at runtime — unavailable resources produce a runtime error. In dev mode, `available_resources` is `None` (all available).

**Unified execution:** The sim runs continuously from game open. Run/Debug compiles GrimScript to IR and hot-swaps the summoner's `ScriptState` (full reset: PC, stack, variables discarded; entity keeps position/health/world state). A `[reload] Script recompiled and loaded` console message is emitted on successful hot-swap. The interpreter path is only used for terminal one-liners.

**Spawn state:** Dynamically spawned entities (from effects) have `spawn_ticks_remaining > 0` — they play their spawn animation and can't act or be targeted by queries until the timer reaches 0. Duration is computed from the entity type's atlas JSON spawn animation. Initial mod spawns start ready (`spawn_ticks_remaining = 0`).

**Death lifecycle:** When an entity dies (`alive = false`), a `SimEvent::EntityDied { entity_id, name }` is emitted. The sim removes the entity at end-of-tick via `flush_pending()`. The game loop handles the event by calling `UnitManager::kill(uid)`, which plays the "death" animation (if the atlas has one) and marks the unit `pending_destroy`. Units with `pending_destroy` are reaped by `reap_dead()` after their death animation finishes. If no death animation exists, the unit is destroyed immediately.

**Fixed timestep:** The sim runs at exactly 30 TPS via an accumulator in `do_tick()`. Wall-clock delta is accumulated; `sim.tick()` fires once per 33ms. Capped at 4 sim ticks per frame to prevent spiral of death. Animations are sim-driven (advanced once per sim tick via `UnitManager::tick_animations()`), movement interpolation remains render-driven.

**Tick loop** (`SimWorld::tick()`):
1. Derive per-tick RNG: `SimRng::new(seed ^ tick)`
2. Decrement spawn timers on spawning entities
3. Shuffle ready entity IDs (excludes spawning entities, includes entities with active channels)
4. For each: process active channel if present (phase effects, interruption check), otherwise take script state out, execute, handle instant actions via `try_handle_instant()` (Print, resource ops — re-enter executor), collect tick-consuming action, put state back
5. Resolve all actions against world state
6. Tick passive systems (cooldowns)
7. Flush pending spawns/despawns

### IPC

JSON messages as serde-tagged enums in `deadcode-editor/src/ipc.rs` (`JsToRust`/`RustToJs`). JS side types in `editor-ui/src/ipc/types.ts`, handler in `editor-ui/src/ipc/bridge.ts`.

Message categories:
- **Script ops:** ScriptSave, ScriptRequest, ScriptList, RunScript, StopScript
- **Debug:** DebugStart, DebugContinue, StepOver/Into/Out, ToggleBreakpoint, DebugPaused/Resumed
- **Simulation:** StartSimulation, StopSimulation, PauseSimulation → SimulationStarted, SimulationStopped, SimulationTick
- **Window:** Minimize, Maximize, Close, DragStart, ResizeStart, Shake, SetSize
- **Console:** ConsoleOutput, ConsoleCommand
- **Game state:** AvailableCommands (Rust→JS, sent on EditorReady; includes commands, resources, command_info, dev_mode)

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
- Compiler is feature-gated: `deadcode-sim` stays independent without `grimscript-lang`
- The summoner is a hardcoded core entity — always spawned by `app.rs` at position 500 using embedded assets, not defined by mods
- Theme-agnostic sim: no baked-in entity type constants, entity types are runtime strings (except summoner, hardcoded in `app.rs`)

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
| Add new effect type | `crates/deadcode-sim/src/action.rs` → `CommandEffect` enum + handler in `resolve_custom_effects()` |
| Add instant effect builtin | `ir.rs` (InstantXxx) + `executor.rs` + `action.rs` (UnitAction) + `builtins.rs` (InstantEffectBuiltin) + `world.rs` (try_handle_instant) |
| Define mod resources | `mods/<mod>/mod.toml` → `[resources]` table (name = initial_value), `[initial] resources` list for availability |

## Documentation Maintenance

Every change to the project must be reflected in the related `.md` files. When making code changes:

- **`CHANGELOG.md`** — Always update. Add an entry under the appropriate section (`### Simulation Engine`, `### Modding System`, etc.) describing what changed, why, and under which item ID (S-xx, M-xx).
- **`CLAUDE.md`** — Update if the change affects architecture, conventions, types, the common tasks table, or introduces new patterns. Keep descriptions concise and accurate to current code.
- **`docs/modding.md`** — Update if the change affects mod.toml schema, loading/validation behavior, custom command flow, or any modder-facing API.
- **`TODO.md`** — Update if items are resolved or partially resolved, or if new issues are discovered.
- **`bugs&issues.md`** — Add entries for newly discovered bugs. Update status when bugs are fixed.

When in doubt, update the doc. Stale documentation is worse than missing documentation because it misleads.
