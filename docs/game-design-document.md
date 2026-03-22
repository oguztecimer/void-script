# VOID//SCRIPT — Game Design Document

## Elevator Pitch

**VOID//SCRIPT** is a necromancer-themed desktop automation game where you write code to command the undead. The game lives on your desktop — a transparent strip above your taskbar where pixel-art minions roam while you write GrimScript in a built-in IDE.

---

## Genre & Platform

- **Genre:** Programming game / desktop automation
- **Platforms:** Windows, macOS, Linux
- **Engine:** Custom Rust + React hybrid (winit, wry, softbuffer, tiny-skia)

---

## Core Concept

Players take on the role of a necromancer who controls minions by writing scripts in **GrimScript**, a custom Python-like language. The game renders on a transparent always-on-top strip above the system taskbar/dock, blending into the desktop environment. An integrated code editor (CodeMirror 6 in a WebView) provides the scripting interface with syntax highlighting, autocomplete, and a debug panel.

The simulation is fully deterministic — same code produces the same result every time, enabling reproducible experimentation and strategic optimization.

---

## Target Audience

- Programmers looking for a creative coding sandbox
- Fans of programming/automation games (Zachtronics, Screeps, Shenzhen IO, Bitburner)
- Players who enjoy incremental progression and optimization loops
- Dark fantasy / necromancer aesthetic enthusiasts

---

## Unique Selling Points

1. **Desktop integration** — The game renders transparently above your taskbar. Units walk across your screen while you work.
2. **Custom scripting language** — GrimScript is Python-like and designed for approachability. No boilerplate, no imports — just logic.
3. **Deterministic simulation** — Fixed 30 TPS tick-based engine with seeded RNG. Same script, same outcome.
4. **Fully moddable** — TOML manifests define entities, types, resources, and buff stats; Lua scripts define commands, triggers, buff callbacks, and initialization. No Rust knowledge needed to create content.
5. **Necromancer theme** — Dark, atmospheric pixel art. Raise skeletons, harvest bones, channel forbidden spells.
6. **Built-in IDE** — Integrated editor with syntax highlighting, autocompletion, breakpoint debugging, and a console.

---

## Gameplay Loop

1. **Write** — Open the editor and write GrimScript to control your summoner and minions
2. **Execute** — Watch your scripts run in real time on the desktop strip
3. **Observe** — Units move, attack, cast spells, and interact based on your code
4. **Iterate** — Modify scripts, experiment with different strategies, optimize resource usage
5. **Unlock** — Gain access to new commands and resources as you progress
6. **Expand** — Summon more minions, each running their own soul scripts autonomously

### Resource Management

- **Mana** — Primary resource (starts at 50, max 100). Regenerated via `trance()`, spent to summon and cast.
- **Bones** — Secondary resource (starts at 0, uncapped). Harvested from minions, used for advanced abilities.

---

## Core Mechanics

### GrimScript

A custom Python-like language with:
- Variables, functions, loops, conditionals
- Dynamic types: int, string, bool, None, list, dict, entity references
- Python-style floor division and modulo semantics
- No imports or boilerplate — scripts execute from the first line
- Soul scripts implicitly loop (restart from the top each tick when they halt)

### Command System

Commands are the verbs of the game — everything a script can do:

- **Stdlib** (always available, don't consume a tick): `echo()`, `len()`, `range()`, `abs()`, `min()`, `max()`, `percent()`, `scale()`
- **Custom commands** (mod-defined in Lua, consume one tick): `trance()`, `raise()`, `harvest()`, `pact()`, `help()`
- **Multi-tick commands** (Lua coroutines with `ctx:yield_ticks()`): Commands can span multiple ticks with interruptible/non-interruptible segments and resource gates

All game commands are defined in Lua via `mod.command()`. There are no hardcoded game builtins — the sim compiles all non-stdlib commands to `ActionCustom` and dispatches them to the Lua runtime.

### Entity System

- **Summoner** — The player's main entity (100 HP). Runs the player's scripts.
- **Minions** — Spawned entities (e.g., skeletons with 5 HP) that run autonomous soul scripts.
- **Composable types** — Entities are composed of type tags (e.g., `["unit", "skeleton"]`). Types provide stats, commands, and soul scripts. Stats merge in type order with entity-level overrides.

### Soul Scripts

Each entity type can have a `.gs` soul script that runs autonomously:
- Soul scripts implicitly loop — no `while True:` needed
- `self` refers to the executing entity
- Scripts are hot-reloaded on save — edit and see results instantly
- Error recovery: if a script crashes, it resets and retries next tick

### Buff System

Temporary stat modifiers with:
- Duration, stackability, max stacks
- On-apply, per-tick, and on-expire effect lists
- Stat modifiers that reverse on expiry

### Trigger System

Event-driven reactive rules:
- Events: entity_died, entity_spawned, entity_damaged, command_used, channel_completed, channel_interrupted
- Filters narrow which events match
- Conditions gate trigger firing
- Effects resolve against game state

### Command Gating

Type-based capability system:
- **Type capability** — Entity types define which commands their entities can use via `commands` on `[[types]]` in `mod.toml`
- An entity's effective commands = union of all its types' `commands` lists
- If no types define `commands`, all commands are available (backward compat)
- In dev mode, all commands are available (gate bypassed)

---

## Visual Style

- **Desktop overlay** — Transparent always-on-top window strip above the taskbar/dock
- **Pixel art** — Hand-crafted sprite atlases with idle, walk, attack, cast, spawn, and death animations
- **Dark IDE aesthetic** — The editor uses a dark theme matching the necromancer motif
- **Minimal UI** — The game gets out of your way. Units on the strip, editor when you need it, system tray icon for control.
- **Sim-driven animations** — All animation timing is tied to sim ticks (30 TPS), not wall clock, ensuring consistency

---

## Technical Overview

| Aspect | Detail |
|--------|--------|
| Language | Rust (core) + TypeScript/React (editor UI) |
| Rendering | CPU-based: softbuffer + tiny-skia on a transparent winit window |
| WebView | wry (Chromium/WebKit) for the editor |
| Simulation | Deterministic 30 TPS, integer-only math, seeded PRNG (SplitMix64) |
| Scripting | GrimScript: lexer → parser → AST → compiler → stack-based IR → executor |
| IPC | JSON messages over WebView bridge (serde-tagged enums) |
| Rendering rate | 30 FPS active, 10 FPS idle |
| State | zustand (frontend), crossbeam-channel (Rust IPC) |

---

## Modding System

The game content layer is split between TOML data declarations and Lua behavior scripts:

**TOML (`mod.toml`) — data declarations:**
- **Types** — Composable type tags with stats, commands, and soul scripts
- **Entities** — Define new entity types with stats, sprites, and composable type tags
- **Resources** — World-level integer resources with availability gating
- **Buffs** — Temporary stat modifier definitions (duration, modifiers, stacking)
- **Libraries** — Shared `.grim` files automatically prepended to player scripts
- **Dependencies** — Mods declare dependencies and conflicts; load order is topologically sorted

**Lua (`mod.lua`) — behavior:**
- **Commands** — Custom commands with full programmatic control (damage, heal, spawn, resource costs, multi-tick coroutines)
- **Triggers** — Event-driven reactive rules (entity_died, entity_spawned, etc.)
- **Buff callbacks** — on_apply, per_tick, on_expire lifecycle hooks
- **Initialization** — Spawn entities, set up initial state at game start

No Rust or TypeScript knowledge required to create mods — only TOML and Lua. See `docs/modding.md` for the full reference.

---

## Current State

**Version:** 0.1.0 (Unreleased)

### Complete

- Custom scripting language (GrimScript) with compiler and tree-walking interpreter
- Deterministic simulation engine with stack-based IR executor
- Desktop strip rendering with transparent overlay
- Integrated code editor with syntax highlighting, autocomplete, debugging
- Entity system with composable types and soul scripts
- Resource management (mana, bones)
- Custom command system with phased abilities
- Buff/debuff system
- Event trigger system
- Full modding framework
- Save/load system
- Cross-platform support (Windows, macOS, Linux)
- Hot-reload on script save

### Planned

- Content expansion (more entity types, commands, progression)
- Data-driven entity behaviors as an alternative to soul scripts
- Additional game mechanics and win conditions
- Steam/itch.io distribution

---

## Comparables

| Game | Shared DNA | VOID//SCRIPT differentiator |
|------|-----------|----------------------------|
| **Screeps** | Write code to control units in a persistent world | Desktop-native, single-player, custom language |
| **Zachtronics games** (TIS-100, Shenzhen IO) | Programming puzzles with constrained languages | Real-time simulation, persistent desktop presence |
| **Hacknet** | Hacking-themed terminal game | Necromancer theme, custom language, desktop overlay |
| **while True: learn()** | Programming-adjacent puzzle game | Actual code writing, not visual programming |
| **Bitburner** | JavaScript-based hacking idle game | Custom language, desktop integration, pixel art aesthetic |
