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

---

## BUG-002: For-loop `continue` Jumps to PC=0

**Severity: Critical**
**Status: Resolved**

For-loop `continue` emitted `Jump(0)` because `LoopContext.continue_target` was set to 0 and only patched after body compilation. Fixed by using `usize::MAX` sentinel and deferred patching via `continue_patches` vector.

---

## BUG-003: Augmented Index Assignment Fragile Truncation

**Severity: High**
**Status: Resolved**

`x[i] += v` used `instructions.truncate(len - 5)` assuming simple index expressions. Complex indices (e.g., `x[a + b]`) emitted more instructions, causing truncation to leave junk IR. Fixed with clean dual-emit pattern that evaluates index twice.

---

## BUG-004: Division/Modulo Uses Wrong Semantics

**Severity: High**
**Status: Resolved**

Executor used C-style truncating division (`wrapping_div`/`wrapping_rem`), interpreter used Euclidean. Both wrong for Python-style floor division. Fixed to use `floor_div`/`floor_mod` in both paths.

---

## BUG-005: Silent Integer Literal Overflow

**Severity: Medium**
**Status: Resolved**

`parse().unwrap_or(0)` in lexer silently converted overflowing integer literals to 0. Fixed: `tokenize()` now returns `Result` and reports a syntax error.

---

## BUG-006: Dict Iteration Not Supported in Interpreter

**Severity: Medium**
**Status: Resolved**

`for k in dict:` raised "not iterable" in the interpreter. Added `Value::Dict` arm that iterates over keys.

---

## BUG-007: min/max Silent on Incomparable Types

**Severity: Medium**
**Status: Resolved**

`compare_values()` returned `Equal` for mismatched types (e.g., `min(5, "hello")`). Fixed to return `Result`, erroring on type mismatch.

---

## BUG-008: percent/scale Integer Overflow

**Severity: Medium**
**Status: Resolved (re-fixed)**

`wrapping_mul` in `percent()` and `scale()` silently wrapped on overflow. Originally marked resolved but regressed — `wrapping_mul` was still present in executor.rs. Re-fixed with `checked_mul` that returns `SimError::Overflow`.

---

## BUG-009: Instant Action Infinite Loop Risk

**Severity: Medium**
**Status: Resolved**

Both instant-action loops in `SimWorld::tick()` had no guard against infinite Print/GainResource chains. Added 1000-iteration cap with error emission.

---

## BUG-010: Buff Modifier Stat Names Unvalidated

**Severity: Low**
**Status: Resolved**

`validate_buffs()` did not check modifier stat names against known entity stats. Now warns on unknown stat names at load time.

---

## BUG-011: Library Files Not Syntax-Checked

**Severity: Low**
**Status: Resolved**

`.grim` library files were loaded without parsing. Syntax errors were only discovered at script compilation time. Now lex+parse at load time with warnings.

---

## BUG-012: Resource Cap vs Value Not Validated

**Severity: Low**
**Status: Resolved**

No warning when a resource's initial value exceeded its max cap. Added validation in `collect_initial_resources()`.

---

## BUG-013: Dead `fixup_calls` Code

**Severity: Low**
**Status: Resolved**

`fixup_calls()` iterated instructions but did nothing. Removed from `emit.rs` and its call in `compiler/mod.rs`.

---

## BUG-014: Windows GDI DC Leak on Panic

**Severity: Low**
**Status: Resolved**

`CreateDIBSection().unwrap()` panicked without releasing DCs. Replaced with match that releases resources on error.

---

## BUG-015: modding.md Stat Table Lists Non-Existent `mana` Stat

**Severity: Low**
**Status: Resolved**

`mana` was listed as an entity stat in the docs but is actually a global resource. Removed from the stats table.
