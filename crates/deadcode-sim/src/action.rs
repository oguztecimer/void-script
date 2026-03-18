use crate::entity::{EntityId, EntityType};
use crate::world::{SimEvent, SimWorld};

/// An action a unit wants to perform this tick.
#[derive(Debug, Clone)]
pub enum UnitAction {
    /// Move toward a target position by `speed` units.
    Move { target_pos: i64 },
    /// Attack a target entity.
    Attack { target: EntityId },
    /// Mine the nearest asteroid in range.
    Mine,
    /// Deposit cargo at nearest station/mothership.
    Deposit,
    /// Flee from a threat (move away).
    Flee { threat: EntityId },
    /// Do nothing for one tick.
    Wait,
    /// Set the unit's target.
    SetTarget { target: EntityId },
    /// Transfer cargo to current target.
    Transfer { resource: String, amount: i64 },
    /// Print a value (not really a game action, but uses the same yield path).
    Print { text: String },
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
            // Read attacker stats.
            let (damage, range, attacker_pos) = match world.get_entity(entity_id) {
                Some(e) => (e.attack_damage, e.attack_range, e.position),
                None => return events,
            };

            // Check range and apply damage.
            match world.get_entity(target) {
                Some(t) if t.alive => {
                    let dist = (t.position - attacker_pos).abs();
                    if dist > range {
                        return events; // Out of range, action wasted.
                    }
                }
                _ => return events,
            };

            if let Some(target_entity) = world.get_entity_mut(target) {
                // Damage goes through shield first, then health.
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

            // Set cooldown.
            if let Some(attacker) = world.get_entity_mut(entity_id) {
                attacker.cooldown_remaining = attacker.attack_cooldown;
            }
        }

        UnitAction::Mine => {
            let (pos, range, mine_amount, cargo_full) = match world.get_entity(entity_id) {
                Some(e) => (e.position, e.mine_range, e.mine_amount, e.cargo_full()),
                None => return events,
            };
            if cargo_full {
                return events;
            }

            // Find nearest asteroid in range.
            let asteroid_id = world
                .entities()
                .filter(|e| {
                    e.alive
                        && e.entity_type == EntityType::Asteroid
                        && (e.position - pos).abs() <= range
                })
                .min_by_key(|e| (e.position - pos).abs())
                .map(|e| e.id);

            if let Some(aid) = asteroid_id {
                // Extract resource from asteroid.
                let resource = {
                    let asteroid = world.get_entity(aid).unwrap();
                    asteroid
                        .cargo
                        .first()
                        .map(|(r, _)| r.clone())
                        .unwrap_or_else(|| "iron".to_string())
                };

                let mined = {
                    let asteroid = world.get_entity_mut(aid).unwrap();
                    asteroid.remove_cargo(&resource, mine_amount)
                };

                if mined > 0 {
                    let miner = world.get_entity_mut(entity_id).unwrap();
                    let added = miner.add_cargo(&resource, mined);
                    events.push(SimEvent::ResourceMined {
                        miner_id: entity_id,
                        asteroid_id: aid,
                        resource: resource.clone(),
                        amount: added,
                    });

                    // Check if asteroid is depleted.
                    let asteroid = world.get_entity(aid).unwrap();
                    if asteroid.cargo_total() <= 0 {
                        let asteroid = world.get_entity_mut(aid).unwrap();
                        asteroid.alive = false;
                        events.push(SimEvent::EntityDied { entity_id: aid });
                    }
                }
            }
        }

        UnitAction::Deposit => {
            let (pos, range) = match world.get_entity(entity_id) {
                Some(e) => (e.position, e.mine_range), // reuse mine_range for deposit range
                None => return events,
            };

            // Find nearest mothership/station in range.
            let depot_id = world
                .entities()
                .filter(|e| {
                    e.alive
                        && matches!(e.entity_type, EntityType::Mothership | EntityType::Station)
                        && (e.position - pos).abs() <= range
                })
                .min_by_key(|e| (e.position - pos).abs())
                .map(|e| e.id);

            if let Some(did) = depot_id {
                // Transfer all cargo.
                let cargo_snapshot = match world.get_entity(entity_id) {
                    Some(e) => e.cargo.clone(),
                    None => return events,
                };

                for (resource, amount) in &cargo_snapshot {
                    if let Some(miner) = world.get_entity_mut(entity_id) {
                        miner.remove_cargo(resource, *amount);
                    }
                    if let Some(depot) = world.get_entity_mut(did) {
                        depot.add_cargo(resource, *amount);
                    }
                    events.push(SimEvent::CargoDeposited {
                        entity_id,
                        depot_id: did,
                        resource: resource.clone(),
                        amount: *amount,
                    });
                }
            }
        }

        UnitAction::Flee { threat } => {
            let threat_pos = match world.get_entity(threat) {
                Some(e) => e.position,
                None => return events,
            };
            if let Some(entity) = world.get_entity_mut(entity_id) {
                let speed = entity.speed;
                // Move away from threat.
                let direction = if entity.position >= threat_pos { 1 } else { -1 };
                entity.position += direction * speed;
                events.push(SimEvent::EntityMoved {
                    entity_id,
                    new_position: entity.position,
                });
            }
        }

        UnitAction::Wait => {
            // Intentional no-op.
        }

        UnitAction::SetTarget { target } => {
            if let Some(entity) = world.get_entity_mut(entity_id) {
                entity.target = Some(target);
            }
        }

        UnitAction::Transfer { resource, amount } => {
            let target_id = match world.get_entity(entity_id) {
                Some(e) => match e.target {
                    Some(tid) => tid,
                    None => return events,
                },
                None => return events,
            };

            let removed = match world.get_entity_mut(entity_id) {
                Some(e) => e.remove_cargo(&resource, amount),
                None => return events,
            };

            if removed > 0 {
                if let Some(target) = world.get_entity_mut(target_id) {
                    target.add_cargo(&resource, removed);
                }
                events.push(SimEvent::CargoDeposited {
                    entity_id,
                    depot_id: target_id,
                    resource,
                    amount: removed,
                });
            }
        }

        UnitAction::Print { text } => {
            events.push(SimEvent::ScriptOutput { entity_id, text });
        }
    }

    events
}
