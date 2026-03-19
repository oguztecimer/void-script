mod builtins;
mod emit;
pub mod error;
mod symbol_table;

use std::collections::{HashMap, HashSet};

use grimscript_lang::ast::Program;

use crate::entity::EntityId;
use crate::ir::CompiledScript;
use crate::value::SimValue;

pub use error::CompileError;

/// Compile a GrimScript AST into simulation IR.
pub fn compile(
    program: &Program,
    available_commands: Option<HashSet<String>>,
) -> Result<CompiledScript, CompileError> {
    compile_with_custom(program, available_commands, HashMap::new())
}

/// Compile a GrimScript AST into simulation IR, with custom command definitions.
pub fn compile_with_custom(
    program: &Program,
    available_commands: Option<HashSet<String>>,
    custom_commands: HashMap<String, usize>,
) -> Result<CompiledScript, CompileError> {
    let compiler = emit::Compiler::new(available_commands)
        .with_custom_commands(custom_commands);
    let mut script = compiler.compile(program)?;
    emit::fixup_calls(&mut script);
    Ok(script)
}

/// Create the initial variables vector for an entity.
/// Slot 0 = self (EntityRef for the given entity).
pub fn initial_variables(entity_id: EntityId, num_globals: usize) -> Vec<SimValue> {
    let mut vars = vec![SimValue::None; num_globals.max(1)];
    vars[0] = SimValue::EntityRef(entity_id);
    vars
}

/// Parse source code and compile to IR in one step.
pub fn compile_source(source: &str) -> Result<CompiledScript, String> {
    compile_source_with(source, None)
}

/// Parse source code and compile to IR, with optional command gating.
pub fn compile_source_with(
    source: &str,
    available_commands: Option<HashSet<String>>,
) -> Result<CompiledScript, String> {
    compile_source_full(source, available_commands, HashMap::new())
}

/// Parse source code and compile to IR, with command gating and custom commands.
pub fn compile_source_full(
    source: &str,
    available_commands: Option<HashSet<String>>,
    custom_commands: HashMap<String, usize>,
) -> Result<CompiledScript, String> {
    let tokens = grimscript_lang::lexer::Lexer::new(source).tokenize();
    let program = grimscript_lang::parser::Parser::new(tokens)
        .parse()
        .map_err(|e| format!("parse error (line {}): {}", e.line, e.message))?;
    compile_with_custom(&program, available_commands, custom_commands)
        .map_err(|e| format!("compile error (line {}): {}", e.line, e.message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::UnitAction;
    use crate::entity::ScriptState;
    use crate::executor;
    use crate::ir::Instruction;
    use crate::world::SimWorld;

    /// Helper: compile source, execute on a fresh world, return final state + action.
    fn compile_and_run(source: &str) -> (ScriptState, Option<UnitAction>) {
        let script = compile_source(source).expect("compilation failed");
        let mut world = SimWorld::new(42);
        let eid = world.spawn_entity("skeleton".into(), "test".into(), 100);
        let num_vars = script.num_variables;
        let mut state = ScriptState::new(script, num_vars);
        // Set self = EntityRef for the entity.
        if !state.variables.is_empty() {
            state.variables[0] = SimValue::EntityRef(eid);
        }
        let action = executor::execute_unit(eid, &mut state, &world).unwrap();
        (state, action)
    }

    /// Helper: compile source, execute until halt, return variables.
    fn run_to_completion(source: &str) -> Vec<SimValue> {
        let (state, _) = compile_and_run(source);
        state.variables
    }

    #[test]
    fn simple_assignment() {
        let vars = run_to_completion("x = 42");
        // slot 0 = self, slot 1 = x
        assert_eq!(vars.get(1), Some(&SimValue::Int(42)));
    }

    #[test]
    fn arithmetic_expression() {
        let vars = run_to_completion("x = 10 + 5 * 3");
        // 10 + (5*3) = 25 — but our compiler emits left-to-right with operator precedence
        // handled by the parser. The parser produces correct AST: BinOp(10, +, BinOp(5, *, 3))
        assert_eq!(vars.get(1), Some(&SimValue::Int(25)));
    }

    #[test]
    fn string_concatenation() {
        let vars = run_to_completion("x = \"hello\" + \" \" + \"world\"");
        assert_eq!(vars.get(1), Some(&SimValue::Str("hello world".to_string())));
    }

    #[test]
    fn boolean_logic() {
        let vars = run_to_completion("x = True and False\ny = True or False");
        assert_eq!(vars.get(1), Some(&SimValue::Bool(false)));
        assert_eq!(vars.get(2), Some(&SimValue::Bool(true)));
    }

    #[test]
    fn if_else() {
        let vars = run_to_completion("x = 0\nif True:\n    x = 1\nelse:\n    x = 2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(1)));
    }

    #[test]
    fn if_false_branch() {
        let vars = run_to_completion("x = 0\nif False:\n    x = 1\nelse:\n    x = 2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(2)));
    }

    #[test]
    fn while_loop() {
        let vars = run_to_completion("x = 0\nwhile x < 10:\n    x = x + 1");
        assert_eq!(vars.get(1), Some(&SimValue::Int(10)));
    }

    #[test]
    fn while_with_break() {
        let vars = run_to_completion("x = 0\nwhile True:\n    x = x + 1\n    if x == 5:\n        break");
        assert_eq!(vars.get(1), Some(&SimValue::Int(5)));
    }

    #[test]
    fn for_loop_range() {
        let vars = run_to_completion("total = 0\nfor i in range(5):\n    total = total + i");
        // 0 + 1 + 2 + 3 + 4 = 10
        assert_eq!(vars.get(1), Some(&SimValue::Int(10)));
    }

    #[test]
    fn for_loop_list() {
        let vars = run_to_completion("total = 0\nfor x in [10, 20, 30]:\n    total = total + x");
        assert_eq!(vars.get(1), Some(&SimValue::Int(60)));
    }

    #[test]
    fn function_def_and_call() {
        let vars = run_to_completion("def add(a, b):\n    return a + b\nx = add(3, 7)");
        assert_eq!(vars.get(1), Some(&SimValue::Int(10)));
    }

    #[test]
    fn function_with_main() {
        let vars = run_to_completion("def main():\n    pass");
        // Should not crash — main is auto-called.
        assert!(!vars.is_empty());
    }

    #[test]
    fn recursive_function() {
        let src = "def factorial(n):\n    if n <= 1:\n        return 1\n    return n * factorial(n - 1)\nx = factorial(5)";
        let vars = run_to_completion(src);
        assert_eq!(vars.get(1), Some(&SimValue::Int(120)));
    }

    #[test]
    fn list_operations() {
        let vars = run_to_completion("x = [1, 2, 3]\ny = x[1]");
        assert_eq!(vars.get(2), Some(&SimValue::Int(2)));
    }

    #[test]
    fn len_builtin() {
        let vars = run_to_completion("x = len([1, 2, 3])");
        assert_eq!(vars.get(1), Some(&SimValue::Int(3)));
    }

    #[test]
    fn range_builtin() {
        let vars = run_to_completion("x = range(5)");
        assert_eq!(
            vars.get(1),
            Some(&SimValue::List(vec![
                SimValue::Int(0),
                SimValue::Int(1),
                SimValue::Int(2),
                SimValue::Int(3),
                SimValue::Int(4),
            ]))
        );
    }

    #[test]
    fn abs_builtin() {
        let vars = run_to_completion("x = abs(-42)");
        assert_eq!(vars.get(1), Some(&SimValue::Int(42)));
    }

    #[test]
    fn min_max_builtin() {
        let vars = run_to_completion("x = min(3, 7)\ny = max(3, 7)");
        assert_eq!(vars.get(1), Some(&SimValue::Int(3)));
        assert_eq!(vars.get(2), Some(&SimValue::Int(7)));
    }

    #[test]
    fn str_int_type_builtins() {
        let vars = run_to_completion("x = str(42)\ny = int(\"99\")\nz = type(True)");
        assert_eq!(vars.get(1), Some(&SimValue::Str("42".to_string())));
        assert_eq!(vars.get(2), Some(&SimValue::Int(99)));
        assert_eq!(vars.get(3), Some(&SimValue::Str("bool".to_string())));
    }

    #[test]
    fn float_literal_error() {
        let result = compile_source("x = 3.14");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("floating-point"));
    }

    #[test]
    fn break_outside_loop_error() {
        let result = compile_source("break");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside loop"));
    }

    #[test]
    fn action_move_yields() {
        let (state, action) = compile_and_run("move(500)");
        assert!(state.yielded);
        assert!(matches!(action, Some(UnitAction::Move { target_pos: 500 })));
    }

    #[test]
    fn action_wait_yields() {
        let (state, action) = compile_and_run("wait()");
        assert!(state.yielded);
        assert!(matches!(action, Some(UnitAction::Wait)));
    }

    #[test]
    fn query_scan() {
        let script = compile_source("targets = scan(\"fighter\")").unwrap();
        // Verify QueryScan is in the instruction stream.
        assert!(script.instructions.iter().any(|i| matches!(i, Instruction::QueryScan)));
    }

    #[test]
    fn is_none_check() {
        let vars = run_to_completion("x = None\ny = x is None\nz = x is not None");
        assert_eq!(vars.get(2), Some(&SimValue::Bool(true)));
        assert_eq!(vars.get(3), Some(&SimValue::Bool(false)));
    }

    #[test]
    fn augmented_assignment() {
        let vars = run_to_completion("x = 10\nx += 5\nx -= 3");
        assert_eq!(vars.get(1), Some(&SimValue::Int(12)));
    }

    #[test]
    fn comparison_operators() {
        let vars = run_to_completion("a = 5 == 5\nb = 5 != 3\nc = 3 < 5\nd = 5 >= 5");
        assert_eq!(vars.get(1), Some(&SimValue::Bool(true)));
        assert_eq!(vars.get(2), Some(&SimValue::Bool(true)));
        assert_eq!(vars.get(3), Some(&SimValue::Bool(true)));
        assert_eq!(vars.get(4), Some(&SimValue::Bool(true)));
    }

    #[test]
    fn unary_operators() {
        let vars = run_to_completion("x = -42\ny = not True");
        assert_eq!(vars.get(1), Some(&SimValue::Int(-42)));
        assert_eq!(vars.get(2), Some(&SimValue::Bool(false)));
    }

    #[test]
    fn list_append_method() {
        let vars = run_to_completion("x = [1, 2]\nx.append(3)");
        assert_eq!(
            vars.get(1),
            Some(&SimValue::List(vec![
                SimValue::Int(1),
                SimValue::Int(2),
                SimValue::Int(3),
            ]))
        );
    }

    #[test]
    fn dict_methods() {
        let vars = run_to_completion("d = {\"a\": 1, \"b\": 2}\nk = d.keys()\nv = d.get(\"c\", 99)");
        assert_eq!(
            vars.get(2),
            Some(&SimValue::List(vec![
                SimValue::Str("a".to_string()),
                SimValue::Str("b".to_string()),
            ]))
        );
        assert_eq!(vars.get(3), Some(&SimValue::Int(99)));
    }

    #[test]
    fn list_comprehension() {
        let vars = run_to_completion("x = [i * 2 for i in range(4)]");
        assert_eq!(
            vars.get(1),
            Some(&SimValue::List(vec![
                SimValue::Int(0),
                SimValue::Int(2),
                SimValue::Int(4),
                SimValue::Int(6),
            ]))
        );
    }

    #[test]
    fn list_comp_with_condition() {
        let vars = run_to_completion("x = [i for i in range(6) if i > 2]");
        assert_eq!(
            vars.get(1),
            Some(&SimValue::List(vec![
                SimValue::Int(3),
                SimValue::Int(4),
                SimValue::Int(5),
            ]))
        );
    }

    #[test]
    fn elif_chain() {
        let src = "x = 5\nif x > 10:\n    y = 1\nelif x > 3:\n    y = 2\nelse:\n    y = 3";
        let vars = run_to_completion(src);
        assert_eq!(vars.get(2), Some(&SimValue::Int(2)));
    }

    #[test]
    fn nested_function_calls() {
        let src = "def double(n):\n    return n * 2\ndef quad(n):\n    return double(double(n))\nx = quad(3)";
        let vars = run_to_completion(src);
        assert_eq!(vars.get(1), Some(&SimValue::Int(12)));
    }

    #[test]
    fn self_is_entity_ref() {
        // `self` should resolve to EntityRef at slot 0.
        let script = compile_source("x = self").unwrap();
        assert!(script.instructions.iter().any(|i| matches!(i, Instruction::LoadVar(0))));
    }

    #[test]
    fn implicit_self_query() {
        // 0-arg query auto-pushes self.
        let script = compile_source("h = get_health()").unwrap();
        // Should have LoadVar(0) [self] followed by QueryGetHealth.
        let instrs: Vec<_> = script.instructions.iter().collect();
        let has_self_load = instrs.windows(2).any(|w| {
            matches!(w[0], Instruction::LoadVar(0)) && matches!(w[1], Instruction::QueryGetHealth)
        });
        assert!(has_self_load);
    }

    #[test]
    fn deterministic_compilation() {
        let src = "x = 1\ny = 2\nz = x + y";
        let a = compile_source(src).unwrap();
        let b = compile_source(src).unwrap();
        assert_eq!(a.instructions.len(), b.instructions.len());
        assert_eq!(a.num_variables, b.num_variables);
    }

    #[test]
    fn while_true_move_oscillate() {
        // The classic test from the plan: while True: move(100); move(0)
        // Should yield on first move(100).
        let src = "while True:\n    move(100)\n    move(0)";
        let (state, action) = compile_and_run(src);
        assert!(state.yielded);
        assert!(matches!(action, Some(UnitAction::Move { target_pos: 100 })));
    }

    #[test]
    fn print_builtin() {
        let (_state, action) = compile_and_run("print(\"hello\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "hello"));
    }

    #[test]
    fn print_multi_arg() {
        let (_state, action) = compile_and_run("print(\"x\", 42)");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "x 42"));
    }

    #[test]
    fn index_assignment() {
        let vars = run_to_completion("x = [1, 2, 3]\nx[1] = 99");
        assert_eq!(
            vars.get(1),
            Some(&SimValue::List(vec![
                SimValue::Int(1),
                SimValue::Int(99),
                SimValue::Int(3),
            ]))
        );
    }

    #[test]
    fn pass_statement() {
        let vars = run_to_completion("pass\nx = 1");
        assert_eq!(vars.get(1), Some(&SimValue::Int(1)));
    }

    #[test]
    fn short_circuit_and() {
        // and should return first falsy value
        let vars = run_to_completion("x = 0 and 42");
        assert_eq!(vars.get(1), Some(&SimValue::Int(0)));
    }

    #[test]
    fn short_circuit_or() {
        // or should return first truthy value
        let vars = run_to_completion("x = 0 or 42");
        assert_eq!(vars.get(1), Some(&SimValue::Int(42)));
    }
}
