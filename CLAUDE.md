# CLAUDE.md — VOID//SCRIPT

## Project Overview

Desktop automation game with a necromancer theme. Players write GrimScript (a custom Python-like language) to control units on a 1D strip rendered above the taskbar/dock. Scripts compile to a deterministic stack-based IR and execute in a tick-based simulation engine.

Hybrid Rust + TypeScript/React desktop app targeting Windows, macOS, and Linux.

## Tech Stack

- **Core:** Rust (edition 2024), Cargo workspace
- **UI:** React 19, TypeScript 5.7, Vite 6, CodeMirror 6
- **Desktop:** winit 0.30 (windowing), wry 0.50 (webview), softbuffer 0.4 + tiny-skia 0.12 (2D CPU rendering)
- **State:** zustand (frontend), crossbeam-channel (Rust IPC)
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
cargo test                          # All Rust tests (67 tests)
cargo test -p deadcode-sim          # Sim engine + compiler tests
cargo test -p grimscript-lang       # Language crate only
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
  value.rs        — SimValue: Int, Bool, Str, None, List, Dict, EntityRef (no floats)
  error.rs        — SimError types
  entity.rs       — SimEntity, EntityId, EntityType, ScriptState, CallFrame
  ir.rs           — 50+ stack-based Instruction variants, CompiledScript, FunctionEntry
  executor.rs     — Stack machine: steps IR until action/halt/error, 10k step limit per tick
  world.rs        — SimWorld: entity storage, tick() loop, event collection, snapshots
  action.rs       — UnitAction enum, resolve_action() against world state
  query.rs        — scan(), nearest(), distance() — linear scan over entities
  compiler/       — GrimScript AST → IR compiler (feature-gated behind "compiler")
    mod.rs        — compile(), compile_source(), initial_variables()
    emit.rs       — AST walk, instruction emission, jump patching, function compilation
    symbol_table.rs — Scope tracking, global slots vs function-local offsets
    builtins.rs   — Maps 30+ game builtins to IR instructions
    error.rs      — CompileError
```

**Feature flag:** `deadcode-sim` has an optional `compiler` feature that enables the `grimscript-lang` dependency. Without it, the sim crate is standalone (IR types + executor only). `deadcode-app` enables this feature.

**Execution model:**
- GrimScript source → lexer → parser → AST → compiler → `CompiledScript` (flat instruction vec + function table)
- Each entity has a `ScriptState` (program counter, value stack, variable slots, call stack)
- Per tick: seeded shuffle entity order, execute each until an action yields
- Queries (scan, get_health, etc.) are instant; actions (move, attack, wait) consume the tick
- `self` is pre-allocated at variable slot 0 as `EntityRef` for the executing entity

**Tick loop** (`SimWorld::tick()`):
1. Derive per-tick RNG: `SimRng::new(seed ^ tick)`
2. Shuffle scriptable entity IDs
3. For each: take script state out, execute, collect action, put state back
4. Resolve all actions against world state
5. Tick passive systems (cooldowns)
6. Flush pending spawns/despawns

### IPC

JSON messages as serde-tagged enums in `deadcode-editor/src/ipc.rs` (`JsToRust`/`RustToJs`). JS side types in `editor-ui/src/ipc/types.ts`, handler in `editor-ui/src/ipc/bridge.ts`.

Message categories:
- **Script ops:** ScriptSave, ScriptRequest, ScriptList, RunScript, StopScript
- **Debug:** DebugStart, DebugContinue, StepOver/Into/Out, ToggleBreakpoint, DebugPaused/Resumed
- **Simulation:** StartSimulation, StopSimulation, PauseSimulation → SimulationStarted, SimulationStopped, SimulationTick
- **Window:** Minimize, Maximize, Close, DragStart, ResizeStart, Shake, SetSize
- **Console:** ConsoleOutput, ConsoleCommand

### Game Loop

`App::do_tick()` in `deadcode-app/src/app.rs`:
1. Unit system tick (animations, movement)
2. Simulation tick (if running) → snapshot → sync to UnitManager → forward events to editor
3. Auto-save timer
4. Fullscreen detection
5. Per-pixel hit testing
6. Editor IPC polling
7. Script execution polling

30 FPS active / 10 FPS idle.

## Key Conventions

- Workspace deps in root `Cargo.toml`, referenced with `{ workspace = true }`
- Platform code uses `#[cfg(target_os = "...")]`
- Editor UI uses CSS modules
- IPC enums use `#[serde(tag = "type")]`
- Sprite atlases are JSON + PNG pairs in `deadcode-desktop/src/assets/`
- Simulation uses only `i64` (no floats) for determinism; `Dict` uses `Vec<(K,V)>` for deterministic iteration
- Compiler is feature-gated: `deadcode-sim` stays independent without `grimscript-lang`
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
