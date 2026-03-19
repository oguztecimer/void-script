use serde::{Deserialize, Serialize};

use crate::entity::{EntityId, SimEntity};
use crate::value::SimValue;
use crate::world::{SimEvent, SimWorld};

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
    /// Custom mod-defined command with resolved arguments.
    Custom { name: String, args: Vec<SimValue> },
}

/// An effect that a custom command applies when resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandEffect {
    /// Print a message to the console.
    #[serde(rename = "output")]
    Output { message: String },
    /// Deal damage to a target (shield-first).
    #[serde(rename = "damage")]
    Damage { target: String, amount: i64 },
    /// Heal a target (capped at max).
    #[serde(rename = "heal")]
    Heal { target: String, amount: i64 },
    /// Spawn an entity at self.position + offset.
    #[serde(rename = "spawn")]
    Spawn { entity_type: String, offset: i64 },
    /// Add to a stat (health/energy/shield/speed).
    #[serde(rename = "modify_stat")]
    ModifyStat { target: String, stat: String, amount: i64 },
    /// Check and deduct a resource; if insufficient, abort remaining effects.
    #[serde(rename = "use_resource")]
    UseResource { stat: String, amount: i64 },
    /// List all registered commands and their descriptions.
    #[serde(rename = "list_commands")]
    ListCommands,
}

/// Definition of a custom command (parsed from mod.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub effects: Vec<CommandEffect>,
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
                let speed = entity.speed;
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
                Some(e) => (e.attack_damage, e.attack_range, e.position),
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
                if target_entity.shield > 0 {
                    let shield_absorbed = remaining.min(target_entity.shield);
                    target_entity.shield -= shield_absorbed;
                    remaining -= shield_absorbed;
                }
                target_entity.health = (target_entity.health - remaining).max(0);

                events.push(SimEvent::EntityDamaged {
                    entity_id: target,
                    damage,
                    new_health: target_entity.health,
                });

                if target_entity.health <= 0 {
                    target_entity.alive = false;
                    events.push(SimEvent::EntityDied { entity_id: target });
                }
            }

            if let Some(attacker) = world.get_entity_mut(entity_id) {
                attacker.cooldown_remaining = attacker.attack_cooldown;
            }
        }

        UnitAction::Flee { threat } => {
            let threat_pos = match world.get_entity(threat) {
                Some(e) => e.position,
                None => return events,
            };
            if let Some(entity) = world.get_entity_mut(entity_id) {
                let speed = entity.speed;
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


        UnitAction::Custom { name, args } => {
            if let Some(effects) = world.custom_commands.get(&name).cloned() {
                resolve_custom_effects(world, entity_id, &name, &effects, &args, &mut events);
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

/// Resolve custom command effects against the world.
/// Effects are resolved in order. A `UseResource` effect that fails (insufficient
/// resource) aborts the remaining effects — the command ends early.
pub fn resolve_custom_effects(
    world: &mut SimWorld,
    entity_id: EntityId,
    cmd_name: &str,
    effects: &[CommandEffect],
    args: &[SimValue],
    events: &mut Vec<SimEvent>,
) {
    for effect in effects {
        match effect {
            CommandEffect::Output { message } => {
                events.push(SimEvent::ScriptOutput {
                    entity_id,
                    text: message.clone(),
                });
            }
            CommandEffect::Damage { target, amount } => {
                let target_id = resolve_target_from_args(entity_id, target, args);
                if let Some(tid) = target_id {
                    if let Some(target_entity) = world.get_entity_mut(tid) {
                        let mut remaining = *amount;
                        if target_entity.shield > 0 {
                            let shield_absorbed = remaining.min(target_entity.shield);
                            target_entity.shield -= shield_absorbed;
                            remaining -= shield_absorbed;
                        }
                        target_entity.health = (target_entity.health - remaining).max(0);
                        events.push(SimEvent::EntityDamaged {
                            entity_id: tid,
                            damage: *amount,
                            new_health: target_entity.health,
                        });
                        if target_entity.health <= 0 {
                            target_entity.alive = false;
                            events.push(SimEvent::EntityDied { entity_id: tid });
                        }
                    }
                }
            }
            CommandEffect::Heal { target, amount } => {
                let target_id = resolve_target_from_args(entity_id, target, args);
                if let Some(tid) = target_id {
                    if let Some(target_entity) = world.get_entity_mut(tid) {
                        target_entity.health = (target_entity.health + amount).min(target_entity.max_health);
                    }
                }
            }
            CommandEffect::Spawn { entity_type, offset } => {
                let position = world.get_entity(entity_id)
                    .map(|e| e.position + offset)
                    .unwrap_or(*offset);
                let spawned = SimEntity::new(
                    EntityId(world.next_entity_id()),
                    entity_type.clone(),
                    format!("{}_{}", entity_type, position),
                    position,
                );
                let spawned_id = spawned.id;
                // Apply entity config if available.
                world.queue_spawn(spawned);
                events.push(SimEvent::EntitySpawned {
                    entity_id: spawned_id,
                    entity_type: entity_type.clone(),
                    position,
                });
            }
            CommandEffect::ModifyStat { target, stat, amount } => {
                let target_id = resolve_target_from_args(entity_id, target, args);
                if let Some(tid) = target_id {
                    if let Some(target_entity) = world.get_entity_mut(tid) {
                        match stat.as_str() {
                            "health" => {
                                target_entity.health = (target_entity.health + amount)
                                    .max(0)
                                    .min(target_entity.max_health);
                            }
                            "energy" => {
                                target_entity.energy = (target_entity.energy + amount)
                                    .max(0)
                                    .min(target_entity.max_energy);
                            }
                            "shield" => {
                                target_entity.shield = (target_entity.shield + amount).max(0);
                            }
                            "speed" => {
                                target_entity.speed = (target_entity.speed + amount).max(0);
                            }
                            _ => {}
                        }
                    }
                }
            }
            CommandEffect::UseResource { stat, amount } => {
                let has_enough = world.get_entity(entity_id).map_or(false, |e| {
                    let current = match stat.as_str() {
                        "health" => e.health,
                        "energy" => e.energy,
                        "shield" => e.shield,
                        _ => return false,
                    };
                    current >= *amount
                });
                if !has_enough {
                    events.push(SimEvent::ScriptOutput {
                        entity_id,
                        text: format!("[{cmd_name}] not enough {stat}"),
                    });
                    return; // Abort remaining effects.
                }
                // Deduct the resource.
                if let Some(entity) = world.get_entity_mut(entity_id) {
                    match stat.as_str() {
                        "health" => entity.health = (entity.health - amount).max(0),
                        "energy" => entity.energy = (entity.energy - amount).max(0),
                        "shield" => entity.shield = (entity.shield - amount).max(0),
                        _ => {}
                    }
                }
            }
            CommandEffect::ListCommands => {
                let mut commands: Vec<(&String, &String)> =
                    world.custom_command_descriptions.iter().collect();
                commands.sort_by_key(|(name, _)| (*name).clone());
                events.push(SimEvent::ScriptOutput {
                    entity_id,
                    text: "The bones speak... they reveal known commands:".into(),
                });
                for (name, description) in commands {
                    events.push(SimEvent::ScriptOutput {
                        entity_id,
                        text: format!("  {name} — {description}"),
                    });
                }
            }
        }
    }
}

/// Resolve target string to EntityId using positional args.
/// "self" → executing entity, "arg:<name>" → matched by position (first arg = index 0).
fn resolve_target_from_args(
    entity_id: EntityId,
    target_str: &str,
    args: &[SimValue],
) -> Option<EntityId> {
    if target_str == "self" {
        return Some(entity_id);
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
