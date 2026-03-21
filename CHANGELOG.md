# Changelog

## [Unreleased]

### Simulation Engine

#### Added
- **S-32: Implicit brain loop** — Brain scripts (`ScriptState.is_brain = true`) now implicitly restart from the top when they halt, removing the need for explicit `while True:` loops. `ScriptState::reset_for_restart(entity_id)` resets PC/stack/vars. Non-brain scripts (terminal commands) halt normally. Brain flag is set by `compile_and_assign_entity_brain()` and main brain compilation.
- **S-33: Auto brain assignment on spawn** — When entities are spawned during gameplay (via spawn effects), their brain scripts are automatically compiled and assigned via `EntitySpawned` event handling in `do_tick()`. Brain execution is gated by `is_ready()` so it doesn't start until spawn animation completes.
- **S-34: `SimWorld::flush_pending()` public method** — Extracted inline flush logic into a reusable public method for flushing pending spawns/despawns.

#### Changed
- **S-31: Spawn effect `entity_type` → `entity_id`** — The `CommandEffect::Spawn` field `entity_type` has been renamed to `entity_id` to match entity definition terminology. Serde alias `entity_type` preserved for backward compatibility with existing mod.toml files.

#### Fixed
- **BUG-R6: Scripts not running on startup** — `compile_and_assign_all_brains()` was running before the script store was initialized, so user scripts in `scripts/types/` were not found. Moved to run after script store init and after initial effects flush.
- **BUG-R7: Empty brain script doesn't stop execution** — Saving an empty brain script now clears the entity's `script_state` (and main brain), stopping execution immediately.
- **BUG-R8: Script type classification** — `ensure_type_scripts()` now corrects already-loaded scripts to match the actual `brain` flag from `mod.toml`, fixing non-brain types showing as brains in the editor.

#### Removed
- **S-30: `sacrifice` effect removed** — The `CommandEffect::Sacrifice` variant has been removed from the effect system. The `harvest` command in `core` mod no longer uses it (phase left as empty on_start). Validation in `validate_spawn_effects()` no longer checks for sacrifice entity types.

#### Fixed
- **BUG-R1: percent/scale overflow** — `wrapping_mul` in `Percent` and `Scale` executor instructions replaced with `checked_mul` that returns `SimError::Overflow`. New `SimErrorKind::Overflow` variant and `SimError::overflow()` constructor added.
- **BUG-R2: Negative resources** — `gain_resource()` now clamps to `[0, cap]` instead of just `[_, cap]`, preventing resources from going negative via negative `gain_resource()` amounts.
- **BUG-R3: Integer overflow in DynInt and stat effects** — `DynInt` multipliers use `saturating_mul()`. `Heal`, `ModifyStat` effects and buff modifier application/reversal use `saturating_add()`/`saturating_sub()` to prevent overflow.
- **BUG-R4: Unresolved function calls** — Forward-referenced function calls that are never defined now produce a compile-time error (`undefined function: <name>`) instead of silently leaving `usize::MAX` as the call target for a runtime error.
- **BUG-R5: SimWorld HashMap→IndexMap** — Converted 9 `HashMap` fields in `SimWorld` to `IndexMap` for deterministic iteration order: `entity_index`, `custom_commands`, `custom_command_arg_counts`, `custom_command_descriptions`, `custom_command_phases`, `entity_configs`, `entity_types_registry`, `spawn_durations`, `resource_caps`, `buff_registry`.

#### Added
- **S-04: Error recovery** — Scripts that hit runtime errors now automatically recover on the next tick: error is cleared, script state is reset (PC, stack, call stack, variables), entity yields `wait()` for one tick, then re-executes from the beginning. A `[error recovery]` message is emitted to the console. Applies to entity scripts, main brain, and channel interruptible scripts.
- **S-07 Phase 1: Variable state dump on error** — `SimEvent::ScriptError` now includes variable state snapshot, stack contents, and program counter at the time of error. New `ScriptErrorDetail` IPC message forwards this data to the editor UI, which displays it in the terminal panel for debugging.

### Editor

#### Added
- **E-06: ScriptReloaded IPC handler** — Frontend now handles the `script_reloaded` IPC message and displays a `[reload]` notification in the terminal panel.
- **E-07: ScriptErrorDetail IPC handler** — Frontend now handles the `script_error_detail` IPC message and displays variable state, stack contents, and PC in the terminal panel when a script error occurs.

### Simulation Engine (previous)

#### Added
- **S-21: Multi-type entity system** — Entities now support composable type tags via a `types: Vec<String>` field on `SimEntity`. Query functions `scan()` and `nearest()` filter by `has_type()` instead of exact `entity_type` match, so `scan("undead")` matches any entity with the "undead" type tag. New `has_type(entity, name)` and `get_types(entity)` GrimScript builtins added (mapped to `QueryHasType` and `QueryGetTypes` IR instructions). Entity attribute access via `entity.types` returns the type list. `DynInt::EntityCount` and `Condition::EntityCount` use `has_type()` for matching. Trigger filter matching uses the types list instead of the single entity_type. `SimWorld` gains `entity_types_registry` for spawn effect type resolution and `spawn_entity_with_types()` for creating entities with explicit type tags. `EntitySnapshot` includes `types` field.
- **S-22: Entity definition IDs** — `SimEntity.entity_type` now serves as the unique entity definition ID for registry lookups (sprites, configs), while `SimEntity.types` provides composable tags for queries and filtering. `SimEntity::new()` auto-populates `types = [entity_type]` for backward compatibility. `SimEntity::new_with_types()` allows explicit type tag specification.

### Modding System

#### Added
- **M-03: Type definitions** — New `[[types]]` section in `mod.toml` for defining composable type tags with stats, commands, and brain scripts. Each `TypeDef` has `name`, `brain` (bool), `stats` (IndexMap), `commands` (Vec), and optional `script` path. Type stats are merged in type order, then entity-level stats override. Type `.gs` scripts are loaded from `grimscript/` directory and syntax-checked at mod load time.
- **M-04: Entity def ID and types** — `[[entities]]` gains `id` (unique definition key) and `types` (list of type tag names). Backward compat: if `id` absent, `type` is used; if `types` absent, defaults to `[id]`. Entity stats are now merged from types (in order) then entity-level overrides.
- **M-05: Spawn entity_id field** — ~~`[[spawn]]` gains `entity_id` field.~~ Removed — `[[spawn]]` replaced by `[initial].effects` spawn effects.

#### Removed
- **M-11: `[[spawn]]` removed** — `[[spawn]]` blocks in `mod.toml` are no longer supported. Use `[initial].effects` with `{ type = "spawn", entity_id = "...", offset = 0 }` instead. The `SpawnDef` struct and spawn processing loop have been removed. Validation of entity type references in spawn effects is now handled by `validate_spawn_effects()`.
- **M-12: Embedded fallback removed** — If no mods are found in the `mods/` directory, nothing loads. The hardcoded `embedded_fallback()` function and its compile-time asset references have been removed.

### Simulation Engine (continued)

#### Added
- **S-23: Main brain** — Entity-less built-in brain stored as `SimWorld.main_brain: Option<ScriptState>`. Runs first each tick (before entity shuffle), using sentinel `EntityId(0)`. Can call resource ops, queries, print, and custom commands. Cannot perform entity actions (move, attack, flee). Terminal commands execute against the main brain.

### Modding System (continued)

#### Added
- **M-06: Type-based command gating** — Two-layer gate: global unlock (`[initial].commands`) intersected with type capability (`commands` on `[[types]]`). An entity can only use commands that appear in at least one of its types' `commands` lists AND are globally unlocked. If no types define commands, falls back to all globally unlocked commands (backward compat). Computed via `compute_effective_commands()`.
- **M-07: Brain script compilation** — Entities with brain types get compiled scripts assigned on spawn. Script composition: non-brain type `.gs` files (library functions) + mod libraries + brain type `.gs` (main execution logic). Brain scripts are compiled with the entity's effective command set.
- **M-08: Auto-reload on save** — Saving a type `.gs` file in the editor triggers hot-reload: brain type changes recompile all entities with that brain; non-brain type changes recompile all entities that include that type (library changed); `main.gs` changes recompile the main brain.
- **M-09: Type script file management** — `ensure_type_scripts()` creates `.gs` files in `scripts/types/` from mod defaults if they don't already exist. Run/Debug buttons on type scripts trigger auto-reload instead of summoner binding.
- **M-10: Type and entity validation** — `validate_type_defs()` checks for empty/duplicate type names. `validate_entity_defs()` checks for empty/duplicate entity IDs, unknown type references, duplicate types per entity, and brain count (warns on >1 brain type).

### Editor

#### Added
- **E-01: Type script categories** — Script list now supports `type_brain` (Brains) and `type_library` (Libraries) script types for type-associated scripts in the `scripts/types/` subdirectory.
- **E-02: ScriptReloaded IPC message** — New `script_reloaded` message notifies frontend when a type script is hot-reloaded.

#### Removed
- **E-03: Run/Stop script buttons** — Replaced by Pause/Resume simulation buttons. Scripts auto-reload on save; no manual run/stop needed. `RunScript`, `StopScript` IPC messages removed. `ScriptStarted`, `ScriptFinished` IPC messages removed.
- **E-04: SummonerBrain script type** — Removed. All entity scripts are now type-based (TypeBrain/TypeLibrary). The `summoner_brain` category no longer exists.

#### Changed
- **E-05: Pause/Resume simulation** — Former no-op simulation controls now actually pause/resume the sim. Toolbar shows a pause button when running, resume button when paused.
- **M-01: Removed entity convenience stat fields** — `[[entities]]` no longer supports top-level `health`, `speed`, `attack_damage`, `attack_range`, `attack_cooldown`, and `shield` fields. All stats are now defined exclusively in the `stats` table (e.g., `stats = { health = 50, speed = 2 }`). Auto-max behavior (`max_health`/`max_shield`) is preserved via `apply_config()`. The `custom_stats` alias still works.
- **M-02: Summoner defined by core mod** — The summoner entity is no longer hardcoded in `app.rs`. It is now defined in `mods/core/mod.toml` as a normal `[[entities]]` entry, spawned via `[initial].effects`. Entity type, stats, sprite, and pivot are all moddable. No embedded fallback — if no mods found, nothing loads. Script execution still finds the summoner by entity type `"summoner"`.

### Simulation Engine

#### Added
- **S-20: `is_alive` and `distance` conditions** — Two new `Condition` variants for target-bearing spatial checks. `is_alive { target }` resolves a target entity and returns true if it exists and is alive (false if the target can't be resolved). `distance { target, compare, amount }` computes the absolute integer distance between the caster's position and the target's position and compares it using a `CompareOp`; returns false if the target can't be resolved. Both use `resolve_target_from_args()` and accept all scoped target strings (`"self"`, `"arg:name"`, `"source"`, `"owner"`, `"attacker"`, `"killer"`). `evaluate_condition_with_ctx()` is the new canonical condition evaluator, accepting args and `EffectContext`; `evaluate_condition()` is preserved as a backward-compatible wrapper that passes empty args and a default context. `resolve_effects_inner()` calls `evaluate_condition_with_ctx()` so `if` effects and trigger conditions can resolve scoped targets in condition `target` fields.
- **S-19: Scoped targets in trigger effects** — Trigger effect resolution now carries an `EffectContext` struct that holds references to event participants: `source` (event subject), `owner` (owner entity), `attacker` (damage dealer), and `killer` (killing-blow dealer). Four new scoped target strings — `"source"`, `"owner"`, `"attacker"`, `"killer"` — can be used in trigger effect `target` fields and resolve via `EffectContext` before falling back to entity fields for `"owner"`. Scoped targets that are not applicable to the current event (e.g., `"attacker"` in a `tick_interval` trigger) silently no-op the effect. `SimEntity.owner` changed from `u64` to `Option<EntityId>`, automatically set during `spawn` effects. `SimEvent` variants enriched: `EntityDamaged` gains `attacker_id`, `EntityDied` gains `killer_id`/`owner_id`, `EntitySpawned` gains `spawner_id`. `get_owner()` builtin now returns `EntityRef` or `None` instead of `Int(0)`. `validate_target()` in `modding.rs` accepts scoped target strings for trigger effects.

### Compiler

#### Fixed
- **C-01: For-loop `continue` jumps to PC=0** — `continue` inside for-loops emitted `Jump(0)` because the increment target wasn't known yet. Now uses `usize::MAX` sentinel and deferred patching (same pattern as `break`). While-loop `continue` unchanged. 2 new tests.
- **C-02: Augmented index assignment fragile truncation** — `x[i] += v` used `instructions.truncate(len - 5)` which assumed index expressions emit exactly 1 instruction. Complex indices like `x[a + b]` left junk IR. Replaced with clean dual-emit pattern: `obj, idx, obj, idx → Index → rhs → op → StoreIndex → store`. 2 new tests.
- **C-03: Dead `fixup_calls` code** — Removed no-op `fixup_calls()` from `emit.rs` and its call in `compiler/mod.rs`.

### Simulation Engine

#### Fixed
- **S-12: `evaluate_condition()` DynInt resolution** — Conditions with game-state `DynInt` thresholds (`entity_count(...)`, `resource(...)`, `stat(...)`) now use `resolve_with_world()` instead of `resolve()`. Previously these always resolved to 0, making all game-state-aware condition thresholds broken.
- **S-13: `nearest()` deterministic tie-breaking** — When multiple entities are equidistant, the one with the lower `EntityId` now wins. Previously tie-breaking depended on iteration order, which could vary with spawn/despawn patterns, breaking seed-based determinism.
- **S-14: Entity stats HashMap → IndexMap** — `SimEntity.stats` and `EntityConfig.stats` now use `IndexMap<String, i64>` for deterministic iteration order. `BuffDef.modifiers` also converted. Prevents future determinism regressions if stats are ever iterated during simulation.
- **S-15: `in`/`not in` operators** — Added `Contains` and `NotContains` IR instructions and implemented membership testing in both the interpreter and compiler/executor. `in` works for lists (element membership), strings (substring), and dicts (key lookup). Previously `in` was parsed but mapped to `==`, always returning wrong results.
- **S-16: Unicode string `len()` and negative indexing** — `len()` on strings now returns character count instead of UTF-8 byte count. Negative string indexing uses character count for bounds calculation. Negative list/tuple index underflow now returns an error instead of wrapping to a huge value and panicking.
- **S-17: Division/modulo semantics** — Executor `Div`/`Mod` instructions now use Python-style floor division/modulo instead of C-style truncating division. `-7 // 2 = -4` (was -3), `-7 % 2 = 1` (was -1). Interpreter `//` and `%` also fixed from Euclidean to floor semantics for negative divisors. 7 new executor tests, 4 parity tests.
- **S-18: Instant action infinite loop guard** — Both instant-action loops in `SimWorld::tick()` (channel processing and normal execution) now cap at 1000 iterations. Prevents infinite loops from scripts that only emit instant actions (e.g., `while True: gain_resource("x", 1)`).

### Language

#### Fixed
- **L-01: Silent number parsing overflow** — `Lexer::tokenize()` now returns `Result`, reporting an error for integer literals that overflow i64 instead of silently converting them to 0. Call sites in `lib.rs` and `compiler/mod.rs` updated to propagate errors.
- **L-02: Dict iteration** — `for k in dict:` now iterates over dictionary keys in the interpreter. Previously raised "not iterable" for dict values.
- **L-03: `min`/`max` incomparable types** — `compare_values()` now returns `Result` and errors on type mismatches (e.g., `min(5, "hello")`). Previously returned `Equal`, giving silently wrong results.
- **L-04: `percent`/`scale` integer overflow** — `wrapping_mul` replaced with `checked_mul` in both `percent()` and `scale()`. Overflow now returns a runtime error instead of silently wrapping.

### Desktop / Rendering

#### Fixed
- **D-01: Hit test negative coordinate guard** — `hit_test_at()` now checks for negative coordinates before casting to `u32`, preventing an out-of-bounds read when mouse coordinates are negative.
- **D-02: macOS pixel buffer bounds validation** — Added `debug_assert` for canvas/buffer size parity and bounds-checked the pixel conversion loop and `copy_nonoverlapping` call to prevent buffer overflows on size mismatches.
- **D-03: Entity position sync by ID** — Position sync between sim entities and render units now uses an `EntityId→UnitId` mapping instead of string name matching. Fixes incorrect syncing when multiple entities share the same name. Death handling and animation targeting also use the ID map.
- **D-04: Windows GDI DC leak on panic** — `CreateDIBSection` failure in Windows renderer now gracefully releases `hdc_mem` and `hdc_screen` and returns instead of panicking with `.unwrap()`, preventing GDI resource leaks.

### Editor UI

#### Fixed
- **E-01: Error boundary** — Added React error boundary wrapping the editor UI. Rendering errors are caught and display a fallback with an error message and reload button instead of crashing the entire UI.
- **E-02: Diagnostic line positioning** — CodeMirror linter now computes proper `from`/`to` positions from diagnostic line/column data instead of hardcoding position 0. Falls back to highlighting the whole line when column info is missing.
- **E-03: IPC bridge default case** — Unknown IPC message types are now logged via `console.warn` instead of silently ignored.

#### Changed
- **S-XX: Unified entity stats into single HashMap** — Removed 9 hardcoded stat fields (`health`, `max_health`, `shield`, `max_shield`, `speed`, `attack_damage`, `attack_range`, `attack_cooldown`, `cooldown_remaining`) from `SimEntity` and the separate `custom_stats: HashMap<String, i64>`. All stats now live in a single `stats: HashMap<String, i64>` accessed via `stat()`, `set_stat()`, and `clamp_stat()` helpers. `EntityConfig` simplified to `{ stats: HashMap<String, i64> }`. Eliminated parallel effect/condition systems: `ModifyCustomStat`/`UseCustomStat`/`Condition::CustomStat` removed — `ModifyStat`/`UseResource`/`Condition::Stat` now handle all stats generically. Serde aliases preserve backward compatibility for existing `mod.toml` files (`modify_custom_stat`, `use_custom_stat`, `custom_stat` still parse correctly). Renamed `get_custom_stat` GrimScript builtin to `get_stat` (old name kept as alias). Fixed pre-existing bug: `Spawn` effect now applies `EntityConfig` to dynamically spawned entities. 182 tests pass.

### Modding System

#### Fixed
- **M-24: Buff modifier stat names unvalidated** — `validate_buffs()` now checks modifier stat names against known stats from entity configs. Unknown stat names produce a warning at load time.
- **M-25: Library files not syntax-checked** — `.grim` library files are now parsed (lex + parse) at mod load time. Syntax errors produce a warning with mod ID and filename. The source is still prepended for graceful degradation.
- **M-26: Resource cap vs value not validated** — `collect_initial_resources()` now warns when a resource's initial value exceeds its defined max cap.

### Documentation

#### Fixed
- **DOC-01: modding.md stat table** — Removed incorrect `mana` row (mana is a global resource, not an entity stat).

#### Added
- **M-22: Mod dependency resolution** — `depends_on` and `conflicts_with` fields in `[mod]` are now enforced. Mods are topologically sorted by dependencies (Kahn's algorithm with alphabetical tie-breaking for determinism). Missing dependencies cause the mod and its dependants to be skipped with warnings. Conflicts skip the second-loaded mod. Circular dependencies are detected and fall back to alphabetical order with an error message. 8 new tests.
- **M-23: Library file system** — Mods can now provide `.grim` library files via `commands.libraries` in `mod.toml`. Library source files are loaded at mod load time and prepended to player scripts before compilation, making library functions available as if defined at the top of the script. Flat namespace with first-loaded-wins (consistent with commands/entities). Library functions are subject to the same command gating as player scripts. Works for both Run and Console compilation paths.

#### Changed
- **M-12: Rename `per_tick` to `per_update` with `update_interval`** — Phase effects field `per_tick` renamed to `per_update`. New `update_interval` field (default 1) controls how often `per_update` effects fire: every N ticks instead of every tick. `(ticks_elapsed + 1) % update_interval == 0` determines update ticks — interval=2 fires at ticks 1,3,5 (not tick 0). Validated at load time (`update_interval` must be > 0). All `mod.toml` files, tests, and docs updated.

#### Added
- **M-14: Conditional effects (`if` effect type)** — New `CommandEffect::If` variant enables branching in effect lists based on game state. Conditions evaluate global resources (`resource`), entity counts (`entity_count`), or caster stats (`stat`) against a threshold using comparison operators (`eq`, `ne`, `gt`, `gte`, `lt`, `lte`). Supports `then` and optional `else` effect lists with full nesting. Abort propagation: a `use_resource`/`use_global_resource` failure inside a branch aborts the entire command. Conditions are validated at mod load time (empty names, unknown stat names).
- **M-15: `start_channel` effect type** — New `CommandEffect::StartChannel` variant initiates a phased channel from within an effect list. Combined with `if`, enables conditional phase branching — different branches can start different channels. Effects before `start_channel` run normally; remaining effects after it are skipped. Phase definitions use the same schema as top-level `phases`. `start_channel` inside an already-active channel is ignored. Phase ticks and update_interval are validated at mod load time.
- **M-21: Computed values / DynValue (Phase 6)** — `DynInt` extended with three game-state-dependent variants: `EntityCount` (count of alive entities of a type), `ResourceValue` (global resource value), `CasterStat` (caster's stat value, including custom stats). TOML format: `"entity_count(skeleton)"`, `"resource(mana)"`, `"stat(health)"`, with optional multiplier syntax `"entity_count(skeleton)*2"`. All effect `amount`/`offset` fields now use `resolve_with_world()` for game-state access. Backward compatible with plain integers and `rand(min,max)`. 4 new tests.
- **M-20: Custom entity stats (Phase 5)** — Entity types can define `custom_stats` in `[[entities]]` in `mod.toml` (e.g., `custom_stats = { armor = 5, crit_chance = 10 }`). Custom stats stored per-entity in `SimEntity.custom_stats: HashMap<String, i64>`, applied via `EntityConfig` at spawn time. Two new `CommandEffect` variants: `modify_custom_stat` (add/subtract custom stat on target) and `use_custom_stat` (check-and-deduct, aborts remaining effects if insufficient). New `custom_stat` condition for branching on custom stat values. New `get_custom_stat(entity, "name")` GrimScript builtin (query, instant) returning Int (0 if undefined). Custom stats also accessible via entity attribute access (e.g., `entity.armor` in scripts). 3 new tests.
- **M-19: Extended conditions (Phase 4)** — Four new `Condition` variants: `has_buff` checks if the caster has a specific active buff, `random_chance` with deterministic RNG (`roll < percent`), `and` (all sub-conditions must be true), and `or` (at least one must be true). Compound conditions support nesting. Validated at mod load time.
- **M-18: Buff/modifier system (Phase 3)** — Mods can define `[[buffs]]` in `mod.toml` — temporary stat modifiers with automatic expiry. Each `BuffDef` has `name`, `duration` (ticks), `modifiers` (stat→amount for health/shield/speed/attack_damage/attack_range), `per_tick`/`on_apply`/`on_expire` effect lists, `stackable` flag, and `max_stacks`. Two new `CommandEffect` variants: `apply_buff` (apply to target with optional duration override) and `remove_buff` (remove from target, reversing modifiers). Active buffs tracked per-entity via `SimEntity.active_buffs`. Modifiers directly modify stats on apply and reverse on expire (health clamped to 1 to prevent buff-expiry death). Stackable buffs accumulate stacks; non-stackable refreshes duration. Buff tick processing (step 6b) runs per_tick effects, decrements durations, and handles expiry. Buff definitions stored in `SimWorld.buff_registry`. Full validation at load time. 7 new tests.
- **M-16: Event/trigger system** — Mods can now define `[[triggers]]` in `mod.toml` — event-driven rules that fire effects when game events occur. Supports 8 event types: `entity_died`, `entity_spawned`, `entity_damaged`, `resource_changed`, `command_used`, `tick_interval`, `channel_completed`, `channel_interrupted`. Each trigger has type-specific filters (entity_type, resource, command, interval), optional conditions (reuses existing `Condition` system), and effects (reuses existing `CommandEffect` system). Triggers are processed once at the end of each tick — events are matched against registered triggers, filters and conditions gate firing, effects resolve against the first alive entity. Trigger effects do not cascade (no re-triggering within the same tick). `resource_changed` uses snapshot-based detection. `tick_interval` fires when `tick % interval == 0`. Three new `SimEvent` variants added: `CommandUsed`, `ChannelCompleted`, `ChannelInterrupted`. Full validation at load time (event names, interval values, conditions, effects). 7 new tests covering entity death triggers, filters, tick intervals, resource changes, conditions, and command triggers.
- **M-13: `unlisted` field on command definitions** — Commands can set `unlisted = true` in `mod.toml` to be hidden from `list_commands` output while remaining fully functional. Useful for meta-commands like `help` that trigger `list_commands` themselves.
- **M-11: `sacrifice` effect type** — ~~Removed in S-30.~~

### Desktop / Rendering

#### Fixed
- **Entity death rendering cleanup** — `SimEvent::EntityDied` is now handled in the game loop. Dead entities play their "death" animation (if the atlas has one) and are removed from rendering once it finishes. Previously, `EntityDied` was unhandled — dead entities' render units lingered in `UnitManager` forever as invisible orphans. `SimEvent::EntityDied` now carries the entity `name` so the handler can find the matching render unit after the sim has already removed the entity. `UnitManager` gained `kill(id)` (play death + mark pending_destroy) and `reap_dead()` (remove units whose death animation finished).
- **Fixed duplicate render units from Spawn effect** — The `Spawn` command effect was emitting an `EntitySpawned` event directly, but `flush_pending()` in the tick loop already emits `EntitySpawned` when the queued entity is actually added. This created two render units per spawn, making death cleanup appear to do nothing (one unit was killed but the duplicate remained). Removed the premature event from the effect handler.
- **Unique entity naming for runtime spawns** — Runtime-spawned entities (from `Spawn` effects) now use `"{type}_{entity_id}"` instead of `"{type}_{position}"` as their name. The old scheme could produce duplicates when entities spawned at the same position (e.g. calling `raise()` twice without moving), causing name collisions in position sync, death handling, and animation targeting. Entity ID is a monotonically increasing u64, guaranteed unique. `SimEvent::EntitySpawned` now carries the entity `name` directly so app.rs doesn't reconstruct it.
- **Unified sim event handling for console and script paths** — Console commands (`handle_console_command_sim`) and the sim tick loop (`do_tick`) now share the same `forward_sim_event_to_editor()` and `apply_sim_event_to_units()` methods. Previously, console commands had a separate `apply_sim_event_to_units` that only handled `PlayAnimation`, missing `EntitySpawned` and `EntityDied` events. This caused `harvest()` typed in the console to kill skeletons in the sim but not update the render units.

### Core

#### Changed
- **Summoner is now a hardcoded core entity** — The summoner is no longer defined or spawned via `mod.toml`. It is always created by the game engine at position 500 with fixed stats (100 HP, 100 mana, speed 1) using embedded sprite assets. Mods cannot override or redefine the summoner entity type. This ensures the player's core entity is always present and consistent regardless of mod configuration.

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
- **M-06: `use_resource` effect replaces cost system** — The separate `cost` field on custom commands has been removed. Resource costs are now expressed as a `use_resource` effect (e.g., `{ type = "use_resource", stat = "mana", amount = 30 }`). When a `use_resource` effect encounters insufficient resources, it aborts the command early — remaining effects are skipped and a console warning is printed. This unifies costs into the effect pipeline, giving modders precise control over when resource checks happen relative to other effects.
- **M-05: Phase 2 library API design sketch** — Reserved `libraries` field in `[commands]` schema. Design sketch for `.grim` library files added to `docs/modding.md` covering namespace strategy, gating, and compilation order.
- **M-09: Developer guide for effects vs builtins** — New "Adding New Effects (Developer Guide)" section in `docs/modding.md` explaining the two paths for adding game mechanics (effect system vs builtins), when to use each, step-by-step instructions for adding a new effect, and when validation in `modding.rs` is needed.

#### Fixed
- **BUG-001: Base commands no longer shadow custom definitions** — Removed hardcoded `ActionConsult/Raise/Harvest/Pact` IR instructions, executor handlers, and `UnitAction` variants. The four base commands (`consult`, `raise`, `harvest`, `pact`) now use the data-driven custom command path, meaning their mod.toml effects (spawn, energy cost, stat changes) and costs are actually executed.

### Not Changed
- **S-03 (Print consuming tick):** Investigation confirmed this is already handled correctly — `world.rs` processes Print actions without consuming the tick, re-entering the executor loop.
- **S-08 (Coroutines):** Design exploration only, deferred to future work.
- **Phase 2/3 API:** Covered by reserved fields added in M-02 and design sketch in M-05.
