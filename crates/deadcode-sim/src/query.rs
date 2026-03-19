use crate::entity::EntityId;
use crate::error::SimError;
use crate::value::SimValue;
use crate::world::SimWorld;

/// Scan for entities matching a type filter string.
/// Returns a list of EntityRef values. Excludes the querying entity itself.
pub fn scan(world: &SimWorld, self_id: EntityId, filter: &str) -> Vec<SimValue> {
    let filter = filter.to_lowercase();
    world
        .entities()
        .filter(|e| e.alive && e.id != self_id)
        .filter(|e| filter.is_empty() || filter == "*" || filter == "all" || e.entity_type == filter)
        .map(|e| SimValue::EntityRef(e.id))
        .collect()
}

/// Find nearest entity matching filter. Returns EntityRef or None.
pub fn nearest(world: &SimWorld, self_id: EntityId, filter: &str) -> SimValue {
    let self_pos = match world.get_entity(self_id) {
        Some(e) => e.position,
        None => return SimValue::None,
    };

    let filter = filter.to_lowercase();

    let mut best: Option<(EntityId, i64)> = None;
    for e in world.entities() {
        if !e.alive || e.id == self_id {
            continue;
        }
        if !filter.is_empty() && filter != "*" && filter != "all" && e.entity_type != filter {
            continue;
        }
        let dist = (e.position - self_pos).abs();
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((e.id, dist));
        }
    }

    match best {
        Some((id, _)) => SimValue::EntityRef(id),
        None => SimValue::None,
    }
}

/// Get position of an entity.
pub fn get_pos(world: &SimWorld, id: EntityId) -> Result<i64, SimError> {
    world
        .get_entity(id)
        .map(|e| e.position)
        .ok_or_else(|| SimError::entity_not_found(id.0))
}

/// Distance between two entities.
pub fn distance(world: &SimWorld, a: EntityId, b: EntityId) -> Result<i64, SimError> {
    let ea = world
        .get_entity(a)
        .ok_or_else(|| SimError::entity_not_found(a.0))?;
    let eb = world
        .get_entity(b)
        .ok_or_else(|| SimError::entity_not_found(b.0))?;
    Ok((ea.position - eb.position).abs())
}

/// Get a stat (health/energy/shield) from an entity.
pub fn get_stat(world: &SimWorld, id: EntityId, stat: &str) -> Result<SimValue, SimError> {
    let e = world
        .get_entity(id)
        .ok_or_else(|| SimError::entity_not_found(id.0))?;
    let val = match stat {
        "health" => e.health,
        "energy" => e.energy,
        "shield" => e.shield,
        _ => return Err(SimError::type_error(format!("unknown stat: {stat}"))),
    };
    Ok(SimValue::Int(val))
}

/// Get entity's current target.
pub fn get_target(world: &SimWorld, id: EntityId) -> Result<SimValue, SimError> {
    let e = world
        .get_entity(id)
        .ok_or_else(|| SimError::entity_not_found(id.0))?;
    Ok(match e.target {
        Some(tid) => SimValue::EntityRef(tid),
        None => SimValue::None,
    })
}

/// Check if entity has a target.
pub fn has_target(world: &SimWorld, id: EntityId) -> Result<bool, SimError> {
    world
        .get_entity(id)
        .map(|e| e.target.is_some())
        .ok_or_else(|| SimError::entity_not_found(id.0))
}

/// Get entity type as string.
pub fn get_type(world: &SimWorld, id: EntityId) -> Result<String, SimError> {
    world
        .get_entity(id)
        .map(|e| e.entity_type.clone())
        .ok_or_else(|| SimError::entity_not_found(id.0))
}

/// Get entity name.
pub fn get_name(world: &SimWorld, id: EntityId) -> Result<String, SimError> {
    world
        .get_entity(id)
        .map(|e| e.name.clone())
        .ok_or_else(|| SimError::entity_not_found(id.0))
}

/// Get entity owner.
pub fn get_owner(world: &SimWorld, id: EntityId) -> Result<u64, SimError> {
    world
        .get_entity(id)
        .map(|e| e.owner)
        .ok_or_else(|| SimError::entity_not_found(id.0))
}

/// Get an entity attribute by name (used by GetAttr instruction on EntityRef).
pub fn get_entity_attr(
    world: &SimWorld,
    id: EntityId,
    attr: &str,
) -> Result<SimValue, SimError> {
    let e = world
        .get_entity(id)
        .ok_or_else(|| SimError::entity_not_found(id.0))?;
    match attr {
        "position" | "pos" | "x" => Ok(SimValue::Int(e.position)),
        "health" | "hp" => Ok(SimValue::Int(e.health)),
        "max_health" | "max_hp" => Ok(SimValue::Int(e.max_health)),
        "energy" => Ok(SimValue::Int(e.energy)),
        "max_energy" => Ok(SimValue::Int(e.max_energy)),
        "shield" => Ok(SimValue::Int(e.shield)),
        "max_shield" => Ok(SimValue::Int(e.max_shield)),
        "speed" => Ok(SimValue::Int(e.speed)),
        "name" => Ok(SimValue::Str(e.name.clone())),
        "type" => Ok(SimValue::Str(e.entity_type.clone())),
        "owner" => Ok(SimValue::Int(e.owner as i64)),
        "alive" => Ok(SimValue::Bool(e.alive)),
        "attack_damage" => Ok(SimValue::Int(e.attack_damage)),
        "attack_range" => Ok(SimValue::Int(e.attack_range)),
        "target" => Ok(match e.target {
            Some(tid) => SimValue::EntityRef(tid),
            None => SimValue::None,
        }),
        _ => Err(SimError::type_error(format!(
            "entity has no attribute '{attr}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::SimWorld;

    #[test]
    fn scan_filters_by_type() {
        let mut world = SimWorld::new(42);
        let unit = world.spawn_entity("skeleton".into(), "skel1".into(), 100);
        let _other = world.spawn_entity("zombie".into(), "zom1".into(), 200);
        let _grave = world.spawn_entity("grave".into(), "grave1".into(), 150);

        let results = scan(&world, unit, "grave");
        assert_eq!(results.len(), 1);

        let results = scan(&world, unit, "");
        assert_eq!(results.len(), 2); // zombie + grave (not self)
    }

    #[test]
    fn nearest_finds_closest() {
        let mut world = SimWorld::new(42);
        let unit = world.spawn_entity("skeleton".into(), "skel1".into(), 100);
        let _g1 = world.spawn_entity("grave".into(), "grave1".into(), 110);
        let _g2 = world.spawn_entity("grave".into(), "grave2".into(), 500);

        let result = nearest(&world, unit, "grave");
        match result {
            SimValue::EntityRef(id) => {
                let e = world.get_entity(id).unwrap();
                assert_eq!(e.position, 110);
            }
            _ => panic!("expected EntityRef"),
        }
    }

    #[test]
    fn distance_correct() {
        let mut world = SimWorld::new(42);
        let a = world.spawn_entity("skeleton".into(), "a".into(), 100);
        let b = world.spawn_entity("zombie".into(), "b".into(), 250);
        assert_eq!(distance(&world, a, b).unwrap(), 150);
    }
}
