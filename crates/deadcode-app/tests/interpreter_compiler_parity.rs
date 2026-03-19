//! Integration tests verifying parity between the tree-walking interpreter
//! (`grimscript_lang`) and the compiler/executor (`deadcode_sim::compiler` + `executor`).
//!
//! Both paths should produce identical outputs for stdlib functions and basic
//! language constructs.
//!
//! # Known intentional divergences
//!
//! - `float()`: interpreter returns Float, compiler errors (sim has no floats)
//! - Game builtins (move, scan, etc.): interpreter returns stubs, compiler uses real sim world
//! - Custom command gating: interpreter only checks `is_game_builtin()`, compiler checks all types

use crossbeam_channel::unbounded;

use deadcode_sim::compiler;
use deadcode_sim::entity::ScriptState;
use deadcode_sim::executor;
use deadcode_sim::value::SimValue;
use deadcode_sim::world::SimWorld;

use grimscript_lang::debug::{DebugCommand, ScriptEvent, OutputLevel};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Run source through compiler + executor, collecting print outputs from sim events.
fn compiler_outputs(source: &str) -> Vec<String> {
    let script = compiler::compile_source(source).expect("compilation failed");
    let mut world = SimWorld::new(42);
    let eid = world.spawn_entity("skeleton".into(), "test".into(), 100);
    let num_vars = script.num_variables;
    let mut state = ScriptState::new(script, num_vars);
    if !state.variables.is_empty() {
        state.variables[0] = SimValue::EntityRef(eid);
    }

    let mut outputs = Vec::new();

    // Run until halt, collecting print outputs.
    loop {
        match executor::execute_unit(eid, &mut state, &world) {
            Ok(Some(deadcode_sim::action::UnitAction::Print { text })) => {
                outputs.push(text);
            }
            Ok(Some(_)) => {
                // Action consumed tick — in tests we just continue.
                // (For these parity tests we only check stdlib/print behavior.)
            }
            Ok(None) => break,
            Err(e) => {
                panic!("executor error: {e}");
            }
        }
    }
    outputs
}

/// Assert interpreter and compiler produce the same print outputs.
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
    assert_parity("print(42)");
}

#[test]
fn parity_print_string() {
    assert_parity("print(\"hello world\")");
}

#[test]
fn parity_print_bool() {
    assert_parity("print(True)");
    assert_parity("print(False)");
}

#[test]
fn parity_print_none() {
    assert_parity("print(None)");
}

#[test]
fn parity_len() {
    assert_parity("print(len([1, 2, 3]))");
    assert_parity("print(len(\"hello\"))");
    assert_parity("print(len({\"a\": 1}))");
}

#[test]
fn parity_abs() {
    assert_parity("print(abs(-42))");
    assert_parity("print(abs(42))");
    assert_parity("print(abs(0))");
}

#[test]
fn parity_min_max() {
    assert_parity("print(min(3, 7))");
    assert_parity("print(max(3, 7))");
    assert_parity("print(min(10, 2))");
    assert_parity("print(max(10, 2))");
}

#[test]
fn parity_int_cast() {
    assert_parity("print(int(\"99\"))");
    assert_parity("print(int(True))");
    assert_parity("print(int(False))");
    assert_parity("print(int(42))");
}

#[test]
fn parity_str_cast() {
    assert_parity("print(str(42))");
    assert_parity("print(str(True))");
    assert_parity("print(str(None))");
}

#[test]
fn parity_type_builtin() {
    assert_parity("print(type(42))");
    assert_parity("print(type(\"hello\"))");
    assert_parity("print(type(True))");
    assert_parity("print(type(None))");
    assert_parity("print(type([1, 2]))");
}

#[test]
fn parity_range() {
    assert_parity("print(range(5))");
    assert_parity("print(range(2, 5))");
    assert_parity("print(range(0, 10, 3))");
}

// ---------------------------------------------------------------------------
// Variable assignment and arithmetic
// ---------------------------------------------------------------------------

#[test]
fn parity_arithmetic() {
    assert_parity("print(10 + 5)");
    assert_parity("print(10 - 3)");
    assert_parity("print(4 * 7)");
    assert_parity("print(10 // 3)");
    assert_parity("print(10 % 3)");
}

#[test]
fn parity_variable_assignment() {
    assert_parity("x = 42\nprint(x)");
    assert_parity("x = 10\ny = x + 5\nprint(y)");
}

#[test]
fn parity_string_concat() {
    assert_parity("print(\"hello\" + \" \" + \"world\")");
}

#[test]
fn parity_unary_negate() {
    assert_parity("print(-42)");
    assert_parity("x = 10\nprint(-x)");
}

// ---------------------------------------------------------------------------
// Conditionals
// ---------------------------------------------------------------------------

#[test]
fn parity_if_else() {
    assert_parity("x = 5\nif x > 3:\n    print(\"big\")\nelse:\n    print(\"small\")");
    assert_parity("x = 1\nif x > 3:\n    print(\"big\")\nelse:\n    print(\"small\")");
}

#[test]
fn parity_elif() {
    let src = "x = 5\nif x > 10:\n    print(\"huge\")\nelif x > 3:\n    print(\"medium\")\nelse:\n    print(\"tiny\")";
    assert_parity(src);
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

#[test]
fn parity_while_loop() {
    assert_parity("x = 0\nwhile x < 5:\n    print(x)\n    x = x + 1");
}

#[test]
fn parity_for_range() {
    assert_parity("for i in range(5):\n    print(i)");
}

#[test]
fn parity_for_list() {
    assert_parity("for x in [10, 20, 30]:\n    print(x)");
}

#[test]
fn parity_while_break() {
    assert_parity("x = 0\nwhile True:\n    x = x + 1\n    if x == 3:\n        break\nprint(x)");
}

// ---------------------------------------------------------------------------
// List/dict operations
// ---------------------------------------------------------------------------

#[test]
fn parity_list_index() {
    assert_parity("x = [10, 20, 30]\nprint(x[1])");
}

#[test]
fn parity_list_append() {
    assert_parity("x = [1, 2]\nx.append(3)\nprint(x)");
}

#[test]
fn parity_dict_access() {
    assert_parity("d = {\"a\": 1, \"b\": 2}\nprint(d[\"a\"])");
}

#[test]
fn parity_dict_get() {
    assert_parity("d = {\"a\": 1}\nprint(d.get(\"a\", 0))");
    assert_parity("d = {\"a\": 1}\nprint(d.get(\"b\", 99))");
}

#[test]
fn parity_dict_keys() {
    // Note: dict keys display differs slightly — interpreter quotes strings in lists
    // (["a", "b"]) while compiler doesn't ([a, b]). This is a known display divergence
    // in how Value::String vs SimValue::Str format inside list Display.
    // We test each path produces output, but don't assert exact parity.
    let interp = interpreter_outputs("d = {\"a\": 1, \"b\": 2}\nprint(d.keys())");
    let comp = compiler_outputs("d = {\"a\": 1, \"b\": 2}\nprint(d.keys())");
    assert_eq!(interp.len(), 1);
    assert_eq!(comp.len(), 1);
    // Both contain "a" and "b" as keys.
    assert!(interp[0].contains("a") && interp[0].contains("b"));
    assert!(comp[0].contains("a") && comp[0].contains("b"));
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[test]
fn parity_function_call() {
    assert_parity("def add(a, b):\n    return a + b\nprint(add(3, 7))");
}

#[test]
fn parity_recursive_function() {
    let src = "def fact(n):\n    if n <= 1:\n        return 1\n    return n * fact(n - 1)\nprint(fact(5))";
    assert_parity(src);
}

// ---------------------------------------------------------------------------
// Fixed-point arithmetic helpers
// ---------------------------------------------------------------------------

#[test]
fn parity_percent() {
    assert_parity("print(percent(200, 50))");  // 100
    assert_parity("print(percent(100, 150))"); // 150
    assert_parity("print(percent(10, 33))");   // 3 (10*33/100 = 3.3 → 3)
}

#[test]
fn parity_scale() {
    assert_parity("print(scale(100, 1, 3))");  // 33 (100/3 = 33.33 → 33)
    assert_parity("print(scale(100, 2, 3))");  // 67 (200/3 = 66.67 → 67)
    assert_parity("print(scale(10, 3, 4))");   // 8 (30/4 = 7.5 → 8, banker's round to even)
}

// ---------------------------------------------------------------------------
// Known divergences (documented, not asserted for parity)
// ---------------------------------------------------------------------------

#[test]
fn divergence_float_not_supported_in_compiler() {
    // Interpreter supports float(); compiler does not.
    let interp = interpreter_outputs("print(float(42))");
    assert!(!interp.is_empty(), "interpreter should produce output for float()");

    let result = compiler::compile_source("print(float(42))");
    assert!(result.is_err(), "compiler should reject float()");
}
