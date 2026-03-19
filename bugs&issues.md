# Bugs & Issues

## BUG-001: Base Commands Shadow Custom Command Definitions

**Severity: High**
**Status: Resolved**
**Affects: `raise`, `consult`, `harvest`, `pact`**

### Description

The four base commands (`consult`, `raise`, `harvest`, `pact`) are hardcoded as `ActionBuiltin` variants in `crates/deadcode-sim/src/compiler/builtins.rs:94-97`. The compiler's `classify_with_custom()` function (`builtins.rs:62-70`) checks hardcoded builtins **first** — only if a name is `NotBuiltin` does it check custom commands. This means custom command definitions for these names in `mod.toml` are parsed, validated, and registered but **never executed**.

### Execution path (current, broken)

When a player writes `raise()`:

1. Compiler calls `classify_with_custom("raise", &custom_commands)`
2. Inner `classify("raise")` returns `BuiltinKind::Action(ActionBuiltin::Raise)` — match found
3. Custom commands map is **never consulted**
4. Compiler emits `Instruction::ActionRaise` (hardcoded IR instruction)
5. Executor yields `UnitAction::Raise` (hardcoded action variant)
6. `resolve_action()` handles `UnitAction::Raise` → prints `"[raise] Raising the dead..."` and nothing else

### What should happen

`raise` is defined in `mods/core/mod.toml` as:

```toml
[[commands.definitions]]
name = "raise"
args = []
cost = [{ type = "energy", amount = 30 }]
effects = [
  { type = "spawn", entity_type = "skeleton", offset = 1 },
  { type = "output", message = "[raise] A skeleton rises!" },
]
```

This definition (spawn a skeleton, cost 30 energy) is registered in `SimWorld.custom_commands` and `SimWorld.custom_command_costs` but **never triggered** because `ActionCustom("raise")` is never emitted.

### Impact

- The `cost` field on `raise` (added in M-06) does nothing — raise is free despite mod.toml saying it costs 30 energy.
- The spawn effect in `raise` does nothing — no skeleton is actually spawned via this path.
- The same applies to `consult`, `harvest`, and `pact` — their mod.toml effects are dead code.
- Validation (`validate_command_defs`) validates these dead definitions, giving a false sense of correctness.

### Affected files

| File | What's shadowed |
|------|----------------|
| `crates/deadcode-sim/src/compiler/builtins.rs:94-97` | `classify()` returns hardcoded `ActionBuiltin` for these 4 names |
| `crates/deadcode-sim/src/compiler/builtins.rs:62-70` | `classify_with_custom()` checks hardcoded first, custom second |
| `crates/deadcode-sim/src/ir.rs:106-112` | Hardcoded `ActionConsult/Raise/Harvest/Pact` IR instructions |
| `crates/deadcode-sim/src/executor.rs:622-637` | Executor yields hardcoded `UnitAction` variants |
| `crates/deadcode-sim/src/action.rs:174-197` | `resolve_action()` handles hardcoded variants as print-only |

### Fix options

**(a) Remove hardcoded instructions (recommended, clean)**
Remove `ActionConsult`, `ActionRaise`, `ActionHarvest`, `ActionPact` from IR, executor, builtins, and `UnitAction`. Let the custom command path handle them entirely. The mod.toml definitions already specify the correct effects. This is the architecturally correct fix — the whole point of the custom command system is to make these data-driven.

Files to change: `ir.rs` (remove 4 instructions), `executor.rs` (remove 4 match arms), `builtins.rs` (remove 4 classify entries), `action.rs` (remove 4 UnitAction variants + 4 resolve_action arms).

**(b) Reverse priority in `classify_with_custom()`**
Check custom commands first, hardcoded builtins second. One-line change in `builtins.rs:62-70`. Simple but leaves dead code (the hardcoded IR instructions and action variants would still exist but never be emitted for these 4 names).

**(c) Hardcode cost checking into `UnitAction::Raise` etc.**
Add cost logic to the hardcoded resolve paths. Hacky, doesn't scale, duplicates the custom command cost system.

### Recommendation

Option (a). The custom command system was built to replace these hardcoded stubs. The hardcoded variants are vestigial from before Phase 2 modding was implemented.
