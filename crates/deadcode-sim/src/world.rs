use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::action::{CommandDef, CommandEffect, UnitAction, resolve_action};
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
    },
    EntitySpawned {
        entity_id: EntityId,
        entity_type: String,
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
    /// Entity type → stat overrides (for spawning from effects).
    pub entity_configs: HashMap<String, EntityConfig>,
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
            entity_configs: HashMap::new(),
        }
    }

    /// Register a custom command with its effects and arg count.
    pub fn register_custom_command(&mut self, def: &CommandDef) {
        self.custom_command_arg_counts.insert(def.name.clone(), def.args.len());
        self.custom_commands.insert(def.name.clone(), def.effects.clone());
    }

    /// Get the next entity ID (for pre-allocating IDs in effect resolution).
    pub fn next_entity_id(&mut self) -> u64 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
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

        // 3. Collect scriptable entity IDs, shuffle.
        let mut scriptable_ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|e| e.alive && e.script_state.is_some())
            .map(|e| e.id)
            .collect();
        rng.shuffle(&mut scriptable_ids);

        // 4. Execute each unit's script.
        let mut actions: Vec<(EntityId, UnitAction)> = Vec::new();

        for &eid in &scriptable_ids {
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
                    // Handle Print specially — it doesn't consume the tick,
                    // so we resolve it immediately and continue execution.
                    if matches!(action, UnitAction::Print { .. }) {
                        let print_events = resolve_action(self, eid, action);
                        self.events.extend(print_events);

                        // Continue executing after print (don't yield).
                        // We need to run the executor again for the rest of the tick.
                        loop {
                            match executor::execute_unit(eid, &mut script_state, self) {
                                Ok(Some(UnitAction::Print { text })) => {
                                    self.events.push(SimEvent::ScriptOutput {
                                        entity_id: eid,
                                        text,
                                    });
                                }
                                Ok(Some(action)) => {
                                    actions.push((eid, action));
                                    break;
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
                    } else {
                        actions.push((eid, action));
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
            let pos = entity.position;
            let index = self.entities.len();
            self.entities.push(entity);
            self.entity_index.insert(id, index);
            self.events.push(SimEvent::EntitySpawned {
                entity_id: id,
                entity_type: etype,
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
}
