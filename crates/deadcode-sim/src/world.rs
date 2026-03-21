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
        self.custom_command_arg_counts.insert(def.name.clone(), def.args.len());
        self.custom_commands.insert(def.name.clone(), def.effects.clone());
        if !def.phases.is_empty() {
            self.custom_command_phases.insert(def.name.clone(), def.phases.clone());
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
            0,
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
                                        Ok(None) => break,
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
                        // Main brain halted.
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
                let mut channel = self.get_entity_mut(eid).unwrap().active_channel.take().unwrap();
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
                                                Ok(None) => break,
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
                            Ok(None) => {} // script halted, no interruption
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
                        entity.active_channel = Some(channel);
                    }
                } else {
                    // Channel done — entity resumes normal script execution next tick.
                    self.events.push(SimEvent::ChannelCompleted {
                        entity_id: eid,
                        command: channel.command_name.clone(),
                    });
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
                                Ok(None) => break,
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
                    // Script halted or finished.
                    self.events.push(SimEvent::ScriptFinished {
                        entity_id: eid,
                        success: true,
                        error: None,
                    });
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

    /// Handle an instant action (Print, GainResource, TrySpendResource).
    /// Returns `None` if the action was handled (instant), `Some(action)` if it
    /// should be collected as a tick-consuming action.
    fn try_handle_instant(
        &mut self,
        eid: EntityId,
        action: UnitAction,
        script_state: &mut crate::entity::ScriptState,
    ) -> Option<UnitAction> {
        match action {
            UnitAction::Print { text } => {
                self.events.push(SimEvent::ScriptOutput {
                    entity_id: eid,
                    text,
                });
                None
            }
            UnitAction::GainResource { name, amount } => {
                let new_total = self.gain_resource(&name, amount);
                script_state.stack.push(crate::value::SimValue::Int(new_total));
                None
            }
            UnitAction::TrySpendResource { name, amount } => {
                let success = self.try_spend_resource(&name, amount);
                script_state.stack.push(crate::value::SimValue::Bool(success));
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
    fn tick_moves_unit() {
        let mut world = SimWorld::new(42);
        let id = spawn_test_entity(&mut world, "skeleton", "miner1", 0);

        // Give it a script: move(100), halt
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Int(100)),
                Instruction::ActionMove,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let entity = world.get_entity(id).unwrap();
        // Should have moved by speed (default 1) toward 100.
        assert!(entity.position > 0);
    }

    #[test]
    fn determinism() {
        fn run_sim(seed: u64) -> SimSnapshot {
            let mut world = SimWorld::new(seed);

            let m1 = spawn_test_entity(&mut world, "skeleton", "m1", 0);
            let m2 = spawn_test_entity(&mut world, "skeleton", "m2", 500);
            let _ast = spawn_test_entity(&mut world, "grave", "rock", 250);

            // Script: while True: move(250)
            let program = CompiledScript::new(
                vec![
                    Instruction::LoadConst(SimValue::Bool(true)),
                    Instruction::JumpIfFalse(4),
                    Instruction::LoadConst(SimValue::Int(250)),
                    Instruction::ActionMove,
                    Instruction::Jump(0),
                    Instruction::Halt,
                ],
                0,
            );

            for id in [m1, m2] {
                world.get_entity_mut(id).unwrap().script_state =
                    Some(ScriptState::new(program.clone(), 0));
            }

            world.start();
            for _ in 0..100 {
                world.tick();
            }

            world.snapshot()
        }

        let a = run_sim(42);
        let b = run_sim(42);

        assert_eq!(a.tick, b.tick);
        assert_eq!(a.entities.len(), b.entities.len());
        for (ea, eb) in a.entities.iter().zip(b.entities.iter()) {
            assert_eq!(ea.id, eb.id);
            assert_eq!(ea.position, eb.position);
            assert_eq!(ea.health, eb.health);
        }
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
        };
        world.register_custom_command(&def);

        // Script: call spell(), then wait forever.
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("spell".into()),
                // After channel completes, loop wait.
                Instruction::ActionWait,
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
    fn phased_command_interruptible_cancelled_by_action() {
        let mut world = SimWorld::new(42);
        let id = spawn_test_entity(&mut world, "summoner", "s", 0);

        // Register phased command with interruptible phase.
        let def = CommandDef {
            name: "channel".into(),
            description: "".into(),
            args: vec![],
            effects: vec![],
            unlisted: false,
            phases: vec![
                PhaseDef {
                    ticks: 5,
                    interruptible: true,
                    per_update: vec![CommandEffect::Output { message: "channeling".into() }],
                    update_interval: 1,
                    on_start: vec![],
                },
            ],
        };
        world.register_custom_command(&def);

        // Script: call channel(), then immediately move(100) (which interrupts).
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("channel".into()),
                // Next tick (during interruptible phase), script runs and yields move.
                Instruction::LoadConst(SimValue::Int(100)),
                Instruction::ActionMove,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: script calls channel() → channel set up.
        world.tick();
        world.take_events();

        // Tick 2: interruptible phase — script yields move(100) → interrupt.
        world.tick();
        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(texts.iter().any(|t| t.contains("interrupted")), "Should emit interrupted: {:?}", texts);
        // Channel should be gone.
        assert!(world.get_entity(id).unwrap().active_channel.is_none());
        // Entity should have moved.
        assert!(world.get_entity(id).unwrap().position > 0);
    }

    #[test]
    fn phased_command_use_global_resource_failure_cancels() {
        let mut world = SimWorld::new(42);
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
        };
        world.register_custom_command(&def);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("drain".into()),
                Instruction::ActionWait,
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
    fn phased_command_non_interruptible_blocks_script() {
        let mut world = SimWorld::new(42);
        let id = spawn_test_entity(&mut world, "summoner", "s", 0);

        let def = CommandDef {
            name: "lock".into(),
            description: "".into(),
            args: vec![],
            effects: vec![],
            unlisted: false,
            phases: vec![
                PhaseDef {
                    ticks: 2,
                    interruptible: false,
                    per_update: vec![],
                    update_interval: 1,
                    on_start: vec![CommandEffect::Output { message: "locked".into() }],
                },
            ],
        };
        world.register_custom_command(&def);

        // Script: call lock(), then move(100).
        // During non-interruptible phase, script should NOT execute.
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("lock".into()),
                Instruction::LoadConst(SimValue::Int(100)),
                Instruction::ActionMove,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: initiation.
        world.tick();
        world.take_events();
        assert_eq!(world.get_entity(id).unwrap().position, 0);

        // Tick 2: non-interruptible phase tick 0 — script blocked.
        world.tick();
        world.take_events();
        assert_eq!(world.get_entity(id).unwrap().position, 0, "Script should be blocked");

        // Tick 3: non-interruptible phase tick 1 — still blocked.
        world.tick();
        world.take_events();
        assert_eq!(world.get_entity(id).unwrap().position, 0, "Script should still be blocked");

        // Tick 4: channel done — script resumes, moves.
        world.tick();
        world.take_events();
        assert!(world.get_entity(id).unwrap().position > 0, "Script should resume and move");
    }

    #[test]
    fn phased_command_update_interval() {
        let mut world = SimWorld::new(42);
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
        };
        world.register_custom_command(&def);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("pulse".into()),
                Instruction::ActionWait,
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

    #[test]
    fn resource_via_script_get_resource() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 42);
        let id = spawn_test_entity(&mut world, "skeleton", "test", 0);

        // Script: print(get_resource("souls")); halt
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Str("souls".into())),
                Instruction::QueryGetResource,
                Instruction::Print,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert_eq!(texts, vec!["42"]);
    }

    #[test]
    fn resource_via_script_gain_and_spend() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 10);
        let id = spawn_test_entity(&mut world, "skeleton", "test", 0);

        // Script: gain_resource("souls", 5); try_spend_resource("souls", 20); wait
        // gain_resource should return 15 (pushed to stack, then popped by Pop)
        // try_spend_resource should return false (15 < 20)
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Str("souls".into())),
                Instruction::LoadConst(SimValue::Int(5)),
                Instruction::InstantGainResource,
                Instruction::Pop, // discard return value
                Instruction::LoadConst(SimValue::Str("souls".into())),
                Instruction::LoadConst(SimValue::Int(20)),
                Instruction::InstantTrySpendResource,
                Instruction::Print, // prints "false"
                Instruction::ActionWait,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert_eq!(texts, vec!["False"]);
        assert_eq!(world.get_resource("souls"), 15); // gain worked, spend failed
    }

    #[test]
    fn resource_via_script_successful_spend() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 10);
        let id = spawn_test_entity(&mut world, "skeleton", "test", 0);

        // Script: try_spend_resource("souls", 3); print result; wait
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Str("souls".into())),
                Instruction::LoadConst(SimValue::Int(3)),
                Instruction::InstantTrySpendResource,
                Instruction::Print, // prints "true"
                Instruction::ActionWait,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert_eq!(texts, vec!["True"]);
        assert_eq!(world.get_resource("souls"), 7);
    }

    #[test]
    fn resource_in_conditional_flow() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 5);
        let id = spawn_test_entity(&mut world, "skeleton", "test", 0);

        // Script:
        //   result = try_spend_resource("souls", 3)  // true, souls -> 2
        //   if result:
        //     print("spent")
        //   else:
        //     print("broke")
        //   wait
        let program = CompiledScript::new(
            vec![
                // try_spend_resource("souls", 3)
                Instruction::LoadConst(SimValue::Str("souls".into())),
                Instruction::LoadConst(SimValue::Int(3)),
                Instruction::InstantTrySpendResource,
                // store result in var 1 (var 0 is self)
                Instruction::StoreVar(1),
                // if result:
                Instruction::LoadVar(1),
                Instruction::JumpIfFalse(9),
                // print("spent")
                Instruction::LoadConst(SimValue::Str("spent".into())),
                Instruction::Print,
                Instruction::Jump(11),
                // else: print("broke")
                Instruction::LoadConst(SimValue::Str("broke".into())),
                Instruction::Print,
                // wait
                Instruction::ActionWait,
            ],
            2,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 2));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"spent".to_string()), "Should have spent: {:?}", texts);
        assert_eq!(world.get_resource("souls"), 2);
    }

    // --- Sacrifice effect tests ---

    #[test]
    fn sacrifice_effect_kills_matching_entities_and_gains_resource() {
        use crate::action::{CommandDef, CommandEffect, DynInt};

        let mut world = SimWorld::new(42);
        world.resources.insert("bones".into(), 0);

        // Spawn a summoner (caster) and 3 skeletons.
        let caster = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        let _sk1 = spawn_test_entity(&mut world, "skeleton", "sk1", 100);
        let _sk2 = spawn_test_entity(&mut world, "skeleton", "sk2", 200);
        let _sk3 = spawn_test_entity(&mut world, "skeleton", "sk3", 300);

        // Register the harvest command with sacrifice effect.
        let harvest = CommandDef {
            name: "harvest".into(),
            description: "sacrifice skeletons".into(),
            args: vec![],
            effects: vec![CommandEffect::Sacrifice {
                entity_type: "skeleton".into(),
                resource: "bones".into(),
                per_kill: DynInt::Fixed(2),
            }],
            unlisted: false,
            phases: vec![],
        };
        world.register_custom_command(&harvest);

        // Give the caster a script: harvest(); halt
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("harvest".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();

        // All 3 skeletons should have died.
        let deaths: Vec<_> = events.iter()
            .filter(|e| matches!(e, SimEvent::EntityDied { .. }))
            .collect();
        assert_eq!(deaths.len(), 3, "Expected 3 deaths, got {}", deaths.len());

        // With Fixed(2) per kill and 3 kills, bones should be 6.
        assert_eq!(world.get_resource("bones"), 6);

        // Should have a summary output.
        let texts = output_texts(&events);
        assert!(
            texts.iter().any(|t| t.contains("Sacrificed 3")),
            "Expected sacrifice summary, got {:?}", texts
        );
    }

    #[test]
    fn sacrifice_effect_nothing_to_sacrifice() {
        use crate::action::{CommandDef, CommandEffect, DynInt};

        let mut world = SimWorld::new(42);
        world.resources.insert("bones".into(), 0);

        let caster = spawn_test_entity(&mut world, "summoner", "summoner", 500);

        let harvest = CommandDef {
            name: "harvest".into(),
            description: "sacrifice skeletons".into(),
            args: vec![],
            effects: vec![CommandEffect::Sacrifice {
                entity_type: "skeleton".into(),
                resource: "bones".into(),
                per_kill: DynInt::Fixed(1),
            }],
            unlisted: false,
            phases: vec![],
        };
        world.register_custom_command(&harvest);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("harvest".into()),
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(caster).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(
            texts.iter().any(|t| t.contains("Nothing to sacrifice")),
            "Expected nothing-to-sacrifice message, got {:?}", texts
        );
        assert_eq!(world.get_resource("bones"), 0);
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

    #[test]
    fn unavailable_resource_get_errors_in_script() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 42);
        world.available_resources = Some(["bones".to_string()].into_iter().collect());
        let id = spawn_test_entity(&mut world, "skeleton", "test", 0);

        // Script: get_resource("souls"); halt — should error because "souls" is not available.
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Str("souls".into())),
                Instruction::QueryGetResource,
                Instruction::Halt,
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

    #[test]
    fn unavailable_resource_gain_errors_in_script() {
        let mut world = SimWorld::new(42);
        world.resources.insert("souls".into(), 10);
        world.available_resources = Some(["bones".to_string()].into_iter().collect());
        let id = spawn_test_entity(&mut world, "skeleton", "test", 0);

        // Script: gain_resource("souls", 5); wait — should error.
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Str("souls".into())),
                Instruction::LoadConst(SimValue::Int(5)),
                Instruction::InstantGainResource,
                Instruction::ActionWait,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        assert!(events.iter().any(|e| matches!(e, SimEvent::ScriptError { .. })));
        assert_eq!(world.get_resource("souls"), 10); // unchanged
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
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("inline_channel".into()),
                Instruction::ActionWait,
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
        };
        world.register_custom_command(&cmd);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("branch_channel".into()),
                Instruction::ActionWait,
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

    #[test]
    fn available_resource_works_normally() {
        let mut world = SimWorld::new(42);
        world.resources.insert("bones".into(), 0);
        world.available_resources = Some(["bones".to_string()].into_iter().collect());
        let id = spawn_test_entity(&mut world, "skeleton", "test", 0);

        // Script: gain_resource("bones", 5); print(get_resource("bones")); wait
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Str("bones".into())),
                Instruction::LoadConst(SimValue::Int(5)),
                Instruction::InstantGainResource,
                Instruction::Pop, // discard return value
                Instruction::LoadConst(SimValue::Str("bones".into())),
                Instruction::QueryGetResource,
                Instruction::Print,
                Instruction::ActionWait,
            ],
            0,
        );
        world.get_entity_mut(id).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let texts = output_texts(&world.take_events());
        assert_eq!(texts, vec!["5"]);
        assert_eq!(world.get_resource("bones"), 5);
    }

    // ---------------------------------------------------------------
    // Trigger system tests
    // ---------------------------------------------------------------

    #[test]
    fn trigger_entity_died_fires_effects() {
        use crate::action::{TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        let skeleton = spawn_test_entity(&mut world, "skeleton", "skel1", 502);

        // Register a trigger: when a skeleton dies, output a message.
        world.register_trigger(TriggerDef {
            event: "entity_died".into(),
            filter: TriggerFilter {
                entity_type: Some("skeleton".into()),
                ..Default::default()
            },
            conditions: vec![],
            effects: vec![
                CommandEffect::Output { message: "A skeleton has fallen!".into() },
            ],
        });

        // Give summoner a script that attacks the skeleton.
        // First, set skeleton health low so one attack kills it.
        world.get_entity_mut(skeleton).unwrap().set_stat("health", 1);

        // Give summoner a script: attack(skeleton), halt
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(skeleton)),
                Instruction::ActionAttack,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(texts.contains(&"A skeleton has fallen!".to_string()),
            "Expected trigger output, got: {:?}", texts);
    }

    #[test]
    fn trigger_entity_died_filter_ignores_wrong_type() {
        use crate::action::{TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        let zombie = spawn_test_entity(&mut world, "zombie", "z1", 502);

        // Trigger only fires for skeleton deaths, not zombie deaths.
        world.register_trigger(TriggerDef {
            event: "entity_died".into(),
            filter: TriggerFilter {
                entity_type: Some("skeleton".into()),
                ..Default::default()
            },
            conditions: vec![],
            effects: vec![
                CommandEffect::Output { message: "Should NOT appear".into() },
            ],
        });

        world.get_entity_mut(zombie).unwrap().set_stat("health", 1);
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(zombie)),
                Instruction::ActionAttack,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        let events = world.take_events();
        let texts = output_texts(&events);
        assert!(!texts.contains(&"Should NOT appear".to_string()),
            "Trigger should not fire for zombie death, got: {:?}", texts);
    }

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

    #[test]
    fn trigger_entity_died_with_spawn_effect() {
        use crate::action::{TriggerDef, TriggerFilter, DynInt};

        let mut world = SimWorld::new(42);
        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 500);
        let skeleton = spawn_test_entity(&mut world, "skeleton", "skel1", 502);

        // When a skeleton dies, spawn a ghost.
        world.register_trigger(TriggerDef {
            event: "entity_died".into(),
            filter: TriggerFilter {
                entity_type: Some("skeleton".into()),
                ..Default::default()
            },
            conditions: vec![],
            effects: vec![
                CommandEffect::Spawn { entity_type: "ghost".into(), offset: DynInt::Fixed(0) },
            ],
        });

        world.get_entity_mut(skeleton).unwrap().set_stat("health", 1);
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(skeleton)),
                Instruction::ActionAttack,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        // The ghost should be queued for spawn (pending). It won't be flushed
        // because trigger effects fire after flush_pending. Check pending_spawns.
        // Actually, trigger effects call queue_spawn which adds to pending_spawns.
        // These will be flushed next tick.
        world.tick(); // flush the pending spawn

        let ghost_count = world.entities()
            .filter(|e| e.entity_type == "ghost" && e.alive)
            .count();
        assert_eq!(ghost_count, 1, "Trigger should have spawned a ghost");
    }

    // ---------------------------------------------------------------
    // Buff system tests
    // ---------------------------------------------------------------

    #[test]
    fn buff_apply_modifies_stats_and_expires() {
        use crate::action::BuffDef;
        use indexmap::IndexMap as StdMap;

        let mut world = SimWorld::new(42);
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
        });
        world.custom_command_arg_counts.insert("cast_haste".into(), 0);

        let program = CompiledScript::new(
            vec![Instruction::ActionCustom("cast_haste".into()), Instruction::Halt],
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
        });
        world.custom_command_arg_counts.insert("rage_up".into(), 0);

        // Script: rage_up(), wait, rage_up(), halt
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("rage_up".into()),
                Instruction::ActionWait,
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
        });
        world.custom_command_arg_counts.insert("shield_cast".into(), 0);

        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("shield_cast".into()),
                Instruction::ActionWait,
                Instruction::ActionWait,
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

    #[test]
    fn nearest_tiebreak_lower_entity_id_wins() {
        use crate::query::nearest;

        let mut world = SimWorld::new(42);
        // Querier at position 100
        let querier = spawn_test_entity(&mut world, "summoner", "me", 100);
        // Two entities equidistant at 90 and 110 (both distance 10 from querier)
        let e_left = spawn_test_entity(&mut world, "skeleton", "left", 90);
        let e_right = spawn_test_entity(&mut world, "skeleton", "right", 110);

        let result = nearest(&world, querier, "skeleton");
        // The one with the lower entity ID should win the tie.
        let expected_winner = if e_left.0 < e_right.0 { e_left } else { e_right };
        assert_eq!(result, SimValue::EntityRef(expected_winner));
    }

    #[test]
    fn nearest_tiebreak_reversed_spawn_order() {
        use crate::query::nearest;

        let mut world = SimWorld::new(99);
        // Spawn far entity first, then close, then another at same distance as close.
        let querier = spawn_test_entity(&mut world, "summoner", "me", 0);
        let _far = spawn_test_entity(&mut world, "zombie", "far", 100);
        let close_a = spawn_test_entity(&mut world, "zombie", "a", 50);
        let _close_b = spawn_test_entity(&mut world, "zombie", "b", -50); // abs(-50 - 0) = 50

        let result = nearest(&world, querier, "zombie");
        // close_a and close_b are equidistant (50). close_a has lower ID => wins.
        assert_eq!(result, SimValue::EntityRef(close_a));
    }

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
    fn trigger_killer_target_gets_heal() {
        use crate::action::{CommandEffect, DynInt, TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);

        // Spawn attacker (summoner) and a skeleton.
        let attacker = spawn_test_entity(&mut world, "summoner", "summoner", 0);
        let victim = spawn_test_entity(&mut world, "skeleton", "skel", 3);

        // Lower attacker's health so we can detect heal.
        world.get_entity_mut(attacker).unwrap().set_stat("health", 50);
        // Set skeleton health low enough to die from one hit.
        world.get_entity_mut(victim).unwrap().set_stat("health", 5);

        // Register a trigger: when a skeleton dies, heal the killer by 20.
        world.register_trigger(TriggerDef {
            event: "entity_died".into(),
            filter: TriggerFilter { entity_type: Some("skeleton".into()), ..Default::default() },
            conditions: vec![],
            effects: vec![CommandEffect::Heal {
                target: "killer".into(),
                amount: DynInt::Fixed(20),
            }],
        });

        // Script: attack the skeleton.
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(victim)),
                Instruction::ActionAttack,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(attacker).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        // Attacker should have been healed: 50 + 20 = 70.
        let attacker_health = world.get_entity(attacker).unwrap().stat("health");
        assert_eq!(attacker_health, 70, "Killer should have been healed to 70, got {}", attacker_health);
    }

    #[test]
    fn trigger_source_target_references_dead_entity_type() {
        use crate::action::{CommandEffect, DynInt, TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);

        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 0);
        let victim = spawn_test_entity(&mut world, "skeleton", "skel", 3);
        world.get_entity_mut(victim).unwrap().set_stat("health", 5);

        // Trigger on entity_died: modify_stat on source (the dead entity).
        // Since the entity is dead, this should silently no-op.
        world.register_trigger(TriggerDef {
            event: "entity_died".into(),
            filter: TriggerFilter { entity_type: Some("skeleton".into()), ..Default::default() },
            conditions: vec![],
            effects: vec![CommandEffect::ModifyStat {
                target: "source".into(),
                stat: "speed".into(),
                amount: DynInt::Fixed(10),
            }],
        });

        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(victim)),
                Instruction::ActionAttack,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        // Should not crash. The dead entity is removed, so the effect is a no-op.
    }

    #[test]
    fn trigger_owner_target_from_entity_field() {
        use crate::action::{CommandDef, CommandEffect, DynInt, TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);

        let summoner = spawn_test_entity(&mut world, "summoner", "summoner", 0);
        world.get_entity_mut(summoner).unwrap().set_stat("health", 50);

        // Register a spawn command (offset 3 = within attack range of 5).
        let spawn_cmd = CommandDef {
            name: "raise".into(),
            description: "".into(),
            args: vec![],
            effects: vec![CommandEffect::Spawn {
                entity_type: "skeleton".into(),
                offset: DynInt::Fixed(3),
            }],
            unlisted: false,
            phases: vec![],
        };
        world.register_custom_command(&spawn_cmd);

        // Set up entity config for skeletons.
        world.entity_configs.insert("skeleton".into(), EntityConfig {
            stats: IndexMap::from([
                ("health".into(), 5),
                ("max_health".into(), 5),
                ("speed".into(), 1),
                ("attack_damage".into(), 10),
                ("attack_range".into(), 5),
                ("attack_cooldown".into(), 3),
            ]),
        });

        // Trigger: when a skeleton dies, heal its owner by 10.
        world.register_trigger(TriggerDef {
            event: "entity_died".into(),
            filter: TriggerFilter { entity_type: Some("skeleton".into()), ..Default::default() },
            conditions: vec![],
            effects: vec![CommandEffect::Heal {
                target: "owner".into(),
                amount: DynInt::Fixed(10),
            }],
        });

        // Script: raise(); wait forever
        let program = CompiledScript::new(
            vec![
                Instruction::ActionCustom("raise".into()),
                Instruction::ActionWait,
                Instruction::Jump(1),
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();

        // Tick 1: summoner calls raise(), skeleton gets queued and spawned.
        world.tick();
        world.take_events();

        // Find the spawned skeleton.
        let skel_id = world.entities()
            .find(|e| e.entity_type == "skeleton")
            .map(|e| e.id)
            .expect("skeleton should exist");

        // Verify owner was set.
        assert_eq!(
            world.get_entity(skel_id).unwrap().owner,
            Some(summoner),
            "Skeleton should be owned by summoner"
        );

        // Set skeleton to 1 health so summoner can kill it.
        world.get_entity_mut(skel_id).unwrap().set_stat("health", 1);

        // Give summoner an attack script targeting the skeleton.
        let attack_program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(skel_id)),
                Instruction::ActionAttack,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(summoner).unwrap().script_state =
            Some(ScriptState::new(attack_program, 0));

        world.tick();

        // Summoner should have been healed: 50 + 10 = 60.
        let summoner_health = world.get_entity(summoner).unwrap().stat("health");
        assert_eq!(summoner_health, 60, "Owner should have been healed to 60, got {}", summoner_health);
    }

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
                entity_type: "skeleton".into(),
                offset: DynInt::Fixed(3),
            }],
            unlisted: false,
            phases: vec![],
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

    #[test]
    fn trigger_attacker_target_on_damage() {
        use crate::action::{CommandEffect, DynInt, TriggerDef, TriggerFilter};

        let mut world = SimWorld::new(42);

        let attacker = spawn_test_entity(&mut world, "summoner", "summoner", 0);
        let target = spawn_test_entity(&mut world, "skeleton", "skel", 3);

        // Register trigger: when a skeleton is damaged, modify_stat on attacker.
        world.register_trigger(TriggerDef {
            event: "entity_damaged".into(),
            filter: TriggerFilter { entity_type: Some("skeleton".into()), ..Default::default() },
            conditions: vec![],
            effects: vec![CommandEffect::ModifyStat {
                target: "attacker".into(),
                stat: "xp".into(),
                amount: DynInt::Fixed(5),
            }],
        });

        // Script: attack the skeleton.
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::EntityRef(target)),
                Instruction::ActionAttack,
                Instruction::Halt,
            ],
            0,
        );
        world.get_entity_mut(attacker).unwrap().script_state =
            Some(ScriptState::new(program, 0));

        world.start();
        world.tick();

        // Attacker should have gained 5 xp.
        let xp = world.get_entity(attacker).unwrap().stat("xp");
        assert_eq!(xp, 5, "Attacker should have 5 xp from trigger, got {}", xp);
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
