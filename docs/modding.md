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

The game scans `mods/` at startup and loads every directory that contains a valid `mod.toml` **in alphabetical order by directory name** (deterministic across platforms). If no mods are found, the game falls back to embedded assets (identical to the pre-modding behavior).

## mod.toml Reference

```toml
[mod]
id = "my-mod"           # Unique identifier (lowercase, no spaces)
name = "My Mod"         # Display name
version = "0.1.0"       # Semver version string
depends_on = []         # Reserved: mod IDs this mod requires (not enforced yet)
conflicts_with = []     # Reserved: mod IDs this mod conflicts with (not enforced yet)
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

All fields in `[[entities]]` except `type` are optional. Omitted stats use engine defaults (health=100, mana=100, speed=1, etc.). Omitted `sprite` means the entity won't have a render unit.

**Reserved entity type:** The `"summoner"` entity type is hardcoded by the game engine and cannot be defined or overridden by mods. It is always spawned at position 500 with fixed stats. Mods that define an entity with `type = "summoner"` will see a warning and their definition will be ignored.

## Entity Definitions

Entity definitions register types that can be spawned — either at startup via `[[spawn]]` or at runtime by game actions (e.g., the `raise` command creates skeletons).

### Stats

| Field | Default | Description |
|-------|---------|-------------|
| `health` | 100 | Max and current health |
| `mana` | 100 | Max and current mana |
| `speed` | 1 | Movement speed (tiles per tick) |
| `attack_damage` | 10 | Damage dealt per attack |
| `attack_range` | 5 | Range for attack actions |
| `attack_cooldown` | 3 | Ticks between attacks |
| `shield` | 0 | Max and current shield |

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

For `modify_stat`, valid stat names are: `health`, `mana`, `shield`, `speed`.

### Base Game Commands as Effects

The base game commands (`help`, `raise`, `harvest`, `pact`) are defined as `[[commands.definitions]]` in `mods/core/mod.toml` with data-driven effects. They use the same custom command path as any mod-defined command — their effects are fully executed by the data-driven system.

## Multiple Mods

Multiple mods can be active simultaneously. Entity types from all mods are merged into a shared registry. If two mods define the same entity type or command name, the first one loaded wins (alphabetical directory order). A warning is logged identifying the collision and which mod's definition was kept.

Each mod's `[[spawn]]` entries all execute, and each mod's `[commands].initial` entries are merged.

### Validation

After all mods are loaded, the engine validates:
- **Spawn entity types**: every `[[spawn]]` entry's `entity_type` must match a registered entity type. Unknown types produce a warning: `[mod:<id>] warning: spawn '<name>' references unknown entity type '<type>'`.
- **Spawn effects in custom commands**: `spawn` effects in `[[commands.definitions]]` are also checked against known entity types.
- **Stat names in `modify_stat` and `use_resource` effects**: must be one of `health`, `mana`, `shield`, `speed`. Unknown stat names produce a warning.
- **Target references in effects**: `target` fields must be `"self"` or `"arg:<ref>"` where `<ref>` is a valid numeric index or a name matching one of the command's `args` entries. Invalid references produce a warning.
- **`use_resource` amounts**: must be positive. Non-positive values produce a warning.
- **`if` conditions**: stat names in `stat` conditions must be valid (`health`, `shield`, `speed`). Empty resource or entity_type names produce a warning.
- **`start_channel` phases**: phase ticks must be > 0, update_interval must be > 0. Effects within phases are validated recursively.
- **Nested validation**: validation recurses into `if` branches and `start_channel` phase effect lists.

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

Its `mod.toml` defines two entity types (summoner, skeleton), spawns one summoner at position 500, and unlocks the four starter commands. You can edit this file to change the starting configuration without recompiling.

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

The `use_resource` effect checks and deducts the resource atomically. Valid stats: `health`, `mana`, `shield`. If the entity's current value for that stat is less than `amount`, a warning is printed (e.g., `[raise] not enough mana`) and no further effects run.

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
| `stat` | `stat`, `compare`, `amount` | Caster's stat (`health`, `shield`, `speed`) vs threshold |

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

## Phase 2: Library Files (Future)

> **Status:** Design sketch. The `libraries` field is reserved in the schema but not yet loaded or compiled.

Mods will be able to provide `.grim` library files whose functions are compiled and merged into player scripts. This allows mods to ship reusable GrimScript utilities alongside their custom commands and entity types.

### Schema

```toml
[commands]
initial = ["raise", "drain"]
libraries = ["lib/utils.grim", "lib/combat.grim"]
```

### Namespace Strategy

Flat namespace with first-loaded-wins, consistent with entity types and custom commands. No mod-prefixed namespaces yet — adds complexity with minimal benefit at current scale. If collisions become a problem, namespacing can be added later.

### Gating

Library functions inherit the mod's available commands set. If a library function calls `raise()`, the `raise` command must be in the available set. The compiler validates this at compile time, not at runtime.

### Compilation Order

1. Load all mods, register custom commands and entity types
2. Compile library `.grim` files (they can reference custom commands from the same mod)
3. Library functions are injected into the player's script as additional function definitions before compilation
4. The player's script is compiled with all library functions available

### Interaction with Custom Commands

Library functions can call custom commands from the same mod. Cross-mod library-to-custom-command calls work if the target command is in the available set (i.e., in `initial` or unlocked at runtime).

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
| `stat` (in `modify_stat`, `use_resource`) | Must be one of `health`, `mana`, `shield`, `speed` | Catches invalid stat names like `"energy"` or `"hp"` |
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
6. `[[spawn]]` entries create both sim entities and render units
7. `[initial].commands` entries populate `App::available_commands`
8. `[[commands.definitions]]` entries populate `App::command_defs` and are registered with `SimWorld` (effects, arg counts), with collision warnings on duplicate command names
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
