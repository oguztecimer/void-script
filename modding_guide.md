# VOID//SCRIPT Modding Guide

This guide explains how to create mods for VOID//SCRIPT. The base game itself is a mod (`mods/core/`), so everything you see in the game can be customized or extended.

## Getting Started

1. Create a folder inside `mods/` with your mod's name:
   ```
   mods/my-mod/
   ```

2. Create a `mod.toml` file inside it:
   ```toml
   [mod]
   id = "my-mod"
   name = "My Mod"
   version = "0.1.0"
   ```

3. Run the game. Your mod is loaded automatically.

Mods load in alphabetical order by folder name. If two mods define the same entity type or command, the first one loaded wins.

## Defining Entities

Entities are the units that live on the strip. Define them with `[[entities]]` blocks:

```toml
[[entities]]
type = "warrior"
sprite = "sprites/warrior_atlas"
pivot = [24.0, 0.0]
health = 80
mana = 60
speed = 2
attack_damage = 15
attack_range = 3
attack_cooldown = 2
shield = 10
```

Only `type` is required. All other fields have defaults:

| Field | Default | Description |
|-------|---------|-------------|
| `health` | 100 | Max and starting health |
| `mana` | 100 | Max and starting mana |
| `speed` | 1 | Tiles moved per tick |
| `attack_damage` | 10 | Damage per attack |
| `attack_range` | 5 | Attack range in tiles |
| `attack_cooldown` | 3 | Ticks between attacks |
| `shield` | 0 | Max and starting shield |

### Sprites

Sprites use an atlas system: a PNG spritesheet paired with a JSON metadata file. The `sprite` field points to both (without extension) — the engine looks for `.png` and `.json`.

The JSON describes animations:

```json
{
  "frame_width": 48,
  "frame_height": 48,
  "animations": [
    {
      "name": "idle",
      "row": 0,
      "frames": [
        { "col": 0, "duration_ms": 200 },
        { "col": 1, "duration_ms": 200 }
      ],
      "loop_mode": "loop"
    }
  ]
}
```

An `idle` animation is required. Loop modes: `"loop"` (repeats) or `"play_once"` (plays then returns to idle).

## Spawning Entities

Place entities on the strip at game start with `[[spawn]]`:

```toml
[[spawn]]
entity_type = "warrior"
name = "guard"
position = 300
```

- `entity_type` must match a `type` from any loaded mod's `[[entities]]`
- `name` is the instance name (links to the render unit)
- `position` is the 1D coordinate on the strip

## Initial Effects

The `[initial]` section defines effects that run when the game opens without loading a saved game. Use this for intro text, starting bonuses, or any setup logic.

```toml
[initial]
effects = [
  { type = "output", message = "The void welcomes you..." },
  { type = "modify_stat", target = "self", stat = "mana", amount = 10 },
]
```

Effects run in order against the first entity in the world (typically the summoner). Any effect type is valid here — see the [Effects Reference](#effects-reference) below. Initial effects from all loaded mods are merged in load order.

## Commands

### Unlocking Commands

The `[commands].initial` list controls which GrimScript commands players can use from the start:

```toml
[commands]
initial = ["raise", "harvest", "my_spell"]
```

Commands not in this list are locked until unlocked at runtime. Stdlib functions (`print`, `len`, `range`, `abs`, `min`, `max`, etc.) are always available.

### Defining Custom Commands

Create new commands with `[[commands.definitions]]`. Each custom command is an **action** that consumes one tick.

```toml
[[commands.definitions]]
name = "smite"
description = "Strike with dark energy"
args = ["target"]
effects = [
  { type = "use_resource", stat = "mana", amount = 25 },
  { type = "damage", target = "arg:target", amount = 30 },
  { type = "output", message = "[smite] Dark energy strikes!" },
]
```

- `name` — the function name players call in GrimScript (e.g., `smite(target)`)
- `description` — shown when players use the `help` command
- `args` — argument names (positional)
- `effects` — what happens when the command runs (see below)

## Effects Reference

Effects run in order when a command executes. If a `use_resource` effect fails (not enough of the resource), the command stops immediately — no further effects run.

### `output`

Print a message to the console.

```toml
{ type = "output", message = "[heal] You feel restored." }
```

| Field | Description |
|-------|-------------|
| `message` | The text to display |

### `damage`

Deal damage to a target. Shield absorbs damage first.

```toml
{ type = "damage", target = "arg:target", amount = 20 }
```

| Field | Description |
|-------|-------------|
| `target` | Who to damage (see [Target Resolution](#target-resolution)) |
| `amount` | Damage to deal |

### `heal`

Restore health to a target, capped at their max health.

```toml
{ type = "heal", target = "self", amount = 15 }
```

| Field | Description |
|-------|-------------|
| `target` | Who to heal |
| `amount` | Health to restore |

### `spawn`

Create a new entity at a position relative to the caster.

```toml
{ type = "spawn", entity_type = "skeleton", offset = 1 }
```

| Field | Description |
|-------|-------------|
| `entity_type` | Type of entity to create (must be defined in some mod's `[[entities]]`) |
| `offset` | Position offset from the caster (e.g., `1` = one tile ahead) |

### `modify_stat`

Add to (or subtract from) a stat on a target.

```toml
{ type = "modify_stat", target = "self", stat = "mana", amount = 20 }
{ type = "modify_stat", target = "self", stat = "health", amount = -10 }
```

| Field | Description |
|-------|-------------|
| `target` | Who to modify |
| `stat` | One of: `health`, `mana`, `shield`, `speed` |
| `amount` | Value to add (negative to subtract). Clamped to 0 at minimum, max stat at maximum. |

### `use_resource`

Check that the caster has enough of a resource, then deduct it. If the caster doesn't have enough, the command **stops** — no further effects execute, and a warning is printed to the console.

```toml
{ type = "use_resource", stat = "mana", amount = 30 }
```

| Field | Description |
|-------|-------------|
| `stat` | One of: `health`, `mana`, `shield` |
| `amount` | Amount required and deducted |

Place `use_resource` **before** the effects it should gate. You can use multiple `use_resource` effects to require different resources, or place them at different points in the effect list for fine-grained control.

**Example — command that costs both energy and health:**

```toml
[[commands.definitions]]
name = "sacrifice"
description = "Power through pain"
args = []
effects = [
  { type = "use_resource", stat = "mana", amount = 20 },
  { type = "use_resource", stat = "health", amount = 10 },
  { type = "modify_stat", target = "self", stat = "shield", amount = 50 },
  { type = "output", message = "[sacrifice] Shield surges!" },
]
```

### `list_commands`

Print all registered commands and their descriptions to the console.

```toml
{ type = "list_commands" }
```

No fields. Useful for discovery/help commands.

## Target Resolution

Effects that take a `target` field accept:

- `"self"` — the entity running the command
- `"arg:<name>"` — an entity passed as an argument by the player's script

For `arg:` targets, matching is by position: the first arg defined = index 0, second = index 1, etc. You can also use `"arg:0"`, `"arg:1"` directly.

**Example:**

```toml
[[commands.definitions]]
name = "drain"
args = ["victim"]
effects = [
  { type = "damage", target = "arg:victim", amount = 20 },
  { type = "heal", target = "self", amount = 10 },
]
```

In GrimScript, the player calls: `drain(some_entity)`

## Full Example Mod

```toml
[mod]
id = "undead-expansion"
name = "Undead Expansion"
version = "1.0.0"

[[entities]]
type = "wraith"
sprite = "sprites/wraith_atlas"
pivot = [24.0, 0.0]
health = 40
mana = 80
speed = 3
shield = 20

[[spawn]]
entity_type = "wraith"
name = "phantom"
position = 700

[initial]
effects = [
  { type = "output", message = "The veil between worlds grows thin..." },
]

[commands]
initial = ["haunt", "devour"]

[[commands.definitions]]
name = "haunt"
description = "Send a wraith to terrorize"
args = []
effects = [
  { type = "use_resource", stat = "mana", amount = 40 },
  { type = "spawn", entity_type = "wraith", offset = 2 },
  { type = "output", message = "[haunt] A wraith appears!" },
]

[[commands.definitions]]
name = "devour"
description = "Consume a target's essence"
args = ["prey"]
effects = [
  { type = "use_resource", stat = "mana", amount = 15 },
  { type = "damage", target = "arg:prey", amount = 25 },
  { type = "heal", target = "self", amount = 15 },
  { type = "output", message = "[devour] Essence consumed!" },
]
```

## Validation

The engine validates your mod at load time and prints warnings for:

- Spawn definitions referencing unknown entity types
- Unknown stat names in `modify_stat` or `use_resource` effects (valid: `health`, `mana`, `shield`, `speed`)
- Invalid target references in effects
- Non-positive `use_resource` amounts

Check the console output when launching the game to see any warnings about your mod.

## Tips

- Look at `mods/core/mod.toml` to see how the base game is defined — it's a regular mod
- Entity types are shared across mods, so your mod can spawn entities defined by other mods
- Commands from all mods are merged into one pool
- In dev mode (`--features dev-mode`), all commands are unlocked regardless of the `initial` list
- Effects run in the exact order listed — put `use_resource` first to gate everything, or later if some effects should always run
