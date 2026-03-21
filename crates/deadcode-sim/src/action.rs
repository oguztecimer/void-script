use serde::{Deserialize, Serialize};

use crate::entity::{EntityId, SimEntity};
use crate::rng::SimRng;
use crate::value::SimValue;
use crate::world::{SimEvent, SimWorld};

/// An integer value that is either fixed, randomized, or computed from game state.
///
/// In mod.toml, write a plain integer for fixed values, `"rand(min,max)"` for
/// random, or game-state queries like `"entity_count(skeleton)"`,
/// `"resource(mana)"`, `"stat(health)"`.
#[derive(Debug, Clone)]
pub enum DynInt {
    Fixed(i64),
    Rand { min: i64, max: i64 },
    /// Count of alive, ready entities of a type.
    EntityCount { entity_type: String, multiplier: i64 },
    /// Current value of a global resource.
    ResourceValue { resource: String, multiplier: i64 },
    /// Caster's stat value (health, shield, speed, attack_damage, attack_range).
    CasterStat { stat: String, multiplier: i64 },
}

impl DynInt {
    pub fn resolve(&self, rng: &mut SimRng) -> i64 {
        match self {
            DynInt::Fixed(v) => *v,
            DynInt::Rand { min, max } => {
                if min >= max {
                    return *min;
                }
                let range = (max - min + 1) as u64;
                *min + rng.next_bounded(range) as i64
            }
            // Game-state variants return 0 when resolved without world context.
            DynInt::EntityCount { .. } | DynInt::ResourceValue { .. } | DynInt::CasterStat { .. } => 0,
        }
    }

    /// Resolve with world context for game-state-dependent values.
    pub fn resolve_with_world(
        &self,
        rng: &mut SimRng,
        world: &SimWorld,
        entity_id: EntityId,
    ) -> i64 {
        match self {
            DynInt::Fixed(v) => *v,
            DynInt::Rand { min, max } => {
                if min >= max {
                    return *min;
                }
                let range = (max - min + 1) as u64;
                *min + rng.next_bounded(range) as i64
            }
            DynInt::EntityCount { entity_type, multiplier } => {
                let count = world.entities()
                    .filter(|e| e.alive && e.spawn_ticks_remaining == 0 && e.has_type(entity_type))
                    .count() as i64;
                count.saturating_mul(*multiplier)
            }
            DynInt::ResourceValue { resource, multiplier } => {
                world.get_resource(resource).saturating_mul(*multiplier)
            }
            DynInt::CasterStat { stat, multiplier } => {
                let value = world.get_entity(entity_id).map_or(0, |e| e.stat(stat));
                value.saturating_mul(*multiplier)
            }
        }
    }
}

impl Serialize for DynInt {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            DynInt::Fixed(v) => serializer.serialize_i64(*v),
            DynInt::Rand { min, max } => {
                serializer.serialize_str(&format!("rand({min},{max})"))
            }
            DynInt::EntityCount { entity_type, multiplier } => {
                if *multiplier == 1 {
                    serializer.serialize_str(&format!("entity_count({entity_type})"))
                } else {
                    serializer.serialize_str(&format!("entity_count({entity_type})*{multiplier}"))
                }
            }
            DynInt::ResourceValue { resource, multiplier } => {
                if *multiplier == 1 {
                    serializer.serialize_str(&format!("resource({resource})"))
                } else {
                    serializer.serialize_str(&format!("resource({resource})*{multiplier}"))
                }
            }
            DynInt::CasterStat { stat, multiplier } => {
                if *multiplier == 1 {
                    serializer.serialize_str(&format!("stat({stat})"))
                } else {
                    serializer.serialize_str(&format!("stat({stat})*{multiplier}"))
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for DynInt {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;

        struct DynIntVisitor;

        impl<'de> de::Visitor<'de> for DynIntVisitor {
            type Value = DynInt;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "an integer, \"rand(min,max)\", \"entity_count(type)\", \"resource(name)\", or \"stat(name)\"")
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<DynInt, E> {
                Ok(DynInt::Fixed(v))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<DynInt, E> {
                Ok(DynInt::Fixed(v as i64))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<DynInt, E> {
                // Parse optional multiplier: "func(arg)*N" or "func(arg)"
                let (base_end, multiplier) = if let Some(pos) = s.rfind('*') {
                    let after = s[pos + 1..].trim();
                    if let Ok(m) = after.parse::<i64>() {
                        (pos, m)
                    } else {
                        (s.len(), 1)
                    }
                } else {
                    (s.len(), 1)
                };
                let base = s[..base_end].trim_end();

                if let Some(inner) = base.strip_prefix("rand(").and_then(|s: &str| s.strip_suffix(')')) {
                    let parts: Vec<&str> = inner.split(',').collect();
                    if parts.len() == 2 {
                        let min = parts[0].trim().parse::<i64>().map_err(de::Error::custom)?;
                        let max = parts[1].trim().parse::<i64>().map_err(de::Error::custom)?;
                        return Ok(DynInt::Rand { min, max });
                    }
                }
                if let Some(inner) = base.strip_prefix("entity_count(").and_then(|s| s.strip_suffix(')')) {
                    return Ok(DynInt::EntityCount {
                        entity_type: inner.trim().to_string(),
                        multiplier,
                    });
                }
                if let Some(inner) = base.strip_prefix("resource(").and_then(|s| s.strip_suffix(')')) {
                    return Ok(DynInt::ResourceValue {
                        resource: inner.trim().to_string(),
                        multiplier,
                    });
                }
                if let Some(inner) = base.strip_prefix("stat(").and_then(|s| s.strip_suffix(')')) {
                    return Ok(DynInt::CasterStat {
                        stat: inner.trim().to_string(),
                        multiplier,
                    });
                }
                Err(de::Error::custom(format!(
                    "expected integer, \"rand(min,max)\", \"entity_count(type)\", \
                     \"resource(name)\", or \"stat(name)\", got \"{s}\""
                )))
            }
        }

        deserializer.deserialize_any(DynIntVisitor)
    }
}

/// Comparison operator for condition evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl CompareOp {
    pub fn evaluate(&self, lhs: i64, rhs: i64) -> bool {
        match self {
            CompareOp::Eq => lhs == rhs,
            CompareOp::Ne => lhs != rhs,
            CompareOp::Gt => lhs > rhs,
            CompareOp::Gte => lhs >= rhs,
            CompareOp::Lt => lhs < rhs,
            CompareOp::Lte => lhs <= rhs,
        }
    }
}

/// A condition that can be evaluated against game state for branching in effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Condition {
    /// Compare a global resource value against a threshold.
    #[serde(rename = "resource")]
    Resource {
        resource: String,
        compare: CompareOp,
        amount: DynInt,
    },
    /// Compare the count of alive, ready entities of a type against a threshold.
    #[serde(rename = "entity_count")]
    EntityCount {
        entity_type: String,
        compare: CompareOp,
        amount: DynInt,
    },
    /// Compare the caster's stat against a threshold.
    #[serde(rename = "stat", alias = "custom_stat")]
    Stat {
        stat: String,
        compare: CompareOp,
        amount: DynInt,
    },
    /// Check if the caster has a specific buff active.
    #[serde(rename = "has_buff")]
    HasBuff {
        buff: String,
    },
    /// Random chance check (deterministic). Fires if roll < percent.
    #[serde(rename = "random_chance")]
    RandomChance {
        percent: i64,
    },
    /// Logical AND: all sub-conditions must be true.
    #[serde(rename = "and")]
    And {
        conditions: Vec<Condition>,
    },
    /// Logical OR: at least one sub-condition must be true.
    #[serde(rename = "or")]
    Or {
        conditions: Vec<Condition>,
    },
    /// Check if a target entity is alive.
    #[serde(rename = "is_alive")]
    IsAlive {
        target: String,
    },
    /// Compare distance between caster and target against a threshold.
    #[serde(rename = "distance")]
    Distance {
        target: String,
        compare: CompareOp,
        amount: DynInt,
    },
}

/// Context for scoped target resolution in trigger effects.
///
/// Provides references to event participants (who died, who attacked, who spawned, etc.)
/// so that trigger effects can target them using `"source"`, `"owner"`, `"attacker"`, `"killer"`.
#[derive(Debug, Clone, Default)]
pub struct EffectContext {
    /// The entity that is the subject of the event (e.g., the entity that died, was damaged, or spawned).
    pub source: Option<EntityId>,
    /// The owner of the source entity (captured at event time for dead entities).
    pub owner: Option<EntityId>,
    /// The entity that dealt damage (for entity_damaged events).
    pub attacker: Option<EntityId>,
    /// The entity that dealt the killing blow (for entity_died events).
    pub killer: Option<EntityId>,
}

/// Outcome of resolving a list of effects.
pub enum EffectOutcome {
    /// All effects completed normally.
    Complete,
    /// A use_resource/use_global_resource effect failed — abort.
    Aborted,
    /// A start_channel effect was encountered — initiate a channel.
    StartChannel { phases: Vec<PhaseDef> },
}

/// An action a unit wants to perform this tick.
#[derive(Debug, Clone)]
pub enum UnitAction {
    /// Move toward a target position by `speed` units.
    Move { target_pos: i64 },
    /// Attack a target entity.
    Attack { target: EntityId },
    /// Flee from a threat (move away).
    Flee { threat: EntityId },
    /// Do nothing for one tick.
    Wait,
    /// Set the unit's target.
    SetTarget { target: EntityId },
    /// Print a value (not really a game action, but uses the same yield path).
    Print { text: String },
    /// Gain a global resource (instant — handled in tick loop, not resolve_action).
    GainResource { name: String, amount: i64 },
    /// Try to spend a global resource (instant — handled in tick loop, not resolve_action).
    TrySpendResource { name: String, amount: i64 },
    /// Custom mod-defined command with resolved arguments.
    Custom { name: String, args: Vec<SimValue> },
}

/// An effect that a custom command applies when resolved.
///
/// Integer fields use `DynInt`: write a plain integer for fixed values,
/// or `"rand(min,max)"` for a random value in [min, max] inclusive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandEffect {
    /// Print a message to the console.
    #[serde(rename = "output")]
    Output { message: String },
    /// Deal damage to a target (shield-first).
    #[serde(rename = "damage")]
    Damage { target: String, amount: DynInt },
    /// Heal a target (capped at max).
    #[serde(rename = "heal")]
    Heal { target: String, amount: DynInt },
    /// Spawn an entity at self.position + offset.
    #[serde(rename = "spawn")]
    Spawn { #[serde(alias = "entity_type")] entity_id: String, offset: DynInt },
    /// Add to a stat. Works for all stats (health, shield, speed, custom stats, etc.).
    #[serde(rename = "modify_stat", alias = "modify_custom_stat")]
    ModifyStat { target: String, stat: String, amount: DynInt },
    /// Check and deduct a stat; if insufficient, abort remaining effects.
    #[serde(rename = "use_resource", alias = "use_custom_stat")]
    UseResource { stat: String, amount: DynInt },
    /// List all registered commands and their descriptions.
    #[serde(rename = "list_commands")]
    ListCommands,
    /// Trigger an animation on a target entity.
    #[serde(rename = "animate")]
    Animate { target: String, animation: String },
    /// Add to a global resource (clamped to cap if capped). Can be negative.
    #[serde(rename = "modify_resource")]
    ModifyResource { resource: String, amount: DynInt },
    /// Check and deduct a global resource; if insufficient, abort remaining effects.
    #[serde(rename = "use_global_resource")]
    UseGlobalResource { resource: String, amount: DynInt },
    /// Conditional branching: evaluate a condition and run one of two effect lists.
    #[serde(rename = "if")]
    If {
        condition: Condition,
        #[serde(rename = "then")]
        then_effects: Vec<CommandEffect>,
        #[serde(default, rename = "else")]
        otherwise: Vec<CommandEffect>,
    },
    /// Start a phased channel from within an effect list.
    #[serde(rename = "start_channel")]
    StartChannel { phases: Vec<PhaseDef> },
    /// Apply a buff to a target entity.
    #[serde(rename = "apply_buff")]
    ApplyBuff {
        target: String,
        buff: String,
        /// Duration override (uses buff's default if omitted).
        #[serde(default)]
        duration: Option<i64>,
    },
    /// Remove a buff from a target entity.
    #[serde(rename = "remove_buff")]
    RemoveBuff {
        target: String,
        buff: String,
    },
}

/// A single phase in a multi-tick phased command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseDef {
    pub ticks: i64,
    #[serde(default)]
    pub interruptible: bool,
    #[serde(default)]
    pub per_update: Vec<CommandEffect>,
    #[serde(default = "default_update_interval")]
    pub update_interval: i64,
    #[serde(default)]
    pub on_start: Vec<CommandEffect>,
}

fn default_update_interval() -> i64 {
    1
}

/// Definition of a custom command (parsed from mod.toml).
///
/// Commands use either `effects` (instant, single-tick) or `phases` (multi-tick
/// with channeling). They are mutually exclusive — validated at load time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub effects: Vec<CommandEffect>,
    #[serde(default)]
    pub phases: Vec<PhaseDef>,
    /// If true, the command is hidden from `list_commands` output.
    #[serde(default)]
    pub unlisted: bool,
}

/// A trigger definition: fires effects when a game event matches.
///
/// Triggers are defined in `[[triggers]]` sections in `mod.toml`. Each trigger
/// listens for a specific event type, optionally filters by type-specific fields,
/// checks conditions against the current world state, and runs effects if all pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDef {
    /// Event type: "entity_died", "entity_spawned", "entity_damaged",
    /// "resource_changed", "command_used", "tick_interval",
    /// "channel_completed", "channel_interrupted"
    pub event: String,
    /// Type-specific filters to narrow which events match.
    #[serde(default)]
    pub filter: TriggerFilter,
    /// Conditions that must all be true for the trigger to fire.
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Effects to run when the trigger fires.
    #[serde(default)]
    pub effects: Vec<CommandEffect>,
}

/// Filters for narrowing which events a trigger matches.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggerFilter {
    /// Match only events involving this entity type (for entity_died, entity_spawned, entity_damaged).
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Match only events for this resource (for resource_changed).
    #[serde(default)]
    pub resource: Option<String>,
    /// Match only events for this command (for command_used, channel_completed, channel_interrupted).
    #[serde(default)]
    pub command: Option<String>,
    /// Tick interval for tick_interval triggers (fires when tick % interval == 0).
    #[serde(default)]
    pub interval: Option<i64>,
}

/// A buff definition that can be applied to entities.
///
/// Defined in `[[buffs]]` sections in `mod.toml`. Buffs provide temporary
/// stat modifications with automatic expiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuffDef {
    pub name: String,
    /// Default duration in ticks.
    #[serde(default)]
    pub duration: i64,
    /// Stat modifiers applied while active (stat → amount).
    #[serde(default)]
    pub modifiers: indexmap::IndexMap<String, i64>,
    /// Effects that run each tick while the buff is active.
    #[serde(default)]
    pub per_tick: Vec<CommandEffect>,
    /// Effects that run when the buff is applied.
    #[serde(default)]
    pub on_apply: Vec<CommandEffect>,
    /// Effects that run when the buff expires or is removed.
    #[serde(default)]
    pub on_expire: Vec<CommandEffect>,
    /// Whether multiple applications stack (true) or refresh duration (false).
    #[serde(default)]
    pub stackable: bool,
    /// Maximum stack count (0 = unlimited). Only relevant if stackable.
    #[serde(default)]
    pub max_stacks: i64,
}

/// Resolve a unit's action against the world state.
/// Mutates the world directly. Returns events generated.
pub fn resolve_action(
    world: &mut SimWorld,
    entity_id: EntityId,
    action: UnitAction,
) -> Vec<SimEvent> {
    let mut events = Vec::new();

    match action {
        UnitAction::Move { target_pos } => {
            if let Some(entity) = world.get_entity_mut(entity_id) {
                let speed = entity.stat("speed");
                let dx = target_pos - entity.position;
                let step = dx.signum() * speed.min(dx.abs());
                entity.position += step;
                events.push(SimEvent::EntityMoved {
                    entity_id,
                    new_position: entity.position,
                });
            }
        }

        UnitAction::Attack { target } => {
            let (damage, range, attacker_pos) = match world.get_entity(entity_id) {
                Some(e) => (e.stat("attack_damage"), e.stat("attack_range"), e.position),
                None => return events,
            };

            match world.get_entity(target) {
                Some(t) if t.alive => {
                    let dist = (t.position - attacker_pos).abs();
                    if dist > range {
                        return events;
                    }
                }
                _ => return events,
            };

            if let Some(target_entity) = world.get_entity_mut(target) {
                let mut remaining = damage;
                let shield = target_entity.stat("shield");
                if shield > 0 {
                    let shield_absorbed = remaining.min(shield);
                    target_entity.set_stat("shield", shield - shield_absorbed);
                    remaining -= shield_absorbed;
                }
                let new_health = (target_entity.stat("health") - remaining).max(0);
                target_entity.set_stat("health", new_health);

                events.push(SimEvent::EntityDamaged {
                    entity_id: target,
                    damage,
                    new_health,
                    attacker_id: Some(entity_id),
                });

                if new_health <= 0 {
                    target_entity.alive = false;
                    let owner_id = target_entity.owner;
                    events.push(SimEvent::EntityDied {
                        entity_id: target,
                        name: target_entity.name.clone(),
                        killer_id: Some(entity_id),
                        owner_id,
                    });
                }
            }

            if let Some(attacker) = world.get_entity_mut(entity_id) {
                let cooldown = attacker.stat("attack_cooldown");
                attacker.set_stat("cooldown_remaining", cooldown);
            }
        }

        UnitAction::Flee { threat } => {
            let threat_pos = match world.get_entity(threat) {
                Some(e) => e.position,
                None => return events,
            };
            if let Some(entity) = world.get_entity_mut(entity_id) {
                let speed = entity.stat("speed");
                let direction = if entity.position >= threat_pos { 1 } else { -1 };
                entity.position += direction * speed;
                events.push(SimEvent::EntityMoved {
                    entity_id,
                    new_position: entity.position,
                });
            }
        }

        UnitAction::Wait => {}

        UnitAction::SetTarget { target } => {
            if let Some(entity) = world.get_entity_mut(entity_id) {
                entity.target = Some(target);
            }
        }

        UnitAction::Print { text } => {
            events.push(SimEvent::ScriptOutput { entity_id, text });
        }

        // Instant resource actions are handled in the tick loop, not here.
        UnitAction::GainResource { .. } | UnitAction::TrySpendResource { .. } => {}

        UnitAction::Custom { name, args } => {
            events.push(SimEvent::CommandUsed {
                entity_id,
                command: name.clone(),
            });
            // Check if this is a phased command.
            if let Some(phases) = world.custom_command_phases.get(&name).cloned() {
                if !phases.is_empty() {
                    // Set up channel state — effects start next tick.
                    if let Some(entity) = world.get_entity_mut(entity_id) {
                        entity.active_channel = Some(crate::entity::ChannelState {
                            command_name: name,
                            args,
                            phases,
                            phase_index: 0,
                            ticks_elapsed_in_phase: 0,
                        });
                    }
                    return events;
                }
            }
            // Instant (non-phased) command.
            if let Some(effects) = world.custom_commands.get(&name).cloned() {
                let outcome = resolve_custom_effects(world, entity_id, &name, &effects, &args, &mut events);
                if let EffectOutcome::StartChannel { phases } = outcome {
                    if let Some(entity) = world.get_entity_mut(entity_id) {
                        entity.active_channel = Some(crate::entity::ChannelState {
                            command_name: name,
                            args,
                            phases,
                            phase_index: 0,
                            ticks_elapsed_in_phase: 0,
                        });
                    }
                }
            } else {
                events.push(SimEvent::ScriptOutput {
                    entity_id,
                    text: format!("[{name}] (no effects defined)"),
                });
            }
        }
    }

    events
}

/// Evaluate a condition against the current world state.
///
/// `args` and `ctx` are used for target-bearing conditions (`is_alive`, `distance`).
/// Callers without args/ctx (e.g., trigger condition checks) pass `&[]` and `&EffectContext::default()`.
pub fn evaluate_condition(
    condition: &Condition,
    world: &SimWorld,
    entity_id: EntityId,
    rng: &mut SimRng,
) -> bool {
    evaluate_condition_with_ctx(condition, world, entity_id, rng, &[], &EffectContext::default())
}

/// Evaluate a condition with full args and effect context for target resolution.
pub fn evaluate_condition_with_ctx(
    condition: &Condition,
    world: &SimWorld,
    entity_id: EntityId,
    rng: &mut SimRng,
    args: &[SimValue],
    ctx: &EffectContext,
) -> bool {
    match condition {
        Condition::Resource { resource, compare, amount } => {
            let current = world.get_resource(resource);
            let threshold = amount.resolve_with_world(rng, world, entity_id);
            compare.evaluate(current, threshold)
        }
        Condition::EntityCount { entity_type, compare, amount } => {
            let count = world.entities()
                .filter(|e| e.alive && e.spawn_ticks_remaining == 0 && e.has_type(entity_type))
                .count() as i64;
            let threshold = amount.resolve_with_world(rng, world, entity_id);
            compare.evaluate(count, threshold)
        }
        Condition::Stat { stat, compare, amount } => {
            let current = world.get_entity(entity_id).map_or(0, |e| e.stat(stat));
            let threshold = amount.resolve_with_world(rng, world, entity_id);
            compare.evaluate(current, threshold)
        }
        Condition::HasBuff { buff } => {
            world.get_entity(entity_id).map_or(false, |e| {
                e.active_buffs.iter().any(|b| b.name == *buff)
            })
        }
        Condition::RandomChance { percent } => {
            let roll = rng.next_bounded(100) as i64;
            roll < *percent
        }
        Condition::And { conditions } => {
            conditions.iter().all(|c| evaluate_condition_with_ctx(c, world, entity_id, rng, args, ctx))
        }
        Condition::Or { conditions } => {
            conditions.iter().any(|c| evaluate_condition_with_ctx(c, world, entity_id, rng, args, ctx))
        }
        Condition::IsAlive { target } => {
            let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
            match target_id {
                Some(tid) => world.get_entity(tid).map_or(false, |e| e.alive),
                None => false,
            }
        }
        Condition::Distance { target, compare, amount } => {
            let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
            match target_id {
                Some(tid) => {
                    let self_pos = world.get_entity(entity_id).map_or(0, |e| e.position);
                    let target_pos = world.get_entity(tid).map_or(0, |e| e.position);
                    let dist = (self_pos - target_pos).abs();
                    let threshold = amount.resolve_with_world(rng, world, entity_id);
                    compare.evaluate(dist, threshold)
                }
                None => false,
            }
        }
    }
}

/// Resolve custom command effects against the world.
/// Effects are resolved in order. A `UseResource` effect that fails (insufficient
/// resource) aborts the remaining effects — the command ends early.
/// Returns an `EffectOutcome` indicating completion, abort, or channel start.
pub fn resolve_custom_effects(
    world: &mut SimWorld,
    entity_id: EntityId,
    cmd_name: &str,
    effects: &[CommandEffect],
    args: &[SimValue],
    events: &mut Vec<SimEvent>,
) -> EffectOutcome {
    resolve_custom_effects_with_ctx(world, entity_id, cmd_name, effects, args, events, &EffectContext::default())
}

/// Resolve custom command effects with an explicit effect context for scoped targets.
pub fn resolve_custom_effects_with_ctx(
    world: &mut SimWorld,
    entity_id: EntityId,
    cmd_name: &str,
    effects: &[CommandEffect],
    args: &[SimValue],
    events: &mut Vec<SimEvent>,
    ctx: &EffectContext,
) -> EffectOutcome {
    // Derive an RNG from the world's current tick for deterministic randomness.
    let mut rng = SimRng::new(world.tick_seed() ^ entity_id.0 as u64);
    resolve_effects_inner(world, entity_id, cmd_name, effects, args, events, &mut rng, ctx)
}

/// Inner recursive effect resolver, sharing an RNG across nested calls.
fn resolve_effects_inner(
    world: &mut SimWorld,
    entity_id: EntityId,
    cmd_name: &str,
    effects: &[CommandEffect],
    args: &[SimValue],
    events: &mut Vec<SimEvent>,
    rng: &mut SimRng,
    ctx: &EffectContext,
) -> EffectOutcome {
    for effect in effects {
        match effect {
            CommandEffect::Output { message } => {
                events.push(SimEvent::ScriptOutput {
                    entity_id,
                    text: message.clone(),
                });
            }
            CommandEffect::Damage { target, amount } => {
                let amount = amount.resolve_with_world(rng, world, entity_id);
                let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
                if let Some(tid) = target_id {
                    if let Some(target_entity) = world.get_entity_mut(tid) {
                        let mut remaining = amount;
                        let shield = target_entity.stat("shield");
                        if shield > 0 {
                            let shield_absorbed = remaining.min(shield);
                            target_entity.set_stat("shield", shield - shield_absorbed);
                            remaining -= shield_absorbed;
                        }
                        let new_health = (target_entity.stat("health") - remaining).max(0);
                        target_entity.set_stat("health", new_health);
                        events.push(SimEvent::EntityDamaged {
                            entity_id: tid,
                            damage: amount,
                            new_health,
                            attacker_id: Some(entity_id),
                        });
                        if new_health <= 0 {
                            target_entity.alive = false;
                            let owner_id = target_entity.owner;
                            events.push(SimEvent::EntityDied {
                                entity_id: tid,
                                name: target_entity.name.clone(),
                                killer_id: Some(entity_id),
                                owner_id,
                            });
                        }
                    }
                }
            }
            CommandEffect::Heal { target, amount } => {
                let amount = amount.resolve_with_world(rng, world, entity_id);
                let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
                if let Some(tid) = target_id {
                    if let Some(target_entity) = world.get_entity_mut(tid) {
                        let new_health = target_entity.stat("health").saturating_add(amount);
                        target_entity.set_stat("health", new_health);
                        target_entity.clamp_stat("health");
                    }
                }
            }
            CommandEffect::Spawn { entity_id: spawn_entity_id, offset } => {
                let offset = offset.resolve_with_world(rng, world, entity_id);
                let position = world.get_entity(entity_id)
                    .map(|e| e.position + offset)
                    .unwrap_or(offset);
                let id = EntityId(world.next_entity_id());
                let types = world.entity_types_registry
                    .get(spawn_entity_id)
                    .cloned()
                    .unwrap_or_else(|| vec![spawn_entity_id.clone()]);
                let mut spawned = SimEntity::new_with_types(
                    id,
                    spawn_entity_id.clone(),
                    types,
                    format!("{}_{}", spawn_entity_id, id.0),
                    position,
                );
                // Apply entity config (stats) if defined for this type.
                if let Some(config) = world.entity_configs.get(spawn_entity_id) {
                    spawned.apply_config(config);
                }
                // Set owner to the entity that spawned this one.
                spawned.owner = Some(entity_id);
                // Set spawn duration so entity can't act until animation finishes.
                spawned.spawn_ticks_remaining = world.spawn_durations
                    .get(spawn_entity_id)
                    .copied()
                    .unwrap_or(0);
                // EntitySpawned event is emitted by flush_pending() when the
                // queued entity is actually added to the world.
                world.queue_spawn(spawned);
            }
            CommandEffect::ModifyStat { target, stat, amount } => {
                let amount = amount.resolve_with_world(rng, world, entity_id);
                let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
                if let Some(tid) = target_id {
                    if let Some(target_entity) = world.get_entity_mut(tid) {
                        let new_val = target_entity.stat(stat).saturating_add(amount);
                        target_entity.set_stat(stat, new_val);
                        target_entity.clamp_stat(stat);
                    }
                }
            }
            CommandEffect::UseResource { stat, amount } => {
                let amount = amount.resolve_with_world(rng, world, entity_id);
                let current = world.get_entity(entity_id).map_or(0, |e| e.stat(stat));
                if current < amount {
                    events.push(SimEvent::ScriptOutput {
                        entity_id,
                        text: format!("[{cmd_name}] not enough {stat}"),
                    });
                    return EffectOutcome::Aborted;
                }
                if let Some(entity) = world.get_entity_mut(entity_id) {
                    entity.set_stat(stat, (current - amount).max(0));
                }
            }
            CommandEffect::ListCommands => {
                // Use command_order to respect the order commands were made available.
                // Skip unlisted commands.
                // Compute max width for aligned output.
                let max_width = world.command_order.iter()
                    .filter(|n| world.custom_command_descriptions.contains_key(*n) && !world.unlisted_commands.contains(*n))
                    .map(|n| n.len() + 2) // +2 for "()"
                    .max()
                    .unwrap_or(0);
                for name in &world.command_order {
                    if world.unlisted_commands.contains(name) {
                        continue;
                    }
                    if let Some(description) = world.custom_command_descriptions.get(name) {
                        let padded = format!("{name}()");
                        events.push(SimEvent::ScriptOutput {
                            entity_id,
                            text: format!("{padded:<width$} — {description}", width = max_width + 1),
                        });
                    }
                }
            }
            CommandEffect::Animate { target, animation } => {
                let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
                if let Some(tid) = target_id {
                    events.push(SimEvent::PlayAnimation {
                        entity_id: tid,
                        animation: animation.clone(),
                    });
                }
            }
            CommandEffect::ModifyResource { resource, amount } => {
                let amount = amount.resolve_with_world(rng, world, entity_id);
                world.gain_resource(resource, amount);
            }
            CommandEffect::UseGlobalResource { resource, amount } => {
                let amount = amount.resolve_with_world(rng, world, entity_id);
                if !world.try_spend_resource(resource, amount) {
                    events.push(SimEvent::ScriptOutput {
                        entity_id,
                        text: format!("[{cmd_name}] not enough {resource}"),
                    });
                    return EffectOutcome::Aborted;
                }
            }
            CommandEffect::If { condition, then_effects, otherwise } => {
                let branch = if evaluate_condition_with_ctx(condition, world, entity_id, rng, args, ctx) {
                    then_effects
                } else {
                    otherwise
                };
                if !branch.is_empty() {
                    let outcome = resolve_effects_inner(world, entity_id, cmd_name, branch, args, events, rng, ctx);
                    if !matches!(outcome, EffectOutcome::Complete) {
                        return outcome;
                    }
                }
            }
            CommandEffect::StartChannel { phases } => {
                return EffectOutcome::StartChannel { phases: phases.clone() };
            }
            CommandEffect::ApplyBuff { target, buff, duration } => {
                let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
                if let Some(tid) = target_id {
                    if let Some(buff_def) = world.buff_registry.get(buff).cloned() {
                        let dur = duration.unwrap_or(buff_def.duration);

                        // Check if the entity already has this buff.
                        let existing = world.get_entity(tid)
                            .and_then(|e| e.active_buffs.iter().position(|b| b.name == *buff));

                        if let Some(idx) = existing {
                            if buff_def.stackable {
                                let at_max = buff_def.max_stacks > 0
                                    && world.get_entity(tid).map_or(true, |e| e.active_buffs[idx].stacks >= buff_def.max_stacks);
                                if !at_max {
                                    // Add a stack: apply modifiers, increment stacks.
                                    apply_buff_modifiers(world, tid, &buff_def);
                                    if let Some(entity) = world.get_entity_mut(tid) {
                                        entity.active_buffs[idx].stacks += 1;
                                        entity.active_buffs[idx].remaining_ticks = dur;
                                    }
                                }
                            } else {
                                // Refresh duration.
                                if let Some(entity) = world.get_entity_mut(tid) {
                                    entity.active_buffs[idx].remaining_ticks = dur;
                                }
                            }
                        } else {
                            // New buff: apply modifiers, run on_apply, create tracking.
                            apply_buff_modifiers(world, tid, &buff_def);
                            if let Some(entity) = world.get_entity_mut(tid) {
                                entity.active_buffs.push(crate::entity::ActiveBuff {
                                    name: buff.clone(),
                                    remaining_ticks: dur,
                                    stacks: 1,
                                });
                            }
                            // Run on_apply effects.
                            if !buff_def.on_apply.is_empty() {
                                let outcome = resolve_effects_inner(
                                    world, tid, cmd_name, &buff_def.on_apply, args, events, rng, ctx,
                                );
                                if !matches!(outcome, EffectOutcome::Complete) {
                                    return outcome;
                                }
                            }
                        }
                    }
                }
            }
            CommandEffect::RemoveBuff { target, buff } => {
                let target_id = resolve_target_from_args(entity_id, target, args, ctx, Some(world));
                if let Some(tid) = target_id {
                    if let Some(buff_def) = world.buff_registry.get(buff).cloned() {
                        let removed = world.get_entity(tid)
                            .and_then(|e| e.active_buffs.iter().position(|b| b.name == *buff))
                            .map(|idx| {
                                let stacks = world.get_entity(tid).unwrap().active_buffs[idx].stacks;
                                (idx, stacks)
                            });
                        if let Some((idx, stacks)) = removed {
                            // Reverse all stacks of modifiers.
                            for _ in 0..stacks {
                                reverse_buff_modifiers(world, tid, &buff_def);
                            }
                            if let Some(entity) = world.get_entity_mut(tid) {
                                entity.active_buffs.remove(idx);
                            }
                            // Run on_expire effects.
                            if !buff_def.on_expire.is_empty() {
                                let outcome = resolve_effects_inner(
                                    world, tid, cmd_name, &buff_def.on_expire, args, events, rng, ctx,
                                );
                                if !matches!(outcome, EffectOutcome::Complete) {
                                    return outcome;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    EffectOutcome::Complete
}

/// Apply a buff's stat modifiers to an entity (one stack worth).
pub(crate) fn apply_buff_modifiers(world: &mut SimWorld, entity_id: EntityId, buff_def: &BuffDef) {
    if let Some(entity) = world.get_entity_mut(entity_id) {
        for (stat, amount) in &buff_def.modifiers {
            match stat.as_str() {
                "health" => {
                    let new_max = entity.stat("max_health").saturating_add(*amount).max(1);
                    entity.set_stat("max_health", new_max);
                    let new_health = entity.stat("health").saturating_add(*amount).max(1).min(new_max);
                    entity.set_stat("health", new_health);
                }
                "shield" => {
                    let new_max = entity.stat("max_shield").saturating_add(*amount).max(0);
                    entity.set_stat("max_shield", new_max);
                    let new_shield = entity.stat("shield").saturating_add(*amount).max(0).min(new_max);
                    entity.set_stat("shield", new_shield);
                }
                _ => {
                    let new_val = entity.stat(stat).saturating_add(*amount).max(0);
                    entity.set_stat(stat, new_val);
                }
            }
        }
    }
}

/// Reverse a buff's stat modifiers on an entity (one stack worth).
pub(crate) fn reverse_buff_modifiers(world: &mut SimWorld, entity_id: EntityId, buff_def: &BuffDef) {
    if let Some(entity) = world.get_entity_mut(entity_id) {
        for (stat, amount) in &buff_def.modifiers {
            match stat.as_str() {
                "health" => {
                    let new_max = entity.stat("max_health").saturating_sub(*amount).max(1);
                    entity.set_stat("max_health", new_max);
                    let clamped = entity.stat("health").min(new_max).max(1);
                    entity.set_stat("health", clamped);
                }
                "shield" => {
                    let new_max = entity.stat("max_shield").saturating_sub(*amount).max(0);
                    entity.set_stat("max_shield", new_max);
                    let clamped = entity.stat("shield").min(new_max).max(0);
                    entity.set_stat("shield", clamped);
                }
                _ => {
                    let new_val = entity.stat(stat).saturating_sub(*amount).max(0);
                    entity.set_stat(stat, new_val);
                }
            }
        }
    }
}

/// Resolve target string to EntityId using positional args and scoped effect context.
///
/// Supported targets:
/// - `"self"` → executing entity
/// - `"arg:<name>"` → matched by position (first arg = index 0)
/// - `"source"` → the event subject (from `EffectContext`)
/// - `"owner"` → owner from context, or fallback to entity's stored owner field
/// - `"attacker"` → the entity that dealt damage (from `EffectContext`)
/// - `"killer"` → the entity that dealt the killing blow (from `EffectContext`)
fn resolve_target_from_args(
    entity_id: EntityId,
    target_str: &str,
    args: &[SimValue],
    ctx: &EffectContext,
    world: Option<&SimWorld>,
) -> Option<EntityId> {
    if target_str == "self" {
        return Some(entity_id);
    }
    // Scoped targets from EffectContext.
    match target_str {
        "source" => return ctx.source,
        "attacker" => return ctx.attacker,
        "killer" => return ctx.killer,
        "owner" => {
            // Try context first (for dead entities whose owner was captured at death time).
            if ctx.owner.is_some() {
                return ctx.owner;
            }
            // Fallback: look up the executing entity's stored owner field.
            if let Some(w) = world {
                return w.get_entity(entity_id).and_then(|e| e.owner);
            }
            return None;
        }
        _ => {}
    }
    if let Some(arg_ref) = target_str.strip_prefix("arg:") {
        // Try as numeric index first.
        if let Ok(idx) = arg_ref.parse::<usize>() {
            if let Some(SimValue::EntityRef(eid)) = args.get(idx) {
                return Some(*eid);
            }
        }
        // Named arg: treat as positional — first defined arg name = index 0, etc.
        // Since we can't look up arg names here, fall back to matching the first
        // entity ref in args if there's exactly one arg.
        if args.len() == 1 {
            if let Some(SimValue::EntityRef(eid)) = args.first() {
                return Some(*eid);
            }
        }
    }
    None
}
