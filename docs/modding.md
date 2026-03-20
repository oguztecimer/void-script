# Modding Guide

How to create mods for VOID//SCRIPT. The base game ("Core") is itself a mod — the same system that loads it can load custom content.

## Mod Structure

Each mod is a directory inside `mods/` containing a `mod.toml` manifest and any associated assets:

```
mods/
  my-mod/
    mod.toml                # Required: mod manifest
    sprites/
      warrior_atlas.png     # Sprite sheet PNG
      warrior_atlas.json    # Atlas metadata (frame layout)
```

The game scans `mods/` at startup and loads every directory that contains a valid `mod.toml`. Mods are then reordered by their dependency graph (topological sort, with alphabetical tie-breaking for determinism). If no mods are found, the game falls back to embedded assets (identical to the pre-modding behavior).

### Dependencies and Conflicts

Mods can declare dependencies and conflicts in the `[mod]` section:

- **`depends_on`**: List of mod IDs that must be loaded. Dependencies are loaded first. If a dependency is missing, the mod (and any mods that depend on it) is skipped with a warning.
- **`conflicts_with`**: List of mod IDs that cannot be loaded alongside this mod. If a conflict exists, the first-loaded mod wins and the conflicting mod is skipped with a warning.

Circular dependencies are detected and logged as an error. The affected mods fall back to alphabetical ordering.

## mod.toml Reference

```toml
[mod]
id = "my-mod"           # Unique identifier (lowercase, no spaces)
name = "My Mod"         # Display name
version = "0.1.0"       # Semver version string
depends_on = []         # Mod IDs this mod requires (must be loaded first)
conflicts_with = []     # Mod IDs that cannot be loaded alongside this mod
min_game_version = ""   # Reserved: minimum game version (not enforced yet)

# --- Entity Definitions ---
# Define entity types with sprites and stats.
# Each [[entities]] block registers a type that can be spawned.

[[entities]]
type = "warrior"                    # Entity type string (used in scripts and spawn defs)
sprite = "sprites/warrior_atlas"    # Path to sprite files (relative to mod dir, no extension)
                                    # Expects both .png and .json to exist
pivot = [24.0, 0.0]                 # Sprite pivot point [x, y] for positioning
health = 80                         # Max health (also sets current health)
mana = 60                         # Max mana
speed = 2                           # Movement speed (tiles per tick)
attack_damage = 15                  # Damage per attack
attack_range = 3                    # Attack range in tiles
attack_cooldown = 2                 # Ticks between attacks
shield = 10                         # Max shield (also sets current shield)

# --- Initial Spawns ---
# Entities to place on the strip when the game starts.

[[spawn]]
entity_type = "warrior"    # Must match a type from [[entities]] (in any loaded mod)
name = "warrior"           # Instance name (used for render unit matching)
position = 300             # 1D position on the strip

# --- Initial Effects ---
# Effects that run when the game opens (without loading a save).

[initial]
effects = [
  { type = "output", message = "Welcome..." },
]

# --- Global Resources ---
# World-level integer resources shared across all entities.

[resources]
souls = 0
gold = 100

# --- Initial State ---
# Commands, resources, and effects available/run at game start.

[initial]
commands = ["consult", "raise", "harvest", "pact"]
resources = ["souls"]
effects = [
  { type = "output", message = "Welcome..." },
]
```

All fields in `[[entities]]` except `type` are optional. Stats not explicitly defined default to 0. Omitted `sprite` means the entity won't have a render unit.

**Reserved entity type:** The `"summoner"` entity type is hardcoded by the game engine and cannot be defined or overridden by mods. It is always spawned at position 500 with fixed stats. Mods that define an entity with `type = "summoner"` will see a warning and their definition will be ignored.

## Entity Definitions

Entity definitions register types that can be spawned — either at startup via `[[spawn]]` or at runtime by game actions (e.g., the `raise` command creates skeletons).

### Stats

All entity stats live in a single unified map. There are no built-in defaults — entities start with **no stats** unless the mod defines them. Any stat not explicitly set defaults to 0.

There are two ways to define stats on an entity:

1. **Convenience fields** — top-level fields in `[[entities]]` for common stats:

| Field | Description |
|-------|-------------|
| `health` | Current health (auto-sets `max_health` if not specified) |
| `speed` | Movement speed (tiles per tick) |
| `attack_damage` | Damage dealt per attack |
| `attack_range` | Range for attack actions |
| `attack_cooldown` | Ticks between attacks |
| `shield` | Current shield (auto-sets `max_shield` if not specified) |

2. **`stats` table** — for any stat, including the ones above (aliased as `custom_stats` for backward compat):

```toml
[[entities]]
type = "golem"
health = 200
speed = 1
stats = { armor = 5, crit_chance = 10, attack_damage = 25 }
```

The convenience fields and `stats` table are merged into the same map. If a stat appears in both, the convenience field takes precedence.

All stats are accessible in GrimScript via `entity.stat_name` attribute access and `get_stat(entity, "name")`, and in mod effects via `modify_stat` / `use_resource` / `Condition::stat`.

### Sprite Format

Sprites use a sprite atlas system: one PNG containing all animation frames laid out in a grid, paired with a JSON file describing the frame layout.

**JSON metadata format:**

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
      "name": "attack",
      "row": 2,
      "frames": [
        { "col": 0, "ticks": 2 },
        { "col": 1, "ticks": 2 },
        { "col": 2, "ticks": 2 }
      ],
      "loop_mode": "play_once"
    }
  ]
}
```

**Requirements:**
- An `idle` animation must exist (the engine starts on it and transitions back to it after `play_once` animations finish)
- `frame_width` and `frame_height` must match the grid cell size in the PNG
- `row` is the 0-indexed row in the atlas, `col` is the 0-indexed column
- `ticks` is how many sim ticks (at 30 TPS, 1 tick ≈ 33ms) each frame lasts
- `loop_mode`: `"loop"` repeats forever, `"play_once"` plays through then returns to idle
- Animations are sim-driven and deterministic — they advance exactly once per sim tick
- A `spawn` animation (if present) plays automatically when an entity is spawned; the entity can't act or be targeted until it finishes
- Unknown animation names in `play()` calls are logged as warnings, not crashes

**Pivot point:** The `pivot` field in `[[entities]]` controls where the sprite's anchor point is. `[x, y]` offsets from the top-left corner of the frame. The sprite is positioned so the pivot sits at the entity's world position.

## Spawn Definitions

`[[spawn]]` blocks define entities placed when the game starts. Each needs:

- `entity_type` — must match a `type` from an `[[entities]]` block (can be from any loaded mod)
- `name` — instance name, used to link the sim entity to its render unit
- `position` — 1D integer position on the strip

You can have multiple spawns of the same type:

```toml
[[spawn]]
entity_type = "skeleton"
name = "guard_left"
position = 200

[[spawn]]
entity_type = "skeleton"
name = "guard_right"
position = 800
```

## Available Commands

The `[initial].commands` list controls which GrimScript game commands are unlocked at game start. Commands from all loaded mods are merged.

Stdlib functions (`print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`, `percent`, `scale`) are always available regardless of this setting.

Game commands that can be gated:

| Command | Description |
|---------|-------------|
| `move` | Move toward a position |
| `attack` | Attack a target entity |
| `flee` | Move away from a target |
| `wait` | Skip the current tick |
| `scan` | Find entities by type |
| `nearest` | Find nearest entity of a type |
| `distance` | Get distance to an entity |
| `get_pos` | Get entity position |
| `get_health` | Get entity health |
| `get_mana` | Get entity mana |
| `get_shield` | Get entity shield |
| `get_type` | Get entity type string |
| `get_name` | Get entity name |
| `get_owner` | Get entity owner |
| `set_target` | Set combat target |
| `get_target` | Get current target |
| `has_target` | Check if target is set |
| `get_resource` | Get a global resource value |
| `gain_resource` | Add to a global resource |
| `try_spend_resource` | Spend a global resource if sufficient |

Custom commands defined via `[[commands.definitions]]` are also gated by the `[initial].commands` list. If a command is defined but not in `commands`, players can't use it until it's unlocked at runtime.

In dev mode (`--features dev-mode`), all commands (including custom) are available regardless of the `[initial]` setting.

## Global Resources

Mods define world-level integer resources in a `[resources]` table in `mod.toml`. Resources are shared across all entities — they are not per-entity stats like health or mana.

```toml
[resources]
souls = 0
gold = 100
```

Each key is the resource name, and the value is the initial amount. Resources from all mods are merged at load time (first-defined wins for duplicates, with a warning).

### Resource Availability

Resources have an available/unavailable mechanic mirroring the command availability system. The `[initial].resources` list controls which resources are usable from game start. Calling `get_resource()`, `gain_resource()`, or `try_spend_resource()` on an unavailable resource produces a runtime error.

If `[initial].resources` is omitted or empty, **all defined resources are available by default** (backward compatible with mods that don't use gating).

In dev mode (`--features dev-mode`), all resources are available regardless of the `[initial]` setting.

Available resource names are sent to the frontend via IPC alongside available commands.

### Script API

Three GrimScript builtins interact with global resources:

| Function | Returns | Description |
|----------|---------|-------------|
| `get_resource("name")` | `Int` | Get the current value of a resource (0 if undefined) |
| `gain_resource("name", amount)` | `Int` | Add `amount` to a resource, returns the new total |
| `try_spend_resource("name", amount)` | `Bool` | If the resource has at least `amount`, deduct it and return `True`; otherwise return `False` (no deduction) |

The resource builtin *function names* (`get_resource`, `gain_resource`, `try_spend_resource`) are gated by `[initial].commands` like other game commands. The *resource names* passed as arguments are gated by `[initial].resources` at runtime.

`get_resource` is a query (instant, like `get_health`). `gain_resource` and `try_spend_resource` are instant effects — they mutate world state without consuming the tick. The script continues executing after calling them.

### Example

```python
# Check if we can afford to raise a skeleton
if try_spend_resource("souls", 3):
    raise()
    print("Skeleton raised! Souls remaining:", get_resource("souls"))
else:
    print("Not enough souls!")

# Gain souls from harvesting
gain_resource("souls", 1)
```

## Custom Command Definitions

Mods can define entirely new commands with data-driven effects using `[[commands.definitions]]`. Custom commands are always **actions** — they consume a tick when executed.

```toml
[[commands.definitions]]
name = "drain"
description = "Drain life from target"
unlisted = false                # If true, hidden from list_commands output (default: false)
args = ["target"]               # Argument names (positional)
effects = [
  { type = "damage", target = "arg:target", amount = 20 },
  { type = "heal", target = "self", amount = 10 },
  { type = "output", message = "[drain] Draining life..." },
]

[[commands.definitions]]
name = "summon"
description = "Summon a skeleton at your position"
args = []
effects = [
  { type = "spawn", entity_type = "skeleton", offset = 1 },
  { type = "output", message = "[summon] A skeleton rises!" },
]

[[commands.definitions]]
name = "fortify"
description = "Gain shield"
args = []
effects = [
  { type = "modify_stat", target = "self", stat = "shield", amount = 25 },
  { type = "output", message = "[fortify] Shield raised!" },
]
```

### Effect Types

Effects are resolved in order when the command executes.

| Effect | Fields | Description |
|--------|--------|-------------|
| `output` | `message` | Print a message to the console. Supports `<hl>text</hl>` for highlighted text. |
| `damage` | `target`, `amount` | Deal damage (shield absorbs first) |
| `heal` | `target`, `amount` | Restore health (capped at max) |
| `spawn` | `entity_type`, `offset` | Spawn entity at self.position + offset. Entity plays spawn animation and can't act until it finishes. |
| `modify_stat` | `target`, `stat`, `amount` | Add to a stat (can be negative) |
| `use_resource` | `stat`, `amount` | Check and deduct a resource from self; aborts remaining effects if insufficient |
| `list_commands` | *(none)* | List all registered commands and descriptions (in unlock order) |
| `animate` | `target`, `animation` | Trigger a sprite animation on the target entity (e.g., `"cast"`, `"attack"`) |
| `sacrifice` | `entity_type`, `resource`, `per_kill` | Kill all alive, non-spawning entities of a type and gain `per_kill` of `resource` per kill. Outputs a summary or "Nothing to sacrifice" if none found. |
| `modify_resource` | `resource`, `amount` | Add to a global resource (clamped to cap if capped). Can be negative. |
| `use_global_resource` | `resource`, `amount` | Check and deduct a global resource; aborts remaining effects if insufficient. |
| `if` | `condition`, `then`, `else` | Evaluate a condition and run one of two effect lists. `else` is optional. Supports nesting. |
| `start_channel` | `phases` | Start a phased channel from within an effect list. Remaining effects after `start_channel` are skipped. |

**DynInt fields:** The `amount` and `offset` fields accept either a plain integer or `"rand(min,max)"` for a random value in [min, max] inclusive. Randomness is deterministic (seeded from tick + entity ID).

```toml
{ type = "spawn", entity_type = "skeleton", offset = "rand(50, 150)" }
{ type = "damage", target = "arg:0", amount = "rand(5, 15)" }
```

### Target Resolution

The `target` field in effects uses these formats:
- `"self"` — the entity executing the command
- `"arg:<name>"` — an entity reference passed as a command argument (matched by position: first arg = index 0)

### Stat Names

Any stat name can be used in `modify_stat` and `use_resource` effects — stats are arbitrary strings defined by entity types in `[[entities]]`. Common stats include `health`, `shield`, `speed`, `attack_damage`, `attack_range`, `attack_cooldown`, but mods can define any stat name they want.

### Base Game Commands as Effects

The base game commands (`help`, `raise`, `harvest`, `pact`) are defined as `[[commands.definitions]]` in `mods/core/mod.toml` with data-driven effects. They use the same custom command path as any mod-defined command — their effects are fully executed by the data-driven system.

## Multiple Mods

Multiple mods can be active simultaneously. Entity types from all mods are merged into a shared registry. If two mods define the same entity type or command name, the first one loaded wins (alphabetical directory order). A warning is logged identifying the collision and which mod's definition was kept.

Each mod's `[[spawn]]` entries all execute, and each mod's `[commands].initial` entries are merged.

### Validation

After all mods are loaded, the engine validates:
- **Spawn entity types**: every `[[spawn]]` entry's `entity_type` must match a registered entity type. Unknown types produce a warning: `[mod:<id>] warning: spawn '<name>' references unknown entity type '<type>'`.
- **Spawn effects in custom commands**: `spawn` effects in `[[commands.definitions]]` are also checked against known entity types.
- **Stat names in `modify_stat` and `use_resource` effects**: any stat name is valid. The engine does not restrict which stat names can be used.
- **Target references in effects**: `target` fields must be `"self"` or `"arg:<ref>"` where `<ref>` is a valid numeric index or a name matching one of the command's `args` entries. Invalid references produce a warning.
- **`use_resource` amounts**: must be positive. Non-positive values produce a warning.
- **`if` conditions**: empty resource or entity_type names produce a warning.
- **`start_channel` phases**: phase ticks must be > 0, update_interval must be > 0. Effects within phases are validated recursively.
- **Nested validation**: validation recurses into `if` branches and `start_channel` phase effect lists.
- **Trigger event names**: must be one of the 8 supported types. Unknown event names produce a warning.
- **Trigger `tick_interval`**: must have a positive `interval` filter value.
- **Trigger conditions and effects**: validated with the same checks as command effects and conditions.

## Runtime Entity Spawning

When the simulation spawns new entities at runtime (e.g., the `raise` command creates a skeleton), the engine looks up the entity type in the sprite registry to create a render unit. This means a mod only needs to define the entity type in `[[entities]]` once — it will be used both for initial spawns and for runtime spawns.

Dynamically spawned entities have a **spawn state**: they play their `spawn` animation (if the atlas has one) and can't act or be targeted by queries (`scan`, `nearest`) until the animation finishes. The duration is computed from the atlas JSON's spawn animation total ticks. Entities without a `spawn` animation are immediately ready.

If no sprite data is found for a runtime-spawned entity type, the sim entity is still created but won't have a visible sprite.

## Fallback Behavior

If the `mods/` directory doesn't exist or contains no valid mods, the game falls back to compile-time embedded assets. This ensures `cargo run` works without a `mods/` directory. The fallback provides the same content as the `core` mod: summoner entity at position 500 with `consult`, `raise`, `harvest`, `pact` commands.

## The Base Game Mod

The `mods/core/` directory is the base game, structured as a mod:

```
mods/core/
  mod.toml
  sprites/
    summoner_atlas.png
    summoner_atlas.json
    skeleton_atlas.png
    skeleton_atlas.json
```

Its `mod.toml` defines the skeleton entity type and unlocks the starter commands. The summoner is hardcoded by the engine (not defined in mod.toml). You can edit this file to change the starting configuration without recompiling.

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

3. Add entity definitions with sprite assets:
   - Create your sprite atlas PNG (grid of animation frames)
   - Write the JSON metadata describing frame layout
   - Reference them in `[[entities]]`

4. Add `[[spawn]]` entries for entities you want present at game start.

5. Optionally add `[commands].initial` to unlock additional GrimScript commands.

6. Run the game — your mod will be loaded automatically.

## Initial Effects

The `[initial]` section consolidates all game-start configuration: available commands, available resources, and startup effects. Effects from all mods are merged in load order and resolved against the first entity in the world (typically the summoner).

```toml
[initial]
commands = ["raise", "harvest"]
resources = ["souls"]
effects = [
  { type = "output", message = "Welcome to the void..." },
  { type = "modify_stat", target = "self", stat = "mana", amount = 10 },
]
```

Any effect type can be used. Effects run in order; a `use_resource` effect that fails will abort the remaining initial effects.

## Resource Costs via `use_resource`

Resource costs are expressed as `use_resource` effects within the effects list. Place them before the effects they gate — if the caster doesn't have enough of the resource, the command ends early and remaining effects are skipped.

```toml
[[commands.definitions]]
name = "raise"
description = "Raise the dead"
args = []
effects = [
  { type = "use_resource", stat = "mana", amount = 30 },
  { type = "spawn", entity_type = "skeleton", offset = 1 },
  { type = "output", message = "[raise] A skeleton rises!" },
]
```

The `use_resource` effect checks and deducts the stat atomically. Any stat name defined on the entity can be used. If the entity's current value for that stat is less than `amount`, a warning is printed and no further effects run.

Since `use_resource` is just an effect, you can place multiple resource checks or interleave them with other effects for fine-grained control.

## Conditional Effects

The `if` effect type enables branching based on game state. Effects inside `then` or `else` are resolved recursively, so nesting is supported.

### Syntax

```toml
effects = [
  { type = "if",
    condition = { type = "resource", resource = "mana", compare = "gte", amount = 20 },
    then = [
      { type = "spawn", entity_type = "skeleton", offset = "rand(-100,100)" },
    ],
    else = [
      { type = "output", message = "Not enough mana..." },
    ]
  }
]
```

The `else` branch is optional (defaults to empty — nothing happens if the condition is false).

### Condition Types

| Type | Fields | Evaluates |
|------|--------|-----------|
| `resource` | `resource`, `compare`, `amount` | Global resource value vs threshold |
| `entity_count` | `entity_type`, `compare`, `amount` | Count of alive, ready entities of type vs threshold |
| `stat` | `stat`, `compare`, `amount` | Caster's stat value vs threshold |

### Compare Operators

`eq`, `ne`, `gt`, `gte`, `lt`, `lte`

The `amount` field supports `DynInt` (plain integer or `"rand(min,max)"`).

### Abort Propagation

If a `use_resource` or `use_global_resource` effect inside a branch fails (insufficient resource), the entire command aborts — not just the branch. Effects after the `if` are also skipped.

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

## Conditional Phase Branching (`start_channel`)

The `start_channel` effect initiates a phased channel from within an effect list. Combined with `if`, this enables conditional phase selection — different branches can start different channels.

```toml
effects = [
  { type = "if",
    condition = { type = "resource", resource = "mana", compare = "gte", amount = 20 },
    then = [
      { type = "use_global_resource", resource = "mana", amount = 20 },
      { type = "start_channel", phases = [
        { ticks = 12, on_start = [{ type = "animate", target = "self", animation = "cast" }] },
        { ticks = 18, on_start = [{ type = "spawn", entity_type = "skeleton", offset = "rand(-300,300)" }] },
      ]},
    ],
    else = [
      { type = "output", message = "Not enough mana!" },
    ]
  }
]
```

`start_channel` can also be used without `if` to define phases inline:

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
- The phase definitions use the same schema as top-level `phases` (see Phased Commands below).
- `start_channel` inside an already-active channel (e.g., in phase `on_start`/`per_update`) is ignored — entities can only have one active channel.

## Event Triggers

Mods can define **triggers** — event-driven rules that fire effects when game events occur. Triggers reuse the existing effect resolution engine and condition system.

### Schema

```toml
[[triggers]]
event = "entity_died"
filter = { entity_type = "skeleton" }
conditions = [
  { type = "entity_count", entity_type = "skeleton", compare = "eq", amount = 0 },
]
effects = [
  { type = "output", message = "All skeletons have fallen!" },
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `event` | Yes | — | Event type to listen for (see table below) |
| `filter` | No | `{}` | Type-specific filters to narrow which events match |
| `conditions` | No | `[]` | Conditions that must all be true for the trigger to fire |
| `effects` | No | `[]` | Effects to run when the trigger fires |

### Event Types

| Event | Description | Filter Fields |
|-------|-------------|---------------|
| `entity_died` | An entity was killed | `entity_type` — match only deaths of this type |
| `entity_spawned` | An entity was spawned | `entity_type` — match only spawns of this type |
| `entity_damaged` | An entity took damage | `entity_type` — match only damage to this type |
| `resource_changed` | A global resource value changed | `resource` — match only changes to this resource |
| `command_used` | A custom command was used | `command` — match only this command name |
| `tick_interval` | Fires periodically every N ticks | `interval` — tick interval (required, must be > 0) |
| `channel_completed` | A phased channel finished all phases | `command` — match only this command name |
| `channel_interrupted` | A phased channel was interrupted | `command` — match only this command name |

### Filter Fields

```toml
# Only match skeleton deaths
filter = { entity_type = "skeleton" }

# Only match mana changes
filter = { resource = "mana" }

# Only match when "raise" is used
filter = { command = "raise" }

# Fire every 300 ticks (10 seconds at 30 TPS)
filter = { interval = 300 }
```

Filters are type-specific. Use `entity_type` for entity events, `resource` for resource events, `command` for command/channel events, and `interval` for tick_interval.

### Conditions

Triggers reuse the same condition system as `if` effects. All conditions must be true for the trigger to fire:

```toml
conditions = [
  { type = "resource", resource = "souls", compare = "gte", amount = 10 },
  { type = "entity_count", entity_type = "skeleton", compare = "lt", amount = 5 },
]
```

See [Condition Types](#condition-types) for the full list.

### Trigger Effects

Trigger effects use the same effect types as custom commands (`output`, `damage`, `heal`, `spawn`, `modify_stat`, `modify_resource`, etc.). Effects resolve against the first alive entity in the world (typically the summoner) as the "caster" — `self` in effect targets refers to this entity.

### Execution Model

1. Triggers are processed once at the end of each tick, after all actions are resolved and pending spawns/despawns are flushed.
2. Each trigger is checked against every matching event from the tick.
3. If multiple events match (e.g., 3 skeletons die), the trigger fires once per matching event.
4. Trigger effects do **not** re-trigger other triggers within the same tick (no cascading).
5. Effects from triggers that modify world state (spawning, resources) take effect immediately and are visible to subsequent trigger condition checks.

### Examples

**Spawn wave every 300 ticks:**

```toml
[[triggers]]
event = "tick_interval"
filter = { interval = 300 }
effects = [
  { type = "spawn", entity_type = "skeleton", offset = "rand(-200,200)" },
  { type = "output", message = "A new wave approaches!" },
]
```

**Gain bones when skeletons die:**

```toml
[[triggers]]
event = "entity_died"
filter = { entity_type = "skeleton" }
effects = [
  { type = "modify_resource", resource = "bones", amount = 1 },
]
```

**Alert when mana is full:**

```toml
[[triggers]]
event = "resource_changed"
filter = { resource = "mana" }
conditions = [
  { type = "resource", resource = "mana", compare = "gte", amount = 100 },
]
effects = [
  { type = "output", message = "Mana is full!" },
]
```

**React to raise command:**

```toml
[[triggers]]
event = "command_used"
filter = { command = "raise" }
effects = [
  { type = "output", message = "The earth trembles..." },
]
```

### Validation

At mod load time, the engine validates:
- **Event names**: must be one of the 8 supported event types
- **tick_interval interval**: must be present and > 0
- **Conditions**: same validation as `if` effect conditions
- **Effects**: same recursive validation as command effects (stat names, target references, etc.)

## Buffs/Modifiers

Mods can define **buffs** — temporary stat modifiers with automatic expiry. Buffs are applied and removed via effect types and provide per_tick/on_apply/on_expire effect hooks.

### Buff Definition

```toml
[[buffs]]
name = "rage"
duration = 60
stackable = true
max_stacks = 3
[buffs.modifiers]
attack_damage = 5
speed = 2

[[buffs]]
name = "regen"
duration = 30
per_tick = [
  { type = "modify_stat", target = "self", stat = "health", amount = 1 },
]
on_expire = [
  { type = "output", message = "Regeneration faded." },
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | — | Unique buff identifier |
| `duration` | No | `0` | Default duration in ticks |
| `modifiers` | No | `{}` | Stat modifiers applied while active (stat → amount) |
| `per_tick` | No | `[]` | Effects that run each tick while active |
| `on_apply` | No | `[]` | Effects that run when the buff is first applied |
| `on_expire` | No | `[]` | Effects that run when the buff expires or is removed |
| `stackable` | No | `false` | Whether multiple applications stack |
| `max_stacks` | No | `0` | Maximum stack count (0 = unlimited, only if stackable) |

### Valid Modifier Stats

`health`, `shield`, `speed`, `attack_damage`, `attack_range`

Modifiers are applied additively when the buff is applied and reversed when it expires. Health is clamped to at least 1 on reversal to prevent buff-expiry death.

### Apply/Remove Effects

```toml
effects = [
  { type = "apply_buff", target = "self", buff = "rage" },
  { type = "apply_buff", target = "self", buff = "regen", duration = 100 },
  { type = "remove_buff", target = "self", buff = "rage" },
]
```

| Effect | Fields | Description |
|--------|--------|-------------|
| `apply_buff` | `target`, `buff`, `duration` (optional) | Apply a buff. Duration overrides the buff's default if specified. |
| `remove_buff` | `target`, `buff` | Remove a buff, reversing all modifiers and running on_expire effects. |

### Stacking Behavior

- **Non-stackable** (`stackable = false`): re-applying refreshes the duration without adding additional modifiers.
- **Stackable** (`stackable = true`): each application adds a stack (up to `max_stacks`), applying modifiers again. All stacks share the same duration timer (refreshed on each application).

### Buff Lifecycle

1. **Apply**: modifiers are applied to entity stats, on_apply effects run, ActiveBuff is tracked on the entity.
2. **Each tick** (step 6b): per_tick effects run, duration is decremented.
3. **Expire** (duration reaches 0): modifiers are reversed, on_expire effects run, buff is removed.
4. **Manual remove** (via `remove_buff` effect): same as expire — modifiers reversed, on_expire effects run.

### has_buff Condition

Check if an entity has a specific buff:

```toml
{ type = "if",
  condition = { type = "has_buff", buff = "rage" },
  then = [{ type = "output", message = "Raging!" }],
}
```

## Extended Conditions

In addition to the base conditions (`resource`, `entity_count`, `stat`), the following condition types are available:

| Type | Fields | Evaluates |
|------|--------|-----------|
| `has_buff` | `buff` | Whether the caster has a specific active buff |
| `random_chance` | `percent` | Random check: fires if deterministic roll < percent (1-100) |
| `and` | `conditions` | All sub-conditions must be true |
| `or` | `conditions` | At least one sub-condition must be true |

### Compound Conditions

```toml
{ type = "if",
  condition = { type = "and", conditions = [
    { type = "resource", resource = "mana", compare = "gte", amount = 20 },
    { type = "entity_count", entity_type = "skeleton", compare = "lt", amount = 5 },
  ]},
  then = [{ type = "spawn", entity_type = "skeleton", offset = "rand(-100,100)" }],
}
```

### Random Chance

```toml
{ type = "if",
  condition = { type = "random_chance", percent = 25 },
  then = [{ type = "output", message = "Critical hit!" }],
}
```

Randomness is deterministic — same seed produces same result. The `percent` field should be 1-100.

## Entity Stats

All entity stats live in a single unified map — there are no built-in stats vs custom stats, they're all the same system. Entity types define their stats in `[[entities]]` (see [Stats](#stats) above). Any stat not defined defaults to 0.

```toml
[[entities]]
type = "warrior"
health = 80
speed = 1
stats = { armor = 5, crit_chance = 10, rage = 0 }
```

The convenience fields (`health`, `speed`, `shield`, `attack_damage`, `attack_range`, `attack_cooldown`) are syntactic sugar — they merge into the same stats map as entries in the `stats` table. All stats default to 0 if not defined.

### Effect Types

`modify_stat` and `use_resource` work with **all** stats:

| Effect | Fields | Description |
|--------|--------|-------------|
| `modify_stat` | `target`, `stat`, `amount` | Add to any stat (can be negative). Clamped to `[0, max_{stat}]` if a max exists. |
| `use_resource` | `stat`, `amount` | Check and deduct any stat from self; aborts remaining effects if insufficient. |

```toml
effects = [
  { type = "use_resource", stat = "rage", amount = 10 },
  { type = "modify_stat", target = "self", stat = "armor", amount = -5 },
]
```

For backward compatibility, `modify_custom_stat` is accepted as an alias for `modify_stat`, and `use_custom_stat` as an alias for `use_resource`.

### Condition

The `stat` condition works with all stats:

```toml
{ type = "stat", stat = "armor", compare = "gte", amount = 10 }
```

For backward compatibility, `custom_stat` is accepted as an alias for `stat`.

### Script Access

Stats are accessible via entity attribute access or the `get_stat` builtin:

```python
# Via attribute access (dot notation)
my_armor = self.armor
print("Armor:", my_armor)

# Via builtin function (takes entity ref + stat name)
armor = get_stat(self, "armor")
target_armor = get_stat(target, "armor")
```

| Function | Returns | Description |
|----------|---------|-------------|
| `get_stat(entity, "name")` | `Int` | Get the value of any stat on an entity (0 if undefined) |

`get_stat` is a game builtin gated by `[initial].commands`. `get_custom_stat` is accepted as an alias for backward compatibility.

## Computed Values (DynInt)

Effect `amount` and `offset` fields support game-state-dependent computation in addition to plain integers and `rand(min,max)`:

| Format | Description | Example |
|--------|-------------|---------|
| `42` | Fixed integer | `amount = 42` |
| `"rand(min,max)"` | Random in [min, max] | `amount = "rand(5,15)"` |
| `"entity_count(type)"` | Count of alive entities of type | `amount = "entity_count(skeleton)"` |
| `"resource(name)"` | Current value of a global resource | `amount = "resource(mana)"` |
| `"stat(name)"` | Caster's stat value | `amount = "stat(health)"` |

### Multiplier

All computed values support a `*N` multiplier suffix:

```toml
amount = "entity_count(skeleton)*2"    # 2x skeleton count
amount = "resource(mana)*3"            # 3x mana value
amount = "stat(attack_damage)*2"       # 2x caster's attack damage
```

### Examples

**Damage based on skeleton count:**
```toml
effects = [
  { type = "damage", target = "arg:target", amount = "entity_count(skeleton)*5" },
]
```

**Heal based on current mana:**
```toml
effects = [
  { type = "heal", target = "self", amount = "resource(mana)" },
]
```

**Spawn skeletons based on a custom stat:**
```toml
effects = [
  { type = "spawn", entity_type = "skeleton", offset = "stat(summon_power)*10" },
]
```

## Phased Commands

Commands can optionally use **phases** instead of instant effects to create multi-tick abilities with distinct stages — e.g., a windup, impact, and recovery. Commands with `phases` use the phased system; commands with only `effects` use the instant system. They are mutually exclusive (validated at load time).

### Schema

```toml
[[commands.definitions]]
name = "fireball"
description = "Channel a fireball"
args = ["target"]
phases = [
  { ticks = 5, interruptible = true, per_update = [
    { type = "use_resource", stat = "mana", amount = 10 },
  ], on_start = [
    { type = "output", message = "[fireball] Channeling..." },
  ]},
  { ticks = 1, interruptible = false, on_start = [
    { type = "damage", target = "arg:target", amount = 50 },
    { type = "output", message = "[fireball] Impact!" },
  ]},
  { ticks = 4, interruptible = false, on_start = [
    { type = "output", message = "[fireball] Recovering..." },
  ]},
]
```

### Phase Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `ticks` | Yes | — | Number of ticks this phase lasts (must be > 0) |
| `interruptible` | No | `false` | Whether the entity's script runs during this phase |
| `on_start` | No | `[]` | Effects that run on the first tick of the phase |
| `per_update` | No | `[]` | Effects that run on update ticks of the phase (frequency controlled by `update_interval`) |
| `update_interval` | No | `1` | Run `per_update` effects every N ticks (1 = every tick, 2 = every other tick, etc.) |

Effects inside `on_start` and `per_update` use the same effect types as instant commands (output, damage, heal, spawn, modify_stat, use_resource, list_commands, animate, sacrifice, modify_resource, use_global_resource, if). Note: `start_channel` inside an active channel's phase effects is ignored since the entity is already channeling.

### Execution Model

1. **Initiation tick:** The script calls the command (e.g., `fireball(target)`) → a channel is set up on the entity. No effects run this tick — the action is consumed.
2. **Phase ticks (next tick onward):** Before normal script execution, the tick loop processes active channels:
   - **First tick of a phase:** `on_start` effects run, then `per_update` fires if it's an update tick.
   - **Update tick schedule:** `per_update` effects run when `(ticks_elapsed + 1) % update_interval == 0`. With interval=1 this fires every tick (0,1,2,...); interval=2 fires at ticks 1,3,5; interval=3 fires at ticks 2,5,8.
   - When ticks remaining reaches 0, the next phase begins.
3. **Channel complete:** All phases done → entity resumes normal script execution next tick.

### Timing Example

For the fireball example above (5 + 1 + 4 = 10 phase ticks):

| Tick | What happens |
|------|-------------|
| N | Script calls `fireball(target)` → channel set up |
| N+1 to N+5 | Phase 0: windup (interruptible, drains 10 mana per update) |
| N+6 | Phase 1: impact (50 damage, non-interruptible) |
| N+7 to N+10 | Phase 2: recovery (non-interruptible) |
| N+11 | Script resumes normally |

### Interruption

During **interruptible** phases, the entity's script runs normally each tick:
- If the script yields a real action (move, attack, flee, or another command), the channel is **cancelled** and that action executes instead. A `[<command>] interrupted` message is emitted.
- If the script yields `wait` or halts, the channel continues as normal.

During **non-interruptible** phases, the script is not executed at all — the entity is locked into the channel.

### Cancellation on Resource Failure

If a `use_resource` effect within a phase's `on_start` or `per_update` fails (insufficient resource), the entire channel is cancelled. Remaining effects in that tick and all subsequent phases are skipped. The standard `[<command>] not enough <stat>` message is emitted.

### Edge Cases

| Case | Behavior |
|------|----------|
| Entity dies mid-channel | Dead entities don't tick → channel abandoned |
| Target dies mid-channel | Effects targeting it become no-ops (existing behavior) |
| Script hot-reload during channel | Channel is cleared along with script state |
| Both `effects` and `phases` set | Validation warning; `phases` takes precedence |

## Library Files

Mods can provide `.grim` library files whose functions are prepended to player scripts before compilation. This allows mods to ship reusable GrimScript utilities alongside their custom commands and entity types.

### Schema

```toml
[commands]
libraries = ["lib/utils.grim", "lib/combat.grim"]
```

Paths are relative to the mod directory. Files are loaded in the order listed. If a file is missing, a warning is emitted and loading continues.

### Namespace Strategy

Flat namespace with first-loaded-wins, consistent with entity types and custom commands. If two mods define a function with the same name, the first-loaded mod's version is used (the compiler will see it first in the prepended source). No mod-prefixed namespaces — adds complexity with minimal benefit at current scale.

### Gating

Library functions are subject to the same command gating as player scripts. If a library function calls `raise()`, the `raise` command must be in the available set. The compiler validates this at compile time.

### How It Works

1. At mod load time, `.grim` files listed in `commands.libraries` are read and concatenated into the mod's `library_source`
2. When a player script is compiled (Run or Console), all mod library sources are prepended to the script source
3. The combined source is compiled as a single unit — library functions are visible to the player's code as if defined at the top of their script

### Interaction with Custom Commands

Library functions can call custom commands from any loaded mod, subject to the same available commands gating as player scripts.

## Adding New Effects (Developer Guide)

There are two paths for adding new game mechanics, depending on what the mechanic needs to do.

### Effects vs Builtins

| | Effect path | Builtin path |
|---|---|---|
| **What it is** | A new `CommandEffect` variant modders use from `mod.toml` | A new function scripts call directly (e.g., `nearest("skeleton")`) |
| **Files touched** | 1–2 Rust files | 5 files |
| **Use when** | The mechanic modifies world state as part of a command (teleport, area damage, buff/debuff, resource transfer, etc.) | The mechanic returns a value to the script, or needs script-computed arguments (`move(get_pos(target))`) |
| **Examples** | `damage`, `heal`, `spawn`, `modify_stat`, `use_resource` | `nearest`, `scan`, `get_health`, `move`, `attack` |

**Rule of thumb:** if a modder can express it as static TOML values (fixed amounts, stat names, entity types), it belongs in the effect system. If the script needs to compute arguments at runtime or read a return value, it needs a builtin.

### Step-by-step: Adding a New Effect

1. **Add a variant to `CommandEffect`** in `crates/deadcode-sim/src/action.rs`:

   ```rust
   #[serde(rename = "teleport")]
   Teleport {
       target: String,
       position: i64,
   },
   ```

   The `#[serde(rename = "...")]` controls the `type` string used in `mod.toml`.

2. **Add a match arm in `resolve_custom_effects()`** in the same file. This is where the effect actually modifies world state:

   ```rust
   CommandEffect::Teleport { target, position } => {
       let target_id = resolve_target_from_args(entity_id, target, args);
       if let Some(tid) = target_id {
           if let Some(entity) = world.get_entity_mut(tid) {
               entity.position = *position;
           }
       }
   }
   ```

3. **(Optional) Add validation in `modding.rs`** — see below for when this is needed.

4. **Use it in `mod.toml`:**

   ```toml
   effects = [
     { type = "teleport", target = "self", position = 100 },
   ]
   ```

5. **Update `docs/modding.md`** — add the effect to the Effect Types table and document its fields.

### When to Add Validation in `modding.rs`

`validate_command_defs()` in `crates/deadcode-app/src/modding.rs` runs at mod load time and checks that TOML values reference valid things. Not all effects need validation — only effects with fields whose values can be wrong in ways TOML parsing alone can't catch.

**Fields that need validation:**

| Field | What's checked | Why |
|-------|---------------|-----|
| `target` (in `damage`, `heal`, `modify_stat`) | Must be `"self"` or `"arg:<name>"` where `<name>` matches a declared arg | Catches typos like `"arg:victem"` when the arg is `"victim"` |
| `stat` (in `modify_stat`, `use_resource`) | Any string — maps to the entity's stats map | Stats not defined on the entity default to 0 |
| `amount` (in `use_resource`) | Must be positive | A non-positive cost doesn't make sense |
| `entity_type` (in `spawn`) | Validated separately by `validate_spawns()` — checks the type exists in some loaded mod | Catches references to undefined entity types |

**Fields that don't need validation:**

- `message` in `output` — any string is valid
- Effects with no fields (like `list_commands`)

If your new effect has a `target` or `stat` field, add it to the existing match arms in `validate_command_defs()`. If it introduces a new kind of validated field, add a new check.

### Step-by-step: Adding a New Builtin

For completeness, here's the builtin path (5 files):

1. `crates/deadcode-sim/src/ir.rs` — add `Instruction` variant(s) (e.g., `QueryTeleport`, `ActionTeleport`)
2. `crates/deadcode-sim/src/executor.rs` — add match arm(s) to execute the instruction
3. `crates/deadcode-sim/src/action.rs` — if it's an action, add a `UnitAction` variant and handle it in `resolve_action()`
4. `crates/deadcode-sim/src/compiler/builtins.rs` — map the function name to IR instruction(s)
5. `crates/grimscript-lang/src/builtins.rs` — register the function name so the interpreter knows about it; set `is_game_builtin()` to return true so it's gated by available commands

## Internals

The mod system lives in `crates/deadcode-app/src/modding.rs`. Key types:

| Type | Purpose |
|------|---------|
| `ModManifest` | Deserialized `mod.toml` |
| `EntityDef` | Entity type definition (type, sprite path, pivot, stats) |
| `SpawnDef` | Initial spawn definition (type, name, position) |
| `InitialDef` | Initial state: available commands, available resources, startup effects |
| `CommandsDef` | Command definitions + reserved library paths |
| `CommandDef` | Custom command definition (name, description, args, effects, phases, unlisted) — lives in `deadcode-sim/action.rs` |
| `PhaseDef` | Single phase in a multi-tick phased command (ticks, interruptible, on_start, per_update, update_interval) — lives in `deadcode-sim/action.rs` |
| `ChannelState` | Active channel state on an entity during phased execution — lives in `deadcode-sim/entity.rs` |
| `CommandEffect` | Effect type enum (output, damage, heal, spawn, modify_stat, use_resource, list_commands, animate, sacrifice, modify_resource, use_global_resource, if, start_channel) — lives in `deadcode-sim/action.rs` |
| `Condition` | Condition enum for `if` effects (resource, entity_count, stat) with `CompareOp` — lives in `deadcode-sim/action.rs` |
| `EffectOutcome` | Result of effect resolution: Complete, Aborted, or StartChannel — lives in `deadcode-sim/action.rs` |
| `TriggerDef` | Trigger definition (event, filter, conditions, effects) — lives in `deadcode-sim/action.rs` |
| `TriggerFilter` | Filter fields for narrowing trigger event matches — lives in `deadcode-sim/action.rs` |
| `DynInt` | Integer value: fixed or `rand(min,max)` — deserialized from TOML, resolved at effect execution time — lives in `deadcode-sim/action.rs` |
| `SpriteData` | Loaded PNG bytes + JSON metadata string |
| `LoadedMod` | Fully resolved mod with sprite/pivot/config/command registries |
| `ModMeta` | Mod metadata: id, name, version, reserved dependency fields |
| `EntityConfig` | Stat overrides applied at entity spawn time (in `deadcode-sim`) |

**Loading flow:**
1. `modding::load_mods()` scans `mods/` for directories containing `mod.toml`, sorted alphabetically by directory name
2. Each manifest is parsed, sprite files are read from disk
3. Registries (sprites, pivots, entity configs) are merged into `App`, with collision warnings on duplicates
4. `modding::validate_spawns()` checks all spawn entity types and spawn effects against known types
5. `modding::validate_command_defs()` checks stat names, target references, `use_resource` amounts, phase ticks > 0, phase update_interval > 0, effects/phases mutual exclusivity, effects within phase `on_start`/`per_update` lists, `if` conditions (empty names, unknown stats), and `start_channel` phase parameters. Validation recurses into `if` branches and `start_channel` phases.
5b. `modding::validate_triggers()` checks event names, tick_interval interval values, conditions, and effects within triggers.
6. `[[spawn]]` entries create both sim entities and render units
7. `[initial].commands` entries populate `App::available_commands`
8. `[[commands.definitions]]` entries populate `App::command_defs` and are registered with `SimWorld` (effects, arg counts), with collision warnings on duplicate command names
8b. `[[triggers]]` entries are collected via `collect_triggers()` and registered with `SimWorld`
9. `[resources]` entries are collected via `collect_initial_resources()` and stored in `SimWorld.resources`
10. `[initial].resources` entries are collected via `collect_available_resources()` and stored in `SimWorld.available_resources`

**Custom command flow:**
1. `CommandDef` structs are parsed from TOML and collected in `App::command_defs`
2. At sim init, each def is registered via `SimWorld::register_custom_command()` — stores effects, arg counts, and phases
3. The compiler receives custom command arg counts, emits `ActionCustom(name)` IR instructions
4. The executor pops args and yields `UnitAction::Custom { name, args }`
5. `resolve_action()` checks if the command has phases: if yes, creates a `ChannelState` on the entity (effects start next tick); if no, resolves instant effects in order; `use_resource` effects abort the command early if the resource check fails; `if` effects branch on conditions; `start_channel` effects initiate a channel from within instant effects
6. **Phased commands:** each tick, the tick loop processes active channels before script execution — runs `on_start`/`per_update` effects (respecting `update_interval`), handles interruption for interruptible phases, advances phase counters
7. Custom command metadata (name, description, args) is sent to the frontend via `AvailableCommands` IPC for autocomplete

**Reserved schema fields** (parsed but not enforced):
- `depends_on: Vec<String>` — mod IDs this mod requires
- `conflicts_with: Vec<String>` — mod IDs this mod conflicts with
- `min_game_version: Option<String>` — minimum game version required
- `libraries: Vec<String>` — paths to `.grim` library files (Phase 2)

**Future phases** (not yet implemented):
- **Phase 2**: Mods provide `.grim` library files whose functions are compiled and merged into player scripts
- **Dependency resolution**: `depends_on` / `conflicts_with` enforcement, load order based on dependency graph
