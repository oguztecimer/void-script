# Modding Guide

How to create mods for VOID//SCRIPT. The base game ("Necromancer") is itself a mod — the same system that loads it can load custom content.

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
energy = 60                         # Max energy
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

# --- Available Commands ---
# GrimScript commands unlocked at game start.

[commands]
initial = ["consult", "raise", "harvest", "pact"]
```

All fields in `[[entities]]` except `type` are optional. Omitted stats use engine defaults (health=100, energy=100, speed=1, etc.). Omitted `sprite` means the entity won't have a render unit.

## Entity Definitions

Entity definitions register types that can be spawned — either at startup via `[[spawn]]` or at runtime by game actions (e.g., the `raise` command creates skeletons).

### Stats

| Field | Default | Description |
|-------|---------|-------------|
| `health` | 100 | Max and current health |
| `energy` | 100 | Max and current energy |
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
        { "col": 0, "duration_ms": 200 },
        { "col": 1, "duration_ms": 200 },
        { "col": 2, "duration_ms": 200 },
        { "col": 3, "duration_ms": 200 }
      ],
      "loop_mode": "loop"
    },
    {
      "name": "walk",
      "row": 1,
      "frames": [
        { "col": 0, "duration_ms": 100 },
        { "col": 1, "duration_ms": 100 },
        { "col": 2, "duration_ms": 100 },
        { "col": 3, "duration_ms": 100 }
      ],
      "loop_mode": "loop"
    },
    {
      "name": "attack",
      "row": 2,
      "frames": [
        { "col": 0, "duration_ms": 80 },
        { "col": 1, "duration_ms": 80 },
        { "col": 2, "duration_ms": 80 }
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
- `loop_mode`: `"loop"` repeats forever, `"play_once"` plays through then returns to idle

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

The `[commands].initial` list controls which GrimScript game commands are unlocked at game start. Commands from all loaded mods are merged.

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
| `get_energy` | Get entity energy |
| `get_shield` | Get entity shield |
| `get_type` | Get entity type string |
| `get_name` | Get entity name |
| `get_owner` | Get entity owner |
| `set_target` | Set combat target |
| `get_target` | Get current target |
| `has_target` | Check if target is set |

Custom commands defined via `[[commands.definitions]]` are also gated by the `initial` list. If a command is defined but not in `initial`, players can't use it until it's unlocked at runtime.

In dev mode (`--features dev-mode`), all commands (including custom) are available regardless of the `[commands]` setting.

## Custom Command Definitions

Mods can define entirely new commands with data-driven effects using `[[commands.definitions]]`. Custom commands are always **actions** — they consume a tick when executed.

```toml
[[commands.definitions]]
name = "drain"
description = "Drain life from target"
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
| `output` | `message` | Print a message to the console |
| `damage` | `target`, `amount` | Deal damage (shield absorbs first) |
| `heal` | `target`, `amount` | Restore health (capped at max) |
| `spawn` | `entity_type`, `offset` | Spawn entity at self.position + offset |
| `modify_stat` | `target`, `stat`, `amount` | Add to a stat (can be negative) |

### Target Resolution

The `target` field in effects uses these formats:
- `"self"` — the entity executing the command
- `"arg:<name>"` — an entity reference passed as a command argument (matched by position: first arg = index 0)

### Stat Names

For `modify_stat`, valid stat names are: `health`, `energy`, `shield`, `speed`.

### Base Game Commands as Effects

The base game commands (`consult`, `raise`, `harvest`, `pact`) are defined as `[[commands.definitions]]` in `mods/necromancer/mod.toml` with data-driven effects (e.g., `raise` specifies spawn + energy cost).

> **Known issue (BUG-001):** These four commands are currently shadowed by hardcoded `ActionBuiltin` entries in the compiler. The compiler matches the hardcoded path before checking custom commands, so the mod.toml definitions (effects and costs) are registered but never executed. The hardcoded path only prints a message. See `bugs&issues.md` for details and fix options.

## Multiple Mods

Multiple mods can be active simultaneously. Entity types from all mods are merged into a shared registry. If two mods define the same entity type or command name, the first one loaded wins (alphabetical directory order). A warning is logged identifying the collision and which mod's definition was kept.

Each mod's `[[spawn]]` entries all execute, and each mod's `[commands].initial` entries are merged.

### Validation

After all mods are loaded, the engine validates:
- **Spawn entity types**: every `[[spawn]]` entry's `entity_type` must match a registered entity type. Unknown types produce a warning: `[mod:<id>] warning: spawn '<name>' references unknown entity type '<type>'`.
- **Spawn effects in custom commands**: `spawn` effects in `[[commands.definitions]]` are also checked against known entity types.
- **Stat names in `modify_stat` effects**: must be one of `health`, `energy`, `shield`, `speed`. Unknown stat names produce a warning.
- **Target references in effects**: `target` fields must be `"self"` or `"arg:<ref>"` where `<ref>` is a valid numeric index or a name matching one of the command's `args` entries. Invalid references produce a warning.
- **Cost amounts**: each cost entry must have a positive `amount`. Non-positive values produce a warning.

## Runtime Entity Spawning

When the simulation spawns new entities at runtime (e.g., the `raise` command creates a skeleton), the engine looks up the entity type in the sprite registry to create a render unit. This means a mod only needs to define the entity type in `[[entities]]` once — it will be used both for initial spawns and for runtime spawns.

If no sprite data is found for a runtime-spawned entity type, the sim entity is still created but won't have a visible sprite.

## Fallback Behavior

If the `mods/` directory doesn't exist or contains no valid mods, the game falls back to compile-time embedded assets. This ensures `cargo run` works without a `mods/` directory. The fallback provides the same content as the `necromancer` mod: summoner entity at position 500 with `consult`, `raise`, `harvest`, `pact` commands.

## The Base Game Mod

The `mods/necromancer/` directory is the base game, structured as a mod:

```
mods/necromancer/
  mod.toml
  sprites/
    summoner_atlas.png
    summoner_atlas.json
    skeleton_atlas.png
    skeleton_atlas.json
    merchant_atlas.png
    merchant_atlas.json
```

Its `mod.toml` defines three entity types (summoner, skeleton, merchant), spawns one summoner at position 500, and unlocks the four necromancer starter commands. You can edit this file to change the starting configuration without recompiling.

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

## Command Costs

Custom commands can specify resource costs that are checked and deducted before effects resolve. If the caster doesn't have enough of a resource, the command is skipped and a warning is printed to the console.

```toml
[[commands.definitions]]
name = "raise"
description = "Raise the dead"
args = []
cost = [{ type = "energy", amount = 30 }]
effects = [
  { type = "spawn", entity_type = "skeleton", offset = 1 },
  { type = "output", message = "[raise] A skeleton rises!" },
]
```

### Cost Types

| Type | Field | Description |
|------|-------|-------------|
| `energy` | `amount` | Deduct energy from the caster |
| `health` | `amount` | Deduct health from the caster |

Multiple costs can be specified — all are checked before any are deducted. If any cost cannot be paid, the entire command is skipped.

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

## Internals

The mod system lives in `crates/deadcode-app/src/modding.rs`. Key types:

| Type | Purpose |
|------|---------|
| `ModManifest` | Deserialized `mod.toml` |
| `EntityDef` | Entity type definition (type, sprite path, pivot, stats) |
| `SpawnDef` | Initial spawn definition (type, name, position) |
| `CommandsDef` | Available command list + command definitions |
| `CommandDef` | Custom command definition (name, description, args, effects, cost) — lives in `deadcode-sim/action.rs` |
| `CommandEffect` | Effect type enum (output, damage, heal, spawn, modify_stat) — lives in `deadcode-sim/action.rs` |
| `CommandCost` | Cost type enum (energy, health) — lives in `deadcode-sim/action.rs` |
| `SpriteData` | Loaded PNG bytes + JSON metadata string |
| `LoadedMod` | Fully resolved mod with sprite/pivot/config/command registries |
| `ModMeta` | Mod metadata: id, name, version, reserved dependency fields |
| `EntityConfig` | Stat overrides applied at entity spawn time (in `deadcode-sim`) |

**Loading flow:**
1. `modding::load_mods()` scans `mods/` for directories containing `mod.toml`, sorted alphabetically by directory name
2. Each manifest is parsed, sprite files are read from disk
3. Registries (sprites, pivots, entity configs) are merged into `App`, with collision warnings on duplicates
4. `modding::validate_spawns()` checks all spawn entity types and spawn effects against known types
5. `modding::validate_command_defs()` checks stat names, target references, and cost amounts in custom command definitions
6. `[[spawn]]` entries create both sim entities and render units
7. `[commands].initial` entries populate `App::available_commands`
8. `[[commands.definitions]]` entries populate `App::command_defs` and are registered with `SimWorld` (effects, arg counts, costs), with collision warnings on duplicate command names

**Custom command flow:**
1. `CommandDef` structs are parsed from TOML and collected in `App::command_defs`
2. At sim init, each def is registered via `SimWorld::register_custom_command()`
3. The compiler receives custom command arg counts, emits `ActionCustom(name)` IR instructions
4. The executor pops args and yields `UnitAction::Custom { name, args }`
5. `resolve_action()` checks costs from `SimWorld::custom_command_costs` (aggregated per resource, fails if insufficient), deducts them, then looks up effects in `SimWorld::custom_commands` and applies them
6. Custom command metadata (name, description, args) is sent to the frontend via `AvailableCommands` IPC for autocomplete

**Reserved schema fields** (parsed but not enforced):
- `depends_on: Vec<String>` — mod IDs this mod requires
- `conflicts_with: Vec<String>` — mod IDs this mod conflicts with
- `min_game_version: Option<String>` — minimum game version required
- `libraries: Vec<String>` — paths to `.grim` library files (Phase 2)

**Future phases** (not yet implemented):
- **Phase 2**: Mods provide `.grim` library files whose functions are compiled and merged into player scripts
- **Dependency resolution**: `depends_on` / `conflicts_with` enforcement, load order based on dependency graph
