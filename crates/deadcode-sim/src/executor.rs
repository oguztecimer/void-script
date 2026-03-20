use crate::action::UnitAction;
use crate::entity::{CallFrame, EntityId, ScriptState};
use crate::error::SimError;
use crate::ir::Instruction;
use crate::query;
use crate::value::SimValue;
use crate::world::SimWorld;

/// Maximum instructions a unit may execute per tick before auto-yielding.
const STEP_LIMIT: usize = 10_000;

/// Maximum call stack depth.
const MAX_CALL_DEPTH: usize = 256;

/// Execute one tick of a unit's script.
///
/// Returns `Ok(Some(action))` if the unit performed an action (tick consumed),
/// `Ok(None)` if the script halted or was already done, or `Err` on fatal error.
///
/// The caller takes `ScriptState` out of the entity before calling this, then
/// puts it back afterwards. The executor gets `&SimWorld` for read-only queries.
pub fn execute_unit(
    entity_id: EntityId,
    state: &mut ScriptState,
    world: &SimWorld,
) -> Result<Option<UnitAction>, SimError> {
    if state.error.is_some() || state.pc >= state.program.instructions.len() {
        return Ok(None);
    }

    state.yielded = false;
    state.step_limit_hit = false;
    let mut steps = 0usize;

    while state.pc < state.program.instructions.len() {
        steps += 1;
        if steps > STEP_LIMIT {
            state.yielded = true;
            state.step_limit_hit = true;
            return Ok(Some(UnitAction::Wait));
        }

        let inst = state.program.instructions[state.pc].clone();
        state.pc += 1;

        match inst {
            // --- Stack ops ---
            Instruction::LoadConst(val) => {
                state.stack.push(val);
            }
            Instruction::LoadVar(slot) => {
                let val = state
                    .variables
                    .get(slot)
                    .cloned()
                    .ok_or_else(|| SimError::invalid_variable(slot))?;
                state.stack.push(val);
            }
            Instruction::StoreVar(slot) => {
                let val = pop(&mut state.stack)?;
                if slot >= state.variables.len() {
                    state.variables.resize(slot + 1, SimValue::None);
                }
                state.variables[slot] = val;
            }
            Instruction::Pop => {
                pop(&mut state.stack)?;
            }
            Instruction::Dup => {
                let val = state
                    .stack
                    .last()
                    .cloned()
                    .ok_or_else(SimError::stack_underflow)?;
                state.stack.push(val);
            }

            // --- Arithmetic ---
            Instruction::Add => binary_op(&mut state.stack, |a, b| match (a, b) {
                (SimValue::Int(x), SimValue::Int(y)) => Ok(SimValue::Int(x.wrapping_add(y))),
                (SimValue::Str(x), SimValue::Str(y)) => Ok(SimValue::Str(format!("{x}{y}"))),
                (a, b) => Err(SimError::type_error(format!(
                    "cannot add {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            })?,
            Instruction::Sub => binary_int_op(&mut state.stack, "subtract", i64::wrapping_sub)?,
            Instruction::Mul => binary_int_op(&mut state.stack, "multiply", i64::wrapping_mul)?,
            Instruction::Div => {
                let b = pop_int(&mut state.stack)?;
                let a = pop_int(&mut state.stack)?;
                if b == 0 {
                    return Err(SimError::division_by_zero());
                }
                state.stack.push(SimValue::Int(floor_div(a, b)));
            }
            Instruction::Mod => {
                let b = pop_int(&mut state.stack)?;
                let a = pop_int(&mut state.stack)?;
                if b == 0 {
                    return Err(SimError::division_by_zero());
                }
                state.stack.push(SimValue::Int(floor_mod(a, b)));
            }
            Instruction::Negate => {
                let val = pop_int(&mut state.stack)?;
                state.stack.push(SimValue::Int(val.wrapping_neg()));
            }

            // --- Comparison ---
            Instruction::CmpEq => {
                let b = pop(&mut state.stack)?;
                let a = pop(&mut state.stack)?;
                state.stack.push(SimValue::Bool(a == b));
            }
            Instruction::CmpNe => {
                let b = pop(&mut state.stack)?;
                let a = pop(&mut state.stack)?;
                state.stack.push(SimValue::Bool(a != b));
            }
            Instruction::CmpLt => cmp_int(&mut state.stack, |a, b| a < b)?,
            Instruction::CmpGt => cmp_int(&mut state.stack, |a, b| a > b)?,
            Instruction::CmpLe => cmp_int(&mut state.stack, |a, b| a <= b)?,
            Instruction::CmpGe => cmp_int(&mut state.stack, |a, b| a >= b)?,
            Instruction::Contains => {
                let container = pop(&mut state.stack)?;
                let item = pop(&mut state.stack)?;
                let result = sim_contains(&container, &item)?;
                state.stack.push(SimValue::Bool(result));
            }
            Instruction::NotContains => {
                let container = pop(&mut state.stack)?;
                let item = pop(&mut state.stack)?;
                let result = sim_contains(&container, &item)?;
                state.stack.push(SimValue::Bool(!result));
            }

            // --- Boolean ---
            Instruction::Not => {
                let val = pop(&mut state.stack)?;
                state.stack.push(SimValue::Bool(!val.is_truthy()));
            }
            Instruction::IsNone => {
                let val = pop(&mut state.stack)?;
                state
                    .stack
                    .push(SimValue::Bool(matches!(val, SimValue::None)));
            }
            Instruction::IsNotNone => {
                let val = pop(&mut state.stack)?;
                state
                    .stack
                    .push(SimValue::Bool(!matches!(val, SimValue::None)));
            }

            // --- Control flow ---
            Instruction::Jump(target) => {
                state.pc = target;
            }
            Instruction::JumpIfFalse(target) => {
                let val = pop(&mut state.stack)?;
                if !val.is_truthy() {
                    state.pc = target;
                }
            }
            Instruction::JumpIfTrue(target) => {
                let val = pop(&mut state.stack)?;
                if val.is_truthy() {
                    state.pc = target;
                }
            }

            // --- Functions ---
            Instruction::Call(target_pc, num_args) => {
                if state.call_stack.len() >= MAX_CALL_DEPTH {
                    return Err(SimError::stack_overflow());
                }
                let stack_base = state.stack.len().saturating_sub(num_args);
                let var_base = state.variables.len();

                // Move arguments from stack into new variable slots.
                let args: Vec<SimValue> = state.stack.drain(stack_base..).collect();
                state.variables.extend(args);

                // Allocate any additional local slots from function entry.
                if let Some(func) = state
                    .program
                    .functions
                    .iter()
                    .find(|f| f.pc == target_pc)
                {
                    let total_locals = func.num_params + func.num_locals;
                    while state.variables.len() < var_base + total_locals {
                        state.variables.push(SimValue::None);
                    }
                }

                state.call_stack.push(CallFrame {
                    return_pc: state.pc,
                    stack_base,
                    var_base,
                });
                state.pc = target_pc;
            }
            Instruction::Return => {
                let return_val = state.stack.pop().unwrap_or(SimValue::None);
                if let Some(frame) = state.call_stack.pop() {
                    state.pc = frame.return_pc;
                    state.variables.truncate(frame.var_base);
                    state.stack.truncate(frame.stack_base);
                    state.stack.push(return_val);
                } else {
                    // Return from top-level = halt.
                    return Ok(None);
                }
            }

            // --- Data structures ---
            Instruction::BuildList(count) => {
                let start = state.stack.len().saturating_sub(count);
                let items: Vec<SimValue> = state.stack.drain(start..).collect();
                state.stack.push(SimValue::List(items));
            }
            Instruction::BuildDict(count) => {
                let start = state.stack.len().saturating_sub(count * 2);
                let pairs_flat: Vec<SimValue> = state.stack.drain(start..).collect();
                let mut map = indexmap::IndexMap::with_capacity(count);
                for chunk in pairs_flat.chunks_exact(2) {
                    let key = match &chunk[0] {
                        SimValue::Str(s) => s.clone(),
                        other => other.to_string(),
                    };
                    map.insert(key, chunk[1].clone());
                }
                state.stack.push(SimValue::Dict(map));
            }
            Instruction::Index => {
                let index = pop(&mut state.stack)?;
                let collection = pop(&mut state.stack)?;
                match (&collection, &index) {
                    (SimValue::List(list), SimValue::Int(i)) => {
                        let idx = if *i < 0 {
                            (*i + list.len() as i64) as usize
                        } else {
                            *i as usize
                        };
                        let val = list
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| SimError::index_out_of_bounds(*i, list.len()))?;
                        state.stack.push(val);
                    }
                    (SimValue::Dict(map), SimValue::Str(key)) => {
                        let val = map
                            .get(key.as_str())
                            .cloned()
                            .ok_or_else(|| SimError::key_not_found(key))?;
                        state.stack.push(val);
                    }
                    _ => {
                        return Err(SimError::type_error(format!(
                            "cannot index {} with {}",
                            collection.type_name(),
                            index.type_name()
                        )));
                    }
                }
            }
            Instruction::StoreIndex => {
                let value = pop(&mut state.stack)?;
                let index = pop(&mut state.stack)?;
                let mut collection = pop(&mut state.stack)?;
                match (&mut collection, &index) {
                    (SimValue::List(list), SimValue::Int(i)) => {
                        let idx = if *i < 0 {
                            (*i + list.len() as i64) as usize
                        } else {
                            *i as usize
                        };
                        if idx >= list.len() {
                            return Err(SimError::index_out_of_bounds(*i, list.len()));
                        }
                        list[idx] = value;
                    }
                    (SimValue::Dict(map), SimValue::Str(key)) => {
                        map.insert(key.clone(), value);
                    }
                    _ => {
                        return Err(SimError::type_error(format!(
                            "cannot store index {} into {}",
                            index.type_name(),
                            collection.type_name()
                        )));
                    }
                }
                state.stack.push(collection);
            }
            Instruction::GetAttr => {
                let attr = pop_str(&mut state.stack)?;
                let val = pop(&mut state.stack)?;
                match &val {
                    SimValue::EntityRef(eid) => {
                        let result = query::get_entity_attr(world, *eid, &attr)?;
                        state.stack.push(result);
                    }
                    SimValue::Dict(map) => {
                        let result = map
                            .get(&attr)
                            .cloned()
                            .ok_or_else(|| SimError::key_not_found(&attr))?;
                        state.stack.push(result);
                    }
                    _ => {
                        return Err(SimError::type_error(format!(
                            "cannot get attribute '{}' on {}",
                            attr,
                            val.type_name()
                        )));
                    }
                }
            }

            // --- Local variable access ---
            Instruction::LoadLocal(offset) => {
                let var_base = state.call_stack.last().map_or(0, |f| f.var_base);
                let slot = var_base + offset;
                let val = state
                    .variables
                    .get(slot)
                    .cloned()
                    .ok_or_else(|| SimError::invalid_variable(slot))?;
                state.stack.push(val);
            }
            Instruction::StoreLocal(offset) => {
                let var_base = state.call_stack.last().map_or(0, |f| f.var_base);
                let slot = var_base + offset;
                let val = pop(&mut state.stack)?;
                if slot >= state.variables.len() {
                    state.variables.resize(slot + 1, SimValue::None);
                }
                state.variables[slot] = val;
            }

            // --- Standard library builtins ---
            Instruction::Len => {
                let val = pop(&mut state.stack)?;
                let len = match &val {
                    SimValue::List(l) => l.len() as i64,
                    SimValue::Str(s) => s.len() as i64,
                    SimValue::Dict(d) => d.len() as i64,
                    other => {
                        return Err(SimError::type_error(format!(
                            "object of type '{}' has no len()",
                            other.type_name()
                        )));
                    }
                };
                state.stack.push(SimValue::Int(len));
            }
            Instruction::Abs => {
                let n = pop_int(&mut state.stack)?;
                state.stack.push(SimValue::Int(n.abs()));
            }
            Instruction::IntCast => {
                let val = pop(&mut state.stack)?;
                let result = match val {
                    SimValue::Int(n) => n,
                    SimValue::Bool(b) => if b { 1 } else { 0 },
                    SimValue::Str(s) => s.parse::<i64>().map_err(|_| {
                        SimError::type_error(format!("invalid literal for int(): '{s}'"))
                    })?,
                    other => {
                        return Err(SimError::type_error(format!(
                            "int() argument must be a string, number, or bool, not {}",
                            other.type_name()
                        )));
                    }
                };
                state.stack.push(SimValue::Int(result));
            }
            Instruction::StrCast => {
                let val = pop(&mut state.stack)?;
                state.stack.push(SimValue::Str(val.to_string()));
            }
            Instruction::TypeOf => {
                let val = pop(&mut state.stack)?;
                state.stack.push(SimValue::Str(val.type_name().to_string()));
            }
            Instruction::Range(nargs) => {
                let (start, end, step) = match nargs {
                    1 => {
                        let end = pop_int(&mut state.stack)?;
                        (0i64, end, 1i64)
                    }
                    2 => {
                        let end = pop_int(&mut state.stack)?;
                        let start = pop_int(&mut state.stack)?;
                        (start, end, 1i64)
                    }
                    3 => {
                        let step = pop_int(&mut state.stack)?;
                        let end = pop_int(&mut state.stack)?;
                        let start = pop_int(&mut state.stack)?;
                        if step == 0 {
                            return Err(SimError::new(
                                crate::error::SimErrorKind::Runtime,
                                "range() step must not be zero",
                            ));
                        }
                        (start, end, step)
                    }
                    _ => {
                        return Err(SimError::type_error("range() takes 1 to 3 arguments"));
                    }
                };
                let mut result = Vec::new();
                if step > 0 {
                    let mut i = start;
                    while i < end {
                        result.push(SimValue::Int(i));
                        i += step;
                    }
                } else {
                    let mut i = start;
                    while i > end {
                        result.push(SimValue::Int(i));
                        i += step;
                    }
                }
                state.stack.push(SimValue::List(result));
            }
            Instruction::ListAppend => {
                let val = pop(&mut state.stack)?;
                let mut list = pop(&mut state.stack)?;
                match &mut list {
                    SimValue::List(items) => items.push(val),
                    other => {
                        return Err(SimError::type_error(format!(
                            "cannot append to {}",
                            other.type_name()
                        )));
                    }
                }
                state.stack.push(list);
            }
            Instruction::Min2 => {
                let b = pop_int(&mut state.stack)?;
                let a = pop_int(&mut state.stack)?;
                state.stack.push(SimValue::Int(a.min(b)));
            }
            Instruction::Max2 => {
                let b = pop_int(&mut state.stack)?;
                let a = pop_int(&mut state.stack)?;
                state.stack.push(SimValue::Int(a.max(b)));
            }
            Instruction::DictKeys => {
                let dict = pop(&mut state.stack)?;
                match dict {
                    SimValue::Dict(map) => {
                        let keys: Vec<SimValue> = map.keys().map(|k| SimValue::Str(k.clone())).collect();
                        state.stack.push(SimValue::List(keys));
                    }
                    other => {
                        return Err(SimError::type_error(format!(
                            "keys() requires dict, got {}",
                            other.type_name()
                        )));
                    }
                }
            }
            Instruction::DictValues => {
                let dict = pop(&mut state.stack)?;
                match dict {
                    SimValue::Dict(map) => {
                        let values: Vec<SimValue> = map.values().cloned().collect();
                        state.stack.push(SimValue::List(values));
                    }
                    other => {
                        return Err(SimError::type_error(format!(
                            "values() requires dict, got {}",
                            other.type_name()
                        )));
                    }
                }
            }
            Instruction::DictItems => {
                let dict = pop(&mut state.stack)?;
                match dict {
                    SimValue::Dict(map) => {
                        let items: Vec<SimValue> = map
                            .into_iter()
                            .map(|(k, v)| SimValue::List(vec![SimValue::Str(k), v]))
                            .collect();
                        state.stack.push(SimValue::List(items));
                    }
                    other => {
                        return Err(SimError::type_error(format!(
                            "items() requires dict, got {}",
                            other.type_name()
                        )));
                    }
                }
            }
            Instruction::DictGet => {
                let default = pop(&mut state.stack)?;
                let key = pop_str(&mut state.stack)?;
                let dict = pop(&mut state.stack)?;
                match dict {
                    SimValue::Dict(map) => {
                        let val = map
                            .get(&key)
                            .cloned()
                            .unwrap_or(default);
                        state.stack.push(val);
                    }
                    other => {
                        return Err(SimError::type_error(format!(
                            "get() requires dict, got {}",
                            other.type_name()
                        )));
                    }
                }
            }

            Instruction::Percent => {
                let pct = pop_int(&mut state.stack)?;
                let value = pop_int(&mut state.stack)?;
                // value * pct / 100 with banker's rounding (round half to even).
                let product = value.wrapping_mul(pct);
                let result = bankers_div(product, 100);
                state.stack.push(SimValue::Int(result));
            }
            Instruction::Scale => {
                let den = pop_int(&mut state.stack)?;
                let num = pop_int(&mut state.stack)?;
                let value = pop_int(&mut state.stack)?;
                if den == 0 {
                    return Err(SimError::division_by_zero());
                }
                let product = value.wrapping_mul(num);
                let result = bankers_div(product, den);
                state.stack.push(SimValue::Int(result));
            }

            // --- Query instructions (instant) ---
            Instruction::QueryScan => {
                let filter = pop_str(&mut state.stack)?;
                let results = query::scan(world, entity_id, &filter);
                state.stack.push(SimValue::List(results));
            }
            Instruction::QueryGetPos => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let pos = query::get_pos(world, eid)?;
                state.stack.push(SimValue::Int(pos));
            }
            Instruction::QueryNearest => {
                let filter = pop_str(&mut state.stack)?;
                let result = query::nearest(world, entity_id, &filter);
                state.stack.push(result);
            }
            Instruction::QueryDistance => {
                let b = pop_entity_ref(&mut state.stack)?;
                let a = pop_entity_ref(&mut state.stack)?;
                let dist = query::distance(world, a, b)?;
                state.stack.push(SimValue::Int(dist));
            }
            Instruction::QueryGetHealth => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = query::get_stat(world, eid, "health")?;
                state.stack.push(val);
            }
            Instruction::QueryGetShield => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = query::get_stat(world, eid, "shield")?;
                state.stack.push(val);
            }
            Instruction::QueryGetTarget => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = query::get_target(world, eid)?;
                state.stack.push(val);
            }
            Instruction::QueryHasTarget => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = query::has_target(world, eid)?;
                state.stack.push(SimValue::Bool(val));
            }
            Instruction::QueryGetType => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = query::get_type(world, eid)?;
                state.stack.push(SimValue::Str(val));
            }
            Instruction::QueryGetName => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = query::get_name(world, eid)?;
                state.stack.push(SimValue::Str(val));
            }
            Instruction::QueryGetOwner => {
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = query::get_owner(world, eid)?;
                state.stack.push(val);
            }
            Instruction::QueryGetResource => {
                let name = pop_str(&mut state.stack)?;
                world.check_resource_available(&name)?;
                let val = world.get_resource(&name);
                state.stack.push(SimValue::Int(val));
            }
            Instruction::QueryGetStat => {
                let stat_name = pop_str(&mut state.stack)?;
                let eid = pop_entity_ref(&mut state.stack)?;
                let val = world.get_entity(eid)
                    .map_or(0, |e| e.stat(&stat_name));
                state.stack.push(SimValue::Int(val));
            }

            // --- Action instructions (consume tick) ---
            Instruction::ActionMove => {
                let target_pos = pop_int(&mut state.stack)?;
                state.yielded = true;
                return Ok(Some(UnitAction::Move { target_pos }));
            }
            Instruction::ActionAttack => {
                let target = pop_entity_ref(&mut state.stack)?;
                state.yielded = true;
                return Ok(Some(UnitAction::Attack { target }));
            }
            Instruction::ActionFlee => {
                let threat = pop_entity_ref(&mut state.stack)?;
                state.yielded = true;
                return Ok(Some(UnitAction::Flee { threat }));
            }
            Instruction::ActionWait => {
                state.yielded = true;
                return Ok(Some(UnitAction::Wait));
            }
            Instruction::ActionSetTarget => {
                let target = pop_entity_ref(&mut state.stack)?;
                state.yielded = true;
                return Ok(Some(UnitAction::SetTarget { target }));
            }
            Instruction::ActionCustom(name) => {
                // Pop N args from stack (N from custom command registry).
                let num_args = world.custom_command_arg_counts
                    .get(&name)
                    .copied()
                    .unwrap_or(0);
                let mut args = Vec::with_capacity(num_args);
                for _ in 0..num_args {
                    args.push(pop(&mut state.stack)?);
                }
                args.reverse(); // Args were pushed left-to-right, popped in reverse.
                state.yielded = true;
                return Ok(Some(UnitAction::Custom { name, args }));
            }
            // --- Instant effect instructions (don't consume tick) ---
            Instruction::InstantGainResource => {
                let amount = pop_int(&mut state.stack)?;
                let name = pop_str(&mut state.stack)?;
                world.check_resource_available(&name)?;
                return Ok(Some(UnitAction::GainResource { name, amount }));
            }
            Instruction::InstantTrySpendResource => {
                let amount = pop_int(&mut state.stack)?;
                let name = pop_str(&mut state.stack)?;
                world.check_resource_available(&name)?;
                return Ok(Some(UnitAction::TrySpendResource { name, amount }));
            }

            // --- Misc ---
            Instruction::Print => {
                let val = pop(&mut state.stack)?;
                // Events are collected by the world; we store the output text.
                // The caller (world.tick) will create the SimEvent.
                return Ok(Some(UnitAction::Print {
                    text: val.to_string(),
                }));
            }
            Instruction::Halt => {
                return Ok(None);
            }
        }
    }

    // Fell off the end of instructions.
    Ok(None)
}

// ---------------------------------------------------------------------------
// Stack helpers
// ---------------------------------------------------------------------------

fn pop(stack: &mut Vec<SimValue>) -> Result<SimValue, SimError> {
    stack.pop().ok_or_else(SimError::stack_underflow)
}

fn pop_int(stack: &mut Vec<SimValue>) -> Result<i64, SimError> {
    match pop(stack)? {
        SimValue::Int(n) => Ok(n),
        other => Err(SimError::type_error(format!(
            "expected int, got {}",
            other.type_name()
        ))),
    }
}

fn pop_str(stack: &mut Vec<SimValue>) -> Result<String, SimError> {
    match pop(stack)? {
        SimValue::Str(s) => Ok(s),
        other => Err(SimError::type_error(format!(
            "expected str, got {}",
            other.type_name()
        ))),
    }
}

fn pop_entity_ref(stack: &mut Vec<SimValue>) -> Result<EntityId, SimError> {
    match pop(stack)? {
        SimValue::EntityRef(id) => Ok(id),
        other => Err(SimError::type_error(format!(
            "expected entity, got {}",
            other.type_name()
        ))),
    }
}

fn binary_op(
    stack: &mut Vec<SimValue>,
    f: impl FnOnce(SimValue, SimValue) -> Result<SimValue, SimError>,
) -> Result<(), SimError> {
    let b = pop(stack)?;
    let a = pop(stack)?;
    let result = f(a, b)?;
    stack.push(result);
    Ok(())
}

fn binary_int_op(
    stack: &mut Vec<SimValue>,
    op_name: &str,
    f: fn(i64, i64) -> i64,
) -> Result<(), SimError> {
    let b = pop(stack)?;
    let a = pop(stack)?;
    match (a, b) {
        (SimValue::Int(x), SimValue::Int(y)) => {
            stack.push(SimValue::Int(f(x, y)));
            Ok(())
        }
        (a, b) => Err(SimError::type_error(format!(
            "cannot {op_name} {} and {}",
            a.type_name(),
            b.type_name()
        ))),
    }
}

/// Python-style floor division: rounds toward negative infinity.
fn floor_div(a: i64, b: i64) -> i64 {
    let q = a / b;
    let r = a % b;
    if (r != 0) && ((r ^ b) < 0) { q - 1 } else { q }
}

/// Python-style floor modulo: result has same sign as divisor.
fn floor_mod(a: i64, b: i64) -> i64 {
    let r = a % b;
    if (r != 0) && ((r ^ b) < 0) { r + b } else { r }
}

/// Integer division with banker's rounding (round half to even).
fn bankers_div(numerator: i64, denominator: i64) -> i64 {
    let quotient = numerator / denominator;
    let remainder = (numerator % denominator).abs();
    let half = denominator.abs() / 2;
    let is_exact_half = denominator.abs() % 2 == 0 && remainder == half;

    if is_exact_half {
        // Round to even.
        if quotient % 2 == 0 { quotient } else { quotient + numerator.signum() }
    } else if remainder > half {
        quotient + numerator.signum()
    } else {
        quotient
    }
}

fn cmp_int(
    stack: &mut Vec<SimValue>,
    f: fn(i64, i64) -> bool,
) -> Result<(), SimError> {
    let b = pop(stack)?;
    let a = pop(stack)?;
    match (a, b) {
        (SimValue::Int(x), SimValue::Int(y)) => {
            stack.push(SimValue::Bool(f(x, y)));
            Ok(())
        }
        (a, b) => Err(SimError::type_error(format!(
            "cannot compare {} and {}",
            a.type_name(),
            b.type_name()
        ))),
    }
}

/// Check if `container` contains `item` (for `in` / `not in` operators).
fn sim_contains(container: &SimValue, item: &SimValue) -> Result<bool, SimError> {
    match container {
        SimValue::List(list) => Ok(list.iter().any(|v| v == item)),
        SimValue::Str(s) => {
            if let SimValue::Str(sub) = item {
                Ok(s.contains(sub.as_str()))
            } else {
                Err(SimError::type_error(format!(
                    "'in <str>' requires str as left operand, not {}",
                    item.type_name()
                )))
            }
        }
        SimValue::Dict(d) => {
            if let SimValue::Str(key) = item {
                Ok(d.contains_key(key.as_str()))
            } else {
                Err(SimError::type_error(format!(
                    "'in <dict>' requires str as left operand, not {}",
                    item.type_name()
                )))
            }
        }
        _ => Err(SimError::type_error(format!(
            "argument of type '{}' is not iterable",
            container.type_name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::ScriptState;
    use crate::ir::{CompiledScript, Instruction};
    use crate::world::SimWorld;

    fn make_world() -> SimWorld {
        SimWorld::new(42)
    }

    fn run_script(instructions: Vec<Instruction>, num_vars: usize) -> (ScriptState, Option<UnitAction>) {
        let mut world = make_world();
        let eid = world.spawn_entity("skeleton".into(), "test".into(), 0);
        let program = CompiledScript::new(instructions, num_vars);
        let mut state = ScriptState::new(program, num_vars);
        let action = execute_unit(eid, &mut state, &world).unwrap();
        (state, action)
    }

    #[test]
    fn arithmetic() {
        let (state, action) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(10)),
                Instruction::LoadConst(SimValue::Int(3)),
                Instruction::Add,
                Instruction::Halt,
            ],
            0,
        );
        assert!(action.is_none());
        assert_eq!(state.stack.last(), Some(&SimValue::Int(13)));
    }

    #[test]
    fn variables() {
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(42)),
                Instruction::StoreVar(0),
                Instruction::LoadVar(0),
                Instruction::Halt,
            ],
            1,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(42)));
    }

    #[test]
    fn jump_if_false() {
        // if false: skip to halt, so stack should have 99
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Bool(false)),
                Instruction::JumpIfFalse(3),
                Instruction::LoadConst(SimValue::Int(1)), // skipped
                Instruction::LoadConst(SimValue::Int(99)),
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(99)));
    }

    #[test]
    fn action_move_yields() {
        let (state, action) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(500)),
                Instruction::ActionMove,
                Instruction::LoadConst(SimValue::Int(999)), // should not execute
            ],
            0,
        );
        assert!(state.yielded);
        assert!(matches!(action, Some(UnitAction::Move { target_pos: 500 })));
    }

    #[test]
    fn step_limit_auto_yields() {
        // Infinite loop with no action — should auto-yield after STEP_LIMIT.
        let (state, action) = run_script(
            vec![
                Instruction::Jump(0), // infinite loop
            ],
            0,
        );
        assert!(state.yielded);
        assert!(matches!(action, Some(UnitAction::Wait)));
    }

    #[test]
    fn division_by_zero() {
        let mut world = make_world();
        let eid = world.spawn_entity("skeleton".into(), "test".into(), 0);
        let program = CompiledScript::new(
            vec![
                Instruction::LoadConst(SimValue::Int(10)),
                Instruction::LoadConst(SimValue::Int(0)),
                Instruction::Div,
            ],
            0,
        );
        let mut state = ScriptState::new(program, 0);
        let result = execute_unit(eid, &mut state, &world);
        assert!(result.is_err());
    }

    #[test]
    fn build_list() {
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(1)),
                Instruction::LoadConst(SimValue::Int(2)),
                Instruction::LoadConst(SimValue::Int(3)),
                Instruction::BuildList(3),
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(
            state.stack.last(),
            Some(&SimValue::List(vec![
                SimValue::Int(1),
                SimValue::Int(2),
                SimValue::Int(3)
            ]))
        );
    }

    #[test]
    fn while_loop_move_oscillate() {
        // Hand-assembled: while True: move(100); move(0)
        // Should yield on first move(100).
        let (state, action) = run_script(
            vec![
                // 0: push True
                Instruction::LoadConst(SimValue::Bool(true)),
                // 1: jump if false to 5 (end)
                Instruction::JumpIfFalse(5),
                // 2: move(100)
                Instruction::LoadConst(SimValue::Int(100)),
                Instruction::ActionMove,
                // 4: jump back to 0
                Instruction::Jump(0),
                // 5: halt
                Instruction::Halt,
            ],
            0,
        );
        assert!(state.yielded);
        assert!(matches!(action, Some(UnitAction::Move { target_pos: 100 })));
        // PC should be at 4 (the instruction after ActionMove)
        assert_eq!(state.pc, 4);
    }

    // --- Floor division/modulo tests ---

    #[test]
    fn floor_div_positive() {
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(7)),
                Instruction::LoadConst(SimValue::Int(2)),
                Instruction::Div,
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(3)));
    }

    #[test]
    fn floor_div_negative_dividend() {
        // Python: -7 // 2 = -4
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(-7)),
                Instruction::LoadConst(SimValue::Int(2)),
                Instruction::Div,
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(-4)));
    }

    #[test]
    fn floor_div_negative_divisor() {
        // Python: 7 // -2 = -4
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(7)),
                Instruction::LoadConst(SimValue::Int(-2)),
                Instruction::Div,
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(-4)));
    }

    #[test]
    fn floor_div_both_negative() {
        // Python: -7 // -2 = 3
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(-7)),
                Instruction::LoadConst(SimValue::Int(-2)),
                Instruction::Div,
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(3)));
    }

    #[test]
    fn floor_mod_positive() {
        // Python: 7 % 2 = 1
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(7)),
                Instruction::LoadConst(SimValue::Int(2)),
                Instruction::Mod,
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(1)));
    }

    #[test]
    fn floor_mod_negative_dividend() {
        // Python: -7 % 2 = 1
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(-7)),
                Instruction::LoadConst(SimValue::Int(2)),
                Instruction::Mod,
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(1)));
    }

    #[test]
    fn floor_mod_negative_divisor() {
        // Python: 7 % -2 = -1
        let (state, _) = run_script(
            vec![
                Instruction::LoadConst(SimValue::Int(7)),
                Instruction::LoadConst(SimValue::Int(-2)),
                Instruction::Mod,
                Instruction::Halt,
            ],
            0,
        );
        assert_eq!(state.stack.last(), Some(&SimValue::Int(-1)));
    }
}
