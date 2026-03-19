use crate::entity::EntityId;
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
    /// Consult the spirits (necromancer starter).
    Consult,
    /// Raise the dead (necromancer starter).
    Raise,
    /// Harvest essence (necromancer starter).
    Harvest,
    /// Forge a dark pact (necromancer starter).
    Pact,
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

        UnitAction::Consult => {
            events.push(SimEvent::ScriptOutput {
                entity_id,
                text: "[consult] Consulting the spirits...".to_string(),
            });
        }
        UnitAction::Raise => {
            events.push(SimEvent::ScriptOutput {
                entity_id,
                text: "[raise] Raising the dead...".to_string(),
            });
        }
        UnitAction::Harvest => {
            events.push(SimEvent::ScriptOutput {
                entity_id,
                text: "[harvest] Harvesting essence...".to_string(),
            });
        }
        UnitAction::Pact => {
            events.push(SimEvent::ScriptOutput {
                entity_id,
                text: "[pact] Forging a dark pact...".to_string(),
            });
        }
    }

    events
}
