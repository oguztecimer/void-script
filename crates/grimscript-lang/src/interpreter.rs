use std::collections::{HashMap, HashSet};

use crossbeam_channel::{Receiver, Sender};

use crate::ast::*;
use crate::builtins;
use crate::debug::*;
use crate::environment::Environment;
use crate::error::GrimScriptError;
use crate::value::Value;

enum ControlFlow {
    None,
    Break,
    Continue,
    Return(Value),
}

pub struct Interpreter {
    env: Environment,
    output_tx: Sender<ScriptEvent>,
    command_rx: Receiver<DebugCommand>,
    breakpoints: HashSet<u32>,
    debug_mode: bool,
    step_mode: StepMode,
    step_count: u64,
    max_steps: u64,
    functions: HashMap<String, (Vec<String>, Vec<Statement>)>,
    call_stack: Vec<String>,
    stopped: bool,
    available_commands: Option<HashSet<String>>,
    custom_commands: HashSet<String>,
}

impl Interpreter {
    pub fn new(
        output_tx: Sender<ScriptEvent>,
        command_rx: Receiver<DebugCommand>,
        debug_mode: bool,
    ) -> Self {
        let mut env = Environment::new();

        // self keyword
        env.set(
            "self".to_string(),
            Value::Entity {
                id: 0,
                name: "self".into(),
                entity_type: "unit".into(),
            },
        );

        Self {
            env,
            output_tx,
            command_rx,
            breakpoints: HashSet::new(),
            debug_mode,
            step_mode: if debug_mode {
                StepMode::StepInto
            } else {
                StepMode::Run
            },
            step_count: 0,
            max_steps: 100_000,
            functions: HashMap::new(),
            call_stack: vec!["<module>".to_string()],
            stopped: false,
            available_commands: None,
            custom_commands: HashSet::new(),
        }
    }

    pub fn set_available_commands(&mut self, cmds: HashSet<String>) {
        self.available_commands = Some(cmds);
    }

    pub fn set_custom_commands(&mut self, cmds: HashSet<String>) {
        self.custom_commands = cmds;
    }

    fn check_command_available(&self, name: &str, line: u32) -> Result<(), GrimScriptError> {
        if builtins::is_stdlib(name) {
            return Ok(());
        }
        if builtins::is_game_builtin(name) || self.custom_commands.contains(name) {
            if let Some(ref set) = self.available_commands {
                if !set.contains(name) {
                    return Err(GrimScriptError::runtime(
                        line,
                        format!("'{name}' is not available yet"),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn set_breakpoints(&mut self, breakpoints: HashSet<u32>) {
        self.breakpoints = breakpoints;
    }

    pub fn execute(&mut self, program: &Program) -> Result<(), GrimScriptError> {
        // First pass: collect top-level function definitions
        for stmt in &program.statements {
            if let StmtKind::FunctionDef { name, params, body } = &stmt.kind {
                self.functions
                    .insert(name.clone(), (params.clone(), body.clone()));
            }
        }

        // Second pass: execute top-level statements
        for stmt in &program.statements {
            if matches!(stmt.kind, StmtKind::FunctionDef { .. }) {
                continue; // Already collected
            }
            match self.execute_statement(stmt)? {
                ControlFlow::Return(_) => break,
                ControlFlow::Break => {
                    return Err(GrimScriptError::syntax(
                        stmt.line,
                        "'break' outside loop",
                    ))
                }
                ControlFlow::Continue => {
                    return Err(GrimScriptError::syntax(
                        stmt.line,
                        "'continue' outside loop",
                    ))
                }
                ControlFlow::None => {}
            }
        }

        // Auto-call main() if it exists
        if self.functions.contains_key("main") {
            self.call_function("main", vec![], 0)?;
        }

        Ok(())
    }

    fn check_step_limit(&mut self, line: u32) -> Result<(), GrimScriptError> {
        self.step_count += 1;
        if self.step_count > self.max_steps {
            return Err(GrimScriptError::step_limit(line));
        }
        Ok(())
    }

    fn check_debug(&mut self, line: u32) -> Result<(), GrimScriptError> {
        if self.stopped {
            return Err(GrimScriptError::stopped(line));
        }

        // Check for incoming commands non-blockingly (e.g., stop or breakpoint updates)
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                DebugCommand::Stop => {
                    self.stopped = true;
                    return Err(GrimScriptError::stopped(line));
                }
                DebugCommand::SetBreakpoints(bps) => {
                    self.breakpoints = bps;
                }
                DebugCommand::Continue => {
                    self.step_mode = StepMode::Run;
                }
                DebugCommand::StepOver => {
                    self.step_mode = StepMode::StepOver {
                        depth: self.call_stack.len(),
                    };
                }
                DebugCommand::StepInto => {
                    self.step_mode = StepMode::StepInto;
                }
                DebugCommand::StepOut => {
                    self.step_mode = StepMode::StepOut {
                        target_depth: self.call_stack.len().saturating_sub(1),
                    };
                }
            }
        }

        if !self.debug_mode {
            return Ok(());
        }

        let should_pause = match &self.step_mode {
            StepMode::Run => self.breakpoints.contains(&line),
            StepMode::StepInto => true,
            StepMode::StepOver { depth } => {
                self.call_stack.len() <= *depth || self.breakpoints.contains(&line)
            }
            StepMode::StepOut { target_depth } => {
                self.call_stack.len() <= *target_depth || self.breakpoints.contains(&line)
            }
        };

        if should_pause {
            self.pause_at(line)?;
        }

        Ok(())
    }

    fn pause_at(&mut self, line: u32) -> Result<(), GrimScriptError> {
        let variables: Vec<VariableInfo> = self
            .env
            .all_variables()
            .into_iter()
            .map(|(name, val)| VariableInfo {
                var_type: val.type_name().to_string(),
                value: val.display(),
                name,
            })
            .collect();

        let _ = self.output_tx.send(ScriptEvent::Paused {
            line,
            variables,
            call_stack: self.call_stack.clone(),
        });

        // Block waiting for next command
        loop {
            match self.command_rx.recv() {
                Ok(DebugCommand::Continue) => {
                    self.step_mode = StepMode::Run;
                    break;
                }
                Ok(DebugCommand::StepOver) => {
                    self.step_mode = StepMode::StepOver {
                        depth: self.call_stack.len(),
                    };
                    break;
                }
                Ok(DebugCommand::StepInto) => {
                    self.step_mode = StepMode::StepInto;
                    break;
                }
                Ok(DebugCommand::StepOut) => {
                    self.step_mode = StepMode::StepOut {
                        target_depth: self.call_stack.len().saturating_sub(1),
                    };
                    break;
                }
                Ok(DebugCommand::Stop) => {
                    self.stopped = true;
                    return Err(GrimScriptError::stopped(line));
                }
                Ok(DebugCommand::SetBreakpoints(bps)) => {
                    self.breakpoints = bps;
                    // Continue waiting for a stepping command
                }
                Err(_) => {
                    // Channel closed
                    self.stopped = true;
                    return Err(GrimScriptError::stopped(line));
                }
            }
        }

        Ok(())
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<ControlFlow, GrimScriptError> {
        self.check_step_limit(stmt.line)?;
        self.check_debug(stmt.line)?;

        match &stmt.kind {
            StmtKind::FunctionDef { name, params, body } => {
                self.functions
                    .insert(name.clone(), (params.clone(), body.clone()));
                Ok(ControlFlow::None)
            }
            StmtKind::Expr(expr) => {
                self.eval_expr(expr)?;
                Ok(ControlFlow::None)
            }
            StmtKind::Assign { target, value } => {
                let val = self.eval_expr(value)?;
                self.assign_target(target, val, stmt.line)?;
                Ok(ControlFlow::None)
            }
            StmtKind::AugAssign { target, op, value } => {
                let current = self.get_target_value(target, stmt.line)?;
                let rhs = self.eval_expr(value)?;
                let result = self.apply_aug_op(op, &current, &rhs, stmt.line)?;
                self.assign_target(target, result, stmt.line)?;
                Ok(ControlFlow::None)
            }
            StmtKind::If {
                condition,
                body,
                elif_clauses,
                else_body,
            } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val.is_truthy() {
                    return self.execute_block(body);
                }
                for (elif_cond, elif_body) in elif_clauses {
                    let elif_val = self.eval_expr(elif_cond)?;
                    if elif_val.is_truthy() {
                        return self.execute_block(elif_body);
                    }
                }
                if let Some(else_b) = else_body {
                    return self.execute_block(else_b);
                }
                Ok(ControlFlow::None)
            }
            StmtKind::While { condition, body } => {
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if !cond_val.is_truthy() {
                        break;
                    }
                    match self.execute_block(body)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            StmtKind::For {
                var,
                iterable,
                body,
            } => {
                let iter_val = self.eval_expr(iterable)?;
                let items = match iter_val {
                    Value::List(l) => l,
                    Value::Tuple(t) => t,
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    other => {
                        return Err(GrimScriptError::type_error(
                            stmt.line,
                            format!("'{}' is not iterable", other.type_name()),
                        ))
                    }
                };
                for item in items {
                    self.env.update(var.clone(), item);
                    match self.execute_block(body)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            StmtKind::Return { value } => {
                let val = match value {
                    Some(expr) => self.eval_expr(expr)?,
                    Option::None => Value::None,
                };
                Ok(ControlFlow::Return(val))
            }
            StmtKind::Break => Ok(ControlFlow::Break),
            StmtKind::Continue => Ok(ControlFlow::Continue),
            StmtKind::Pass => Ok(ControlFlow::None),
        }
    }

    fn execute_block(&mut self, stmts: &[Statement]) -> Result<ControlFlow, GrimScriptError> {
        for stmt in stmts {
            match self.execute_statement(stmt)? {
                ControlFlow::None => {}
                flow => return Ok(flow),
            }
        }
        Ok(ControlFlow::None)
    }

    fn assign_target(
        &mut self,
        target: &AssignTarget,
        value: Value,
        line: u32,
    ) -> Result<(), GrimScriptError> {
        match target {
            AssignTarget::Name(name) => {
                self.env.update(name.clone(), value);
                Ok(())
            }
            AssignTarget::Index { object, index } => {
                // We need to evaluate the object to find where to assign
                // For now, handle list index and dict key assignment
                let obj_name = self.extract_name(object);
                let idx = self.eval_expr(index)?;

                if let Some(name) = obj_name {
                    let mut obj_val = self
                        .env
                        .get(&name)
                        .cloned()
                        .ok_or_else(|| GrimScriptError::name_error(line, format!("'{name}' is not defined")))?;

                    match (&mut obj_val, &idx) {
                        (Value::List(list), Value::Int(i)) => {
                            let index = if *i < 0 {
                                let adjusted = list.len() as i64 + *i;
                                if adjusted < 0 {
                                    return Err(GrimScriptError::index_error(
                                        line,
                                        "list index out of range",
                                    ));
                                }
                                adjusted as usize
                            } else {
                                *i as usize
                            };
                            if index >= list.len() {
                                return Err(GrimScriptError::index_error(
                                    line,
                                    "list index out of range",
                                ));
                            }
                            list[index] = value;
                        }
                        (Value::Dict(dict), Value::String(key)) => {
                            dict.insert(key.clone(), value);
                        }
                        _ => {
                            return Err(GrimScriptError::type_error(
                                line,
                                "Invalid index assignment",
                            ))
                        }
                    }

                    self.env.update(name, obj_val);
                    Ok(())
                } else {
                    Err(GrimScriptError::type_error(
                        line,
                        "Cannot assign to complex index expression",
                    ))
                }
            }
        }
    }

    fn extract_name(&self, expr: &Expr) -> Option<String> {
        if let ExprKind::Name(n) = &expr.kind {
            Some(n.clone())
        } else {
            Option::None
        }
    }

    fn get_target_value(
        &mut self,
        target: &AssignTarget,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match target {
            AssignTarget::Name(name) => self
                .env
                .get(name)
                .cloned()
                .ok_or_else(|| GrimScriptError::name_error(line, format!("'{name}' is not defined"))),
            AssignTarget::Index { object, index } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.index_value(&obj, &idx, line)
            }
        }
    }

    fn apply_aug_op(
        &self,
        op: &AugOp,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match op {
            AugOp::Add => self.add_values(left, right, line),
            AugOp::Sub => self.sub_values(left, right, line),
            AugOp::Mul => self.mul_values(left, right, line),
            AugOp::Div => self.div_values(left, right, line),
        }
    }

    // --- Expression evaluation ---

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, GrimScriptError> {
        match &expr.kind {
            ExprKind::Integer(n) => Ok(Value::Int(*n)),
            ExprKind::Float(f) => Ok(Value::Float(*f)),
            ExprKind::StringLit(s) => Ok(Value::String(s.clone())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::NoneLit => Ok(Value::None),

            ExprKind::Name(name) => self
                .env
                .get(name)
                .cloned()
                .ok_or_else(|| GrimScriptError::name_error(expr.line, format!("'{name}' is not defined"))),

            ExprKind::List(items) => {
                let mut vals = Vec::new();
                for item in items {
                    vals.push(self.eval_expr(item)?);
                }
                Ok(Value::List(vals))
            }

            ExprKind::ListComp {
                expr: item_expr,
                var,
                iter,
                condition,
            } => {
                let iter_val = self.eval_expr(iter)?;
                let items = match iter_val {
                    Value::List(l) => l,
                    Value::Tuple(t) => t,
                    _ => {
                        return Err(GrimScriptError::type_error(
                            expr.line,
                            "list comprehension requires an iterable",
                        ))
                    }
                };

                let mut result = Vec::new();
                self.env.push_scope();
                for item in items {
                    self.env.set(var.clone(), item);
                    if let Some(cond) = condition {
                        let cond_val = self.eval_expr(cond)?;
                        if !cond_val.is_truthy() {
                            continue;
                        }
                    }
                    result.push(self.eval_expr(item_expr)?);
                }
                self.env.pop_scope();
                Ok(Value::List(result))
            }

            ExprKind::BinOp { left, op, right } => {
                let lval = self.eval_expr(left)?;
                let rval = self.eval_expr(right)?;
                self.eval_binop(op, &lval, &rval, expr.line)
            }

            ExprKind::UnaryOp { op, operand } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => match val {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(GrimScriptError::type_error(
                            expr.line,
                            format!("bad operand type for unary -: '{}'", val.type_name()),
                        )),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
                }
            }

            ExprKind::BoolOp { op, left, right } => {
                let lval = self.eval_expr(left)?;
                match op {
                    BoolOpKind::And => {
                        if !lval.is_truthy() {
                            Ok(lval)
                        } else {
                            self.eval_expr(right)
                        }
                    }
                    BoolOpKind::Or => {
                        if lval.is_truthy() {
                            Ok(lval)
                        } else {
                            self.eval_expr(right)
                        }
                    }
                }
            }

            ExprKind::Compare { left, op, right } => {
                let lval = self.eval_expr(left)?;
                let rval = self.eval_expr(right)?;
                let result = match op {
                    CmpOp::Eq => lval == rval,
                    CmpOp::NotEq => lval != rval,
                    CmpOp::Lt => self.compare_lt(&lval, &rval, expr.line)?,
                    CmpOp::Gt => self.compare_gt(&lval, &rval, expr.line)?,
                    CmpOp::LtEq => !self.compare_gt(&lval, &rval, expr.line)?,
                    CmpOp::GtEq => !self.compare_lt(&lval, &rval, expr.line)?,
                    CmpOp::In => self.check_contains(&rval, &lval, expr.line)?,
                    CmpOp::NotIn => !self.check_contains(&rval, &lval, expr.line)?,
                };
                Ok(Value::Bool(result))
            }

            ExprKind::IsNone { expr: inner, negated } => {
                let val = self.eval_expr(inner)?;
                let is_none = matches!(val, Value::None);
                Ok(Value::Bool(if *negated { !is_none } else { is_none }))
            }

            ExprKind::Call { func, args } => {
                // Handle special internal calls
                if let ExprKind::Name(name) = &func.kind {
                    if name == "__tuple__" {
                        let mut vals = Vec::new();
                        for arg in args {
                            vals.push(self.eval_expr(arg)?);
                        }
                        return Ok(Value::Tuple(vals));
                    }
                    if name == "__dict__" {
                        let mut map = HashMap::new();
                        let mut eval_args = Vec::new();
                        for arg in args {
                            eval_args.push(self.eval_expr(arg)?);
                        }
                        let mut i = 0;
                        while i + 1 < eval_args.len() {
                            let key = match &eval_args[i] {
                                Value::String(s) => s.clone(),
                                other => other.display(),
                            };
                            let value = eval_args[i + 1].clone();
                            map.insert(key, value);
                            i += 2;
                        }
                        return Ok(Value::Dict(map));
                    }
                }

                // Evaluate arguments
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.eval_expr(arg)?);
                }

                // Handle method calls (attribute access)
                if let ExprKind::Attribute { object, attr } = &func.kind {
                    let obj = self.eval_expr(object)?;
                    return self.call_method(&obj, attr, eval_args, expr.line, object);
                }

                // Handle named function calls — check user functions and builtins before eval
                if let ExprKind::Name(name) = &func.kind {
                    if self.functions.contains_key(name.as_str()) {
                        return self.call_function(name, eval_args, expr.line);
                    }
                    if builtins::is_builtin_with_custom(name, &self.custom_commands) {
                        self.check_command_available(name, expr.line)?;
                        return builtins::call_builtin_with_custom(
                            name,
                            eval_args,
                            &self.output_tx,
                            &self.custom_commands,
                        );
                    }
                }

                // Fallback: evaluate the expression as a callable value
                let func_val = self.eval_expr(func)?;
                match func_val {
                    _ => {
                        Err(GrimScriptError::type_error(
                            expr.line,
                            "object is not callable",
                        ))
                    }
                }
            }

            ExprKind::Index { object, index } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.index_value(&obj, &idx, expr.line)
            }

            ExprKind::Attribute { object, attr } => {
                let obj = self.eval_expr(object)?;
                self.get_attribute(&obj, attr, expr.line)
            }
        }
    }

    fn call_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        let (params, body) = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| GrimScriptError::name_error(line, format!("Function '{name}' not defined")))?;

        if args.len() != params.len() {
            return Err(GrimScriptError::type_error(
                line,
                format!(
                    "{name}() takes {} arguments but {} were given",
                    params.len(),
                    args.len()
                ),
            ));
        }

        self.env.push_scope();
        for (param, arg) in params.iter().zip(args.into_iter()) {
            self.env.set(param.clone(), arg);
        }
        self.call_stack.push(name.to_string());

        let result = self.execute_block(&body);

        self.call_stack.pop();
        self.env.pop_scope();

        match result {
            Ok(ControlFlow::Return(val)) => Ok(val),
            Ok(ControlFlow::Break) => {
                Err(GrimScriptError::syntax(line, "'break' outside loop"))
            }
            Ok(ControlFlow::Continue) => {
                Err(GrimScriptError::syntax(line, "'continue' outside loop"))
            }
            Ok(ControlFlow::None) => Ok(Value::None),
            Err(e) => Err(e),
        }
    }

    fn call_method(
        &mut self,
        obj: &Value,
        method: &str,
        args: Vec<Value>,
        line: u32,
        object_expr: &Expr,
    ) -> Result<Value, GrimScriptError> {
        match (obj, method) {
            (Value::List(_), "append") => {
                if args.len() != 1 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "append() takes exactly 1 argument",
                    ));
                }
                // We need to mutate the list in the environment
                if let Some(name) = self.extract_name(object_expr) {
                    let mut list_val = self.env.get(&name).cloned().ok_or_else(|| {
                        GrimScriptError::name_error(line, format!("'{name}' is not defined"))
                    })?;
                    if let Value::List(ref mut list) = list_val {
                        list.push(args.into_iter().next().unwrap());
                    }
                    self.env.update(name, list_val);
                    Ok(Value::None)
                } else {
                    Err(GrimScriptError::type_error(
                        line,
                        "Cannot append to expression",
                    ))
                }
            }
            (Value::List(list), "pop") => {
                if let Some(name) = self.extract_name(object_expr) {
                    let mut list_val = self.env.get(&name).cloned().ok_or_else(|| {
                        GrimScriptError::name_error(line, format!("'{name}' is not defined"))
                    })?;
                    if let Value::List(ref mut l) = list_val {
                        if l.is_empty() {
                            return Err(GrimScriptError::index_error(
                                line,
                                "pop from empty list",
                            ));
                        }
                        let popped = if args.is_empty() {
                            l.pop().unwrap()
                        } else if let Value::Int(i) = &args[0] {
                            let idx = if *i < 0 {
                                (l.len() as i64 + *i) as usize
                            } else {
                                *i as usize
                            };
                            if idx >= l.len() {
                                return Err(GrimScriptError::index_error(
                                    line,
                                    "pop index out of range",
                                ));
                            }
                            l.remove(idx)
                        } else {
                            return Err(GrimScriptError::type_error(
                                line,
                                "pop() index must be int",
                            ));
                        };
                        self.env.update(name, list_val);
                        Ok(popped)
                    } else {
                        Err(GrimScriptError::type_error(line, "pop() on non-list"))
                    }
                } else {
                    // Operate on a copy
                    let mut l = list.clone();
                    if l.is_empty() {
                        return Err(GrimScriptError::index_error(
                            line,
                            "pop from empty list",
                        ));
                    }
                    Ok(l.pop().unwrap())
                }
            }
            (Value::List(_), "insert") => {
                if args.len() != 2 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "insert() takes exactly 2 arguments",
                    ));
                }
                if let Some(name) = self.extract_name(object_expr) {
                    let mut list_val = self.env.get(&name).cloned().ok_or_else(|| {
                        GrimScriptError::name_error(line, format!("'{name}' is not defined"))
                    })?;
                    if let Value::List(ref mut l) = list_val {
                        let idx = match &args[0] {
                            Value::Int(i) => *i as usize,
                            _ => {
                                return Err(GrimScriptError::type_error(
                                    line,
                                    "insert() index must be int",
                                ))
                            }
                        };
                        let idx = idx.min(l.len());
                        l.insert(idx, args[1].clone());
                    }
                    self.env.update(name, list_val);
                    Ok(Value::None)
                } else {
                    Err(GrimScriptError::type_error(
                        line,
                        "Cannot insert to expression",
                    ))
                }
            }
            (Value::List(_), "remove") => {
                if args.len() != 1 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "remove() takes exactly 1 argument",
                    ));
                }
                if let Some(name) = self.extract_name(object_expr) {
                    let mut list_val = self.env.get(&name).cloned().ok_or_else(|| {
                        GrimScriptError::name_error(line, format!("'{name}' is not defined"))
                    })?;
                    if let Value::List(ref mut l) = list_val {
                        if let Some(pos) = l.iter().position(|v| *v == args[0]) {
                            l.remove(pos);
                        } else {
                            return Err(GrimScriptError::runtime(
                                line,
                                "list.remove(x): x not in list",
                            ));
                        }
                    }
                    self.env.update(name, list_val);
                    Ok(Value::None)
                } else {
                    Err(GrimScriptError::type_error(
                        line,
                        "Cannot remove from expression",
                    ))
                }
            }
            (Value::String(s), "upper") => Ok(Value::String(s.to_uppercase())),
            (Value::String(s), "lower") => Ok(Value::String(s.to_lowercase())),
            (Value::String(s), "strip") => Ok(Value::String(s.trim().to_string())),
            (Value::String(s), "split") => {
                let sep = if args.is_empty() {
                    " ".to_string()
                } else {
                    match &args[0] {
                        Value::String(sep) => sep.clone(),
                        _ => {
                            return Err(GrimScriptError::type_error(
                                line,
                                "split() separator must be str",
                            ))
                        }
                    }
                };
                let parts: Vec<Value> = s
                    .split(&sep)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                Ok(Value::List(parts))
            }
            (Value::String(s), "join") => {
                if args.len() != 1 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "join() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::List(items) => {
                        let strs: Vec<String> = items.iter().map(|v| v.display()).collect();
                        Ok(Value::String(strs.join(s)))
                    }
                    _ => Err(GrimScriptError::type_error(
                        line,
                        "join() argument must be a list",
                    )),
                }
            }
            (Value::String(s), "startswith") => {
                if args.len() != 1 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "startswith() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::String(prefix) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
                    _ => Err(GrimScriptError::type_error(
                        line,
                        "startswith() argument must be str",
                    )),
                }
            }
            (Value::String(s), "endswith") => {
                if args.len() != 1 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "endswith() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::String(suffix) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
                    _ => Err(GrimScriptError::type_error(
                        line,
                        "endswith() argument must be str",
                    )),
                }
            }
            (Value::String(s), "replace") => {
                if args.len() != 2 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "replace() takes exactly 2 arguments",
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(old), Value::String(new)) => {
                        Ok(Value::String(s.replace(old.as_str(), new.as_str())))
                    }
                    _ => Err(GrimScriptError::type_error(
                        line,
                        "replace() arguments must be str",
                    )),
                }
            }
            (Value::String(s), "find") => {
                if args.len() != 1 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "find() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::String(sub) => {
                        let idx = s.find(sub.as_str()).map(|i| i as i64).unwrap_or(-1);
                        Ok(Value::Int(idx))
                    }
                    _ => Err(GrimScriptError::type_error(
                        line,
                        "find() argument must be str",
                    )),
                }
            }
            (Value::Dict(dict), "keys") => {
                let keys: Vec<Value> = dict.keys().map(|k| Value::String(k.clone())).collect();
                Ok(Value::List(keys))
            }
            (Value::Dict(dict), "values") => {
                let values: Vec<Value> = dict.values().cloned().collect();
                Ok(Value::List(values))
            }
            (Value::Dict(dict), "items") => {
                let items: Vec<Value> = dict
                    .iter()
                    .map(|(k, v)| Value::Tuple(vec![Value::String(k.clone()), v.clone()]))
                    .collect();
                Ok(Value::List(items))
            }
            (Value::Dict(dict), "get") => {
                if args.is_empty() || args.len() > 2 {
                    return Err(GrimScriptError::type_error(
                        line,
                        "get() takes 1 or 2 arguments",
                    ));
                }
                let key = match &args[0] {
                    Value::String(s) => s.clone(),
                    other => other.display(),
                };
                let default = if args.len() > 1 {
                    args[1].clone()
                } else {
                    Value::None
                };
                Ok(dict.get(&key).cloned().unwrap_or(default))
            }
            // Game entity methods - dispatch to builtins with the entity as first arg
            (Value::Entity { .. }, _) => {
                let mut full_args = vec![obj.clone()];
                full_args.extend(args);
                if builtins::is_builtin_with_custom(method, &self.custom_commands) {
                    self.check_command_available(method, line)?;
                    builtins::call_builtin_with_custom(method, full_args, &self.output_tx, &self.custom_commands)
                } else {
                    Err(GrimScriptError::runtime(
                        line,
                        format!("Entity has no method '{method}'"),
                    ))
                }
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "'{}' object has no method '{method}'",
                    obj.type_name()
                ),
            )),
        }
    }

    fn eval_binop(
        &self,
        op: &BinOp,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match op {
            BinOp::Add => self.add_values(left, right, line),
            BinOp::Sub => self.sub_values(left, right, line),
            BinOp::Mul => self.mul_values(left, right, line),
            BinOp::Div => self.div_values(left, right, line),
            BinOp::FloorDiv => self.floordiv_values(left, right, line),
            BinOp::Mod => self.mod_values(left, right, line),
        }
    }

    fn add_values(
        &self,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
            (Value::List(a), Value::List(b)) => {
                let mut result = a.clone();
                result.extend(b.iter().cloned());
                Ok(Value::List(result))
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "unsupported operand type(s) for +: '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn sub_values(
        &self,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "unsupported operand type(s) for -: '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn mul_values(
        &self,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            (Value::String(s), Value::Int(n)) | (Value::Int(n), Value::String(s)) => {
                if *n <= 0 {
                    Ok(Value::String(String::new()))
                } else {
                    Ok(Value::String(s.repeat(*n as usize)))
                }
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "unsupported operand type(s) for *: '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn div_values(
        &self,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Float(*a as f64 / *b as f64))
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Float(a / b))
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Float(*a as f64 / b))
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Float(a / *b as f64))
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "unsupported operand type(s) for /: '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn floordiv_values(
        &self,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Int(a.div_euclid(*b)))
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Float((a / b).floor()))
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Float((*a as f64 / b).floor()))
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(GrimScriptError::runtime(line, "division by zero"));
                }
                Ok(Value::Float((a / *b as f64).floor()))
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "unsupported operand type(s) for //: '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn mod_values(
        &self,
        left: &Value,
        right: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(GrimScriptError::runtime(line, "modulo by zero"));
                }
                Ok(Value::Int(a.rem_euclid(*b)))
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(GrimScriptError::runtime(line, "modulo by zero"));
                }
                Ok(Value::Float(a % b))
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(GrimScriptError::runtime(line, "modulo by zero"));
                }
                Ok(Value::Float(*a as f64 % b))
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(GrimScriptError::runtime(line, "modulo by zero"));
                }
                Ok(Value::Float(a % *b as f64))
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "unsupported operand type(s) for %: '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn compare_lt(&self, left: &Value, right: &Value, line: u32) -> Result<bool, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(a < b),
            (Value::Float(a), Value::Float(b)) => Ok(a < b),
            (Value::Int(a), Value::Float(b)) => Ok((*a as f64) < *b),
            (Value::Float(a), Value::Int(b)) => Ok(*a < (*b as f64)),
            (Value::String(a), Value::String(b)) => Ok(a < b),
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "'<' not supported between instances of '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn compare_gt(&self, left: &Value, right: &Value, line: u32) -> Result<bool, GrimScriptError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(a > b),
            (Value::Float(a), Value::Float(b)) => Ok(a > b),
            (Value::Int(a), Value::Float(b)) => Ok((*a as f64) > *b),
            (Value::Float(a), Value::Int(b)) => Ok(*a > (*b as f64)),
            (Value::String(a), Value::String(b)) => Ok(a > b),
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "'>' not supported between instances of '{}' and '{}'",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    /// Check if `container` contains `item` (for `in` / `not in` operators).
    fn check_contains(&self, container: &Value, item: &Value, line: u32) -> Result<bool, GrimScriptError> {
        match container {
            Value::List(list) => Ok(list.iter().any(|v| v == item)),
            Value::Tuple(items) => Ok(items.iter().any(|v| v == item)),
            Value::String(s) => {
                if let Value::String(sub) = item {
                    Ok(s.contains(sub.as_str()))
                } else {
                    Err(GrimScriptError::type_error(
                        line,
                        format!("'in <string>' requires string as left operand, not {}", item.type_name()),
                    ))
                }
            }
            Value::Dict(d) => {
                if let Value::String(key) = item {
                    Ok(d.contains_key(key.as_str()))
                } else {
                    Err(GrimScriptError::type_error(
                        line,
                        format!("'in <dict>' requires string as left operand, not {}", item.type_name()),
                    ))
                }
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!("argument of type '{}' is not iterable", container.type_name()),
            )),
        }
    }

    fn index_value(
        &self,
        obj: &Value,
        index: &Value,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match (obj, index) {
            (Value::List(list), Value::Int(i)) => {
                let idx = if *i < 0 {
                    let adjusted = list.len() as i64 + *i;
                    if adjusted < 0 {
                        return Err(GrimScriptError::index_error(line, "list index out of range"));
                    }
                    adjusted as usize
                } else {
                    *i as usize
                };
                list.get(idx).cloned().ok_or_else(|| {
                    GrimScriptError::index_error(line, "list index out of range")
                })
            }
            (Value::Tuple(items), Value::Int(i)) => {
                let idx = if *i < 0 {
                    let adjusted = items.len() as i64 + *i;
                    if adjusted < 0 {
                        return Err(GrimScriptError::index_error(line, "tuple index out of range"));
                    }
                    adjusted as usize
                } else {
                    *i as usize
                };
                items.get(idx).cloned().ok_or_else(|| {
                    GrimScriptError::index_error(line, "tuple index out of range")
                })
            }
            (Value::String(s), Value::Int(i)) => {
                let char_count = s.chars().count() as i64;
                let idx = if *i < 0 {
                    let adjusted = char_count + *i;
                    if adjusted < 0 {
                        return Err(GrimScriptError::index_error(line, "string index out of range"));
                    }
                    adjusted as usize
                } else {
                    *i as usize
                };
                s.chars()
                    .nth(idx)
                    .map(|c| Value::String(c.to_string()))
                    .ok_or_else(|| {
                        GrimScriptError::index_error(line, "string index out of range")
                    })
            }
            (Value::Dict(dict), Value::String(key)) => {
                dict.get(key).cloned().ok_or_else(|| {
                    GrimScriptError::runtime(line, format!("KeyError: '{key}'"))
                })
            }
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "'{}' object is not subscriptable with '{}'",
                    obj.type_name(),
                    index.type_name()
                ),
            )),
        }
    }

    fn get_attribute(
        &self,
        obj: &Value,
        attr: &str,
        line: u32,
    ) -> Result<Value, GrimScriptError> {
        match obj {
            Value::Entity {
                id,
                name,
                entity_type,
            } => match attr {
                "id" => Ok(Value::Int(*id as i64)),
                "name" => Ok(Value::String(name.clone())),
                "type" | "entity_type" => Ok(Value::String(entity_type.clone())),
                _ => Err(GrimScriptError::runtime(
                    line,
                    format!("Entity has no attribute '{attr}'"),
                )),
            },
            Value::Tuple(items) => match attr {
                "x" if items.len() >= 1 => Ok(items[0].clone()),
                "y" if items.len() >= 2 => Ok(items[1].clone()),
                _ => Err(GrimScriptError::runtime(
                    line,
                    format!("tuple has no attribute '{attr}'"),
                )),
            },
            _ => Err(GrimScriptError::type_error(
                line,
                format!(
                    "'{}' object has no attribute '{attr}'",
                    obj.type_name()
                ),
            )),
        }
    }
}
