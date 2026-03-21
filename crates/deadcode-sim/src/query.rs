use crate::entity::EntityId;
use crate::error::SimError;
use crate::value::SimValue;
use crate::world::SimWorld;

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
        "name" => Ok(SimValue::Str(e.name.clone())),
        "type" => Ok(SimValue::Str(e.entity_type.clone())),
        "types" => Ok(SimValue::List(e.types.iter().map(|t| SimValue::Str(t.clone())).collect())),
        "owner" => Ok(match e.owner {
            Some(owner_id) => SimValue::EntityRef(owner_id),
            None => SimValue::None,
        }),
        "alive" => Ok(SimValue::Bool(e.alive)),
        "target" => Ok(match e.target {
            Some(tid) => SimValue::EntityRef(tid),
            None => SimValue::None,
        }),
        // All other attrs resolve as stats (returns 0 for unknown).
        _ => Ok(SimValue::Int(e.stat(attr))),
    }
}
