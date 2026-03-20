use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::action::{CommandDef, CommandEffect, PhaseDef, UnitAction, resolve_action, resolve_custom_effects};
use crate::entity::{EntityConfig, EntityId, SimEntity};
use crate::executor;
use crate::rng::SimRng;

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
    },
    EntityDied {
        entity_id: EntityId,
        name: String,
    },
    EntitySpawned {
        entity_id: EntityId,
        entity_type: String,
        name: String,
        position: i64,
    },
    ScriptOutput {
        entity_id: EntityId,
        text: String,
    },
    ScriptError {
        entity_id: EntityId,
        error: String,
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
    entity_index: HashMap<EntityId, usize>,
    next_entity_id: u64,
    pending_spawns: Vec<SimEntity>,
    pending_despawns: Vec<EntityId>,
    events: Vec<SimEvent>,
    running: bool,
    /// Custom command name → effects (populated from mod definitions).
    pub custom_commands: HashMap<String, Vec<CommandEffect>>,
    /// Custom command name → arg count (for the executor to know how many args to pop).
    pub custom_command_arg_counts: HashMap<String, usize>,
    /// Custom command name → description (for list_commands effect).
    pub custom_command_descriptions: HashMap<String, String>,
    /// Custom command name → phases (for phased/channeled commands).
    pub custom_command_phases: HashMap<String, Vec<PhaseDef>>,
    /// Entity type → stat overrides (for spawning from effects).
    pub entity_configs: HashMap<String, EntityConfig>,
    /// Entity type → spawn animation duration in ticks (0 = no spawn animation).
    pub spawn_durations: HashMap<String, i64>,
    /// Command display order (from available_commands insertion order).
    pub command_order: Vec<String>,
    /// Global resources shared across all entities.
    pub resources: IndexMap<String, i64>,
    /// Optional max values for resources. Absent = capless.
    pub resource_caps: HashMap<String, i64>,
    /// Available resource names. None = all available (dev mode).
    pub available_resources: Option<HashSet<String>>,
}

impl SimWorld {
    pub fn new(seed: u64) -> Self {
        Self {
            tick: 0,
            seed,
            entities: Vec::new(),
            entity_index: HashMap::new(),
            next_entity_id: 1,
            pending_spawns: Vec::new(),
            pending_despawns: Vec::new(),
            events: Vec::new(),
            running: false,
            custom_commands: HashMap::new(),
            custom_command_arg_counts: HashMap::new(),
            custom_command_descriptions: HashMap::new(),
            custom_command_phases: HashMap::new(),
            entity_configs: HashMap::new(),
            spawn_durations: HashMap::new(),
            command_order: Vec::new(),
            resources: IndexMap::new(),
            resource_caps: HashMap::new(),
            available_resources: None,
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

    /// Add to a global resource, returning the new total. Clamped to max if capped.
    pub fn gain_resource(&mut self, name: &str, amount: i64) -> i64 {
        let entry = self.resources.entry(name.to_string()).or_insert(0);
        *entry += amount;
        if let Some(&cap) = self.resource_caps.get(name) {
            *entry = (*entry).min(cap);
        }
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
                    name: e.name.clone(),
                    position: e.position,
                    health: e.health,
                    max_health: e.max_health,
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

        // 3. Collect scriptable entity IDs (including channeling entities), shuffle.
        //    Skip entities that are still spawning.
        let mut scriptable_ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|e| e.is_ready() && (e.script_state.is_some() || e.active_channel.is_some()))
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
                        match executor::execute_unit(eid, state, self) {
                            Ok(Some(action)) => {
                                // Instant actions don't interrupt — handle and keep going.
                                match self.try_handle_instant(eid, action, state) {
                                    None => {
                                        // Was instant. Continue executing in a loop.
                                        loop {
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
                                                    self.events.push(SimEvent::ScriptError {
                                                        entity_id: eid,
                                                        error: err.to_string(),
                                                    });
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
                                self.events.push(SimEvent::ScriptError {
                                    entity_id: eid,
                                    error: err.to_string(),
                                });
                            }
                        }

                        if state.step_limit_hit {
                            self.events.push(SimEvent::ScriptOutput {
                                entity_id: eid,
                                text: "[warning] Script exceeded step limit (10000 instructions) — auto-yielded".into(),
                            });
                        }
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
                    // Channel cancelled, entity proceeds with interrupting action.
                    continue;
                }

                // Run on_start effects (first tick of phase only).
                let mut channel_cancelled = false;
                if is_first_tick && !phase.on_start.is_empty() {
                    let mut effect_events = Vec::new();
                    let aborted = resolve_custom_effects(
                        self, eid, &channel.command_name,
                        &phase.on_start, &channel.args, &mut effect_events,
                    );
                    self.events.extend(effect_events);
                    if aborted {
                        channel_cancelled = true;
                    }
                }

                // Run per_tick effects.
                if !channel_cancelled && !phase.per_tick.is_empty() {
                    let mut effect_events = Vec::new();
                    let aborted = resolve_custom_effects(
                        self, eid, &channel.command_name,
                        &phase.per_tick, &channel.args, &mut effect_events,
                    );
                    self.events.extend(effect_events);
                    if aborted {
                        channel_cancelled = true;
                    }
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
                }
                // else: channel done, entity resumes normal script execution next tick.

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

            // Execute until action or halt.
            match executor::execute_unit(eid, &mut script_state, self) {
                Ok(Some(action)) => {
                    // Handle instant actions (Print, resource ops) — they don't
                    // consume the tick, so we handle them and re-enter the executor.
                    if let Some(real_action) = self.try_handle_instant(eid, action, &mut script_state) {
                        actions.push((eid, real_action));
                    } else {
                        // Instant action handled. Continue executing for the rest of the tick.
                        loop {
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
                                    self.events.push(SimEvent::ScriptError {
                                        entity_id: eid,
                                        error: err.to_string(),
                                    });
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
                    self.events.push(SimEvent::ScriptError {
                        entity_id: eid,
                        error: err.to_string(),
                    });
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
            if entity.cooldown_remaining > 0 {
                entity.cooldown_remaining -= 1;
            }
        }

        // 7. Flush pending spawns/despawns.
        for entity in self.pending_spawns.drain(..) {
            let id = entity.id;
            let etype = entity.entity_type.clone();
            let ename = entity.name.clone();
            let pos = entity.position;
            let index = self.entities.len();
            self.entities.push(entity);
            self.entity_index.insert(id, index);
            self.events.push(SimEvent::EntitySpawned {
                entity_id: id,
                entity_type: etype,
                name: ename,
                position: pos,
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
    use crate::entity::ScriptState;
    use crate::ir::{CompiledScript, Instruction};
    use crate::value::SimValue;

    #[test]
    fn spawn_and_query() {
        let mut world = SimWorld::new(42);
        let id = world.spawn_entity("skeleton".into(), "miner1".into(), 100);
        let entity = world.get_entity(id).unwrap();
        assert_eq!(entity.position, 100);
        assert_eq!(entity.name, "miner1");
    }

    #[test]
    fn tick_moves_unit() {
        let mut world = SimWorld::new(42);
        let id = world.spawn_entity("skeleton".into(), "miner1".into(), 0);

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

            let m1 = world.spawn_entity("skeleton".into(), "m1".into(), 0);
            let m2 = world.spawn_entity("skeleton".into(), "m2".into(), 500);
            let _ast = world.spawn_entity("grave".into(), "rock".into(), 250);

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
        world.spawn_entity("skeleton".into(), "m".into(), 0);
        world.tick(); // not started
        assert_eq!(world.tick, 0);
    }

    #[test]
    fn script_error_recorded() {
        let mut world = SimWorld::new(42);
        let id = world.spawn_entity("skeleton".into(), "m".into(), 0);

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
        let id = world.spawn_entity("summoner".into(), "s".into(), 0);

        // Register a phased command: 2-tick phase 0, 1-tick phase 1, 1-tick phase 2.
        let def = CommandDef {
            name: "spell".into(),
            description: "test spell".into(),
            args: vec![],
            effects: vec![],
            phases: vec![
                PhaseDef {
                    ticks: 2,
                    interruptible: false,
                    per_tick: vec![CommandEffect::Output { message: "phase0-tick".into() }],
                    on_start: vec![CommandEffect::Output { message: "phase0-start".into() }],
                },
                PhaseDef {
                    ticks: 1,
                    interruptible: false,
                    per_tick: vec![],
                    on_start: vec![CommandEffect::Output { message: "phase1-start".into() }],
                },
                PhaseDef {
                    ticks: 1,
                    interruptible: false,
                    per_tick: vec![CommandEffect::Output { message: "phase2-tick".into() }],
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

        // Tick 2: phase 0, tick 0 → on_start + per_tick.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"phase0-start".to_string()), "on_start runs first tick: {:?}", texts);
        assert!(texts.contains(&"phase0-tick".to_string()), "per_tick runs first tick: {:?}", texts);

        // Tick 3: phase 0, tick 1 → per_tick only (no on_start).
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(!texts.contains(&"phase0-start".to_string()), "on_start should not repeat: {:?}", texts);
        assert!(texts.contains(&"phase0-tick".to_string()), "per_tick runs: {:?}", texts);

        // Tick 4: phase 1, tick 0 → on_start.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"phase1-start".to_string()), "phase1 on_start: {:?}", texts);

        // Tick 5: phase 2, tick 0 → per_tick.
        world.tick();
        let texts = output_texts(&world.take_events());
        assert!(texts.contains(&"phase2-tick".to_string()), "phase2 per_tick: {:?}", texts);

        // Channel should be done now.
        assert!(world.get_entity(id).unwrap().active_channel.is_none());
    }

    #[test]
    fn phased_command_interruptible_cancelled_by_action() {
        let mut world = SimWorld::new(42);
        let id = world.spawn_entity("summoner".into(), "s".into(), 0);

        // Register phased command with interruptible phase.
        let def = CommandDef {
            name: "channel".into(),
            description: "".into(),
            args: vec![],
            effects: vec![],
            phases: vec![
                PhaseDef {
                    ticks: 5,
                    interruptible: true,
                    per_tick: vec![CommandEffect::Output { message: "channeling".into() }],
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
        let id = world.spawn_entity("summoner".into(), "s".into(), 0);
        // Set mana resource to 15 — enough for 1 tick of 10 drain but not 2.
        world.resources.insert("mana".into(), 15);

        let def = CommandDef {
            name: "drain".into(),
            description: "".into(),
            args: vec![],
            effects: vec![],
            phases: vec![
                PhaseDef {
                    ticks: 3,
                    interruptible: false,
                    per_tick: vec![
                        CommandEffect::UseGlobalResource { resource: "mana".into(), amount: crate::action::DynInt::Fixed(10) },
                        CommandEffect::Output { message: "drained".into() },
                    ],
                    on_start: vec![],
                },
                PhaseDef {
                    ticks: 1,
                    interruptible: false,
                    per_tick: vec![],
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
        let id = world.spawn_entity("summoner".into(), "s".into(), 0);

        let def = CommandDef {
            name: "lock".into(),
            description: "".into(),
            args: vec![],
            effects: vec![],
            phases: vec![
                PhaseDef {
                    ticks: 2,
                    interruptible: false,
                    per_tick: vec![],
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
        let id = world.spawn_entity("skeleton".into(), "test".into(), 0);

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
        let id = world.spawn_entity("skeleton".into(), "test".into(), 0);

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
        let id = world.spawn_entity("skeleton".into(), "test".into(), 0);

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
        let id = world.spawn_entity("skeleton".into(), "test".into(), 0);

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
        let caster = world.spawn_entity("summoner".into(), "summoner".into(), 500);
        let _sk1 = world.spawn_entity("skeleton".into(), "sk1".into(), 100);
        let _sk2 = world.spawn_entity("skeleton".into(), "sk2".into(), 200);
        let _sk3 = world.spawn_entity("skeleton".into(), "sk3".into(), 300);

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

        let caster = world.spawn_entity("summoner".into(), "summoner".into(), 500);

        let harvest = CommandDef {
            name: "harvest".into(),
            description: "sacrifice skeletons".into(),
            args: vec![],
            effects: vec![CommandEffect::Sacrifice {
                entity_type: "skeleton".into(),
                resource: "bones".into(),
                per_kill: DynInt::Fixed(1),
            }],
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
        let id = world.spawn_entity("skeleton".into(), "test".into(), 0);

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
        let id = world.spawn_entity("skeleton".into(), "test".into(), 0);

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

    #[test]
    fn available_resource_works_normally() {
        let mut world = SimWorld::new(42);
        world.resources.insert("bones".into(), 0);
        world.available_resources = Some(["bones".to_string()].into_iter().collect());
        let id = world.spawn_entity("skeleton".into(), "test".into(), 0);

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
}
