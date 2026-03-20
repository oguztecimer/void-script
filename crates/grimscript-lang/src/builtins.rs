use crossbeam_channel::Sender;

use crate::debug::{OutputLevel, ScriptEvent};
use crate::error::GrimScriptError;
use crate::value::Value;

fn send_output(output_tx: &Sender<ScriptEvent>, msg: &str) {
    let _ = output_tx.send(ScriptEvent::Output {
        line: msg.to_string(),
        level: OutputLevel::Info,
    });
}

pub fn is_stdlib(name: &str) -> bool {
    matches!(
        name,
        "print" | "len" | "range" | "abs" | "min" | "max" | "int" | "float" | "str" | "type"
            | "percent" | "scale"
    )
}

pub fn is_game_builtin(name: &str) -> bool {
    is_builtin(name) && !is_stdlib(name)
}

pub fn is_builtin(name: &str) -> bool {
    is_builtin_static(name)
}

/// Check against the statically known builtins.
pub fn is_builtin_static(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "len"
            | "range"
            | "abs"
            | "min"
            | "max"
            | "int"
            | "float"
            | "str"
            | "type"
            | "percent"
            | "scale"
            | "append"
            | "move"
            | "get_pos"
            | "scan"
            | "nearest"
            | "distance"
            | "attack"
            | "flee"
            | "get_health"
            | "get_shield"
            | "wait"
            | "set_target"
            | "get_target"
            | "has_target"
            | "get_type"
            | "get_name"
            | "get_owner"
            | "get_resource"
            | "gain_resource"
            | "try_spend_resource"
            | "get_custom_stat"
    )
}

/// Check if a name is a builtin, considering dynamic custom commands.
pub fn is_builtin_with_custom(name: &str, custom_commands: &std::collections::HashSet<String>) -> bool {
    is_builtin_static(name) || custom_commands.contains(name)
}

pub fn call_builtin(
    name: &str,
    args: Vec<Value>,
    output_tx: &Sender<ScriptEvent>,
) -> Result<Value, GrimScriptError> {
    match name {
        "print" => {
            let parts: Vec<String> = args.iter().map(|v| v.display()).collect();
            let msg = parts.join(" ");
            send_output(output_tx, &msg);
            Ok(Value::None)
        }
        "len" => {
            if args.len() != 1 {
                return Err(GrimScriptError::type_error(
                    0,
                    "len() takes exactly 1 argument",
                ));
            }
            match &args[0] {
                Value::List(l) => Ok(Value::Int(l.len() as i64)),
                Value::String(s) => Ok(Value::Int(s.len() as i64)),
                Value::Dict(d) => Ok(Value::Int(d.len() as i64)),
                Value::Tuple(t) => Ok(Value::Int(t.len() as i64)),
                other => Err(GrimScriptError::type_error(
                    0,
                    format!("object of type '{}' has no len()", other.type_name()),
                )),
            }
        }
        "range" => {
            let (start, end, step) = match args.len() {
                1 => {
                    let end = match &args[0] {
                        Value::Int(n) => *n,
                        _ => {
                            return Err(GrimScriptError::type_error(
                                0,
                                "range() argument must be int",
                            ))
                        }
                    };
                    (0i64, end, 1i64)
                }
                2 => {
                    let start = match &args[0] {
                        Value::Int(n) => *n,
                        _ => {
                            return Err(GrimScriptError::type_error(
                                0,
                                "range() argument must be int",
                            ))
                        }
                    };
                    let end = match &args[1] {
                        Value::Int(n) => *n,
                        _ => {
                            return Err(GrimScriptError::type_error(
                                0,
                                "range() argument must be int",
                            ))
                        }
                    };
                    (start, end, 1i64)
                }
                3 => {
                    let start = match &args[0] {
                        Value::Int(n) => *n,
                        _ => {
                            return Err(GrimScriptError::type_error(
                                0,
                                "range() argument must be int",
                            ))
                        }
                    };
                    let end = match &args[1] {
                        Value::Int(n) => *n,
                        _ => {
                            return Err(GrimScriptError::type_error(
                                0,
                                "range() argument must be int",
                            ))
                        }
                    };
                    let step = match &args[2] {
                        Value::Int(n) => *n,
                        _ => {
                            return Err(GrimScriptError::type_error(
                                0,
                                "range() argument must be int",
                            ))
                        }
                    };
                    if step == 0 {
                        return Err(GrimScriptError::runtime(
                            0,
                            "range() step argument must not be zero",
                        ));
                    }
                    (start, end, step)
                }
                _ => {
                    return Err(GrimScriptError::type_error(
                        0,
                        "range() takes 1 to 3 arguments",
                    ))
                }
            };

            let mut result = Vec::new();
            if step > 0 {
                let mut i = start;
                while i < end {
                    result.push(Value::Int(i));
                    i += step;
                }
            } else {
                let mut i = start;
                while i > end {
                    result.push(Value::Int(i));
                    i += step;
                }
            }
            Ok(Value::List(result))
        }
        "abs" => {
            if args.len() != 1 {
                return Err(GrimScriptError::type_error(
                    0,
                    "abs() takes exactly 1 argument",
                ));
            }
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(n.abs())),
                Value::Float(f) => Ok(Value::Float(f.abs())),
                _ => Err(GrimScriptError::type_error(
                    0,
                    "abs() argument must be numeric",
                )),
            }
        }
        "min" => {
            if args.is_empty() {
                return Err(GrimScriptError::type_error(
                    0,
                    "min() requires at least 1 argument",
                ));
            }
            if args.len() == 1 {
                if let Value::List(list) = &args[0] {
                    if list.is_empty() {
                        return Err(GrimScriptError::runtime(
                            0,
                            "min() arg is an empty sequence",
                        ));
                    }
                    let mut best = &list[0];
                    for item in list.iter().skip(1) {
                        if compare_values(item, best) == std::cmp::Ordering::Less {
                            best = item;
                        }
                    }
                    return Ok(best.clone());
                }
            }
            let mut best = &args[0];
            for item in args.iter().skip(1) {
                if compare_values(item, best) == std::cmp::Ordering::Less {
                    best = item;
                }
            }
            Ok(best.clone())
        }
        "max" => {
            if args.is_empty() {
                return Err(GrimScriptError::type_error(
                    0,
                    "max() requires at least 1 argument",
                ));
            }
            if args.len() == 1 {
                if let Value::List(list) = &args[0] {
                    if list.is_empty() {
                        return Err(GrimScriptError::runtime(
                            0,
                            "max() arg is an empty sequence",
                        ));
                    }
                    let mut best = &list[0];
                    for item in list.iter().skip(1) {
                        if compare_values(item, best) == std::cmp::Ordering::Greater {
                            best = item;
                        }
                    }
                    return Ok(best.clone());
                }
            }
            let mut best = &args[0];
            for item in args.iter().skip(1) {
                if compare_values(item, best) == std::cmp::Ordering::Greater {
                    best = item;
                }
            }
            Ok(best.clone())
        }
        "int" => {
            if args.len() != 1 {
                return Err(GrimScriptError::type_error(
                    0,
                    "int() takes exactly 1 argument",
                ));
            }
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(*n)),
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                Value::String(s) => s.parse::<i64>().map(Value::Int).map_err(|_| {
                    GrimScriptError::runtime(
                        0,
                        format!("invalid literal for int(): '{s}'"),
                    )
                }),
                Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                _ => Err(GrimScriptError::type_error(
                    0,
                    "int() argument must be a string or number",
                )),
            }
        }
        "float" => {
            if args.len() != 1 {
                return Err(GrimScriptError::type_error(
                    0,
                    "float() takes exactly 1 argument",
                ));
            }
            match &args[0] {
                Value::Int(n) => Ok(Value::Float(*n as f64)),
                Value::Float(f) => Ok(Value::Float(*f)),
                Value::String(s) => s.parse::<f64>().map(Value::Float).map_err(|_| {
                    GrimScriptError::runtime(
                        0,
                        format!("could not convert string to float: '{s}'"),
                    )
                }),
                _ => Err(GrimScriptError::type_error(
                    0,
                    "float() argument must be a string or number",
                )),
            }
        }
        "str" => {
            if args.len() != 1 {
                return Err(GrimScriptError::type_error(
                    0,
                    "str() takes exactly 1 argument",
                ));
            }
            Ok(Value::String(args[0].display()))
        }
        "type" => {
            if args.len() != 1 {
                return Err(GrimScriptError::type_error(
                    0,
                    "type() takes exactly 1 argument",
                ));
            }
            Ok(Value::String(args[0].type_name().to_string()))
        }
        "percent" => {
            if args.len() != 2 {
                return Err(GrimScriptError::type_error(
                    0,
                    "percent() takes exactly 2 arguments",
                ));
            }
            let value = match &args[0] {
                Value::Int(n) => *n,
                _ => return Err(GrimScriptError::type_error(0, "percent() arguments must be int")),
            };
            let pct = match &args[1] {
                Value::Int(n) => *n,
                _ => return Err(GrimScriptError::type_error(0, "percent() arguments must be int")),
            };
            let product = value.wrapping_mul(pct);
            Ok(Value::Int(bankers_div(product, 100)))
        }
        "scale" => {
            if args.len() != 3 {
                return Err(GrimScriptError::type_error(
                    0,
                    "scale() takes exactly 3 arguments",
                ));
            }
            let value = match &args[0] {
                Value::Int(n) => *n,
                _ => return Err(GrimScriptError::type_error(0, "scale() arguments must be int")),
            };
            let num = match &args[1] {
                Value::Int(n) => *n,
                _ => return Err(GrimScriptError::type_error(0, "scale() arguments must be int")),
            };
            let den = match &args[2] {
                Value::Int(n) => *n,
                _ => return Err(GrimScriptError::type_error(0, "scale() arguments must be int")),
            };
            if den == 0 {
                return Err(GrimScriptError::runtime(0, "scale() division by zero"));
            }
            let product = value.wrapping_mul(num);
            Ok(Value::Int(bankers_div(product, den)))
        }
        "append" => {
            // This is a special case - handled as method call in interpreter
            Err(GrimScriptError::runtime(
                0,
                "append() should be called as a method",
            ))
        }
        "move" => {
            send_output(output_tx, "[move] Moving...");
            Ok(Value::None)
        }
        "get_pos" => Ok(Value::Int(0)),
        "scan" => Ok(Value::List(vec![])),
        "nearest" => Ok(Value::Entity {
            id: 1,
            name: "target".into(),
            entity_type: "unknown".into(),
        }),
        "distance" => Ok(Value::Int(10)),
        "attack" => {
            send_output(output_tx, "[attack] Attacking...");
            Ok(Value::None)
        }
        "flee" => {
            send_output(output_tx, "[flee] Fleeing!");
            Ok(Value::None)
        }
        "get_health" => Ok(Value::Int(100)),
        "get_shield" => Ok(Value::Int(50)),
        "wait" => {
            send_output(output_tx, "[wait] Waiting...");
            Ok(Value::None)
        }
        "set_target" => Ok(Value::None),
        "get_target" => Ok(Value::None),
        "has_target" => Ok(Value::Bool(false)),
        "get_type" => Ok(Value::String("unknown".into())),
        "get_name" => Ok(Value::String("entity".into())),
        "get_owner" => Ok(Value::String("player".into())),
        "get_resource" => Ok(Value::Int(0)),
        "gain_resource" => Ok(Value::Int(0)),
        "try_spend_resource" => Ok(Value::Bool(true)),
        "get_custom_stat" => Ok(Value::Int(0)),
        _ => {
            // Custom command stub — just prints the command name.
            send_output(output_tx, &format!("[{name}] (custom command)"));
            Ok(Value::None)
        }
    }
}

/// Call a builtin, checking custom commands for unknown names.
pub fn call_builtin_with_custom(
    name: &str,
    args: Vec<Value>,
    output_tx: &Sender<ScriptEvent>,
    custom_commands: &std::collections::HashSet<String>,
) -> Result<Value, GrimScriptError> {
    if is_builtin_static(name) {
        call_builtin(name, args, output_tx)
    } else if custom_commands.contains(name) {
        send_output(output_tx, &format!("[{name}] (custom command)"));
        Ok(Value::None)
    } else {
        Err(GrimScriptError::runtime(
            0,
            format!("Unknown function: {name}"),
        ))
    }
}

/// Integer division with banker's rounding (round half to even).
fn bankers_div(numerator: i64, denominator: i64) -> i64 {
    let quotient = numerator / denominator;
    let remainder = (numerator % denominator).abs();
    let half = denominator.abs() / 2;
    let is_exact_half = denominator.abs() % 2 == 0 && remainder == half;

    if is_exact_half {
        if quotient % 2 == 0 { quotient } else { quotient + numerator.signum() }
    } else if remainder > half {
        quotient + numerator.signum()
    } else {
        quotient
    }
}

fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Int(x), Value::Float(y)) => (*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::Float(x), Value::Int(y)) => x
            .partial_cmp(&(*y as f64))
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => std::cmp::Ordering::Equal,
    }
}
