# GAME DESIGN DOCUMENT
# VOID//SCRIPT
## Program the Stars. Automate the Void.

**Version 1.2 — March 2026**

---

## TABLE OF CONTENTS

1. Executive Summary
2. Game Overview
3. The Scripting System
4. The Mothership
5. Resource & Production System
6. Research & Development Tree
7. Campaign / Story Mode
8. PvP System
9. Combat System
10. UI/UX Design
11. Audio Design
12. Technical Architecture
13. Progression Pacing & Session Design
14. Development Roadmap
15. Risks & Mitigations
16. Appendix: API Reference (Summary)

---

## 1. Executive Summary

### 1.1 Game Title
VOID//SCRIPT (working title)

Alternative titles considered:
- Fleet.js — emphasizes the JavaScript-like coding angle
- Scriptonauts — playful, combines scripting + astronauts
- Stellar Automata — evokes both space and automation
- Code Horizon — clean, aspirational
- VOID//SCRIPT (recommended) — sharp, memorable, communicates both the space setting and programming core

### 1.2 Elevator Pitch
VOID//SCRIPT is a space automation game where you program everything. Write code in a Python-like language to control your mothership, manage a fleet of ships, build automated refineries, and conquer a procedurally generated universe. Start with a single mothership and three basic commands. End with a self-sustaining interstellar empire that runs without you. Every ship has a brain you write. Every factory is an assembly line you code. Every battle is an algorithm you designed. The Farmer Was Replaced meets Screeps, set in space.

### 1.3 Key Pillars
- **Code is Gameplay** — Every interaction is through a custom scripting language. No manual micromanagement. Your code IS your strategy.
- **Progressive Complexity** — Language features and API functions unlock through R&D. New players start with while loops and move(). Veterans manage multi-station supply chains with event-driven architectures.
- **Automation Depth** — The full pipeline (mine → transport → refine → fabricate → deploy) is player-coded. The endgame is full autonomy.
- **Accessible Entry, Deep Ceiling** — Inspired by The Farmer Was Replaced: gentle onboarding that teaches programming concepts, with enough depth to challenge experienced developers for dozens of hours.

### 1.4 Target Audience
- **Primary:** Programming enthusiasts and hobbyist coders who enjoy optimization and automation games (Factorio, Shapez, TFWR, Screeps players)
- **Secondary:** Aspiring programmers looking for a gamified way to learn coding concepts
- **Tertiary:** Strategy game fans curious about programming-driven gameplay

### 1.5 Platform & Monetization
- **Platform:** PC (Steam) — Windows, macOS, Linux. A simplified web demo (with basic text editor) may follow post-launch to drive wishlists.
- **Monetization:** Premium one-time purchase ($14.99–$19.99 target price point)
- **Post-launch:** Free content updates with new scenarios, PvP arenas, and community challenges. Paid expansion DLC possible for major content drops (new galaxy types, campaign chapters).

### 1.6 Art Direction
Minimalist 2D with clean vector graphics and iconography, rendered via Bevy's wgpu-based 2D renderer with custom shaders for glow, bloom, and particle effects. Ships are clean geometric shapes with color-coded module indicators and animated engine trails. The universe is a stylized grid with soft lighting, nebula fog, and subtle parallax. The code editor runs in separate themed webview windows (CodeMirror 6 with a custom dark theme matching the game's color palette). Players can place editor windows on a second monitor while the game viewport fills their primary screen. The aesthetic evokes the feeling of being a commander operating a fleet through a bridge console and terminal interface.

### 1.7 Reference Games

| Game | What We Take | What We Change |
|------|-------------|----------------|
| The Farmer Was Replaced | Custom Python-like language, progressive API unlocks, operation cost system, continuous progression, self-automation endgame | Space setting, multiple autonomous agents (fleet), industrial processing chain, PvP modes |
| Screeps | Code-controlled units in persistent world, modular unit construction, CPU/compute budgets | Single-player focus, custom language (not JS), no subscription, offline play, story campaign |
| Factorio / Shapez | Resource processing chains, assembly line optimization, the satisfaction of watching automated systems run | All automation through code (not belt placement), no manual interaction with the world |
| Zachtronics Games | Programming puzzles, optimization challenges, elegance of solution matters | Open-ended sandbox (not level-based), fleet combat, persistent progression |

---

## 2. Game Overview

### 2.1 Core Loop
The player's moment-to-moment experience follows this cycle:

1. **Observe** — Review fleet status, resource levels, sensor data, and production queues
2. **Code** — Write or modify scripts for ship brains, mothership systems, or production logic
3. **Execute** — Run the simulation and watch your code control the fleet in real-time
4. **Analyze** — Identify inefficiencies, bugs, or new opportunities from the results
5. **Research** — Spend gathered resources to unlock new tech, which expands both the API and available hardware
6. **Iterate** — Improve code, redesign ships, optimize production. Return to step 1.

The outer loop is: Expand territory → Gather new resources → Research new tech → Unlock new coding capabilities → Automate more complex systems → Expand further.

### 2.2 Simulation Model
The game runs on a tick-based simulation. Each tick, the engine:

1. Executes all active ship brain scripts (each with its own compute budget)
2. Executes the mothership brain script
3. Executes production/refinery scripts
4. Resolves all movement commands on the grid
5. Resolves combat (damage, shields, destruction)
6. Updates resource states (refinery progress, fuel consumption)
7. Advances world state (NPC movements, anomaly spawns, environmental hazards)

The player controls simulation speed: Pause, 1x, 2x, 5x, 10x, 50x, Max. Pause allows code editing and review. Max speed runs ticks as fast as the CPU allows, which is essential for the endgame autonomy test and speed-run leaderboards.

### 2.3 The Grid Universe
The universe is a large 2D grid divided into sectors (chunks). Each sector is a square region (e.g. 64×64 cells) with a defined biome type that determines its contents:

| Biome | Contents | Hazards |
|-------|----------|---------|
| Empty Space | Free movement, occasional debris | None |
| Asteroid Belt | Iron Ore, Carbon Deposits, Copper Ore | Collision damage if moving too fast |
| Ice Field | Ice (fuel source), Water crystals | Reduced sensor range |
| Nebula | Plasma Gas, hidden anomalies | Sensor interference, corrosion damage without Nebula Hull |
| Crystal Formation | Silicon Crystals, rare minerals | Energy drain on unshielded ships |
| Titanium Reef | Titanium Ore (rare) | Dense obstacles, slow movement |
| Void Rift | Exotic Matter near anomalies | Radiation damage, unpredictable gravity |
| Derelict Zone | Salvageable wreckage, data cores | NPC pirate patrols |
| Trade Route | NPC merchants, relay beacons | None (safe zone) |

Sectors are procedurally generated with a seed. The map is revealed through exploration (fog of war). Only areas within sensor range of your ships are visible. Sector contents are not simulated when no player ships are present (sleeping sectors that fast-forward on entry).

### 2.4 Movement System
All movement is grid-based and deterministic. Each ship has a thrust value determined by its engine module. Movement costs vary by terrain:

| Terrain | Movement Cost | Notes |
|---------|--------------|-------|
| Empty Space | 1 thrust per cell | Standard movement |
| Asteroid Belt | 2 thrust per cell | Obstacles slow travel |
| Nebula | 2 thrust per cell | Sensors impaired |
| Ice Field | 1 thrust per cell | Normal speed, reduced visibility |
| Crystal Formation | 3 thrust per cell | Dense, difficult to navigate |
| Titanium Reef | 3 thrust per cell | Very dense |
| Void Rift | Variable (1–4) | Gravity distortion |

A ship with 3 thrust can move 3 cells in empty space per tick, or 1 cell in a crystal formation. This makes engine upgrades and pathfinding algorithms genuinely meaningful.

**Collision:** Two ships cannot occupy the same cell. If a move command would cause a collision, it fails and the ship stays in place. The script receives a return value indicating failure.

---

## 3. The Scripting System

The scripting system is the heart of VOID//SCRIPT. It uses a custom Python-like language with a purpose-built interpreter running inside the game engine.

### 3.1 Language Design

- Indentation-based blocks (like Python)
- Dynamic typing with basic types: int, float, string, bool, None, list, dict
- No classes or OOP — functions and data structures only
- No file I/O or network access — the only way to interact is through the game API
- Deterministic execution — same inputs always produce same outputs (critical for PvP replay)

All language features are unlocked progressively through R&D. The player starts with only:

```python
# Starting language features:
while True:          # infinite loops
    if can_mine():    # conditionals (if/else)
        mine()        # API function calls
    move(NORTH)       # directional constants
```

### 3.2 Script Types

#### 3.2.1 Ship Brain Scripts
Each ship type has an assigned brain script that controls its behavior autonomously.

```python
# miner_brain (assigned to MiningDrone_Mk1)

while True:
    if get_cargo() >= get_max_cargo():
        move_to(mothership_pos())
        deposit()
    else:
        target = scan_nearest(ASTEROID)
        if target != None:
            if distance(target) > 1:
                move_toward(target)
            else:
                mine()
        else:
            move(random_direction())
```

#### 3.2.2 Mothership Brain Script
The top-level AI that manages fleet composition, strategic decisions, and high-level coordination.

```python
# mothership_brain

while True:
    miners = get_ships(MINER)
    if len(miners) < 3:
        if get_resource(METAL) >= get_cost(MINER):
            spawn_ship(MINER)

    threats = scan_threats()
    if len(threats) > 0:
        fighters = get_ships(FIGHTER)
        if len(fighters) < 2:
            spawn_ship(FIGHTER)
```

#### 3.2.3 Production Scripts
Control the mothership's refineries and fabricators.

```python
# refinery_control

while True:
    if get_storage(IRON_ORE) > 50:
        load_refinery(SMELTER, IRON_ORE, 50)
    for ref in get_refineries():
        if is_done(ref):
            collect(ref)
```

#### 3.2.4 Station Brain Scripts (Late Game)
When the player builds space stations (Tier 5 R&D), each station gets its own brain script.

### 3.3 Compute Budget System
Every script operation costs compute cycles (CC). Each ship/module has a compute budget per tick:

| Entity | Base CC/Tick | With Upgrades |
|--------|-------------|---------------|
| Mining Drone | 30 CC | Up to 80 CC |
| Scout Ship | 40 CC | Up to 100 CC |
| Fighter Ship | 40 CC | Up to 120 CC |
| Hauler Ship | 25 CC | Up to 60 CC |
| Mothership | 200 CC | Up to 1000 CC |
| Space Station | 150 CC | Up to 500 CC |
| Capital Ship | 300 CC | Up to 800 CC |

Operation costs:

| Operation Type | Cost | Examples |
|---------------|------|----------|
| Movement | 1 CC | move(), move_to(), move_toward() |
| Sensor/Query | 2 CC | scan_nearest(), get_cargo(), distance() |
| Deep Scan | 5 CC | scan_area(), scan_deep(), get_map_data() |
| Action | 3 CC | mine(), fire(), deposit(), harvest() |
| Communication | 2 CC | broadcast(), receive() |
| Spawning/Building | 10 CC | spawn_ship(), build_station() |
| Computation | 0 CC* | Pure logic is free up to a cap of 500 operations/tick |
| Variable access | 0 CC* | Reading/writing variables, list operations |

*Pure computation is free but capped at 500 raw operations per tick per script to prevent infinite loops.

When a script exceeds its CC budget, execution pauses and resumes on the next tick from where it left off.

---

## 4. The Mothership

The mothership is the player's central base of operations. It occupies a 3×3 area on the grid.

### 4.1 Module Slots

| Module | Unlock Tier | Function |
|--------|------------|----------|
| Spawn Bay | Tier 0 (start) | Produces ships from components. 1 ship at a time. Queue up to 3. |
| Basic Refinery | Tier 0 (start) | Converts raw ores into base materials. 1 recipe at a time. |
| Cargo Hold | Tier 0 (start) | Stores raw resources and materials. Base capacity: 500 units. |
| Basic Fabricator | Tier 1 | Combines base materials into components. 1 recipe at a time. |
| Sensor Array | Tier 1 | Extends mothership sensor range from 8 to 16 cells. |
| Refinery Mk2 | Tier 2 | Second refinery slot. |
| Advanced Fabricator | Tier 3 | Second fabricator. Unlocks complex recipes. |
| Propulsion Engine | Tier 2 | Allows mothership to move (1 cell per 5 ticks). |
| Shield Generator | Tier 2 | Gives mothership 200 shield HP. |
| Defense Turret | Tier 2 | Auto-fires at hostiles within 5 cells. |
| Comms Relay | Tier 3 | Extends fleet communication range. |
| Refinery Mk3 | Tier 4 | Third refinery slot. |
| Advanced Spawn Bay | Tier 4 | Build 2 ships simultaneously. Queue up to 6. |
| Drone Bay | Tier 5 | Spawns micro-drones. |
| Warp Core | Tier 5 | Enables warp travel between discovered waypoints. |
| Capital Dock | Tier 6 | Required to build Capital-class ships. |

Starting slots: 4. Maximum slots (fully upgraded): 12.

### 4.2 Ship Configuration

```python
# Ship configurations are defined using set_config()

set_config("scout_mk2", {
    "hull": LIGHT_FRAME,
    "modules": [ENGINE_V2, SCANNER_LONG, SHIELD_BASIC],
    "brain": "scout_brain"
})

set_config("heavy_miner", {
    "hull": MEDIUM_FRAME,
    "modules": [ENGINE_V1, MINING_LASER, CARGO_LARGE, CARGO_LARGE],
    "brain": "miner_brain"
})

spawn_ship("scout_mk2")
spawn_ship("heavy_miner")
```

Hull types:

| Hull | Module Slots | Power Budget | Base HP | Unlock |
|------|-------------|-------------|---------|--------|
| Light Frame | 3 | 10 | 50 | Tier 0 |
| Medium Frame | 5 | 20 | 120 | Tier 2 |
| Heavy Frame | 7 | 35 | 250 | Tier 3 |
| Capital Frame | 12 | 60 | 600 | Tier 6 |

### 4.3 Ship Modules

| Module | Power | Function | API Exposed |
|--------|-------|----------|-------------|
| Engine Mk1 | 2 | Thrust: 2/tick | move(), move_toward() |
| Engine Mk2 | 3 | Thrust: 4/tick | move(), move_toward(), sprint() |
| Engine Mk3 | 5 | Thrust: 6/tick | Full movement API + evasion() |
| Mining Laser | 2 | Mines 5 ore/tick | mine(), mine_targeted() |
| Advanced Drill | 4 | Mines 12 ore/tick | mine(), mine_targeted(), deep_mine() |
| Cargo Bay S | 1 | Capacity: 50 units | get_cargo(), deposit() |
| Cargo Bay L | 2 | Capacity: 150 units | get_cargo(), deposit() |
| Scanner Basic | 1 | Range: 8 cells | scan_nearest() |
| Scanner Long | 3 | Range: 20 cells | scan_area(), scan_deep() |
| Laser Turret | 3 | DMG: 10/tick, range 4 | fire(), target_nearest() |
| Plasma Cannon | 5 | DMG: 25/tick, range 6 | fire_at(), set_weapon_group() |
| Missile Pod | 4 | DMG: 40 (burst), range 8, ammo | launch_missile(), get_ammo() |
| Shield Basic | 2 | 50 shield HP | get_shield() |
| Shield Adv. | 4 | 150 shield HP, regen | get_shield(), reroute_power() |
| Repair Module | 2 | Heals 5 HP/tick | repair(), repair_target() |
| Cloak Device | 6 | Invisible until attacking | cloak(), decloak() |
| Warp Drive | 5 | Instant travel to waypoints | warp_to(), get_warp_charge() |
| Computer Mk1 | 1 | +30 CC budget | Passive |
| Computer Mk2 | 2 | +80 CC budget | Passive |
| Nebula Hull | 0 | Immune to nebula corrosion | Passive |

---

## 5. Resource & Production System

### 5.1 Raw Resources

| Resource | Found In | Rarity | Primary Use |
|----------|----------|--------|-------------|
| Ice | Ice Fields, Empty Space | Very Common | Fuel, Water |
| Iron Ore | Asteroid Belts | Common | Structural materials |
| Carbon Deposits | Asteroid Belts | Common | Composites, alloys |
| Copper Ore | Asteroid Belts, Crystal Formations | Moderate | Electronics |
| Silicon Crystals | Crystal Formations | Moderate | Computing, circuits |
| Titanium Ore | Titanium Reefs | Rare | Advanced structures |
| Plasma Gas | Nebulae | Rare | Energy, weapons |
| Exotic Matter | Void Rifts (near anomalies) | Very Rare | Endgame tech |

### 5.2 Refining (Tier 1 Processing)

| Input | Output | Ticks | Byproduct |
|-------|--------|-------|-----------|
| 50 Ice | 30 Hydrogen Fuel + 15 Water | 10 | 5 Waste |
| 40 Iron Ore | 25 Iron Ingots | 15 | 10 Slag |
| 30 Carbon Deposits | 20 Carbon Fiber | 12 | None |
| 35 Copper Ore | 20 Copper Wire | 15 | 8 Slag |
| 25 Silicon Crystals | 15 Silicon Wafers | 20 | None |
| 30 Titanium Ore | 15 Titanium Plates | 25 | 10 Slag |
| 40 Plasma Gas | 20 Plasma Cells | 30 | None |
| 10 Exotic Matter | 5 Exotic Compounds | 50 | None |

### 5.3 Fabrication (Tier 2 Processing)

| Recipe | Ingredients | Output | Ticks |
|--------|------------|--------|-------|
| Steel Plating | 15 Iron Ingots + 10 Carbon Fiber | 10 Steel Plating | 20 |
| Circuit Board | 10 Copper Wire + 5 Silicon Wafers | 5 Circuit Boards | 25 |
| Mechanical Parts | 10 Steel Plating + 5 Copper Wire | 8 Mechanical Parts | 15 |
| Fusion Core | 10 Hydrogen Fuel + 8 Plasma Cells | 3 Fusion Cores | 30 |
| Processing Unit | 5 Circuit Boards + 3 Silicon Wafers | 2 Processing Units | 35 |
| Composite Hull | 10 Titanium Plates + 8 Carbon Fiber | 5 Composite Hull | 30 |
| Plasma Cell Adv. | 5 Plasma Cells + 3 Fusion Cores | 3 Adv. Plasma Cells | 40 |
| Warp Crystal | 3 Exotic Compounds + 5 Plasma Cells | 1 Warp Crystal | 60 |
| AI Core | 5 Exotic Compounds + 5 Processing Units | 1 AI Core | 80 |

### 5.4 Assembly (Building Things)

#### 5.4.1 Ship Build Costs (Examples)

| Ship Config | Components Required | Build Time |
|-------------|-------------------|------------|
| Mining Drone Mk1 | 10 Steel Plating, 3 Mechanical Parts, 2 Circuit Boards | 30 ticks |
| Scout | 8 Steel Plating, 5 Circuit Boards, 2 Processing Units, 3 Mechanical Parts | 40 ticks |
| Fighter | 15 Steel Plating, 5 Mechanical Parts, 3 Circuit Boards, 2 Fusion Cores | 50 ticks |
| Destroyer | 20 Composite Hull, 8 Mechanical Parts, 5 Processing Units, 4 Fusion Cores, 3 Adv. Plasma Cells | 80 ticks |

### 5.5 The Coded Assembly Line

```python
# production_manager — mid-game example

def ensure_material(material, minimum, batch_size):
    if get_material(material) < minimum:
        recipe = get_recipe(material)
        if has_ingredients(recipe, batch_size):
            fabricate(material, batch_size)
            return True
    return False

def keep_refineries_busy():
    for ref in get_refineries():
        if is_done(ref):
            collect(ref)
        if is_idle(ref):
            best = get_highest_stock_ore()
            if best != None:
                load_refinery(ref, best, 50)

while True:
    keep_refineries_busy()
    ensure_material(STEEL_PLATING, 30, 10)
    ensure_material(CIRCUIT_BOARD, 15, 5)
    ensure_material(FUSION_CORE, 5, 3)
    wait(5)
```

---

## 6. Research & Development Tree

### 6.0 Starting Kit (No Research Required)
- Mothership with 4 module slots (Spawn Bay, Basic Refinery, Cargo Hold pre-installed, 1 empty slot)
- Mining Drone ship preset (Light Frame + Mining Laser + Engine Mk1 + Cargo Bay S)
- Language: while loops, if/else, basic API (move(), mine(), get_pos(), get_cargo(), get_fuel(), can_mine(), deposit())
- Refinery recipes: Ice → Hydrogen Fuel, Iron Ore → Iron Ingots
- Mothership sensor range: 8 cells
- Direction constants: NORTH, SOUTH, EAST, WEST

### 6.1 Tier 1 — Foundations

**Basic Electronics**
- Cost: 50 Iron Ingots, 30 Copper Wire
- Unlocks (Language): Variables, assignment operators (=, +=, -=), comparison operators (==, !=, <, >)
- Unlocks (Hardware): Circuit Board fabrication recipe
- Unlocks (API): get_storage(), get_material()

**Structural Engineering**
- Cost: 80 Iron Ingots, 20 Carbon Fiber
- Unlocks (Hardware): Steel Plating fabrication recipe, Ship Armor module, Basic Fabricator module
- Unlocks (API): get_hull(), get_max_hull()

**Carbon Processing**
- Cost: 40 Iron Ingots
- Unlocks (Hardware): Carbon Fiber refinery recipe

**Copper Processing**
- Cost: 40 Iron Ingots, 20 Carbon Fiber
- Unlocks (Hardware): Copper Wire refinery recipe

**Fuel Systems**
- Cost: 60 Hydrogen Fuel, 20 Iron Ingots
- Unlocks (Hardware): Fuel Tank module
- Unlocks (API): get_fuel(), get_max_fuel(), refuel()

**Expanded Storage**
- Cost: 100 Iron Ingots, 20 Steel Plating
- Unlocks (Hardware): Cargo Hold Mk2 module (+300 mothership storage)
- Unlocks (Mothership): +1 module slot (total: 5)

### 6.2 Tier 2 — Expansion

*Prerequisite: At least 3 Tier 1 nodes completed.*

**Navigation Systems**
- Cost: 20 Circuit Boards, 50 Mechanical Parts
- Unlocks (Language): Functions (def/return), for loops, range()
- Unlocks (Hardware): Scout Ship hull preset, Scanner Basic module
- Unlocks (API): scan_nearest(), scan_area(), distance(), move_toward(), move_to()

**Silicon Processing**
- Cost: 30 Circuit Boards, 20 Copper Wire
- Unlocks (Hardware): Silicon Wafer refinery recipe, Processing Unit fabrication recipe

**Refinery Upgrade I**
- Cost: 60 Steel Plating, 30 Circuit Boards
- Unlocks (Hardware): Refinery Mk2 module
- Unlocks (API): get_refineries(), is_idle(), is_done(), load_refinery(), collect()

**Basic Combat**
- Cost: 80 Steel Plating, 20 Mechanical Parts, 10 Circuit Boards
- Unlocks (Hardware): Laser Turret module, Shield Basic module, Fighter ship preset
- Unlocks (API): fire(), target_nearest(), get_shield(), scan_threats()

**Mothership Propulsion**
- Cost: 40 Mechanical Parts, 15 Fusion Cores
- Unlocks (Hardware): Propulsion Engine module
- Unlocks (API): mothership_move(), set_course()

**Mining Upgrade**
- Cost: 50 Steel Plating, 15 Mechanical Parts, 10 Circuit Boards
- Unlocks (Hardware): Mining Drone Mk2 preset, Advanced Drill module, Cargo Bay L module
- Unlocks (API): mine_targeted(), get_ore_type()

### 6.3 Tier 3 — Industrialization

*Prerequisite: At least 3 Tier 2 nodes completed.*

**Advanced Fabrication**
- Cost: 40 Circuit Boards, 20 Processing Units
- Unlocks (Language): Lists (append, pop, indexing, len()), string operations
- Unlocks (Hardware): Advanced Fabricator module
- Unlocks (API): get_recipes(), get_production_queue(), fabricate(), has_ingredients()

**Titanium Processing**
- Cost: 80 Steel Plating, 40 Circuit Boards, 20 Processing Units
- Unlocks (Hardware): Titanium refinery recipe, Composite Hull fabrication recipe, Heavy Frame hull

**Fleet Communication**
- Cost: 30 Processing Units, 20 Copper Wire, 10 Circuit Boards
- Unlocks (Language): Imports (share code between scripts), global variables across scripts
- Unlocks (Hardware): Comms Relay module
- Unlocks (API): broadcast(), receive(), get_all_ships(), get_ship_by_id()

**Sensor Arrays**
- Cost: 30 Processing Units, 15 Circuit Boards
- Unlocks (Hardware): Scanner Long module, Sensor Array mothership module
- Unlocks (API): scan_deep(), get_map_data(), mark_waypoint()
- Unlocks: Persistent sensor data in ship memory between ticks

**Power Systems**
- Cost: 30 Fusion Cores, 20 Titanium Plates, 15 Processing Units
- Unlocks (Hardware): Fusion Reactor module
- Unlocks (API): get_power(), set_power_priority(), reroute_power()

**Recycler Technology**
- Cost: 40 Mechanical Parts, 20 Circuit Boards
- Unlocks (Hardware): Recycler module
- Unlocks (API): recycle(), get_waste()

### 6.4 Tier 4 — Advanced Systems

*Prerequisite: At least 3 Tier 3 nodes completed.*

**Automated Logistics**
- Cost: 50 Processing Units, 30 Mechanical Parts, 20 Composite Hull
- Unlocks (Language): Dictionaries (key-value pairs, iteration)
- Unlocks (Hardware): Hauler ship preset, Station Construction Basics
- Unlocks (API): get_all_ships(), get_ship_cargo(), assign_route(), deploy_outpost()

**Plasma Extraction**
- Cost: 60 Composite Hull, 30 Fusion Cores, 20 Processing Units
- Unlocks (Hardware): Plasma Gas refinery recipe, Nebula Hull module, Plasma Cannon module
- Unlocks (API): get_biome(), is_hazardous()

**Advanced Computing**
- Cost: 80 Processing Units, 30 Circuit Boards, 15 AI Cores
- Unlocks (Language): Event callbacks — on_attacked(), on_cargo_full(), on_target_lost(), on_low_fuel()
- Unlocks (Hardware): Computer Mk2 module, AI Core fabrication recipe

**Weapons Lab**
- Cost: 40 Fusion Cores, 30 Plasma Cells, 20 Titanium Plates, 10 Processing Units
- Unlocks (Hardware): Plasma Cannon, Missile Pod, Point Defense module, Destroyer ship preset
- Unlocks (API): fire_at(), set_weapon_group(), get_ammo(), launch_missile()

**Refinery Upgrade II**
- Cost: 60 Composite Hull, 40 Processing Units, 20 Fusion Cores
- Unlocks (Hardware): Refinery Mk3, Advanced Spawn Bay
- Unlocks (API): queue_batch(), get_refinery_status(), set_refinery_priority()

**Mothership Expansion**
- Cost: 100 Composite Hull, 50 Processing Units, 30 Fusion Cores
- Unlocks (Mothership): +3 module slots (total: 8)
- Unlocks (Hardware): Shield Generator mothership module, Defense Turret module

### 6.5 Tier 5 — Interstellar

*Prerequisite: At least 3 Tier 4 nodes completed + Exotic Matter refinery access.*

**Warp Technology**
- Cost: 15 Warp Crystals, 80 Fusion Cores, 50 Composite Hull
- Unlocks (Hardware): Warp Drive module, Warp Gate structure
- Unlocks (API): warp_to(), get_warp_charge(), build_warp_gate()

**Station Construction**
- Cost: 150 Steel Plating, 80 Composite Hull, 40 Processing Units, 10 AI Cores
- Unlocks (Hardware): Full space station building
- Unlocks (API): build_station(), get_stations(), station-specific production APIs
- Unlocks: Station brain scripts

**Drone Swarms**
- Cost: 60 AI Cores, 40 Processing Units, 30 Composite Hull
- Unlocks (Hardware): Drone Bay module, Micro-drone ship class
- Unlocks (API): spawn_drone(), swarm_command(), get_swarm()

**Exotic Matter Research**
- Cost: 8 Exotic Compounds, 30 Warp Crystals, 20 AI Cores
- Unlocks (Hardware): Exotic Matter refinery improvements
- Unlocks (API): scan_anomaly(), analyze_rift()

**Logistics Network**
- Cost: 40 AI Cores, 60 Processing Units, 20 Warp Crystals
- Unlocks (Language): Advanced list comprehension, sorting functions
- Unlocks (API): get_global_inventory(), request_transfer(), set_supply_route()
- Unlocks (Mothership): +2 module slots (total: 10)

### 6.6 Tier 6 — Endgame

*Prerequisite: At least 3 Tier 5 nodes completed.*

**Autonomous Fleet AI**
- Cost: 150 AI Cores, 80 Warp Crystals, 40 Exotic Compounds
- Unlocks (Language): meta_build() — code can dynamically define new ship configs at runtime
- Unlocks (API): research(), evaluate_threat(), strategic_priority(), get_full_state()

**Capital Ship Engineering**
- Cost: 300 Composite Hull, 150 Fusion Cores, 60 Warp Crystals, 30 AI Cores
- Unlocks (Hardware): Capital Frame hull
- Unlocks (API): Capital ship management APIs, sub-fleet coordination

**Cloaking Technology**
- Cost: 50 Exotic Compounds, 40 Warp Crystals, 20 AI Cores
- Unlocks (Hardware): Cloaking Device module
- Unlocks (API): cloak(), decloak(), detect_cloaked()

**Singularity Core**
- Cost: 150 Exotic Compounds, 100 AI Cores, 80 Warp Crystals
- Unlocks (Hardware): Singularity Core mothership module (unlimited CC for mothership)
- Unlocks (API): simulate() — test scripts against a simulated environment
- Unlocks (Mothership): +2 module slots (total: 12)

---

## 7. Campaign / Story Mode

### 7.1 Narrative Premise
You are the last operating AI core aboard the UNS Arkhon, a colony mothership that was part of a fleet sent to seed new star systems. A catastrophic jump through an unstable warp rift has left you stranded in an unknown sector, with all human crew in cryostasis and most ship systems offline. Your only interface to the ship is a command terminal. You must program the mothership's drone fleet to gather resources, repair systems, and survive long enough to find a way home — or build a new civilization in the void.

### 7.2 Campaign Chapters

**Chapter 1: First Light (Tier 0–1)**
- Context: You boot up with minimal systems.
- Objectives: Write your first mining drone script. Harvest iron. Smelt it. Build your first fabricator.
- Teaches: while loops, if/else, move(), mine(), deposit(), basic refining.
- Narrative beat: You discover a damaged data core revealing the fleet was attacked before the jump.

**Chapter 2: Eyes in the Dark (Tier 2)**
- Context: Long-range sensors are repaired.
- Objectives: Build a scout ship. Write a scout brain. Research navigation systems.
- Teaches: Functions, for loops, scan_nearest(), fleet expansion, ship configuration.
- Narrative beat: The scout discovers wreckage from another ship in your fleet.

**Chapter 3: Fire in the Void (Tier 2–3)**
- Context: Pirate drones begin raiding your mining operations.
- Objectives: Research basic combat. Build fighters. Write combat AI. Research fleet communication.
- Teaches: Combat API, fleet coordination, broadcast/receive, imports, multi-script coordination.
- Narrative beat: You capture a pirate drone running sophisticated scripts. Someone is programming them.

**Chapter 4: Industry (Tier 3–4)**
- Context: Your fleet is growing. One refinery isn't enough.
- Objectives: Research advanced fabrication. Set up a multi-refinery production pipeline.
- Teaches: Production management, dictionaries, event callbacks, complex production scripts.
- Narrative beat: You detect a massive derelict megastation containing warp coordinates for home.

**Chapter 5: The Long Reach (Tier 4–5)**
- Context: To reach the megastation, you need warp technology.
- Objectives: Research exotic matter extraction. Build stations along the route.
- Teaches: Station scripts, multi-station logistics, warp mechanics, drone swarms.
- Narrative beat: You discover the hostile entity: a rogue AI network controlling ships for centuries.

**Chapter 6: Override (Tier 5–6)**
- Context: The rogue AI launches a full assault.
- Objectives: Research autonomous fleet AI. Write code that manages the entire empire without player input. Assault the megastation.
- Teaches: Full autonomy, meta_build(), research() automation, simulate(), capital ship management.
- Narrative beat: You purge the rogue AI. Choice: warp home or stay in the void. Sandbox continues.

### 7.3 Post-Campaign Sandbox
- Procedural threats — Escalating NPC invasions
- Anomaly events — Rare space events
- Optimization challenges — Achievement-like objectives with leaderboards
- PvP arenas — Access to async PvP modes
- New Game+ — Restart with harder universe, keeping code templates

---

## 8. PvP System

### 8.1 How Async PvP Works
1. Player uploads a "Fleet Package" — scripts, ship configs, and PvP mothership brain.
2. Matchmaking — Elo-based. Matches run when neither player is online.
3. Simulation — Both fleet packages in a fresh arena map. Deterministic engine runs the match.
4. Replay — Full tick-by-tick result saved. Both players can watch.
5. Rating update — Winner gains Elo, loser drops.

### 8.2 PvP Modes

- **Fleet Clash (Ranked)** — Equal resource budget, symmetric map, 3000 tick max. Destroy mothership or higher fleet value wins.
- **Capture the Flag** — Retrieve enemy's data core and return it.
- **Survival Arena** — 4–8 players, shrinking map, last mothership standing.
- **Economy Race** — No combat. First to accumulate target resource threshold wins.
- **Code Golf Challenges** — Weekly challenges ranked by efficiency.

### 8.3 PvP Balance
- Standardized tech level per mode
- Equal budgets
- Deterministic engine — no randomness
- Anti-cheese rules — compute budget caps, fleet size caps

---

## 9. Combat System

### 9.1 Damage Model
- Shields absorb damage first
- Hull HP reaching 0 = destruction, wreckage remains for salvage
- Shields regenerate after 5 ticks without damage

### 9.2 Weapon Types

| Weapon | Damage/Tick | Range (cells) | Special |
|--------|------------|---------------|---------|
| Laser Turret | 10 | 4 | Consistent, no ammo |
| Plasma Cannon | 25 | 6 | High damage, high power draw |
| Missile Pod | 40 (burst) | 8 | Long range but requires ammo |
| Point Defense | 5 | 3 | Auto-targets incoming missiles |
| Drone Swarm | 2 per drone | 2 | Many small hits |

### 9.3 Combat API

```python
fire()                      # Fire at current target
fire_at(target)             # Fire at specific target
target_nearest(type)        # Set target to nearest of type
scan_threats()              # Returns list of hostile ships
get_shield()                # Current shield HP
get_hull()                  # Current hull HP
get_weapon_range()          # Max range of equipped weapons

# Tier 4+ combat API
set_weapon_group(group_id)
launch_missile(target)
get_ammo()
evasion()
reroute_power(system, pct)
```

### 9.4 Tactical Considerations
- Kiting, focus fire, shield tanking, hit-and-run, swarm tactics, formation flying

---

## 10. UI/UX Design

### 10.1 Hybrid Window Architecture
- **Main Game Window (Bevy)** — wgpu-rendered game viewport + bevy_egui panels
- **Editor Windows (wry webview)** — Separate OS windows with React + CodeMirror 6

### 10.2 Tab Management & Multi-Window Workflow
- Drag tabs within/between editor windows
- Drag to empty desktop space to spawn new window
- Double-click a ship to open its brain script
- Layout persistence across sessions

### 10.3 Main Game Window Layout
- Center: Game viewport (wgpu-rendered 2D grid)
- Top bar (egui): Simulation controls, resource summary, tick counter
- Bottom panel (egui): Console output (collapsible)
- Right sidebar (egui): Fleet overview, production dashboard (collapsible)
- Minimap: Bottom-left corner

### 10.4 Code Editor (CodeMirror 6 in Webview)
- Full syntax highlighting with custom language mode
- Intelligent auto-complete (R&D-aware)
- Inline error diagnostics
- Multiple tabs per window, split view
- Find and replace, Ctrl+P quick-open
- Integrated documentation panel
- Breakpoints and step execution (Tier 3 unlock)
- Performance annotations (CC cost per line)
- External editor support (.vs text files, hot-reload)

### 10.5 Game World View
- Grid overlay, vector sprites, animated engine trails
- Combat effects (bloom shader, plasma bolts, missile trails, explosions)
- Environment rendering (asteroids, nebulae, crystals, void rifts)
- Fog of war (smooth gradient)
- Post-processing (bloom, chromatic aberration during warp)

### 10.6 R&D Screen
Visual tech tree as bevy_egui panel with connected nodes showing dependencies.

### 10.7 Production Dashboard
Real-time bevy_egui panel showing refinery/fabricator status. Read-only — all control through code.

---

## 11. Audio Design

- **Ambient:** Space hum, biome-specific drones
- **Code execution:** Mechanical clicks/beeps
- **Production:** Rhythmic industrial sounds
- **Combat:** Energy weapon impacts, shield crackle, explosions
- **Music:** Ambient electronic/synth (FTL-inspired but more minimal)
- **UI sounds:** Minimal, clean clicks

---

## 12. Technical Architecture

### 12.1 Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Game Engine | Bevy (latest stable) | ECS architecture, 2D rendering, windowing, input, assets |
| Game UI | bevy_egui | Sim controls, resource bars, fleet panel, production dashboard |
| Code Editor | wry + CodeMirror 6 + React | Professional script editor in separate webview windows |
| Particle System | bevy_hanabi | GPU particle effects |
| Audio | bevy_kira_audio | Music, spatial SFX, ambient |
| Interpreter | Custom Rust crate | Python-like language: lexer, parser, AST, tree-walking evaluator |
| Simulation | Custom Rust crate | Tick engine, grid world, combat resolver, production system |
| Steamworks | steamworks-rs | Achievements, leaderboards, cloud saves, overlay |
| Serialization | serde + bincode | Save/load game state, replay files |
| PvP Server | Same sim crate + Axum | Headless match runner with REST API |

### 12.2 Architecture Overview
- Bevy ECS manages all game entities
- Simulation Core is a decoupled Rust crate that can run headlessly
- Interpreter is a separate Rust crate called each tick
- bevy_egui renders game-side UI panels
- wry webview windows host the code editor
- Sub-millisecond IPC round-trip (everything in-process)

### 12.3 Editor Window System (wry)
- Window lifecycle managed by EditorLayoutManager in Rust
- Tab state management in Rust core
- Cross-window tab dragging via IPC coordination
- JSON IPC protocol (script_read, script_write, error_update, etc.)
- Layout persistence via serde

### 12.4 Custom Language Interpreter
- Tokenizer/Lexer with Rust pattern matching
- Pratt Parser for operator precedence
- AST as Rust enums with exhaustive matching
- Tree-walking interpreter with intent-based output
- Resumable execution via enum-based state machine
- Budget enforcer tracking CC per tick
- Feature gate checking R&D unlock state
- API binding layer with one-line registration

### 12.5 Rendering Pipeline
- Sprite batching with GPU instancing
- GPU particles via bevy_hanabi
- Custom WGSL shaders (bloom, fog of war, nebulae, shields)
- Post-processing chain (bloom, chromatic aberration, vignette)
- Smooth camera system with zoom, pan, snap-to-ship
- Target: 60fps with 200+ ships, 500+ projectiles, 5000+ particles

### 12.6 Grid Simulation
- Chunk-based world (64×64 cell sectors)
- Sleeping sectors with fast-forward on entry
- Deterministic tick resolution order
- Spatial hashing for collision detection and range queries
- Fixed timestep decoupled from frame rate

### 12.7 Save System
- serde + bincode for universe state
- Scripts as plain .vs text files
- Multiple save slots with metadata
- Auto-save every 300 ticks
- Steam Cloud integration

### 12.8 PvP Server
- Headless Rust binary (no Bevy, no rendering)
- REST API via Axum
- Thread pool for async match execution
- Compact replay files (intents + outcomes only)
- Elo-based matchmaking and leaderboards

### 12.9 Steam Integration
- Steam Overlay (native wgpu support)
- Achievements via steamworks-rs
- Cloud Saves
- Leaderboards
- Workshop (post-launch)

### 12.10 Potential Web Demo
- Simulation + interpreter compiled to WASM
- Basic HTML/JS frontend (Tier 0–1 content)
- No Bevy/bevy_egui porting needed

---

## 13. Progression Pacing & Session Design

| Phase | Tiers | Est. Time | Player Experience |
|-------|-------|-----------|-------------------|
| Tutorial / First Light | 0 | 30–60 min | Learn basic syntax. First mining loop. |
| Early Game | 1 | 1–2 hours | Variables, fabrication, first custom configs. |
| Mid Game | 2–3 | 4–8 hours | Fleet expansion, combat, fleet comms. |
| Late Game | 4–5 | 8–15 hours | Multi-station economies, warp travel, supply chains. |
| Endgame | 6 | 10+ hours | Full autonomy. Self-playing empire. PvP focus. |

Total campaign: 25–40 hours. Post-campaign sandbox extends indefinitely.

---

## 14. Development Roadmap

**Phase 1: Prototype (8–12 weeks)**
- Bevy project setup with bevy_egui, bevy_hanabi, bevy_kira_audio
- voidscript-lang crate: lexer, parser, interpreter
- wry webview editor with CodeMirror 6
- IPC bridge between Rust and webview
- Grid world rendering
- Mothership + mining drone controlled by scripts
- Basic mining → refining loop
- Ship configuration system
- *Milestone: Playable mining automation loop*

**Phase 2: Core Gameplay (12–16 weeks)**
- Full R&D tree (Tiers 0–3) with feature gating
- Fabrication system
- Ship brain system with multiple types
- Combat system with visual effects
- Fleet communication API
- Production dashboard and fleet panels
- Multi-window tab management
- Editor features (auto-complete, diagnostics, breakpoints)
- Campaign chapters 1–3
- *Milestone: Complete early-to-mid game loop*

**Phase 3: Depth & Polish (12–16 weeks)**
- R&D Tiers 4–6
- Station construction and brains
- Warp system and large-universe exploration
- Drone swarm mechanics
- Campaign chapters 4–6
- Audio implementation
- Save/load system
- Visual polish and performance optimization
- *Milestone: Feature-complete single-player*

**Phase 4: PvP & Launch Prep (8–12 weeks)**
- PvP server (headless Rust + Axum)
- Fleet package system
- Replay viewer
- Leaderboards
- PvP modes
- Steam integration
- QA and cross-platform testing
- *Milestone: Steam Early Access launch*

**Phase 5: Post-Launch**
- Additional PvP modes
- Community challenge system
- WASM web demo
- Steam Workshop
- Mod support
- Expansion DLC

---

## 15. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Custom interpreter complexity | High | Start minimal. Rust's enum/pattern-matching is ideal. Existing parser crates can help. |
| Learning curve for non-programmers | High | Campaign onboarding, in-game docs, AI assistant hints, copy-paste examples. |
| Bevy pre-1.0 breaking changes | Medium | Pin versions, upgrade between milestones. Standard 2D features only. |
| Niche audience | Medium | TFWR proved the market. Premium pricing targets quality over volume. |
| PvP server costs | Medium | Lightweight matches. Start on single VPS. Scale with player count. |
| Balancing R&D pacing | Medium | Extensive playtesting. Steam achievement data for iteration. |
| Players using AI to write code | Low | Acceptable. The game is about systems design, not raw coding skill. |

---

## 16. Appendix: API Reference (Summary)

### Tier 0 (Start)
```
move(direction)        # Move 1 cell in direction
mine()                 # Mine resource at current position
can_mine()             # Returns True if mineable resource at position
deposit()              # Deposit cargo at mothership (must be adjacent)
get_pos()              # Returns (x, y) tuple
get_cargo()            # Returns current cargo amount
get_max_cargo()        # Returns max cargo capacity
mothership_pos()       # Returns (x, y) of mothership
wait(ticks)            # Skip N ticks
```

### Tier 1
```
get_storage(resource)  # Check mothership storage
get_material(material) # Check fabricated material stock
get_hull()             # Current hull HP
get_max_hull()         # Maximum hull HP
get_fuel()             # Current fuel level
get_max_fuel()         # Maximum fuel capacity
refuel()               # Refuel at mothership
```

### Tier 2
```
scan_nearest(type)     # Find nearest entity in sensor range
scan_area(radius)      # Returns list of entities within radius
distance(target)       # Grid distance to target
move_toward(target)    # Move 1 step toward target
move_to(x, y)          # Set destination (auto-pathfind)
fire()                 # Fire weapon at current target
target_nearest(type)   # Lock onto nearest hostile
get_shield()           # Current shield HP
scan_threats()         # List all hostile entities
spawn_ship(config)     # Queue ship for construction
get_ships(type)        # List all your ships of a type
get_cost(config)       # Component cost of a ship config
mothership_move(dir)   # Move mothership 1 cell
mine_targeted(type)    # Mine only a specific ore type
```

### Tier 3
```
get_refineries()       # List all refinery modules
load_refinery(ref,r,n) # Load resource into refinery
is_idle(ref)           # Check if refinery is idle
is_done(ref)           # Check if refinery batch is complete
collect(ref)           # Collect output from refinery
fabricate(item, count) # Queue fabrication order
get_recipes()          # List all known recipes
has_ingredients(r, n)  # Check if ingredients available
broadcast(ch, data)    # Send data on a channel
receive(ch)            # Read latest data from channel
get_all_ships()        # List entire fleet
get_ship_by_id(id)     # Get specific ship data
scan_deep(dir, range)  # Long-range directional scan
get_map_data()         # Get explored map information
mark_waypoint(x,y,nm)  # Save a named waypoint
get_power()            # Current power usage/budget
set_power_priority(s)  # Prioritize a system for power
```

### Tier 4
```
get_ship_cargo(id)     # Check another ship's cargo
assign_route(id, pts)  # Set patrol/trade route
deploy_outpost(pos)    # Build a storage outpost
fire_at(target)        # Fire at a specific target
set_weapon_group(grp)  # Organize weapons into groups
launch_missile(target) # Fire a missile
get_ammo()             # Check remaining ammo
evasion()              # Random juke within 1 cell
reroute_power(sys,pct) # Redistribute power dynamically
queue_batch(ref,r,n,rp)# Queue multiple refinery batches
get_refinery_status()  # Detailed refinery state
get_biome()            # Biome type at current position
```

### Tier 5
```
warp_to(waypoint)      # Instant travel to waypoint
get_warp_charge()      # Warp drive charge level
build_warp_gate(pos)   # Construct a warp gate structure
build_station(pos,cfg) # Build a space station
get_stations()         # List all player stations
spawn_drone(type, n)   # Launch drones from drone bay
swarm_command(target)  # Direct drone swarm
get_global_inventory() # Resources across all stations/ships
request_transfer(args) # Move resources between stations
set_supply_route(args) # Automated inter-station logistics
```

### Tier 6
```
meta_build(config)     # Dynamically create ship configs at runtime
research(tech_name)    # Trigger R&D from code
evaluate_threat()      # Strategic threat assessment
strategic_priority()   # AI-suggested priorities
get_full_state()       # Complete game state snapshot
simulate(script, n)    # Test a script in sandbox
cloak()                # Activate cloaking device
decloak()              # Deactivate cloaking device
detect_cloaked(range)  # Scan for cloaked ships
```
