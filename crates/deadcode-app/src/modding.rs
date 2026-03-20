//! Mod loading system.
//!
//! Loads `mod.toml` manifests from the `mods/` directory. Each mod defines
//! entity types (with sprites and stats), initial spawn definitions, and
//! available commands. The base game ("core") is itself a mod.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use deadcode_desktop::animation::{SKELETON_ATLAS_PNG, skeleton_atlas_json};
use deadcode_sim::action::{BehaviorDef, BuffDef, CommandDef, CommandEffect, TriggerDef};
use deadcode_sim::entity::EntityConfig;

// ---------------------------------------------------------------------------
// Manifest types (deserialized from mod.toml)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ModManifest {
    #[serde(rename = "mod")]
    pub meta: ModMeta,
    #[serde(default)]
    pub entities: Vec<EntityDef>,
    #[serde(default)]
    pub spawn: Vec<SpawnDef>,
    #[serde(default)]
    pub commands: Option<CommandsDef>,
    #[serde(default)]
    pub initial: Option<InitialDef>,
    /// Global resources: name → definition (plain int for capless, or {value, max} for capped).
    #[serde(default)]
    pub resources: HashMap<String, ResourceDef>,
    /// Event-driven triggers that fire effects when game events match.
    #[serde(default)]
    pub triggers: Vec<TriggerDef>,
    /// Buff definitions (temporary stat modifiers with automatic expiry).
    #[serde(default)]
    pub buffs: Vec<BuffDef>,
}

/// A resource definition: either a plain integer (capless) or `{ value, max }` (capped).
#[derive(Debug, Clone)]
pub struct ResourceDef {
    pub value: i64,
    pub max: Option<i64>,
}

impl<'de> Deserialize<'de> for ResourceDef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;

        struct ResourceDefVisitor;

        impl<'de> de::Visitor<'de> for ResourceDefVisitor {
            type Value = ResourceDef;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "an integer or {{ value, max }}")
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<ResourceDef, E> {
                Ok(ResourceDef { value: v, max: None })
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<ResourceDef, E> {
                Ok(ResourceDef { value: v as i64, max: None })
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<ResourceDef, A::Error> {
                let mut value = None;
                let mut max = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "value" => value = Some(map.next_value::<i64>()?),
                        "max" => max = Some(map.next_value::<i64>()?),
                        _ => { let _ = map.next_value::<de::IgnoredAny>()?; }
                    }
                }
                Ok(ResourceDef {
                    value: value.unwrap_or(0),
                    max,
                })
            }
        }

        deserializer.deserialize_any(ResourceDefVisitor)
    }
}

/// The `[initial]` section: commands, resources, and effects available at game start.
#[derive(Debug, Deserialize, Default)]
pub struct InitialDef {
    /// Initially available command names.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Initially available resource names. If empty, all defined resources are available.
    #[serde(default)]
    pub resources: Vec<String>,
    /// Effects to run on first game open.
    #[serde(default)]
    pub effects: Vec<CommandEffect>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ModMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    pub min_game_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EntityDef {
    #[serde(rename = "type")]
    pub entity_type: String,
    /// Sprite path relative to mod dir (without extension). Expects .png + .json pair.
    pub sprite: Option<String>,
    /// Sprite pivot [x, y].
    #[serde(default)]
    pub pivot: Option<[f32; 2]>,
    pub health: Option<i64>,
    pub speed: Option<i64>,
    pub attack_damage: Option<i64>,
    pub attack_range: Option<i64>,
    pub attack_cooldown: Option<i64>,
    pub shield: Option<i64>,
    /// Autonomous behaviors for non-scripted entities of this type.
    #[serde(default)]
    pub behaviors: Vec<BehaviorDef>,
    /// Mod-defined custom stats (e.g., armor = 5, crit_chance = 10).
    #[serde(default)]
    pub custom_stats: HashMap<String, i64>,
}

impl EntityDef {
    /// Convert to a sim `EntityConfig` for stat overrides.
    pub fn to_entity_config(&self) -> EntityConfig {
        EntityConfig {
            health: self.health,
            speed: self.speed,
            attack_damage: self.attack_damage,
            attack_range: self.attack_range,
            attack_cooldown: self.attack_cooldown,
            shield: self.shield,
            custom_stats: self.custom_stats.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SpawnDef {
    pub entity_type: String,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Deserialize)]
pub struct CommandsDef {
    #[serde(default)]
    pub definitions: Vec<CommandDef>,
    /// Reserved: paths to .grim library files (Phase 2, not yet loaded).
    #[serde(default)]
    #[allow(dead_code)]
    pub libraries: Vec<String>,
}

// ---------------------------------------------------------------------------
// Sprite data: PNG bytes + JSON metadata string
// ---------------------------------------------------------------------------

pub struct SpriteData {
    pub png: Vec<u8>,
    pub json: String,
}

// ---------------------------------------------------------------------------
// Loaded mod: manifest + resolved sprite data
// ---------------------------------------------------------------------------

pub struct LoadedMod {
    pub manifest: ModManifest,
    /// Entity type → sprite data (PNG bytes + JSON string).
    pub sprites: HashMap<String, SpriteData>,
    /// Entity type → pivot [x, y].
    pub pivots: HashMap<String, [f32; 2]>,
    /// Entity type → entity config (stat overrides).
    pub entity_configs: HashMap<String, EntityConfig>,
    /// Command name → command definition.
    pub command_defs: HashMap<String, CommandDef>,
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Try to load the mod at `mod_dir`. Returns `None` if `mod.toml` doesn't exist.
fn load_mod_from_dir(mod_dir: &Path) -> Option<LoadedMod> {
    let manifest_path = mod_dir.join("mod.toml");
    let toml_str = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: ModManifest = toml::from_str(&toml_str)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", manifest_path.display()));

    let mut sprites = HashMap::new();
    let mut pivots = HashMap::new();
    let mut entity_configs = HashMap::new();
    let mut command_defs = HashMap::new();

    // Parse command definitions.
    if let Some(cmds) = &manifest.commands {
        for def in &cmds.definitions {
            command_defs.insert(def.name.clone(), def.clone());
        }
    }

    for entity_def in &manifest.entities {
        // Load sprite files if a sprite path is specified.
        if let Some(sprite_path) = &entity_def.sprite {
            let png_path = mod_dir.join(format!("{sprite_path}.png"));
            let json_path = mod_dir.join(format!("{sprite_path}.json"));

            if png_path.exists() && json_path.exists() {
                let png = std::fs::read(&png_path)
                    .unwrap_or_else(|e| panic!("Failed to read {}: {e}", png_path.display()));
                let json = std::fs::read_to_string(&json_path)
                    .unwrap_or_else(|e| panic!("Failed to read {}: {e}", json_path.display()));
                sprites.insert(entity_def.entity_type.clone(), SpriteData { png, json });
            } else {
                eprintln!(
                    "[mod] warning: sprite files not found for '{}': {} / {}",
                    entity_def.entity_type,
                    png_path.display(),
                    json_path.display()
                );
            }
        }

        if let Some(pivot) = entity_def.pivot {
            pivots.insert(entity_def.entity_type.clone(), pivot);
        }

        entity_configs.insert(entity_def.entity_type.clone(), entity_def.to_entity_config());
    }

    Some(LoadedMod {
        manifest,
        sprites,
        pivots,
        entity_configs,
        command_defs,
    })
}

/// Load mods from the `mods/` directory. Falls back to embedded assets if
/// the directory doesn't exist or contains no valid mods.
pub fn load_mods(mods_dir: &Path) -> Vec<LoadedMod> {
    let mut loaded = Vec::new();

    if mods_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(mods_dir) {
            let mut dirs: Vec<_> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .collect();
            dirs.sort_by_key(|e| e.file_name());
            for entry in dirs {
                let path = entry.path();
                if let Some(m) = load_mod_from_dir(&path) {
                    eprintln!("[mod] loaded: {} v{}", m.manifest.meta.name, m.manifest.meta.version);
                    loaded.push(m);
                }
            }
        }
    }

    if loaded.is_empty() {
        eprintln!("[mod] no mods found, using embedded fallback");
        loaded.push(embedded_fallback());
    }

    loaded
}

/// Build a `LoadedMod` from the compile-time embedded assets (the same
/// behavior as before modding support was added).
fn embedded_fallback() -> LoadedMod {
    let mut sprites = HashMap::new();
    let mut pivots = HashMap::new();
    let mut entity_configs = HashMap::new();

    sprites.insert("skeleton".into(), SpriteData {
        png: SKELETON_ATLAS_PNG.to_vec(),
        json: skeleton_atlas_json(),
    });
    pivots.insert("skeleton".into(), [24.0, 0.0]);

    let manifest = ModManifest {
        meta: ModMeta {
            id: "core".into(),
            name: "Core".into(),
            version: "0.1.0".into(),
            depends_on: vec![],
            conflicts_with: vec![],
            min_game_version: None,
        },
        entities: vec![],
        spawn: vec![],
        commands: Some(CommandsDef {
            definitions: vec![],
            libraries: vec![],
        }),
        initial: Some(InitialDef {
            commands: vec![
                "help".into(),
                "raise".into(),
                "harvest".into(),
                "pact".into(),
            ],
            resources: vec!["bones".into()],
            effects: vec![
                CommandEffect::Output { message: "The dead stir beneath your feet".into() },
                CommandEffect::Output { message: "Call for <hl>help()</hl> to hear them speak".into() },
            ],
        }),
        resources: {
            let mut r = HashMap::new();
            r.insert("bones".into(), ResourceDef { value: 0, max: None });
            r
        },
        triggers: Vec::new(),
        buffs: Vec::new(),
    };

    entity_configs.insert("skeleton".into(), EntityConfig {
        health: Some(50),
        speed: Some(2),
        ..Default::default()
    });

    LoadedMod {
        manifest,
        sprites,
        pivots,
        entity_configs,
        command_defs: HashMap::new(),
    }
}

/// Resolve the mods directory path (next to the executable's working dir).
pub fn mods_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join("mods")
}

/// Collect all command definitions from loaded mods.
pub fn collect_command_defs(mods: &[LoadedMod]) -> HashMap<String, CommandDef> {
    let mut defs = HashMap::new();
    for m in mods {
        for (name, def) in &m.command_defs {
            if defs.contains_key(name) {
                eprintln!(
                    "[mod] warning: command '{}' already defined, skipping duplicate from '{}'",
                    name, m.manifest.meta.id
                );
            } else {
                defs.insert(name.clone(), def.clone());
            }
        }
    }
    defs
}

/// Recursively collect all effects from a list, including those nested inside
/// `If` branches and `StartChannel` phases.
fn collect_all_effects_recursive<'a>(effects: &'a [CommandEffect], out: &mut Vec<&'a CommandEffect>) {
    for effect in effects {
        out.push(effect);
        match effect {
            CommandEffect::If { then_effects, otherwise, .. } => {
                collect_all_effects_recursive(then_effects, out);
                collect_all_effects_recursive(otherwise, out);
            }
            CommandEffect::StartChannel { phases } => {
                for phase in phases {
                    collect_all_effects_recursive(&phase.on_start, out);
                    collect_all_effects_recursive(&phase.per_update, out);
                }
            }
            _ => {}
        }
    }
}

/// Validate that spawn definitions reference known entity types.
pub fn validate_spawns(mods: &[LoadedMod], known_types: &HashSet<String>) {
    for m in mods {
        for spawn in &m.manifest.spawn {
            if !known_types.contains(&spawn.entity_type) {
                eprintln!(
                    "[mod:{}] warning: spawn '{}' references unknown entity type '{}'",
                    m.manifest.meta.id, spawn.name, spawn.entity_type
                );
            }
        }
        // Also validate spawn effects in custom command definitions (effects + phases),
        // recursively including If branches and StartChannel phases.
        if let Some(cmds) = &m.manifest.commands {
            for def in &cmds.definitions {
                let mut all_effects = Vec::new();
                collect_all_effects_recursive(&def.effects, &mut all_effects);
                for phase in &def.phases {
                    collect_all_effects_recursive(&phase.on_start, &mut all_effects);
                    collect_all_effects_recursive(&phase.per_update, &mut all_effects);
                }
                for effect in all_effects {
                    let referenced_type = match effect {
                        CommandEffect::Spawn { entity_type, .. }
                        | CommandEffect::Sacrifice { entity_type, .. } => Some(entity_type.as_str()),
                        _ => None,
                    };
                    if let Some(entity_type) = referenced_type {
                        if !known_types.contains(entity_type) {
                            eprintln!(
                                "[mod:{}] warning: command '{}' references unknown entity type '{}'",
                                m.manifest.meta.id, def.name, entity_type
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Validate a list of effects against known stat names, target references, and arg names.
/// Recurses into `If` branches and `StartChannel` phases.
fn validate_effects(
    effects: &[CommandEffect],
    args: &[String],
    cmd_name: &str,
    mod_id: &str,
    valid_stats: &HashSet<&str>,
) {
    for effect in effects {
        // Validate stat names in ModifyStat and UseResource effects.
        let stat_name = match effect {
            CommandEffect::ModifyStat { stat, .. }
            | CommandEffect::UseResource { stat, .. } => Some(stat.as_str()),
            _ => None,
        };
        if let Some(stat) = stat_name {
            if !valid_stats.contains(stat) {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' references unknown stat '{stat}' \
                     (valid: health, energy, shield, speed)",
                );
            }
        }
        // Validate UseResource amounts (only check fixed values).
        if let CommandEffect::UseResource { stat, amount } = effect {
            if let deadcode_sim::action::DynInt::Fixed(v) = amount {
                if *v <= 0 {
                    eprintln!(
                        "[mod:{mod_id}] warning: command '{cmd_name}' has non-positive use_resource amount {v} for {stat}",
                    );
                }
            }
        }
        // Validate target strings in effects that have them.
        let target_str = match effect {
            CommandEffect::Damage { target, .. }
            | CommandEffect::Heal { target, .. }
            | CommandEffect::ModifyStat { target, .. }
            | CommandEffect::ApplyBuff { target, .. }
            | CommandEffect::RemoveBuff { target, .. }
            | CommandEffect::ModifyCustomStat { target, .. } => Some(target.as_str()),
            _ => None,
        };
        if let Some(target) = target_str {
            validate_target(target, args, cmd_name, mod_id);
        }
        // Validate condition in If effects, then recurse into both branches.
        if let CommandEffect::If { condition, then_effects, otherwise } = effect {
            validate_condition(condition, cmd_name, mod_id, valid_stats);
            validate_effects(then_effects, args, cmd_name, mod_id, valid_stats);
            validate_effects(otherwise, args, cmd_name, mod_id, valid_stats);
        }
        // Validate StartChannel phases.
        if let CommandEffect::StartChannel { phases } = effect {
            for (i, phase) in phases.iter().enumerate() {
                if phase.ticks <= 0 {
                    eprintln!(
                        "[mod:{mod_id}] warning: command '{cmd_name}' start_channel phase {i} has non-positive ticks ({})",
                        phase.ticks
                    );
                }
                if phase.update_interval <= 0 {
                    eprintln!(
                        "[mod:{mod_id}] warning: command '{cmd_name}' start_channel phase {i} has non-positive update_interval ({})",
                        phase.update_interval
                    );
                }
                validate_effects(&phase.on_start, args, cmd_name, mod_id, valid_stats);
                validate_effects(&phase.per_update, args, cmd_name, mod_id, valid_stats);
            }
        }
    }
}

/// Validate a condition's fields.
fn validate_condition(
    condition: &deadcode_sim::action::Condition,
    cmd_name: &str,
    mod_id: &str,
    valid_stats: &HashSet<&str>,
) {
    match condition {
        deadcode_sim::action::Condition::Resource { resource, .. } => {
            if resource.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has if-condition with empty resource name",
                );
            }
        }
        deadcode_sim::action::Condition::EntityCount { entity_type, .. } => {
            if entity_type.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has if-condition with empty entity_type",
                );
            }
        }
        deadcode_sim::action::Condition::Stat { stat, .. } => {
            if !valid_stats.contains(stat.as_str()) {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has if-condition referencing unknown stat '{stat}' \
                     (valid: health, shield, speed)",
                );
            }
        }
        deadcode_sim::action::Condition::HasBuff { buff } => {
            if buff.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has if-condition with empty buff name",
                );
            }
        }
        deadcode_sim::action::Condition::RandomChance { percent } => {
            if *percent <= 0 || *percent > 100 {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has random_chance with percent={percent} (expected 1-100)",
                );
            }
        }
        deadcode_sim::action::Condition::And { conditions }
        | deadcode_sim::action::Condition::Or { conditions } => {
            for sub in conditions {
                validate_condition(sub, cmd_name, mod_id, valid_stats);
            }
        }
        deadcode_sim::action::Condition::CustomStat { stat, .. } => {
            if stat.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has custom_stat condition with empty stat name",
                );
            }
        }
    }
}

/// Validate custom command definitions at load time.
///
/// Checks stat names in `ModifyStat`/`UseResource` effects, `arg:` target references.
/// For phased commands: validates mutual exclusivity with `effects`, phase ticks > 0,
/// and effects within `on_start`/`per_update` lists.
pub fn validate_command_defs(mods: &[LoadedMod]) {
    let valid_stats: HashSet<&str> = ["health", "shield", "speed"].into_iter().collect();

    for m in mods {
        let Some(cmds) = &m.manifest.commands else { continue };
        let mod_id = &m.manifest.meta.id;
        for def in &cmds.definitions {
            // Mutual exclusivity: warn if both effects and phases are non-empty.
            if !def.effects.is_empty() && !def.phases.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{}' has both 'effects' and 'phases' — \
                     'phases' takes precedence",
                    def.name
                );
            }

            // Validate phases.
            for (i, phase) in def.phases.iter().enumerate() {
                if phase.ticks <= 0 {
                    eprintln!(
                        "[mod:{mod_id}] warning: command '{}' phase {i} has non-positive ticks ({})",
                        def.name, phase.ticks
                    );
                }
                if phase.update_interval <= 0 {
                    eprintln!(
                        "[mod:{mod_id}] warning: command '{}' phase {i} has non-positive update_interval ({})",
                        def.name, phase.update_interval
                    );
                }
                validate_effects(&phase.on_start, &def.args, &def.name, mod_id, &valid_stats);
                validate_effects(&phase.per_update, &def.args, &def.name, mod_id, &valid_stats);
            }

            // Validate instant effects (reuses validate_effects which handles If/StartChannel recursion).
            validate_effects(&def.effects, &def.args, &def.name, mod_id, &valid_stats);
        }
    }
}

fn validate_target(target: &str, args: &[String], cmd_name: &str, mod_id: &str) {
    if target == "self" {
        return;
    }
    if let Some(arg_ref) = target.strip_prefix("arg:") {
        // Numeric index.
        if let Ok(idx) = arg_ref.parse::<usize>() {
            if idx >= args.len() {
                eprintln!(
                    "[mod:{}] warning: command '{}' effect references arg index {} but only {} args defined",
                    mod_id, cmd_name, idx, args.len()
                );
            }
            return;
        }
        // Named arg.
        if !args.contains(&arg_ref.to_string()) {
            eprintln!(
                "[mod:{}] warning: command '{}' effect references unknown arg '{}' (available: {:?})",
                mod_id, cmd_name, arg_ref, args
            );
        }
        return;
    }
    eprintln!(
        "[mod:{}] warning: command '{}' effect has invalid target '{}' (expected 'self' or 'arg:<name>')",
        mod_id, cmd_name, target
    );
}

/// Collect initial commands from all loaded mods, preserving insertion order.
pub fn collect_initial_commands(mods: &[LoadedMod]) -> Vec<String> {
    let mut commands = Vec::new();
    let mut seen = HashSet::new();
    for m in mods {
        if let Some(initial) = &m.manifest.initial {
            for cmd in &initial.commands {
                if seen.insert(cmd.clone()) {
                    commands.push(cmd.clone());
                }
            }
        }
    }
    if commands.is_empty() {
        // Default fallback
        commands.extend(["consult", "raise", "harvest", "pact"].iter().map(|s| s.to_string()));
    }
    commands
}

/// Collected resource definitions: values and optional caps.
pub struct CollectedResources {
    pub values: deadcode_sim::IndexMap<String, i64>,
    pub caps: std::collections::HashMap<String, i64>,
}

/// Collect global resources from all loaded mods, merging them.
/// Duplicate resource names: first-defined wins (with a warning).
pub fn collect_initial_resources(mods: &[LoadedMod]) -> CollectedResources {
    let mut values = deadcode_sim::IndexMap::new();
    let mut caps = std::collections::HashMap::new();
    for m in mods {
        for (name, def) in &m.manifest.resources {
            if values.contains_key(name) {
                eprintln!(
                    "[mod] warning: resource '{}' already defined, skipping duplicate from '{}'",
                    name, m.manifest.meta.id
                );
            } else {
                values.insert(name.clone(), def.value);
                if let Some(max) = def.max {
                    caps.insert(name.clone(), max);
                }
            }
        }
    }
    CollectedResources { values, caps }
}

/// Collect initially available resource names from all loaded mods.
/// If a mod has no `initial.resources` list, all of its defined resources are available.
pub fn collect_available_resources(mods: &[LoadedMod]) -> Vec<String> {
    let mut available = Vec::new();
    let mut seen = HashSet::new();
    for m in mods {
        let initial_resources = m.manifest.initial.as_ref().map(|i| &i.resources);
        let has_explicit_list = initial_resources.map_or(false, |r| !r.is_empty());
        if has_explicit_list {
            for name in initial_resources.unwrap() {
                if seen.insert(name.clone()) {
                    available.push(name.clone());
                }
            }
        } else {
            // No explicit initial list → all defined resources are available.
            for name in m.manifest.resources.keys() {
                if seen.insert(name.clone()) {
                    available.push(name.clone());
                }
            }
        }
    }
    available
}

/// Collect initial effects from all loaded mods (in load order).
pub fn collect_initial_effects(mods: &[LoadedMod]) -> Vec<CommandEffect> {
    let mut effects = Vec::new();
    for m in mods {
        if let Some(initial) = &m.manifest.initial {
            effects.extend(initial.effects.iter().cloned());
        }
    }
    effects
}

/// Collect triggers from all loaded mods (in load order).
pub fn collect_triggers(mods: &[LoadedMod]) -> Vec<TriggerDef> {
    let mut triggers = Vec::new();
    for m in mods {
        triggers.extend(m.manifest.triggers.iter().cloned());
    }
    triggers
}

/// Collect buff definitions from all loaded mods.
pub fn collect_buffs(mods: &[LoadedMod]) -> Vec<BuffDef> {
    let mut buffs = Vec::new();
    let mut seen = HashSet::new();
    for m in mods {
        for buff in &m.manifest.buffs {
            if seen.contains(&buff.name) {
                eprintln!(
                    "[mod] warning: buff '{}' already defined, skipping duplicate from '{}'",
                    buff.name, m.manifest.meta.id
                );
            } else {
                seen.insert(buff.name.clone());
                buffs.push(buff.clone());
            }
        }
    }
    buffs
}

/// Validate buff definitions at load time.
pub fn validate_buffs(mods: &[LoadedMod]) {
    let valid_stats: HashSet<&str> = ["health", "shield", "speed", "attack_damage", "attack_range"]
        .into_iter().collect();

    for m in mods {
        let mod_id = &m.manifest.meta.id;
        for buff in &m.manifest.buffs {
            if buff.name.is_empty() {
                eprintln!("[mod:{mod_id}] warning: buff has empty name");
            }
            if buff.duration <= 0 {
                eprintln!(
                    "[mod:{mod_id}] warning: buff '{}' has non-positive duration ({})",
                    buff.name, buff.duration
                );
            }
            for stat in buff.modifiers.keys() {
                if !valid_stats.contains(stat.as_str()) {
                    eprintln!(
                        "[mod:{mod_id}] warning: buff '{}' references unknown stat '{stat}' \
                         (valid: health, shield, speed, attack_damage, attack_range)",
                        buff.name
                    );
                }
            }
            // Validate effect lists.
            let effect_ctx = format!("buff '{}'", buff.name);
            let valid_effect_stats: HashSet<&str> = ["health", "shield", "speed"].into_iter().collect();
            validate_effects(&buff.per_tick, &[], &effect_ctx, mod_id, &valid_effect_stats);
            validate_effects(&buff.on_apply, &[], &effect_ctx, mod_id, &valid_effect_stats);
            validate_effects(&buff.on_expire, &[], &effect_ctx, mod_id, &valid_effect_stats);
        }
    }
}

/// Collect entity behaviors from all loaded mods.
/// Returns a map from entity type to behavior definitions.
pub fn collect_behaviors(mods: &[LoadedMod]) -> HashMap<String, Vec<BehaviorDef>> {
    let mut behaviors = HashMap::new();
    for m in mods {
        for entity_def in &m.manifest.entities {
            if !entity_def.behaviors.is_empty() {
                behaviors.insert(entity_def.entity_type.clone(), entity_def.behaviors.clone());
            }
        }
    }
    behaviors
}

/// Validate entity behavior definitions at load time.
pub fn validate_behaviors(mods: &[LoadedMod]) {
    let valid_stats: HashSet<&str> = ["health", "shield", "speed"].into_iter().collect();

    for m in mods {
        let mod_id = &m.manifest.meta.id;
        for entity_def in &m.manifest.entities {
            for (i, behavior) in entity_def.behaviors.iter().enumerate() {
                let ctx = format!("entity '{}' behavior {i}", entity_def.entity_type);

                // Validate cooldown.
                if behavior.cooldown < 0 {
                    eprintln!(
                        "[mod:{mod_id}] warning: {ctx} has negative cooldown ({})",
                        behavior.cooldown
                    );
                }

                // Validate behavior action.
                match &behavior.action {
                    deadcode_sim::action::BehaviorAction::FleeWhenLow { stat, .. } => {
                        if !valid_stats.contains(stat.as_str()) {
                            eprintln!(
                                "[mod:{mod_id}] warning: {ctx} references unknown stat '{stat}' \
                                 (valid: health, shield, speed)"
                            );
                        }
                    }
                    deadcode_sim::action::BehaviorAction::RunEffects { effects } => {
                        validate_effects(effects, &[], &ctx, mod_id, &valid_stats);
                    }
                    _ => {}
                }

                // Validate conditions.
                for condition in &behavior.conditions {
                    validate_condition(condition, &ctx, mod_id, &valid_stats);
                }
            }
        }
    }
}

/// Validate trigger definitions at load time.
///
/// Checks event names, filter fields, conditions, and effects within triggers.
pub fn validate_triggers(mods: &[LoadedMod]) {
    let valid_events: HashSet<&str> = [
        "entity_died", "entity_spawned", "entity_damaged",
        "resource_changed", "command_used", "tick_interval",
        "channel_completed", "channel_interrupted",
    ].into_iter().collect();

    let valid_stats: HashSet<&str> = ["health", "shield", "speed"].into_iter().collect();

    for m in mods {
        let mod_id = &m.manifest.meta.id;
        for (i, trigger) in m.manifest.triggers.iter().enumerate() {
            // Validate event name.
            if !valid_events.contains(trigger.event.as_str()) {
                eprintln!(
                    "[mod:{mod_id}] warning: trigger {i} references unknown event '{}'",
                    trigger.event
                );
            }

            // tick_interval requires a positive interval filter.
            if trigger.event == "tick_interval" {
                match trigger.filter.interval {
                    None | Some(0) => {
                        eprintln!(
                            "[mod:{mod_id}] warning: trigger {i} (tick_interval) missing or zero interval filter"
                        );
                    }
                    Some(v) if v < 0 => {
                        eprintln!(
                            "[mod:{mod_id}] warning: trigger {i} (tick_interval) has negative interval ({v})"
                        );
                    }
                    _ => {}
                }
            }

            // Validate conditions.
            for condition in &trigger.conditions {
                validate_condition(
                    condition,
                    &format!("trigger {i}"),
                    mod_id,
                    &valid_stats,
                );
            }

            // Validate effects (reuses existing recursive validation).
            validate_effects(
                &trigger.effects,
                &[], // triggers have no args
                &format!("trigger {i}"),
                mod_id,
                &valid_stats,
            );
        }
    }
}
