# Changelog

## [Unreleased]

### Simulation Engine

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
- **M-07: `list_commands` effect type** — New `CommandEffect::ListCommands` variant that emits all registered custom commands and their descriptions as `ScriptOutput` events. Commands are sorted alphabetically for deterministic output. The `consult` command in `mods/core/mod.toml` now uses this effect instead of a static output message, making it a discovery mechanic that reveals available commands to the player.

#### Fixed
- **M-01: Deterministic mod load order** — Mods are now loaded in alphabetical order by directory name instead of filesystem iteration order. This ensures consistent behavior across platforms and runs. First-loaded-wins for entity type collisions is now predictable.
- **M-04: Spawn entity type validation** — After all mods load, spawn definitions are validated against registered entity types. Unknown entity types produce a clear warning: "[mod:<id>] warning: spawn '<name>' references unknown entity type '<type>'". Also validates spawn effects in custom command definitions.

#### Added
- **M-02: Mod collision warnings** — When multiple mods define the same entity type or command name, a warning is logged identifying both the collision and which mod's definition was kept.
- **M-02: Reserved dependency fields in mod.toml** — `depends_on`, `conflicts_with`, and `min_game_version` fields added to `[mod]` section schema. Parsed but not enforced yet — reserves schema space for future dependency resolution.
- **M-03: Custom command definition validation** — Stat names in `modify_stat` effects and `arg:` target references are now validated at mod load time. Unknown stat names, out-of-range arg indices, and unrecognized arg names produce clear warnings.
- **M-08: `[initial]` section with startup effects** — Mods can now define an `[initial]` section with an `effects` list in `mod.toml`. These effects run in order when the game opens (without loading a saved game state). The intro text ("The dead stir beneath your feet") is now data-driven via `output` effects in the core mod's `[initial]` section instead of hardcoded in the frontend.
- **M-06: `use_resource` effect replaces cost system** — The separate `cost` field on custom commands has been removed. Resource costs are now expressed as a `use_resource` effect (e.g., `{ type = "use_resource", stat = "energy", amount = 30 }`). When a `use_resource` effect encounters insufficient resources, it aborts the command early — remaining effects are skipped and a console warning is printed. This unifies costs into the effect pipeline, giving modders precise control over when resource checks happen relative to other effects.
- **M-05: Phase 2 library API design sketch** — Reserved `libraries` field in `[commands]` schema. Design sketch for `.grim` library files added to `docs/modding.md` covering namespace strategy, gating, and compilation order.

#### Fixed
- **BUG-001: Base commands no longer shadow custom definitions** — Removed hardcoded `ActionConsult/Raise/Harvest/Pact` IR instructions, executor handlers, and `UnitAction` variants. The four base commands (`consult`, `raise`, `harvest`, `pact`) now use the data-driven custom command path, meaning their mod.toml effects (spawn, energy cost, stat changes) and costs are actually executed.

### Not Changed
- **S-03 (Print consuming tick):** Investigation confirmed this is already handled correctly — `world.rs` processes Print actions without consuming the tick, re-entering the executor loop.
- **S-08 (Coroutines):** Design exploration only, deferred to future work.
- **Phase 2/3 API:** Covered by reserved fields added in M-02 and design sketch in M-05.
