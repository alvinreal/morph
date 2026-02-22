use crate::error;
use crate::value::Value;

/// Call a built-in function by name.
pub fn call_function(name: &str, args: &[Value]) -> error::Result<Value> {
    match name {
        // String functions
        "lower" | "lowercase" | "downcase" => fn_lower(args),
        "upper" | "uppercase" | "upcase" => fn_upper(args),
        "trim" => fn_trim(args),
        "trim_start" | "ltrim" => fn_trim_start(args),
        "trim_end" | "rtrim" => fn_trim_end(args),
        "len" | "length" | "size" => fn_len(args),
        "replace" => fn_replace(args),
        "contains" => fn_contains(args),
        "starts_with" => fn_starts_with(args),
        "ends_with" => fn_ends_with(args),
        "substr" | "substring" => fn_substr(args),
        "concat" => fn_concat(args),
        "split" => fn_split(args),
        "join" => fn_join(args),
        "reverse" => fn_reverse(args),

        // Type functions
        "to_int" | "int" => fn_to_int(args),
        "to_float" | "float" => fn_to_float(args),
        "to_string" | "string" | "str" => fn_to_string(args),
        "to_bool" | "bool" => fn_to_bool(args),
        "type_of" | "typeof" => fn_type_of(args),

        // Math functions
        "abs" => fn_abs(args),
        "min" => fn_min(args),
        "max" => fn_max(args),
        "floor" => fn_floor(args),
        "ceil" => fn_ceil(args),
        "round" => fn_round(args),

        // Null / existence
        "is_null" => fn_is_null(args),
        "is_array" => fn_is_array(args),
        "coalesce" => fn_coalesce(args),
        "default" => fn_default(args),

        // Collection functions
        "keys" => fn_keys(args),
        "values" => fn_values(args),
        "unique" => fn_unique(args),
        "first" => fn_first(args),
        "last" => fn_last(args),
        "sum" => fn_sum(args),
        "group_by" | "groupby" => fn_group_by(args),

        // Conditional
        "if" => fn_if(args),

        _ => {
            let mut msg = format!("unknown function: {name}");
            if let Some(suggestion) = crate::error::suggest_function(name) {
                msg.push_str(&format!(". Did you mean '{suggestion}'?"));
            }
            Err(error::MorphError::mapping(msg))
        }
    }
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------

fn expect_args(name: &str, args: &[Value], expected: usize) -> error::Result<()> {
    if args.len() != expected {
        return Err(error::MorphError::mapping(format!(
            "{name}() expects {expected} argument(s), got {}",
            args.len()
        )));
    }
    Ok(())
}

fn expect_min_args(name: &str, args: &[Value], min: usize) -> error::Result<()> {
    if args.len() < min {
        return Err(error::MorphError::mapping(format!(
            "{name}() expects at least {min} argument(s), got {}",
            args.len()
        )));
    }
    Ok(())
}

pub fn to_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".into(),
        _ => format!("{value:?}"),
    }
}

fn fn_lower(args: &[Value]) -> error::Result<Value> {
    expect_args("lower", args, 1)?;
    Ok(Value::String(to_str(&args[0]).to_lowercase()))
}

fn fn_upper(args: &[Value]) -> error::Result<Value> {
    expect_args("upper", args, 1)?;
    Ok(Value::String(to_str(&args[0]).to_uppercase()))
}

fn fn_trim(args: &[Value]) -> error::Result<Value> {
    expect_args("trim", args, 1)?;
    Ok(Value::String(to_str(&args[0]).trim().to_string()))
}

fn fn_trim_start(args: &[Value]) -> error::Result<Value> {
    expect_args("trim_start", args, 1)?;
    Ok(Value::String(to_str(&args[0]).trim_start().to_string()))
}

fn fn_trim_end(args: &[Value]) -> error::Result<Value> {
    expect_args("trim_end", args, 1)?;
    Ok(Value::String(to_str(&args[0]).trim_end().to_string()))
}

fn fn_len(args: &[Value]) -> error::Result<Value> {
    expect_args("len", args, 1)?;
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.len() as i64)),
        Value::Array(a) => Ok(Value::Int(a.len() as i64)),
        Value::Map(m) => Ok(Value::Int(m.len() as i64)),
        Value::Bytes(b) => Ok(Value::Int(b.len() as i64)),
        _ => Ok(Value::Int(0)),
    }
}

fn fn_replace(args: &[Value]) -> error::Result<Value> {
    expect_args("replace", args, 3)?;
    let s = to_str(&args[0]);
    let from = to_str(&args[1]);
    let to = to_str(&args[2]);
    Ok(Value::String(s.replace(&from, &to)))
}

fn fn_contains(args: &[Value]) -> error::Result<Value> {
    expect_args("contains", args, 2)?;
    let s = to_str(&args[0]);
    let needle = to_str(&args[1]);
    Ok(Value::Bool(s.contains(&needle)))
}

fn fn_starts_with(args: &[Value]) -> error::Result<Value> {
    expect_args("starts_with", args, 2)?;
    let s = to_str(&args[0]);
    let prefix = to_str(&args[1]);
    Ok(Value::Bool(s.starts_with(&prefix)))
}

fn fn_ends_with(args: &[Value]) -> error::Result<Value> {
    expect_args("ends_with", args, 2)?;
    let s = to_str(&args[0]);
    let suffix = to_str(&args[1]);
    Ok(Value::Bool(s.ends_with(&suffix)))
}

fn fn_substr(args: &[Value]) -> error::Result<Value> {
    expect_min_args("substr", args, 2)?;
    let s = to_str(&args[0]);
    let start = match &args[1] {
        Value::Int(i) => *i as usize,
        _ => {
            return Err(error::MorphError::mapping(
                "substr() start index must be an integer",
            ));
        }
    };
    let len = if args.len() >= 3 {
        match &args[2] {
            Value::Int(i) => Some(*i as usize),
            _ => {
                return Err(error::MorphError::mapping(
                    "substr() length must be an integer",
                ));
            }
        }
    } else {
        None
    };

    let chars: Vec<char> = s.chars().collect();
    let start = start.min(chars.len());
    let result: String = match len {
        Some(l) => chars[start..].iter().take(l).collect(),
        None => chars[start..].iter().collect(),
    };
    Ok(Value::String(result))
}

fn fn_concat(args: &[Value]) -> error::Result<Value> {
    let result: String = args.iter().map(to_str).collect();
    Ok(Value::String(result))
}

fn fn_split(args: &[Value]) -> error::Result<Value> {
    expect_args("split", args, 2)?;
    let s = to_str(&args[0]);
    let sep = to_str(&args[1]);
    let parts: Vec<Value> = s
        .split(&sep)
        .map(|p| Value::String(p.to_string()))
        .collect();
    Ok(Value::Array(parts))
}

fn fn_join(args: &[Value]) -> error::Result<Value> {
    expect_min_args("join", args, 2)?;
    // When the first argument is an array and there are exactly 2 args,
    // behave as array-join(array, separator).
    if args.len() == 2 {
        if let Value::Array(a) = &args[0] {
            let sep = to_str(&args[1]);
            let parts: Vec<String> = a.iter().map(to_str).collect();
            return Ok(Value::String(parts.join(&sep)));
        }
    }
    // Otherwise concatenate all arguments as strings.
    let result: String = args.iter().map(to_str).collect();
    Ok(Value::String(result))
}

fn fn_reverse(args: &[Value]) -> error::Result<Value> {
    expect_args("reverse", args, 1)?;
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.chars().rev().collect())),
        Value::Array(a) => {
            let mut reversed = a.clone();
            reversed.reverse();
            Ok(Value::Array(reversed))
        }
        _ => Err(error::MorphError::mapping(
            "reverse() expects a string or array",
        )),
    }
}

// ---------------------------------------------------------------------------
// Type functions
// ---------------------------------------------------------------------------

fn fn_to_int(args: &[Value]) -> error::Result<Value> {
    expect_args("to_int", args, 1)?;
    match &args[0] {
        Value::Int(_) => Ok(args[0].clone()),
        Value::Float(f) => Ok(Value::Int(*f as i64)),
        Value::String(s) => s
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| error::MorphError::mapping(format!("cannot convert \"{s}\" to int"))),
        Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
        _ => Err(error::MorphError::mapping(format!(
            "cannot convert {:?} to int",
            args[0]
        ))),
    }
}

fn fn_to_float(args: &[Value]) -> error::Result<Value> {
    expect_args("to_float", args, 1)?;
    match &args[0] {
        Value::Float(_) => Ok(args[0].clone()),
        Value::Int(i) => Ok(Value::Float(*i as f64)),
        Value::String(s) => s
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| error::MorphError::mapping(format!("cannot convert \"{s}\" to float"))),
        Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Err(error::MorphError::mapping(format!(
            "cannot convert {:?} to float",
            args[0]
        ))),
    }
}

fn fn_to_string(args: &[Value]) -> error::Result<Value> {
    expect_args("to_string", args, 1)?;
    Ok(Value::String(to_str(&args[0])))
}

fn fn_to_bool(args: &[Value]) -> error::Result<Value> {
    expect_args("to_bool", args, 1)?;
    match &args[0] {
        Value::Bool(_) => Ok(args[0].clone()),
        Value::Int(i) => Ok(Value::Bool(*i != 0)),
        Value::Float(f) => Ok(Value::Bool(*f != 0.0)),
        Value::String(s) => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(Value::Bool(true)),
            "false" | "0" | "no" | "" => Ok(Value::Bool(false)),
            _ => Err(error::MorphError::mapping(format!(
                "cannot convert \"{s}\" to bool"
            ))),
        },
        Value::Null => Ok(Value::Bool(false)),
        _ => Err(error::MorphError::mapping(format!(
            "cannot convert {:?} to bool",
            args[0]
        ))),
    }
}

fn fn_type_of(args: &[Value]) -> error::Result<Value> {
    expect_args("type_of", args, 1)?;
    let type_name = match &args[0] {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Map(_) => "map",
        Value::Bytes(_) => "bytes",
    };
    Ok(Value::String(type_name.to_string()))
}

// ---------------------------------------------------------------------------
// Math functions
// ---------------------------------------------------------------------------

fn fn_abs(args: &[Value]) -> error::Result<Value> {
    expect_args("abs", args, 1)?;
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(i.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err(error::MorphError::mapping("abs() expects a number")),
    }
}

fn fn_min(args: &[Value]) -> error::Result<Value> {
    expect_min_args("min", args, 2)?;
    let mut result = &args[0];
    for arg in &args[1..] {
        if compare_for_minmax(arg, result) == std::cmp::Ordering::Less {
            result = arg;
        }
    }
    Ok(result.clone())
}

fn fn_max(args: &[Value]) -> error::Result<Value> {
    expect_min_args("max", args, 2)?;
    let mut result = &args[0];
    for arg in &args[1..] {
        if compare_for_minmax(arg, result) == std::cmp::Ordering::Greater {
            result = arg;
        }
    }
    Ok(result.clone())
}

fn compare_for_minmax(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Int(a), Value::Float(b)) => (*a as f64)
            .partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::Float(a), Value::Int(b)) => a
            .partial_cmp(&(*b as f64))
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => std::cmp::Ordering::Equal,
    }
}

fn fn_floor(args: &[Value]) -> error::Result<Value> {
    expect_args("floor", args, 1)?;
    match &args[0] {
        Value::Float(f) => Ok(Value::Int(f.floor() as i64)),
        Value::Int(_) => Ok(args[0].clone()),
        _ => Err(error::MorphError::mapping("floor() expects a number")),
    }
}

fn fn_ceil(args: &[Value]) -> error::Result<Value> {
    expect_args("ceil", args, 1)?;
    match &args[0] {
        Value::Float(f) => Ok(Value::Int(f.ceil() as i64)),
        Value::Int(_) => Ok(args[0].clone()),
        _ => Err(error::MorphError::mapping("ceil() expects a number")),
    }
}

fn fn_round(args: &[Value]) -> error::Result<Value> {
    expect_args("round", args, 1)?;
    match &args[0] {
        Value::Float(f) => Ok(Value::Int(f.round() as i64)),
        Value::Int(_) => Ok(args[0].clone()),
        _ => Err(error::MorphError::mapping("round() expects a number")),
    }
}

// ---------------------------------------------------------------------------
// Null / existence functions
// ---------------------------------------------------------------------------

fn fn_is_null(args: &[Value]) -> error::Result<Value> {
    expect_args("is_null", args, 1)?;
    Ok(Value::Bool(args[0] == Value::Null))
}

fn fn_coalesce(args: &[Value]) -> error::Result<Value> {
    expect_min_args("coalesce", args, 1)?;
    for arg in args {
        if *arg != Value::Null {
            return Ok(arg.clone());
        }
    }
    Ok(Value::Null)
}

fn fn_default(args: &[Value]) -> error::Result<Value> {
    expect_args("default", args, 2)?;
    if args[0] == Value::Null {
        Ok(args[1].clone())
    } else {
        Ok(args[0].clone())
    }
}

// ---------------------------------------------------------------------------
// Collection functions
// ---------------------------------------------------------------------------

fn fn_keys(args: &[Value]) -> error::Result<Value> {
    expect_args("keys", args, 1)?;
    match &args[0] {
        Value::Map(m) => Ok(Value::Array(
            m.keys().map(|k| Value::String(k.clone())).collect(),
        )),
        _ => Err(error::MorphError::mapping("keys() expects a map")),
    }
}

fn fn_values(args: &[Value]) -> error::Result<Value> {
    expect_args("values", args, 1)?;
    match &args[0] {
        Value::Map(m) => Ok(Value::Array(m.values().cloned().collect())),
        _ => Err(error::MorphError::mapping("values() expects a map")),
    }
}

fn fn_unique(args: &[Value]) -> error::Result<Value> {
    expect_args("unique", args, 1)?;
    match &args[0] {
        Value::Array(arr) => {
            let mut seen = Vec::new();
            for item in arr {
                if !seen.contains(item) {
                    seen.push(item.clone());
                }
            }
            Ok(Value::Array(seen))
        }
        _ => Err(error::MorphError::mapping("unique() expects an array")),
    }
}

fn fn_first(args: &[Value]) -> error::Result<Value> {
    expect_args("first", args, 1)?;
    match &args[0] {
        Value::Array(arr) => Ok(arr.first().cloned().unwrap_or(Value::Null)),
        _ => Err(error::MorphError::mapping("first() expects an array")),
    }
}

fn fn_last(args: &[Value]) -> error::Result<Value> {
    expect_args("last", args, 1)?;
    match &args[0] {
        Value::Array(arr) => Ok(arr.last().cloned().unwrap_or(Value::Null)),
        _ => Err(error::MorphError::mapping("last() expects an array")),
    }
}

fn fn_sum(args: &[Value]) -> error::Result<Value> {
    expect_args("sum", args, 1)?;
    match &args[0] {
        Value::Array(arr) => {
            let mut int_sum: i64 = 0;
            let mut is_float = false;
            let mut float_sum: f64 = 0.0;
            for item in arr {
                match item {
                    Value::Int(i) => {
                        int_sum += i;
                        float_sum += *i as f64;
                    }
                    Value::Float(f) => {
                        is_float = true;
                        float_sum += f;
                    }
                    _ => {
                        return Err(error::MorphError::mapping(
                            "sum() array must contain only numbers",
                        ));
                    }
                }
            }
            if is_float {
                Ok(Value::Float(float_sum))
            } else {
                Ok(Value::Int(int_sum))
            }
        }
        _ => Err(error::MorphError::mapping("sum() expects an array")),
    }
}

fn fn_group_by(args: &[Value]) -> error::Result<Value> {
    expect_args("group_by", args, 2)?;
    let arr = match &args[0] {
        Value::Array(a) => a,
        _ => {
            return Err(error::MorphError::mapping(
                "group_by() first argument must be an array",
            ));
        }
    };
    let key_field = match &args[1] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(error::MorphError::mapping(
                "group_by() second argument must be a string field name",
            ));
        }
    };

    let mut groups: indexmap::IndexMap<String, Vec<Value>> = indexmap::IndexMap::new();
    for item in arr {
        let key_val = match item {
            Value::Map(m) => m.get(&key_field).cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };
        let key_str = to_str(&key_val);
        groups.entry(key_str).or_default().push(item.clone());
    }

    let mut result = indexmap::IndexMap::new();
    for (key, values) in groups {
        result.insert(key, Value::Array(values));
    }
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Conditional functions
// ---------------------------------------------------------------------------

fn fn_if(args: &[Value]) -> error::Result<Value> {
    expect_args("if", args, 3)?;
    let condition = &args[0];
    let is_truthy = match condition {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Map(m) => !m.is_empty(),
        Value::Bytes(b) => !b.is_empty(),
    };
    if is_truthy {
        Ok(args[1].clone())
    } else {
        Ok(args[2].clone())
    }
}

// ---------------------------------------------------------------------------
// Null / existence functions
// ---------------------------------------------------------------------------

fn fn_is_array(args: &[Value]) -> error::Result<Value> {
    expect_args("is_array", args, 1)?;
    Ok(Value::Bool(matches!(&args[0], Value::Array(_))))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // String functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_lower() {
        let r = call_function("lower", &[Value::String("HELLO".into())]).unwrap();
        assert_eq!(r, Value::String("hello".into()));
    }

    #[test]
    fn test_lower_already_lower() {
        let r = call_function("lower", &[Value::String("already lower".into())]).unwrap();
        assert_eq!(r, Value::String("already lower".into()));
    }

    #[test]
    fn test_lower_non_string_coerces() {
        // lower(42) coerces to string via to_str, producing "42"
        let r = call_function("lower", &[Value::Int(42)]).unwrap();
        assert_eq!(r, Value::String("42".into()));
    }

    #[test]
    fn test_upper() {
        let r = call_function("upper", &[Value::String("hello".into())]).unwrap();
        assert_eq!(r, Value::String("HELLO".into()));
    }

    #[test]
    fn test_trim() {
        let r = call_function("trim", &[Value::String("  hello  ".into())]).unwrap();
        assert_eq!(r, Value::String("hello".into()));
    }

    #[test]
    fn test_trim_no_spaces() {
        let r = call_function("trim", &[Value::String("no-spaces".into())]).unwrap();
        assert_eq!(r, Value::String("no-spaces".into()));
    }

    #[test]
    fn test_trim_start() {
        let r = call_function("trim_start", &[Value::String("  hi  ".into())]).unwrap();
        assert_eq!(r, Value::String("hi  ".into()));
    }

    #[test]
    fn test_trim_end() {
        let r = call_function("trim_end", &[Value::String("  hi  ".into())]).unwrap();
        assert_eq!(r, Value::String("  hi".into()));
    }

    #[test]
    fn test_len_string() {
        let r = call_function("len", &[Value::String("hello".into())]).unwrap();
        assert_eq!(r, Value::Int(5));
    }

    #[test]
    fn test_len_empty_string() {
        let r = call_function("len", &[Value::String("".into())]).unwrap();
        assert_eq!(r, Value::Int(0));
    }

    #[test]
    fn test_len_array() {
        let r = call_function("len", &[Value::Array(vec![Value::Int(1), Value::Int(2)])]).unwrap();
        assert_eq!(r, Value::Int(2));
    }

    #[test]
    fn test_len_array_3() {
        let r = call_function(
            "len",
            &[Value::Array(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
            ])],
        )
        .unwrap();
        assert_eq!(r, Value::Int(3));
    }

    #[test]
    fn test_replace() {
        let r = call_function(
            "replace",
            &[
                Value::String("hello world".into()),
                Value::String("world".into()),
                Value::String("rust".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::String("hello rust".into()));
    }

    #[test]
    fn test_replace_all_occurrences() {
        let r = call_function(
            "replace",
            &[
                Value::String("aaa".into()),
                Value::String("a".into()),
                Value::String("b".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::String("bbb".into()));
    }

    #[test]
    fn test_contains() {
        let r = call_function(
            "contains",
            &[
                Value::String("hello world".into()),
                Value::String("world".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::Bool(true));
    }

    #[test]
    fn test_contains_false() {
        let r = call_function(
            "contains",
            &[Value::String("hello".into()), Value::String("world".into())],
        )
        .unwrap();
        assert_eq!(r, Value::Bool(false));
    }

    #[test]
    fn test_starts_with() {
        let r = call_function(
            "starts_with",
            &[
                Value::String("hello world".into()),
                Value::String("hello".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::Bool(true));
    }

    #[test]
    fn test_ends_with() {
        let r = call_function(
            "ends_with",
            &[
                Value::String("hello world".into()),
                Value::String("world".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::Bool(true));
    }

    #[test]
    fn test_substr() {
        let r = call_function(
            "substr",
            &[Value::String("hello world".into()), Value::Int(6)],
        )
        .unwrap();
        assert_eq!(r, Value::String("world".into()));
    }

    #[test]
    fn test_substr_with_length() {
        let r = call_function(
            "substr",
            &[
                Value::String("hello world".into()),
                Value::Int(0),
                Value::Int(5),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::String("hello".into()));
    }

    #[test]
    fn test_concat() {
        let r = call_function(
            "concat",
            &[
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::String("abc".into()));
    }

    #[test]
    fn test_split() {
        let r = call_function(
            "split",
            &[Value::String("a,b,c".into()), Value::String(",".into())],
        )
        .unwrap();
        assert_eq!(
            r,
            Value::Array(vec![
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ])
        );
    }

    #[test]
    fn test_split_no_match() {
        // split("hello", "x") → ["hello"]
        let r = call_function(
            "split",
            &[Value::String("hello".into()), Value::String("x".into())],
        )
        .unwrap();
        assert_eq!(r, Value::Array(vec![Value::String("hello".into())]));
    }

    #[test]
    fn test_join_array() {
        let arr = Value::Array(vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ]);
        let r = call_function("join", &[arr, Value::String(",".into())]).unwrap();
        assert_eq!(r, Value::String("a,b,c".into()));
    }

    #[test]
    fn test_join_strings() {
        // join("a", "b", "c") → "abc"
        let r = call_function(
            "join",
            &[
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::String("abc".into()));
    }

    #[test]
    fn test_join_with_field_values() {
        // join(.first, " ", .last) simulated with direct values
        let r = call_function(
            "join",
            &[
                Value::String("John".into()),
                Value::String(" ".into()),
                Value::String("Doe".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, Value::String("John Doe".into()));
    }

    #[test]
    fn test_reverse_string() {
        let r = call_function("reverse", &[Value::String("hello".into())]).unwrap();
        assert_eq!(r, Value::String("olleh".into()));
    }

    #[test]
    fn test_reverse_array() {
        let r = call_function(
            "reverse",
            &[Value::Array(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
            ])],
        )
        .unwrap();
        assert_eq!(
            r,
            Value::Array(vec![Value::Int(3), Value::Int(2), Value::Int(1)])
        );
    }

    // -----------------------------------------------------------------------
    // Type functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_int() {
        assert_eq!(
            call_function("to_int", &[Value::String("42".into())]).unwrap(),
            Value::Int(42)
        );
        assert_eq!(
            call_function("to_int", &[Value::Float(3.7)]).unwrap(),
            Value::Int(3)
        );
        assert_eq!(
            call_function("to_int", &[Value::Bool(true)]).unwrap(),
            Value::Int(1)
        );
    }

    #[test]
    fn test_to_float() {
        assert_eq!(
            call_function("to_float", &[Value::String("2.72".into())]).unwrap(),
            Value::Float(2.72)
        );
        assert_eq!(
            call_function("to_float", &[Value::Int(42)]).unwrap(),
            Value::Float(42.0)
        );
    }

    #[test]
    fn test_to_string() {
        assert_eq!(
            call_function("to_string", &[Value::Int(42)]).unwrap(),
            Value::String("42".into())
        );
    }

    #[test]
    fn test_to_bool() {
        assert_eq!(
            call_function("to_bool", &[Value::String("true".into())]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            call_function("to_bool", &[Value::Int(0)]).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_type_of() {
        assert_eq!(
            call_function("type_of", &[Value::Int(42)]).unwrap(),
            Value::String("int".into())
        );
        assert_eq!(
            call_function("type_of", &[Value::String("hello".into())]).unwrap(),
            Value::String("string".into())
        );
        assert_eq!(
            call_function("type_of", &[Value::Null]).unwrap(),
            Value::String("null".into())
        );
        assert_eq!(
            call_function("type_of", &[Value::Bool(true)]).unwrap(),
            Value::String("bool".into())
        );
    }

    // -----------------------------------------------------------------------
    // Math functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_abs() {
        assert_eq!(
            call_function("abs", &[Value::Int(-5)]).unwrap(),
            Value::Int(5)
        );
        assert_eq!(
            call_function("abs", &[Value::Float(-2.72)]).unwrap(),
            Value::Float(2.72)
        );
    }

    #[test]
    fn test_min() {
        assert_eq!(
            call_function("min", &[Value::Int(5), Value::Int(3)]).unwrap(),
            Value::Int(3)
        );
    }

    #[test]
    fn test_max() {
        assert_eq!(
            call_function("max", &[Value::Int(5), Value::Int(3)]).unwrap(),
            Value::Int(5)
        );
    }

    #[test]
    fn test_floor() {
        assert_eq!(
            call_function("floor", &[Value::Float(3.7)]).unwrap(),
            Value::Int(3)
        );
    }

    #[test]
    fn test_ceil() {
        assert_eq!(
            call_function("ceil", &[Value::Float(3.2)]).unwrap(),
            Value::Int(4)
        );
    }

    #[test]
    fn test_round() {
        assert_eq!(
            call_function("round", &[Value::Float(3.5)]).unwrap(),
            Value::Int(4)
        );
        assert_eq!(
            call_function("round", &[Value::Float(3.4)]).unwrap(),
            Value::Int(3)
        );
    }

    // -----------------------------------------------------------------------
    // Null functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_null() {
        assert_eq!(
            call_function("is_null", &[Value::Null]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            call_function("is_null", &[Value::Int(0)]).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_coalesce() {
        assert_eq!(
            call_function("coalesce", &[Value::Null, Value::Int(42)]).unwrap(),
            Value::Int(42)
        );
        assert_eq!(
            call_function(
                "coalesce",
                &[Value::Null, Value::Null, Value::String("fallback".into())]
            )
            .unwrap(),
            Value::String("fallback".into())
        );
    }

    #[test]
    fn test_coalesce_first_non_null() {
        // coalesce("first", "second") → "first"
        assert_eq!(
            call_function(
                "coalesce",
                &[
                    Value::String("first".into()),
                    Value::String("second".into())
                ]
            )
            .unwrap(),
            Value::String("first".into())
        );
    }

    #[test]
    fn test_default_fn() {
        assert_eq!(
            call_function("default", &[Value::Null, Value::Int(42)]).unwrap(),
            Value::Int(42)
        );
        assert_eq!(
            call_function("default", &[Value::Int(10), Value::Int(42)]).unwrap(),
            Value::Int(10)
        );
    }

    // -----------------------------------------------------------------------
    // Error: unknown function
    // -----------------------------------------------------------------------

    #[test]
    fn test_unknown_function() {
        let err = call_function("foobar", &[]).unwrap_err();
        match err {
            crate::error::MorphError::Mapping { message, .. } => {
                assert!(message.contains("unknown function"), "msg: {message}");
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: wrong arg count
    // -----------------------------------------------------------------------

    #[test]
    fn test_wrong_arg_count() {
        let err = call_function("lower", &[]).unwrap_err();
        match err {
            crate::error::MorphError::Mapping { message, .. } => {
                assert!(message.contains("expects"), "msg: {message}");
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Aliases
    // -----------------------------------------------------------------------

    #[test]
    fn test_aliases() {
        // All aliases should work
        assert!(call_function("lowercase", &[Value::String("A".into())]).is_ok());
        assert!(call_function("downcase", &[Value::String("A".into())]).is_ok());
        assert!(call_function("uppercase", &[Value::String("a".into())]).is_ok());
        assert!(call_function("upcase", &[Value::String("a".into())]).is_ok());
        assert!(call_function("length", &[Value::String("a".into())]).is_ok());
        assert!(call_function("size", &[Value::String("a".into())]).is_ok());
        assert!(call_function("ltrim", &[Value::String(" a ".into())]).is_ok());
        assert!(call_function("rtrim", &[Value::String(" a ".into())]).is_ok());
        assert!(call_function("substring", &[Value::String("abc".into()), Value::Int(0)]).is_ok());
        assert!(call_function("int", &[Value::String("42".into())]).is_ok());
        assert!(call_function("float", &[Value::String("3.15".into())]).is_ok());
        assert!(call_function("str", &[Value::Int(42)]).is_ok());
        assert!(call_function("string", &[Value::Int(42)]).is_ok());
        assert!(call_function("bool", &[Value::Int(1)]).is_ok());
        assert!(call_function("typeof", &[Value::Int(1)]).is_ok());
    }
}
