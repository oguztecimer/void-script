//! Lua trigger dispatch and buff callback invocation.

use mlua::{Lua, Function, Value};

use deadcode_sim::action::BuffCallbackType;
use deadcode_sim::entity::EntityId;
use deadcode_sim::world::{SimEvent, WorldAccess};

use crate::api::CtxUserData;
use crate::coroutine::{CasterEntity, CollectedEvents, WorldAccessPtr};

/// Process Lua-registered triggers against collected events.
pub fn process_lua_triggers(
    lua: &Lua,
    world: &mut WorldAccess,
    events: &[SimEvent],
) -> Vec<SimEvent> {
    let mut all_events = Vec::new();

    // Find all trigger tables.
    let trigger_keys: Vec<String> = lua.globals()
        .pairs::<String, Value>()
        .flatten()
        .filter(|(k, v)| k.starts_with("__mod_") && k.ends_with("_triggers") && matches!(v, Value::Table(_)))
        .map(|(k, _)| k)
        .collect();

    for key in trigger_keys {
        let Ok(triggers_table) = lua.globals().get::<mlua::Table>(key.as_str()) else { continue };

        for i in 1..=triggers_table.raw_len() {
            let Ok(entry) = triggers_table.get::<mlua::Table>(i) else { continue };
            let Ok(event_name) = entry.get::<String>("event") else { continue };
            let Ok(handler) = entry.get::<Function>("handler") else { continue };
            let filter = entry.get::<mlua::Table>("opts").ok()
                .and_then(|t| t.get::<mlua::Table>("filter").ok());

            // Match events.
            for sim_event in events {
                let (matches, event_data) = match_event(lua, sim_event, &event_name, filter.as_ref());
                if !matches { continue; }

                // Create ctx and call handler.
                let caster_id = world.caster_id;
                let ctx = CtxUserData::new(caster_id, world, &[]);
                let Ok(ud) = lua.create_userdata(ctx) else { continue };

                let world_ptr = world as *mut WorldAccess as usize;
                lua.set_app_data(WorldAccessPtr(world_ptr));
                lua.set_app_data(CasterEntity(caster_id));

                let _ = handler.call::<()>((Value::UserData(ud), event_data));

                if let Some(collected) = lua.remove_app_data::<CollectedEvents>() {
                    all_events.extend(collected.0);
                }
                lua.remove_app_data::<WorldAccessPtr>();
                lua.remove_app_data::<CasterEntity>();
            }
        }
    }

    all_events
}

/// Check if a SimEvent matches a trigger's event name and optional filter.
/// Returns (matched, event_data_table) where event_data is a Lua table with event details.
fn match_event(
    lua: &Lua,
    event: &SimEvent,
    event_name: &str,
    filter: Option<&mlua::Table>,
) -> (bool, Value) {
    match (event_name, event) {
        ("entity_died", SimEvent::EntityDied { entity_id, name, killer_id, owner_id }) => {
            // Check entity_type filter.
            if let Some(filter) = filter
                && let Ok(filter_type) = filter.get::<String>("entity_type") {
                    // We don't have type info in the event directly — would need
                    // world access to check. For now, match by name prefix.
                    // TODO: pass types in event data for proper filtering.
                    let _ = filter_type;
                }
            let table = lua.create_table().unwrap();
            let _ = table.set("entity_id", entity_id.0 as i64);
            let _ = table.set("name", name.as_str());
            if let Some(kid) = killer_id { let _ = table.set("killer_id", kid.0 as i64); }
            if let Some(oid) = owner_id { let _ = table.set("owner_id", oid.0 as i64); }
            (true, Value::Table(table))
        }
        ("entity_spawned", SimEvent::EntitySpawned { entity_id, entity_type, name, position, spawner_id }) => {
            if let Some(filter) = filter
                && let Ok(filter_type) = filter.get::<String>("entity_type")
                    && *entity_type != filter_type { return (false, Value::Nil); }
            let table = lua.create_table().unwrap();
            let _ = table.set("entity_id", entity_id.0 as i64);
            let _ = table.set("entity_type", entity_type.as_str());
            let _ = table.set("name", name.as_str());
            let _ = table.set("position", *position);
            if let Some(sid) = spawner_id { let _ = table.set("spawner_id", sid.0 as i64); }
            (true, Value::Table(table))
        }
        ("entity_damaged", SimEvent::EntityDamaged { entity_id, damage, new_health, attacker_id }) => {
            let table = lua.create_table().unwrap();
            let _ = table.set("entity_id", entity_id.0 as i64);
            let _ = table.set("damage", *damage);
            let _ = table.set("new_health", *new_health);
            if let Some(aid) = attacker_id { let _ = table.set("attacker_id", aid.0 as i64); }
            (true, Value::Table(table))
        }
        ("command_used", SimEvent::CommandUsed { entity_id, command }) => {
            if let Some(filter) = filter
                && let Ok(filter_cmd) = filter.get::<String>("command")
                    && *command != filter_cmd { return (false, Value::Nil); }
            let table = lua.create_table().unwrap();
            let _ = table.set("entity_id", entity_id.0 as i64);
            let _ = table.set("command", command.as_str());
            (true, Value::Table(table))
        }
        ("channel_completed", SimEvent::ChannelCompleted { entity_id, command }) => {
            if let Some(filter) = filter
                && let Ok(filter_cmd) = filter.get::<String>("command")
                    && *command != filter_cmd { return (false, Value::Nil); }
            let table = lua.create_table().unwrap();
            let _ = table.set("entity_id", entity_id.0 as i64);
            let _ = table.set("command", command.as_str());
            (true, Value::Table(table))
        }
        _ => (false, Value::Nil),
    }
}

/// Run a buff callback (on_apply, per_tick, on_expire) from Lua.
pub fn run_buff_callback(
    lua: &Lua,
    world: &mut WorldAccess,
    entity_id: EntityId,
    buff_name: &str,
    callback_type: BuffCallbackType,
) -> Vec<SimEvent> {
    let mut all_events = Vec::new();

    // Find buff tables.
    let buff_keys: Vec<String> = lua.globals()
        .pairs::<String, Value>()
        .flatten()
        .filter(|(k, v)| k.starts_with("__mod_") && k.ends_with("_buffs") && matches!(v, Value::Table(_)))
        .map(|(k, _)| k)
        .collect();

    let callback_name = match callback_type {
        BuffCallbackType::OnApply => "on_apply",
        BuffCallbackType::PerTick => "per_tick",
        BuffCallbackType::OnExpire => "on_expire",
    };

    for key in buff_keys {
        let Ok(buffs_table) = lua.globals().get::<mlua::Table>(key.as_str()) else { continue };
        let Ok(callbacks) = buffs_table.get::<mlua::Table>(buff_name) else { continue };
        let Ok(handler) = callbacks.get::<Function>(callback_name) else { continue };

        let ctx = CtxUserData::new(entity_id, world, &[]);
        let Ok(ud) = lua.create_userdata(ctx) else { continue };

        let world_ptr = world as *mut WorldAccess as usize;
        lua.set_app_data(WorldAccessPtr(world_ptr));
        lua.set_app_data(CasterEntity(entity_id));

        let _ = handler.call::<()>((Value::UserData(ud), entity_id.0 as i64));

        if let Some(collected) = lua.remove_app_data::<CollectedEvents>() {
            all_events.extend(collected.0);
        }
        lua.remove_app_data::<WorldAccessPtr>();
        lua.remove_app_data::<CasterEntity>();
    }

    all_events
}
