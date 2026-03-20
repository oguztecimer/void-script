# Changelog

## [Unreleased]

### Modding System

#### Added
- **M-11: `sacrifice` effect type** — New `CommandEffect::Sacrifice` variant that kills all alive, non-spawning entities of a given type and gains a resource per kill. Fields: `entity_type` (which entities to kill), `resource` (which global resource to gain), `per_kill` (DynInt amount gained per kill, supports `rand(min,max)`). Outputs a summary message or "Nothing to sacrifice" if no matching entities exist. The `harvest` command in `mods/core/mod.toml` now uses this effect to sacrifice skeletons for 1-2 bones each, replacing the old +20 energy placeholder. Entity type references in sacrifice effects are validated at mod load time alongside spawn effects.

### Desktop / Rendering

#### Fixed
- **Entity death rendering cleanup** — `SimEvent::EntityDied` is now handled in the game loop. Dead entities play their "death" animation (if the atlas has one) and are removed from rendering once it finishes. Previously, `EntityDied` was unhandled — dead entities' render units lingered in `UnitManager` forever as invisible orphans. `SimEvent::EntityDied` now carries the entity `name` so the handler can find the matching render unit after the sim has already removed the entity. `UnitManager` gained `kill(id)` (play death + mark pending_destroy) and `reap_dead()` (remove units whose death animation finished).
- **Fixed duplicate render units from Spawn effect** — The `Spawn` command effect was emitting an `EntitySpawned` event directly, but `flush_pending()` in the tick loop already emits `EntitySpawned` when the queued entity is actually added. This created two render units per spawn, making death cleanup appear to do nothing (one unit was killed but the duplicate remained). Removed the premature event from the effect handler.
- **Unique entity naming for runtime spawns** — Runtime-spawned entities (from `Spawn` effects) now use `"{type}_{entity_id}"` instead of `"{type}_{position}"` as their name. The old scheme could produce duplicates when entities spawned at the same position (e.g. calling `raise()` twice without moving), causing name collisions in position sync, death handling, and animation targeting. Entity ID is a monotonically increasing u64, guaranteed unique. `SimEvent::EntitySpawned` now carries the entity `name` directly so app.rs doesn't reconstruct it.
- **Unified sim event handling for console and script paths** — Console commands (`handle_console_command_sim`) and the sim tick loop (`do_tick`) now share the same `forward_sim_event_to_editor()` and `apply_sim_event_to_units()` methods. Previously, console commands had a separate `apply_sim_event_to_units` that only handled `PlayAnimation`, missing `EntitySpawned` and `EntityDied` events. This caused `harvest()` typed in the console to kill skeletons in the sim but not update the render units.

### Core

#### Changed
- **Summoner is now a hardcoded core entity** — The summoner is no longer defined or spawned via `mod.toml`. It is always created by the game engine at position 500 with fixed stats (100 HP, 100 energy, speed 1) using embedded sprite assets. Mods cannot override or redefine the summoner entity type. This ensures the player's core entity is always present and consistent regardless of mod configuration.

### Simulation Engine

#### Added
- **S-10: Global resource system** — New world-level integer resources (e.g. souls, gold) shared across all entities. Three new GrimScript builtins: `get_resource(name)` returns the current value (0 if undefined), `gain_resource(name, amount)` adds to a resource and returns the new total, `try_spend_resource(name, amount)` atomically checks and deducts, returning true/false. Resources are defined by mods in `mod.toml` via a `[resources]` table and merged at load time (first-defined wins for duplicates). Mutating operations use the "instant action" pattern — the executor yields without consuming the tick, the tick loop handles mutation and pushes the return value onto the script's stack before re-entering. The tick loop's instant action handling has been refactored into `SimWorld::try_handle_instant()`, replacing the previous inline Print handling in both normal and interruptible-channel paths.
- **S-11: Resource availability gating** — Global resources now have an available/unavailable mechanic mirroring the command availability system. Mods define initially available resources via `[initial] resources = [...]` in `mod.toml`. If omitted, all defined resources are available by default. At runtime, calling `get_resource()`, `gain_resource()`, or `try_spend_resource()` on an unavailable resource produces a runtime error ("resource 'X' is not available yet"). In dev mode, all resources are available (gate bypassed). Available resource names are sent to the frontend via IPC alongside available commands. All initial-state config (commands, resources, effects) is now unified under the `[initial]` section in `mod.toml`.

#### Fixed
- **S-05: Step limit auto-yield now warns the player** — When a script exceeds the 10,000 instruction limit per tick, a warning message is emitted to the editor console instead of silently auto-yielding with a wait action. The warning reads: "[warning] Script exceeded step limit (10000 instructions) — auto-yielded".

#### Changed
- **S-02: Dict now uses IndexMap for O(1) lookup** — `SimValue::Dict` replaced from `Vec<(String, SimValue)>` to `IndexMap<String, SimValue>`. Preserves deterministic insertion-order iteration while providing O(1) amortized key lookup instead of O(n) linear scan. Affects all dict operations: index access, store, get_attr, dict_keys/values/items/get.
- **S-06: Hot-reload behavior documented and surfaced** — When a script is recompiled via Run/Debug, the editor console now shows "[reload] Script recompiled and loaded". Behavior is explicitly defined: ScriptState is fully replaced (PC, stack, variables reset), entity keeps world state (position, health). Doc comments added to `handle_run_script_sim`.

#### Added
- **S-09: Fixed-point arithmetic helpers** — New stdlib builtins `percent(value, pct)` and `scale(value, num, den)` for integer-safe fractional math with banker's rounding. Avoids manual `value * 150 / 100` patterns and integer division pitfalls.
- **S-01: Interpreter/compiler parity test suite** — Integration tests in `crates/deadcode-app/tests/interpreter_compiler_parity.rs` that run identical GrimScript through both the tree-walking interpreter and the compiler/executor paths, comparing outputs. Documents known intentional divergences (float(), game builtin stubs, string display in lists).

### Interpreter

#### Added
- **Custom mod commands in interpreter** — Custom commands defined in mods (via `mod.toml`) now work in the interpreter path (console one-liners, Run/Debug via interpreter). Previously, custom command names like `smite()` would fail with "object is not callable" or "Entity has no method 'smite'" because the interpreter only checked a hardcoded builtin list. The interpreter now accepts a `custom_commands` set, wired through from loaded mod definitions, and dispatches custom commands via `call_builtin_with_custom()`. Command availability gating also applies to custom commands.

### Modding System

#### Added
- **M-10: Phased commands (multi-tick abilities)** — Custom commands can now use `phases` instead of `effects` to create multi-tick abilities with distinct stages. Each phase has a tick duration, `on_start`/`per_tick` effect lists, and an `interruptible` flag. During interruptible phases, the entity's script runs and can cancel the channel by yielding a real action. During non-interruptible phases, script execution is blocked. Channels are cancelled if a `use_resource` effect fails mid-phase. Hot-reload clears active channels. `ChannelState` is stored on `SimEntity`, processed in the tick loop before normal script execution.
- **M-07: `list_commands` effect type** — New `CommandEffect::ListCommands` variant that emits all registered custom commands and their descriptions as `ScriptOutput` events. Commands are sorted alphabetically for deterministic output. The `consult` command in `mods/core/mod.toml` now uses this effect instead of a static output message, making it a discovery mechanic that reveals available commands to the player.

#### Fixed
- **Console commands now execute through the sim** — All console commands are now compiled to IR and executed through the sim world instead of the tree-walking interpreter. This means custom commands (e.g., `help()`) resolve their actual effects, queries return real entity data, and all builtins work identically to scripts. Previously, the interpreter path stubbed custom commands and returned dummy values for queries.
- **M-01: Deterministic mod load order** — Mods are now loaded in alphabetical order by directory name instead of filesystem iteration order. This ensures consistent behavior across platforms and runs. First-loaded-wins for entity type collisions is now predictable.
- **M-04: Spawn entity type validation** — After all mods load, spawn definitions are validated against registered entity types. Unknown entity types produce a clear warning: "[mod:<id>] warning: spawn '<name>' references unknown entity type '<type>'". Also validates spawn effects in custom command definitions.

#### Added
- **M-02: Mod collision warnings** — When multiple mods define the same entity type or command name, a warning is logged identifying both the collision and which mod's definition was kept.
- **M-02: Reserved dependency fields in mod.toml** — `depends_on`, `conflicts_with`, and `min_game_version` fields added to `[mod]` section schema. Parsed but not enforced yet — reserves schema space for future dependency resolution.
- **M-03: Custom command definition validation** — Stat names in `modify_stat` effects and `arg:` target references are now validated at mod load time. Unknown stat names, out-of-range arg indices, and unrecognized arg names produce clear warnings.
- **M-08: `[initial]` section with startup effects** — Mods can now define an `[initial]` section with an `effects` list in `mod.toml`. These effects run in order when the game opens (without loading a saved game state). The intro text ("The dead stir beneath your feet") is now data-driven via `output` effects in the core mod's `[initial]` section instead of hardcoded in the frontend.
- **M-06: `use_resource` effect replaces cost system** — The separate `cost` field on custom commands has been removed. Resource costs are now expressed as a `use_resource` effect (e.g., `{ type = "use_resource", stat = "energy", amount = 30 }`). When a `use_resource` effect encounters insufficient resources, it aborts the command early — remaining effects are skipped and a console warning is printed. This unifies costs into the effect pipeline, giving modders precise control over when resource checks happen relative to other effects.
- **M-05: Phase 2 library API design sketch** — Reserved `libraries` field in `[commands]` schema. Design sketch for `.grim` library files added to `docs/modding.md` covering namespace strategy, gating, and compilation order.
- **M-09: Developer guide for effects vs builtins** — New "Adding New Effects (Developer Guide)" section in `docs/modding.md` explaining the two paths for adding game mechanics (effect system vs builtins), when to use each, step-by-step instructions for adding a new effect, and when validation in `modding.rs` is needed.

#### Fixed
- **BUG-001: Base commands no longer shadow custom definitions** — Removed hardcoded `ActionConsult/Raise/Harvest/Pact` IR instructions, executor handlers, and `UnitAction` variants. The four base commands (`consult`, `raise`, `harvest`, `pact`) now use the data-driven custom command path, meaning their mod.toml effects (spawn, energy cost, stat changes) and costs are actually executed.

### Not Changed
- **S-03 (Print consuming tick):** Investigation confirmed this is already handled correctly — `world.rs` processes Print actions without consuming the tick, re-entering the executor loop.
- **S-08 (Coroutines):** Design exploration only, deferred to future work.
- **Phase 2/3 API:** Covered by reserved fields added in M-02 and design sketch in M-05.
