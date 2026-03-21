# Modding Guide

How to create mods for VOID//SCRIPT. The base game ("Core") is itself a mod — the same system that loads it can load custom content.

---

## Table of Contents

- [Mod Structure](#mod-structure)
- [mod.toml Reference](#modtoml-reference)
- [Type Definitions](#type-definitions)
- [Entity Definitions](#entity-definitions)
- [Global Resources](#global-resources)
- [Available Commands](#available-commands)
- [Initial Effects](#initial-effects)
- [Custom Command Definitions](#custom-command-definitions)
- [Effect Types](#effect-types)
- [Computed Values (DynInt)](#computed-values-dynint)
- [Target Resolution](#target-resolution)
- [Condition Types](#condition-types)
- [Conditional Effects](#conditional-effects)
- [Phased Commands](#phased-commands)
- [Conditional Phase Branching (`start_channel`)](#conditional-phase-branching-start_channel)
- [Event Triggers](#event-triggers)
- [Buffs / Modifiers](#buffs--modifiers)
- [Entity Stats](#entity-stats)
- [Library Files](#library-files)
- [Sprite Format](#sprite-format)
- [Multiple Mods](#multiple-mods)
- [Validation](#validation)
- [Runtime Entity Spawning](#runtime-entity-spawning)
- [Fallback Behavior](#fallback-behavior)
- [The Base Game Mod](#the-base-game-mod)
- [Creating a New Mod](#creating-a-new-mod)
- [GrimScript API Reference](#grimscript-api-reference)
- [Adding New Effects (Developer Guide)](#adding-new-effects-developer-guide)
- [Internals](#internals)

---

## Mod Structure

Each mod is a directory inside `mods/` containing a `mod.toml` manifest and any associated assets:

```
mods/
  my-mod/
    mod.toml                # Required: mod manifest
    lib/
      utils.grim            # GrimScript library files (optional)
    sprites/
      warrior_atlas.png     # Sprite sheet PNG
      warrior_atlas.json    # Atlas metadata (frame layout)
```

The game scans `mods/` at startup and loads every directory that contains a valid `mod.toml`. Mods are then reordered by their dependency graph (topological sort via Kahn's algorithm, with alphabetical tie-breaking for determinism). If no mods are found, nothing loads.

### Dependencies and Conflicts

Mods can declare dependencies and conflicts in the `[mod]` section:

- **`depends_on`**: List of mod IDs that must be loaded first. If a dependency is missing, the mod (and any mods that depend on it) is skipped with a warning. This cascades — if A depends on B and B is missing, A is also skipped.
- **`conflicts_with`**: List of mod IDs that cannot be loaded alongside this mod. If a conflict exists, the first-loaded mod wins and the conflicting mod is skipped with a warning.

Circular dependencies are detected and logged as an error. The affected mods fall back to alphabetical ordering.

---

## mod.toml Reference

A complete `mod.toml` with all sections:

```toml
# --- Mod Metadata ---
[mod]
id = "my-mod"           # Unique identifier (lowercase, no spaces)
name = "My Mod"         # Display name
version = "0.1.0"       # Semver version string
depends_on = []         # Mod IDs this mod requires (loaded first)
conflicts_with = []     # Mod IDs that cannot coexist with this mod

# --- Type Definitions ---
[[types]]
name = "undead"                      # Type tag name
stats = { health = 50 }              # Stats provided by this type
commands = ["raise", "harvest"]      # Commands entities with this type can use

[[types]]
name = "melee"
stats = { speed = 2, attack_damage = 10 }
commands = ["attack"]

[[types]]
name = "skeleton_ai"
brain = true                         # Brain types drive entity execution via .gs files
commands = ["move", "attack", "flee"]

# --- Entity Definitions ---
[[entities]]
id = "warrior"                      # Unique entity definition ID (for registry lookups)
types = ["undead", "melee"]         # Composable type tags (stats merged in order)
sprite = "sprites/warrior_atlas"    # Path to sprite files (no extension; expects .png + .json)
pivot = [24.0, 0.0]                 # Sprite pivot point [x, y]
stats = { armor = 5, crit = 10 }   # Entity-level stats (override type stats)

# Backward compat: if `id` is absent, `type` is used as the ID.
# If `types` is absent, defaults to `[id]`.

# --- Global Resources ---
[resources]
souls = 0                           # Capless resource (plain integer)
mana = { value = 50, max = 100 }    # Capped resource (initial value + max cap)
gold = 100                          # Capless resource

# --- Initial State ---
[initial]
commands = ["raise", "harvest"]     # Commands available at game start
resources = ["souls", "mana"]       # Resources available at game start (omit = all available)
effects = [                         # Effects that run on first game open
  { type = "output", message = "Welcome to the void..." },
]

# --- Custom Commands ---
[commands]
libraries = ["lib/utils.grim"]      # GrimScript library files to prepend to player scripts

[[commands.definitions]]
name = "smite"
description = "Strike with dark power"
unlisted = false                    # If true, hidden from list_commands (default: false)
args = ["target"]                   # Positional argument names
effects = [                         # Instant effects (mutually exclusive with phases)
  { type = "damage", target = "arg:target", amount = 30 },
]

[[commands.definitions]]
name = "channel_fire"
description = "Channel flames"
args = []
phases = [                          # Multi-tick phases (mutually exclusive with effects)
  { ticks = 10, interruptible = true, on_start = [
    { type = "output", message = "Channeling..." }
  ]},
]

# --- Event Triggers ---
[[triggers]]
event = "entity_died"
filter = { entity_type = "skeleton" }
conditions = [
  { type = "entity_count", entity_type = "skeleton", compare = "eq", amount = 0 },
]
effects = [
  { type = "output", message = "All skeletons have fallen!" },
]

# --- Buff Definitions ---
[[buffs]]
name = "rage"
duration = 60
stackable = true
max_stacks = 3
[buffs.modifiers]
attack_damage = 5
speed = 2
```

---

## Type Definitions

Types are composable tags that provide stats, commands, and behavior to entities. An entity can have multiple types — their stats are merged in order.

```toml
[[types]]
name = "undead"                      # Type tag name (unique across all mods)
stats = { health = 50 }              # Stats provided by this type
commands = ["raise", "harvest"]      # Commands entities with this type can use

[[types]]
name = "melee"
stats = { speed = 2, attack_damage = 10, attack_range = 3 }
commands = ["attack"]

[[types]]
name = "skeleton_ai"
brain = true                         # Brain types drive entity execution via .gs files
commands = ["move", "attack", "flee"]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | yes | — | Unique type tag name |
| `brain` | no | `false` | If true, this type drives entity execution via a `.gs` script |
| `stats` | no | `{}` | Stats provided by this type (merged in type order) |
| `commands` | no | `[]` | Commands entities with this type can use |
| `script` | no | `{name}.gs` | Path to .gs script in `grimscript/` directory |

### Stats Resolution

For an entity with `types = ["undead", "melee"]` and entity-level `stats = { armor = 5 }`:

1. Start with empty stats
2. Apply `undead` stats: `{ health = 50 }`
3. Apply `melee` stats: `{ health = 50, speed = 2, attack_damage = 10 }` (speed/attack added, no conflicts)
4. Apply entity-level stats: `{ health = 50, speed = 2, attack_damage = 10, armor = 5 }` (armor added)
5. `apply_config()` auto-sets `max_health`/`max_shield` as before

### Type Queries in GrimScript

Entities can be queried by type tag:

```python
# scan/nearest match by type tag (not just entity def ID)
minions = scan("undead")          # finds all entities with "undead" type
closest = nearest("melee")        # finds nearest entity with "melee" type

# Direct type queries
types = get_types(entity)          # returns list of type tags
is_undead = has_type(entity, "undead")  # returns bool
entity_types = entity.types        # attribute access to type list
```

---

## Entity Definitions

Entity definitions register entities that can be spawned — either at startup via `[[spawn]]` or at runtime by effects (e.g., the `raise` command spawns skeletons).

```toml
[[entities]]
id = "golem"                        # Unique entity definition ID
types = ["undead", "melee"]         # Composable type tags (stats merged in order)
sprite = "sprites/golem_atlas"
pivot = [24.0, 0.0]
stats = { armor = 5, crit_chance = 10 }   # Override/extend type stats
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `id` | yes* | from `type` | Unique entity definition ID (for registry lookups) |
| `type` | no | — | Legacy field, used as `id` if `id` is absent |
| `types` | no | `[id]` | Composable type tags from `[[types]]` definitions |
| `sprite` | no | none | Sprite atlas path (no extension; expects .png + .json) |
| `pivot` | no | `[24, 0]` | Sprite pivot point `[x, y]` |
| `stats` | no | `{}` | Entity-level stats (override type stats) |

*Either `id` or `type` must be present.

**Backward compatibility:** If `id` is absent, `type` is used as the ID. If `types` is absent, defaults to `[id]`.

**Note:** The `"summoner"` entity is defined by the core mod. Custom mods can define their own entities but cannot redefine the summoner — it is always provided by core.

### Auto-Max Behavior

When `health` or `shield` are set and no explicit `max_health`/`max_shield` is provided, the engine automatically sets `max_health`/`max_shield` to the same value.

---

## Global Resources

Global resources are world-level integer values shared across all entities (not per-entity stats like health).

### Definition

```toml
[resources]
# Plain integer (capless) — no maximum
souls = 0
bones = 0
gold = 100

# Object with cap — initial value and maximum
mana = { value = 50, max = 100 }
```

Each key is the resource name. The value is either a plain integer (capless) or `{ value, max }` (capped). Resources from all mods are merged at load time (first-defined wins with warning on duplicates). A warning is emitted at load time if a resource's initial value exceeds its max cap.

### Resource Availability

Resources have an available/unavailable mechanic. `[initial].resources` controls which resources are usable from game start:

```toml
[initial]
resources = ["souls", "mana"]
```

Calling `get_resource()`, `gain_resource()`, or `try_spend_resource()` on an unavailable resource produces a runtime error.

If `[initial].resources` is omitted or empty, **all defined resources are available by default**.

In dev mode (`--features dev-mode`), all resources are available regardless.

### GrimScript API

Three builtins interact with global resources:

| Function | Returns | Description |
|----------|---------|-------------|
| `get_resource("name")` | `Int` | Get the current value (0 if undefined) |
| `gain_resource("name", amount)` | `Int` | Add `amount`, returns new total |
| `try_spend_resource("name", amount)` | `Bool` | If enough, deduct and return `True`; else `False` |

`get_resource` is a query (instant). `gain_resource` and `try_spend_resource` are instant effects — they mutate world state without consuming the tick.

```python
if try_spend_resource("souls", 3):
    raise()
    print("Souls remaining:", get_resource("souls"))
else:
    print("Not enough souls!")

gain_resource("souls", 1)
```

---

## Available Commands

The `[initial].commands` list controls which GrimScript game commands are unlocked at game start.

```toml
[initial]
commands = ["help", "raise", "harvest", "pact"]
```

### Always Available (Stdlib)

These are always available regardless of `[initial].commands`:

`print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`, `percent`, `scale`

### Gatable Game Commands

| Command | Type | Args | Description |
|---------|------|------|-------------|
| `move(pos)` | Action | 1 | Move toward a position |
| `attack(entity)` | Action | 1 | Attack a target entity |
| `flee(entity)` | Action | 1 | Move away from a threat |
| `wait()` | Action | 0 | Skip the current tick |
| `set_target(entity)` | Action | 1 | Set combat target |
| `scan("type")` | Query | 1 | Find all entities of a type (returns list) |
| `nearest("type")` | Query | 1 | Find nearest entity of a type |
| `distance(a, b)` | Query | 2 | Get distance between two entities |
| `get_pos(entity)` | Query | 1 | Get entity position |
| `get_health(entity)` | Query | 1 | Get entity health |
| `get_shield(entity)` | Query | 1 | Get entity shield |
| `get_type(entity)` | Query | 1 | Get entity type string |
| `get_name(entity)` | Query | 1 | Get entity name |
| `get_owner(entity)` | Query | 1 | Get entity owner (EntityRef or None) |
| `get_target(entity)` | Query | 1 | Get current target (EntityRef or None) |
| `has_target(entity)` | Query | 1 | Check if target is set (Bool) |
| `get_resource("name")` | Query | 1 | Get a global resource value |
| `get_stat(entity, "name")` | Query | 2 | Get any stat from an entity (0 if undefined) |
| `get_types(entity)` | Query | 1 | Get all type tags as a list of strings |
| `has_type(entity, "name")` | Query | 2 | Check if entity has a type tag (Bool) |
| `gain_resource("name", amount)` | Instant | 2 | Add to a global resource (returns new total) |
| `try_spend_resource("name", amount)` | Instant | 2 | Spend a global resource if sufficient (returns Bool) |

**Type legend:**
- **Action** — consumes the tick (the entity yields after calling it)
- **Query** — instant, does not consume the tick (returns a value)
- **Instant** — mutates world state without consuming the tick

Some queries accept implicit `self` when called with 0 args: `get_pos()`, `get_health()`, `get_shield()`, `get_target()`, `has_target()`.

Custom commands defined via `[[commands.definitions]]` are also gated by `[initial].commands`. In dev mode (`--features dev-mode`), all commands are available.

### Command Capability Gating

Commands are gated on **two levels**:

1. **Global Unlock** (`[initial].commands`) — Progression gate. A command must be in the global unlock list to be usable by any entity.
2. **Type Capability** (`commands` on `[[types]]`) — Per-entity gate. If a type defines `commands`, its entities can only use those commands.

An entity's effective commands = union of all its types' `commands` lists ∩ globally unlocked commands.

If no types define `commands`, all globally unlocked commands are available (backward compatibility).

In dev mode (`--features dev-mode`), both gates are bypassed — all commands are available to all entities.

---

## Initial Effects

The `[initial]` section consolidates all game-start configuration. Effects from all mods are merged in load order and resolved against the first entity (typically the summoner).

```toml
[initial]
commands = ["raise", "harvest"]
resources = ["souls"]
effects = [
  { type = "output", message = "Welcome to the void..." },
  { type = "modify_stat", target = "self", stat = "health", amount = 10 },
]
```

Any effect type can be used. A `use_resource` or `use_global_resource` failure aborts remaining initial effects.

---

## Custom Command Definitions

Mods define new commands with data-driven effects using `[[commands.definitions]]`. Custom commands are always **actions** — they consume a tick when executed.

```toml
[[commands.definitions]]
name = "drain"
description = "Drain life from target"
unlisted = false                # Hidden from list_commands output (default: false)
args = ["target"]               # Positional argument names
effects = [
  { type = "damage", target = "arg:target", amount = 20 },
  { type = "heal", target = "self", amount = 10 },
  { type = "output", message = "[drain] Life drained!" },
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | — | Command name (used in GrimScript as `name()`) |
| `description` | No | `""` | Shown in `list_commands` output |
| `unlisted` | No | `false` | If true, hidden from `list_commands` |
| `args` | No | `[]` | Positional argument names |
| `effects` | No | `[]` | Instant effects (mutually exclusive with `phases`) |
| `phases` | No | `[]` | Multi-tick phases (mutually exclusive with `effects`) |

Commands use either `effects` (instant, single-tick) or `phases` (multi-tick channeling). They are mutually exclusive — validated at load time.

### Resource Costs

Resource costs are expressed as `use_resource` or `use_global_resource` effects. Place them before the effects they gate — if the check fails, remaining effects are skipped:

```toml
effects = [
  { type = "use_resource", stat = "mana", amount = 30 },   # Entity stat cost
  { type = "use_global_resource", resource = "souls", amount = 5 },  # Global resource cost
  { type = "spawn", entity_id = "skeleton", offset = 1 },
  { type = "output", message = "[raise] A skeleton rises!" },
]
```

`use_resource` checks an entity stat on the caster. `use_global_resource` checks a world-level resource. Both abort the command if insufficient.

### Base Game Commands

The base game commands (`help`, `trance`, `raise`, `harvest`, `pact`) are defined in `mods/core/mod.toml` using the same `[[commands.definitions]]` system as any custom mod command.

---

## Effect Types

Effects are resolved in order when a command executes, a trigger fires, or a buff ticks. All integer fields (`amount`, `offset`, `per_kill`) support [DynInt](#computed-values-dynint) values.

| Effect | Fields | Description |
|--------|--------|-------------|
| `output` | `message` | Print a message to the console. Supports `<hl>text</hl>` for highlighted text. |
| `damage` | `target`, `amount` | Deal damage to a target (shield absorbs first). Kills the entity if health reaches 0. |
| `heal` | `target`, `amount` | Restore health (clamped to `max_health`). |
| `spawn` | `entity_id`, `offset` | Spawn entity at caster's position + offset. Entity plays spawn animation and can't act until it finishes. Owner is set to the caster. |
| `modify_stat` | `target`, `stat`, `amount` | Add to a stat (can be negative). Clamped to `[0, max_{stat}]` if a max exists, else `[0, +inf)`. |
| `use_resource` | `stat`, `amount` | Check and deduct an entity stat from the caster. **Aborts** remaining effects if insufficient. |
| `modify_resource` | `resource`, `amount` | Add to a global resource (clamped to cap if capped). Can be negative. |
| `use_global_resource` | `resource`, `amount` | Check and deduct a global resource. **Aborts** remaining effects if insufficient. |
| `list_commands` | *(none)* | Emit all registered commands and descriptions (in unlock order, excluding `unlisted` commands). |
| `animate` | `target`, `animation` | Trigger a sprite animation on the target entity (e.g., `"cast"`, `"attack"`). |
| `if` | `condition`, `then`, `else` | Evaluate a [condition](#condition-types) and run one of two effect lists. `else` is optional. Supports nesting. |
| `start_channel` | `phases` | Start a [phased channel](#conditional-phase-branching-start_channel) from within an effect list. Remaining effects are skipped. |
| `apply_buff` | `target`, `buff`, `duration` (opt) | Apply a [buff](#buffs--modifiers). Duration overrides the buff's default if specified. |
| `remove_buff` | `target`, `buff` | Remove a buff, reversing all modifiers and running `on_expire` effects. |

### Abort Propagation

When `use_resource` or `use_global_resource` fails:
- Remaining effects in the current list are skipped
- If inside an `if` branch, the entire command aborts (not just the branch)
- If inside a phased command's `on_start` or `per_update`, the channel is cancelled

---

## Computed Values (DynInt)

All integer fields in effects (`amount`, `offset`, `per_kill`) accept computed values:

| Format | Description | Example |
|--------|-------------|---------|
| `42` | Fixed integer | `amount = 42` |
| `"rand(min,max)"` | Random value in [min, max] inclusive | `amount = "rand(5,15)"` |
| `"entity_count(type)"` | Count of alive, ready entities of a type | `amount = "entity_count(skeleton)"` |
| `"resource(name)"` | Current value of a global resource | `amount = "resource(mana)"` |
| `"stat(name)"` | Caster's stat value | `amount = "stat(health)"` |

### Multiplier

All computed values support a `*N` multiplier suffix:

```toml
amount = "entity_count(skeleton)*2"    # 2x skeleton count
amount = "resource(mana)*3"            # 3x mana value
amount = "stat(attack_damage)*2"       # 2x caster's attack damage
```

Randomness is deterministic — seeded from tick number + entity ID. Same seed always produces the same result.

### Examples

```toml
# Damage based on skeleton count
{ type = "damage", target = "arg:target", amount = "entity_count(skeleton)*5" }

# Heal based on current mana
{ type = "heal", target = "self", amount = "resource(mana)" }

# Spawn offset based on a stat
{ type = "spawn", entity_id = "skeleton", offset = "stat(summon_power)*10" }

# Random damage
{ type = "damage", target = "arg:target", amount = "rand(10,25)" }
```

---

## Target Resolution

The `target` field in effects determines which entity is affected.

### Standard Targets

Available in all effects:

| Target | Resolves to |
|--------|-------------|
| `"self"` | The entity executing the command |
| `"arg:<name>"` | Entity reference from a command argument (matched by position: first arg = index 0) |
| `"arg:<index>"` | Entity reference from a command argument by numeric index |

```toml
# self — the caster
{ type = "heal", target = "self", amount = 20 }

# arg — first argument passed to the command
args = ["target"]
effects = [
  { type = "damage", target = "arg:target", amount = 30 },
]
```

### Scoped Targets (Trigger Effects)

Additional targets available in **trigger effects** and **conditions**. They resolve to event participants:

| Target | Resolves to | Available in |
|--------|-------------|--------------|
| `"source"` | The event subject (entity that died, was damaged, or spawned) | All entity events |
| `"owner"` | Owner of the source entity (fallback to entity's stored `owner` field) | All events |
| `"attacker"` | Entity that dealt the damage | `entity_damaged` triggers |
| `"killer"` | Entity that dealt the killing blow | `entity_died` triggers |

Scoped targets that don't apply to the current event (e.g., `"attacker"` in a `tick_interval` trigger) silently no-op — the effect is skipped without error.

```toml
# Reward the killer when a skeleton dies
[[triggers]]
event = "entity_died"
filter = { entity_type = "skeleton" }
effects = [
  { type = "heal", target = "owner", amount = 10 },
  { type = "modify_stat", target = "killer", stat = "xp", amount = 5 },
]

# Punish the attacker when any entity takes damage
[[triggers]]
event = "entity_damaged"
effects = [
  { type = "modify_stat", target = "attacker", stat = "rage", amount = 1 },
]
```

Scoped targets are validated at load time. They are accepted in trigger effect `target` fields and in condition `target` fields (`is_alive`, `distance`). They are **not** valid in regular command effects (only `"self"` and `"arg:<ref>"`).

---

## Condition Types

Conditions are used in `if` effects and trigger `conditions` to gate execution based on game state.

### Basic Conditions

| Type | Fields | Evaluates |
|------|--------|-----------|
| `resource` | `resource`, `compare`, `amount` | Global resource value vs threshold |
| `entity_count` | `entity_type`, `compare`, `amount` | Count of alive, ready entities of a type vs threshold |
| `stat` | `stat`, `compare`, `amount` | Caster's stat value vs threshold (alias: `custom_stat`) |
| `has_buff` | `buff` | Whether the caster has a specific active buff |
| `random_chance` | `percent` | Deterministic random check: fires if roll < percent (1–100) |

### Target-Bearing Conditions

| Type | Fields | Evaluates |
|------|--------|-----------|
| `is_alive` | `target` | Whether the target entity exists and is alive. Returns `false` if target can't be resolved. |
| `distance` | `target`, `compare`, `amount` | Absolute distance from caster to target vs threshold. Returns `false` if target can't be resolved. |

The `target` field accepts all target strings: `"self"`, `"arg:name"`, `"source"`, `"owner"`, `"attacker"`, `"killer"`.

### Compound Conditions

| Type | Fields | Evaluates |
|------|--------|-----------|
| `and` | `conditions` | All sub-conditions must be true |
| `or` | `conditions` | At least one sub-condition must be true |

Compound conditions support arbitrary nesting.

### Compare Operators

Used in `resource`, `entity_count`, `stat`, and `distance` conditions:

`eq`, `ne`, `gt`, `gte`, `lt`, `lte`

The `amount` field supports [DynInt](#computed-values-dynint) values.

### Examples

```toml
# Check global resource
{ type = "resource", resource = "mana", compare = "gte", amount = 20 }

# Count alive entities
{ type = "entity_count", entity_type = "skeleton", compare = "lt", amount = 5 }

# Check caster stat
{ type = "stat", stat = "armor", compare = "gte", amount = 10 }

# Check buff
{ type = "has_buff", buff = "rage" }

# 25% chance
{ type = "random_chance", percent = 25 }

# Check if target is alive
{ type = "is_alive", target = "arg:target" }
{ type = "is_alive", target = "source" }

# Check distance to target
{ type = "distance", target = "arg:target", compare = "lte", amount = 5 }

# Compound: AND
{ type = "and", conditions = [
  { type = "resource", resource = "mana", compare = "gte", amount = 20 },
  { type = "entity_count", entity_type = "skeleton", compare = "lt", amount = 5 },
]}

# Compound: OR
{ type = "or", conditions = [
  { type = "has_buff", buff = "rage" },
  { type = "stat", stat = "health", compare = "lt", amount = 20 },
]}
```

---

## Conditional Effects

The `if` effect branches on game state:

```toml
effects = [
  { type = "if",
    condition = { type = "resource", resource = "mana", compare = "gte", amount = 20 },
    then = [
      { type = "spawn", entity_id = "skeleton", offset = "rand(-100,100)" },
    ],
    else = [
      { type = "output", message = "Not enough mana..." },
    ]
  }
]
```

The `else` branch is optional (defaults to empty).

### Nested Conditions

```toml
effects = [
  { type = "if",
    condition = { type = "resource", resource = "mana", compare = "gte", amount = 10 },
    then = [
      { type = "if",
        condition = { type = "entity_count", entity_type = "skeleton", compare = "gt", amount = 0 },
        then = [
          { type = "output", message = "Has mana and skeletons!" },
        ],
      },
    ],
  }
]
```

### Abort Propagation

If `use_resource` or `use_global_resource` inside a branch fails, the **entire command** aborts — not just the branch. Effects after the `if` are also skipped.

---

## Phased Commands

Commands can use `phases` instead of `effects` to create multi-tick abilities with distinct stages. `effects` and `phases` are mutually exclusive (validated at load time).

### Schema

```toml
[[commands.definitions]]
name = "fireball"
description = "Channel a fireball"
args = ["target"]
phases = [
  # Phase 0: windup (interruptible)
  { ticks = 5, interruptible = true, per_update = [
    { type = "use_resource", stat = "mana", amount = 10 },
  ], on_start = [
    { type = "output", message = "[fireball] Channeling..." },
    { type = "animate", target = "self", animation = "cast" },
  ]},
  # Phase 1: impact (non-interruptible)
  { ticks = 1, interruptible = false, on_start = [
    { type = "damage", target = "arg:target", amount = 50 },
    { type = "output", message = "[fireball] Impact!" },
  ]},
  # Phase 2: recovery (non-interruptible)
  { ticks = 4, interruptible = false, on_start = [
    { type = "output", message = "[fireball] Recovering..." },
  ]},
]
```

### Phase Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `ticks` | Yes | — | Duration in ticks (must be > 0) |
| `interruptible` | No | `false` | Whether the entity's script runs during this phase |
| `on_start` | No | `[]` | Effects that run on the first tick of the phase |
| `per_update` | No | `[]` | Effects that run on update ticks |
| `update_interval` | No | `1` | Run `per_update` every N ticks (must be > 0) |

### `per_update` Timing

`per_update` effects fire when `(ticks_elapsed + 1) % update_interval == 0`:

| `update_interval` | Fires at ticks (0-indexed within phase) |
|---|---|
| 1 | 0, 1, 2, 3, 4, ... (every tick) |
| 2 | 1, 3, 5, 7, ... (every other tick) |
| 3 | 2, 5, 8, 11, ... (every 3rd tick) |

### Execution Model

1. **Initiation tick:** Script calls the command → channel is set up on the entity. The action is consumed; no effects run this tick.
2. **Phase ticks (next tick onward):** Before normal script execution, the tick loop processes active channels:
   - First tick of a phase: `on_start` effects run, then `per_update` fires if it's an update tick.
   - Subsequent ticks: `per_update` fires on update ticks.
   - When all ticks of a phase complete, the next phase begins.
3. **Channel complete:** All phases done → entity resumes normal script execution next tick.

### Interruption

During **interruptible** phases:
- The entity's script runs each tick alongside the channel.
- If the script yields a real action (move, attack, flee, or another command), the channel is **cancelled** and a `[<command>] interrupted` message is emitted.
- If the script yields `wait` or halts, the channel continues.

During **non-interruptible** phases:
- The entity's script does not run at all — the entity is locked into the channel.

### Cancellation

| Case | Behavior |
|------|----------|
| `use_resource` fails mid-phase | Channel cancelled, `"not enough <stat>"` emitted |
| Entity dies mid-channel | Dead entities don't tick, channel abandoned |
| Target dies mid-channel | Effects targeting it become no-ops |
| Script hot-reload during channel | Channel cleared along with script state |
| Both `effects` and `phases` set | Warning at load time; `phases` takes precedence |

### Timing Example

For the fireball above (5 + 1 + 4 = 10 phase ticks):

| Tick | What happens |
|------|-------------|
| N | Script calls `fireball(target)` → channel set up |
| N+1 to N+5 | Phase 0: windup (interruptible, drains 10 mana per update) |
| N+6 | Phase 1: impact (50 damage, non-interruptible) |
| N+7 to N+10 | Phase 2: recovery (non-interruptible) |
| N+11 | Script resumes normally |

---

## Conditional Phase Branching (`start_channel`)

The `start_channel` effect initiates a phased channel from within an effect list. Combined with `if`, this enables conditional phase selection:

```toml
effects = [
  { type = "if",
    condition = { type = "resource", resource = "mana", compare = "gte", amount = 20 },
    then = [
      { type = "use_global_resource", resource = "mana", amount = 20 },
      { type = "start_channel", phases = [
        { ticks = 12, on_start = [{ type = "animate", target = "self", animation = "cast" }] },
        { ticks = 18, on_start = [{ type = "spawn", entity_id = "skeleton", offset = "rand(-300,300)" }] },
      ]},
    ],
    else = [
      { type = "output", message = "Not enough mana!" },
    ]
  }
]
```

`start_channel` can also be used without `if`:

```toml
effects = [
  { type = "use_global_resource", resource = "mana", amount = 20 },
  { type = "start_channel", phases = [
    { ticks = 12, on_start = [{ type = "animate", target = "self", animation = "cast" }] },
  ]},
]
```

**Behavior:**
- Effects before `start_channel` run normally.
- When `start_channel` is reached, remaining effects are skipped and the channel begins.
- Phase definitions use the same schema as top-level `phases`.
- `start_channel` inside an already-active channel is ignored — entities can only have one active channel.

---

## Event Triggers

Triggers are event-driven rules that fire effects when game events occur.

### Schema

```toml
[[triggers]]
event = "entity_died"                    # Required: event type
filter = { entity_type = "skeleton" }    # Optional: narrow which events match
conditions = [                           # Optional: gate firing on world state
  { type = "entity_count", entity_type = "skeleton", compare = "eq", amount = 0 },
]
effects = [                              # Optional: effects to run
  { type = "output", message = "All skeletons have fallen!" },
]
```

### Event Types

| Event | Description | Filter Fields | Scoped Targets |
|-------|-------------|---------------|----------------|
| `entity_died` | An entity was killed | `entity_type` | `source`, `owner`, `killer` |
| `entity_spawned` | An entity was spawned | `entity_type` | `source`, `owner` |
| `entity_damaged` | An entity took damage | `entity_type` | `source`, `owner`, `attacker` |
| `resource_changed` | A global resource value changed | `resource` | *(none)* |
| `command_used` | A custom command was used | `command` | *(none)* |
| `tick_interval` | Fires every N ticks | `interval` (required, > 0) | *(none)* |
| `channel_completed` | A phased channel finished | `command` | *(none)* |
| `channel_interrupted` | A phased channel was interrupted | `command` | *(none)* |

### Filter Fields

Filters narrow which events match. They are type-specific:

```toml
# Only skeleton deaths
filter = { entity_type = "skeleton" }

# Only mana changes
filter = { resource = "mana" }

# Only "raise" command
filter = { command = "raise" }

# Every 300 ticks (10 seconds at 30 TPS)
filter = { interval = 300 }
```

### `resource_changed` Detection

`resource_changed` uses snapshot comparison: resource values at tick start are compared to values after all actions resolve. A trigger fires only if the named resource's value actually changed during the tick.

### Execution Model

1. Triggers are processed once at **end of tick**, after all actions are resolved and pending spawns/despawns are flushed.
2. Each trigger is checked against every matching event from the tick.
3. If multiple events match (e.g., 3 skeletons die), the trigger fires **once per matching event**.
4. Trigger effects do **not** re-trigger other triggers within the same tick (no cascading).
5. Effects from triggers that modify world state (spawning, resources) take effect immediately and are visible to subsequent trigger condition checks within the same tick.
6. Trigger effects resolve against the first alive entity in the world (typically the summoner) as the "caster."

### Examples

```toml
# Spawn wave every 300 ticks
[[triggers]]
event = "tick_interval"
filter = { interval = 300 }
effects = [
  { type = "spawn", entity_id = "skeleton", offset = "rand(-200,200)" },
  { type = "output", message = "A new wave approaches!" },
]

# Gain bones when skeletons die
[[triggers]]
event = "entity_died"
filter = { entity_type = "skeleton" }
effects = [
  { type = "modify_resource", resource = "bones", amount = 1 },
]

# Alert when mana is full
[[triggers]]
event = "resource_changed"
filter = { resource = "mana" }
conditions = [
  { type = "resource", resource = "mana", compare = "gte", amount = 100 },
]
effects = [
  { type = "output", message = "Mana is full!" },
]

# React to raise command
[[triggers]]
event = "command_used"
filter = { command = "raise" }
effects = [
  { type = "output", message = "The earth trembles..." },
]

# Heal the owner when a spawned skeleton dies
[[triggers]]
event = "entity_died"
filter = { entity_type = "skeleton" }
effects = [
  { type = "heal", target = "owner", amount = 10 },
  { type = "modify_resource", resource = "bones", amount = "rand(1,3)" },
]
```

---

## Buffs / Modifiers

Buffs are temporary stat modifiers with automatic expiry, per-tick effects, and lifecycle hooks.

### Definition

```toml
[[buffs]]
name = "rage"
duration = 60               # Default duration in ticks
stackable = true
max_stacks = 3
[buffs.modifiers]
attack_damage = 5           # +5 attack_damage while active
speed = 2                   # +2 speed while active

[[buffs]]
name = "regen"
duration = 30
per_tick = [
  { type = "modify_stat", target = "self", stat = "health", amount = 1 },
]
on_apply = [
  { type = "output", message = "Regeneration begins." },
]
on_expire = [
  { type = "output", message = "Regeneration faded." },
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | — | Unique buff identifier |
| `duration` | No | `0` | Default duration in ticks |
| `modifiers` | No | `{}` | Stat modifiers applied while active (stat name → amount) |
| `per_tick` | No | `[]` | Effects that run each tick while active |
| `on_apply` | No | `[]` | Effects that run when the buff is first applied |
| `on_expire` | No | `[]` | Effects that run when the buff expires or is removed |
| `stackable` | No | `false` | Whether multiple applications stack |
| `max_stacks` | No | `0` | Maximum stacks (0 = unlimited, only relevant if stackable) |

### Modifiers

Modifiers work with **any stat name**. They are applied additively when the buff is applied and reversed when it expires.

Special handling for `health` and `shield`:
- `health` modifier adjusts `max_health` and clamps current health to `[1, max_health]` (prevents buff-expiry death)
- `shield` modifier adjusts `max_shield` and clamps current shield to `[0, max_shield]`
- All other stats are modified directly and clamped to `[0, +inf)`

Modifier stat names are validated at load time against known stats from entity configs.

### Apply/Remove Effects

```toml
effects = [
  { type = "apply_buff", target = "self", buff = "rage" },
  { type = "apply_buff", target = "self", buff = "regen", duration = 100 },  # Override duration
  { type = "remove_buff", target = "self", buff = "rage" },
]
```

### Stacking Behavior

- **Non-stackable** (`stackable = false`): Re-applying refreshes the duration without adding additional modifiers.
- **Stackable** (`stackable = true`): Each application adds a stack (up to `max_stacks` if set), applying modifiers again. All stacks share the same duration timer (refreshed on each application). At max stacks, further applications are ignored.

### Buff Lifecycle

1. **Apply**: Modifiers are applied to entity stats, `on_apply` effects run, `ActiveBuff` is tracked on the entity.
2. **Each tick** (step 6b in tick loop): `per_tick` effects run, duration is decremented.
3. **Expire** (duration reaches 0): Modifiers are reversed (all stacks), `on_expire` effects run, buff is removed.
4. **Manual remove** (via `remove_buff` effect): Same as expire — all stacks of modifiers are reversed, `on_expire` effects run, buff is removed.

### `has_buff` Condition

Check if the caster has a specific buff:

```toml
{ type = "if",
  condition = { type = "has_buff", buff = "rage" },
  then = [{ type = "output", message = "Raging!" }],
}
```

---

## Entity Stats

All entity stats live in a single unified map. There are no built-in stats vs custom stats — they're all the same system. Any stat not explicitly defined defaults to 0.

### Defining Stats

```toml
[[entities]]
type = "warrior"
stats = { health = 80, speed = 1, armor = 5, crit_chance = 10, rage = 0 }
```

### Auto-Max Behavior

When `health` or `shield` are defined and no explicit `max_health`/`max_shield` is provided:
- `max_health` is automatically set to the same value as `health`
- `max_shield` is automatically set to the same value as `shield`

### Accessing Stats in Effects

```toml
# Modify any stat
{ type = "modify_stat", target = "self", stat = "armor", amount = -5 }
{ type = "modify_stat", target = "self", stat = "health", amount = 20 }

# Check and deduct any stat from self
{ type = "use_resource", stat = "rage", amount = 10 }

# Condition on any stat
{ type = "stat", stat = "armor", compare = "gte", amount = 10 }
```

For backward compatibility: `modify_custom_stat` → `modify_stat`, `use_custom_stat` → `use_resource`, `custom_stat` → `stat`.

### Accessing Stats in GrimScript

```python
# Via attribute access (dot notation) — works for any stat
my_armor = self.armor
health = self.health
print("Armor:", my_armor)

# Via builtin function
armor = get_stat(self, "armor")
target_armor = get_stat(target, "armor")
```

### Entity Attributes

Entity references support these attribute names via dot notation:

| Attribute | Type | Description |
|-----------|------|-------------|
| `position`, `pos`, `x` | Int | 1D position |
| `name` | Str | Entity name |
| `type` | Str | Entity type string |
| `owner` | EntityRef or None | Owner entity |
| `alive` | Bool | Whether the entity is alive |
| `target` | EntityRef or None | Current combat target |
| *(any other name)* | Int | Looked up in the stats map (0 if undefined) |

```python
enemy = nearest("skeleton")
if enemy is not None:
    print("Type:", enemy.type)
    print("HP:", enemy.health)
    print("Custom:", enemy.armor)
    print("At:", enemy.position)
```

---

## Library Files

Mods can provide `.grim` GrimScript library files whose functions are prepended to player scripts before compilation.

### Schema

```toml
[commands]
libraries = ["lib/utils.grim", "lib/combat.grim"]
```

Paths are relative to the mod directory. Files are loaded in the order listed. Missing files emit a warning and loading continues. Library files are syntax-checked (lex + parse) at mod load time; syntax errors produce a warning.

### Namespace

Flat namespace with first-loaded-wins, consistent with entity types and commands. If two mods define a function with the same name, the first-loaded mod's version is used.

### Gating

Library functions are subject to the same command gating as player scripts. If a library function calls `raise()`, the `raise` command must be in the available set.

### How It Works

1. At mod load time, `.grim` files are read and concatenated into each mod's library source.
2. All mod library sources are concatenated in load order.
3. When a player script is compiled (Run or Console), the combined library source is prepended.
4. The combined source compiles as a single unit — library functions are visible to player code as if defined at the top of the script.

---

## Sprite Format

Sprites use a sprite atlas system: one PNG with all animation frames in a grid, paired with a JSON metadata file.

### Atlas JSON Format

```json
{
  "frame_width": 48,
  "frame_height": 48,
  "animations": [
    {
      "name": "idle",
      "row": 0,
      "frames": [
        { "col": 0, "ticks": 3 },
        { "col": 1, "ticks": 3 },
        { "col": 2, "ticks": 3 },
        { "col": 3, "ticks": 3 }
      ],
      "loop_mode": "loop"
    },
    {
      "name": "walk",
      "row": 1,
      "frames": [
        { "col": 0, "ticks": 3 },
        { "col": 1, "ticks": 3 },
        { "col": 2, "ticks": 3 },
        { "col": 3, "ticks": 3 }
      ],
      "loop_mode": "loop"
    },
    {
      "name": "cast",
      "row": 2,
      "frames": [
        { "col": 0, "ticks": 2 },
        { "col": 1, "ticks": 2 },
        { "col": 2, "ticks": 2 }
      ],
      "loop_mode": "play_once"
    },
    {
      "name": "death",
      "row": 3,
      "frames": [
        { "col": 0, "ticks": 4 },
        { "col": 1, "ticks": 4 },
        { "col": 2, "ticks": 4 }
      ],
      "loop_mode": "play_once"
    },
    {
      "name": "spawn",
      "row": 4,
      "frames": [
        { "col": 0, "ticks": 3 },
        { "col": 1, "ticks": 3 },
        { "col": 2, "ticks": 3 }
      ],
      "loop_mode": "play_once"
    }
  ]
}
```

### Requirements

- **`idle` animation is required** — the engine starts on it and returns to it after `play_once` animations finish.
- `frame_width` and `frame_height` must match the grid cell size in the PNG.
- `row` is 0-indexed in the atlas, `col` is 0-indexed.
- `ticks` is how many sim ticks (at 30 TPS, 1 tick ≈ 33ms) each frame lasts.
- `loop_mode`: `"loop"` repeats forever, `"play_once"` plays through then returns to idle.
- Animations are sim-driven and deterministic — they advance exactly once per sim tick.

### Special Animations

| Animation | Purpose |
|-----------|---------|
| `idle` | **Required.** Default animation. |
| `spawn` | Plays automatically when spawned. Entity can't act or be targeted until it finishes. Duration determines `spawn_ticks_remaining`. |
| `death` | Plays when entity dies. Entity is removed from rendering after it finishes. |
| `walk` | Used during movement. |
| `cast`, `attack`, etc. | Triggered by the `animate` effect. |

Unknown animation names in `animate` effects are logged as warnings.

### Pivot Point

The `pivot` field in `[[entities]]` controls the sprite anchor:

```toml
pivot = [24.0, 0.0]    # [x, y] offset from top-left of frame
```

The sprite is positioned so the pivot sits at the entity's world position.

### File Naming

Reference sprites without extension — the engine looks for both `.png` and `.json`:

```toml
sprite = "sprites/warrior_atlas"
# Loads: sprites/warrior_atlas.png + sprites/warrior_atlas.json
```

---

## Multiple Mods

Multiple mods can be active simultaneously. Content from all mods is merged:

- **Entity types**: Merged into a shared registry. First-loaded-wins with warning on duplicates.
- **Commands**: Merged into a shared registry. First-loaded-wins with warning on duplicates.
- **Resources**: Merged. First-defined-wins with warning on duplicates.
- **Buffs**: Merged. First-defined-wins with warning on duplicates.
- **Triggers**: All triggers from all mods are collected in load order.
- **Spawns**: All `[[spawn]]` entries from all mods execute.
- **Initial commands/resources**: Merged from all mods' `[initial]` sections.
- **Initial effects**: Merged in load order.
- **Library files**: Concatenated in load order.

Load order is determined by the dependency graph (topological sort, alphabetical tie-breaking).

---

## Validation

After all mods are loaded, the engine validates:

### Entity/Spawn Validation
- `[[spawn]]` entity types must match registered entity types
- `spawn` effect entity types are checked against known types (recursively through `if` branches and `start_channel` phases)
- Entities with multiple brain types are **rejected** and removed from all registries (configs, types, sprites, pivots) — they will not load or spawn

### Command Validation
- `effects` and `phases` are mutually exclusive (warning; `phases` takes precedence)
- Phase `ticks` must be > 0
- Phase `update_interval` must be > 0
- `use_resource` amounts (fixed values) must be positive

### Target Validation
- `target` fields must be `"self"`, `"arg:<name>"` (matching a declared arg), `"arg:<index>"` (within args bounds), or a scoped target in trigger contexts
- Invalid targets produce a warning

### Condition Validation
- Resource name, entity_type, stat, buff names must be non-empty
- `random_chance` percent must be 1–100
- `is_alive` and `distance` targets must be non-empty
- Compound conditions (`and`/`or`) are validated recursively

### Trigger Validation
- Event names must be one of the 8 supported types
- `tick_interval` must have a positive `interval` filter
- Conditions and effects are validated with the same checks as command conditions/effects

### Buff Validation
- Buff names must be non-empty
- Buff duration must be > 0
- Modifier stat names are checked against known stats from entity configs
- `per_tick`, `on_apply`, `on_expire` effects are validated

### Library Validation
- `.grim` files are syntax-checked (lex + parse) at load time
- Syntax errors produce a warning (source is still prepended for graceful degradation)

### Resource Validation
- Warning if initial value exceeds defined max cap

All validation failures produce warnings (not errors) — the mod still loads with best-effort behavior.

---

## Runtime Entity Spawning

When the simulation spawns entities at runtime (via `spawn` effects), the engine:

1. Looks up the entity type in the sprite registry to create a render unit
2. Applies `EntityConfig` (stats) from the entity type definition
3. Sets `owner` to the entity that spawned it
4. Starts a **spawn state**: the entity plays its `spawn` animation (if present) and can't act or be targeted by queries until it finishes

Entities without a `spawn` animation are immediately ready. If no sprite data exists for the type, the sim entity is created but has no visible sprite.

Runtime-spawned entities use `"{type}_{entity_id}"` as their name (guaranteed unique via monotonically increasing entity IDs).

---

## Fallback Behavior

If `mods/` doesn't exist or contains no valid mods, nothing loads — no entities, commands, or resources will be available.

---

## The Base Game Mod

`mods/core/` is the base game, structured as a mod:

```
mods/core/
  mod.toml
  sprites/
    summoner_atlas.png
    summoner_atlas.json
    skeleton_atlas.png
    skeleton_atlas.json
```

Its `mod.toml` defines:
- The `summoner` entity type (100 HP, speed 1) — the player-controlled entity that runs scripts
- The `skeleton` entity type (5 HP, inherits speed from `unit` type)
- The summoner spawn at position 500
- Global resources: `mana` (50/100 capped), `bones` (0, capless)
- Initial commands: `help`, `trance`, `raise`, `harvest`, `pact`
- Initial resources: `mana`, `bones`
- Startup messages via initial effects
- Five custom commands: `help` (list commands), `trance` (mana regen channel), `raise` (spawn skeleton), `harvest` (phased channel), `pact` (self-damage)

---

## Creating a New Mod

1. Create a directory under `mods/`:
   ```
   mods/my-mod/
   ```

2. Create `mod.toml` with at minimum:
   ```toml
   [mod]
   id = "my-mod"
   name = "My Mod"
   version = "0.1.0"
   ```

3. (Optional) Add entity definitions with sprite assets:
   - Create your sprite atlas PNG (grid of animation frames)
   - Write the JSON metadata describing frame layout
   - Reference them in `[[entities]]`

4. (Optional) Add spawn effects in `[initial].effects` for entities present at game start.

5. (Optional) Add resources, commands, triggers, buffs.

6. (Optional) Add `[initial]` to control which commands and resources are unlocked.

7. Run the game — your mod is loaded automatically.

### Depending on Another Mod

```toml
[mod]
id = "my-expansion"
name = "My Expansion"
version = "0.1.0"
depends_on = ["core"]
```

Your mod loads after `core` and can reference its entity types, resources, and commands.

---

## GrimScript API Reference

### Data Types

| Type | Description | Examples |
|------|-------------|---------|
| `Int` | 64-bit integer (no floats in sim) | `42`, `-7` |
| `Bool` | Boolean | `True`, `False` |
| `Str` | String | `"hello"` |
| `None` | Null value | `None` |
| `List` | Ordered collection | `[1, 2, 3]` |
| `Dict` | Ordered key-value map (deterministic iteration) | `{"a": 1, "b": 2}` |
| `EntityRef` | Reference to a sim entity | *(returned by queries)* |

### Stdlib (Always Available)

| Function | Description |
|----------|-------------|
| `print(value, ...)` | Print values to the console |
| `len(collection)` | Length of list, string, or dict |
| `range(stop)` / `range(start, stop)` / `range(start, stop, step)` | Generate a list of integers |
| `abs(n)` | Absolute value |
| `min(a, b)` | Minimum of two values |
| `max(a, b)` | Maximum of two values |
| `int(value)` | Convert to integer |
| `str(value)` | Convert to string |
| `type(value)` | Get type name as string |
| `percent(value, pct)` | `value * pct / 100` with banker's rounding |
| `scale(value, num, den)` | `value * num / den` with banker's rounding |

### Operators

| Operator | Description |
|----------|-------------|
| `+`, `-`, `*`, `//`, `%` | Arithmetic (floor division/modulo, Python-style) |
| `==`, `!=`, `<`, `>`, `<=`, `>=` | Comparison |
| `and`, `or`, `not` | Boolean logic |
| `in`, `not in` | Membership (lists, strings, dict keys) |
| `-x` | Unary negate |
| `x is None`, `x is not None` | None checks |

Division and modulo use Python-style floor semantics: `-7 // 2 = -4`, `-7 % 2 = 1`.

### Control Flow

```python
# If/elif/else
if condition:
    ...
elif other:
    ...
else:
    ...

# While loop
while condition:
    ...

# For loop
for item in collection:
    ...

# Break and continue
for x in range(10):
    if x == 5:
        break
    if x % 2 == 0:
        continue
    print(x)
```

### Functions

```python
def my_function(a, b):
    return a + b

result = my_function(3, 4)
```

### Collections

```python
# Lists
my_list = [1, 2, 3]
my_list.append(4)
first = my_list[0]
length = len(my_list)

# Dicts (deterministic insertion-order iteration)
my_dict = {"a": 1, "b": 2}
val = my_dict["a"]
my_dict["c"] = 3
keys = my_dict.keys()
values = my_dict.values()
items = my_dict.items()
got = my_dict.get("missing", 0)

# Iteration
for key in my_dict:
    print(key, my_dict[key])
```

### `self` Variable

In scripts, `self` is pre-allocated at variable slot 0 as an `EntityRef` for the executing entity (the entity running the script):

```python
my_pos = get_pos(self)
# or
my_pos = self.position

my_hp = get_health(self)
# or
my_hp = self.health
```

### Script Execution

- Scripts compile to IR and execute in a tick-based loop
- **Actions** (move, attack, flee, wait, custom commands) consume the tick — the script pauses until next tick
- **Queries** (scan, nearest, get_health, etc.) are instant — the script continues
- **Instant effects** (gain_resource, try_spend_resource) mutate world state without consuming the tick
- 10,000 instruction step limit per tick — exceeding it emits a warning and auto-yields with wait
- `print()` does not consume the tick

---

## Adding New Effects (Developer Guide)

### Effects vs Builtins

| | Effect path | Builtin path |
|---|---|---|
| **What** | New `CommandEffect` variant for `mod.toml` | New function scripts call directly |
| **Files** | 1–2 Rust files | 5 files |
| **Use when** | Mechanic modifies world state as part of a command | Mechanic returns a value to the script or needs computed arguments |
| **Examples** | `damage`, `heal`, `spawn`, `modify_stat` | `nearest`, `scan`, `get_health`, `move` |

**Rule of thumb:** If a modder can express it as static TOML values, use the effect system. If the script needs runtime computation or a return value, use a builtin.

### Adding a New Effect

1. Add a variant to `CommandEffect` in `crates/deadcode-sim/src/action.rs`:
   ```rust
   #[serde(rename = "teleport")]
   Teleport { target: String, position: i64 },
   ```

2. Add a match arm in `resolve_effects_inner()` in the same file:
   ```rust
   CommandEffect::Teleport { target, position } => {
       let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
       if let Some(tid) = target_id {
           if let Some(entity) = world.get_entity_mut(tid) {
               entity.position = *position;
           }
       }
   }
   ```

3. (Optional) Add validation in `modding.rs` if the effect has `target`, `stat`, or other fields that can reference invalid things.

4. Use it in `mod.toml`:
   ```toml
   effects = [{ type = "teleport", target = "self", position = 100 }]
   ```

5. Update this document.

### Adding a New Builtin

5 files:

1. `crates/deadcode-sim/src/ir.rs` — add `Instruction` variant
2. `crates/deadcode-sim/src/executor.rs` — execute the instruction
3. `crates/deadcode-sim/src/action.rs` — add `UnitAction` variant if it's an action
4. `crates/deadcode-sim/src/compiler/builtins.rs` — map function name to IR
5. `crates/grimscript-lang/src/builtins.rs` — register the name, set `is_game_builtin()` to `true`

---

## Internals

### Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `ModManifest` | `deadcode-app/modding.rs` | Deserialized `mod.toml` |
| `ModMeta` | `deadcode-app/modding.rs` | Mod metadata (id, name, version, depends_on, conflicts_with) |
| `EntityDef` | `deadcode-app/modding.rs` | Entity type definition (type, sprite, pivot, stats) |
| `SpawnDef` | `deadcode-app/modding.rs` | Initial spawn (type, name, position) |
| `InitialDef` | `deadcode-app/modding.rs` | Initial commands, resources, startup effects |
| `ResourceDef` | `deadcode-app/modding.rs` | Resource definition (value, optional max) |
| `CommandsDef` | `deadcode-app/modding.rs` | Command definitions + library paths |
| `LoadedMod` | `deadcode-app/modding.rs` | Fully resolved mod (sprites, configs, commands, library source) |
| `CommandDef` | `deadcode-sim/action.rs` | Custom command (name, description, args, effects/phases, unlisted) |
| `PhaseDef` | `deadcode-sim/action.rs` | Phase in a multi-tick command |
| `CommandEffect` | `deadcode-sim/action.rs` | Effect type enum (16 variants) |
| `Condition` | `deadcode-sim/action.rs` | Condition enum (9 variants) |
| `CompareOp` | `deadcode-sim/action.rs` | Comparison operators (eq, ne, gt, gte, lt, lte) |
| `DynInt` | `deadcode-sim/action.rs` | Computed integer (fixed, rand, entity_count, resource, stat) |
| `TriggerDef` | `deadcode-sim/action.rs` | Trigger (event, filter, conditions, effects) |
| `TriggerFilter` | `deadcode-sim/action.rs` | Filter fields (entity_type, resource, command, interval) |
| `BuffDef` | `deadcode-sim/action.rs` | Buff definition (name, duration, modifiers, effects, stacking) |
| `EffectContext` | `deadcode-sim/action.rs` | Scoped target context (source, owner, attacker, killer) |
| `EffectOutcome` | `deadcode-sim/action.rs` | Effect resolution result (Complete, Aborted, StartChannel) |
| `EntityConfig` | `deadcode-sim/entity.rs` | Stat overrides applied at spawn |
| `SimEntity` | `deadcode-sim/entity.rs` | Game entity (stats map, owner, position, buffs, channel) |
| `ChannelState` | `deadcode-sim/entity.rs` | Active phased channel state |
| `ActiveBuff` | `deadcode-sim/entity.rs` | Tracked buff on an entity |

### Loading Flow

1. `load_mods()` scans `mods/` for directories with `mod.toml`, sorted alphabetically
2. Each manifest is parsed; sprite files, library files are read from disk
3. `resolve_mod_dependencies()` reorders mods by dependency graph
4. Registries (sprites, pivots, entity configs, commands) are merged into `App`, with collision warnings
5. `validate_spawns()` checks spawn entity types in effects
6. `validate_command_defs()` checks targets, stat names, amounts, phases, conditions
7. `validate_triggers()` checks event names, intervals, conditions, effects
8. `validate_buffs()` checks buff names, durations, modifier stats, effect lists
9. `collect_initial_resources()` merges resources; `collect_available_resources()` determines which are unlocked
10. `collect_triggers()` and `collect_buffs()` gather all triggers/buffs from all mods
11. `collect_library_source()` concatenates library source in load order
12. `[initial].effects` spawn entries create sim entities and render units
13. `[initial].commands` populate available commands; `[initial].effects` run against the summoner
14. Commands, triggers, buffs, resources are registered with `SimWorld`

### Custom Command Flow

1. `CommandDef` is parsed from TOML and collected
2. At sim init, registered via `SimWorld::register_custom_command()` (effects, arg counts, phases)
3. Compiler receives custom command arg counts, emits `ActionCustom(name)` IR
4. Executor pops args and yields `UnitAction::Custom { name, args }`
5. `resolve_action()` checks phases → if yes, creates `ChannelState`; if no, resolves instant effects
6. Phased: tick loop processes channels before script execution — runs effects, handles interruption
7. Command metadata sent to frontend via `AvailableCommands` IPC for autocomplete/highlighting

### Tick Loop

`SimWorld::tick()` processes in this order:

1. Derive per-tick RNG, snapshot entity types and resources for triggers
2. Decrement spawn timers
3. Shuffle ready entity IDs (seeded, deterministic)
4. For each entity: process active channel (if any) or execute script
5. Resolve all tick-consuming actions
6. Tick passive systems (cooldowns)
7. Tick buffs (per_tick effects, duration decrement, expiry handling)
8. Flush pending spawns/despawns
9. Process triggers (match events against registered triggers, check filters/conditions, fire effects)

Simulation runs at fixed 30 TPS. Animations advance once per sim tick.
