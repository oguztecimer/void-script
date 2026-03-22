//! Lua API: `ctx` userdata methods and `mod` registration functions.
//!
//! The mod API is exposed via global functions prefixed with `__void_` that
//! the `require("void")` shim wraps into a nice Lua table.

use mlua::{Lua, Function, Value, UserDataMethods};

use deadcode_sim::action::CommandMeta;
use deadcode_sim::entity::EntityId;
use deadcode_sim::rng::SimRng;
use deadcode_sim::value::SimValue;
use deadcode_sim::world::{SimEvent, WorldAccess};

use crate::convert;
use crate::coroutine::{CasterEntity, CollectedEvents, WorldAccessPtr, YieldInfo};
use crate::error::LuaModError;

// ---------------------------------------------------------------------------
// ctx userdata — passed to command handlers
// ---------------------------------------------------------------------------

/// Userdata representing the execution context for a command.
///
/// Methods on ctx access the world through Lua's app_data (WorldAccessPtr)
/// which is set up before each coroutine resume.
pub struct CtxUserData {
    pub caster_id: EntityId,
    #[allow(dead_code)]
    pub args: Vec<SimValue>,
    pub tick_seed: u64,
    rng_counter: u64,
}

impl CtxUserData {
    pub fn new(caster_id: EntityId, world: &WorldAccess, args: &[SimValue]) -> Self {
        Self {
            caster_id,
            args: args.to_vec(),
            tick_seed: world.tick_seed(),
            rng_counter: 0,
        }
    }

    /// Get a deterministic RNG seeded from tick and caster.
    fn rng(&mut self) -> SimRng {
        self.rng_counter += 1;
        SimRng::new(self.tick_seed ^ self.caster_id.0 ^ self.rng_counter)
    }
}

/// Resolve a target value from Lua to an EntityId.
fn resolve_target_val(val: &Value, caster_id: EntityId) -> Option<EntityId> {
    convert::lua_to_entity_id(val, caster_id)
}

/// Helper: get the world access pointer from Lua app_data.
/// Panics if not available (should only be called during coroutine resume).
fn with_world<F, R>(lua: &Lua, f: F) -> R
where
    F: FnOnce(&mut WorldAccess) -> R,
{
    let ptr = lua.app_data_ref::<WorldAccessPtr>()
        .expect("WorldAccess not available (called outside coroutine resume)");
    let world = unsafe { &mut *(ptr.0 as *mut WorldAccess) };
    f(world)
}

/// Helper: push an event to the collected events buffer.
fn push_event(lua: &Lua, event: SimEvent) {
    let mut collected = lua.remove_app_data::<CollectedEvents>()
        .unwrap_or(CollectedEvents(Vec::new()));
    collected.0.push(event);
    lua.set_app_data(collected);
}

// Register ctx methods on the CtxUserData type.
impl mlua::UserData for CtxUserData {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // --- Entity operations ---

        methods.add_method("damage", |lua, this, (target, amount): (Value, i64)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            with_world(lua, |w| w.damage(this.caster_id, target_id, amount));
            Ok(())
        });

        methods.add_method("heal", |lua, this, (target, amount): (Value, i64)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            with_world(lua, |w| w.heal(target_id, amount));
            Ok(())
        });

        methods.add_method("modify_stat", |lua, this, (target, stat, amount): (Value, String, i64)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            with_world(lua, |w| w.modify_stat(target_id, &stat, amount));
            Ok(())
        });

        methods.add_method("set_stat", |lua, this, (target, stat, value): (Value, String, i64)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            with_world(lua, |w| w.set_stat(target_id, &stat, value));
            Ok(())
        });

        methods.add_method("get_stat", |lua, this, (target, stat): (Value, String)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            let val = with_world(lua, |w| w.get_stat(target_id, &stat));
            Ok(val)
        });

        methods.add_method("spawn", |lua, this, (entity_type, opts): (String, Option<mlua::Table>)| {
            let offset = opts.as_ref()
                .and_then(|t| t.get::<i64>("offset").ok())
                .unwrap_or(0);
            let id = with_world(lua, |w| w.spawn(this.caster_id, &entity_type, offset));
            Ok(id.0 as i64)
        });

        methods.add_method("animate", |lua, this, (target, animation): (Value, String)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            with_world(lua, |w| w.animate(target_id, &animation));
            Ok(())
        });

        methods.add_method("move_to", |lua, this, position: i64| {
            with_world(lua, |w| w.move_to(this.caster_id, position));
            Ok(())
        });

        methods.add_method("move_by", |lua, this, offset: i64| {
            with_world(lua, |w| w.move_by(this.caster_id, offset));
            Ok(())
        });

        methods.add_method("face_to", |lua, this, target: Value| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            with_world(lua, |w| w.face_to(this.caster_id, target_id));
            Ok(())
        });

        methods.add_method("apply_buff", |lua, this, (target, buff, opts): (Value, String, Option<mlua::Table>)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            let duration = opts.as_ref().and_then(|t| t.get::<i64>("duration").ok());
            with_world(lua, |w| w.apply_buff(target_id, &buff, duration));
            Ok(())
        });

        methods.add_method("remove_buff", |lua, this, (target, buff): (Value, String)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            with_world(lua, |w| w.remove_buff(target_id, &buff));
            Ok(())
        });

        // --- Resource operations ---

        methods.add_method("use_resource", |lua, _this, (resource, amount): (String, i64)| {
            let ok = with_world(lua, |w| w.try_spend_resource(&resource, amount));
            Ok(ok)
        });

        methods.add_method("modify_resource", |lua, _this, (resource, amount): (String, i64)| {
            with_world(lua, |w| { w.gain_resource(&resource, amount); });
            Ok(())
        });

        methods.add_method("get_resource", |lua, _this, resource: String| {
            let val = with_world(lua, |w| w.get_resource(&resource));
            Ok(val)
        });

        // --- Output ---

        methods.add_method("output", |lua, this, message: String| {
            push_event(lua, SimEvent::ScriptOutput {
                entity_id: this.caster_id,
                text: message,
            });
            Ok(())
        });

        methods.add_method("list_commands", |lua, this, ()| {
            with_world(lua, |w| w.list_commands(this.caster_id));
            Ok(())
        });

        // --- Queries ---

        methods.add_method("entity_count", |lua, _this, type_name: String| {
            let count = with_world(lua, |w| w.entity_count(&type_name));
            Ok(count)
        });

        methods.add_method("is_alive", |lua, this, target: Value| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            let alive = with_world(lua, |w| w.is_alive(target_id));
            Ok(alive)
        });

        methods.add_method("distance", |lua, this, (a, b): (Value, Value)| {
            let a_id = resolve_target_val(&a, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target a"))?;
            let b_id = resolve_target_val(&b, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target b"))?;
            let dist = with_world(lua, |w| w.distance(a_id, b_id));
            Ok(dist)
        });

        methods.add_method("has_buff", |lua, this, (target, buff): (Value, String)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            let has = with_world(lua, |w| w.has_buff(target_id, &buff));
            Ok(has)
        });

        methods.add_method("has_type", |lua, this, (target, type_name): (Value, String)| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            let has = with_world(lua, |w| w.has_type(target_id, &type_name));
            Ok(has)
        });

        methods.add_method("position", |lua, this, target: Value| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            let pos = with_world(lua, |w| w.position(target_id));
            Ok(pos)
        });

        methods.add_method("owner", |lua, this, target: Value| {
            let target_id = resolve_target_val(&target, this.caster_id)
                .ok_or_else(|| mlua::Error::runtime("invalid target"))?;
            let owner = with_world(lua, |w| w.owner(target_id));
            match owner {
                Some(oid) => Ok(Value::Integer(oid.0 as i64)),
                None => Ok(Value::Nil),
            }
        });

        methods.add_method("entities_of_type", |lua, _this, type_name: String| {
            let ids = with_world(lua, |w| w.entities_of_type(&type_name));
            let table = lua.create_table()?;
            for (i, id) in ids.iter().enumerate() {
                table.set(i + 1, id.0 as i64)?;
            }
            Ok(Value::Table(table))
        });

        // --- Timing / Coroutines ---

        // yield_ticks and wait are handled via Lua wrappers (see setup_mod_tables)
        // since mlua doesn't support yielding from Rust callbacks directly.
        // The Rust method sets yield info in app_data, then the Lua wrapper
        // calls coroutine.yield().

        methods.add_method("_set_yield_info", |lua, _this, (n, interruptible): (i64, bool)| {
            lua.set_app_data(YieldInfo { ticks: n, interruptible });
            Ok(())
        });

        // yield_ticks and wait are Lua-side wrappers defined in setup_mod_tables.
        // They call ctx:_set_yield_info() then coroutine.yield().

        // --- RNG (deterministic) ---

        methods.add_method_mut("rand", |_lua, this, (min, max): (i64, i64)| {
            if min >= max { return Ok(min); }
            let mut rng = this.rng();
            let range = (max - min + 1) as u64;
            Ok(min + rng.next_bounded(range) as i64)
        });

        methods.add_method_mut("random_chance", |_lua, this, percent: i64| {
            let mut rng = this.rng();
            let roll = rng.next_bounded(100) as i64;
            Ok(roll < percent)
        });

        // --- Context fields ---

        methods.add_method("get_caster", |_lua, this, ()| {
            Ok(this.caster_id.0 as i64)
        });

        methods.add_method("get_args", |lua, this, ()| {
            let table = lua.create_table()?;
            for (i, arg) in this.args.iter().enumerate() {
                table.set(i + 1, convert::sim_to_lua(lua, arg)?)?;
            }
            Ok(Value::Table(table))
        });

        methods.add_method("get_tick", |lua, _this, ()| {
            let tick = with_world(lua, |w| w.tick());
            Ok(tick as i64)
        });

        // --- Resource availability ---

        methods.add_method("set_available_resources", |lua, _this, names: mlua::Table| {
            let mut res_names = Vec::new();
            for name in names.sequence_values::<String>().flatten() {
                res_names.push(name);
            }
            with_world(lua, |w| w.set_available_resources(&res_names));
            Ok(())
        });
    }
}

// ---------------------------------------------------------------------------
// Mod registration API
// ---------------------------------------------------------------------------

/// Set up per-mod registry tables and the `void` module.
pub fn setup_mod_tables(lua: &Lua, mod_id: &str) -> Result<(), LuaModError> {
    // Create registry keys for this mod's commands, triggers, buffs, init handlers.
    let cmd_list_key = format!("__mod_{mod_id}_commands");
    let _init_key = format!("__mod_{mod_id}_init");
    let trigger_key = format!("__mod_{mod_id}_triggers");
    let buff_key = format!("__mod_{mod_id}_buffs");
    let meta_key = format!("__mod_{mod_id}_meta");

    // Initialize empty tables.
    lua.globals().set(cmd_list_key.as_str(), lua.create_table()?)?;
    lua.globals().set(trigger_key.as_str(), lua.create_table()?)?;
    lua.globals().set(buff_key.as_str(), lua.create_table()?)?;
    lua.globals().set(meta_key.as_str(), lua.create_table()?)?;
    // init is nil by default (no init handler)

    // Create the `void` module table that mod.lua gets via `require("void")`.
    // We just set it as a global `mod` since Lua 5.4 `require` is sandboxed out.
    let void_mod = lua.create_table()?;
    let mod_id_str = mod_id.to_string();

    // mod.command(name, [opts], handler)
    let mid = mod_id_str.clone();
    let command_fn = lua.create_function(move |lua, args: mlua::MultiValue| {
        let mut args_iter = args.into_iter();
        let name: String = match args_iter.next() {
            Some(Value::String(s)) => s.to_str()?.to_string(),
            _ => return Err(mlua::Error::runtime("mod.command: first arg must be a string (command name)")),
        };

        // Parse optional opts table and handler function.
        let (opts, handler) = match (args_iter.next(), args_iter.next()) {
            (Some(Value::Table(t)), Some(Value::Function(f))) => (Some(t), f),
            (Some(Value::Function(f)), _) => (None, f),
            _ => return Err(mlua::Error::runtime("mod.command: expected (name, [opts], handler)")),
        };

        // Store the handler function globally.
        let handler_key = format!("__mod_{}_cmd_{}", mid, name);
        lua.globals().set(handler_key.as_str(), handler)?;

        // Store command name in the command list.
        let cmd_list_key = format!("__mod_{}_commands", mid);
        let cmd_list: mlua::Table = lua.globals().get(cmd_list_key.as_str())?;
        let len = cmd_list.raw_len();
        cmd_list.set(len + 1, name.clone())?;

        // Store metadata.
        let meta_key = format!("__mod_{}_meta", mid);
        let meta_table: mlua::Table = lua.globals().get(meta_key.as_str())?;
        let meta = lua.create_table()?;
        if let Some(ref opts) = opts {
            if let Ok(desc) = opts.get::<String>("description") {
                meta.set("description", desc)?;
            }
            if let Ok(unlisted) = opts.get::<bool>("unlisted") {
                meta.set("unlisted", unlisted)?;
            }
            if let Ok(args_table) = opts.get::<mlua::Table>("args") {
                meta.set("args", args_table)?;
            }
            if let Ok(kind_str) = opts.get::<String>("kind") {
                meta.set("kind", kind_str)?;
            }
        }
        meta_table.set(name, meta)?;

        Ok(())
    })?;
    void_mod.set("command", command_fn)?;

    // mod.on_init(handler)
    let mid = mod_id_str.clone();
    let init_fn = lua.create_function(move |lua, handler: Function| {
        let init_key = format!("__mod_{}_init", mid);
        lua.globals().set(init_key.as_str(), handler)?;
        Ok(())
    })?;
    void_mod.set("on_init", init_fn)?;

    // mod.on(event, [opts], handler)
    let mid = mod_id_str.clone();
    let on_fn = lua.create_function(move |lua, args: mlua::MultiValue| {
        let mut args_iter = args.into_iter();
        let event: String = match args_iter.next() {
            Some(Value::String(s)) => s.to_str()?.to_string(),
            _ => return Err(mlua::Error::runtime("mod.on: first arg must be event name")),
        };

        let (opts, handler) = match (args_iter.next(), args_iter.next()) {
            (Some(Value::Table(t)), Some(Value::Function(f))) => (Some(t), f),
            (Some(Value::Function(f)), _) => (None, f),
            _ => return Err(mlua::Error::runtime("mod.on: expected (event, [opts], handler)")),
        };

        let trigger_key = format!("__mod_{}_triggers", mid);
        let triggers: mlua::Table = lua.globals().get(trigger_key.as_str())?;
        let entry = lua.create_table()?;
        entry.set("event", event)?;
        entry.set("handler", handler)?;
        if let Some(opts) = opts {
            entry.set("opts", opts)?;
        }
        let len = triggers.raw_len();
        triggers.set(len + 1, entry)?;

        Ok(())
    })?;
    void_mod.set("on", on_fn)?;

    // mod.buff(name, callbacks)
    let mid = mod_id_str.clone();
    let buff_fn = lua.create_function(move |lua, (name, callbacks): (String, mlua::Table)| {
        let buff_key = format!("__mod_{}_buffs", mid);
        let buffs: mlua::Table = lua.globals().get(buff_key.as_str())?;
        buffs.set(name, callbacks)?;
        Ok(())
    })?;
    void_mod.set("buff", buff_fn)?;

    // Set as global `mod` — mod.lua scripts use `local mod = require("void")`
    // but since require is sandboxed, we provide it directly.
    lua.globals().set("mod", void_mod.clone())?;

    // Also set up a `require` shim that returns our void module.
    let require_fn = lua.create_function(move |_lua, name: String| {
        if name == "void" {
            Ok(Value::Table(void_mod.clone()))
        } else {
            Err(mlua::Error::runtime(format!("module '{name}' not found (sandboxed)")))
        }
    })?;
    lua.globals().set("require", require_fn)?;

    // Set up the command wrapper that provides yield_ticks/wait on the ctx proxy.
    // The wrapper creates a proxy table around ctx that intercepts yield_ticks/wait
    // to call coroutine.yield() from the Lua side (can't yield from Rust callbacks).
    //
    // We build a method lookup table from ctx's known methods, then overlay
    // yield_ticks/wait/caster/tick on top.
    lua.load(r#"
        local _coroutine_yield = coroutine.yield

        function __void_cmd_wrapper(handler, ctx)
            -- Add yield_ticks and wait as methods on a proxy that delegates
            -- all other calls to the real ctx userdata.
            local proxy = setmetatable({}, {
                __index = function(_, key)
                    if key == "yield_ticks" then
                        return function(self, n, opts)
                            local interruptible = false
                            if opts and opts.interruptible then
                                interruptible = true
                            end
                            ctx:_set_yield_info(n, interruptible)
                            _coroutine_yield()
                        end
                    elseif key == "wait" then
                        return function(self)
                            ctx:_set_yield_info(1, false)
                            _coroutine_yield()
                        end
                    elseif key == "caster" then
                        return ctx:get_caster()
                    elseif key == "tick" then
                        return ctx:get_tick()
                    elseif key == "args" then
                        return ctx:get_args()
                    else
                        -- Delegate to the userdata method
                        return function(self, ...)
                            return ctx[key](ctx, ...)
                        end
                    end
                end
            })
            handler(proxy)
        end

        function __void_query_wrapper(handler, ctx)
            local proxy = setmetatable({}, {
                __index = function(_, key)
                    if key == "yield_ticks" or key == "wait" then
                        return function()
                            error("query commands cannot yield")
                        end
                    elseif key == "caster" then
                        return ctx:get_caster()
                    elseif key == "tick" then
                        return ctx:get_tick()
                    elseif key == "args" then
                        return ctx:get_args()
                    else
                        return function(self, ...)
                            return ctx[key](ctx, ...)
                        end
                    end
                end
            })
            return handler(proxy)
        end
    "#).exec()?;

    Ok(())
}

/// Get the list of registered command names for a mod.
pub fn get_registered_commands(lua: &Lua, mod_id: &str) -> Result<Vec<String>, LuaModError> {
    let cmd_list_key = format!("__mod_{mod_id}_commands");
    let cmd_list: mlua::Table = lua.globals().get(cmd_list_key.as_str())?;
    let mut commands = Vec::new();
    for i in 1..=cmd_list.raw_len() {
        if let Ok(name) = cmd_list.get::<String>(i) {
            commands.push(name);
        }
    }
    Ok(commands)
}

/// Collect command metadata from all registered mods.
pub fn collect_command_metadata(lua: &Lua) -> Vec<(String, CommandMeta)> {
    let mut result = Vec::new();

    // Iterate through all globals looking for __mod_*_meta keys.
    if let Ok(globals) = lua.globals().pairs::<String, Value>().collect::<Result<Vec<_>, _>>() {
        for (key, value) in &globals {
            if key.starts_with("__mod_") && key.ends_with("_meta")
                && let Value::Table(meta_table) = value
                    && let Ok(pairs) = meta_table.pairs::<String, mlua::Table>().collect::<Result<Vec<_>, _>>() {
                        for (cmd_name, meta) in pairs {
                            let description = meta.get::<String>("description").unwrap_or_default();
                            let unlisted = meta.get::<bool>("unlisted").unwrap_or(false);
                            let args: Vec<String> = meta.get::<mlua::Table>("args")
                                .ok()
                                .map(|t| {
                                    let mut v = Vec::new();
                                    for i in 1..=t.raw_len() {
                                        if let Ok(s) = t.get::<String>(i) { v.push(s); }
                                    }
                                    v
                                })
                                .unwrap_or_default();
                            let kind = match meta.get::<String>("kind").unwrap_or_default().as_str() {
                                "query" => deadcode_sim::CommandKind::Query,
                                _ => deadcode_sim::CommandKind::Custom,
                            };
                            result.push((cmd_name, CommandMeta { description, args, unlisted, kind }));
                        }
                    }
        }
    }

    result
}

/// Clear all registration tables for a mod (for hot-reload).
pub fn clear_mod_tables(lua: &Lua, mod_id: &str) {
    let keys = [
        format!("__mod_{mod_id}_commands"),
        format!("__mod_{mod_id}_init"),
        format!("__mod_{mod_id}_triggers"),
        format!("__mod_{mod_id}_buffs"),
        format!("__mod_{mod_id}_meta"),
    ];
    for key in &keys {
        let _ = lua.globals().set(key.as_str(), Value::Nil);
    }
    // Also remove individual command handler globals.
    if let Ok(globals) = lua.globals().pairs::<String, Value>().collect::<Result<Vec<_>, _>>() {
        let prefix = format!("__mod_{mod_id}_cmd_");
        for (key, _) in globals {
            if key.starts_with(&prefix) {
                let _ = lua.globals().set(key.as_str(), Value::Nil);
            }
        }
    }
}

/// Run all registered init handlers.
pub fn run_init_handlers(lua: &Lua, world: &mut WorldAccess) -> Vec<SimEvent> {
    let mut all_events = Vec::new();

    // Find all init handlers.
    let init_keys: Vec<String> = lua.globals()
        .pairs::<String, Value>()
        .flatten()
        .filter(|(k, v)| k.starts_with("__mod_") && k.ends_with("_init") && matches!(v, Value::Function(_)))
        .map(|(k, _)| k)
        .collect();

    for key in init_keys {
        if let Ok(handler) = lua.globals().get::<Function>(key.as_str()) {
            // Create a ctx for the init handler (no specific caster).
            let caster_id = world.caster_id;
            let ctx = CtxUserData::new(caster_id, world, &[]);
            match lua.create_userdata(ctx) {
                Ok(ud) => {
                    // Set up world access.
                    let world_ptr = world as *mut WorldAccess as usize;
                    lua.set_app_data(WorldAccessPtr(world_ptr));
                    lua.set_app_data(CasterEntity(caster_id));

                    let _ = handler.call::<()>(Value::UserData(ud));

                    if let Some(collected) = lua.remove_app_data::<CollectedEvents>() {
                        all_events.extend(collected.0);
                    }
                    lua.remove_app_data::<WorldAccessPtr>();
                    lua.remove_app_data::<CasterEntity>();
                }
                Err(e) => {
                    eprintln!("[lua] init handler error: {e}");
                }
            }
        }
    }

    all_events
}
