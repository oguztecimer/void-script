use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::action::{BuffDef, CommandDef, CommandEffect, EffectContext, EffectOutcome, PhaseDef, TriggerDef, UnitAction, evaluate_condition, resolve_action, resolve_custom_effects, resolve_custom_effects_with_ctx, reverse_buff_modifiers};
use crate::entity::{EntityConfig, EntityId, SimEntity};
use crate::executor;
use crate::rng::SimRng;
use crate::value::SimValue;

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
    /// Custom command name → effects (populated from mod definitions).
    pub custom_commands: IndexMap<String, Vec<CommandEffect>>,
    /// Custom command name → arg count (for the executor to know how many args to pop).
    pub custom_command_arg_counts: IndexMap<String, usize>,
    /// Custom command name → description (for list_commands effect).
    pub custom_command_descriptions: IndexMap<String, String>,
    /// Custom command name → phases (for phased/channeled commands).
    pub custom_command_phases: IndexMap<String, Vec<PhaseDef>>,
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
    /// Registered triggers — fire effects when game events match.
    pub triggers: Vec<TriggerDef>,
    /// Buff name → buff definition registry.
    pub buff_registry: IndexMap<String, BuffDef>,
    /// Main brain script state — runs first each tick, backed by a real entity.
    pub main_brain: Option<crate::entity::ScriptState>,
    /// Entity ID for the main brain entity.
    pub main_brain_entity: Option<EntityId>,
    /// External command handler (Lua runtime). If set, custom commands are
    /// dispatched here first; falls back to TOML effects if `NotHandled`.
    pub command_handler: Option<Box<dyn crate::action::CommandHandler>>,
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
            custom_commands: IndexMap::new(),
            custom_command_arg_counts: IndexMap::new(),
            custom_command_descriptions: IndexMap::new(),
            custom_command_phases: IndexMap::new(),
            entity_configs: IndexMap::new(),
            entity_types_registry: IndexMap::new(),
            spawn_durations: IndexMap::new(),
            command_order: Vec::new(),
            resources: IndexMap::new(),
            resource_caps: IndexMap::new(),
            available_resources: None,
            unlisted_commands: HashSet::new(),
            triggers: Vec::new(),
            buff_registry: IndexMap::new(),
            main_brain: None,
            main_brain_entity: None,
            command_handler: None,
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

    /// Register a custom command with its effects, arg count, costs, and description.
    /// If the command has phases, they are stored separately for the phased execution path.
    pub fn register_custom_command(&mut self, def: &CommandDef) {
        // Only data-driven custom commands need runtime registration.
        // Query/action/instant commands use their own IR instructions and
        // don't go through the custom command execution path.
        if matches!(def.kind, crate::action::CommandKind::Custom) {
            self.custom_command_arg_counts.insert(def.name.clone(), def.args.len());
            self.custom_commands.insert(def.name.clone(), def.effects.clone());
            if !def.phases.is_empty() {
                self.custom_command_phases.insert(def.name.clone(), def.phases.clone());
            }
        }
        // Description and unlisted apply to all command kinds (for list_commands).
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

    /// Register a trigger that fires effects when game events match.
    pub fn register_trigger(&mut self, trigger: TriggerDef) {
        self.triggers.push(trigger);
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

        // 1b. Snapshot state for trigger processing.
        let resource_snapshot = self.resources.clone();
        let entity_types_map: HashMap<EntityId, Vec<String>> = self
            .entities
            .iter()
            .map(|e| (e.id, e.types.clone()))
            .collect();

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
                match executor::execute_unit(eid, state, self) {
                    Ok(Some(action)) => {
                        // Handle instant actions in a loop.
                        match self.try_handle_instant(eid, action, state) {
                            None => {
                                let mut instant_count = 0u32;
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
                                                None => {} // another instant, keep going
                                                Some(UnitAction::Wait) => break,
                                                Some(real_action) => {
                                                    // Resolve tick-consuming action (custom commands, etc.)
                                                    let action_events = resolve_action(self, eid, real_action);
                                                    self.events.extend(action_events);
                                                    break;
                                                }
                                            }
                                        }
                                        Ok(None) => {
                                            if state.is_brain { state.reset_for_restart(eid); }
                                            break;
                                        }
                                        Err(err) => {
                                            state.error = Some(err.to_string());
                                            self.events.push(Self::script_error_event(eid, &err.to_string(), state));
                                            break;
                                        }
                                    }
                                }
                            }
                            Some(UnitAction::Wait) => {} // no-op
                            Some(real_action) => {
                                // Resolve tick-consuming action (custom commands, etc.)
                                let action_events = resolve_action(self, eid, real_action);
                                self.events.extend(action_events);
                            }
                        }
                    }
                    Ok(None) => {
                        if state.is_brain { state.reset_for_restart(eid); }
                    }
                    Err(err) => {
                        state.error = Some(err.to_string());
                        self.events.push(Self::script_error_event(eid, &err.to_string(), state));
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
            // --- Channel processing ---
            let has_channel = self.get_entity(eid).map_or(false, |e| e.active_channel.is_some());

            if has_channel {
                use crate::entity::ActiveChannel;
                let active = self.get_entity_mut(eid).unwrap().active_channel.take().unwrap();

                match active {
                    ActiveChannel::Lua(mut lua_ch) => {
                        // --- Lua coroutine channel processing ---
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
                                    match executor::execute_unit(eid, state, self) {
                                        Ok(Some(action)) => {
                                            match self.try_handle_instant(eid, action, state) {
                                                None => {
                                                    let mut instant_count = 0u32;
                                                    loop {
                                                        instant_count += 1;
                                                        if instant_count > 1000 { break; }
                                                        match executor::execute_unit(eid, state, self) {
                                                            Ok(Some(action)) => {
                                                                match self.try_handle_instant(eid, action, state) {
                                                                    None => {}
                                                                    Some(UnitAction::Wait) => break,
                                                                    Some(real_action) => {
                                                                        interrupted = true;
                                                                        actions.push((eid, real_action));
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                            Ok(None) => {
                                                                if state.is_brain { state.reset_for_restart(eid); }
                                                                break;
                                                            }
                                                            Err(err) => {
                                                                state.error = Some(err.to_string());
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                                Some(UnitAction::Wait) => {}
                                                Some(real_action) => {
                                                    interrupted = true;
                                                    actions.push((eid, real_action));
                                                }
                                            }
                                        }
                                        Ok(None) => {
                                            if state.is_brain { state.reset_for_restart(eid); }
                                        }
                                        Err(err) => { state.error = Some(err.to_string()); }
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
                                            entity.active_channel = Some(ActiveChannel::Lua(crate::entity::LuaCoroutineState {
                                                handle,
                                                command_name: lua_ch.command_name,
                                                remaining_ticks,
                                                interruptible,
                                            }));
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
                                entity.active_channel = Some(ActiveChannel::Lua(lua_ch));
                            }
                        }

                        continue;
                    }

                    ActiveChannel::Toml(mut channel) => {
                // --- TOML channel processing (legacy) ---
                let phase = channel.phases[channel.phase_index].clone();
                let is_interruptible = phase.interruptible;
                let is_first_tick = channel.ticks_elapsed_in_phase == 0;

                // If interruptible, let the script run and check if it interrupts.
                let mut interrupted = false;
                if is_interruptible {
                    let mut script_state: Option<_> = self.get_entity_mut(eid)
                        .and_then(|entity| entity.script_state.take());

                    if let Some(ref mut state) = script_state {
                        // Error recovery during channel: reset script, don't interrupt.
                        if let Some(err_msg) = state.error.take() {
                            state.pc = 0;
                            state.stack.clear();
                            state.call_stack.clear();
                            state.yielded = false;
                            state.step_limit_hit = false;
                            let num_vars = state.variables.len();
                            state.variables = vec![SimValue::None; num_vars];
                            state.variables[0] = SimValue::EntityRef(eid);
                            self.events.push(SimEvent::ScriptOutput {
                                entity_id: eid,
                                text: format!("[error recovery] Previous error: {err_msg} — script restarted"),
                            });
                        } else {
                        match executor::execute_unit(eid, state, self) {
                            Ok(Some(action)) => {
                                // Instant actions don't interrupt — handle and keep going.
                                match self.try_handle_instant(eid, action, state) {
                                    None => {
                                        // Was instant. Continue executing in a loop.
                                        let mut instant_count = 0u32;
                                        loop {
                                            instant_count += 1;
                                            if instant_count > 1000 {
                                                state.error = Some("too many instant actions in one tick (infinite loop?)".to_string());
                                                self.events.push(Self::script_error_event(eid, "too many instant actions in one tick (infinite loop?)", state));
                                                break;
                                            }
                                            match executor::execute_unit(eid, state, self) {
                                                Ok(Some(action)) => {
                                                    match self.try_handle_instant(eid, action, state) {
                                                        None => {} // another instant, keep going
                                                        Some(UnitAction::Wait) => break,
                                                        Some(real_action) => {
                                                            interrupted = true;
                                                            actions.push((eid, real_action));
                                                            break;
                                                        }
                                                    }
                                                }
                                                Ok(None) => {
                                                    if state.is_brain { state.reset_for_restart(eid); }
                                                    break;
                                                }
                                                Err(err) => {
                                                    state.error = Some(err.to_string());
                                                    self.events.push(Self::script_error_event(eid, &err.to_string(), state));
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Some(UnitAction::Wait) => {} // no interruption
                                    Some(real_action) => {
                                        interrupted = true;
                                        actions.push((eid, real_action));
                                    }
                                }
                            }
                            Ok(None) => {
                                if state.is_brain { state.reset_for_restart(eid); }
                            }
                            Err(err) => {
                                state.error = Some(err.to_string());
                                self.events.push(Self::script_error_event(eid, &err.to_string(), state));
                            }
                        }

                        if state.step_limit_hit {
                            self.events.push(SimEvent::ScriptOutput {
                                entity_id: eid,
                                text: "[warning] Script exceeded step limit (10000 instructions) — auto-yielded".into(),
                            });
                        }
                        } // end else (error recovery)
                    }

                    // Put script state back.
                    if let Some(entity) = self.get_entity_mut(eid) {
                        entity.script_state = script_state;
                    }
                }

                if interrupted {
                    self.events.push(SimEvent::ScriptOutput {
                        entity_id: eid,
                        text: format!("[{}] interrupted", channel.command_name),
                    });
                    self.events.push(SimEvent::ChannelInterrupted {
                        entity_id: eid,
                        command: channel.command_name.clone(),
                    });
                    // Channel cancelled, entity proceeds with interrupting action.
                    continue;
                }

                // Run on_start effects (first tick of phase only).
                let mut channel_cancelled = false;
                if is_first_tick && !phase.on_start.is_empty() {
                    let mut effect_events = Vec::new();
                    let outcome = resolve_custom_effects(
                        self, eid, &channel.command_name,
                        &phase.on_start, &channel.args, &mut effect_events,
                    );
                    self.events.extend(effect_events);
                    if matches!(outcome, EffectOutcome::Aborted) {
                        channel_cancelled = true;
                    }
                    // StartChannel inside an active channel is ignored.
                }

                // Run per_update effects (respecting update_interval).
                // update_interval=1: fires every tick (0,1,2,...).
                // update_interval=2: fires at elapsed 1,3,5,... (every 2nd tick).
                // update_interval=3: fires at elapsed 2,5,8,... (every 3rd tick).
                let interval = phase.update_interval.max(1);
                let should_update = (channel.ticks_elapsed_in_phase + 1) % interval == 0;
                if !channel_cancelled && should_update && !phase.per_update.is_empty() {
                    let mut effect_events = Vec::new();
                    let outcome = resolve_custom_effects(
                        self, eid, &channel.command_name,
                        &phase.per_update, &channel.args, &mut effect_events,
                    );
                    self.events.extend(effect_events);
                    if matches!(outcome, EffectOutcome::Aborted) {
                        channel_cancelled = true;
                    }
                    // StartChannel inside an active channel is ignored.
                }

                if channel_cancelled {
                    continue;
                }

                // Advance tick counter within phase.
                channel.ticks_elapsed_in_phase += 1;
                if channel.ticks_elapsed_in_phase >= phase.ticks {
                    channel.phase_index += 1;
                    channel.ticks_elapsed_in_phase = 0;
                }

                // Check if channel is complete.
                if channel.phase_index < channel.phases.len() {
                    if let Some(entity) = self.get_entity_mut(eid) {
                        entity.active_channel = Some(ActiveChannel::Toml(channel));
                    }
                } else {
                    // Channel done — entity resumes normal script execution next tick.
                    self.events.push(SimEvent::ChannelCompleted {
                        entity_id: eid,
                        command: channel.command_name.clone(),
                    });
                }

                continue;
                    } // end ActiveChannel::Toml
                } // end match active
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

            // Execute until action or halt.
            match executor::execute_unit(eid, &mut script_state, self) {
                Ok(Some(action)) => {
                    // Handle instant actions (Print, resource ops) — they don't
                    // consume the tick, so we handle them and re-enter the executor.
                    if let Some(real_action) = self.try_handle_instant(eid, action, &mut script_state) {
                        actions.push((eid, real_action));
                    } else {
                        // Instant action handled. Continue executing for the rest of the tick.
                        let mut instant_count = 0u32;
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
                                    // else: another instant action, keep looping
                                }
                                Ok(None) => {
                                    if script_state.is_brain { script_state.reset_for_restart(eid); }
                                    break;
                                }
                                Err(err) => {
                                    script_state.error = Some(err.to_string());
                                    self.events.push(Self::script_error_event(eid, &err.to_string(), &script_state));
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    if script_state.is_brain { script_state.reset_for_restart(eid); }
                }
                Err(err) => {
                    script_state.error = Some(err.to_string());
                    self.events.push(Self::script_error_event(eid, &err.to_string(), &script_state));
                    self.events.push(SimEvent::ScriptFinished {
                        entity_id: eid,
                        success: false,
                        error: Some(err.to_string()),
                    });
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

        // 6b. Tick buffs: per_tick effects, duration decrement, expiry.
        if !self.buff_registry.is_empty() {
            self.tick_buffs();
        }

        // 7. Flush pending spawns/despawns.
        self.flush_pending();

        // Clean up dead entities (swap-remove for performance).
        self.entities.retain(|e| e.alive);
        self.rebuild_index();

        // 8. Process triggers against events collected during this tick.
        self.process_triggers(&entity_types_map, &resource_snapshot);
    }

    /// Tick all active buffs: run per_tick effects, decrement durations, expire.
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

            // Run per_tick effects for each buff.
            for name in &buff_names {
                if let Some(buff_def) = self.buff_registry.get(name).cloned() {
                    if !buff_def.per_tick.is_empty() {
                        let mut events = Vec::new();
                        resolve_custom_effects(
                            self, eid, "buff", &buff_def.per_tick, &[], &mut events,
                        );
                        self.events.extend(events);
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

            // Handle expired buffs: reverse modifiers and run on_expire effects.
            for (name, stacks) in expired {
                if let Some(buff_def) = self.buff_registry.get(&name).cloned() {
                    // Reverse all stacks of modifiers.
                    for _ in 0..stacks {
                        reverse_buff_modifiers(self, eid, &buff_def);
                    }
                    // Run on_expire effects.
                    if !buff_def.on_expire.is_empty() {
                        let mut events = Vec::new();
                        resolve_custom_effects(
                            self, eid, "buff_expire", &buff_def.on_expire, &[], &mut events,
                        );
                        self.events.extend(events);
                    }
                }
            }
        }
    }

    /// Process triggers against events collected during this tick.
    ///
    /// Trigger effects use the first alive entity as the "caster" (typically the
    /// summoner), consistent with how `[initial].effects` are resolved.
    /// Trigger effects do not re-trigger other triggers within the same tick.
    fn process_triggers(
        &mut self,
        entity_types_map: &HashMap<EntityId, Vec<String>>,
        resource_snapshot: &IndexMap<String, i64>,
    ) {
        if self.triggers.is_empty() {
            return;
        }

        // Find the first alive entity as the "caster" for trigger effects.
        let caster_id = self.entities.iter().find(|e| e.alive).map(|e| e.id);
        let Some(caster) = caster_id else { return };

        // Clone events and triggers to avoid borrow conflicts.
        let tick_events = self.events.clone();
        let triggers = self.triggers.clone();

        for trigger in &triggers {
            match trigger.event.as_str() {
                "entity_died" => {
                    for event in &tick_events {
                        if let SimEvent::EntityDied { entity_id, killer_id, owner_id, .. } = event {
                            if let Some(ref filter_type) = trigger.filter.entity_type {
                                let types = entity_types_map.get(entity_id).map(|v| v.as_slice()).unwrap_or(&[]);
                                if !types.iter().any(|t| t == filter_type) { continue; }
                            }
                            if self.check_trigger_conditions(trigger, caster) {
                                let ctx = EffectContext {
                                    source: Some(*entity_id),
                                    killer: *killer_id,
                                    owner: *owner_id,
                                    attacker: None,
                                };
                                self.fire_trigger_effects(trigger, caster, &ctx);
                            }
                        }
                    }
                }
                "entity_spawned" => {
                    for event in &tick_events {
                        if let SimEvent::EntitySpawned { entity_id, entity_type, spawner_id, .. } = event {
                            if let Some(ref filter_type) = trigger.filter.entity_type {
                                if entity_type != filter_type { continue; }
                            }
                            if self.check_trigger_conditions(trigger, caster) {
                                let ctx = EffectContext {
                                    source: Some(*entity_id),
                                    owner: *spawner_id,
                                    attacker: None,
                                    killer: None,
                                };
                                self.fire_trigger_effects(trigger, caster, &ctx);
                            }
                        }
                    }
                }
                "entity_damaged" => {
                    for event in &tick_events {
                        if let SimEvent::EntityDamaged { entity_id, attacker_id, .. } = event {
                            if let Some(ref filter_type) = trigger.filter.entity_type {
                                let types = entity_types_map.get(entity_id).map(|v| v.as_slice()).unwrap_or(&[]);
                                if !types.iter().any(|t| t == filter_type) { continue; }
                            }
                            if self.check_trigger_conditions(trigger, caster) {
                                let owner = self.get_entity(*entity_id).and_then(|e| e.owner);
                                let ctx = EffectContext {
                                    source: Some(*entity_id),
                                    attacker: *attacker_id,
                                    owner,
                                    killer: None,
                                };
                                self.fire_trigger_effects(trigger, caster, &ctx);
                            }
                        }
                    }
                }
                "resource_changed" => {
                    let default_ctx = EffectContext::default();
                    // Compare current resource values to snapshot to detect changes.
                    for (name, &old_value) in resource_snapshot.iter() {
                        let new_value = self.get_resource(name);
                        if old_value != new_value {
                            if let Some(ref filter_res) = trigger.filter.resource {
                                if name != filter_res { continue; }
                            }
                            if self.check_trigger_conditions(trigger, caster) {
                                self.fire_trigger_effects(trigger, caster, &default_ctx);
                            }
                        }
                    }
                    // Also check for newly created resources (not in snapshot).
                    let new_resources: Vec<String> = self.resources.keys()
                        .filter(|name| !resource_snapshot.contains_key(*name))
                        .cloned()
                        .collect();
                    for name in &new_resources {
                        if let Some(ref filter_res) = trigger.filter.resource {
                            if name != filter_res { continue; }
                        }
                        if self.check_trigger_conditions(trigger, caster) {
                            self.fire_trigger_effects(trigger, caster, &default_ctx);
                        }
                    }
                }
                "command_used" => {
                    for event in &tick_events {
                        if let SimEvent::CommandUsed { entity_id: cmd_eid, command, .. } = event {
                            if let Some(ref filter_cmd) = trigger.filter.command {
                                if command != filter_cmd { continue; }
                            }
                            if self.check_trigger_conditions(trigger, caster) {
                                let owner = self.get_entity(*cmd_eid).and_then(|e| e.owner);
                                let ctx = EffectContext {
                                    source: Some(*cmd_eid),
                                    owner,
                                    attacker: None,
                                    killer: None,
                                };
                                self.fire_trigger_effects(trigger, caster, &ctx);
                            }
                        }
                    }
                }
                "tick_interval" => {
                    if let Some(interval) = trigger.filter.interval {
                        if interval > 0 && self.tick % interval as u64 == 0 {
                            if self.check_trigger_conditions(trigger, caster) {
                                self.fire_trigger_effects(trigger, caster, &EffectContext::default());
                            }
                        }
                    }
                }
                "channel_completed" => {
                    for event in &tick_events {
                        if let SimEvent::ChannelCompleted { entity_id: ch_eid, command, .. } = event {
                            if let Some(ref filter_cmd) = trigger.filter.command {
                                if command != filter_cmd { continue; }
                            }
                            if self.check_trigger_conditions(trigger, caster) {
                                let owner = self.get_entity(*ch_eid).and_then(|e| e.owner);
                                let ctx = EffectContext {
                                    source: Some(*ch_eid),
                                    owner,
                                    attacker: None,
                                    killer: None,
                                };
                                self.fire_trigger_effects(trigger, caster, &ctx);
                            }
                        }
                    }
                }
                "channel_interrupted" => {
                    for event in &tick_events {
                        if let SimEvent::ChannelInterrupted { entity_id: int_eid, command, .. } = event {
                            if let Some(ref filter_cmd) = trigger.filter.command {
                                if command != filter_cmd { continue; }
                            }
                            if self.check_trigger_conditions(trigger, caster) {
                                let owner = self.get_entity(*int_eid).and_then(|e| e.owner);
                                let ctx = EffectContext {
                                    source: Some(*int_eid),
                                    owner,
                                    attacker: None,
                                    killer: None,
                                };
                                self.fire_trigger_effects(trigger, caster, &ctx);
                            }
                        }
                    }
                }
                _ => {} // Unknown event type — skip silently (validated at load time).
            }
        }
    }

    /// Check all conditions on a trigger against current world state.
    fn check_trigger_conditions(&self, trigger: &TriggerDef, caster: EntityId) -> bool {
        if trigger.conditions.is_empty() {
            return true;
        }
        let mut rng = SimRng::new(self.tick_seed() ^ caster.0 as u64);
        trigger.conditions.iter().all(|c| evaluate_condition(c, self, caster, &mut rng))
    }

    /// Fire a trigger's effects against the world with scoped effect context.
    fn fire_trigger_effects(&mut self, trigger: &TriggerDef, caster: EntityId, ctx: &EffectContext) {
        let mut events = Vec::new();
        resolve_custom_effects_with_ctx(
            self, caster, "trigger", &trigger.effects, &[], &mut events, ctx,
        );
        self.events.extend(events);
    }

    /// Handle an instant action (Print).
    /// Returns `None` if the action was handled (instant), `Some(action)` if it
    /// should be collected as a tick-consuming action.
    fn try_handle_instant(
        &mut self,
        eid: EntityId,
        action: UnitAction,
        _script_state: &mut crate::entity::ScriptState,
    ) -> Option<UnitAction> {
        match action {
            UnitAction::Print { text } => {
                self.events.push(SimEvent::ScriptOutput {
                    entity_id: eid,
                    text,
                });
                None
            }
            other => Some(other),
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
/// Provides the same operations that TOML effects use, but as an explicit API
/// that Lua can call. The `events` buffer collects SimEvents generated during
/// the handler invocation.
pub struct WorldAccess<'a> {
    world: &'a mut SimWorld,
    pub caster_id: EntityId,
    pub events: Vec<SimEvent>,
    tick_seed: u64,
}

impl<'a> WorldAccess<'a> {
    /// Create a WorldAccess from a raw pointer to SimWorld.
    /// This is used in the tick loop where we already have &mut SimWorld
    /// but need to pass it through to the command handler.
    ///
    /// # Safety
    /// The caller must ensure that the SimWorld pointer is valid for the
    /// lifetime of this WorldAccess, and that no other mutable references
    /// to the SimWorld exist while this WorldAccess is in use (except via
    /// the command_handler field which is temporarily taken out).
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

    /// List commands, matching the TOML ListCommands effect behavior.
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

    /// Register a no-op "wait" custom command so `ActionCustom("wait")` works in tests.
    fn register_wait_command(world: &mut SimWorld) {
        world.register_custom_command(&CommandDef {
            name: "wait".into(),
            description: "wait one tick".into(),
            args: vec![],
            effects: vec![],
            unlisted: true,
            phases: vec![],
            ..Default::default()
        });
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

    /// Helper: collect all ScriptOutput texts from events.
    fn output_texts(events: &[SimEvent]) -> Vec<String> {
        events.iter().filter_map(|e| match e {
            SimEvent::ScriptOutput { text, .. } => Some(text.clone()),
            _ => None,
        }).collect()
    }

    #[test]
    fn phased_command_three_phases() {
        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        let id = spawn_test_entity(&mut world, "summoner", "s", 0);

        // Register a phased command: 2-tick phase 0, 1-tick phase 1, 1-tick phase 2.
        let def = CommandDef {
            name: "spell".into(),
            description: "test spell".into(),
            args: vec![],
            effects: vec![],
            unlisted: false,
            phases: vec![
                PhaseDef {
                    ticks: 2,
                    interruptible: false,
                    per_update: vec![CommandEffect::Output { message: "phase0-tick".into() }],
                    update_interval: 1,
                    on_start: vec![CommandEffect::Output { message: "phase0-start".into() }],
                },
                PhaseDef {
                    ticks: 1,
                    interruptible: false,
                    per_update: vec![],
                    update_interval: 1,
                    on_start: vec![CommandEffect::Output { message: "phase1-start".into() }],
                },
                PhaseDef {
                    ticks: 1,
                    interruptible: false,
                    per_update: vec![CommandEffect::Output { message: "phase2-tick".into() }],
                    update_interval: 1,
                    on_start: vec![],
                },
            ],
            ..Default::default()
        };
        world.register_custom_command(&def);

        // Script: call spell(), then wait forever.
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("spell".into()),
                // After channel completes, loop wait.
                Instruction::ActionCustom("wait".into()),
                Instruction::Jump(1),
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: script calls spell() → channel set up (no effects yet).
        world.tick();
        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(texts.is_empty(), "No effects on initiation tick: {:?}", texts);
        assert!(world.get_entity(id).unwrap().active_channel.is_some());

        // Tick 2: phase 0, tick 0 → on_start + per_update.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"phase0-start".to_string()), "on_start runs first tick: {:?}", texts);
        assert!(texts.contains(&"phase0-tick".to_string()), "per_update runs first tick: {:?}", texts);

        // Tick 3: phase 0, tick 1 → per_update only (no on_start).
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(!texts.contains(&"phase0-start".to_string()), "on_start should not repeat: {:?}", texts);
        assert!(texts.contains(&"phase0-tick".to_string()), "per_update runs: {:?}", texts);

        // Tick 4: phase 1, tick 0 → on_start.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"phase1-start".to_string()), "phase1 on_start: {:?}", texts);

        // Tick 5: phase 2, tick 0 → per_update.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"phase2-tick".to_string()), "phase2 per_update: {:?}", texts);

        // Channel should be done now.
        assert!(world.get_entity(id).unwrap().active_channel.is_none());
    }

    #[test]
    fn phased_command_use_global_resource_failure_cancels() {
        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        let id = spawn_test_entity(&mut world, "summoner", "s", 0);
        // Set mana resource to 15 — enough for 1 tick of 10 drain but not 2.
        world.resources.insert("mana".into(), 15);

        let def = CommandDef {
            name: "drain".into(),
            description: "".into(),
            args: vec![],
            effects: vec![],
            unlisted: false,
            phases: vec![
                PhaseDef {
                    ticks: 3,
                    interruptible: false,
                    per_update: vec![
                        CommandEffect::UseGlobalResource { resource: "mana".into(), amount: crate::action::DynInt::Fixed(10) },
                        CommandEffect::Output { message: "drained".into() },
                    ],
                    update_interval: 1,
                    on_start: vec![],
                },
                PhaseDef {
                    ticks: 1,
                    interruptible: false,
                    per_update: vec![],
                    update_interval: 1,
                    on_start: vec![CommandEffect::Output { message: "should not reach".into() }],
                },
            ],
            ..Default::default()
        };
        world.register_custom_command(&def);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("drain".into()),
                Instruction::ActionCustom("wait".into()),
                Instruction::Jump(1),
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: initiation.
        world.tick();
        world.take_events();

        // Tick 2: phase 0, tick 0 — drains 10 mana (15 → 5). Should succeed.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"drained".to_string()), "First drain should succeed: {:?}", texts);
        assert_eq!(world.get_resource("mana"), 5);

        // Tick 3: phase 0, tick 1 — needs 10 energy but only 5 → abort.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.iter().any(|t| t.contains("not enough")), "Should fail: {:?}", texts);
        assert!(!texts.contains(&"drained".to_string()), "Should not drain: {:?}", texts);
        // Channel should be cancelled.
        assert!(world.get_entity(id).unwrap().active_channel.is_none());

        // Tick 4: no more channel — "should not reach" never fires.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(!texts.contains(&"should not reach".to_string()));
    }

    #[test]
    fn phased_command_update_interval() {
        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        let id = spawn_test_entity(&mut world, "summoner", "s", 0);

        // Phase with update_interval = 2 over 4 ticks.
        // per_update fires when (ticks_elapsed + 1) % interval == 0,
        // i.e., at elapsed 1 and 3 (not 0 and 2).
        let def = CommandDef {
            name: "pulse".into(),
            description: "".into(),
            args: vec![],
            effects: vec![],
            unlisted: false,
            phases: vec![
                PhaseDef {
                    ticks: 4,
                    interruptible: false,
                    per_update: vec![CommandEffect::Output { message: "pulse".into() }],
                    update_interval: 2,
                    on_start: vec![],
                },
            ],
            ..Default::default()
        };
        world.register_custom_command(&def);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("pulse".into()),
                Instruction::ActionCustom("wait".into()),
                Instruction::Jump(1),
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: initiation — no effects.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.is_empty(), "No effects on initiation tick: {:?}", texts);

        // Tick 2: ticks_elapsed=0, (0+1) % 2 != 0 → skipped.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert_eq!(texts.iter().filter(|t| *t == "pulse").count(), 0, "Should skip at elapsed 0: {:?}", texts);

        // Tick 3: ticks_elapsed=1, (1+1) % 2 == 0 → fires.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert_eq!(texts.iter().filter(|t| *t == "pulse").count(), 1, "Should fire at elapsed 1: {:?}", texts);

        // Tick 4: ticks_elapsed=2, (2+1) % 2 != 0 → skipped.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert_eq!(texts.iter().filter(|t| *t == "pulse").count(), 0, "Should skip at elapsed 2: {:?}", texts);

        // Tick 5: ticks_elapsed=3, (3+1) % 2 == 0 → fires.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert_eq!(texts.iter().filter(|t| *t == "pulse").count(), 1, "Should fire at elapsed 3: {:?}", texts);

        // Channel should be done now.
        assert!(world.get_entity(id).unwrap().active_channel.is_none());
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

    // --- Conditional effects tests ---

    #[test]
    fn if_effect_takes_then_branch_when_condition_true() {
        use crate::action::{CommandDef, CommandEffect, Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        world.resources.insert("mana".into(), 20);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);

        let cmd = CommandDef {
            name: "condtest".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::If {
                condition: Condition::Resource {
                    resource: "mana".into(),
                    compare: CompareOp::Gte,
                    amount: DynInt::Fixed(20),
                },
                then_effects: vec![CommandEffect::Output { message: "has mana".into() }],
                otherwise: vec![CommandEffect::Output { message: "no mana".into() }],
            }],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("condtest".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"has mana".to_string()), "Should take then branch: {:?}", texts);
        assert!(!texts.contains(&"no mana".to_string()));
    }

    #[test]
    fn if_effect_takes_else_branch_when_condition_false() {
        use crate::action::{CommandDef, CommandEffect, Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        world.resources.insert("mana".into(), 5);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);

        let cmd = CommandDef {
            name: "condtest".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::If {
                condition: Condition::Resource {
                    resource: "mana".into(),
                    compare: CompareOp::Gte,
                    amount: DynInt::Fixed(20),
                },
                then_effects: vec![CommandEffect::Output { message: "has mana".into() }],
                otherwise: vec![CommandEffect::Output { message: "no mana".into() }],
            }],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("condtest".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"no mana".to_string()), "Should take else branch: {:?}", texts);
        assert!(!texts.contains(&"has mana".to_string()));
    }

    #[test]
    fn if_effect_entity_count_condition() {
        use crate::action::{CommandDef, CommandEffect, Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);
        let _sk1 = spawn_test_entity(&mut world, "skeleton", "sk1", 100);
        let _sk2 = spawn_test_entity(&mut world, "skeleton", "sk2", 200);

        let cmd = CommandDef {
            name: "counttest".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::If {
                condition: Condition::EntityCount {
                    entity_type: "skeleton".into(),
                    compare: CompareOp::Gte,
                    amount: DynInt::Fixed(2),
                },
                then_effects: vec![CommandEffect::Output { message: "enough skeletons".into() }],
                otherwise: vec![CommandEffect::Output { message: "need more".into() }],
            }],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("counttest".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"enough skeletons".to_string()), "Should match 2 skeletons: {:?}", texts);
    }

    #[test]
    fn if_effect_stat_condition() {
        use crate::action::{CommandDef, CommandEffect, Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);
        // Default health is 100.

        let cmd = CommandDef {
            name: "stattest".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::If {
                condition: Condition::Stat {
                    stat: "health".into(),
                    compare: CompareOp::Gt,
                    amount: DynInt::Fixed(50),
                },
                then_effects: vec![CommandEffect::Output { message: "healthy".into() }],
                otherwise: vec![CommandEffect::Output { message: "injured".into() }],
            }],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("stattest".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"healthy".to_string()), "Health 100 > 50: {:?}", texts);
    }

    #[test]
    fn if_effect_abort_propagates_from_branch() {
        use crate::action::{CommandDef, CommandEffect, Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        world.resources.insert("mana".into(), 100);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);

        // use_global_resource inside then branch should abort the entire effect list.
        let cmd = CommandDef {
            name: "abort_test".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::If {
                    condition: Condition::Resource {
                        resource: "mana".into(),
                        compare: CompareOp::Gte,
                        amount: DynInt::Fixed(1),
                    },
                    then_effects: vec![
                        CommandEffect::UseGlobalResource { resource: "mana".into(), amount: DynInt::Fixed(999) },
                    ],
                    otherwise: vec![],
                },
                CommandEffect::Output { message: "should not reach".into() },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("abort_test".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(!texts.contains(&"should not reach".to_string()), "Abort should propagate: {:?}", texts);
    }

    #[test]
    fn start_channel_from_effects_creates_channel() {
        use crate::action::{CommandDef, CommandEffect, PhaseDef};

        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);

        let cmd = CommandDef {
            name: "inline_channel".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::Output { message: "before channel".into() },
                CommandEffect::StartChannel {
                    phases: vec![PhaseDef {
                        ticks: 2,
                        interruptible: false,
                        on_start: vec![CommandEffect::Output { message: "phase-start".into() }],
                        per_update: vec![CommandEffect::Output { message: "phase-tick".into() }],
                        update_interval: 1,
                    }],
                },
                // This should not run — start_channel returns immediately.
                CommandEffect::Output { message: "after channel".into() },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("inline_channel".into()),
                Instruction::ActionCustom("wait".into()),
                Instruction::Jump(1),
            ],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: script calls inline_channel() → effects run, start_channel sets up channel.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"before channel".to_string()), "Effects before start_channel should run: {:?}", texts);
        assert!(!texts.contains(&"after channel".to_string()), "Effects after start_channel should not run: {:?}", texts);
        assert!(world.get_entity(caster).unwrap().active_channel.is_some(), "Channel should be active");

        // Tick 2: phase 0, tick 0 → on_start + per_update.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"phase-start".to_string()));
        assert!(texts.contains(&"phase-tick".to_string()));

        // Tick 3: phase 0, tick 1 → per_update only.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(!texts.contains(&"phase-start".to_string()));
        assert!(texts.contains(&"phase-tick".to_string()));

        // Channel should be done now.
        assert!(world.get_entity(caster).unwrap().active_channel.is_none());
    }

    #[test]
    fn conditional_start_channel_picks_branch() {
        use crate::action::{CommandDef, CommandEffect, Condition, CompareOp, DynInt, PhaseDef};

        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        world.resources.insert("mana".into(), 50);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);

        // If mana >= 20: spend mana and start a channel.
        // Else: just output a message.
        let cmd = CommandDef {
            name: "branch_channel".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::If {
                condition: Condition::Resource {
                    resource: "mana".into(),
                    compare: CompareOp::Gte,
                    amount: DynInt::Fixed(20),
                },
                then_effects: vec![
                    CommandEffect::UseGlobalResource { resource: "mana".into(), amount: DynInt::Fixed(20) },
                    CommandEffect::StartChannel {
                        phases: vec![PhaseDef {
                            ticks: 1,
                            interruptible: false,
                            on_start: vec![CommandEffect::Output { message: "channeling!".into() }],
                            per_update: vec![],
                            update_interval: 1,
                        }],
                    },
                ],
                otherwise: vec![CommandEffect::Output { message: "not enough mana!".into() }],
            }],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("branch_channel".into()),
                Instruction::ActionCustom("wait".into()),
                Instruction::Jump(1),
            ],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: condition true → spend 20 mana, start channel.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(!texts.contains(&"not enough mana!".to_string()));
        assert_eq!(world.get_resource("mana"), 30);
        assert!(world.get_entity(caster).unwrap().active_channel.is_some());

        // Tick 2: channel phase runs.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"channeling!".to_string()));
        assert!(world.get_entity(caster).unwrap().active_channel.is_none());
    }

    #[test]
    fn nested_if_effects() {
        use crate::action::{CommandDef, CommandEffect, Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        world.resources.insert("mana".into(), 30);
        let caster = spawn_test_entity(&mut world, "summoner", "s", 500);
        let _sk = spawn_test_entity(&mut world, "skeleton", "sk1", 100);

        let cmd = CommandDef {
            name: "nested".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::If {
                condition: Condition::Resource {
                    resource: "mana".into(),
                    compare: CompareOp::Gte,
                    amount: DynInt::Fixed(10),
                },
                then_effects: vec![CommandEffect::If {
                    condition: Condition::EntityCount {
                        entity_type: "skeleton".into(),
                        compare: CompareOp::Gt,
                        amount: DynInt::Fixed(0),
                    },
                    then_effects: vec![CommandEffect::Output { message: "mana+skeletons".into() }],
                    otherwise: vec![CommandEffect::Output { message: "mana only".into() }],
                }],
                otherwise: vec![CommandEffect::Output { message: "no mana".into() }],
            }],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("nested".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"mana+skeletons".to_string()), "Nested conditions: {:?}", texts);
    }

    // ---------------------------------------------------------------
    // Trigger system tests
    // ---------------------------------------------------------------

    #[test]
    fn trigger_tick_interval_fires_periodically() {
        use crate::action::{TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);
        spawn_test_entity(&mut world, "summoner", "summoner", 500);

        // Trigger fires every 5 ticks.
        world.register_trigger(TriggerDef {
            event: "tick_interval".into(),
            filter: TriggerFilter {
                interval: Some(5),
                ..Default::default()
            },
            conditions: vec![],
            effects: vec![
                CommandEffect::Output { message: "interval!".into() },
            ],
        });

        world.start();

        // Tick 1-4: no trigger
        let mut fired_count = 0;
        for _ in 0..4 {
            world.tick();
            let events = world.take_events();
            if output_texts(&events).contains(&"interval!".to_string()) {
                fired_count += 1;
            }
        }
        assert_eq!(fired_count, 0, "Should not fire before tick 5");

        // Tick 5: trigger fires
        world.tick();
        let events = world.take_events();
        assert!(output_texts(&events).contains(&"interval!".to_string()),
            "Should fire at tick 5");

        // Tick 10: fires again
        for _ in 0..4 {
            world.tick();
            world.take_events();
        }
        world.tick();
        let events = world.take_events();
        assert!(output_texts(&events).contains(&"interval!".to_string()),
            "Should fire at tick 10");
    }

    #[test]
    fn trigger_resource_changed_detects_modification() {
        use crate::action::{TriggerDef, TriggerFilter, DynInt};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        world.resources.insert("gold".into(), 0);

        // Register a custom command that modifies gold.
        world.register_custom_command(&CommandDef {
            name: "earn".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::ModifyResource { resource: "gold".into(), amount: DynInt::Fixed(10) },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });

        // Trigger: when gold changes, output a message.
        world.register_trigger(TriggerDef {
            event: "resource_changed".into(),
            filter: TriggerFilter {
                resource: Some("gold".into()),
                ..Default::default()
            },
            conditions: vec![],
            effects: vec![
                CommandEffect::Output { message: "Gold changed!".into() },
            ],
        });

        // Script: earn(), halt
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("earn".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.custom_command_arg_counts.insert("earn".into(), 0);
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(texts.contains(&"Gold changed!".to_string()),
            "Should detect gold change, got: {:?}", texts);
    }

    #[test]
    fn trigger_with_conditions_only_fires_when_met() {
        use crate::action::{Condition, CompareOp, TriggerDef, TriggerFilter, DynInt};

        let mut world = SimWorld::new(42);
        spawn_test_entity(&mut world, "summoner", "summoner", 500);
        world.resources.insert("souls".into(), 5);

        // Trigger: on tick_interval(1), but only if souls >= 10.
        world.register_trigger(TriggerDef {
            event: "tick_interval".into(),
            filter: TriggerFilter {
                interval: Some(1),
                ..Default::default()
            },
            conditions: vec![
                Condition::Resource {
                    resource: "souls".into(),
                    compare: CompareOp::Gte,
                    amount: DynInt::Fixed(10),
                },
            ],
            effects: vec![
                CommandEffect::Output { message: "Souls threshold!".into() },
            ],
        });

        world.start();

        // Tick 1: souls=5 < 10, should NOT fire.
        world.tick();
        let events = world.take_events();
        assert!(!output_texts(&events).contains(&"Souls threshold!".to_string()),
            "Should not fire when souls < 10");

        // Set souls to 15, tick again.
        *world.resources.get_mut("souls").unwrap() = 15;
        world.tick();
        let events = world.take_events();
        assert!(output_texts(&events).contains(&"Souls threshold!".to_string()),
            "Should fire when souls >= 10");
    }

    #[test]
    fn trigger_command_used_fires_on_custom_command() {
        use crate::action::{TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        // Register a simple custom command.
        world.register_custom_command(&CommandDef {
            name: "meditate".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::Output { message: "Meditating...".into() },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });

        // Trigger: when "meditate" is used, output a bonus message.
        world.register_trigger(TriggerDef {
            event: "command_used".into(),
            filter: TriggerFilter {
                command: Some("meditate".into()),
                ..Default::default()
            },
            conditions: vec![],
            effects: vec![
                CommandEffect::Output { message: "Bonus from trigger!".into() },
            ],
        });

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("meditate".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.custom_command_arg_counts.insert("meditate".into(), 0);
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(texts.contains(&"Meditating...".to_string()));
        assert!(texts.contains(&"Bonus from trigger!".to_string()),
            "Command trigger should fire, got: {:?}", texts);
    }

    // ---------------------------------------------------------------
    // Buff system tests
    // ---------------------------------------------------------------

    #[test]
    fn buff_apply_modifies_stats_and_expires() {
        use crate::action::BuffDef;
        use indexmap::IndexMap as StdMap;

        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        // Register a buff that adds 5 speed for 3 ticks.
        world.register_buff(BuffDef {
            name: "haste".into(),
            duration: 3,
            modifiers: { let mut m = StdMap::new(); m.insert("speed".into(), 5); m },
            per_tick: vec![],
            on_apply: vec![],
            on_expire: vec![CommandEffect::Output { message: "Haste expired!".into() }],
            stackable: false,
            max_stacks: 0,
        });

        let base_speed = world.get_entity(summoner).unwrap().stat("speed");

        // Apply buff via custom command.
        world.register_custom_command(&CommandDef {
            name: "cast_haste".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::ApplyBuff { target: "self".into(), buff: "haste".into(), duration: None },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });
        world.custom_command_arg_counts.insert("cast_haste".into(), 0);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("cast_haste".into()),
                // Wait forever after casting (don't re-cast on implicit loop restart).
                Instruction::ActionCustom("wait".into()),
                Instruction::Jump(1),
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick(); // Apply buff
        world.take_events();

        // Speed should be increased.
        assert_eq!(world.get_entity(summoner).unwrap().stat("speed"), base_speed + 5,
            "Buff should increase speed");

        // Buff was applied in tick 1 (remaining=3), then buff tick decrements to 2.
        // Tick 2: remaining 2→1. Tick 3: remaining 1→0 → expires.
        world.tick(); world.take_events(); // tick 2
        world.tick();                       // tick 3: remaining=0 → expires

        let events = world.take_events();
        let texts = output_texts(&events);

        // Speed should be back to base.
        assert_eq!(world.get_entity(summoner).unwrap().stat("speed"), base_speed,
            "Buff expiry should reverse speed modifier");
        assert!(texts.contains(&"Haste expired!".to_string()),
            "on_expire effects should fire, got: {:?}", texts);
    }

    #[test]
    fn buff_stackable_adds_multiple_stacks() {
        use crate::action::BuffDef;
        use indexmap::IndexMap as StdMap;

        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        world.register_buff(BuffDef {
            name: "rage".into(),
            duration: 10,
            modifiers: { let mut m = StdMap::new(); m.insert("attack_damage".into(), 3); m },
            per_tick: vec![],
            on_apply: vec![],
            on_expire: vec![],
            stackable: true,
            max_stacks: 5,
        });

        let base_dmg = world.get_entity(summoner).unwrap().stat("attack_damage");

        // Apply buff twice.
        world.register_custom_command(&CommandDef {
            name: "rage_up".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::ApplyBuff { target: "self".into(), buff: "rage".into(), duration: None },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });
        world.custom_command_arg_counts.insert("rage_up".into(), 0);

        // Script: rage_up(), wait, rage_up(), halt
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("rage_up".into()),
                Instruction::ActionCustom("wait".into()),
                Instruction::ActionCustom("rage_up".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick(); world.take_events(); // Apply first stack
        world.tick(); world.take_events(); // Wait
        world.tick(); world.take_events(); // Apply second stack

        assert_eq!(world.get_entity(summoner).unwrap().stat("attack_damage"), base_dmg + 6,
            "Two stacks should add 6 attack damage");
        assert_eq!(world.get_entity(summoner).unwrap().active_buffs[0].stacks, 2);
    }

    #[test]
    fn buff_non_stackable_refreshes_duration() {
        use crate::action::BuffDef;

        let mut world = SimWorld::new(42);
        register_wait_command(&mut world);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        world.register_buff(BuffDef {
            name: "shield_up".into(),
            duration: 5,
            modifiers: indexmap::IndexMap::new(),
            per_tick: vec![],
            on_apply: vec![],
            on_expire: vec![],
            stackable: false,
            max_stacks: 0,
        });

        world.register_custom_command(&CommandDef {
            name: "shield_cast".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::ApplyBuff { target: "self".into(), buff: "shield_up".into(), duration: None },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });
        world.custom_command_arg_counts.insert("shield_cast".into(), 0);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("shield_cast".into()),
                Instruction::ActionCustom("wait".into()),
                Instruction::ActionCustom("wait".into()),
                Instruction::ActionCustom("shield_cast".into()), // Refresh at tick 4
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick(); world.take_events(); // tick 1: apply (remaining=5)
        world.tick(); world.take_events(); // tick 2: wait (remaining=4 after buff tick)
        world.tick(); world.take_events(); // tick 3: wait (remaining=3)
        world.tick(); world.take_events(); // tick 4: re-apply → refreshes to 5

        let remaining = world.get_entity(summoner).unwrap().active_buffs[0].remaining_ticks;
        // After re-apply at tick 4, remaining is set to 5, then buff tick decrements to 4
        assert_eq!(remaining, 4, "Non-stackable re-apply should refresh duration");
    }

    #[test]
    fn buff_remove_reverses_modifiers() {
        use crate::action::BuffDef;
        use indexmap::IndexMap as StdMap;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        world.register_buff(BuffDef {
            name: "armor".into(),
            duration: 100,
            modifiers: { let mut m = StdMap::new(); m.insert("shield".into(), 20); m },
            per_tick: vec![],
            on_apply: vec![],
            on_expire: vec![],
            stackable: false,
            max_stacks: 0,
        });

        world.register_custom_command(&CommandDef {
            name: "armor_on".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::ApplyBuff { target: "self".into(), buff: "armor".into(), duration: None },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });
        world.register_custom_command(&CommandDef {
            name: "armor_off".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::RemoveBuff { target: "self".into(), buff: "armor".into() },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });
        world.custom_command_arg_counts.insert("armor_on".into(), 0);
        world.custom_command_arg_counts.insert("armor_off".into(), 0);

        let base_shield = world.get_entity(summoner).unwrap().stat("shield");

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("armor_on".into()),
                Instruction::ActionCustom("armor_off".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick(); world.take_events(); // Apply armor
        assert_eq!(world.get_entity(summoner).unwrap().stat("shield"), base_shield + 20);

        world.tick(); world.take_events(); // Remove armor
        assert_eq!(world.get_entity(summoner).unwrap().stat("shield"), base_shield,
            "remove_buff should reverse shield modifier");
        assert!(world.get_entity(summoner).unwrap().active_buffs.is_empty());
    }

    #[test]
    fn condition_has_buff_works() {
        use crate::action::{BuffDef, Condition};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        world.register_buff(BuffDef {
            name: "berserk".into(),
            duration: 10,
            modifiers: indexmap::IndexMap::new(),
            per_tick: vec![],
            on_apply: vec![],
            on_expire: vec![],
            stackable: false,
            max_stacks: 0,
        });

        // has_buff should be false initially.
        let mut rng = crate::rng::SimRng::new(42);
        assert!(!evaluate_condition(
            &Condition::HasBuff { buff: "berserk".into() },
            &world, summoner, &mut rng
        ));

        // Apply buff.
        world.get_entity_mut(summoner).unwrap().active_buffs.push(
            crate::entity::ActiveBuff { name: "berserk".into(), remaining_ticks: 10, stacks: 1 }
        );

        assert!(evaluate_condition(
            &Condition::HasBuff { buff: "berserk".into() },
            &world, summoner, &mut rng
        ));
    }

    #[test]
    fn condition_random_chance_is_deterministic() {
        use crate::action::Condition;

        let world = SimWorld::new(42);
        let eid = EntityId(1); // doesn't need to exist for random_chance

        // Run the same seed twice — should get the same result.
        let mut rng1 = crate::rng::SimRng::new(99);
        let result1 = evaluate_condition(
            &Condition::RandomChance { percent: 50 },
            &world, eid, &mut rng1
        );
        let mut rng2 = crate::rng::SimRng::new(99);
        let result2 = evaluate_condition(
            &Condition::RandomChance { percent: 50 },
            &world, eid, &mut rng2
        );
        assert_eq!(result1, result2, "Same seed should produce same random result");
    }

    #[test]
    fn condition_compound_and_or() {
        use crate::action::{Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        world.resources.insert("gold".into(), 50);
        world.resources.insert("mana".into(), 10);
        let mut rng = crate::rng::SimRng::new(42);

        // AND: gold >= 30 AND mana >= 5 → true
        let and_cond = Condition::And {
            conditions: vec![
                Condition::Resource { resource: "gold".into(), compare: CompareOp::Gte, amount: DynInt::Fixed(30) },
                Condition::Resource { resource: "mana".into(), compare: CompareOp::Gte, amount: DynInt::Fixed(5) },
            ],
        };
        assert!(evaluate_condition(&and_cond, &world, summoner, &mut rng));

        // AND: gold >= 30 AND mana >= 20 → false (mana too low)
        let and_fail = Condition::And {
            conditions: vec![
                Condition::Resource { resource: "gold".into(), compare: CompareOp::Gte, amount: DynInt::Fixed(30) },
                Condition::Resource { resource: "mana".into(), compare: CompareOp::Gte, amount: DynInt::Fixed(20) },
            ],
        };
        assert!(!evaluate_condition(&and_fail, &world, summoner, &mut rng));

        // OR: gold >= 100 OR mana >= 5 → true (mana passes)
        let or_cond = Condition::Or {
            conditions: vec![
                Condition::Resource { resource: "gold".into(), compare: CompareOp::Gte, amount: DynInt::Fixed(100) },
                Condition::Resource { resource: "mana".into(), compare: CompareOp::Gte, amount: DynInt::Fixed(5) },
            ],
        };
        assert!(evaluate_condition(&or_cond, &world, summoner, &mut rng));
    }

    // ---------------------------------------------------------------
    // Custom stats tests (Phase 5)
    // ---------------------------------------------------------------

    #[test]
    fn custom_stat_modify_and_condition() {
        use crate::action::{Condition, CompareOp, DynInt};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        // Register a command that modifies a stat.
        world.register_custom_command(&CommandDef {
            name: "train".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::ModifyStat {
                    target: "self".into(),
                    stat: "armor".into(),
                    amount: DynInt::Fixed(5),
                },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });
        world.custom_command_arg_counts.insert("train".into(), 0);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("train".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();
        world.take_events();

        // Stat should be set.
        assert_eq!(world.get_entity(summoner).unwrap().stat("armor"), 5);

        // Condition should work.
        let mut rng = crate::rng::SimRng::new(42);
        assert!(evaluate_condition(
            &Condition::Stat {
                stat: "armor".into(),
                compare: CompareOp::Gte,
                amount: DynInt::Fixed(5),
            },
            &world, summoner, &mut rng,
        ));
        assert!(!evaluate_condition(
            &Condition::Stat {
                stat: "armor".into(),
                compare: CompareOp::Gte,
                amount: DynInt::Fixed(10),
            },
            &world, summoner, &mut rng,
        ));
    }

    #[test]
    fn custom_stat_use_aborts_when_insufficient() {
        use crate::action::DynInt;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        // Set initial stat.
        world.get_entity_mut(summoner).unwrap().set_stat("mojo", 3);

        // Command that requires 5 mojo (entity only has 3).
        world.register_custom_command(&CommandDef {
            name: "power_move".into(),
            description: "".into(),
            args: vec![],
            effects: vec![
                CommandEffect::UseResource { stat: "mojo".into(), amount: DynInt::Fixed(5) },
                CommandEffect::Output { message: "Should NOT appear".into() },
            ],
            phases: vec![],
            unlisted: false,
            ..Default::default()
        });
        world.custom_command_arg_counts.insert("power_move".into(), 0);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("power_move".into()), Instruction::Halt],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(texts.iter().any(|t| t.contains("not enough mojo")),
            "Should abort with insufficient custom stat, got: {:?}", texts);
        assert!(!texts.contains(&"Should NOT appear".to_string()));
        // Mojo should not be deducted.
        assert_eq!(world.get_entity(summoner).unwrap().stat("mojo"), 3);
    }

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

    // ---------------------------------------------------------------
    // Computed values tests (Phase 6)
    // ---------------------------------------------------------------

    #[test]
    fn dynint_entity_count_resolves() {
        use crate::action::DynInt;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        spawn_test_entity(&mut world, "skeleton", "s1", 100);
        spawn_test_entity(&mut world, "skeleton", "s2", 200);
        spawn_test_entity(&mut world, "zombie", "z1", 300);

        let mut rng = crate::rng::SimRng::new(42);

        let dyn_val = DynInt::EntityCount { entity_type: "skeleton".into(), multiplier: 1 };
        assert_eq!(dyn_val.resolve_with_world(&mut rng, &world, summoner), 2);

        let dyn_val_mult = DynInt::EntityCount { entity_type: "skeleton".into(), multiplier: 3 };
        assert_eq!(dyn_val_mult.resolve_with_world(&mut rng, &world, summoner), 6);
    }

    #[test]
    fn dynint_resource_value_resolves() {
        use crate::action::DynInt;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        world.resources.insert("gold".into(), 42);

        let mut rng = crate::rng::SimRng::new(42);
        let dyn_val = DynInt::ResourceValue { resource: "gold".into(), multiplier: 2 };
        assert_eq!(dyn_val.resolve_with_world(&mut rng, &world, summoner), 84);
    }

    #[test]
    fn dynint_caster_stat_resolves() {
        use crate::action::DynInt;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        let mut rng = crate::rng::SimRng::new(42);
        let health = world.get_entity(summoner).unwrap().stat("health");
        let dyn_val = DynInt::CasterStat { stat: "health".into(), multiplier: 1 };
        assert_eq!(dyn_val.resolve_with_world(&mut rng, &world, summoner), health);
    }

    #[test]
    fn dynint_caster_custom_stat_resolves() {
        use crate::action::DynInt;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        world.get_entity_mut(summoner).unwrap().set_stat("armor", 7);

        let mut rng = crate::rng::SimRng::new(42);
        let dyn_val = DynInt::CasterStat { stat: "armor".into(), multiplier: 3 };
        assert_eq!(dyn_val.resolve_with_world(&mut rng, &world, summoner), 21);
    }

    // -----------------------------------------------------------------------
    // Contains / NotContains executor tests
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // evaluate_condition with game-state DynInt
    // -----------------------------------------------------------------------

    #[test]
    fn condition_resource_with_dynint_entity_count() {
        use crate::action::{Condition, CompareOp, DynInt, evaluate_condition};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        spawn_test_entity(&mut world, "skeleton", "s1", 100);
        spawn_test_entity(&mut world, "skeleton", "s2", 200);
        world.resources.insert("souls".into(), 5);

        let mut rng = crate::rng::SimRng::new(42);

        // Resource condition: souls (5) >= entity_count(skeleton)*2 (2*2=4) => true
        let cond = Condition::Resource {
            resource: "souls".into(),
            compare: CompareOp::Gte,
            amount: DynInt::EntityCount { entity_type: "skeleton".into(), multiplier: 2 },
        };
        assert!(evaluate_condition(&cond, &world, summoner, &mut rng));
    }

    #[test]
    fn condition_stat_with_dynint_resource_value() {
        use crate::action::{Condition, CompareOp, DynInt, evaluate_condition};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        // summoner health defaults to 100
        world.resources.insert("mana".into(), 50);

        let mut rng = crate::rng::SimRng::new(42);

        // Stat condition: health (100) > resource(mana)*1 (50) => true
        let cond = Condition::Stat {
            stat: "health".into(),
            compare: CompareOp::Gt,
            amount: DynInt::ResourceValue { resource: "mana".into(), multiplier: 1 },
        };
        assert!(evaluate_condition(&cond, &world, summoner, &mut rng));

        // Flip: health (100) < resource(mana)*1 (50) => false
        let cond2 = Condition::Stat {
            stat: "health".into(),
            compare: CompareOp::Lt,
            amount: DynInt::ResourceValue { resource: "mana".into(), multiplier: 1 },
        };
        assert!(!evaluate_condition(&cond2, &world, summoner, &mut rng));
    }

    #[test]
    fn condition_entity_count_with_dynint_caster_stat() {
        use crate::action::{Condition, CompareOp, DynInt, evaluate_condition};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        world.get_entity_mut(summoner).unwrap().set_stat("army_size", 3);
        spawn_test_entity(&mut world, "skeleton", "s1", 100);
        spawn_test_entity(&mut world, "skeleton", "s2", 200);

        let mut rng = crate::rng::SimRng::new(42);

        // EntityCount condition: skeleton count (2) < stat(army_size)*1 (3) => true
        let cond = Condition::EntityCount {
            entity_type: "skeleton".into(),
            compare: CompareOp::Lt,
            amount: DynInt::CasterStat { stat: "army_size".into(), multiplier: 1 },
        };
        assert!(evaluate_condition(&cond, &world, summoner, &mut rng));
    }

    // -----------------------------------------------------------------------
    // nearest() deterministic tie-breaking
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

    // --- Scoped target tests ---

    #[test]
    fn spawn_effect_sets_owner() {
        use crate::action::{CommandDef, CommandEffect, DynInt};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 0);

        world.entity_configs.insert("skeleton".into(), EntityConfig {
            stats: IndexMap::from([("health".into(), 10), ("max_health".into(), 10)]),
        });

        let spawn_cmd = CommandDef {
            name: "raise".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::Spawn {
                entity_id: "skeleton".into(),
                offset: DynInt::Fixed(3),
            }],
            unlisted: false,
            phases: vec![],
            ..Default::default()
        };
        world.register_custom_command(&spawn_cmd);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("raise".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        // Find the spawned skeleton.
        let skel = world.entities()
            .find(|e| e.entity_type == "skeleton")
            .expect("skeleton should be spawned");

        assert_eq!(skel.owner, Some(summoner), "Spawned entity should have owner set to spawner");
    }

    // --- is_alive and distance condition tests ---

    #[test]
    fn is_alive_condition_true_for_alive_entity() {
        use crate::action::{Condition, EffectContext};
        use crate::rng::SimRng;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "s", 0);
        let skel = spawn_test_entity(&mut world, "skeleton", "sk", 10);

        let mut rng = SimRng::new(42);
        let ctx = EffectContext::default();
        let args = vec![SimValue::EntityRef(skel)];

        let cond = Condition::IsAlive { target: "arg:0".into() };
        assert!(crate::action::evaluate_condition_with_ctx(
            &cond, &world, summoner, &mut rng, &args, &ctx,
        ));

        // Kill the skeleton.
        world.get_entity_mut(skel).unwrap().alive = false;
        assert!(!crate::action::evaluate_condition_with_ctx(
            &cond, &world, summoner, &mut rng, &args, &ctx,
        ));
    }

    #[test]
    fn is_alive_condition_with_self_target() {
        use crate::action::{Condition, EffectContext};
        use crate::rng::SimRng;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "s", 0);

        let mut rng = SimRng::new(42);
        let cond = Condition::IsAlive { target: "self".into() };
        assert!(crate::action::evaluate_condition_with_ctx(
            &cond, &world, summoner, &mut rng, &[], &EffectContext::default(),
        ));
    }

    #[test]
    fn distance_condition_compares_correctly() {
        use crate::action::{Condition, CompareOp, DynInt, EffectContext};
        use crate::rng::SimRng;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "s", 0);
        let skel = spawn_test_entity(&mut world, "skeleton", "sk", 30);

        let mut rng = SimRng::new(42);
        let args = vec![SimValue::EntityRef(skel)];
        let ctx = EffectContext::default();

        // distance is 30, check <= 50 → true
        let cond = Condition::Distance {
            target: "arg:0".into(),
            compare: CompareOp::Lte,
            amount: DynInt::Fixed(50),
        };
        assert!(crate::action::evaluate_condition_with_ctx(
            &cond, &world, summoner, &mut rng, &args, &ctx,
        ));

        // distance is 30, check <= 20 → false
        let cond2 = Condition::Distance {
            target: "arg:0".into(),
            compare: CompareOp::Lte,
            amount: DynInt::Fixed(20),
        };
        assert!(!crate::action::evaluate_condition_with_ctx(
            &cond2, &world, summoner, &mut rng, &args, &ctx,
        ));

        // distance is 30, check == 30 → true
        let cond3 = Condition::Distance {
            target: "arg:0".into(),
            compare: CompareOp::Eq,
            amount: DynInt::Fixed(30),
        };
        assert!(crate::action::evaluate_condition_with_ctx(
            &cond3, &world, summoner, &mut rng, &args, &ctx,
        ));
    }

    #[test]
    fn is_alive_condition_with_scoped_source_target() {
        use crate::action::{Condition, EffectContext};
        use crate::rng::SimRng;

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "s", 0);
        let skel = spawn_test_entity(&mut world, "skeleton", "sk", 10);

        let mut rng = SimRng::new(42);
        let ctx = EffectContext {
            source: Some(skel),
            ..Default::default()
        };

        let cond = Condition::IsAlive { target: "source".into() };
        assert!(crate::action::evaluate_condition_with_ctx(
            &cond, &world, summoner, &mut rng, &[], &ctx,
        ));
    }

    #[test]
    fn if_effect_with_distance_condition() {
        use crate::action::{CommandDef, CommandEffect, CompareOp, Condition, DynInt};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "s", 0);
        let skel = spawn_test_entity(&mut world, "skeleton", "sk", 3);

        // Command: if distance to arg:target <= 5 then output "close" else output "far"
        let cmd = CommandDef {
            name: "check_dist".into(),
            description: "".into(),
            args: vec!["target".into()],
            effects: vec![CommandEffect::If {
                condition: Condition::Distance {
                    target: "arg:0".into(),
                    compare: CompareOp::Lte,
                    amount: DynInt::Fixed(5),
                },
                then_effects: vec![CommandEffect::Output { message: "close".into() }],
                otherwise: vec![CommandEffect::Output { message: "far".into() }],
            }],
            unlisted: false,
            phases: vec![],
            ..Default::default()
        };
        world.register_custom_command(&cmd);

        // Script: check_dist(skeleton_ref)
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(skel)),
                Instruction::ActionCustom("check_dist".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"close".into()), "Distance 3 <= 5 should be 'close': {:?}", texts);
    }
}
