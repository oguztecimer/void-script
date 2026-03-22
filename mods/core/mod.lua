local mod = require("void")

-- Initialization (replaces [initial].effects)
mod.on_init(function(ctx)
  ctx:set_available_resources({"mana", "bones"})
  ctx:spawn("summoner", { offset = 0 })
  ctx:output("The dead stir beneath your feet")
  ctx:output("Call for <hl>help()</hl> to hear them speak")
end)

-- Commands

mod.command("help", { description = "Hidden wisdom will be found", unlisted = true }, function(ctx)
  ctx:output(" ")
  ctx:list_commands()
end)

mod.command("trance", { description = "Deep in trance, what's lost is found" }, function(ctx)
  ctx:output("[trance] You feel the mana flowing in.")
  for i = 1, 200 do
    ctx:yield_ticks(5, { interruptible = true })
    ctx:modify_resource("mana", 1)
  end
end)

mod.command("raise", { description = "From the ground, the dead are bound" }, function(ctx)
  ctx:animate("self", "cast")
  ctx:yield_ticks(12, { interruptible = true })

  if not ctx:use_resource("mana", 20) then return end
  ctx:yield_ticks(1)

  ctx:spawn("skeleton", { offset = ctx:rand(-300, 300) })
  ctx:yield_ticks(18)
end)

mod.command("harvest", { description = "The children slain, bones remain" }, function(ctx)
  ctx:animate("self", "cast")
  ctx:yield_ticks(12, { interruptible = true })

  if not ctx:use_resource("mana", 50) then return end
  ctx:yield_ticks(1)

  ctx:yield_ticks(1)
  ctx:yield_ticks(18)
end)

mod.command("pact", { description = "Pledge your bones to my domain" }, function(ctx)
  ctx:modify_stat("self", "health", -10)
  ctx:output("[pact] Power surges through you...")
end)

mod.command("walk_left", { description = "walk left", }, function(ctx)
  ctx:move_by(-1)
end)

mod.command("walk_right", { description = "walk right" }, function(ctx)
  ctx:move_by(1)
end)

mod.command("get_entities", {
  kind = "query",
  args = { "type_name" },
  description = "Count entities of a type"
}, function(ctx)
  return ctx:entities_of_type(ctx.args[1])
end)

mod.command("tick", { kind = "query", description = "Current simulation tick" }, function(ctx)
  return ctx.tick
end)

mod.command("set_var", { kind = "query", args = { "entity", "key", "value" } }, function(ctx)
  ctx:set_stat(ctx.args[1], ctx.args[2], ctx.args[3])
end)

-- Triggers

mod.on("entity_died", { filter = { entity_type = "skeleton" } }, function(ctx, event)
  ctx:modify_resource("bones", 1)
end)
