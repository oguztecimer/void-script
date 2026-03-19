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

#### Fixed
- **M-01: Deterministic mod load order** — Mods are now loaded in alphabetical order by directory name instead of filesystem iteration order. This ensures consistent behavior across platforms and runs. First-loaded-wins for entity type collisions is now predictable.
- **M-04: Spawn entity type validation** — After all mods load, spawn definitions are validated against registered entity types. Unknown entity types produce a clear warning: "[mod:<id>] warning: spawn '<name>' references unknown entity type '<type>'". Also validates spawn effects in custom command definitions.

#### Added
- **M-02: Mod collision warnings** — When multiple mods define the same entity type or command name, a warning is logged identifying both the collision and which mod's definition was kept.
- **M-02: Reserved dependency fields in mod.toml** — `depends_on`, `conflicts_with`, and `min_game_version` fields added to `[mod]` section schema. Parsed but not enforced yet — reserves schema space for future dependency resolution.
- **M-03: Custom command definition validation** — Stat names in `modify_stat` effects and `arg:` target references are now validated at mod load time. Unknown stat names, out-of-range arg indices, and unrecognized arg names produce clear warnings.
- **M-06: Command cost system** — Custom commands can now specify resource costs via `cost = [{ type = "energy", amount = 30 }]` in `mod.toml`. Costs are aggregated per resource type, checked for affordability, and deducted before effects resolve. Insufficient resources skip the command with a console warning.
- **M-05: Phase 2 library API design sketch** — Reserved `libraries` field in `[commands]` schema. Design sketch for `.grim` library files added to `docs/modding.md` covering namespace strategy, gating, and compilation order.

#### Known Issues
- **BUG-001: Base commands shadow custom command definitions** — The four base commands (`consult`, `raise`, `harvest`, `pact`) have hardcoded `ActionBuiltin` entries that take priority over their `[[commands.definitions]]` in mod.toml. Their custom effects and costs are registered but never executed. See `bugs&issues.md` for full analysis and fix options.

### Not Changed
- **S-03 (Print consuming tick):** Investigation confirmed this is already handled correctly — `world.rs` processes Print actions without consuming the tick, re-entering the executor loop.
- **S-08 (Coroutines):** Design exploration only, deferred to future work.
- **Phase 2/3 API:** Covered by reserved fields added in M-02 and design sketch in M-05.
