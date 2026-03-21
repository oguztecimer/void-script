use crossbeam_channel::unbounded;
use grimscript_lang::{ScriptEvent, run_script};

/// Run a GrimScript source string and collect all events.
fn run(source: &str) -> Vec<ScriptEvent> {
    let (event_tx, event_rx) = unbounded();
    let (_cmd_tx, cmd_rx) = unbounded();
    run_script(source, event_tx, cmd_rx, None, None);
    event_rx.try_iter().collect()
}

/// Run with a specific set of available commands and custom command names.
fn run_with_commands(source: &str, available: std::collections::HashSet<String>, custom: std::collections::HashSet<String>) -> Vec<ScriptEvent> {
    let (event_tx, event_rx) = unbounded();
    let (_cmd_tx, cmd_rx) = unbounded();
    run_script(source, event_tx, cmd_rx, Some(available), Some(custom));
    event_rx.try_iter().collect()
}

/// Extract output lines from events.
fn outputs(events: &[ScriptEvent]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|e| match e {
            ScriptEvent::Output { line, .. } => Some(line.as_str()),
            _ => None,
        })
        .collect()
}

/// Check whether the script finished successfully.
fn succeeded(events: &[ScriptEvent]) -> bool {
    events.iter().any(|e| matches!(e, ScriptEvent::Finished { success: true, .. }))
}

/// Check whether the script finished with an error.
fn failed(events: &[ScriptEvent]) -> bool {
    events.iter().any(|e| matches!(e, ScriptEvent::Finished { success: false, .. }))
}

// ── Builtin calls ───────────────────────────────────────────────────

#[test]
fn print_string() {
    let events = run(r#"print("hello")"#);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["hello"]);
}

#[test]
fn print_expression() {
    let events = run("print(1 + 2)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["3"]);
}

#[test]
fn print_multiple_args() {
    let events = run(r#"print("a", "b", "c")"#);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["a b c"]);
}

#[test]
fn len_builtin() {
    let events = run(r#"print(len("abc"))"#);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["3"]);
}

#[test]
fn range_builtin() {
    let events = run("print(range(3))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["[0, 1, 2]"]);
}

#[test]
fn abs_builtin() {
    let events = run("print(abs(-5))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["5"]);
}

#[test]
fn min_max_builtin() {
    let events = run("print(min(3, 1, 2))\nprint(max(3, 1, 2))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["1", "3"]);
}

#[test]
fn type_builtin() {
    let events = run(r#"print(type(42))"#);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["int"]);
}

#[test]
fn int_str_float_conversion() {
    let events = run(r#"print(int("7"))"#);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["7"]);
}

// ── Builtins used with variables ────────────────────────────────────

#[test]
fn builtin_with_variable_arg() {
    let events = run("x = 42\nprint(x)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["42"]);
}

#[test]
fn builtin_in_loop() {
    let events = run("for i in range(3):\n  print(i)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["0", "1", "2"]);
}

// ── User-defined functions ──────────────────────────────────────────

#[test]
fn user_function_call() {
    let events = run("def greet():\n  print(\"hi\")\ngreet()");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["hi"]);
}

#[test]
fn user_function_with_args() {
    let events = run("def add(a, b):\n  print(a + b)\nadd(3, 4)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["7"]);
}

#[test]
fn user_function_return_value() {
    let events = run("def double(x):\n  return x * 2\nprint(double(5))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["10"]);
}

// ── Nested builtin calls ────────────────────────────────────────────

#[test]
fn nested_builtin_calls() {
    let events = run("print(len(range(5)))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["5"]);
}

#[test]
fn builtin_as_function_arg() {
    let events = run("print(abs(min(-3, -7)))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["7"]);
}

// ── Control flow ────────────────────────────────────────────────────

#[test]
fn if_else() {
    let events = run("x = 10\nif x > 5:\n  print(\"big\")\nelse:\n  print(\"small\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["big"]);
}

#[test]
fn while_loop() {
    let events = run("x = 0\nwhile x < 3:\n  print(x)\n  x = x + 1");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["0", "1", "2"]);
}

// ── Data structures ─────────────────────────────────────────────────

#[test]
fn list_operations() {
    let events = run("xs = [1, 2, 3]\nprint(len(xs))\nprint(xs[1])");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["3", "2"]);
}

#[test]
fn dict_operations() {
    let events = run("d = {\"a\": 1, \"b\": 2}\nprint(len(d))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["2"]);
}

// ── Error cases ─────────────────────────────────────────────────────

#[test]
fn undefined_variable_error() {
    let events = run("print(xyz)");
    assert!(failed(&events));
}

#[test]
fn undefined_function_error() {
    let events = run("foo()");
    assert!(failed(&events));
}

#[test]
fn syntax_error() {
    let events = run("if True");
    assert!(failed(&events));
}

// ── One-liner expressions (terminal use case) ───────────────────────

#[test]
fn one_liner_print() {
    let events = run(r#"print("terminal test")"#);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["terminal test"]);
}

#[test]
fn one_liner_arithmetic() {
    let events = run("print(2 * 10)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["20"]);
}

#[test]
fn one_liner_string_concat() {
    let events = run(r#"print("hello" + " " + "world")"#);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["hello world"]);
}

#[test]
fn one_liner_variable_and_print() {
    let events = run("x = 5 * 10\nprint(x)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["50"]);
}

// ── Available commands gating ───────────────────────────────────────

#[test]
fn unavailable_game_builtin_produces_error() {
    // scan is a known custom command but not in the available set.
    let empty: std::collections::HashSet<String> = std::collections::HashSet::new();
    let custom: std::collections::HashSet<String> = ["scan"].iter().map(|s| s.to_string()).collect();
    let events = run_with_commands(r#"scan("fighter")"#, empty, custom);
    assert!(failed(&events));
    // Check the error message mentions "not available"
    let has_not_available = events.iter().any(|e| match e {
        ScriptEvent::Output { line, .. } => line.contains("not available yet"),
        _ => false,
    });
    assert!(has_not_available);
}

#[test]
fn stdlib_works_with_empty_available_set() {
    let empty: std::collections::HashSet<String> = std::collections::HashSet::new();
    let events = run_with_commands(r#"print("hello")"#, empty, std::collections::HashSet::new());
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["hello"]);
}

#[test]
fn available_game_builtin_works_when_in_set() {
    let mut cmds = std::collections::HashSet::new();
    cmds.insert("wait".to_string());
    let custom = cmds.clone();
    let events = run_with_commands("wait()", cmds, custom);
    assert!(succeeded(&events));
    // Game builtins are now treated as custom commands in the interpreter.
    assert_eq!(outputs(&events), vec!["[wait] (custom command)"]);
}

// ── Bug fix tests ─────────────────────────────────────────────────────

#[test]
fn dict_iteration() {
    let events = run("d = {\"a\": 1, \"b\": 2}\nresult = []\nfor k in d:\n    result.append(k)\nprint(len(result))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["2"]);
}

#[test]
fn min_max_type_error() {
    let events = run("print(min(5, \"hello\"))");
    assert!(failed(&events));
}

#[test]
fn floor_div_negative() {
    // Python: -7 // 2 = -4
    let events = run("print(-7 // 2)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["-4"]);
}

#[test]
fn floor_mod_negative() {
    // Python: -7 % 2 = 1
    let events = run("print(-7 % 2)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["1"]);
}

#[test]
fn integer_overflow_lexer_error() {
    let events = run("x = 99999999999999999999");
    assert!(failed(&events));
}

#[test]
fn percent_overflow_error() {
    // i64::MAX * 2 would overflow
    let events = run("print(percent(9223372036854775807, 2))");
    assert!(failed(&events));
}

// ── Enum and Match ─────────────────────────────────────────────────────

#[test]
fn enum_basic() {
    let events = run("enum Color:\n    RED\n    GREEN\n    BLUE\nprint(Color.RED)\nprint(Color.GREEN)\nprint(Color.BLUE)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["0", "1", "2"]);
}

#[test]
fn enum_auto_increment() {
    let events = run("enum State:\n    IDLE\n    MOVING\n    ATTACKING\nprint(State.IDLE)\nprint(State.MOVING)\nprint(State.ATTACKING)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["0", "1", "2"]);
}

#[test]
fn enum_explicit_values() {
    let events = run("enum State:\n    IDLE\n    DEAD = 10\n    BURIED\nprint(State.IDLE)\nprint(State.DEAD)\nprint(State.BURIED)");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["0", "10", "11"]);
}

#[test]
fn match_literal() {
    let events = run("x = 2\nmatch x:\n    case 1:\n        print(\"one\")\n    case 2:\n        print(\"two\")\n    case 3:\n        print(\"three\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["two"]);
}

#[test]
fn match_string_literal() {
    let events = run("x = \"hello\"\nmatch x:\n    case \"hello\":\n        print(\"greeting\")\n    case \"bye\":\n        print(\"farewell\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["greeting"]);
}

#[test]
fn match_enum() {
    let events = run("enum State:\n    IDLE\n    MOVING\ns = State.MOVING\nmatch s:\n    case State.IDLE:\n        print(\"idle\")\n    case State.MOVING:\n        print(\"moving\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["moving"]);
}

#[test]
fn match_wildcard() {
    let events = run("x = 99\nmatch x:\n    case 1:\n        print(\"one\")\n    case _:\n        print(\"default\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["default"]);
}

#[test]
fn match_or_pattern() {
    let events = run("x = 2\nmatch x:\n    case 1 | 2:\n        print(\"low\")\n    case 3:\n        print(\"three\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["low"]);
}

#[test]
fn match_no_match() {
    let events = run("x = 99\nmatch x:\n    case 1:\n        print(\"one\")\n    case 2:\n        print(\"two\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), Vec::<&str>::new());
}

#[test]
fn match_first_wins() {
    let events = run("x = 1\nmatch x:\n    case 1:\n        print(\"first\")\n    case 1:\n        print(\"second\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["first"]);
}

#[test]
fn enum_undefined_error() {
    let events = run("x = Bogus.THING");
    assert!(failed(&events));
}

#[test]
fn match_in_function() {
    let events = run("enum Dir:\n    LEFT\n    RIGHT\ndef describe(d):\n    match d:\n        case Dir.LEFT:\n            return \"left\"\n        case Dir.RIGHT:\n            return \"right\"\n        case _:\n            return \"unknown\"\nprint(describe(Dir.LEFT))\nprint(describe(Dir.RIGHT))\nprint(describe(99))");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["left", "right", "unknown"]);
}

#[test]
fn match_negative_literal() {
    let events = run("x = -1\nmatch x:\n    case -1:\n        print(\"neg one\")\n    case 0:\n        print(\"zero\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["neg one"]);
}

#[test]
fn match_bool_none() {
    let events = run("x = None\nmatch x:\n    case True:\n        print(\"true\")\n    case None:\n        print(\"none\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["none"]);
}

#[test]
fn match_or_with_enum() {
    let events = run("enum State:\n    IDLE\n    MOVING\n    ATTACKING\ns = State.ATTACKING\nmatch s:\n    case State.MOVING | State.ATTACKING:\n        print(\"active\")\n    case _:\n        print(\"other\")");
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["active"]);
}
