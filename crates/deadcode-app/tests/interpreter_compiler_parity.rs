//! Integration tests verifying parity between the tree-walking interpreter
//! (`grimscript_lang`) and the compiler/executor (`deadcode_sim::compiler` + `executor`).
//!
//! Both paths should produce identical outputs for stdlib functions and basic
//! language constructs.
//!
//! # Known intentional divergences
//!
//! - `float()`: interpreter returns Float, compiler errors (sim has no floats)

use std::collections::HashMap;

use crossbeam_channel::unbounded;

use deadcode_sim::compiler;
use deadcode_sim::entity::ScriptState;
use deadcode_sim::executor;
use deadcode_sim::value::SimValue;
use deadcode_sim::world::SimWorld;

use grimscript_lang::debug::{DebugCommand, ScriptEvent, OutputLevel};

/// Run source through the interpreter, collecting output lines.
fn interpreter_outputs(source: &str) -> Vec<String> {
    let (output_tx, output_rx) = unbounded();
    let (_cmd_tx, cmd_rx) = unbounded::<DebugCommand>();

    grimscript_lang::run_script(source, output_tx, cmd_rx, None, None);

    let mut outputs = Vec::new();
    while let Ok(event) = output_rx.try_recv() {
        if let ScriptEvent::Output { line, level } = event {
            if matches!(level, OutputLevel::Info) {
                outputs.push(line);
            }
        }
    }
    outputs
}

/// Run source through compiler + executor, collecting echo outputs from sim events.
fn compiler_outputs(source: &str) -> Vec<String> {
    let script = compiler::compile_source_full(source, None, HashMap::new(), false).expect("compilation failed");
    let mut world = SimWorld::new(42);
    let eid = world.spawn_entity("skeleton".into(), "test".into(), 100);
    let num_vars = script.num_variables;
    let mut state = ScriptState::new(script, num_vars);
    if !state.variables.is_empty() {
        state.variables[0] = SimValue::EntityRef(eid);
    }

    let mut outputs = Vec::new();

    // Run until halt, collecting echo outputs.
    loop {
        match executor::execute_unit(eid, &mut state, &world) {
            Ok(Some(deadcode_sim::action::UnitAction::Print { text })) => {
                outputs.push(text);
            }
            Ok(Some(_)) => {
                // Action consumed tick — in tests we just continue.
            }
            Ok(None) => break,
            Err(e) => {
                panic!("executor error: {e}");
            }
        }
    }
    outputs
}

/// Assert interpreter and compiler produce the same echo outputs.
fn assert_parity(source: &str) {
    let interp = interpreter_outputs(source);
    let comp = compiler_outputs(source);
    assert_eq!(
        interp, comp,
        "Output mismatch for:\n{source}\n  interpreter: {interp:?}\n  compiler:    {comp:?}"
    );
}

// ---------------------------------------------------------------------------
// Stdlib parity tests
// ---------------------------------------------------------------------------

#[test]
fn parity_print_int() {
    assert_parity("echo(42)");
}

#[test]
fn parity_print_string() {
    assert_parity("echo(\"hello world\")");
}

#[test]
fn parity_print_bool() {
    assert_parity("echo(True)");
    assert_parity("echo(False)");
}

#[test]
fn parity_print_none() {
    assert_parity("echo(None)");
}

#[test]
fn parity_len() {
    assert_parity("echo(len([1, 2, 3]))");
    assert_parity("echo(len(\"hello\"))");
    assert_parity("echo(len({\"a\": 1}))");
}

#[test]
fn parity_abs() {
    assert_parity("echo(abs(-42))");
    assert_parity("echo(abs(42))");
    assert_parity("echo(abs(0))");
}

#[test]
fn parity_min_max() {
    assert_parity("echo(min(3, 7))");
    assert_parity("echo(max(3, 7))");
    assert_parity("echo(min(10, 2))");
    assert_parity("echo(max(10, 2))");
}

#[test]
fn parity_int_cast() {
    assert_parity("echo(int(\"99\"))");
    assert_parity("echo(int(True))");
    assert_parity("echo(int(False))");
    assert_parity("echo(int(42))");
}

#[test]
fn parity_str_cast() {
    assert_parity("echo(str(42))");
    assert_parity("echo(str(True))");
    assert_parity("echo(str(None))");
}

#[test]
fn parity_type_builtin() {
    assert_parity("echo(type(42))");
    assert_parity("echo(type(\"hello\"))");
    assert_parity("echo(type(True))");
    assert_parity("echo(type(None))");
    assert_parity("echo(type([1, 2]))");
}

#[test]
fn parity_range() {
    assert_parity("echo(range(5))");
    assert_parity("echo(range(2, 5))");
    assert_parity("echo(range(0, 10, 3))");
}

// ---------------------------------------------------------------------------
// Variable assignment and arithmetic
// ---------------------------------------------------------------------------

#[test]
fn parity_arithmetic() {
    assert_parity("echo(10 + 5)");
    assert_parity("echo(10 - 3)");
    assert_parity("echo(4 * 7)");
    assert_parity("echo(10 // 3)");
    assert_parity("echo(10 % 3)");
}

#[test]
fn parity_variable_assignment() {
    assert_parity("x = 42\necho(x)");
    assert_parity("x = 10\ny = x + 5\necho(y)");
}

#[test]
fn parity_string_concat() {
    assert_parity("echo(\"hello\" + \" \" + \"world\")");
}

#[test]
fn parity_unary_negate() {
    assert_parity("echo(-42)");
    assert_parity("x = 10\necho(-x)");
}

// ---------------------------------------------------------------------------
// Conditionals
// ---------------------------------------------------------------------------

#[test]
fn parity_if_else() {
    assert_parity("x = 5\nif x > 3:\n    echo(\"big\")\nelse:\n    echo(\"small\")");
    assert_parity("x = 1\nif x > 3:\n    echo(\"big\")\nelse:\n    echo(\"small\")");
}

#[test]
fn parity_elif() {
    let src = "x = 5\nif x > 10:\n    echo(\"huge\")\nelif x > 3:\n    echo(\"medium\")\nelse:\n    echo(\"tiny\")";
    assert_parity(src);
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

#[test]
fn parity_while_loop() {
    assert_parity("x = 0\nwhile x < 5:\n    echo(x)\n    x = x + 1");
}

#[test]
fn parity_for_range() {
    assert_parity("for i in range(5):\n    echo(i)");
}

#[test]
fn parity_for_list() {
    assert_parity("for x in [10, 20, 30]:\n    echo(x)");
}

#[test]
fn parity_while_break() {
    assert_parity("x = 0\nwhile True:\n    x = x + 1\n    if x == 3:\n        break\necho(x)");
}

// ---------------------------------------------------------------------------
// List/dict operations
// ---------------------------------------------------------------------------

#[test]
fn parity_list_index() {
    assert_parity("x = [10, 20, 30]\necho(x[1])");
}

#[test]
fn parity_list_append() {
    assert_parity("x = [1, 2]\nx.append(3)\necho(x)");
}

#[test]
fn parity_dict_access() {
    assert_parity("d = {\"a\": 1, \"b\": 2}\necho(d[\"a\"])");
}

#[test]
fn parity_dict_get() {
    assert_parity("d = {\"a\": 1}\necho(d.get(\"a\", 0))");
    assert_parity("d = {\"a\": 1}\necho(d.get(\"b\", 99))");
}

#[test]
fn parity_dict_keys() {
    let interp = interpreter_outputs("d = {\"a\": 1, \"b\": 2}\necho(d.keys())");
    let comp = compiler_outputs("d = {\"a\": 1, \"b\": 2}\necho(d.keys())");
    assert_eq!(interp.len(), 1);
    assert_eq!(comp.len(), 1);
    assert!(interp[0].contains("a") && interp[0].contains("b"));
    assert!(comp[0].contains("a") && comp[0].contains("b"));
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[test]
fn parity_function_call() {
    assert_parity("def add(a, b):\n    return a + b\necho(add(3, 7))");
}

#[test]
fn parity_recursive_function() {
    let src = "def fact(n):\n    if n <= 1:\n        return 1\n    return n * fact(n - 1)\necho(fact(5))";
    assert_parity(src);
}

// ---------------------------------------------------------------------------
// Fixed-point arithmetic helpers
// ---------------------------------------------------------------------------

#[test]
fn parity_percent() {
    assert_parity("echo(percent(200, 50))");  // 100
    assert_parity("echo(percent(100, 150))"); // 150
    assert_parity("echo(percent(10, 33))");   // 3 (10*33/100 = 3.3 → 3)
}

#[test]
fn parity_scale() {
    assert_parity("echo(scale(100, 1, 3))");  // 33 (100/3 = 33.33 → 33)
    assert_parity("echo(scale(100, 2, 3))");  // 67 (200/3 = 66.67 → 67)
    assert_parity("echo(scale(10, 3, 4))");   // 8 (30/4 = 7.5 → 8, banker's round to even)
}

// ---------------------------------------------------------------------------
// Division/modulo parity (floor semantics)
// ---------------------------------------------------------------------------

#[test]
fn parity_floor_div_negative() {
    assert_parity("echo(-7 // 2)");
}

#[test]
fn parity_floor_div_both_negative() {
    assert_parity("echo(-7 // -2)");
}

#[test]
fn parity_floor_mod_negative() {
    assert_parity("echo(-7 % 2)");
}

#[test]
fn parity_floor_mod_both_negative() {
    assert_parity("echo(-7 % -2)");
}

// ---------------------------------------------------------------------------
// For-loop continue parity
// ---------------------------------------------------------------------------

#[test]
fn parity_for_continue() {
    assert_parity(
        "total = 0\nfor i in range(5):\n    if i == 2:\n        continue\n    total = total + i\necho(total)",
    );
}

// ---------------------------------------------------------------------------
// Known divergences (documented, not asserted for parity)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Enum and Match parity tests
// ---------------------------------------------------------------------------

#[test]
fn parity_enum_basic() {
    assert_parity("enum Color:\n    RED\n    GREEN\n    BLUE\necho(Color.RED)\necho(Color.GREEN)\necho(Color.BLUE)");
}

#[test]
fn parity_enum_explicit_values() {
    assert_parity("enum State:\n    IDLE\n    DEAD = 10\n    BURIED\necho(State.IDLE)\necho(State.DEAD)\necho(State.BURIED)");
}

#[test]
fn parity_match_literal() {
    assert_parity("x = 2\nmatch x:\n    case 1:\n        echo(\"one\")\n    case 2:\n        echo(\"two\")\n    case 3:\n        echo(\"three\")");
}

#[test]
fn parity_match_enum() {
    assert_parity("enum State:\n    IDLE\n    MOVING\ns = State.MOVING\nmatch s:\n    case State.IDLE:\n        echo(\"idle\")\n    case State.MOVING:\n        echo(\"moving\")");
}

#[test]
fn parity_match_or() {
    assert_parity("x = 2\nmatch x:\n    case 1 | 2:\n        echo(\"low\")\n    case 3:\n        echo(\"three\")");
}

#[test]
fn parity_match_wildcard() {
    assert_parity("x = 99\nmatch x:\n    case 1:\n        echo(\"one\")\n    case _:\n        echo(\"default\")");
}

#[test]
fn parity_match_no_match() {
    assert_parity("x = 99\nmatch x:\n    case 1:\n        echo(\"one\")\n    case 2:\n        echo(\"two\")");
}

#[test]
fn parity_match_in_function() {
    assert_parity("enum Dir:\n    LEFT\n    RIGHT\ndef describe(d):\n    match d:\n        case Dir.LEFT:\n            return \"left\"\n        case Dir.RIGHT:\n            return \"right\"\n        case _:\n            return \"unknown\"\necho(describe(Dir.LEFT))\necho(describe(Dir.RIGHT))\necho(describe(99))");
}

// ---------------------------------------------------------------------------
// Known divergences (documented, not asserted for parity)
// ---------------------------------------------------------------------------

#[test]
fn divergence_float_not_supported_in_compiler() {
    let interp = interpreter_outputs("echo(float(42))");
    assert!(!interp.is_empty(), "interpreter should produce output for float()");

    let result = compiler::compile_source("echo(float(42))");
    assert!(result.is_err(), "compiler should reject float()");
}
