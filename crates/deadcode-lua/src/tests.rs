//! Tests for the Lua mod runtime.

#[cfg(test)]
mod tests {
    use deadcode_sim::action::CommandHandler;
    use deadcode_sim::entity::EntityId;
    use deadcode_sim::world::{SimEvent, SimWorld, WorldAccess};

    use crate::LuaModRuntime;

    fn setup_world() -> SimWorld {
        let mut world = SimWorld::new(42);
        world.start();
        // Spawn a test entity.
        let eid = world.spawn_entity("summoner".into(), "summoner_1".into(), 0);
        // Give it some stats.
        if let Some(e) = world.get_entity_mut(eid) {
            e.set_stat("health", 100);
            e.set_stat("max_health", 100);
        }
        // Set up resources.
        world.resources.insert("mana".into(), 50);
        world.resource_caps.insert("mana".into(), 100);
        world.resources.insert("bones".into(), 0);
        world
    }

    #[test]
    fn test_instant_command() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("hello", { description = "Say hello" }, function(ctx)
                ctx:output("Hello, world!")
            end)
        "#).unwrap();

        assert!(runtime.has_command("hello"));

        let mut world = setup_world();
        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "hello", &[]);

        match result {
            deadcode_sim::action::CommandHandlerResult::Completed { events } => {
                let all_events: Vec<_> = access.events.into_iter().chain(events.into_iter()).collect();
                let outputs: Vec<_> = all_events.iter()
                    .filter_map(|e| match e {
                        SimEvent::ScriptOutput { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert!(outputs.contains(&"Hello, world!"), "Expected hello output, got: {:?}", outputs);
            }
            other => panic!("Expected Completed, got: {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_not_handled() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("foo", {}, function(ctx)
                ctx:output("foo!")
            end)
        "#).unwrap();

        let mut world = setup_world();
        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "nonexistent", &[]);

        match result {
            deadcode_sim::action::CommandHandlerResult::NotHandled => {}
            _ => panic!("Expected NotHandled for unregistered command"),
        }
    }

    #[test]
    fn test_yield_ticks() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("cast", {}, function(ctx)
                ctx:output("start")
                ctx:yield_ticks(5)
                ctx:output("done")
            end)
        "#).unwrap();

        let mut world = setup_world();
        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "cast", &[]);

        match result {
            deadcode_sim::action::CommandHandlerResult::Yielded { events, handle, remaining_ticks, interruptible } => {
                let all_events: Vec<_> = access.events.into_iter().chain(events.into_iter()).collect();
                let outputs: Vec<_> = all_events.iter()
                    .filter_map(|e| match e {
                        SimEvent::ScriptOutput { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert!(outputs.contains(&"start"), "Expected 'start' output");
                assert!(!outputs.contains(&"done"), "Should not have 'done' yet");
                assert_eq!(remaining_ticks, 5);
                assert!(!interruptible);

                // Resume after ticks expire.
                let mut access2 = WorldAccess::new_from_world_ptr(&mut world, eid);
                let result2 = runtime.resume_coroutine(&mut access2, eid, handle);
                match result2 {
                    deadcode_sim::action::CommandHandlerResult::Completed { events } => {
                        let all_events: Vec<_> = access2.events.into_iter().chain(events.into_iter()).collect();
                        let outputs: Vec<_> = all_events.iter()
                            .filter_map(|e| match e {
                                SimEvent::ScriptOutput { text, .. } => Some(text.as_str()),
                                _ => None,
                            })
                            .collect();
                        assert!(outputs.contains(&"done"), "Expected 'done' output after resume");
                    }
                    _ => panic!("Expected Completed after resume"),
                }
            }
            _ => panic!("Expected Yielded"),
        }
    }

    #[test]
    fn test_use_resource() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("spend", {}, function(ctx)
                if ctx:use_resource("mana", 30) then
                    ctx:output("spent 30 mana")
                else
                    ctx:output("not enough mana")
                end
            end)
        "#).unwrap();

        let mut world = setup_world();
        let eid = EntityId(1);

        // Should succeed (50 mana available).
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "spend", &[]);
        match result {
            deadcode_sim::action::CommandHandlerResult::Completed { events } => {
                let all_events: Vec<_> = access.events.into_iter().chain(events.into_iter()).collect();
                let outputs: Vec<_> = all_events.iter()
                    .filter_map(|e| match e {
                        SimEvent::ScriptOutput { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert!(outputs.contains(&"spent 30 mana"));
            }
            _ => panic!("Expected Completed"),
        }
        assert_eq!(world.get_resource("mana"), 20); // 50 - 30 = 20

        // Should fail (only 20 left, trying to spend 30).
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "spend", &[]);
        match result {
            deadcode_sim::action::CommandHandlerResult::Completed { events } => {
                let all_events: Vec<_> = access.events.into_iter().chain(events.into_iter()).collect();
                let outputs: Vec<_> = all_events.iter()
                    .filter_map(|e| match e {
                        SimEvent::ScriptOutput { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert!(outputs.contains(&"not enough mana"));
            }
            _ => panic!("Expected Completed"),
        }
        assert_eq!(world.get_resource("mana"), 20); // unchanged
    }

    #[test]
    fn test_interruptible_yield() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("channel", {}, function(ctx)
                ctx:output("channeling")
                ctx:yield_ticks(10, { interruptible = true })
                ctx:output("finished")
            end)
        "#).unwrap();

        let mut world = setup_world();
        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "channel", &[]);

        match result {
            deadcode_sim::action::CommandHandlerResult::Yielded { handle, interruptible, .. } => {
                assert!(interruptible);
                // Cancel the coroutine (simulate interruption).
                runtime.cancel_coroutine(handle);
            }
            _ => panic!("Expected Yielded"),
        }
    }

    #[test]
    fn test_spawn_entity() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("summon", {}, function(ctx)
                ctx:spawn("skeleton", { offset = 100 })
                ctx:output("summoned")
            end)
        "#).unwrap();

        let mut world = setup_world();
        // Register skeleton entity type.
        world.entity_types_registry.insert("skeleton".into(), vec!["unit".into(), "skeleton".into()]);

        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "summon", &[]);

        match result {
            deadcode_sim::action::CommandHandlerResult::Completed { .. } => {
                // Flush pending spawns.
                world.flush_pending();
                // Count entities — original + newly spawned.
                let entity_count = world.entities().count();
                assert!(entity_count >= 2, "Expected at least 2 entities after spawn, got {entity_count}");
            }
            _ => panic!("Expected Completed"),
        }
    }

    #[test]
    fn test_command_metadata() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("test_cmd", { description = "A test command", unlisted = true }, function(ctx)
            end)
            mod.command("visible_cmd", { description = "A visible command" }, function(ctx)
            end)
        "#).unwrap();

        let meta = runtime.command_metadata();
        let test_meta = meta.iter().find(|(n, _)| n == "test_cmd");
        assert!(test_meta.is_some(), "Expected test_cmd metadata");
        let (_, m) = test_meta.unwrap();
        assert_eq!(m.description, "A test command");
        assert!(m.unlisted);

        let visible_meta = meta.iter().find(|(n, _)| n == "visible_cmd");
        assert!(visible_meta.is_some());
        let (_, m) = visible_meta.unwrap();
        assert!(!m.unlisted);
    }

    #[test]
    fn test_hot_reload() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("v1", {}, function(ctx)
                ctx:output("version 1")
            end)
        "#).unwrap();

        assert!(runtime.has_command("v1"));
        assert!(!runtime.has_command("v2"));

        // Hot-reload with new source.
        runtime.reload_mod("test", r#"
            local mod = require("void")
            mod.command("v2", {}, function(ctx)
                ctx:output("version 2")
            end)
        "#).unwrap();

        assert!(!runtime.has_command("v1"), "v1 should be unregistered after reload");
        assert!(runtime.has_command("v2"), "v2 should be registered after reload");
    }

    #[test]
    fn test_modify_resource() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("gain", {}, function(ctx)
                ctx:modify_resource("mana", 10)
                local mana = ctx:get_resource("mana")
                ctx:output("mana=" .. tostring(mana))
            end)
        "#).unwrap();

        let mut world = setup_world();
        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "gain", &[]);

        match result {
            deadcode_sim::action::CommandHandlerResult::Completed { events } => {
                let all_events: Vec<_> = access.events.into_iter().chain(events.into_iter()).collect();
                let outputs: Vec<_> = all_events.iter()
                    .filter_map(|e| match e {
                        SimEvent::ScriptOutput { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert!(outputs.contains(&"mana=60"), "Expected mana=60, got: {:?}", outputs);
            }
            _ => panic!("Expected Completed"),
        }
    }

    #[test]
    fn test_rand_deterministic() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.command("roll", {}, function(ctx)
                local r = ctx:rand(1, 100)
                ctx:output("roll=" .. tostring(r))
            end)
        "#).unwrap();

        let mut world = setup_world();
        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let result = runtime.resolve_command(&mut access, eid, "roll", &[]);

        match result {
            deadcode_sim::action::CommandHandlerResult::Completed { events } => {
                let all_events: Vec<_> = access.events.into_iter().chain(events.into_iter()).collect();
                let found = all_events.iter().any(|e| matches!(e, SimEvent::ScriptOutput { text, .. } if text.starts_with("roll=")));
                assert!(found, "Expected roll output");
            }
            _ => panic!("Expected Completed"),
        }
    }

    #[test]
    fn test_on_init() {
        let mut runtime = LuaModRuntime::new().unwrap();
        runtime.load_mod_source("test", r#"
            local mod = require("void")
            mod.on_init(function(ctx)
                ctx:output("init ran!")
            end)
        "#).unwrap();

        let mut world = setup_world();
        let eid = EntityId(1);
        let mut access = WorldAccess::new_from_world_ptr(&mut world, eid);
        let events = runtime.run_init(&mut access);

        let all_events: Vec<_> = access.events.into_iter().chain(events.into_iter()).collect();
        let outputs: Vec<_> = all_events.iter()
            .filter_map(|e| match e {
                SimEvent::ScriptOutput { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(outputs.contains(&"init ran!"), "Expected init output, got: {:?}", outputs);
    }
}
