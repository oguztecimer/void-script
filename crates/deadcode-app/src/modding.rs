//! Mod loading system.
//!
//! Loads `mod.toml` manifests from the `mods/` directory. Each mod defines
//! entity types (with sprites and stats), initial spawn definitions, and
//! available commands. The base game ("core") is itself a mod.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use deadcode_desktop::animation::{
    SUMMONER_ATLAS_PNG, summoner_atlas_json,
    SKELETON_ATLAS_PNG, skeleton_atlas_json,
};
use deadcode_sim::action::{CommandDef, CommandEffect};
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
}

#[derive(Debug, Deserialize)]
pub struct InitialDef {
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
    pub energy: Option<i64>,
    pub speed: Option<i64>,
    pub attack_damage: Option<i64>,
    pub attack_range: Option<i64>,
    pub attack_cooldown: Option<i64>,
    pub shield: Option<i64>,
}

impl EntityDef {
    /// Convert to a sim `EntityConfig` for stat overrides.
    pub fn to_entity_config(&self) -> EntityConfig {
        EntityConfig {
            health: self.health,
            energy: self.energy,
            speed: self.speed,
            attack_damage: self.attack_damage,
            attack_range: self.attack_range,
            attack_cooldown: self.attack_cooldown,
            shield: self.shield,
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
    pub initial: Vec<String>,
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

    sprites.insert("summoner".into(), SpriteData {
        png: SUMMONER_ATLAS_PNG.to_vec(),
        json: summoner_atlas_json(),
    });
    sprites.insert("skeleton".into(), SpriteData {
        png: SKELETON_ATLAS_PNG.to_vec(),
        json: skeleton_atlas_json(),
    });
    pivots.insert("summoner".into(), [49.0, 2.0]);
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
        spawn: vec![SpawnDef {
            entity_type: "summoner".into(),
            name: "summoner".into(),
            position: 500,
        }],
        commands: Some(CommandsDef {
            initial: vec![
                "consult".into(),
                "raise".into(),
                "harvest".into(),
                "pact".into(),
            ],
            definitions: vec![],
            libraries: vec![],
        }),
        initial: Some(InitialDef {
            effects: vec![
                CommandEffect::Output { message: "The dead stir beneath your feet".into() },
                CommandEffect::Output { message: "Call for help() to hear them speak".into() },
            ],
        }),
    };

    entity_configs.insert("summoner".into(), EntityConfig {
        health: Some(100),
        energy: Some(100),
        speed: Some(1),
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
        // Also validate spawn effects in custom command definitions.
        if let Some(cmds) = &m.manifest.commands {
            for def in &cmds.definitions {
                for effect in &def.effects {
                    if let CommandEffect::Spawn { entity_type, .. } = effect {
                        if !known_types.contains(entity_type) {
                            eprintln!(
                                "[mod:{}] warning: command '{}' spawns unknown entity type '{}'",
                                m.manifest.meta.id, def.name, entity_type
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Validate custom command definitions at load time.
///
/// Checks stat names in `ModifyStat`/`UseResource` effects, `arg:` target references.
pub fn validate_command_defs(mods: &[LoadedMod]) {
    let valid_stats: HashSet<&str> = ["health", "energy", "shield", "speed"].into_iter().collect();

    for m in mods {
        let Some(cmds) = &m.manifest.commands else { continue };
        let mod_id = &m.manifest.meta.id;
        for def in &cmds.definitions {
            for effect in &def.effects {
                // Validate stat names in ModifyStat and UseResource effects.
                let stat_name = match effect {
                    CommandEffect::ModifyStat { stat, .. }
                    | CommandEffect::UseResource { stat, .. } => Some(stat.as_str()),
                    _ => None,
                };
                if let Some(stat) = stat_name {
                    if !valid_stats.contains(stat) {
                        eprintln!(
                            "[mod:{mod_id}] warning: command '{}' references unknown stat '{stat}' \
                             (valid: health, energy, shield, speed)",
                            def.name
                        );
                    }
                }
                // Validate UseResource amounts.
                if let CommandEffect::UseResource { stat, amount } = effect {
                    if *amount <= 0 {
                        eprintln!(
                            "[mod:{mod_id}] warning: command '{}' has non-positive use_resource amount {amount} for {stat}",
                            def.name
                        );
                    }
                }
                // Validate target strings in effects that have them.
                let target_str = match effect {
                    CommandEffect::Damage { target, .. }
                    | CommandEffect::Heal { target, .. }
                    | CommandEffect::ModifyStat { target, .. } => Some(target.as_str()),
                    _ => None,
                };
                if let Some(target) = target_str {
                    validate_target(target, &def.args, &def.name, mod_id);
                }
            }
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

/// Collect initial commands from all loaded mods.
pub fn collect_initial_commands(mods: &[LoadedMod]) -> HashSet<String> {
    let mut commands = HashSet::new();
    for m in mods {
        if let Some(cmds) = &m.manifest.commands {
            commands.extend(cmds.initial.iter().cloned());
        }
    }
    if commands.is_empty() {
        // Default fallback
        commands.extend(["consult", "raise", "harvest", "pact"].iter().map(|s| s.to_string()));
    }
    commands
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
