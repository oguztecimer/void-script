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

The game scans `mods/` at startup and loads every directory that contains a valid `mod.toml`. If no mods are found, the game falls back to embedded assets (identical to the pre-modding behavior).

## mod.toml Reference

```toml
[mod]
id = "my-mod"           # Unique identifier (lowercase, no spaces)
name = "My Mod"         # Display name
version = "0.1.0"       # Semver version string

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

Stdlib functions (`print`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `type`) are always available regardless of this setting.

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
| `consult` | Necromancer action |
| `raise` | Summon a skeleton |
| `harvest` | Harvest resources |
| `pact` | Form a pact |

In dev mode (`--features dev-mode`), all commands are available regardless of the `[commands]` setting.

## Multiple Mods

Multiple mods can be active simultaneously. Entity types from all mods are merged into a shared registry. If two mods define the same entity type, the first one loaded wins (directory iteration order).

Each mod's `[[spawn]]` entries all execute, and each mod's `[commands].initial` entries are merged.

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

## Internals

The mod system lives in `crates/deadcode-app/src/modding.rs`. Key types:

| Type | Purpose |
|------|---------|
| `ModManifest` | Deserialized `mod.toml` |
| `EntityDef` | Entity type definition (type, sprite path, pivot, stats) |
| `SpawnDef` | Initial spawn definition (type, name, position) |
| `CommandsDef` | Available command list |
| `SpriteData` | Loaded PNG bytes + JSON metadata string |
| `LoadedMod` | Fully resolved mod with sprite/pivot/config registries |
| `EntityConfig` | Stat overrides applied at entity spawn time (in `deadcode-sim`) |

**Loading flow:**
1. `modding::load_mods()` scans `mods/` for directories containing `mod.toml`
2. Each manifest is parsed, sprite files are read from disk
3. Registries (sprites, pivots, entity configs) are merged into `App`
4. `[[spawn]]` entries create both sim entities and render units
5. `[commands].initial` entries populate `App::available_commands`

**Future phases** (not yet implemented):
- **Phase 2**: Mods provide `.grim` library files whose functions are compiled and merged into player scripts
- **Phase 3**: `CustomAction` IR instruction + action handler registry for truly new game mechanics defined by mods
