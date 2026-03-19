use crossbeam_channel::unbounded;
use grimscript_lang::{ScriptEvent, run_script};

/// Run a GrimScript source string and collect all events.
fn run(source: &str) -> Vec<ScriptEvent> {
    let (event_tx, event_rx) = unbounded();
    let (_cmd_tx, cmd_rx) = unbounded();
    run_script(source, event_tx, cmd_rx, None, None);
    event_rx.try_iter().collect()
}

/// Run with a specific set of available commands.
fn run_with_commands(source: &str, available: std::collections::HashSet<String>) -> Vec<ScriptEvent> {
    let (event_tx, event_rx) = unbounded();
    let (_cmd_tx, cmd_rx) = unbounded();
    run_script(source, event_tx, cmd_rx, Some(available), None);
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
    let empty: std::collections::HashSet<String> = std::collections::HashSet::new();
    let events = run_with_commands(r#"scan("fighter")"#, empty);
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
    let events = run_with_commands(r#"print("hello")"#, empty);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["hello"]);
}

#[test]
fn available_game_builtin_works_when_in_set() {
    let mut cmds = std::collections::HashSet::new();
    cmds.insert("consult".to_string());
    let events = run_with_commands("consult()", cmds);
    assert!(succeeded(&events));
    assert_eq!(outputs(&events), vec!["[consult] Consulting the spirits..."]);
}
