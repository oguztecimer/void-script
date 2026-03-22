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

/// Check if a source string defines a function with the given name.
/// Does a quick lex+parse and checks for a `FunctionDef` at the top level.
pub fn source_defines_function(source: &str, name: &str) -> bool {
    let tokens = match grimscript_lang::lexer::Lexer::new(source).tokenize() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let program = match grimscript_lang::parser::Parser::new(tokens).parse() {
        Ok(p) => p,
        Err(_) => return false,
    };
    program.statements.iter().any(|stmt| {
        matches!(&stmt.kind, grimscript_lang::ast::StmtKind::FunctionDef { name: n, .. } if n == name)
    })
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
    compile_source_full(source, available_commands, HashMap::new(), false)
}

/// Parse source code and compile to IR, with command gating, custom commands,
/// and optional soul loop. When `enable_soul_loop` is true and the source
/// defines a `soul()` function, the compiler emits an auto-call to `soul()`
/// and records the PC as `soul_entry_pc` for tick-loop restart.
pub fn compile_source_full(
    source: &str,
    available_commands: Option<HashSet<String>>,
    custom_commands: HashMap<String, CommandMeta>,
    enable_soul_loop: bool,
) -> Result<CompiledScript, String> {
    let tokens = grimscript_lang::lexer::Lexer::new(source).tokenize()
        .map_err(|e| format!("syntax error (line {}): {}", e.line, e.message))?;
    let program = grimscript_lang::parser::Parser::new(tokens)
        .parse()
        .map_err(|e| format!("parse error (line {}): {}", e.line, e.message))?;
    let compiler = emit::Compiler::new(available_commands)
        .with_custom_commands(custom_commands)
        .with_soul_loop(enable_soul_loop);
    compiler.compile(&program)
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

    /// Build the standard set of command metadata for tests.
    /// All game builtins have been removed — returns an empty map.
    fn test_command_metadata() -> HashMap<String, CommandMeta> {
        HashMap::new()
    }

    /// Helper: compile source with game builtins, execute on a fresh world, return final state + action.
    fn compile_and_run(source: &str) -> (ScriptState, Option<UnitAction>) {
        let script = compile_source_full(source, None, test_command_metadata(), false).expect("compilation failed");
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
        let script = compile_source_full("x = self", None, test_command_metadata(), false).unwrap();
        assert!(script.instructions.iter().any(|i| matches!(i, Instruction::LoadVar(0))));
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

    // --- Enum and Match tests ---

    #[test]
    fn enum_basic() {
        let vars = run_to_completion("enum Color:\n    RED\n    GREEN\n    BLUE\nx = Color.RED\ny = Color.GREEN\nz = Color.BLUE");
        assert_eq!(vars.get(1), Some(&SimValue::Int(0)));
        assert_eq!(vars.get(2), Some(&SimValue::Int(1)));
        assert_eq!(vars.get(3), Some(&SimValue::Int(2)));
    }

    #[test]
    fn enum_explicit_values() {
        let vars = run_to_completion("enum State:\n    IDLE\n    DEAD = 10\n    BURIED\nx = State.IDLE\ny = State.DEAD\nz = State.BURIED");
        assert_eq!(vars.get(1), Some(&SimValue::Int(0)));
        assert_eq!(vars.get(2), Some(&SimValue::Int(10)));
        assert_eq!(vars.get(3), Some(&SimValue::Int(11)));
    }

    #[test]
    fn match_literal() {
        let (_state, action) = compile_and_run("x = 2\nmatch x:\n    case 1:\n        print(\"one\")\n    case 2:\n        print(\"two\")\n    case 3:\n        print(\"three\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "two"));
    }

    #[test]
    fn match_enum_member() {
        let (_state, action) = compile_and_run("enum State:\n    IDLE\n    MOVING\ns = State.MOVING\nmatch s:\n    case State.IDLE:\n        print(\"idle\")\n    case State.MOVING:\n        print(\"moving\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "moving"));
    }

    #[test]
    fn match_wildcard() {
        let (_state, action) = compile_and_run("x = 99\nmatch x:\n    case 1:\n        print(\"one\")\n    case _:\n        print(\"default\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "default"));
    }

    #[test]
    fn match_or_pattern() {
        let (_state, action) = compile_and_run("x = 2\nmatch x:\n    case 1 | 2:\n        print(\"low\")\n    case 3:\n        print(\"three\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "low"));
    }

    #[test]
    fn match_no_match() {
        let (_state, action) = compile_and_run("x = 99\nmatch x:\n    case 1:\n        print(\"one\")\n    case 2:\n        print(\"two\")");
        // No case matches, so no print action (halts with None).
        assert!(action.is_none());
    }

    #[test]
    fn match_first_wins() {
        let (_state, action) = compile_and_run("x = 1\nmatch x:\n    case 1:\n        print(\"first\")\n    case 1:\n        print(\"second\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "first"));
    }

    #[test]
    fn match_or_with_enum() {
        let (_state, action) = compile_and_run("enum State:\n    IDLE\n    MOVING\n    ATTACKING\ns = State.ATTACKING\nmatch s:\n    case State.MOVING | State.ATTACKING:\n        print(\"active\")\n    case _:\n        print(\"other\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "active"));
    }

    #[test]
    fn match_negative_literal() {
        let (_state, action) = compile_and_run("x = -1\nmatch x:\n    case -1:\n        print(\"neg one\")\n    case 0:\n        print(\"zero\")");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "neg one"));
    }

    #[test]
    fn enum_in_function() {
        let (_state, action) = compile_and_run("enum Dir:\n    LEFT\n    RIGHT\ndef describe(d):\n    match d:\n        case Dir.LEFT:\n            return \"left\"\n        case _:\n            return \"other\"\nprint(describe(Dir.LEFT))");
        assert!(matches!(action, Some(UnitAction::Print { text }) if text == "left"));
    }
}
