use std::collections::HashSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::action::{BuffDef, CommandDef, UnitAction, resolve_action, reverse_buff_modifiers};
use crate::entity::{EntityConfig, EntityId, SimEntity};
use crate::executor;
use crate::rng::SimRng;
use crate::value::SimValue;

/// Default width of the 1D world strip in logical units.
pub const DEFAULT_WORLD_WIDTH: i64 = 1000;

/// Events emitted during a tick — consumed by rendering/UI layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimEvent {
    EntityMoved {
        entity_id: EntityId,
        new_position: i64,
    },
    EntityDamaged {
        entity_id: EntityId,
        damage: i64,
        new_health: i64,
        /// The entity that dealt the damage (if known).
        attacker_id: Option<EntityId>,
    },
    EntityDied {
        entity_id: EntityId,
        name: String,
        /// The entity that dealt the killing blow (if known).
        killer_id: Option<EntityId>,
        /// The owner of the dead entity (captured at death time since entity is removed before triggers fire).
        owner_id: Option<EntityId>,
    },
    EntitySpawned {
        entity_id: EntityId,
        entity_type: String,
        name: String,
        position: i64,
        /// The entity that spawned this one (if known).
        spawner_id: Option<EntityId>,
    },
    ScriptOutput {
        entity_id: EntityId,
        text: String,
    },
    ScriptError {
        entity_id: EntityId,
        error: String,
        /// Variable state at the time of error (name, value as string).
        variables: Vec<(String, String)>,
        /// Stack state at the time of error (values as strings).
        stack: Vec<String>,
        /// Program counter at the time of error.
        pc: usize,
    },
    ScriptFinished {
        entity_id: EntityId,
        success: bool,
        error: Option<String>,
    },
    PlayAnimation {
        entity_id: EntityId,
        animation: String,
    },
    /// A custom command was used by an entity (for triggers).
    CommandUsed {
        entity_id: EntityId,
        command: String,
    },
    /// A phased channel completed all phases (for triggers).
    ChannelCompleted {
        entity_id: EntityId,
        command: String,
    },
    /// A phased channel was interrupted (for triggers).
    ChannelInterrupted {
        entity_id: EntityId,
        command: String,
    },
    /// An entity's facing direction was changed (consumed by render layer).
    EntityFlipped {
        entity_id: EntityId,
        facing_left: bool,
    },
}

/// A snapshot of the world state for rendering/UI sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimSnapshot {
    pub tick: u64,
    pub entities: Vec<EntitySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub entity_type: String,
    pub types: Vec<String>,
    pub name: String,
    pub position: i64,
    pub health: i64,
    pub max_health: i64,
    pub alive: bool,
}

/// The simulation world — all game state lives here.
pub struct SimWorld {
    pub tick: u64,
    seed: u64,
    entities: Vec<SimEntity>,
    entity_index: IndexMap<EntityId, usize>,
    next_entity_id: u64,
    pending_spawns: Vec<SimEntity>,
    pending_despawns: Vec<EntityId>,
    events: Vec<SimEvent>,
    running: bool,
    /// Custom command name → arg count (for the executor to know how many args to pop).
    pub custom_command_arg_counts: IndexMap<String, usize>,
    /// Custom command name → description (for list_commands effect).
    pub custom_command_descriptions: IndexMap<String, String>,
    /// Entity type → stat overrides (for spawning from effects).
    pub entity_configs: IndexMap<String, EntityConfig>,
    /// Entity def ID → resolved type tags (for spawning from effects).
    pub entity_types_registry: IndexMap<String, Vec<String>>,
    /// Entity type → spawn animation duration in ticks (0 = no spawn animation).
    pub spawn_durations: IndexMap<String, i64>,
    /// Command display order (from available_commands insertion order).
    pub command_order: Vec<String>,
    /// Global resources shared across all entities.
    pub resources: IndexMap<String, i64>,
    /// Optional max values for resources. Absent = capless.
    pub resource_caps: IndexMap<String, i64>,
    /// Available resource names. None = all available (dev mode).
    pub available_resources: Option<HashSet<String>>,
    /// Commands hidden from `list_commands` output.
    pub unlisted_commands: HashSet<String>,
    /// Buff name → buff definition registry.
    pub buff_registry: IndexMap<String, BuffDef>,
    /// Main brain script state — runs first each tick, backed by a real entity.
    pub main_brain: Option<crate::entity::ScriptState>,
    /// Entity ID for the main brain entity.
    pub main_brain_entity: Option<EntityId>,
    /// External command handler (Lua runtime). Custom commands are dispatched here.
    pub command_handler: Option<Box<dyn crate::action::CommandHandler>>,
    /// Width of the 1D world strip in logical units.
    pub world_width: i64,
}

impl SimWorld {
    pub fn new(seed: u64) -> Self {
        Self {
            tick: 0,
            seed,
            entities: Vec::new(),
            entity_index: IndexMap::new(),
            next_entity_id: 1,
            pending_spawns: Vec::new(),
            pending_despawns: Vec::new(),
            events: Vec::new(),
            running: false,
            custom_command_arg_counts: IndexMap::new(),
            custom_command_descriptions: IndexMap::new(),
            entity_configs: IndexMap::new(),
            entity_types_registry: IndexMap::new(),
            spawn_durations: IndexMap::new(),
            command_order: Vec::new(),
            resources: IndexMap::new(),
            resource_caps: IndexMap::new(),
            available_resources: None,
            unlisted_commands: HashSet::new(),
            buff_registry: IndexMap::new(),
            main_brain: None,
            main_brain_entity: None,
            command_handler: None,
            world_width: DEFAULT_WORLD_WIDTH,
        }
    }

    /// Check if a resource is available for use. Returns Err if not available.
    pub fn check_resource_available(&self, name: &str) -> Result<(), crate::error::SimError> {
        if let Some(ref set) = self.available_resources {
            if !set.contains(name) {
                return Err(crate::error::SimError::new(
                    crate::error::SimErrorKind::Runtime,
                    format!("resource '{}' is not available yet", name),
                ));
            }
        }
        Ok(())
    }

    /// Get a global resource value (0 if not defined).
    pub fn get_resource(&self, name: &str) -> i64 {
        self.resources.get(name).copied().unwrap_or(0)
    }

    /// Add to a global resource, returning the new total. Clamped to [0, cap].
    pub fn gain_resource(&mut self, name: &str, amount: i64) -> i64 {
        let entry = self.resources.entry(name.to_string()).or_insert(0);
        *entry += amount;
        if let Some(&cap) = self.resource_caps.get(name) {
            *entry = (*entry).min(cap);
        }
        *entry = (*entry).max(0);
        *entry
    }

    /// Get the max value of a resource, if capped.
    pub fn get_resource_cap(&self, name: &str) -> Option<i64> {
        self.resource_caps.get(name).copied()
    }

    /// Try to spend a global resource. Returns true if successful, false if insufficient.
    pub fn try_spend_resource(&mut self, name: &str, amount: i64) -> bool {
        let entry = self.resources.entry(name.to_string()).or_insert(0);
        if *entry >= amount {
            *entry -= amount;
            true
        } else {
            false
        }
    }

    /// Register a custom command's metadata (description, arg count, unlisted, command_order).
    pub fn register_custom_command(&mut self, def: &CommandDef) {
        if matches!(def.kind, crate::action::CommandKind::Custom | crate::action::CommandKind::Query) {
            self.custom_command_arg_counts.insert(def.name.clone(), def.args.len());
        }
        if !def.description.is_empty() {
            self.custom_command_descriptions.insert(def.name.clone(), def.description.clone());
        }
        if def.unlisted {
            self.unlisted_commands.insert(def.name.clone());
        }
    }

    /// Register a buff definition.
    pub fn register_buff(&mut self, def: BuffDef) {
        self.buff_registry.insert(def.name.clone(), def);
    }

    /// Get the next entity ID (for pre-allocating IDs in effect resolution).
    pub fn next_entity_id(&mut self) -> u64 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    /// Get the seed for the current tick (for deterministic RNG in effect resolution).
    pub fn tick_seed(&self) -> u64 {
        self.seed ^ self.tick
    }

    /// Spawn the main brain entity and return its ID.
    /// The main brain is a real entity (so it can hold channel state, buffs, etc.).
    /// Stats and types come from the "main" entity config/types if defined in mods.
    pub fn spawn_main_brain_entity(&mut self) -> EntityId {
        let config = self.entity_configs.get("main").cloned();
        let types = self.entity_types_registry.get("main").cloned()
            .unwrap_or_else(|| vec!["main".to_string()]);
        let eid = self.spawn_entity_with_types(
            "main".into(),
            types,
            "main".into(),
            500,
            config.as_ref(),
        );
        self.main_brain_entity = Some(eid);
        eid
    }

    /// Get the main brain entity ID (if spawned).
    pub fn main_brain_id(&self) -> Option<EntityId> {
        self.main_brain_entity
    }

    /// Start the simulation.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop the simulation.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Pause/unpause.
    pub fn set_paused(&mut self, paused: bool) {
        self.running = !paused;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Spawn an entity and return its ID.
    pub fn spawn_entity(
        &mut self,
        entity_type: String,
        name: String,
        position: i64,
    ) -> EntityId {
        self.spawn_entity_with_config(entity_type, name, position, None)
    }

    /// Spawn an entity with optional stat overrides and return its ID.
    pub fn spawn_entity_with_config(
        &mut self,
        entity_type: String,
        name: String,
        position: i64,
        config: Option<&EntityConfig>,
    ) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        let mut entity = SimEntity::new(id, entity_type, name, position);
        if let Some(cfg) = config {
            entity.apply_config(cfg);
        }
        let index = self.entities.len();
        self.entities.push(entity);
        self.entity_index.insert(id, index);
        id
    }

    /// Spawn an entity with explicit type tags and optional stat overrides.
    pub fn spawn_entity_with_types(
        &mut self,
        entity_type: String,
        types: Vec<String>,
        name: String,
        position: i64,
        config: Option<&EntityConfig>,
    ) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        let mut entity = SimEntity::new_with_types(id, entity_type, types, name, position);
        if let Some(cfg) = config {
            entity.apply_config(cfg);
        }
        let index = self.entities.len();
        self.entities.push(entity);
        self.entity_index.insert(id, index);
        id
    }

    /// Queue an entity to be spawned at end of tick.
    pub fn queue_spawn(&mut self, entity: SimEntity) {
        self.pending_spawns.push(entity);
    }

    /// Flush pending spawns and despawns immediately.
    pub fn flush_pending(&mut self) {
        for entity in self.pending_spawns.drain(..) {
            let id = entity.id;
            let etype = entity.entity_type.clone();
            let ename = entity.name.clone();
            let pos = entity.position;
            let spawner = entity.owner;
            let index = self.entities.len();
            self.entities.push(entity);
            self.entity_index.insert(id, index);
            self.events.push(SimEvent::EntitySpawned {
                entity_id: id,
                entity_type: etype,
                name: ename,
                position: pos,
                spawner_id: spawner,
            });
        }
        for id in self.pending_despawns.drain(..) {
            if let Some(&idx) = self.entity_index.get(&id) {
                self.entities[idx].alive = false;
            }
        }
    }

    /// Queue an entity for removal at end of tick.
    pub fn queue_despawn(&mut self, id: EntityId) {
        self.pending_despawns.push(id);
    }

    /// Get entity by ID (read-only).
    pub fn get_entity(&self, id: EntityId) -> Option<&SimEntity> {
        self.entity_index
            .get(&id)
            .and_then(|&idx| self.entities.get(idx))
    }

    /// Get entity by ID (mutable).
    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut SimEntity> {
        self.entity_index
            .get(&id)
            .copied()
            .and_then(|idx| self.entities.get_mut(idx))
    }

    /// Iterate over all entities.
    pub fn entities(&self) -> impl Iterator<Item = &SimEntity> {
        self.entities.iter()
    }

    /// Take events from this tick (drains the event buffer).
    pub fn take_events(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    /// Build a ScriptError event with variable/stack snapshots from a ScriptState.
    fn script_error_event(entity_id: EntityId, error: &str, state: &crate::entity::ScriptState) -> SimEvent {
        let variables: Vec<(String, String)> = state.variables.iter()
            .enumerate()
            .map(|(i, v)| {
                let name = state.program.functions.iter()
                    .find(|f| f.pc == 0)
                    .map_or_else(|| format!("var_{i}"), |_| format!("var_{i}"));
                (name, format!("{v}"))
            })
            .collect();
        let stack: Vec<String> = state.stack.iter()
            .map(|v| format!("{v}"))
            .collect();
        SimEvent::ScriptError {
            entity_id,
            error: error.to_string(),
            variables,
            stack,
            pc: state.pc,
        }
    }

    /// Create a snapshot of current state for the rendering layer.
    pub fn snapshot(&self) -> SimSnapshot {
        SimSnapshot {
            tick: self.tick,
            entities: self
                .entities
                .iter()
                .filter(|e| e.alive)
                .map(|e| EntitySnapshot {
                    id: e.id,
                    entity_type: e.entity_type.clone(),
                    types: e.types.clone(),
                    name: e.name.clone(),
                    position: e.position,
                    health: e.stat("health"),
                    max_health: e.stat("max_health"),
                    alive: e.alive,
                })
                .collect(),
        }
    }

    /// Run one simulation tick.
    pub fn tick(&mut self) {
        if !self.running {
            return;
        }

        // 1. Increment tick, clear events.
        self.tick += 1;
        self.events.clear();

        // 2. Derive per-tick RNG.
        let mut rng = SimRng::new(self.seed ^ self.tick);

        // 2b. Tick spawn timers.
        for entity in &mut self.entities {
            if entity.alive && entity.spawn_ticks_remaining > 0 {
                entity.spawn_ticks_remaining -= 1;
            }
        }

        // 2c. Execute main brain (backed by a real entity, runs first before entity shuffle).
        let brain_eid = self.main_brain_entity;
        let mut brain_state = self.main_brain.take();
        if let (Some(state), Some(eid)) = (&mut brain_state, brain_eid) {
            // Process active channel on the main brain entity first.
            let has_channel = self.get_entity(eid).map_or(false, |e| e.active_channel.is_some());
            if has_channel {
                // Let channel processing happen in the normal entity loop (step 4).
                // Just skip main brain script execution this tick.
            } else if let Some(err_msg) = state.error.take() {
                // Error recovery: clear error, reset script, yield wait this tick.
                state.pc = 0;
                state.stack.clear();
                state.call_stack.clear();
                state.yielded = false;
                state.step_limit_hit = false;
                let num_vars = state.variables.len();
                state.variables = vec![SimValue::None; num_vars];
                self.events.push(SimEvent::ScriptOutput {
                    entity_id: eid,
                    text: format!("[error recovery] Previous error: {err_msg} — script restarted"),
                });
            } else if state.pc < state.program.instructions.len() {
                let mut instant_count = 0u32;
                let mut brain_restarted = false;
                loop {
                    instant_count += 1;
                    if instant_count > 1000 {
                        state.error = Some("main brain: too many instant actions".to_string());
                        self.events.push(Self::script_error_event(eid, "main brain: too many instant actions", state));
                        break;
                    }
                    match executor::execute_unit(eid, state, self) {
                        Ok(Some(action)) => {
                            match self.try_handle_instant(eid, action, state) {
                                None => {} // instant action handled, continue
                                Some(UnitAction::Wait) => break,
                                Some(real_action) => {
                                    let action_events = resolve_action(self, eid, real_action);
                                    self.events.extend(action_events);
                                    break;
                                }
                            }
                        }
                        Ok(None) => {
                            if let Some(brain_pc) = state.program.brain_entry_pc {
                                state.reset_for_brain_loop(brain_pc);
                                if !brain_restarted {
                                    brain_restarted = true;
                                    continue;
                                }
                            }
                            break;
                        }
                        Err(err) => {
                            state.error = Some(err.to_string());
                            self.events.push(Self::script_error_event(eid, &err.to_string(), state));
                            break;
                        }
                    }
                }
                if state.step_limit_hit {
                    self.events.push(SimEvent::ScriptOutput {
                        entity_id: eid,
                        text: "[warning] Main brain exceeded step limit — auto-yielded".into(),
                    });
                }
            }
        }
        self.main_brain = brain_state;

        // 3. Collect scriptable entity IDs (including channeling entities), shuffle.
        //    Skip entities that are still spawning.
        //    Exclude main brain entity from script execution (step 2c handles it),
        //    but include it if it has an active channel (phased command in progress).
        let main_eid = self.main_brain_entity;
        let mut scriptable_ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|e| e.is_ready() && (e.script_state.is_some() || e.active_channel.is_some()))
            .filter(|e| {
                if let Some(mid) = main_eid {
                    if e.id == mid {
                        // Only include main brain entity if it has an active channel.
                        return e.active_channel.is_some();
                    }
                }
                true
            })
            .map(|e| e.id)
            .collect();
        rng.shuffle(&mut scriptable_ids);

        // 4. Execute each unit's script (with channel processing).
        let mut actions: Vec<(EntityId, UnitAction)> = Vec::new();

        for &eid in &scriptable_ids {
            // --- Channel processing (Lua coroutines) ---
            let has_channel = self.get_entity(eid).map_or(false, |e| e.active_channel.is_some());

            if has_channel {
                let mut lua_ch = self.get_entity_mut(eid).unwrap().active_channel.take().unwrap();

                lua_ch.remaining_ticks -= 1;

                if lua_ch.interruptible {
                    // Check if the script wants to interrupt.
                    let mut interrupted = false;
                    let mut script_state = self.get_entity_mut(eid)
                        .and_then(|entity| entity.script_state.take());

                    if let Some(ref mut state) = script_state {
                        if state.error.take().is_some() {
                            // Error recovery during channel: reset, don't interrupt.
                            state.pc = 0;
                            state.stack.clear();
                            state.call_stack.clear();
                            state.yielded = false;
                            state.step_limit_hit = false;
                            let num_vars = state.variables.len();
                            state.variables = vec![SimValue::None; num_vars];
                            state.variables[0] = SimValue::EntityRef(eid);
                        } else {
                            let mut instant_count = 0u32;
                            let mut brain_restarted = false;
                            loop {
                                instant_count += 1;
                                if instant_count > 1000 { break; }
                                match executor::execute_unit(eid, state, self) {
                                    Ok(Some(action)) => {
                                        match self.try_handle_instant(eid, action, state) {
                                            None => {} // instant, continue
                                            Some(UnitAction::Wait) => break,
                                            Some(real_action) => {
                                                interrupted = true;
                                                actions.push((eid, real_action));
                                                break;
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        if let Some(brain_pc) = state.program.brain_entry_pc {
                                            state.reset_for_brain_loop(brain_pc);
                                            if !brain_restarted {
                                                brain_restarted = true;
                                                continue;
                                            }
                                        }
                                        break;
                                    }
                                    Err(err) => {
                                        state.error = Some(err.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if let Some(entity) = self.get_entity_mut(eid) {
                        entity.script_state = script_state;
                    }

                    if interrupted {
                        // Cancel the Lua coroutine.
                        if let Some(ref mut handler) = self.command_handler {
                            handler.cancel_coroutine(lua_ch.handle);
                        }
                        self.events.push(SimEvent::ChannelInterrupted {
                            entity_id: eid,
                            command: lua_ch.command_name.clone(),
                        });
                        continue;
                    }
                }

                if lua_ch.remaining_ticks <= 0 {
                    // Time to resume the coroutine. Take handler out to avoid borrow conflicts.
                    if let Some(mut handler) = self.command_handler.take() {
                        let mut access = WorldAccess::new_from_world_ptr(self, eid);
                        match handler.resume_coroutine(&mut access, eid, lua_ch.handle) {
                            crate::action::CommandHandlerResult::Completed { events } => {
                                let lua_events = std::mem::take(&mut access.events);
                                self.events.extend(lua_events);
                                self.events.extend(events);
                                self.events.push(SimEvent::ChannelCompleted {
                                    entity_id: eid,
                                    command: lua_ch.command_name,
                                });
                            }
                            crate::action::CommandHandlerResult::Yielded { events, handle, remaining_ticks, interruptible } => {
                                let lua_events = std::mem::take(&mut access.events);
                                self.events.extend(lua_events);
                                self.events.extend(events);
                                if let Some(entity) = self.get_entity_mut(eid) {
                                    entity.active_channel = Some(crate::entity::LuaCoroutineState {
                                        handle,
                                        command_name: lua_ch.command_name,
                                        remaining_ticks,
                                        interruptible,
                                    });
                                }
                            }
                            crate::action::CommandHandlerResult::Error(msg) => {
                                self.events.push(SimEvent::ScriptOutput {
                                    entity_id: eid,
                                    text: format!("[lua error] {msg}"),
                                });
                            }
                            crate::action::CommandHandlerResult::NotHandled => {}
                        }
                        self.command_handler = Some(handler);
                    }
                } else {
                    // Still waiting — put coroutine state back.
                    if let Some(entity) = self.get_entity_mut(eid) {
                        entity.active_channel = Some(lua_ch);
                    }
                }

                continue;
            }

            // --- Normal script execution ---
            // Take script state out to avoid borrow conflicts.
            let mut script_state = match self.get_entity_mut(eid) {
                Some(entity) => match entity.script_state.take() {
                    Some(s) => s,
                    None => continue,
                },
                None => continue,
            };

            // Error recovery: clear error, reset script, yield wait this tick.
            if let Some(err_msg) = script_state.error.take() {
                script_state.pc = 0;
                script_state.stack.clear();
                script_state.call_stack.clear();
                script_state.yielded = false;
                script_state.step_limit_hit = false;
                let num_vars = script_state.variables.len();
                script_state.variables = vec![SimValue::None; num_vars];
                script_state.variables[0] = SimValue::EntityRef(eid);

                self.events.push(SimEvent::ScriptOutput {
                    entity_id: eid,
                    text: format!("[error recovery] Previous error: {err_msg} — script restarted"),
                });
                if let Some(entity) = self.get_entity_mut(eid) {
                    entity.script_state = Some(script_state);
                }
                actions.push((eid, UnitAction::Wait));
                continue;
            }

            // Execute until a tick-consuming action, error, or halt.
            // Brain scripts get one restart per tick: if they halt without yielding
            // a tick-consuming action, they reset and re-enter immediately so the
            // halt instruction doesn't waste a tick.
            let mut instant_count = 0u32;
            let mut brain_restarted = false;
            loop {
                instant_count += 1;
                if instant_count > 1000 {
                    script_state.error = Some("too many instant actions in one tick (infinite loop?)".to_string());
                    self.events.push(Self::script_error_event(eid, "too many instant actions in one tick (infinite loop?)", &script_state));
                    break;
                }
                match executor::execute_unit(eid, &mut script_state, self) {
                    Ok(Some(action)) => {
                        if let Some(real_action) = self.try_handle_instant(eid, action, &mut script_state) {
                            actions.push((eid, real_action));
                            break;
                        }
                        // Instant action handled, continue executing.
                    }
                    Ok(None) => {
                        if let Some(brain_pc) = script_state.program.brain_entry_pc {
                            script_state.reset_for_brain_loop(brain_pc);
                            if !brain_restarted {
                                brain_restarted = true;
                                continue; // Re-enter executor once to avoid wasting the tick.
                            }
                        }
                        break;
                    }
                    Err(err) => {
                        script_state.error = Some(err.to_string());
                        self.events.push(Self::script_error_event(eid, &err.to_string(), &script_state));
                        self.events.push(SimEvent::ScriptFinished {
                            entity_id: eid,
                            success: false,
                            error: Some(err.to_string()),
                        });
                        break;
                    }
                }
            }

            // Emit warning if step limit was hit.
            if script_state.step_limit_hit {
                self.events.push(SimEvent::ScriptOutput {
                    entity_id: eid,
                    text: "[warning] Script exceeded step limit (10000 instructions) — auto-yielded".into(),
                });
            }

            // Put script state back.
            if let Some(entity) = self.get_entity_mut(eid) {
                entity.script_state = Some(script_state);
            }
        }

        // 5. Resolve all actions (shuffled order = conflict resolution order).
        for (eid, action) in actions {
            let action_events = resolve_action(self, eid, action);
            self.events.extend(action_events);
        }

        // 6. Tick passive systems.
        for entity in &mut self.entities {
            if !entity.alive {
                continue;
            }
            // Cooldown ticking.
            let cd = entity.stat("cooldown_remaining");
            if cd > 0 {
                entity.set_stat("cooldown_remaining", cd - 1);
            }
        }

        // 6b. Tick buffs: duration decrement, modifier reversal on expire, Lua callbacks.
        if !self.buff_registry.is_empty() {
            self.tick_buffs();
        }

        // 7. Flush pending spawns/despawns.
        self.flush_pending();

        // Clean up dead entities (swap-remove for performance).
        self.entities.retain(|e| e.alive);
        self.rebuild_index();

        // 8. Process triggers via command handler (Lua).
        if self.command_handler.is_some() {
            let tick_events = self.events.clone();
            let mut handler = self.command_handler.take().unwrap();
            // Use a dummy caster — the handler decides which entity to use.
            let dummy_caster = self.entities.iter().find(|e| e.alive).map(|e| e.id)
                .unwrap_or(EntityId(0));
            let mut access = WorldAccess::new_from_world_ptr(self, dummy_caster);
            let trigger_events = handler.process_triggers(&mut access, &tick_events);
            let lua_events = std::mem::take(&mut access.events);
            self.events.extend(lua_events);
            self.events.extend(trigger_events);
            self.command_handler = Some(handler);
        }
    }

    /// Tick all active buffs: decrement durations, reverse modifiers on expire, Lua callbacks.
    fn tick_buffs(&mut self) {
        // Collect entity IDs with active buffs.
        let entities_with_buffs: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|e| e.alive && !e.active_buffs.is_empty())
            .map(|e| e.id)
            .collect();

        for eid in entities_with_buffs {
            // Get buff names for this entity.
            let buff_names: Vec<String> = self
                .get_entity(eid)
                .map(|e| e.active_buffs.iter().map(|b| b.name.clone()).collect())
                .unwrap_or_default();

            // Run per_tick callbacks via Lua handler.
            for name in &buff_names {
                if self.buff_registry.contains_key(name) {
                    if let Some(mut handler) = self.command_handler.take() {
                        let mut access = WorldAccess::new_from_world_ptr(self, eid);
                        let events = handler.buff_callback(
                            &mut access, eid, name,
                            crate::action::BuffCallbackType::PerTick,
                        );
                        let lua_events = std::mem::take(&mut access.events);
                        self.events.extend(lua_events);
                        self.events.extend(events);
                        self.command_handler = Some(handler);
                    }
                }
            }

            // Decrement durations and collect expired buffs.
            let mut expired: Vec<(String, i64)> = Vec::new(); // (name, stacks)
            if let Some(entity) = self.get_entity_mut(eid) {
                entity.active_buffs.retain_mut(|buff| {
                    buff.remaining_ticks -= 1;
                    if buff.remaining_ticks <= 0 {
                        expired.push((buff.name.clone(), buff.stacks));
                        false
                    } else {
                        true
                    }
                });
            }

            // Handle expired buffs: reverse modifiers and run on_expire callback.
            for (name, stacks) in expired {
                if let Some(buff_def) = self.buff_registry.get(&name).cloned() {
                    // Reverse all stacks of modifiers.
                    for _ in 0..stacks {
                        reverse_buff_modifiers(self, eid, &buff_def);
                    }
                    // Run on_expire callback via Lua handler.
                    if let Some(mut handler) = self.command_handler.take() {
                        let mut access = WorldAccess::new_from_world_ptr(self, eid);
                        let events = handler.buff_callback(
                            &mut access, eid, &name,
                            crate::action::BuffCallbackType::OnExpire,
                        );
                        let lua_events = std::mem::take(&mut access.events);
                        self.events.extend(lua_events);
                        self.events.extend(events);
                        self.command_handler = Some(handler);
                    }
                }
            }
        }
    }

    /// Returns `None` if the action was handled as instant (no tick consumed),
    /// `Some(action)` if it should be collected as a tick-consuming action.
    fn try_handle_instant(
        &mut self,
        eid: EntityId,
        action: UnitAction,
        script_state: &mut crate::entity::ScriptState,
    ) -> Option<UnitAction> {
        match action {
            UnitAction::Print { text } => {
                self.events.push(SimEvent::ScriptOutput { entity_id: eid, text });
                None // instant — no tick consumed
            }
            UnitAction::Query { name, args } => {
                if self.command_handler.is_some() {
                    let mut handler = self.command_handler.take().unwrap();
                    let mut access = WorldAccess::new_from_world_ptr(self, eid);
                    let result = handler.resolve_query(&mut access, eid, &name, &args);
                    let lua_events = std::mem::take(&mut access.events);
                    self.events.extend(lua_events);
                    match result {
                        crate::action::QueryResult::Value { value, events } => {
                            self.events.extend(events);
                            script_state.stack.push(value);
                        }
                        crate::action::QueryResult::NotHandled => {
                            self.events.push(SimEvent::ScriptOutput {
                                entity_id: eid,
                                text: format!("[{name}] query (no handler)"),
                            });
                            script_state.stack.push(SimValue::None);
                        }
                        crate::action::QueryResult::Error(msg) => {
                            self.events.push(SimEvent::ScriptOutput {
                                entity_id: eid,
                                text: format!("[lua error] {msg}"),
                            });
                            script_state.stack.push(SimValue::None);
                        }
                    }
                    self.command_handler = Some(handler);
                } else {
                    script_state.stack.push(SimValue::None);
                }
                None // instant — continue executing
            }
            // All other actions consume the tick.
            _ => Some(action),
        }
    }

    /// Rebuild the entity index after removals.
    fn rebuild_index(&mut self) {
        self.entity_index.clear();
        for (i, entity) in self.entities.iter().enumerate() {
            self.entity_index.insert(entity.id, i);
        }
    }
}

// ---------------------------------------------------------------------------
// WorldAccess — safe interface for external command handlers (Lua)
// ---------------------------------------------------------------------------

/// Controlled access to SimWorld for external command handlers.
///
/// Provides the same operations that Lua effects use, as an explicit API.
/// The `events` buffer collects SimEvents generated during the handler invocation.
pub struct WorldAccess<'a> {
    world: &'a mut SimWorld,
    pub caster_id: EntityId,
    pub events: Vec<SimEvent>,
    tick_seed: u64,
}

impl<'a> WorldAccess<'a> {
    /// Create a WorldAccess from a mutable reference to SimWorld.
    /// Used in the tick loop where we already have &mut SimWorld
    /// but need to pass it through to the command handler.
    pub fn new_from_world_ptr(world: &'a mut SimWorld, caster_id: EntityId) -> Self {
        let tick_seed = world.tick_seed();
        Self { world, caster_id, events: Vec::new(), tick_seed }
    }

    pub fn tick(&self) -> u64 { self.world.tick }
    pub fn tick_seed(&self) -> u64 { self.tick_seed }

    // --- Entity operations ---

    pub fn get_entity(&self, id: EntityId) -> Option<&SimEntity> {
        self.world.get_entity(id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut SimEntity> {
        self.world.get_entity_mut(id)
    }

    pub fn entities(&self) -> impl Iterator<Item = &SimEntity> {
        self.world.entities()
    }

    pub fn damage(&mut self, caster_id: EntityId, target_id: EntityId, amount: i64) {
        if let Some(target) = self.world.get_entity_mut(target_id) {
            let mut remaining = amount;
            let shield = target.stat("shield");
            if shield > 0 {
                let absorbed = remaining.min(shield);
                target.set_stat("shield", shield - absorbed);
                remaining -= absorbed;
            }
            let new_health = (target.stat("health") - remaining).max(0);
            target.set_stat("health", new_health);
            self.events.push(SimEvent::EntityDamaged {
                entity_id: target_id,
                damage: amount,
                new_health,
                attacker_id: Some(caster_id),
            });
            if new_health <= 0 {
                target.alive = false;
                let owner_id = target.owner;
                self.events.push(SimEvent::EntityDied {
                    entity_id: target_id,
                    name: target.name.clone(),
                    killer_id: Some(caster_id),
                    owner_id,
                });
            }
        }
    }

    pub fn heal(&mut self, target_id: EntityId, amount: i64) {
        if let Some(target) = self.world.get_entity_mut(target_id) {
            let new_health = target.stat("health").saturating_add(amount);
            target.set_stat("health", new_health);
            target.clamp_stat("health");
        }
    }

    pub fn modify_stat(&mut self, target_id: EntityId, stat: &str, amount: i64) {
        if let Some(target) = self.world.get_entity_mut(target_id) {
            let new_val = target.stat(stat).saturating_add(amount);
            target.set_stat(stat, new_val);
            target.clamp_stat(stat);
        }
    }

    pub fn set_stat(&mut self, target_id: EntityId, stat: &str, value: i64) {
        if let Some(target) = self.world.get_entity_mut(target_id) {
            target.set_stat(stat, value);
            target.clamp_stat(stat);
        }
    }

    pub fn get_stat(&self, target_id: EntityId, stat: &str) -> i64 {
        self.world.get_entity(target_id).map_or(0, |e| e.stat(stat))
    }

    pub fn spawn(&mut self, caster_id: EntityId, entity_type: &str, offset: i64) -> EntityId {
        let position = self.world.get_entity(caster_id)
            .map(|e| e.position + offset)
            .unwrap_or(offset);
        let id = EntityId(self.world.next_entity_id());
        let types = self.world.entity_types_registry
            .get(entity_type)
            .cloned()
            .unwrap_or_else(|| vec![entity_type.to_string()]);
        let mut spawned = SimEntity::new_with_types(
            id,
            entity_type.to_string(),
            types,
            format!("{}_{}", entity_type, id.0),
            position,
        );
        if let Some(config) = self.world.entity_configs.get(entity_type) {
            spawned.apply_config(config);
        }
        spawned.owner = Some(caster_id);
        spawned.spawn_ticks_remaining = self.world.spawn_durations
            .get(entity_type)
            .copied()
            .unwrap_or(0);
        self.world.queue_spawn(spawned);
        id
    }

    pub fn animate(&mut self, target_id: EntityId, animation: &str) {
        self.events.push(SimEvent::PlayAnimation {
            entity_id: target_id,
            animation: animation.to_string(),
        });
    }

    // --- Resource operations ---

    pub fn get_resource(&self, name: &str) -> i64 {
        self.world.get_resource(name)
    }

    pub fn try_spend_resource(&mut self, name: &str, amount: i64) -> bool {
        self.world.try_spend_resource(name, amount)
    }

    pub fn gain_resource(&mut self, name: &str, amount: i64) -> i64 {
        self.world.gain_resource(name, amount)
    }

    // --- Output ---

    pub fn output(&mut self, entity_id: EntityId, text: &str) {
        self.events.push(SimEvent::ScriptOutput {
            entity_id,
            text: text.to_string(),
        });
    }

    /// List commands, matching the ListCommands effect behavior.
    pub fn list_commands(&mut self, entity_id: EntityId) {
        let max_width = self.world.command_order.iter()
            .filter(|n| self.world.custom_command_descriptions.contains_key(*n) && !self.world.unlisted_commands.contains(*n))
            .map(|n| n.len() + 2)
            .max()
            .unwrap_or(0);
        for name in &self.world.command_order {
            if self.world.unlisted_commands.contains(name) { continue; }
            if let Some(description) = self.world.custom_command_descriptions.get(name) {
                let padded = format!("{name}()");
                self.events.push(SimEvent::ScriptOutput {
                    entity_id,
                    text: format!("{padded:<width$} — {description}", width = max_width + 1),
                });
            }
        }
    }

    // --- Queries ---

    pub fn entity_count(&self, type_name: &str) -> i64 {
        self.world.entities()
            .filter(|e| e.alive && e.spawn_ticks_remaining == 0 && e.has_type(type_name))
            .count() as i64
    }

    pub fn is_alive(&self, target_id: EntityId) -> bool {
        self.world.get_entity(target_id).map_or(false, |e| e.alive)
    }

    pub fn distance(&self, a: EntityId, b: EntityId) -> i64 {
        let a_pos = self.world.get_entity(a).map_or(0, |e| e.position);
        let b_pos = self.world.get_entity(b).map_or(0, |e| e.position);
        (a_pos - b_pos).abs()
    }

    pub fn has_buff(&self, target_id: EntityId, buff: &str) -> bool {
        self.world.get_entity(target_id)
            .map_or(false, |e| e.active_buffs.iter().any(|b| b.name == buff))
    }

    pub fn has_type(&self, target_id: EntityId, type_name: &str) -> bool {
        self.world.get_entity(target_id).map_or(false, |e| e.has_type(type_name))
    }

    pub fn position(&self, target_id: EntityId) -> i64 {
        self.world.get_entity(target_id).map_or(0, |e| e.position)
    }

    pub fn move_to(&mut self, entity_id: EntityId, position: i64) {
        let clamped = position.clamp(0, self.world.world_width);
        if let Some(entity) = self.world.get_entity_mut(entity_id) {
            entity.position = clamped;
        }
        self.events.push(SimEvent::EntityMoved {
            entity_id,
            new_position: clamped,
        });
    }

    pub fn move_by(&mut self, entity_id: EntityId, offset: i64) {
        let new_pos = self.world.get_entity(entity_id)
            .map_or(0, |e| e.position.saturating_add(offset));
        self.move_to(entity_id, new_pos);
    }

    pub fn face_to(&mut self, entity_id: EntityId, target_id: EntityId) {
        let my_pos = self.world.get_entity(entity_id).map_or(0, |e| e.position);
        let target_pos = self.world.get_entity(target_id).map_or(0, |e| e.position);
        if my_pos != target_pos {
            self.events.push(SimEvent::EntityFlipped {
                entity_id,
                facing_left: target_pos < my_pos,
            });
        }
    }

    pub fn owner(&self, target_id: EntityId) -> Option<EntityId> {
        self.world.get_entity(target_id).and_then(|e| e.owner)
    }

    pub fn entities_of_type(&self, type_name: &str) -> Vec<EntityId> {
        self.world.entities()
            .filter(|e| e.alive && e.spawn_ticks_remaining == 0 && e.has_type(type_name))
            .map(|e| e.id)
            .collect()
    }

    // --- Buff operations ---

    pub fn apply_buff(&mut self, target_id: EntityId, buff_name: &str, duration_override: Option<i64>) {
        if let Some(buff_def) = self.world.buff_registry.get(buff_name).cloned() {
            let dur = duration_override.unwrap_or(buff_def.duration);
            let existing = self.world.get_entity(target_id)
                .and_then(|e| e.active_buffs.iter().position(|b| b.name == buff_name));

            if let Some(idx) = existing {
                if buff_def.stackable {
                    let at_max = buff_def.max_stacks > 0
                        && self.world.get_entity(target_id).map_or(true, |e| e.active_buffs[idx].stacks >= buff_def.max_stacks);
                    if !at_max {
                        crate::action::apply_buff_modifiers(self.world, target_id, &buff_def);
                        if let Some(entity) = self.world.get_entity_mut(target_id) {
                            entity.active_buffs[idx].stacks += 1;
                            entity.active_buffs[idx].remaining_ticks = dur;
                        }
                    }
                } else {
                    if let Some(entity) = self.world.get_entity_mut(target_id) {
                        entity.active_buffs[idx].remaining_ticks = dur;
                    }
                }
            } else {
                crate::action::apply_buff_modifiers(self.world, target_id, &buff_def);
                if let Some(entity) = self.world.get_entity_mut(target_id) {
                    entity.active_buffs.push(crate::entity::ActiveBuff {
                        name: buff_name.to_string(),
                        remaining_ticks: dur,
                        stacks: 1,
                    });
                }
            }
        }
    }

    pub fn remove_buff(&mut self, target_id: EntityId, buff_name: &str) {
        if let Some(buff_def) = self.world.buff_registry.get(buff_name).cloned() {
            let removed = self.world.get_entity(target_id)
                .and_then(|e| e.active_buffs.iter().position(|b| b.name == buff_name))
                .map(|idx| {
                    let stacks = self.world.get_entity(target_id).unwrap().active_buffs[idx].stacks;
                    (idx, stacks)
                });
            if let Some((idx, stacks)) = removed {
                for _ in 0..stacks {
                    crate::action::reverse_buff_modifiers(self.world, target_id, &buff_def);
                }
                if let Some(entity) = self.world.get_entity_mut(target_id) {
                    entity.active_buffs.remove(idx);
                }
            }
        }
    }

    // --- Resource availability ---

    pub fn set_available_resources(&mut self, names: &[String]) {
        let mut set = std::collections::HashSet::new();
        for name in names {
            set.insert(name.clone());
        }
        self.world.available_resources = Some(set);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityConfig, ScriptState};
    use crate::ir::{CompiledScript, Instruction};
    use crate::value::SimValue;

    /// Default test entity config with basic stats for tests that need movement/health.
    fn test_config() -> EntityConfig {
        EntityConfig {
            stats: IndexMap::from([
                ("health".into(), 100),
                ("max_health".into(), 100),
                ("speed".into(), 1),
                ("attack_damage".into(), 10),
                ("attack_range".into(), 5),
                ("attack_cooldown".into(), 3),
            ]),
        }
    }

    /// Spawn a test entity with default stats.
    fn spawn_test_entity(world: &mut SimWorld, etype: &str, name: &str, pos: i64) -> EntityId {
        world.spawn_entity_with_config(etype.into(), name.into(), pos, Some(&test_config()))
    }

    #[test]
    fn spawn_and_query() {
        let mut world = SimWorld::new(42);
        let id = spawn_test_entity(&mut world, "skeleton", "miner1", 100);
        let entity = world.get_entity(id).unwrap();
        assert_eq!(entity.position, 100);
        assert_eq!(entity.name, "miner1");
    }

    #[test]
    fn tick_without_running_is_noop() {
        let mut world = SimWorld::new(42);
        spawn_test_entity(&mut world, "skeleton", "m", 0);
        world.tick(); // not started
        assert_eq!(world.tick, 0);
    }

    #[test]
    fn script_error_recorded() {
        let mut world = SimWorld::new(42);
        let id = spawn_test_entity(&mut world, "skeleton", "m", 0);

        // Division by zero.
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Int(1)),
                Instruction::LoadConst(SimValue::Int(0)),
                Instruction::Div,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        assert!(events.iter().any(|e| matches!(e, SimEvent::ScriptError { .. })));
    }

    // --- Global resource tests ---

    #[test]
    fn get_resource_returns_zero_for_undefined() {
        let world = SimWorld::new(42);
        assert_eq!(world.get_resource("souls"), 0);
    }

    #[test]
    fn gain_resource_adds_correctly() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 0);
        assert_eq!(world.gain_resource("souls", 10), 10);
        assert_eq!(world.gain_resource("souls", 5), 15);
        assert_eq!(world.get_resource("souls"), 15);
    }

    #[test]
    fn try_spend_resource_succeeds_and_fails() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 10);
        assert!(world.try_spend_resource("souls", 7));
        assert_eq!(world.get_resource("souls"), 3);
        assert!(!world.try_spend_resource("souls", 5)); // insufficient
        assert_eq!(world.get_resource("souls"), 3); // unchanged
    }

    #[test]
    fn gain_resource_creates_if_nonexistent() {
        let mut world = SimWorld::new(42);
        assert_eq!(world.gain_resource("gold", 5), 5);
        assert_eq!(world.get_resource("gold"), 5);
    }

    // --- Resource availability tests ---

    #[test]
    fn resource_available_when_no_restriction() {
        let world = SimWorld::new(42);
        // available_resources is None by default → all resources available.
        assert!(world.check_resource_available("anything").is_ok());
    }

    #[test]
    fn resource_available_when_in_set() {
        let mut world = SimWorld::new(42);
        world.available_resources = Some(["bones".to_string()].into_iter().collect());
        assert!(world.check_resource_available("bones").is_ok());
    }

    #[test]
    fn resource_unavailable_when_not_in_set() {
        let mut world = SimWorld::new(42);
        world.available_resources = Some(["bones".to_string()].into_iter().collect());
        assert!(world.check_resource_available("souls").is_err());
    }

    // --- Custom stats tests ---

    #[test]
    fn custom_stat_from_entity_config() {
        let mut world = SimWorld::new(42);
        let config = crate::entity::EntityConfig {
            stats: {
                let mut m = indexmap::IndexMap::new();
                m.insert("armor".into(), 10);
                m.insert("crit".into(), 5);
                m
            },
        };
        let id = world.spawn_entity_with_config(
            "warrior".into(), "w1".into(), 100, Some(&config),
        );

        assert_eq!(world.get_entity(id).unwrap().stat("armor"), 10);
        assert_eq!(world.get_entity(id).unwrap().stat("crit"), 5);
    }

    // -----------------------------------------------------------------------
    // Contains / NotContains executor tests
    // -----------------------------------------------------------------------

    /// Helper: run instructions on a fresh world (single entity) and return final state.
    fn run_instructions(instructions: Vec<Instruction>) -> (ScriptState, Option<UnitAction>) {
        let mut world = SimWorld::new(42);
        let eid = spawn_test_entity(&mut world, "skeleton", "test", 0);
        let program = CompiledScript::new(instructions, 0);
        let mut state = ScriptState::new(program, 0);
        let action = crate::executor::execute_unit(eid, &mut state, &world).unwrap();
        (state, action)
    }

    #[test]
    fn contains_in_list() {
        let (state, _) = run_instructions(vec![
            // Push item, then container => Contains pops container (top), then item
            Instruction::LoadConst(SimValue::Int(2)),
            Instruction::LoadConst(SimValue::List(vec![
                SimValue::Int(1),
                SimValue::Int(2),
                SimValue::Int(3),
            ])),
            Instruction::Contains,
            Instruction::Halt,
        ]);
        assert_eq!(state.stack.last(), Some(&SimValue::Bool(true)));
    }

    #[test]
    fn not_contains_in_list() {
        let (state, _) = run_instructions(vec![
            Instruction::LoadConst(SimValue::Int(99)),
            Instruction::LoadConst(SimValue::List(vec![
                SimValue::Int(1),
                SimValue::Int(2),
            ])),
            Instruction::NotContains,
            Instruction::Halt,
        ]);
        assert_eq!(state.stack.last(), Some(&SimValue::Bool(true)));
    }

    #[test]
    fn contains_in_string_and_dict() {
        // String: "el" in "hello" => true
        let (state, _) = run_instructions(vec![
            Instruction::LoadConst(SimValue::Str("el".into())),
            Instruction::LoadConst(SimValue::Str("hello".into())),
            Instruction::Contains,
            Instruction::Halt,
        ]);
        assert_eq!(state.stack.last(), Some(&SimValue::Bool(true)));

        // Dict: "key" in {"key": 1} => true
        let mut dict = indexmap::IndexMap::new();
        dict.insert("key".to_string(), SimValue::Int(1));
        let (state, _) = run_instructions(vec![
            Instruction::LoadConst(SimValue::Str("key".into())),
            Instruction::LoadConst(SimValue::Dict(dict)),
            Instruction::Contains,
            Instruction::Halt,
        ]);
        assert_eq!(state.stack.last(), Some(&SimValue::Bool(true)));
    }
}
