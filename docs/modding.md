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
- [Initialization](#initialization)
- [Buff Stat Modifiers](#buff-stat-modifiers)
- [Entity Stats](#entity-stats)
- [Lua Scripting (mod.lua)](#lua-scripting-modlua)
- [Library Files](#library-files)
- [Sprite Format](#sprite-format)
- [Multiple Mods](#multiple-mods)
- [Validation](#validation)
- [Runtime Entity Spawning](#runtime-entity-spawning)
- [The Base Game Mod](#the-base-game-mod)
- [Creating a New Mod](#creating-a-new-mod)
- [GrimScript API Reference](#grimscript-api-reference)
- [Internals](#internals)

---

## Mod Structure

Each mod is a directory inside `mods/` containing a `mod.toml` manifest and any associated assets:

```
mods/
  my-mod/
    mod.toml                # Required: mod manifest (data declarations)
    mod.lua                 # Optional: Lua logic (commands, triggers, init)
    grimscript/             # Brain scripts for entity types
    lib/
      utils.grim            # GrimScript library files (optional)
    sprites/
      warrior_atlas.png     # Sprite sheet PNG
      warrior_atlas.json    # Atlas metadata (frame layout)
```

**TOML vs Lua:** `mod.toml` declares *what exists* (types, entities, resources, buff definitions). `mod.lua` defines *what happens* (command logic, triggers, buff callbacks, initialization). When both define a command, the Lua handler takes priority.

The game scans `mods/` at startup and loads every directory that contains a valid `mod.toml`. Mods are then reordered by their dependency graph (topological sort via Kahn's algorithm, with alphabetical tie-breaking for determinism). If no mods are found, nothing loads.

### Dependencies and Conflicts

Mods can declare dependencies and conflicts in the `[mod]` section:

- **`depends_on`**: List of mod IDs that must be loaded first. If a dependency is missing, the mod (and any mods that depend on it) is skipped with a warning. This cascades — if A depends on B and B is missing, A is also skipped.
- **`conflicts_with`**: List of mod IDs that cannot be loaded alongside this mod. If a conflict exists, the first-loaded mod wins and the conflicting mod is skipped with a warning.

Circular dependencies are detected and logged as an error. The affected mods fall back to alphabetical ordering.

---

## mod.toml Reference

A complete `mod.toml` with all sections. Note: `mod.toml` is **data-only**. All behavior (commands, triggers, buff callbacks, init) is defined in `mod.lua`.

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

[[types]]
name = "skeleton_ai"
brain = true                         # Brain types drive entity execution via .gs files

# --- Entity Definitions ---
[[entities]]
id = "warrior"                      # Unique entity definition ID (for registry lookups)
types = ["undead", "melee"]         # Composable type tags (stats merged in order)
sprite = "sprites/warrior_atlas"    # Path to sprite files (no extension; expects .png + .json)
pivot = [24.0, 0.0]                 # Sprite pivot point [x, y]
stats = { armor = 5, crit = 10 }   # Entity-level stats (override type stats)

# If `types` is absent, defaults to `[id]`.

# --- Global Resources ---
[resources]
souls = 0                           # Capless resource (plain integer)
mana = { value = 50, max = 100 }    # Capped resource (initial value + max cap)
gold = 100                          # Capless resource

# --- Initial State ---
[initial]
resources = ["souls", "mana"]       # Resources available at game start (omit = all available)

# --- GrimScript Libraries ---
[commands]
libraries = ["lib/utils.grim"]      # GrimScript library files to prepend to player scripts

# --- Buff Stat Definitions ---
[[buffs]]
name = "rage"
duration = 60
stackable = true
max_stacks = 3
[buffs.modifiers]
attack_damage = 5
speed = 2
```

Commands, triggers, and init effects are defined in `mod.lua` — see [Lua Scripting](#lua-scripting-modlua).

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

[[types]]
name = "skeleton_ai"
brain = true                         # Brain types drive entity execution via .gs files
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

Query functions for types (e.g. scan, nearest, get_types, has_type) are not yet implemented. Entity type tags are used internally by the effect system (`DynInt::EntityCount`, `Condition::EntityCount`, trigger filters) and will be exposed to scripts when query builtins are reimplemented.

---

## Entity Definitions

Entity definitions register entities that can be spawned — either at startup via `[initial].effects` or at runtime by effects (e.g., the `raise` command spawns skeletons).

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

Unavailable resources produce errors when accessed via effects.

If `[initial].resources` is omitted or empty, **all defined resources are available by default**.

In dev mode (`--features dev-mode`), all resources are available regardless.

### Script Access

Resource access from GrimScript is not yet reimplemented. Use Lua `ctx:modify_resource()` and `ctx:use_resource()` in command handlers.

---

## Available Commands

All non-stdlib commands are defined in Lua via `mod.command()` in `mod.lua`.

### Always Available (Stdlib)

These are always available and not defined by mods:

`print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`, `percent`, `scale`

### Command Capability Gating

Commands are gated by **type capability** (`commands` on `[[types]]`). An entity's effective commands = union of all its types' `commands` lists.

If no types define `commands`, all commands are available (`compute_effective_commands()` returns `None`).

In dev mode (`--features dev-mode`), the gate is bypassed — all commands are available to all entities.

---

## Initialization

Initialization is defined in Lua via `mod.on_init()`. The `[initial]` section in mod.toml only specifies initially available resources:

```toml
[initial]
resources = ["mana", "bones"]
```

All spawn effects, output messages, and other init logic go in Lua:

```lua
mod.on_init(function(ctx)
  ctx:set_available_resources({"mana", "bones"})
  ctx:spawn("summoner", { offset = 0 })
  ctx:output("Welcome to the void...")
end)
```

---

## Commands, Triggers, and Effects

All command logic, event triggers, buff callbacks, and initialization effects are defined in Lua. See the [Lua Scripting](#lua-scripting-modlua) section below for the full API reference.

## Buff Stat Modifiers

Buff stat modifiers and duration are defined in TOML via `[[buffs]]`:

```toml
[[buffs]]
name = "rage"
duration = 60
modifiers = { attack_damage = 5 }
stackable = true
max_stacks = 3
```

Buff lifecycle callbacks (`on_apply`, `per_tick`, `on_expire`) are defined in Lua via `mod.buff()`.

## Entity Stats

All entity stats live in a single `stats` map. There are no built-in default stats — all stats come from type and entity definitions in `mod.toml`. Stats merge in type order, then entity-level overrides.

---

## Lua Scripting (mod.lua)

Mods can include a `mod.lua` file for defining command logic, triggers, buff callbacks, and initialization. Lua provides loops, variables, and function composition — things that are verbose or impossible in TOML.

### Getting Started

```lua
local mod = require("void")

mod.on_init(function(ctx)
  ctx:spawn("summoner", { offset = 0 })
  ctx:output("Welcome!")
end)

mod.command("heal_self", { description = "Restore health" }, function(ctx)
  ctx:heal("self", 10)
  ctx:output("Healed!")
end)
```

### Registering Commands

```lua
mod.command(name, [opts], handler)
```

- **`name`** (string): Command name (must be unique across all mods)
- **`opts`** (table, optional): `{ description = "...", unlisted = true/false, args = {"arg1"} }`
- **`handler`** (function): Receives a `ctx` object

**Instant command** (no yield — completes in one tick):
```lua
mod.command("pact", { description = "Pledge your bones" }, function(ctx)
  ctx:modify_stat("self", "health", -10)
  ctx:output("[pact] Power surges through you...")
end)
```

**Multi-tick command** (uses `yield_ticks` — coroutine-based):
```lua
mod.command("raise", { description = "Raise the dead" }, function(ctx)
  ctx:animate("self", "cast")
  ctx:yield_ticks(12, { interruptible = true })

  if not ctx:use_resource("mana", 20) then return end
  ctx:yield_ticks(1)

  ctx:spawn("skeleton", { offset = ctx:rand(-300, 300) })
  ctx:yield_ticks(18)
end)
```

**Looping command** (runs until interrupted or returns):
```lua
mod.command("trance", { description = "Recover mana" }, function(ctx)
  ctx:output("[trance] Mana flows in.")
  for i = 1, 200 do
    ctx:yield_ticks(5, { interruptible = true })
    ctx:modify_resource("mana", 1)
  end
end)
```

### Initialization

```lua
mod.on_init(function(ctx)
  ctx:set_available_resources({"mana", "bones"})
  ctx:spawn("summoner", { offset = 0 })
  ctx:output("The dead stir beneath your feet")
end)
```

Runs once at game startup. Replaces `[initial].effects` in TOML.

### Event Triggers

```lua
mod.on(event_name, [opts], handler)
```

```lua
mod.on("entity_died", { filter = { entity_type = "skeleton" } }, function(ctx, event)
  ctx:modify_resource("bones", 1)
end)

mod.on("entity_spawned", { filter = { entity_type = "warrior" } }, function(ctx, event)
  ctx:output(event.name .. " has arrived!")
end)
```

**Event types:** `entity_died`, `entity_spawned`, `entity_damaged`, `command_used`, `channel_completed`

**Event data fields:** `entity_id`, `name`, `entity_type`, `killer_id`, `owner_id`, `attacker_id`, `command`, `damage`, `new_health`, `position`, `spawner_id` (varies by event type)

### Buff Callbacks

```lua
mod.buff(name, callbacks)
```

```lua
mod.buff("rage", {
  on_apply = function(ctx, target)
    ctx:output("[rage] Fury takes hold!")
  end,
  per_tick = function(ctx, target)
    if ctx:get_stat(target, "health") < 10 then
      ctx:remove_buff(target, "rage")
    end
  end,
  on_expire = function(ctx, target)
    ctx:output("[rage] The fury subsides.")
  end,
})
```

Buff stat modifiers (`modifiers` field) and duration remain in TOML `[[buffs]]`. Lua callbacks replace the `per_tick`, `on_apply`, and `on_expire` effect lists.

### ctx API Reference

#### Entity Operations

| Method | Description |
|--------|-------------|
| `ctx:damage(target, amount)` | Deal damage (shield-first) |
| `ctx:heal(target, amount)` | Heal (capped at max_health) |
| `ctx:modify_stat(target, stat, amount)` | Modify any stat |
| `ctx:get_stat(target, stat)` | Read stat value (0 if unset) |
| `ctx:spawn(entity_id, opts)` | Spawn entity. `opts = { offset = N }` |
| `ctx:animate(target, animation)` | Trigger sprite animation |
| `ctx:apply_buff(target, buff, opts)` | Apply buff. `opts = { duration = N }` |
| `ctx:remove_buff(target, buff)` | Remove buff |

#### Resource Operations

| Method | Description |
|--------|-------------|
| `ctx:use_resource(name, amount)` | Check+deduct. Returns `true`/`false` |
| `ctx:modify_resource(name, amount)` | Add/subtract (can be negative) |
| `ctx:get_resource(name)` | Read current value |
| `ctx:set_available_resources(names)` | Set which resources are available |

#### Output

| Method | Description |
|--------|-------------|
| `ctx:output(message)` | Print to console |
| `ctx:list_commands()` | Show available commands |

#### Queries

| Method | Description |
|--------|-------------|
| `ctx:entity_count(type)` | Count alive entities of type |
| `ctx:is_alive(target)` | Check if entity is alive |
| `ctx:distance(a, b)` | Absolute distance between entities |
| `ctx:has_buff(target, buff)` | Check if buff is active |
| `ctx:has_type(target, type)` | Check type tag |
| `ctx:position(target)` | Get position |
| `ctx:owner(target)` | Get owner (integer ID or nil) |
| `ctx:entities_of_type(type)` | List of entity IDs |

#### Timing / Coroutines

| Method | Description |
|--------|-------------|
| `ctx:yield_ticks(n, opts)` | Pause for N ticks. `opts = { interruptible = true }` |
| `ctx:wait()` | Shorthand for `yield_ticks(1)` |

#### RNG (Deterministic)

| Method | Description |
|--------|-------------|
| `ctx:rand(min, max)` | Random integer [min, max] |
| `ctx:random_chance(percent)` | Returns boolean |

#### Context Properties

| Property | Description |
|----------|-------------|
| `ctx.caster` | Entity ID of the caster |
| `ctx.tick` | Current tick number |

### Target Resolution

Targets accept:
- `"self"` — the executing entity
- An integer entity ID (from `ctx:entities_of_type()`, `ctx.caster`, event data, etc.)

### Sandboxing

The Lua environment is sandboxed:
- **Removed:** `os`, `io`, `debug`, `dofile`, `loadfile`, `package`
- **Available:** `math`, `string`, `table`, `pairs`, `ipairs`, `type`, `tostring`, `tonumber`, `pcall`, `xpcall`, `error`, `select`, `unpack`, `next`, `coroutine`, `setmetatable`, `getmetatable`
- **RNG:** Use `ctx:rand()` for deterministic randomness (seeded from tick + entity)

### Hot-Reload

Saving `mod.lua` triggers hot-reload:
1. All active Lua coroutines for the mod are cancelled
2. All command/trigger/buff handlers are unregistered
3. `mod.lua` is re-executed
4. New handlers are registered
5. Console: `[reload] core/mod.lua reloaded`

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
- **Triggers**: Defined in Lua via `mod.on()`. Collected from all mods' `mod.lua` files.
- **Initial resources**: Merged from all mods' `[initial]` sections.
- **Init handlers**: Defined in Lua via `mod.on_init()`. Run in mod load order.
- **Library files**: Concatenated in load order.

Load order is determined by the dependency graph (topological sort, alphabetical tie-breaking).

---

## Validation

After all mods are loaded, the engine validates:

### Type Validation (`validate_type_defs()`)
- Type names must be non-empty
- Duplicate type names across mods produce a warning

### Entity Validation (`validate_entity_defs()`)
- Entity IDs must be non-empty and unique
- Unknown type references produce a warning
- Duplicate types per entity produce a warning
- Entities with multiple brain types are **rejected** and removed from all registries (configs, types, sprites, pivots) — they will not load or spawn

### Dependency Validation (`validate_dependencies()`)
- Missing dependencies cause the mod (and dependants) to be skipped with a warning
- Conflicting mods: second-loaded mod is skipped
- Circular dependencies are detected and logged as an error

### Library Validation
- `.grim` files are syntax-checked (lex + parse) at load time
- Syntax errors produce a warning (source is still prepended for graceful degradation)

### Resource Validation
- Warning if initial value exceeds defined max cap

All validation failures produce warnings (not errors) — the mod still loads with best-effort behavior.

Note: Command logic, triggers, and buff callbacks are defined in Lua (`mod.lua`) and are not validated at TOML load time. Lua errors are caught at runtime and logged.

---

## Runtime Entity Spawning

When the simulation spawns entities at runtime (via Lua `ctx:spawn()`), the engine:

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
  mod.toml          # Data: types, entities, resources, buff stats
  mod.lua           # Logic: commands, triggers, init, buff callbacks
  grimscript/       # Brain scripts for entity types
  sprites/
    summoner_atlas.png
    summoner_atlas.json
    skeleton_atlas.png
    skeleton_atlas.json
```

Its `mod.toml` defines:
- The `summoner` entity type (100 HP, speed 1) — the player-controlled entity that runs scripts
- The `skeleton` entity type (5 HP, inherits speed from `unit` type)
- Global resources: `mana` (50/100 capped), `bones` (0, capless)
- Initial resources: `mana`, `bones`
- Buff stat definitions

Its `mod.lua` defines:
- Initialization handler (spawns summoner, outputs startup messages)
- Custom commands (`help`, `trance`, `raise`, `harvest`, `pact`)
- Event triggers (e.g., `entity_died` for bone harvesting)

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

4. (Optional) Add a `mod.lua` with `mod.on_init()` to spawn entities at game start and define commands/triggers.

5. (Optional) Add resources (in `mod.toml`) and commands/triggers/buffs (in `mod.lua`).

6. (Optional) Add `[initial]` in `mod.toml` to control which resources are unlocked at game start.

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
my_pos = self.position
my_hp = self.health
my_armor = self.armor
```

### Script Execution

- Scripts compile to IR and execute in a tick-based loop
- Custom commands consume the tick — the script pauses until next tick
- `print()` does not consume the tick
- 10,000 instruction step limit per tick — exceeding it emits a warning and auto-yields with wait

---

## Adding New Capabilities (Developer Guide)

### Lua ctx Method vs IR Builtin

| | Lua ctx method | IR builtin |
|---|---|---|
| **What** | New method on the `ctx` object in Lua | New IR instruction in the sim executor |
| **Files** | 1 Rust file (`deadcode-lua/src/api.rs`) | 4–5 Rust files |
| **Use when** | New world-mutating operation for Lua command handlers | New function callable directly from GrimScript |
| **Examples** | `ctx:damage()`, `ctx:heal()`, `ctx:spawn()` | `print()`, `len()`, `percent()` |

**Rule of thumb:** Most new game mechanics should be Lua ctx methods. IR builtins are only needed for operations that GrimScript code calls directly (stdlib-level functions).

### Adding a New Lua ctx Method

1. Add a method on `CtxUserData` in `crates/deadcode-lua/src/api.rs`
2. Add the method name to the `__void_cmd_wrapper` method list in the same file
3. Use it in `mod.lua`: `ctx:my_method(args)`
4. Update this document

### Adding a New IR Builtin (Advanced)

All hardcoded game builtins (queries, actions, instant effects) have been removed. Currently all commands compile to `ActionCustom` and are handled by Lua. To add a new builtin with a dedicated IR instruction:

1. `crates/deadcode-sim/src/ir.rs` — add `Instruction` variant
2. `crates/deadcode-sim/src/executor.rs` — execute the instruction
3. `crates/deadcode-sim/src/action.rs` — add `UnitAction` variant if it yields an action
4. `crates/deadcode-sim/src/compiler/builtins.rs` — add `CommandMeta` and mapping logic so the compiler emits the dedicated IR instruction instead of `ActionCustom`
5. `mods/core/mod.toml` — add `[[commands.definitions]]` entry

---

## Internals

### Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `ModManifest` | `deadcode-app/modding.rs` | Deserialized `mod.toml` |
| `ModMeta` | `deadcode-app/modding.rs` | Mod metadata (id, name, version, depends_on, conflicts_with) |
| `TypeDef` | `deadcode-app/modding.rs` | Type definition (name, brain, stats, commands, script) |
| `EntityDef` | `deadcode-app/modding.rs` | Entity definition (id, types, sprite, pivot, stats) |
| `InitialDef` | `deadcode-app/modding.rs` | Initially available resource names |
| `ResourceDef` | `deadcode-app/modding.rs` | Resource definition (value, optional max) |
| `CommandsDef` | `deadcode-app/modding.rs` | Library file paths |
| `LoadedMod` | `deadcode-app/modding.rs` | Fully resolved mod (sprites, configs, library source) |
| `CommandDef` | `deadcode-sim/action.rs` | Command metadata (name, description, args, unlisted, kind, implicit_self) |
| `CommandKind` | `deadcode-sim/action.rs` | Command kind enum (Query, Action, Instant, Custom) |
| `CommandMeta` | `deadcode-sim/action.rs` | Compiler-facing command metadata (num_args) |
| `UnitAction` | `deadcode-sim/action.rs` | Action enum (Wait, Print, Custom) |
| `BuffDef` | `deadcode-sim/action.rs` | Buff definition (name, duration, modifiers, stacking) |
| `CommandHandler` | `deadcode-sim/action.rs` | Trait for external runtimes (Lua) to handle commands/triggers/buffs |
| `CommandHandlerResult` | `deadcode-sim/action.rs` | Handler result enum (Handled, Yielded, NotHandled) |
| `BuffCallbackType` | `deadcode-sim/action.rs` | Buff callback enum (OnApply, PerTick, OnExpire) |
| `EntityConfig` | `deadcode-sim/entity.rs` | Stat overrides applied at spawn |
| `SimEntity` | `deadcode-sim/entity.rs` | Game entity (stats map, types, owner, position, buffs) |
| `ActiveBuff` | `deadcode-sim/entity.rs` | Tracked buff on an entity |
| `LuaModRuntime` | `deadcode-lua/lib.rs` | Lua runtime, implements CommandHandler |

### Loading Flow

1. `load_mods()` scans `mods/` for directories with `mod.toml`, sorted alphabetically
2. Each manifest is parsed; sprite files, library files are read from disk
3. `resolve_mod_dependencies()` reorders mods by dependency graph
4. `validate_type_defs()` and `validate_entity_defs()` check types and entities
5. Registries (sprites, pivots, entity configs) are merged into `App`, with collision warnings
6. `collect_initial_resources()` merges resources; `collect_available_resources()` determines which are unlocked
7. `collect_buffs()` gathers buff definitions from all mods
8. `collect_library_source()` concatenates GrimScript library source in load order
9. Lua runtime loads `mod.lua` for each mod (registers commands, triggers, buff callbacks, init handlers)
10. Init handlers run in load order (spawn entities, set up initial state)
11. Command metadata, resources, and buffs are registered with `SimWorld`

### Custom Command Flow

1. Commands are defined in Lua via `mod.command(name, opts, handler)` in `mod.lua`
2. Command metadata (`CommandDef`) is registered with `SimWorld` for compiler access
3. Compiler receives `HashMap<String, CommandMeta>` (num_args). All commands compile to `ActionCustom(name)`.
4. Executor pops args and yields `UnitAction::Custom { name, args }`
5. `resolve_action()` dispatches to Lua `CommandHandler` which wraps the handler in a coroutine
6. Multi-tick commands use `ctx:yield_ticks(N)` — coroutine yields, `LuaCoroutineState` stored on entity, resumed after N ticks
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
