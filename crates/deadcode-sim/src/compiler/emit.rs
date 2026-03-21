use std::collections::{HashMap, HashSet};

use grimscript_lang::ast::*;

use crate::ir::{CompiledScript, FunctionEntry, Instruction};
use crate::value::SimValue;

use super::builtins::{self, ActionBuiltin, BuiltinKind, InstantEffectBuiltin, QueryBuiltin, StdlibBuiltin};
use super::error::CompileError;
use super::symbol_table::{SymbolTable, VarLocation};

/// Context for break/continue inside loops.
struct LoopContext {
    continue_target: usize,
    break_patches: Vec<usize>,
    /// Indices of `Jump(usize::MAX)` placeholders emitted by `continue` statements
    /// when the continue target was not yet known (for-loops only).
    continue_patches: Vec<usize>,
}

/// Collected function definition from pass 1.
struct FuncDef<'a> {
    name: String,
    params: Vec<String>,
    body: &'a [Statement],
}

pub struct Compiler<'a> {
    instructions: Vec<Instruction>,
    functions: Vec<FunctionEntry>,
    symbols: SymbolTable,
    loop_stack: Vec<LoopContext>,
    func_defs: Vec<FuncDef<'a>>,
    /// Temp variable counter for compiler-generated temporaries.
    temp_counter: usize,
    /// Pending call patches: (instruction_index, function_name, source_line).
    pending_calls: Vec<(usize, String, u32)>,
    /// If Some, only these game commands are available. Stdlib is always allowed.
    available_commands: Option<HashSet<String>>,
    /// Custom command name → arg count (from mod definitions).
    custom_commands: HashMap<String, usize>,
}

impl<'a> Compiler<'a> {
    pub fn new(available_commands: Option<HashSet<String>>) -> Self {
        Self {
            instructions: Vec::new(),
            functions: Vec::new(),
            symbols: SymbolTable::new(),
            loop_stack: Vec::new(),
            func_defs: Vec::new(),
            temp_counter: 0,
            pending_calls: Vec::new(),
            available_commands,
            custom_commands: HashMap::new(),
        }
    }

    pub fn with_custom_commands(mut self, custom_commands: HashMap<String, usize>) -> Self {
        self.custom_commands = custom_commands;
        self
    }

    fn check_command_available(&self, name: &str, line: u32) -> Result<(), CompileError> {
        if let Some(ref set) = self.available_commands {
            if !set.contains(name) {
                return Err(CompileError::new(
                    line,
                    format!("'{name}' is not available yet"),
                ));
            }
        }
        Ok(())
    }

    pub fn compile(mut self, program: &'a Program) -> Result<CompiledScript, CompileError> {
        // Pass 1: collect function definitions.
        for stmt in &program.statements {
            if let StmtKind::FunctionDef { name, params, body } = &stmt.kind {
                self.func_defs.push(FuncDef {
                    name: name.clone(),
                    params: params.clone(),
                    body,
                });
            }
        }

        // Pass 2: emit global code (non-FunctionDef statements).
        for stmt in &program.statements {
            if matches!(stmt.kind, StmtKind::FunctionDef { .. }) {
                continue;
            }
            self.compile_stmt(stmt)?;
        }

        // Auto-call main() if defined.
        let main_func = self.func_defs.iter().find(|f| f.name == "main");
        let has_main = main_func.is_some();
        // We'll emit the Call after compiling function bodies (need PC).
        // For now, reserve a placeholder.
        let main_call_patch = if has_main {
            let idx = self.instructions.len();
            self.emit(Instruction::Call(0, 0)); // placeholder
            self.emit(Instruction::Pop); // discard main's return value
            Some(idx)
        } else {
            None
        };

        self.emit(Instruction::Halt);

        // Pass 3: emit function bodies after the Halt.
        let func_defs: Vec<FuncDef<'a>> = std::mem::take(&mut self.func_defs);
        for func_def in &func_defs {
            let func_pc = self.instructions.len();

            self.symbols.push_function_scope();

            // Declare parameters as locals.
            for param in &func_def.params {
                self.symbols.declare(param);
            }

            // Compile body.
            for stmt in func_def.body {
                self.compile_stmt(stmt)?;
            }

            // Implicit return None if body doesn't end with Return.
            let needs_implicit_return = func_def
                .body
                .last()
                .map_or(true, |s| !matches!(s.kind, StmtKind::Return { .. }));
            if needs_implicit_return {
                self.emit(Instruction::LoadConst(SimValue::None));
                self.emit(Instruction::Return);
            }

            let num_locals = self.symbols.pop_function_scope();

            self.functions.push(FunctionEntry {
                name: func_def.name.clone(),
                pc: func_pc,
                num_params: func_def.params.len(),
                num_locals: num_locals - func_def.params.len(),
            });

            // Patch main() call if this is main.
            if func_def.name == "main" {
                if let Some(patch_idx) = main_call_patch {
                    self.instructions[patch_idx] = Instruction::Call(func_pc, 0);
                }
            }
        }

        // Fixup pass: resolve pending function calls.
        for (idx, func_name, line) in &self.pending_calls {
            if let Some(entry) = self.functions.iter().find(|f| f.name == *func_name) {
                if let Instruction::Call(target, _) = &mut self.instructions[*idx] {
                    *target = entry.pc;
                }
            } else {
                return Err(CompileError::new(*line, format!("undefined function: {func_name}")));
            }
        }

        let num_variables = self.symbols.num_globals();
        Ok(CompiledScript {
            instructions: self.instructions,
            functions: self.functions,
            num_variables,
        })
    }

    // -----------------------------------------------------------------------
    // Statement compilation
    // -----------------------------------------------------------------------

    fn compile_stmt(&mut self, stmt: &Statement) -> Result<(), CompileError> {
        match &stmt.kind {
            StmtKind::FunctionDef { .. } => {
                // Already handled in pass 1/3.
                Ok(())
            }

            StmtKind::Assign { target, value } => {
                match target {
                    AssignTarget::Name(name) => {
                        self.compile_expr(value)?;
                        let loc = self.symbols.resolve_or_declare(name);
                        self.emit_store(loc);
                    }
                    AssignTarget::Index { object, index } => {
                        // We need to: load object, compile index, compile value,
                        // StoreIndex (pushes modified collection), store object back.
                        // First, figure out if object is a simple Name we can store back to.
                        let obj_loc = self.resolve_assign_object(object, stmt.line)?;
                        self.emit_load(obj_loc);
                        self.compile_expr(index)?;
                        self.compile_expr(value)?;
                        self.emit(Instruction::StoreIndex);
                        self.emit_store(obj_loc);
                    }
                }
                Ok(())
            }

            StmtKind::AugAssign { target, op, value } => {
                match target {
                    AssignTarget::Name(name) => {
                        let loc = self.symbols.resolve_or_declare(name);
                        self.emit_load(loc);
                        self.compile_expr(value)?;
                        self.emit_aug_op(op);
                        self.emit_store(loc);
                    }
                    AssignTarget::Index { object, index } => {
                        let obj_loc = self.resolve_assign_object(object, stmt.line)?;
                        // Stack layout for StoreIndex: [obj, idx, new_val] (top = new_val)
                        // Emit: obj, idx, obj, idx → Index → current_val, value, aug_op → new_val
                        //   then StoreIndex pops [new_val, idx, obj] and pushes updated obj.
                        self.emit_load(obj_loc);         // [obj]
                        self.compile_expr(index)?;       // [obj, idx]
                        self.emit_load(obj_loc);         // [obj, idx, obj]
                        self.compile_expr(index)?;       // [obj, idx, obj, idx]
                        self.emit(Instruction::Index);   // [obj, idx, current_val]
                        self.compile_expr(value)?;       // [obj, idx, current_val, rhs]
                        self.emit_aug_op(op);            // [obj, idx, new_val]
                        self.emit(Instruction::StoreIndex); // [updated_obj]
                        self.emit_store(obj_loc);
                    }
                }
                Ok(())
            }

            StmtKind::If {
                condition,
                body,
                elif_clauses,
                else_body,
            } => {
                let mut end_patches = Vec::new();

                // if condition
                self.compile_expr(condition)?;
                let false_jump = self.emit_placeholder(Instruction::JumpIfFalse(0));

                // if body
                for s in body {
                    self.compile_stmt(s)?;
                }
                end_patches.push(self.emit_placeholder(Instruction::Jump(0)));

                self.patch_jump(false_jump);

                // elif clauses
                for (elif_cond, elif_body) in elif_clauses {
                    self.compile_expr(elif_cond)?;
                    let elif_false = self.emit_placeholder(Instruction::JumpIfFalse(0));

                    for s in elif_body {
                        self.compile_stmt(s)?;
                    }
                    end_patches.push(self.emit_placeholder(Instruction::Jump(0)));

                    self.patch_jump(elif_false);
                }

                // else body
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.compile_stmt(s)?;
                    }
                }

                // Patch all end jumps.
                let end = self.instructions.len();
                for patch in end_patches {
                    self.patch_jump_to(patch, end);
                }

                Ok(())
            }

            StmtKind::While { condition, body } => {
                let loop_start = self.instructions.len();

                self.compile_expr(condition)?;
                let exit_jump = self.emit_placeholder(Instruction::JumpIfFalse(0));

                self.loop_stack.push(LoopContext {
                    continue_target: loop_start,
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });

                for s in body {
                    self.compile_stmt(s)?;
                }

                self.emit(Instruction::Jump(loop_start));
                self.patch_jump(exit_jump);

                let loop_ctx = self.loop_stack.pop().unwrap();
                let loop_end = self.instructions.len();
                for patch in loop_ctx.break_patches {
                    self.patch_jump_to(patch, loop_end);
                }

                Ok(())
            }

            StmtKind::For { var, iterable, body } => {
                // Desugar: iter_tmp = iterable; idx_tmp = 0
                // while idx_tmp < len(iter_tmp): var = iter_tmp[idx_tmp]; body; idx_tmp += 1
                let iter_name = self.temp_name("__iter");
                let idx_name = self.temp_name("__idx");

                self.compile_expr(iterable)?;
                let iter_loc = self.symbols.resolve_or_declare(&iter_name);
                self.emit_store(iter_loc);

                self.emit(Instruction::LoadConst(SimValue::Int(0)));
                let idx_loc = self.symbols.resolve_or_declare(&idx_name);
                self.emit_store(idx_loc);

                let loop_start = self.instructions.len();

                // idx < len(iter)
                self.emit_load(idx_loc);
                self.emit_load(iter_loc);
                self.emit(Instruction::Len);
                self.emit(Instruction::CmpLt);
                let exit_jump = self.emit_placeholder(Instruction::JumpIfFalse(0));

                // var = iter[idx]
                self.emit_load(iter_loc);
                self.emit_load(idx_loc);
                self.emit(Instruction::Index);
                let var_loc = self.symbols.resolve_or_declare(var);
                self.emit_store(var_loc);

                // Use usize::MAX as sentinel — continue target is not yet known.
                self.loop_stack.push(LoopContext {
                    continue_target: usize::MAX,
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });

                for s in body {
                    self.compile_stmt(s)?;
                }

                // Increment block (continue jumps here).
                let increment_start = self.instructions.len();

                self.emit_load(idx_loc);
                self.emit(Instruction::LoadConst(SimValue::Int(1)));
                self.emit(Instruction::Add);
                self.emit_store(idx_loc);
                self.emit(Instruction::Jump(loop_start));

                self.patch_jump(exit_jump);

                let loop_ctx = self.loop_stack.pop().unwrap();
                let loop_end = self.instructions.len();
                for patch in loop_ctx.break_patches {
                    self.patch_jump_to(patch, loop_end);
                }
                // Patch all continue placeholders to point to increment_start.
                for patch in loop_ctx.continue_patches {
                    self.patch_jump_to(patch, increment_start);
                }

                Ok(())
            }

            StmtKind::Return { value } => {
                if let Some(val) = value {
                    self.compile_expr(val)?;
                } else {
                    self.emit(Instruction::LoadConst(SimValue::None));
                }
                self.emit(Instruction::Return);
                Ok(())
            }

            StmtKind::Break => {
                if self.loop_stack.is_empty() {
                    return Err(CompileError::new(stmt.line, "'break' outside loop"));
                }
                let patch = self.emit_placeholder(Instruction::Jump(0));
                self.loop_stack.last_mut().unwrap().break_patches.push(patch);
                Ok(())
            }

            StmtKind::Continue => {
                let target = self
                    .loop_stack
                    .last()
                    .ok_or_else(|| CompileError::new(stmt.line, "'continue' outside loop"))?
                    .continue_target;
                if target == usize::MAX {
                    // For-loop: target not yet known, emit placeholder and record for patching.
                    let patch = self.emit_placeholder(Instruction::Jump(usize::MAX));
                    self.loop_stack.last_mut().unwrap().continue_patches.push(patch);
                } else {
                    self.emit(Instruction::Jump(target));
                }
                Ok(())
            }

            StmtKind::Pass => Ok(()),

            StmtKind::Expr(expr) => {
                let is_void = self.is_void_expr(expr);
                self.compile_expr(expr)?;
                if !is_void {
                    self.emit(Instruction::Pop);
                }
                Ok(())
            }
        }
    }

    // -----------------------------------------------------------------------
    // Expression compilation
    // -----------------------------------------------------------------------

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::Integer(n) => {
                self.emit(Instruction::LoadConst(SimValue::Int(*n)));
            }
            ExprKind::Float(_) => {
                return Err(CompileError::new(
                    expr.line,
                    "simulation mode does not support floating-point values",
                ));
            }
            ExprKind::StringLit(s) => {
                self.emit(Instruction::LoadConst(SimValue::Str(s.clone())));
            }
            ExprKind::Bool(b) => {
                self.emit(Instruction::LoadConst(SimValue::Bool(*b)));
            }
            ExprKind::NoneLit => {
                self.emit(Instruction::LoadConst(SimValue::None));
            }

            ExprKind::Name(name) => {
                if let Some(loc) = self.symbols.resolve(name) {
                    self.emit_load(loc);
                } else {
                    return Err(CompileError::new(
                        expr.line,
                        format!("undefined name '{name}'"),
                    ));
                }
            }

            ExprKind::List(items) => {
                for item in items {
                    self.compile_expr(item)?;
                }
                self.emit(Instruction::BuildList(items.len()));
            }

            ExprKind::ListComp {
                expr: comp_expr,
                var,
                iter,
                condition,
            } => {
                self.compile_list_comp(comp_expr, var, iter, condition.as_deref(), expr.line)?;
            }

            ExprKind::BinOp { left, op, right } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match op {
                    BinOp::Add => self.emit(Instruction::Add),
                    BinOp::Sub => self.emit(Instruction::Sub),
                    BinOp::Mul => self.emit(Instruction::Mul),
                    BinOp::Div | BinOp::FloorDiv => self.emit(Instruction::Div),
                    BinOp::Mod => self.emit(Instruction::Mod),
                };
            }

            ExprKind::UnaryOp { op, operand } => {
                self.compile_expr(operand)?;
                match op {
                    UnaryOp::Neg => self.emit(Instruction::Negate),
                    UnaryOp::Not => self.emit(Instruction::Not),
                };
            }

            ExprKind::BoolOp { op, left, right } => {
                self.compile_expr(left)?;
                self.emit(Instruction::Dup);
                let short_circuit = match op {
                    BoolOpKind::And => self.emit_placeholder(Instruction::JumpIfFalse(0)),
                    BoolOpKind::Or => self.emit_placeholder(Instruction::JumpIfTrue(0)),
                };
                self.emit(Instruction::Pop);
                self.compile_expr(right)?;
                self.patch_jump(short_circuit);
            }

            ExprKind::Compare { left, op, right } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match op {
                    CmpOp::Eq => self.emit(Instruction::CmpEq),
                    CmpOp::NotEq => self.emit(Instruction::CmpNe),
                    CmpOp::Lt => self.emit(Instruction::CmpLt),
                    CmpOp::Gt => self.emit(Instruction::CmpGt),
                    CmpOp::LtEq => self.emit(Instruction::CmpLe),
                    CmpOp::GtEq => self.emit(Instruction::CmpGe),
                    CmpOp::In => self.emit(Instruction::Contains),
                    CmpOp::NotIn => self.emit(Instruction::NotContains),
                };
            }

            ExprKind::IsNone { expr: inner, negated } => {
                self.compile_expr(inner)?;
                if *negated {
                    self.emit(Instruction::IsNotNone);
                } else {
                    self.emit(Instruction::IsNone);
                }
            }

            ExprKind::Index { object, index } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit(Instruction::Index);
            }

            ExprKind::Attribute { object, attr } => {
                self.compile_expr(object)?;
                self.emit(Instruction::LoadConst(SimValue::Str(attr.clone())));
                self.emit(Instruction::GetAttr);
            }

            ExprKind::Call { func, args } => {
                self.compile_call(func, args, expr.line)?;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Call compilation
    // -----------------------------------------------------------------------

    fn compile_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
        line: u32,
    ) -> Result<(), CompileError> {
        match &func.kind {
            ExprKind::Name(name) => {
                // Check for special synthetic calls.
                if name == "__tuple__" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(Instruction::BuildList(args.len()));
                    return Ok(());
                }
                if name == "__dict__" {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit(Instruction::BuildDict(args.len() / 2));
                    return Ok(());
                }

                // Check builtins (including custom commands from mods).
                match builtins::classify_with_custom(name, &self.custom_commands) {
                    BuiltinKind::Query(q) => {
                        self.check_command_available(name, line)?;
                        self.compile_query_call(&q, args, line)?;
                    }
                    BuiltinKind::Action(a) => {
                        self.check_command_available(name, line)?;
                        self.compile_action_call(&a, args, line)?;
                    }
                    BuiltinKind::Stdlib(s) => {
                        self.compile_stdlib_call(&s, args, line)?;
                    }
                    BuiltinKind::InstantEffect(ie) => {
                        self.check_command_available(name, line)?;
                        self.compile_instant_effect_call(&ie, args, line)?;
                    }
                    BuiltinKind::CustomAction { name: cmd_name, num_args } => {
                        self.check_command_available(&cmd_name, line)?;
                        if args.len() != num_args {
                            return Err(CompileError::new(
                                line,
                                format!(
                                    "{}() takes {} argument(s), got {}",
                                    cmd_name, num_args, args.len()
                                ),
                            ));
                        }
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(Instruction::ActionCustom(cmd_name));
                    }
                    BuiltinKind::NotBuiltin => {
                        // User-defined function call.
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        // Look up function PC. We may not know it yet (forward reference).
                        // Use function name to find entry.
                        let func_pc = self.find_func_pc(name);
                        match func_pc {
                            Some(pc) => {
                                self.emit(Instruction::Call(pc, args.len()));
                            }
                            None => {
                                let idx = self.instructions.len();
                                self.emit(Instruction::Call(usize::MAX, args.len()));
                                self.pending_calls.push((idx, name.clone(), line));
                            }
                        }
                    }
                }
            }

            ExprKind::Attribute { object, attr } => {
                self.compile_method_call(object, attr, args, line)?;
            }

            _ => {
                return Err(CompileError::new(
                    line,
                    "only named function calls are supported",
                ));
            }
        }
        Ok(())
    }

    fn compile_query_call(
        &mut self,
        q: &QueryBuiltin,
        args: &[Expr],
        line: u32,
    ) -> Result<(), CompileError> {
        let expected = builtins::query_expected_args(q);
        if args.is_empty() && builtins::query_takes_implicit_self(q) {
            // Auto-push self.
            self.emit_load(VarLocation::Global(0)); // self is slot 0
        } else if args.len() != expected {
            return Err(CompileError::new(
                line,
                format!(
                    "{}() takes {} argument(s), got {}",
                    query_name(q),
                    expected,
                    args.len()
                ),
            ));
        } else {
            for arg in args {
                self.compile_expr(arg)?;
            }
        }
        self.emit(builtins::query_instruction(q));
        Ok(())
    }

    fn compile_action_call(
        &mut self,
        a: &ActionBuiltin,
        args: &[Expr],
        line: u32,
    ) -> Result<(), CompileError> {
        let expected = builtins::action_expected_args(a);
        if args.len() != expected {
            return Err(CompileError::new(
                line,
                format!(
                    "{}() takes {} argument(s), got {}",
                    action_name(a),
                    expected,
                    args.len()
                ),
            ));
        }
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit(builtins::action_instruction(a));
        Ok(())
    }

    fn compile_instant_effect_call(
        &mut self,
        ie: &InstantEffectBuiltin,
        args: &[Expr],
        line: u32,
    ) -> Result<(), CompileError> {
        let expected = builtins::instant_effect_expected_args(ie);
        if args.len() != expected {
            return Err(CompileError::new(
                line,
                format!(
                    "{}() takes {} argument(s), got {}",
                    instant_effect_name(ie),
                    expected,
                    args.len()
                ),
            ));
        }
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit(builtins::instant_effect_instruction(ie));
        Ok(())
    }

    fn compile_stdlib_call(
        &mut self,
        s: &StdlibBuiltin,
        args: &[Expr],
        line: u32,
    ) -> Result<(), CompileError> {
        match s {
            StdlibBuiltin::Print => {
                if args.is_empty() {
                    self.emit(Instruction::LoadConst(SimValue::Str(String::new())));
                } else if args.len() == 1 {
                    self.compile_expr(&args[0])?;
                    self.emit(Instruction::StrCast);
                } else {
                    // Multi-arg print: convert each to string, join with spaces.
                    self.compile_expr(&args[0])?;
                    self.emit(Instruction::StrCast);
                    for arg in &args[1..] {
                        self.emit(Instruction::LoadConst(SimValue::Str(" ".to_string())));
                        self.emit(Instruction::Add);
                        self.compile_expr(arg)?;
                        self.emit(Instruction::StrCast);
                        self.emit(Instruction::Add);
                    }
                }
                self.emit(Instruction::Print);
            }
            StdlibBuiltin::Len => {
                self.expect_args("len", args, 1, line)?;
                self.compile_expr(&args[0])?;
                self.emit(Instruction::Len);
            }
            StdlibBuiltin::Range => {
                if args.is_empty() || args.len() > 3 {
                    return Err(CompileError::new(line, "range() takes 1 to 3 arguments"));
                }
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit(Instruction::Range(args.len() as u8));
            }
            StdlibBuiltin::Abs => {
                self.expect_args("abs", args, 1, line)?;
                self.compile_expr(&args[0])?;
                self.emit(Instruction::Abs);
            }
            StdlibBuiltin::Min => {
                self.expect_args("min", args, 2, line)?;
                self.compile_expr(&args[0])?;
                self.compile_expr(&args[1])?;
                self.emit(Instruction::Min2);
            }
            StdlibBuiltin::Max => {
                self.expect_args("max", args, 2, line)?;
                self.compile_expr(&args[0])?;
                self.compile_expr(&args[1])?;
                self.emit(Instruction::Max2);
            }
            StdlibBuiltin::Int => {
                self.expect_args("int", args, 1, line)?;
                self.compile_expr(&args[0])?;
                self.emit(Instruction::IntCast);
            }
            StdlibBuiltin::Str => {
                self.expect_args("str", args, 1, line)?;
                self.compile_expr(&args[0])?;
                self.emit(Instruction::StrCast);
            }
            StdlibBuiltin::Type => {
                self.expect_args("type", args, 1, line)?;
                self.compile_expr(&args[0])?;
                self.emit(Instruction::TypeOf);
            }
            StdlibBuiltin::Float => {
                return Err(CompileError::new(
                    line,
                    "float() is not supported in simulation mode",
                ));
            }
            StdlibBuiltin::Percent => {
                self.expect_args("percent", args, 2, line)?;
                self.compile_expr(&args[0])?;
                self.compile_expr(&args[1])?;
                self.emit(Instruction::Percent);
            }
            StdlibBuiltin::Scale => {
                self.expect_args("scale", args, 3, line)?;
                self.compile_expr(&args[0])?;
                self.compile_expr(&args[1])?;
                self.compile_expr(&args[2])?;
                self.emit(Instruction::Scale);
            }
        }
        Ok(())
    }

    fn compile_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
        line: u32,
    ) -> Result<(), CompileError> {
        match method {
            // List methods
            "append" => {
                // list.append(x) → load list, push x, ListAppend, store list back
                let obj_loc = self.resolve_assign_object(object, line)?;
                self.emit_load(obj_loc);
                self.expect_args("append", args, 1, line)?;
                self.compile_expr(&args[0])?;
                self.emit(Instruction::ListAppend);
                self.emit_store(obj_loc);
                // append returns None
                self.emit(Instruction::LoadConst(SimValue::None));
            }
            // Dict methods
            "keys" => {
                self.compile_expr(object)?;
                self.emit(Instruction::DictKeys);
            }
            "values" => {
                self.compile_expr(object)?;
                self.emit(Instruction::DictValues);
            }
            "items" => {
                self.compile_expr(object)?;
                self.emit(Instruction::DictItems);
            }
            "get" => {
                self.compile_expr(object)?;
                if args.len() == 1 {
                    self.compile_expr(&args[0])?;
                    self.emit(Instruction::LoadConst(SimValue::None));
                } else if args.len() == 2 {
                    self.compile_expr(&args[0])?;
                    self.compile_expr(&args[1])?;
                } else {
                    return Err(CompileError::new(line, "get() takes 1 or 2 arguments"));
                }
                self.emit(Instruction::DictGet);
            }
            _ => {
                // Try as a game builtin with the object as first arg.
                // e.g., entity.get_health() → get_health(entity)
                self.check_command_available(method, line)?;
                match builtins::classify(method) {
                    BuiltinKind::Query(q) => {
                        self.compile_expr(object)?;
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(builtins::query_instruction(&q));
                    }
                    BuiltinKind::Action(a) => {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit(builtins::action_instruction(&a));
                    }
                    _ => {
                        return Err(CompileError::new(
                            line,
                            format!("unknown method '{method}'"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // List comprehension
    // -----------------------------------------------------------------------

    fn compile_list_comp(
        &mut self,
        expr: &Expr,
        var: &str,
        iter: &Expr,
        condition: Option<&Expr>,
        _line: u32,
    ) -> Result<(), CompileError> {
        let result_name = self.temp_name("__comp_result");
        let iter_name = self.temp_name("__comp_iter");
        let idx_name = self.temp_name("__comp_idx");

        // result = []
        self.emit(Instruction::BuildList(0));
        let result_loc = self.symbols.resolve_or_declare(&result_name);
        self.emit_store(result_loc);

        // iter_tmp = iterable
        self.compile_expr(iter)?;
        let iter_loc = self.symbols.resolve_or_declare(&iter_name);
        self.emit_store(iter_loc);

        // idx = 0
        self.emit(Instruction::LoadConst(SimValue::Int(0)));
        let idx_loc = self.symbols.resolve_or_declare(&idx_name);
        self.emit_store(idx_loc);

        let loop_start = self.instructions.len();

        // while idx < len(iter)
        self.emit_load(idx_loc);
        self.emit_load(iter_loc);
        self.emit(Instruction::Len);
        self.emit(Instruction::CmpLt);
        let exit_jump = self.emit_placeholder(Instruction::JumpIfFalse(0));

        // var = iter[idx]
        self.emit_load(iter_loc);
        self.emit_load(idx_loc);
        self.emit(Instruction::Index);
        let var_loc = self.symbols.resolve_or_declare(var);
        self.emit_store(var_loc);

        let skip_jump = if let Some(cond) = condition {
            self.compile_expr(cond)?;
            Some(self.emit_placeholder(Instruction::JumpIfFalse(0)))
        } else {
            None
        };

        // result.append(expr)
        self.emit_load(result_loc);
        self.compile_expr(expr)?;
        self.emit(Instruction::ListAppend);
        self.emit_store(result_loc);

        if let Some(skip) = skip_jump {
            self.patch_jump(skip);
        }

        // idx += 1
        self.emit_load(idx_loc);
        self.emit(Instruction::LoadConst(SimValue::Int(1)));
        self.emit(Instruction::Add);
        self.emit_store(idx_loc);
        self.emit(Instruction::Jump(loop_start));

        self.patch_jump(exit_jump);

        // Push result onto stack.
        self.emit_load(result_loc);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    fn emit_placeholder(&mut self, inst: Instruction) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(inst);
        idx
    }

    fn patch_jump(&mut self, idx: usize) {
        let target = self.instructions.len();
        self.patch_jump_to(idx, target);
    }

    fn patch_jump_to(&mut self, idx: usize, target: usize) {
        match &mut self.instructions[idx] {
            Instruction::Jump(t)
            | Instruction::JumpIfFalse(t)
            | Instruction::JumpIfTrue(t) => *t = target,
            Instruction::Call(t, _) => *t = target,
            _ => {}
        }
    }

    fn emit_load(&mut self, loc: VarLocation) {
        match loc {
            VarLocation::Global(slot) => self.emit(Instruction::LoadVar(slot)),
            VarLocation::Local(offset) => self.emit(Instruction::LoadLocal(offset)),
        }
    }

    fn emit_store(&mut self, loc: VarLocation) {
        match loc {
            VarLocation::Global(slot) => self.emit(Instruction::StoreVar(slot)),
            VarLocation::Local(offset) => self.emit(Instruction::StoreLocal(offset)),
        }
    }

    fn emit_aug_op(&mut self, op: &AugOp) {
        match op {
            AugOp::Add => self.emit(Instruction::Add),
            AugOp::Sub => self.emit(Instruction::Sub),
            AugOp::Mul => self.emit(Instruction::Mul),
            AugOp::Div => self.emit(Instruction::Div),
        }
    }

    fn resolve_assign_object(
        &self,
        object: &Expr,
        line: u32,
    ) -> Result<VarLocation, CompileError> {
        match &object.kind {
            ExprKind::Name(name) => self.symbols.resolve(name).ok_or_else(|| {
                CompileError::new(line, format!("undefined name '{name}'"))
            }),
            _ => Err(CompileError::new(
                line,
                "can only assign to index of a named variable",
            )),
        }
    }

    fn find_func_pc(&self, name: &str) -> Option<usize> {
        self.functions.iter().find(|f| f.name == name).map(|f| f.pc)
    }

    fn is_void_expr(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Call { func, .. } => {
                if let ExprKind::Name(name) = &func.kind {
                    matches!(
                        builtins::classify_with_custom(name, &self.custom_commands),
                        BuiltinKind::Action(_)
                            | BuiltinKind::CustomAction { .. }
                            | BuiltinKind::Stdlib(StdlibBuiltin::Print)
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn temp_name(&mut self, prefix: &str) -> String {
        let n = self.temp_counter;
        self.temp_counter += 1;
        format!("{prefix}_{n}")
    }

    fn expect_args(
        &self,
        name: &str,
        args: &[Expr],
        expected: usize,
        line: u32,
    ) -> Result<(), CompileError> {
        if args.len() != expected {
            Err(CompileError::new(
                line,
                format!("{name}() takes {expected} argument(s), got {}", args.len()),
            ))
        } else {
            Ok(())
        }
    }
}

fn query_name(q: &QueryBuiltin) -> &'static str {
    match q {
        QueryBuiltin::Scan => "scan",
        QueryBuiltin::Nearest => "nearest",
        QueryBuiltin::Distance => "distance",
        QueryBuiltin::GetPos => "get_pos",
        QueryBuiltin::GetHealth => "get_health",
        QueryBuiltin::GetShield => "get_shield",
        QueryBuiltin::GetTarget => "get_target",
        QueryBuiltin::HasTarget => "has_target",
        QueryBuiltin::GetType => "get_type",
        QueryBuiltin::GetName => "get_name",
        QueryBuiltin::GetOwner => "get_owner",
        QueryBuiltin::GetResource => "get_resource",
        QueryBuiltin::GetStat => "get_stat",
        QueryBuiltin::GetTypes => "get_types",
        QueryBuiltin::HasType => "has_type",
    }
}

fn instant_effect_name(ie: &InstantEffectBuiltin) -> &'static str {
    match ie {
        InstantEffectBuiltin::GainResource => "gain_resource",
        InstantEffectBuiltin::TrySpendResource => "try_spend_resource",
    }
}

fn action_name(a: &ActionBuiltin) -> &'static str {
    match a {
        ActionBuiltin::Move => "move",
        ActionBuiltin::Attack => "attack",
        ActionBuiltin::Flee => "flee",
        ActionBuiltin::Wait => "wait",
        ActionBuiltin::SetTarget => "set_target",
    }
}
