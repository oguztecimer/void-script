//! Mod loading system.
//!
//! Loads `mod.toml` manifests from the `mods/` directory. Each mod defines
//! entity types (with sprites and stats), initial effects, and available
//! commands. The base game ("core") is itself a mod.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use deadcode_sim::action::{BuffDef, CommandDef, CommandEffect, TriggerDef};
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
    /// Type definitions: composable type tags with stats, commands, and brain scripts.
    #[serde(default)]
    pub types: Vec<TypeDef>,
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

/// The `[initial]` section: resources and effects available at game start.
#[derive(Debug, Deserialize, Default)]
pub struct InitialDef {
    /// Legacy field — ignored. Commands are now defined entirely by mod command definitions
    /// and type-level gating.
    #[serde(default)]
    #[allow(dead_code)]
    pub commands: Vec<String>,
    /// Initially available resource names. If empty, all defined resources are available.
    #[serde(default)]
    pub resources: Vec<String>,
    /// Effects to run on first game open.
    #[serde(default)]
    pub effects: Vec<CommandEffect>,
}

#[derive(Debug, Deserialize)]
pub struct ModMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    /// Mod IDs this mod requires to be loaded (loaded before this mod).
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Mod IDs that cannot be loaded alongside this mod.
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub min_game_version: Option<String>,
}

/// A type definition: composable tag with optional stats, commands, and brain script.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TypeDef {
    pub name: String,
    /// If true, this type drives entity execution via a `.gs` brain script.
    #[serde(default)]
    pub brain: bool,
    /// Stats provided by this type (merged in type order).
    #[serde(default)]
    pub stats: indexmap::IndexMap<String, i64>,
    /// Commands that entities with this type can use (type capability gate).
    #[serde(default)]
    pub commands: Vec<String>,
    /// Path to a .gs script file (relative to mod's grimscript/ dir).
    #[serde(default)]
    pub script: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EntityDef {
    /// Unique entity definition ID. Falls back to `type` if absent.
    #[serde(default)]
    pub id: Option<String>,
    /// Legacy field — used as `id` if `id` is absent.
    #[serde(default, rename = "type")]
    pub entity_type: Option<String>,
    /// Composable type tags. If absent, defaults to `[id]`.
    #[serde(default)]
    pub types: Vec<String>,
    /// Sprite path relative to mod dir (without extension). Expects .png + .json pair.
    pub sprite: Option<String>,
    /// Sprite pivot [x, y].
    #[serde(default)]
    pub pivot: Option<[f32; 2]>,
    /// All stats for this entity (e.g., health = 50, speed = 2, armor = 5).
    /// These override type-level stats.
    #[serde(default, alias = "custom_stats")]
    pub stats: indexmap::IndexMap<String, i64>,
}

impl EntityDef {
    /// Resolve the entity definition ID (prefers `id`, falls back to `entity_type`).
    /// Returns `None` if neither `id` nor `type` is set.
    pub fn resolved_id(&self) -> Option<String> {
        self.id.clone().or_else(|| self.entity_type.clone())
    }

    /// Resolve the types list (defaults to `[resolved_id()]` if empty).
    pub fn resolved_types(&self) -> Vec<String> {
        if self.types.is_empty() {
            self.resolved_id().into_iter().collect()
        } else {
            self.types.clone()
        }
    }

    /// Convert to a sim `EntityConfig` for entity-level stat overrides only (no type merging).
    #[allow(dead_code)]
    pub fn to_entity_config(&self) -> EntityConfig {
        EntityConfig { stats: self.stats.clone() }
    }
}

#[derive(Debug, Deserialize)]
pub struct CommandsDef {
    #[serde(default)]
    pub definitions: Vec<CommandDef>,
    /// Paths to .grim library files (relative to mod dir). Functions defined
    /// in these files are prepended to player scripts before compilation.
    #[serde(default)]
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

#[allow(dead_code)]
pub struct LoadedMod {
    pub manifest: ModManifest,
    /// Directory containing this mod's files.
    pub mod_dir: PathBuf,
    /// Entity def ID → sprite data (PNG bytes + JSON string).
    pub sprites: HashMap<String, SpriteData>,
    /// Entity def ID → pivot [x, y].
    pub pivots: HashMap<String, [f32; 2]>,
    /// Entity def ID → entity config (stat overrides).
    pub entity_configs: HashMap<String, EntityConfig>,
    /// Entity def ID → resolved type tags.
    pub entity_types: HashMap<String, Vec<String>>,
    /// Command name → command definition.
    pub command_defs: HashMap<String, CommandDef>,
    /// GrimScript library source code (concatenated from all library files).
    pub library_source: String,
    /// Type name → type definition (from [[types]] in mod.toml).
    pub type_defs: HashMap<String, TypeDef>,
    /// Type name → script source (from grimscript/ directory).
    pub type_scripts: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Try to load the mod at `mod_dir`. Returns `None` if `mod.toml` doesn't exist.
fn load_mod_from_dir(mod_dir: &Path) -> Option<LoadedMod> {
    let manifest_path = mod_dir.join("mod.toml");
    let toml_str = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: ModManifest = match toml::from_str(&toml_str) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[mod] error: failed to parse {}: {e}", manifest_path.display());
            return None;
        }
    };

    let mut sprites = HashMap::new();
    let mut pivots = HashMap::new();
    let mut entity_configs = HashMap::new();
    let mut entity_types_map = HashMap::new();
    let mut command_defs = HashMap::new();
    let mut type_defs = HashMap::new();
    let mut type_scripts = HashMap::new();

    // Parse command definitions.
    if let Some(cmds) = &manifest.commands {
        for def in &cmds.definitions {
            command_defs.insert(def.name.clone(), def.clone());
        }
    }

    // Load type definitions.
    for tdef in &manifest.types {
        type_defs.insert(tdef.name.clone(), tdef.clone());
    }

    // Load type .gs scripts from grimscript/ directory.
    let grimscript_dir = mod_dir.join("grimscript");
    if grimscript_dir.is_dir() {
        for tdef in &manifest.types {
            let script_path = if let Some(ref path) = tdef.script {
                grimscript_dir.join(path)
            } else {
                grimscript_dir.join(format!("{}.gs", tdef.name))
            };
            if script_path.exists() {
                match std::fs::read_to_string(&script_path) {
                    Ok(src) => {
                        // Syntax-check at load time.
                        match grimscript_lang::lexer::Lexer::new(&src).tokenize() {
                            Ok(tokens) => {
                                if let Err(e) = grimscript_lang::parser::Parser::new(tokens).parse() {
                                    eprintln!(
                                        "[mod:{}] warning: syntax error in type script '{}' (line {}): {}",
                                        manifest.meta.id, tdef.name, e.line, e.message
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "[mod:{}] warning: lex error in type script '{}' (line {}): {}",
                                    manifest.meta.id, tdef.name, e.line, e.message
                                );
                            }
                        }
                        type_scripts.insert(tdef.name.clone(), src);
                    }
                    Err(e) => {
                        eprintln!(
                            "[mod:{}] warning: failed to read type script '{}': {e}",
                            manifest.meta.id, script_path.display()
                        );
                    }
                }
            }
        }
    }

    for entity_def in &manifest.entities {
        let def_id = match entity_def.resolved_id() {
            Some(id) => id,
            None => {
                eprintln!(
                    "[mod:{}] warning: entity has neither 'id' nor 'type' — skipping",
                    manifest.meta.id
                );
                continue;
            }
        };
        let resolved_types = entity_def.resolved_types();

        // Build merged config: type stats (in order) then entity-level overrides.
        let mut merged_stats = indexmap::IndexMap::new();
        for type_name in &resolved_types {
            if let Some(tdef) = type_defs.get(type_name) {
                for (stat, &value) in &tdef.stats {
                    merged_stats.insert(stat.clone(), value);
                }
            }
        }
        // Entity-level stats override type stats.
        for (stat, &value) in &entity_def.stats {
            merged_stats.insert(stat.clone(), value);
        }

        // Load sprite files if a sprite path is specified.
        if let Some(sprite_path) = &entity_def.sprite {
            let png_path = mod_dir.join(format!("{sprite_path}.png"));
            let json_path = mod_dir.join(format!("{sprite_path}.json"));

            if png_path.exists() && json_path.exists() {
                match (std::fs::read(&png_path), std::fs::read_to_string(&json_path)) {
                    (Ok(png), Ok(json)) => {
                        sprites.insert(def_id.clone(), SpriteData { png, json });
                    }
                    (Err(e), _) => {
                        eprintln!(
                            "[mod:{}] warning: failed to read sprite {}: {e}",
                            manifest.meta.id, png_path.display()
                        );
                    }
                    (_, Err(e)) => {
                        eprintln!(
                            "[mod:{}] warning: failed to read sprite {}: {e}",
                            manifest.meta.id, json_path.display()
                        );
                    }
                }
            } else {
                eprintln!(
                    "[mod] warning: sprite files not found for '{}': {} / {}",
                    def_id,
                    png_path.display(),
                    json_path.display()
                );
            }
        }

        if let Some(pivot) = entity_def.pivot {
            pivots.insert(def_id.clone(), pivot);
        }

        entity_configs.insert(def_id.clone(), EntityConfig { stats: merged_stats });
        entity_types_map.insert(def_id, resolved_types);
    }

    // Load .grim library files.
    let mut library_source = String::new();
    if let Some(cmds) = &manifest.commands {
        for lib_path in &cmds.libraries {
            let full_path = mod_dir.join(lib_path);
            match std::fs::read_to_string(&full_path) {
                Ok(src) => {
                    // Syntax-check library source at load time.
                    match grimscript_lang::lexer::Lexer::new(&src).tokenize() {
                        Ok(tokens) => {
                            if let Err(e) = grimscript_lang::parser::Parser::new(tokens).parse() {
                                eprintln!(
                                    "[mod:{}] warning: syntax error in library '{}' (line {}): {}",
                                    manifest.meta.id, lib_path, e.line, e.message
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "[mod:{}] warning: lex error in library '{}' (line {}): {}",
                                manifest.meta.id, lib_path, e.line, e.message
                            );
                        }
                    }
                    // Still prepend the source for graceful degradation.
                    if !library_source.is_empty() {
                        library_source.push('\n');
                    }
                    library_source.push_str(&src);
                }
                Err(e) => {
                    eprintln!(
                        "[mod:{}] warning: failed to read library '{}': {e}",
                        manifest.meta.id, full_path.display()
                    );
                }
            }
        }
    }

    Some(LoadedMod {
        manifest,
        mod_dir: mod_dir.to_path_buf(),
        sprites,
        pivots,
        entity_configs,
        entity_types: entity_types_map,
        command_defs,
        library_source,
        type_defs,
        type_scripts,
    })
}

/// Load mods from the `mods/` directory. Falls back to embedded assets if
/// the directory doesn't exist or contains no valid mods.
///
/// Mods are loaded alphabetically from disk, then reordered by dependency
/// graph (topological sort). Dependencies listed in `depends_on` must be
/// present; conflicts listed in `conflicts_with` cause the conflicting mod
/// to be skipped with a warning.
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
                    loaded.push(m);
                }
            }
        }
    }

    if loaded.is_empty() {
        eprintln!("[mod] no mods found");
        return loaded;
    }

    // Resolve dependencies: validate, detect conflicts, topological sort.
    loaded = resolve_mod_dependencies(loaded);

    for m in &loaded {
        eprintln!("[mod] loaded: {} v{}", m.manifest.meta.name, m.manifest.meta.version);
    }

    loaded
}

/// Validate dependencies and conflicts, then topologically sort mods so that
/// dependencies are loaded before dependants. Mods with missing deps or
/// conflicts are skipped with warnings.
fn resolve_mod_dependencies(mut mods: Vec<LoadedMod>) -> Vec<LoadedMod> {
    let all_ids: HashSet<String> = mods.iter().map(|m| m.manifest.meta.id.clone()).collect();

    // Check for missing dependencies — skip mods whose deps aren't present.
    let mut valid_ids: HashSet<String> = HashSet::new();
    let mut skipped: HashSet<String> = HashSet::new();
    // Iterate until stable (cascading removal if A depends on B and B is skipped).
    loop {
        let mut changed = false;
        for m in &mods {
            let id = &m.manifest.meta.id;
            if skipped.contains(id) || valid_ids.contains(id) {
                continue;
            }
            let mut ok = true;
            for dep in &m.manifest.meta.depends_on {
                if !all_ids.contains(dep) || skipped.contains(dep) {
                    eprintln!(
                        "[mod] error: '{}' depends on '{}' which is not available — skipping",
                        id, dep
                    );
                    ok = false;
                    break;
                }
            }
            if ok {
                valid_ids.insert(id.clone());
            } else {
                skipped.insert(id.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Remove skipped mods.
    mods.retain(|m| !skipped.contains(&m.manifest.meta.id));

    // Check for conflicts — if A conflicts_with B, skip B (first-loaded wins).
    let mut active_ids: HashSet<String> = HashSet::new();
    let mut conflict_skipped: HashSet<String> = HashSet::new();
    for m in &mods {
        let id = &m.manifest.meta.id;
        // Check if any already-active mod conflicts with this one.
        let mut dominated = false;
        for active in &active_ids {
            let active_mod = mods.iter().find(|am| am.manifest.meta.id == *active).unwrap();
            if active_mod.manifest.meta.conflicts_with.contains(id) {
                eprintln!(
                    "[mod] warning: '{}' conflicts with already-loaded '{}' — skipping '{}'",
                    active, id, id
                );
                dominated = true;
                break;
            }
        }
        if dominated {
            conflict_skipped.insert(id.clone());
            continue;
        }
        // Check if this mod conflicts with any already-active mod.
        for conflict in &m.manifest.meta.conflicts_with {
            if active_ids.contains(conflict) {
                eprintln!(
                    "[mod] warning: '{}' conflicts with already-loaded '{}' — skipping '{}'",
                    id, conflict, id
                );
                dominated = true;
                break;
            }
        }
        if dominated {
            conflict_skipped.insert(id.clone());
            continue;
        }
        active_ids.insert(id.clone());
    }
    mods.retain(|m| !conflict_skipped.contains(&m.manifest.meta.id));

    // Topological sort by depends_on (Kahn's algorithm).
    let id_list: Vec<String> = mods.iter().map(|m| m.manifest.meta.id.clone()).collect();
    let id_set: HashSet<&str> = id_list.iter().map(|s| s.as_str()).collect();

    // Build in-degree and adjacency.
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependants: HashMap<&str, Vec<&str>> = HashMap::new();
    for id in &id_list {
        in_degree.insert(id, 0);
    }
    for m in &mods {
        let id = m.manifest.meta.id.as_str();
        for dep in &m.manifest.meta.depends_on {
            if id_set.contains(dep.as_str()) {
                *in_degree.entry(id).or_insert(0) += 1;
                dependants.entry(dep.as_str()).or_default().push(id);
            }
        }
    }

    let mut queue: std::collections::VecDeque<&str> = id_list.iter()
        .filter(|id| in_degree[id.as_str()] == 0)
        .map(|s| s.as_str())
        .collect();
    // Initial order is already alphabetical (from id_list which preserves input order).

    let mut sorted_ids: Vec<String> = Vec::with_capacity(id_list.len());
    while let Some(id) = queue.pop_front() {
        sorted_ids.push(id.to_string());
        if let Some(deps) = dependants.get(id) {
            // Collect newly freed nodes, sort alphabetically for stable ordering.
            let mut newly_free: Vec<&str> = Vec::new();
            for &dep in deps {
                let deg = in_degree.get_mut(dep).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    newly_free.push(dep);
                }
            }
            newly_free.sort();
            for dep in newly_free {
                queue.push_back(dep);
            }
        }
    }

    if sorted_ids.len() < id_list.len() {
        // Cycle detected — find the remaining mods.
        let sorted_set: HashSet<String> = sorted_ids.iter().cloned().collect();
        let cycle_mods: Vec<&str> = id_list.iter()
            .filter(|id| !sorted_set.contains(id.as_str()))
            .map(|s| s.as_str())
            .collect();
        eprintln!(
            "[mod] error: circular dependency detected among: {} — loading in alphabetical order",
            cycle_mods.join(", ")
        );
        // Fall back to alphabetical for the cycle members.
        for id in &id_list {
            if !sorted_set.contains(id.as_str()) {
                sorted_ids.push(id.clone());
            }
        }
    }

    // Reorder mods by sorted_ids.
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();
    for (i, id) in sorted_ids.iter().enumerate() {
        id_to_idx.insert(id.clone(), i);
    }
    mods.sort_by_key(|m| *id_to_idx.get(&m.manifest.meta.id).unwrap_or(&usize::MAX));
    mods
}

/// Create a Lua mod runtime and load all mods' mod.lua files.
/// Returns `None` if no mods have Lua scripts.
pub fn create_lua_runtime(mods: &[LoadedMod]) -> Option<deadcode_lua::LuaModRuntime> {
    let mut runtime = match deadcode_lua::LuaModRuntime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[lua] failed to create runtime: {e}");
            return None;
        }
    };

    let mut any_lua = false;
    for m in mods {
        let lua_path = m.mod_dir.join("mod.lua");
        if lua_path.exists() {
            match runtime.load_mod(&m.manifest.meta.id, &m.mod_dir) {
                Ok(()) => {
                    eprintln!("[lua] loaded: {}/mod.lua", m.manifest.meta.id);
                    any_lua = true;
                }
                Err(e) => {
                    eprintln!("[lua] error loading {}/mod.lua: {e}", m.manifest.meta.id);
                }
            }
        }
    }

    if any_lua {
        // Register Lua command metadata with the sim world.
        Some(runtime)
    } else {
        None
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

/// Validate that spawn effects in commands reference known entity types.
pub fn validate_spawn_effects(mods: &[LoadedMod], known_types: &HashSet<String>) {
    for m in mods {
        // Validate spawn effects in custom command definitions (effects + phases),
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
                        CommandEffect::Spawn { entity_id, .. } => Some(entity_id.as_str()),
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
        // Also validate spawn effects in initial effects.
        if let Some(initial) = &m.manifest.initial {
            let mut all_effects = Vec::new();
            collect_all_effects_recursive(&initial.effects, &mut all_effects);
            for effect in all_effects {
                let referenced_type = match effect {
                    CommandEffect::Spawn { entity_id, .. } => Some(entity_id.as_str()),
                    _ => None,
                };
                if let Some(entity_type) = referenced_type {
                    if !known_types.contains(entity_type) {
                        eprintln!(
                            "[mod:{}] warning: initial effect references unknown entity type '{}'",
                            m.manifest.meta.id, entity_type
                        );
                    }
                }
            }
        }
    }
}

/// Validate a list of effects against target references and arg names.
/// Recurses into `If` branches and `StartChannel` phases.
fn validate_effects(
    effects: &[CommandEffect],
    args: &[String],
    cmd_name: &str,
    mod_id: &str,
) {
    for effect in effects {
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
            | CommandEffect::RemoveBuff { target, .. } => Some(target.as_str()),
            _ => None,
        };
        if let Some(target) = target_str {
            validate_target(target, args, cmd_name, mod_id);
        }
        // Validate condition in If effects, then recurse into both branches.
        if let CommandEffect::If { condition, then_effects, otherwise } = effect {
            validate_condition(condition, cmd_name, mod_id);
            validate_effects(then_effects, args, cmd_name, mod_id);
            validate_effects(otherwise, args, cmd_name, mod_id);
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
                validate_effects(&phase.on_start, args, cmd_name, mod_id);
                validate_effects(&phase.per_update, args, cmd_name, mod_id);
            }
        }
    }
}

/// Validate a condition's fields.
fn validate_condition(
    condition: &deadcode_sim::action::Condition,
    cmd_name: &str,
    mod_id: &str,
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
            if stat.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has if-condition with empty stat name",
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
                validate_condition(sub, cmd_name, mod_id);
            }
        }
        deadcode_sim::action::Condition::IsAlive { target } => {
            if target.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has is_alive condition with empty target",
                );
            }
        }
        deadcode_sim::action::Condition::Distance { target, .. } => {
            if target.is_empty() {
                eprintln!(
                    "[mod:{mod_id}] warning: command '{cmd_name}' has distance condition with empty target",
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
                validate_effects(&phase.on_start, &def.args, &def.name, mod_id);
                validate_effects(&phase.per_update, &def.args, &def.name, mod_id);
            }

            // Validate instant effects (reuses validate_effects which handles If/StartChannel recursion).
            validate_effects(&def.effects, &def.args, &def.name, mod_id);
        }
    }
}

fn validate_target(target: &str, args: &[String], cmd_name: &str, mod_id: &str) {
    if target == "self" {
        return;
    }
    // Scoped targets (valid in trigger contexts).
    if matches!(target, "source" | "owner" | "attacker" | "killer") {
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
        "[mod:{}] warning: command '{}' effect has invalid target '{}' (expected 'self', 'arg:<name>', 'source', 'owner', 'attacker', or 'killer')",
        mod_id, cmd_name, target
    );
}

/// Collect all command names from loaded mods (in definition order).
/// Used to set command display order for `list_commands`.
pub fn collect_command_names(mods: &[LoadedMod]) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    for m in mods {
        if let Some(cmds) = &m.manifest.commands {
            for def in &cmds.definitions {
                if seen.insert(def.name.clone()) {
                    names.push(def.name.clone());
                }
            }
        }
    }
    names
}

/// Collected resource definitions: values and optional caps.
pub struct CollectedResources {
    pub values: deadcode_sim::IndexMap<String, i64>,
    pub caps: deadcode_sim::IndexMap<String, i64>,
}

/// Collect global resources from all loaded mods, merging them.
/// Duplicate resource names: first-defined wins (with a warning).
pub fn collect_initial_resources(mods: &[LoadedMod]) -> CollectedResources {
    let mut values = deadcode_sim::IndexMap::new();
    let mut caps = deadcode_sim::IndexMap::new();
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
    // Warn when initial value exceeds cap.
    for (name, &max) in &caps {
        if let Some(&val) = values.get(name) {
            if val > max {
                eprintln!(
                    "[mod] warning: resource '{name}' initial value ({val}) exceeds max ({max})"
                );
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
pub fn validate_buffs(mods: &[LoadedMod], known_stats: &HashSet<String>) {
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
            // Validate modifier stat names.
            for stat_name in buff.modifiers.keys() {
                if !known_stats.contains(stat_name) {
                    eprintln!(
                        "[mod:{mod_id}] warning: buff '{}' modifies unknown stat '{stat_name}'",
                        buff.name
                    );
                }
            }
            // Validate effect lists.
            let effect_ctx = format!("buff '{}'", buff.name);
            validate_effects(&buff.per_tick, &[], &effect_ctx, mod_id);
            validate_effects(&buff.on_apply, &[], &effect_ctx, mod_id);
            validate_effects(&buff.on_expire, &[], &effect_ctx, mod_id);
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
                );
            }

            // Validate effects (reuses existing recursive validation).
            validate_effects(
                &trigger.effects,
                &[], // triggers have no args
                &format!("trigger {i}"),
                mod_id,
            );
        }
    }
}

/// Validate type definitions across all loaded mods.
pub fn validate_type_defs(mods: &[LoadedMod]) {
    let mut seen_types: HashSet<String> = HashSet::new();
    for m in mods {
        let mod_id = &m.manifest.meta.id;
        for tdef in &m.manifest.types {
            if tdef.name.is_empty() {
                eprintln!("[mod:{mod_id}] warning: type definition has empty name");
                continue;
            }
            if seen_types.contains(&tdef.name) {
                eprintln!(
                    "[mod:{mod_id}] warning: type '{}' already defined in another mod — skipping",
                    tdef.name
                );
            } else {
                seen_types.insert(tdef.name.clone());
            }
        }
    }
}

/// Validate entity definitions across all loaded mods.
/// Returns a set of entity IDs that failed validation and should be skipped.
pub fn validate_entity_defs(mods: &[LoadedMod], all_type_defs: &HashMap<String, TypeDef>) -> HashSet<String> {
    let mut rejected = HashSet::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    for m in mods {
        let mod_id = &m.manifest.meta.id;
        for edef in &m.manifest.entities {
            let def_id = match edef.resolved_id() {
                Some(id) if !id.is_empty() => id,
                _ => {
                    eprintln!("[mod:{mod_id}] warning: entity has no id — skipping");
                    continue;
                }
            };
            if seen_ids.contains(&def_id) {
                eprintln!(
                    "[mod:{mod_id}] warning: entity id '{}' already defined — skipping",
                    def_id
                );
            } else {
                seen_ids.insert(def_id.clone());
            }

            let types = edef.resolved_types();

            // Check for duplicate types in a single entity.
            let mut entity_type_set = HashSet::new();
            for t in &types {
                if !entity_type_set.insert(t.clone()) {
                    eprintln!(
                        "[mod:{mod_id}] warning: entity '{}' has duplicate type '{}'",
                        def_id, t
                    );
                }
            }

            // Check that referenced types exist.
            for t in &types {
                if !all_type_defs.contains_key(t) && t != &def_id {
                    eprintln!(
                        "[mod:{mod_id}] warning: entity '{}' references unknown type '{}'",
                        def_id, t
                    );
                }
            }

            // Check brain count — entities with multiple brain types are rejected.
            let brain_count = types.iter()
                .filter(|t| all_type_defs.get(*t).map_or(false, |td| td.brain))
                .count();
            if brain_count > 1 {
                eprintln!(
                    "[mod:{mod_id}] error: entity '{}' has {} brain types (expected 0 or 1) — entity will not be loaded",
                    def_id, brain_count
                );
                rejected.insert(def_id);
            }
        }
    }
    rejected
}

/// Collect all type definitions from loaded mods (first-defined wins).
pub fn collect_type_defs(mods: &[LoadedMod]) -> HashMap<String, TypeDef> {
    let mut defs = HashMap::new();
    for m in mods {
        for (name, tdef) in &m.type_defs {
            defs.entry(name.clone()).or_insert_with(|| tdef.clone());
        }
    }
    defs
}

/// Collect all type scripts from loaded mods (first-defined wins).
pub fn collect_type_scripts(mods: &[LoadedMod]) -> HashMap<String, String> {
    let mut scripts = HashMap::new();
    for m in mods {
        for (name, src) in &m.type_scripts {
            scripts.entry(name.clone()).or_insert_with(|| src.clone());
        }
    }
    scripts
}

/// Compute effective commands for an entity based on its types' command lists.
/// Returns `None` (all allowed) when no types define commands (backward compat).
/// Returns `Some(set)` when at least one type has a non-empty commands list.
pub fn compute_effective_commands(
    entity_types: &[String],
    all_type_defs: &HashMap<String, TypeDef>,
) -> Option<HashSet<String>> {
    let mut type_commands: HashSet<String> = HashSet::new();
    let mut any_has_commands = false;
    for t in entity_types {
        if let Some(tdef) = all_type_defs.get(t) {
            if !tdef.commands.is_empty() {
                any_has_commands = true;
                type_commands.extend(tdef.commands.iter().cloned());
            }
        }
    }
    if any_has_commands { Some(type_commands) } else { None }
}

/// Collect library source code from all loaded mods (in load order).
/// Returns the concatenated GrimScript source to be prepended to player scripts.
/// Function name collisions use first-loaded-wins (consistent with commands).
pub fn collect_library_source(mods: &[LoadedMod]) -> String {
    let mut combined = String::new();
    for m in mods {
        if !m.library_source.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&m.library_source);
        }
    }
    combined
}

/// Validate dependencies and conflicts at load time (post-resolution).
/// This produces warnings for any issues that were handled silently during resolution.
pub fn validate_dependencies(mods: &[LoadedMod]) {
    let loaded_ids: HashSet<String> = mods.iter().map(|m| m.manifest.meta.id.clone()).collect();
    for m in mods {
        let id = &m.manifest.meta.id;
        for dep in &m.manifest.meta.depends_on {
            if !loaded_ids.contains(dep) {
                // This shouldn't happen after resolution, but just in case.
                eprintln!("[mod] error: '{}' depends on '{}' which is not loaded", id, dep);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal LoadedMod with the given id, depends_on, and conflicts_with.
    fn test_mod(id: &str, depends_on: Vec<&str>, conflicts_with: Vec<&str>) -> LoadedMod {
        LoadedMod {
            manifest: ModManifest {
                meta: ModMeta {
                    id: id.into(),
                    name: id.into(),
                    version: "1.0.0".into(),
                    depends_on: depends_on.into_iter().map(|s| s.to_string()).collect(),
                    conflicts_with: conflicts_with.into_iter().map(|s| s.to_string()).collect(),
                    min_game_version: None,
                },
                entities: vec![],

                commands: None,
                initial: None,
                resources: HashMap::new(),
                triggers: vec![],
                buffs: vec![],
                types: vec![],
            },
            mod_dir: PathBuf::new(),
            sprites: HashMap::new(),
            pivots: HashMap::new(),
            entity_configs: HashMap::new(),
            entity_types: HashMap::new(),
            command_defs: HashMap::new(),
            library_source: String::new(),
            type_defs: HashMap::new(),
            type_scripts: HashMap::new(),
        }
    }

    fn ids(mods: &[LoadedMod]) -> Vec<String> {
        mods.iter().map(|m| m.manifest.meta.id.clone()).collect()
    }

    #[test]
    fn dependency_resolution_no_deps() {
        let mods = vec![
            test_mod("core", vec![], vec![]),
            test_mod("extra", vec![], vec![]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        assert_eq!(ids(&resolved), vec!["core", "extra"]);
    }

    #[test]
    fn dependency_resolution_simple_ordering() {
        // "extra" depends on "core" — core should come first.
        let mods = vec![
            test_mod("extra", vec!["core"], vec![]),
            test_mod("core", vec![], vec![]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        assert_eq!(ids(&resolved), vec!["core", "extra"]);
    }

    #[test]
    fn dependency_resolution_chain() {
        // c depends on b, b depends on a.
        let mods = vec![
            test_mod("c", vec!["b"], vec![]),
            test_mod("a", vec![], vec![]),
            test_mod("b", vec!["a"], vec![]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        assert_eq!(ids(&resolved), vec!["a", "b", "c"]);
    }

    #[test]
    fn dependency_resolution_missing_dep_skipped() {
        // "extra" depends on "missing" which doesn't exist.
        let mods = vec![
            test_mod("core", vec![], vec![]),
            test_mod("extra", vec!["missing"], vec![]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        assert_eq!(ids(&resolved), vec!["core"]);
    }

    #[test]
    fn dependency_resolution_cascade_skip() {
        // b depends on a, c depends on b. If a is missing, both b and c should be skipped.
        let mods = vec![
            test_mod("b", vec!["a"], vec![]),
            test_mod("c", vec!["b"], vec![]),
            test_mod("d", vec![], vec![]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        assert_eq!(ids(&resolved), vec!["d"]);
    }

    #[test]
    fn dependency_resolution_conflict_skips_second() {
        // core conflicts_with extra — extra should be skipped (core loads first alphabetically).
        let mods = vec![
            test_mod("core", vec![], vec!["extra"]),
            test_mod("extra", vec![], vec![]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        assert_eq!(ids(&resolved), vec!["core"]);
    }

    #[test]
    fn dependency_resolution_reverse_conflict() {
        // extra conflicts_with core — extra is skipped since core loads first.
        let mods = vec![
            test_mod("core", vec![], vec![]),
            test_mod("extra", vec![], vec!["core"]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        assert_eq!(ids(&resolved), vec!["core"]);
    }

    #[test]
    fn dependency_resolution_cycle_fallback() {
        // a depends on b, b depends on a — cycle. Both should still appear.
        let mods = vec![
            test_mod("a", vec!["b"], vec![]),
            test_mod("b", vec!["a"], vec![]),
        ];
        let resolved = resolve_mod_dependencies(mods);
        // Both should be present (cycle falls back to alphabetical).
        assert_eq!(resolved.len(), 2);
        let result_ids = ids(&resolved);
        assert!(result_ids.contains(&"a".to_string()));
        assert!(result_ids.contains(&"b".to_string()));
    }

    #[test]
    fn library_source_collection() {
        let mut m1 = test_mod("core", vec![], vec![]);
        m1.library_source = "def helper():\n    return 1".into();
        let mut m2 = test_mod("extra", vec![], vec![]);
        m2.library_source = "def util():\n    return 2".into();

        let combined = collect_library_source(&[m1, m2]);
        assert!(combined.contains("def helper()"));
        assert!(combined.contains("def util()"));
    }

    #[test]
    fn library_source_empty_when_no_libs() {
        let m1 = test_mod("core", vec![], vec![]);
        let combined = collect_library_source(&[m1]);
        assert!(combined.is_empty());
    }
}
