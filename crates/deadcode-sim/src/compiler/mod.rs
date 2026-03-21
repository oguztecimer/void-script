mod builtins;
mod emit;
pub mod error;
mod symbol_table;

use std::collections::{HashMap, HashSet};

use grimscript_lang::ast::Program;

use crate::entity::EntityId;
use crate::ir::CompiledScript;
use crate::value::SimValue;

pub use builtins::CommandMeta;
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
    custom_commands: HashMap<String, CommandMeta>,
) -> Result<CompiledScript, CompileError> {
    let compiler = emit::Compiler::new(available_commands)
        .with_custom_commands(custom_commands);
    let script = compiler.compile(program)?;
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
    custom_commands: HashMap<String, CommandMeta>,
) -> Result<CompiledScript, String> {
    let tokens = grimscript_lang::lexer::Lexer::new(source).tokenize()
        .map_err(|e| format!("syntax error (line {}): {}", e.line, e.message))?;
    let program = grimscript_lang::parser::Parser::new(tokens)
        .parse()
        .map_err(|e| format!("parse error (line {}): {}", e.line, e.message))?;
    compile_with_custom(&program, available_commands, custom_commands)
        .map_err(|e| format!("compile error (line {}): {}", e.line, e.message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{CommandKind, UnitAction};
    use crate::entity::ScriptState;
    use crate::executor;
    use crate::ir::Instruction;
    use crate::world::SimWorld;

    /// Build the standard set of game builtin command metadata for tests.
    fn test_command_metadata() -> HashMap<String, CommandMeta> {
        let mut m = HashMap::new();
        // Queries
        m.insert("scan".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: false });
        m.insert("nearest".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: false });
        m.insert("distance".into(), CommandMeta { num_args: 2, kind: CommandKind::Query, implicit_self: false });
        m.insert("get_pos".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: true });
        m.insert("get_health".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: true });
        m.insert("get_shield".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: true });
        m.insert("get_target".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: true });
        m.insert("has_target".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: true });
        m.insert("get_type".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: false });
        m.insert("get_name".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: false });
        m.insert("get_owner".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: false });
        m.insert("get_resource".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: false });
        m.insert("get_stat".into(), CommandMeta { num_args: 2, kind: CommandKind::Query, implicit_self: false });
        m.insert("get_custom_stat".into(), CommandMeta { num_args: 2, kind: CommandKind::Query, implicit_self: false });
        m.insert("get_types".into(), CommandMeta { num_args: 1, kind: CommandKind::Query, implicit_self: false });
        m.insert("has_type".into(), CommandMeta { num_args: 2, kind: CommandKind::Query, implicit_self: false });
        // Actions
        m.insert("move".into(), CommandMeta { num_args: 1, kind: CommandKind::Action, implicit_self: false });
        m.insert("attack".into(), CommandMeta { num_args: 1, kind: CommandKind::Action, implicit_self: false });
        m.insert("flee".into(), CommandMeta { num_args: 1, kind: CommandKind::Action, implicit_self: false });
        m.insert("wait".into(), CommandMeta { num_args: 0, kind: CommandKind::Action, implicit_self: false });
        m.insert("set_target".into(), CommandMeta { num_args: 1, kind: CommandKind::Action, implicit_self: false });
        // Instant effects
        m.insert("gain_resource".into(), CommandMeta { num_args: 2, kind: CommandKind::Instant, implicit_self: false });
        m.insert("try_spend_resource".into(), CommandMeta { num_args: 2, kind: CommandKind::Instant, implicit_self: false });
        m
    }

    /// Helper: compile source with game builtins, execute on a fresh world, return final state + action.
    fn compile_and_run(source: &str) -> (ScriptState, Option<UnitAction>) {
        let script = compile_source_full(source, None, test_command_metadata()).expect("compilation failed");
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
        let script = compile_source_full("targets = scan(\"fighter\")", None, test_command_metadata()).unwrap();
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
        let script = compile_source_full("x = self", None, test_command_metadata()).unwrap();
        assert!(script.instructions.iter().any(|i| matches!(i, Instruction::LoadVar(0))));
    }

    #[test]
    fn implicit_self_query() {
        let script = compile_source_full("h = get_health()", None, test_command_metadata()).unwrap();
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
        let vars = run_to_completion("x = 0 and 42");
        assert_eq!(vars.get(1), Some(&SimValue::Int(0)));
    }

    #[test]
    fn short_circuit_or() {
        let vars = run_to_completion("x = 0 or 42");
        assert_eq!(vars.get(1), Some(&SimValue::Int(42)));
    }

    #[test]
    fn get_resource_emits_query_instruction() {
        let script = compile_source_full("x = get_resource(\"souls\")", None, test_command_metadata()).unwrap();
        assert!(script.instructions.iter().any(|i| matches!(i, Instruction::QueryGetResource)));
    }

    #[test]
    fn gain_resource_emits_instant_instruction() {
        let script = compile_source_full("x = gain_resource(\"souls\", 5)", None, test_command_metadata()).unwrap();
        assert!(script.instructions.iter().any(|i| matches!(i, Instruction::InstantGainResource)));
    }

    #[test]
    fn try_spend_resource_emits_instant_instruction() {
        let script = compile_source_full("x = try_spend_resource(\"souls\", 3)", None, test_command_metadata()).unwrap();
        assert!(script.instructions.iter().any(|i| matches!(i, Instruction::InstantTrySpendResource)));
    }

    #[test]
    fn resource_builtins_gated_by_available_commands() {
        let available: HashSet<String> = ["move"].iter().map(|s| s.to_string()).collect();
        let result = compile_source_full("get_resource(\"souls\")", Some(available), test_command_metadata());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not available"));
    }

    // --- Bug fix tests ---

    #[test]
    fn for_loop_continue() {
        let vars = run_to_completion(
            "total = 0\nfor i in range(5):\n    if i == 2:\n        continue\n    total = total + i",
        );
        assert_eq!(vars.get(1), Some(&SimValue::Int(8)));
    }

    #[test]
    fn for_loop_nested_continue() {
        let vars = run_to_completion(
            "total = 0\nfor i in range(3):\n    for j in range(3):\n        if j == 1:\n            continue\n        total = total + 1",
        );
        assert_eq!(vars.get(1), Some(&SimValue::Int(6)));
    }

    #[test]
    fn aug_assign_complex_index() {
        let vars = run_to_completion("x = [10, 20, 30]\ny = 1\nx[y] += 5");
        assert_eq!(
            vars.get(1),
            Some(&SimValue::List(vec![
                SimValue::Int(10),
                SimValue::Int(25),
                SimValue::Int(30),
            ]))
        );
    }

    #[test]
    fn aug_assign_dict() {
        let vars = run_to_completion("d = {\"a\": 10}\nd[\"a\"] += 5");
        if let Some(SimValue::Dict(map)) = vars.get(1) {
            assert_eq!(map.get("a"), Some(&SimValue::Int(15)));
        } else {
            panic!("expected dict");
        }
    }

    #[test]
    fn floor_division_negative() {
        let vars = run_to_completion("x = -7 // 2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(-4)));
    }

    #[test]
    fn floor_division_both_negative() {
        let vars = run_to_completion("x = -7 // -2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(3)));
    }

    #[test]
    fn floor_mod_negative() {
        let vars = run_to_completion("x = -7 % 2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(1)));
    }

    #[test]
    fn floor_mod_both_negative() {
        let vars = run_to_completion("x = -7 % -2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(-1)));
    }

    #[test]
    fn floor_division_positive() {
        let vars = run_to_completion("x = 7 // 2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(3)));
    }

    #[test]
    fn floor_mod_positive() {
        let vars = run_to_completion("x = 7 % 2");
        assert_eq!(vars.get(1), Some(&SimValue::Int(1)));
    }

    #[test]
    fn integer_overflow_error() {
        let result = compile_source("x = 99999999999999999999");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("invalid integer literal"), "got: {err}");
    }
}
