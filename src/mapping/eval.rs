use crate::error;
use crate::mapping::ast::*;
use crate::mapping::functions;
use crate::value::Value;
use indexmap::IndexMap;

/// Evaluate a parsed mapping program against a Value.
pub fn eval(program: &Program, input: &Value) -> error::Result<Value> {
    let mut value = input.clone();
    for stmt in &program.statements {
        value = eval_statement(stmt, &value)?;
    }
    Ok(value)
}

/// Evaluate a single statement against a Value.
fn eval_statement(stmt: &Statement, value: &Value) -> error::Result<Value> {
    match stmt {
        Statement::Rename { from, to, .. } => eval_rename(value, from, to),
        Statement::Select { paths, .. } => eval_select(value, paths),
        Statement::Drop { paths, .. } => eval_drop(value, paths),
        Statement::Set { path, expr, .. } => eval_set(value, path, expr),
        Statement::Default { path, expr, .. } => eval_default(value, path, expr),
        Statement::Cast {
            path, target_type, ..
        } => eval_cast(value, path, target_type),
        Statement::Flatten { path, prefix, .. } => eval_flatten(value, path, prefix.as_deref()),
        Statement::Nest { paths, target, .. } => eval_nest(value, paths, target),
        Statement::Where { condition, .. } => eval_where(value, condition),
        Statement::Sort { keys, .. } => eval_sort(value, keys),
    }
}

// ---------------------------------------------------------------------------
// rename
// ---------------------------------------------------------------------------

fn eval_rename(value: &Value, from: &Path, to: &Path) -> error::Result<Value> {
    // Get the value at the source path
    let extracted = resolve_path(value, &from.segments);
    match extracted {
        Some(val) => {
            // Remove from source
            let mut result = remove_path(value, &from.segments);
            // Set at destination
            result = set_path(&result, &to.segments, val);
            Ok(result)
        }
        None => Ok(value.clone()), // Source doesn't exist, no-op
    }
}

// ---------------------------------------------------------------------------
// select
// ---------------------------------------------------------------------------

fn eval_select(value: &Value, paths: &[Path]) -> error::Result<Value> {
    match value {
        Value::Map(_) => {
            let mut result = IndexMap::new();
            for path in paths {
                if let Some(val) = resolve_path(value, &path.segments) {
                    // Use the last segment name as the key
                    if let Some(key) = last_field_name(&path.segments) {
                        result.insert(key, val);
                    }
                }
            }
            Ok(Value::Map(result))
        }
        _ => Ok(value.clone()),
    }
}

// ---------------------------------------------------------------------------
// drop
// ---------------------------------------------------------------------------

fn eval_drop(value: &Value, paths: &[Path]) -> error::Result<Value> {
    let mut result = value.clone();
    for path in paths {
        result = remove_path(&result, &path.segments);
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// set
// ---------------------------------------------------------------------------

fn eval_set(value: &Value, path: &Path, expr: &Expr) -> error::Result<Value> {
    let val = eval_expr(expr, value)?;
    Ok(set_path(value, &path.segments, val))
}

// ---------------------------------------------------------------------------
// default
// ---------------------------------------------------------------------------

fn eval_default(value: &Value, path: &Path, expr: &Expr) -> error::Result<Value> {
    // Only set if path doesn't exist or is null
    let current = resolve_path(value, &path.segments);
    match current {
        None | Some(Value::Null) => {
            let val = eval_expr(expr, value)?;
            Ok(set_path(value, &path.segments, val))
        }
        Some(_) => Ok(value.clone()),
    }
}

// ---------------------------------------------------------------------------
// cast
// ---------------------------------------------------------------------------

fn eval_cast(value: &Value, path: &Path, target_type: &CastType) -> error::Result<Value> {
    let current = resolve_path(value, &path.segments);
    match current {
        Some(val) => {
            let casted = cast_value(&val, target_type)?;
            Ok(set_path(value, &path.segments, casted))
        }
        None => Ok(value.clone()),
    }
}

fn cast_value(value: &Value, target_type: &CastType) -> error::Result<Value> {
    match target_type {
        CastType::Int => match value {
            Value::Int(_) => Ok(value.clone()),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::String(s) => s.parse::<i64>().map(Value::Int).map_err(|_| {
                error::MorphError::mapping(format!("cannot cast string \"{s}\" to int"))
            }),
            Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
            Value::Null => Ok(Value::Int(0)),
            _ => Err(error::MorphError::mapping(format!(
                "cannot cast {value:?} to int"
            ))),
        },
        CastType::Float => match value {
            Value::Float(_) => Ok(value.clone()),
            Value::Int(i) => Ok(Value::Float(*i as f64)),
            Value::String(s) => s.parse::<f64>().map(Value::Float).map_err(|_| {
                error::MorphError::mapping(format!("cannot cast string \"{s}\" to float"))
            }),
            Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
            Value::Null => Ok(Value::Float(0.0)),
            _ => Err(error::MorphError::mapping(format!(
                "cannot cast {value:?} to float"
            ))),
        },
        CastType::String => match value {
            Value::String(_) => Ok(value.clone()),
            Value::Int(i) => Ok(Value::String(i.to_string())),
            Value::Float(f) => Ok(Value::String(f.to_string())),
            Value::Bool(b) => Ok(Value::String(b.to_string())),
            Value::Null => Ok(Value::String("null".to_string())),
            _ => Err(error::MorphError::mapping(format!(
                "cannot cast {value:?} to string"
            ))),
        },
        CastType::Bool => match value {
            Value::Bool(_) => Ok(value.clone()),
            Value::Int(i) => Ok(Value::Bool(*i != 0)),
            Value::Float(f) => Ok(Value::Bool(*f != 0.0)),
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(Value::Bool(true)),
                "false" | "0" | "no" | "" => Ok(Value::Bool(false)),
                _ => Err(error::MorphError::mapping(format!(
                    "cannot cast string \"{s}\" to bool"
                ))),
            },
            Value::Null => Ok(Value::Bool(false)),
            _ => Err(error::MorphError::mapping(format!(
                "cannot cast {value:?} to bool"
            ))),
        },
    }
}

// ---------------------------------------------------------------------------
// flatten
// ---------------------------------------------------------------------------

fn eval_flatten(value: &Value, path: &Path, prefix: Option<&str>) -> error::Result<Value> {
    let target_val = resolve_path(value, &path.segments);
    match target_val {
        Some(Value::Map(inner_map)) => {
            // Determine the prefix for flattened keys
            let key_prefix = match prefix {
                Some(p) => p.to_string(),
                None => {
                    // Use the last segment of the path as the prefix
                    last_field_name(&path.segments).unwrap_or_default()
                }
            };

            // Remove the original nested field
            let mut result = remove_path(value, &path.segments);

            // Insert flattened key-value pairs into the parent map
            if let Value::Map(ref mut parent_map) = result {
                for (key, val) in &inner_map {
                    let flat_key = format!("{key_prefix}_{key}");
                    parent_map.insert(flat_key, val.clone());
                }
            }

            Ok(result)
        }
        Some(_) => {
            // Non-object: no-op
            Ok(value.clone())
        }
        None => {
            // Path doesn't exist: no-op
            Ok(value.clone())
        }
    }
}

// ---------------------------------------------------------------------------
// nest
// ---------------------------------------------------------------------------

fn eval_nest(value: &Value, paths: &[Path], target: &Path) -> error::Result<Value> {
    // Get the target field name to strip as prefix
    let target_name = last_field_name(&target.segments).unwrap_or_default();

    let mut nested_map = IndexMap::new();
    let mut result = value.clone();

    for path in paths {
        if let Some(val) = resolve_path(value, &path.segments) {
            // Get the field name from the path
            let field_name = last_field_name(&path.segments).unwrap_or_default();

            // Strip target prefix (e.g., "a_x" with target "a" → "x")
            let nested_key = if !target_name.is_empty() {
                let prefix_with_underscore = format!("{target_name}_");
                if field_name.starts_with(&prefix_with_underscore) {
                    field_name[prefix_with_underscore.len()..].to_string()
                } else {
                    field_name
                }
            } else {
                field_name
            };

            nested_map.insert(nested_key, val);

            // Remove the original field
            result = remove_path(&result, &path.segments);
        }
    }

    // Set the nested map at the target path
    result = set_path(&result, &target.segments, Value::Map(nested_map));

    Ok(result)
}

// ---------------------------------------------------------------------------
// where (filter)
// ---------------------------------------------------------------------------

fn eval_where(value: &Value, condition: &Expr) -> error::Result<Value> {
    match value {
        Value::Array(arr) => {
            let mut filtered = Vec::new();
            for item in arr {
                let result = eval_expr(condition, item)?;
                if is_truthy(&result) {
                    filtered.push(item.clone());
                }
            }
            Ok(Value::Array(filtered))
        }
        _ => {
            // For non-arrays, apply as a boolean gate: if condition is true,
            // return value unchanged; otherwise return null
            let result = eval_expr(condition, value)?;
            if is_truthy(&result) {
                Ok(value.clone())
            } else {
                Ok(Value::Null)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sort
// ---------------------------------------------------------------------------

fn eval_sort(value: &Value, keys: &[SortKey]) -> error::Result<Value> {
    match value {
        Value::Array(arr) => {
            let mut sorted = arr.clone();
            sorted.sort_by(|a, b| {
                for key in keys {
                    let val_a = resolve_path(a, &key.path.segments).unwrap_or(Value::Null);
                    let val_b = resolve_path(b, &key.path.segments).unwrap_or(Value::Null);

                    // Nulls always sort last regardless of direction
                    let ordering = match (&val_a, &val_b) {
                        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
                        (Value::Null, _) => std::cmp::Ordering::Greater, // null last
                        (_, Value::Null) => std::cmp::Ordering::Less,    // null last
                        _ => compare_values(&val_a, &val_b).unwrap_or(std::cmp::Ordering::Equal),
                    };

                    let ordering = match key.direction {
                        SortDirection::Asc => ordering,
                        SortDirection::Desc => {
                            // Reverse, but keep nulls last
                            match (&val_a, &val_b) {
                                (Value::Null, _) | (_, Value::Null) => ordering,
                                _ => ordering.reverse(),
                            }
                        }
                    };

                    if ordering != std::cmp::Ordering::Equal {
                        return ordering;
                    }
                }
                std::cmp::Ordering::Equal
            });
            Ok(Value::Array(sorted))
        }
        _ => Ok(value.clone()), // non-array: no-op
    }
}

// ---------------------------------------------------------------------------
// Expression evaluation
// ---------------------------------------------------------------------------

fn eval_expr(expr: &Expr, context: &Value) -> error::Result<Value> {
    match expr {
        Expr::Literal(val) => Ok(val.clone()),
        Expr::Path(path) => Ok(resolve_path(context, &path.segments).unwrap_or(Value::Null)),
        Expr::FunctionCall { name, args, .. } => {
            let evaluated_args: Vec<Value> = args
                .iter()
                .map(|a| eval_expr(a, context))
                .collect::<error::Result<Vec<_>>>()?;
            functions::call_function(name, &evaluated_args)
        }
        Expr::BinaryOp { left, op, right } => {
            let l = eval_expr(left, context)?;
            let r = eval_expr(right, context)?;
            eval_binary_op(&l, *op, &r)
        }
        Expr::UnaryOp { op, expr } => {
            let val = eval_expr(expr, context)?;
            eval_unary_op(*op, &val)
        }
    }
}

fn eval_binary_op(left: &Value, op: BinOp, right: &Value) -> error::Result<Value> {
    match op {
        // Arithmetic
        BinOp::Add => eval_add(left, right),
        BinOp::Sub => eval_arithmetic(left, right, |a, b| a - b, |a, b| a - b),
        BinOp::Mul => eval_arithmetic(left, right, |a, b| a * b, |a, b| a * b),
        BinOp::Div => {
            // Check for division by zero
            match right {
                Value::Int(0) => {
                    return Err(error::MorphError::mapping("division by zero"));
                }
                Value::Float(f) if *f == 0.0 => {
                    return Err(error::MorphError::mapping("division by zero"));
                }
                _ => {}
            }
            eval_arithmetic(left, right, |a, b| a / b, |a, b| a / b)
        }
        BinOp::Mod => {
            if let Value::Int(0) = right {
                return Err(error::MorphError::mapping("modulo by zero"));
            }
            eval_arithmetic(left, right, |a, b| a % b, |a, b| a % b)
        }

        // Comparison
        BinOp::Eq => Ok(Value::Bool(values_equal(left, right))),
        BinOp::NotEq => Ok(Value::Bool(!values_equal(left, right))),
        BinOp::Gt => Ok(Value::Bool(
            compare_values(left, right) == Some(std::cmp::Ordering::Greater),
        )),
        BinOp::GtEq => Ok(Value::Bool(matches!(
            compare_values(left, right),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ))),
        BinOp::Lt => Ok(Value::Bool(
            compare_values(left, right) == Some(std::cmp::Ordering::Less),
        )),
        BinOp::LtEq => Ok(Value::Bool(matches!(
            compare_values(left, right),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ))),

        // Logical
        BinOp::And => Ok(Value::Bool(is_truthy(left) && is_truthy(right))),
        BinOp::Or => Ok(Value::Bool(is_truthy(left) || is_truthy(right))),
    }
}

fn eval_add(left: &Value, right: &Value) -> error::Result<Value> {
    match (left, right) {
        // String concatenation
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
        (Value::String(a), other) => Ok(Value::String(format!("{a}{}", value_to_display(other)))),
        (other, Value::String(b)) => Ok(Value::String(format!("{}{b}", value_to_display(other)))),
        // Numeric addition
        _ => eval_arithmetic(left, right, |a, b| a + b, |a, b| a + b),
    }
}

fn eval_arithmetic<F1, F2>(
    left: &Value,
    right: &Value,
    int_op: F1,
    float_op: F2,
) -> error::Result<Value>
where
    F1: Fn(i64, i64) -> i64,
    F2: Fn(f64, f64) -> f64,
{
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(*a, *b as f64))),
        _ => Err(error::MorphError::mapping(format!(
            "cannot perform arithmetic on {left:?} and {right:?}"
        ))),
    }
}

fn eval_unary_op(op: UnaryOp, value: &Value) -> error::Result<Value> {
    match op {
        UnaryOp::Not => Ok(Value::Bool(!is_truthy(value))),
        UnaryOp::Neg => match value {
            Value::Int(i) => Ok(Value::Int(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err(error::MorphError::mapping(format!(
                "cannot negate {value:?}"
            ))),
        },
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
        (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
        _ => a == b,
    }
}

fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
        (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
        (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
        (Value::String(a), Value::String(b)) => a.partial_cmp(b),
        _ => None,
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Map(m) => !m.is_empty(),
        Value::Bytes(b) => !b.is_empty(),
    }
}

fn value_to_display(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) => "[array]".into(),
        Value::Map(_) => "{map}".into(),
        Value::Bytes(_) => "[bytes]".into(),
    }
}

// ---------------------------------------------------------------------------
// Path resolution helpers
// ---------------------------------------------------------------------------

fn resolve_path(value: &Value, segments: &[PathSegment]) -> Option<Value> {
    if segments.is_empty() {
        return Some(value.clone());
    }

    let (first, rest) = segments.split_first().unwrap();
    match first {
        PathSegment::Field(name) => match value {
            Value::Map(map) => map.get(name).and_then(|v| resolve_path(v, rest)),
            _ => None,
        },
        PathSegment::Index(idx) => match value {
            Value::Array(arr) => {
                let index = if *idx < 0 {
                    (arr.len() as i64 + idx) as usize
                } else {
                    *idx as usize
                };
                arr.get(index).and_then(|v| resolve_path(v, rest))
            }
            _ => None,
        },
        PathSegment::Wildcard => match value {
            Value::Array(arr) => {
                let results: Vec<Value> = arr
                    .iter()
                    .filter_map(|item| resolve_path(item, rest))
                    .collect();
                if results.is_empty() {
                    None
                } else {
                    Some(Value::Array(results))
                }
            }
            _ => None,
        },
    }
}

fn set_path(value: &Value, segments: &[PathSegment], new_val: Value) -> Value {
    if segments.is_empty() {
        return new_val;
    }

    let (first, rest) = segments.split_first().unwrap();
    match first {
        PathSegment::Field(name) => {
            let mut map = match value {
                Value::Map(m) => m.clone(),
                _ => IndexMap::new(),
            };
            let existing = map.get(name).cloned().unwrap_or(Value::Null);
            let updated = set_path(&existing, rest, new_val);
            map.insert(name.clone(), updated);
            Value::Map(map)
        }
        PathSegment::Index(idx) => {
            let mut arr = match value {
                Value::Array(a) => a.clone(),
                _ => Vec::new(),
            };
            let index = if *idx < 0 {
                (arr.len() as i64 + idx) as usize
            } else {
                *idx as usize
            };
            // Extend array if needed
            while arr.len() <= index {
                arr.push(Value::Null);
            }
            let existing = arr[index].clone();
            arr[index] = set_path(&existing, rest, new_val);
            Value::Array(arr)
        }
        PathSegment::Wildcard => {
            // Set on all elements
            match value {
                Value::Array(arr) => {
                    let updated: Vec<Value> = arr
                        .iter()
                        .map(|item| set_path(item, rest, new_val.clone()))
                        .collect();
                    Value::Array(updated)
                }
                _ => value.clone(),
            }
        }
    }
}

fn remove_path(value: &Value, segments: &[PathSegment]) -> Value {
    if segments.is_empty() {
        return Value::Null;
    }

    if segments.len() == 1 {
        match &segments[0] {
            PathSegment::Field(name) => match value {
                Value::Map(map) => {
                    let mut new_map = map.clone();
                    new_map.shift_remove(name);
                    Value::Map(new_map)
                }
                _ => value.clone(),
            },
            PathSegment::Index(idx) => match value {
                Value::Array(arr) => {
                    let index = if *idx < 0 {
                        (arr.len() as i64 + idx) as usize
                    } else {
                        *idx as usize
                    };
                    let mut new_arr = arr.clone();
                    if index < new_arr.len() {
                        new_arr.remove(index);
                    }
                    Value::Array(new_arr)
                }
                _ => value.clone(),
            },
            PathSegment::Wildcard => value.clone(),
        }
    } else {
        let (first, rest) = segments.split_first().unwrap();
        match first {
            PathSegment::Field(name) => match value {
                Value::Map(map) => {
                    let mut new_map = map.clone();
                    if let Some(child) = new_map.get(name) {
                        let updated = remove_path(child, rest);
                        new_map.insert(name.clone(), updated);
                    }
                    Value::Map(new_map)
                }
                _ => value.clone(),
            },
            PathSegment::Index(idx) => match value {
                Value::Array(arr) => {
                    let index = if *idx < 0 {
                        (arr.len() as i64 + idx) as usize
                    } else {
                        *idx as usize
                    };
                    let mut new_arr = arr.clone();
                    if index < new_arr.len() {
                        new_arr[index] = remove_path(&new_arr[index], rest);
                    }
                    Value::Array(new_arr)
                }
                _ => value.clone(),
            },
            PathSegment::Wildcard => match value {
                Value::Array(arr) => {
                    let updated: Vec<Value> =
                        arr.iter().map(|item| remove_path(item, rest)).collect();
                    Value::Array(updated)
                }
                _ => value.clone(),
            },
        }
    }
}

fn last_field_name(segments: &[PathSegment]) -> Option<String> {
    for seg in segments.iter().rev() {
        if let PathSegment::Field(name) = seg {
            return Some(name.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::parser;

    fn run(input: &str, value: &Value) -> Value {
        let program = parser::parse_str(input).unwrap();
        eval(&program, value).unwrap()
    }

    fn run_err(input: &str, value: &Value) -> error::MorphError {
        let program = parser::parse_str(input).unwrap();
        eval(&program, value).unwrap_err()
    }

    fn simple_map() -> Value {
        let mut m = IndexMap::new();
        m.insert("name".into(), Value::String("Alice".into()));
        m.insert("age".into(), Value::Int(30));
        m.insert("score".into(), Value::Float(95.5));
        m.insert("active".into(), Value::Bool(true));
        Value::Map(m)
    }

    fn nested_map() -> Value {
        let mut address = IndexMap::new();
        address.insert("street".into(), Value::String("123 Main".into()));
        address.insert("city".into(), Value::String("Springfield".into()));

        let mut user = IndexMap::new();
        user.insert("name".into(), Value::String("Bob".into()));
        user.insert("age".into(), Value::Int(25));
        user.insert("address".into(), Value::Map(address));
        Value::Map(user)
    }

    // -----------------------------------------------------------------------
    // rename
    // -----------------------------------------------------------------------

    #[test]
    fn rename_simple_field() {
        let result = run("rename .name -> .username", &simple_map());
        assert_eq!(
            result.get_path(".username"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(result.get_path(".name"), None);
    }

    #[test]
    fn rename_preserves_other_fields() {
        let result = run("rename .name -> .username", &simple_map());
        assert_eq!(result.get_path(".age"), Some(&Value::Int(30)));
        assert_eq!(result.get_path(".score"), Some(&Value::Float(95.5)));
    }

    #[test]
    fn rename_nested_path() {
        let result = run("rename .address.street -> .address.road", &nested_map());
        assert_eq!(
            result.get_path(".address.road"),
            Some(&Value::String("123 Main".into()))
        );
        assert_eq!(result.get_path(".address.street"), None);
    }

    #[test]
    fn rename_nonexistent_noop() {
        let input = simple_map();
        let result = run("rename .nonexistent -> .other", &input);
        assert_eq!(result, input);
    }

    // -----------------------------------------------------------------------
    // select
    // -----------------------------------------------------------------------

    #[test]
    fn select_single_field() {
        let result = run("select .name", &simple_map());
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
        // Other fields should be gone
        assert_eq!(result.get_path(".age"), None);
    }

    #[test]
    fn select_multiple_fields() {
        let result = run("select .name, .age", &simple_map());
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(result.get_path(".age"), Some(&Value::Int(30)));
        assert_eq!(result.get_path(".score"), None);
    }

    #[test]
    fn select_nonexistent_field() {
        let result = run("select .nonexistent", &simple_map());
        assert_eq!(result, Value::Map(IndexMap::new()));
    }

    // -----------------------------------------------------------------------
    // drop
    // -----------------------------------------------------------------------

    #[test]
    fn drop_single_field() {
        let result = run("drop .age", &simple_map());
        assert_eq!(result.get_path(".age"), None);
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
    }

    #[test]
    fn drop_multiple_fields() {
        let result = run("drop .age, .score", &simple_map());
        assert_eq!(result.get_path(".age"), None);
        assert_eq!(result.get_path(".score"), None);
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
    }

    #[test]
    fn drop_nested_field() {
        let result = run("drop .address.street", &nested_map());
        assert_eq!(result.get_path(".address.street"), None);
        assert_eq!(
            result.get_path(".address.city"),
            Some(&Value::String("Springfield".into()))
        );
    }

    #[test]
    fn drop_nonexistent_noop() {
        let input = simple_map();
        let result = run("drop .nonexistent", &input);
        assert_eq!(result, input);
    }

    // -----------------------------------------------------------------------
    // set with literal
    // -----------------------------------------------------------------------

    #[test]
    fn set_int_literal() {
        let result = run("set .x = 42", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Int(42)));
    }

    #[test]
    fn set_string_literal() {
        let result = run("set .greeting = \"hello\"", &simple_map());
        assert_eq!(
            result.get_path(".greeting"),
            Some(&Value::String("hello".into()))
        );
    }

    #[test]
    fn set_bool_literal() {
        let result = run("set .flag = false", &simple_map());
        assert_eq!(result.get_path(".flag"), Some(&Value::Bool(false)));
    }

    #[test]
    fn set_null_literal() {
        let result = run("set .x = null", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Null));
    }

    #[test]
    fn set_overwrites_existing() {
        let result = run("set .age = 99", &simple_map());
        assert_eq!(result.get_path(".age"), Some(&Value::Int(99)));
    }

    // -----------------------------------------------------------------------
    // set with path reference
    // -----------------------------------------------------------------------

    #[test]
    fn set_from_another_field() {
        let result = run("set .name_copy = .name", &simple_map());
        assert_eq!(
            result.get_path(".name_copy"),
            Some(&Value::String("Alice".into()))
        );
    }

    #[test]
    fn set_from_nested_field() {
        let result = run("set .city = .address.city", &nested_map());
        assert_eq!(
            result.get_path(".city"),
            Some(&Value::String("Springfield".into()))
        );
    }

    #[test]
    fn set_from_nonexistent_gets_null() {
        let result = run("set .x = .nonexistent", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Null));
    }

    // -----------------------------------------------------------------------
    // set with expression
    // -----------------------------------------------------------------------

    #[test]
    fn set_addition() {
        let result = run("set .double_age = .age + .age", &simple_map());
        assert_eq!(result.get_path(".double_age"), Some(&Value::Int(60)));
    }

    #[test]
    fn set_string_concat() {
        let result = run("set .greeting = .name + \" rocks\"", &simple_map());
        assert_eq!(
            result.get_path(".greeting"),
            Some(&Value::String("Alice rocks".into()))
        );
    }

    #[test]
    fn set_arithmetic_chain() {
        let result = run("set .calc = .age * 2 + 10", &simple_map());
        assert_eq!(result.get_path(".calc"), Some(&Value::Int(70)));
    }

    #[test]
    fn set_comparison() {
        let result = run("set .is_old = .age > 25", &simple_map());
        assert_eq!(result.get_path(".is_old"), Some(&Value::Bool(true)));
    }

    #[test]
    fn set_not() {
        let result = run("set .inactive = not .active", &simple_map());
        assert_eq!(result.get_path(".inactive"), Some(&Value::Bool(false)));
    }

    // -----------------------------------------------------------------------
    // set with function call
    // -----------------------------------------------------------------------

    #[test]
    fn set_lower() {
        let mut m = IndexMap::new();
        m.insert("name".into(), Value::String("ALICE".into()));
        let val = Value::Map(m);
        let result = run("set .name = lower(.name)", &val);
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("alice".into()))
        );
    }

    #[test]
    fn set_upper() {
        let result = run("set .name = upper(.name)", &simple_map());
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("ALICE".into()))
        );
    }

    #[test]
    fn set_trim() {
        let mut m = IndexMap::new();
        m.insert("name".into(), Value::String("  Alice  ".into()));
        let val = Value::Map(m);
        let result = run("set .name = trim(.name)", &val);
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
    }

    #[test]
    fn set_len() {
        let result = run("set .name_len = len(.name)", &simple_map());
        assert_eq!(result.get_path(".name_len"), Some(&Value::Int(5)));
    }

    #[test]
    fn set_replace() {
        let result = run(
            "set .name = replace(.name, \"Alice\", \"Bob\")",
            &simple_map(),
        );
        assert_eq!(result.get_path(".name"), Some(&Value::String("Bob".into())));
    }

    #[test]
    fn set_nested_function() {
        let mut m = IndexMap::new();
        m.insert("name".into(), Value::String("  ALICE  ".into()));
        let val = Value::Map(m);
        let result = run("set .name = lower(trim(.name))", &val);
        assert_eq!(
            result.get_path(".name"),
            Some(&Value::String("alice".into()))
        );
    }

    // -----------------------------------------------------------------------
    // default
    // -----------------------------------------------------------------------

    #[test]
    fn default_sets_missing() {
        let result = run("default .x = 42", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Int(42)));
    }

    #[test]
    fn default_skips_existing() {
        let result = run("default .age = 99", &simple_map());
        assert_eq!(result.get_path(".age"), Some(&Value::Int(30)));
    }

    #[test]
    fn default_sets_null() {
        let mut m = IndexMap::new();
        m.insert("x".into(), Value::Null);
        let val = Value::Map(m);
        let result = run("default .x = 42", &val);
        assert_eq!(result.get_path(".x"), Some(&Value::Int(42)));
    }

    // -----------------------------------------------------------------------
    // cast
    // -----------------------------------------------------------------------

    #[test]
    fn cast_string_to_int() {
        let mut m = IndexMap::new();
        m.insert("age".into(), Value::String("30".into()));
        let val = Value::Map(m);
        let result = run("cast .age as int", &val);
        assert_eq!(result.get_path(".age"), Some(&Value::Int(30)));
    }

    #[test]
    fn cast_int_to_string() {
        let result = run("cast .age as string", &simple_map());
        assert_eq!(result.get_path(".age"), Some(&Value::String("30".into())));
    }

    #[test]
    fn cast_int_to_float() {
        let result = run("cast .age as float", &simple_map());
        assert_eq!(result.get_path(".age"), Some(&Value::Float(30.0)));
    }

    #[test]
    fn cast_float_to_int() {
        let result = run("cast .score as int", &simple_map());
        assert_eq!(result.get_path(".score"), Some(&Value::Int(95)));
    }

    #[test]
    fn cast_bool_to_int() {
        let result = run("cast .active as int", &simple_map());
        assert_eq!(result.get_path(".active"), Some(&Value::Int(1)));
    }

    #[test]
    fn cast_int_to_bool() {
        let result = run("cast .age as bool", &simple_map());
        assert_eq!(result.get_path(".age"), Some(&Value::Bool(true)));
    }

    #[test]
    fn cast_zero_to_bool() {
        let mut m = IndexMap::new();
        m.insert("x".into(), Value::Int(0));
        let val = Value::Map(m);
        let result = run("cast .x as bool", &val);
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(false)));
    }

    #[test]
    fn cast_string_to_bool_true() {
        let mut m = IndexMap::new();
        m.insert("x".into(), Value::String("true".into()));
        let val = Value::Map(m);
        let result = run("cast .x as bool", &val);
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(true)));
    }

    #[test]
    fn cast_string_to_bool_false() {
        let mut m = IndexMap::new();
        m.insert("x".into(), Value::String("false".into()));
        let val = Value::Map(m);
        let result = run("cast .x as bool", &val);
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(false)));
    }

    #[test]
    fn cast_null_to_int() {
        let mut m = IndexMap::new();
        m.insert("x".into(), Value::Null);
        let val = Value::Map(m);
        let result = run("cast .x as int", &val);
        assert_eq!(result.get_path(".x"), Some(&Value::Int(0)));
    }

    #[test]
    fn cast_nonexistent_noop() {
        let input = simple_map();
        let result = run("cast .nonexistent as int", &input);
        assert_eq!(result, input);
    }

    #[test]
    fn cast_invalid_string_to_int_error() {
        let mut m = IndexMap::new();
        m.insert("x".into(), Value::String("not_a_number".into()));
        let val = Value::Map(m);
        let err = run_err("cast .x as int", &val);
        assert!(matches!(err, error::MorphError::Mapping { .. }));
    }

    // -----------------------------------------------------------------------
    // Multi-statement
    // -----------------------------------------------------------------------

    #[test]
    fn multi_rename_and_select() {
        let result = run(
            "rename .name -> .username\nselect .username, .age",
            &simple_map(),
        );
        assert_eq!(
            result.get_path(".username"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(result.get_path(".age"), Some(&Value::Int(30)));
        assert_eq!(result.get_path(".score"), None);
    }

    #[test]
    fn multi_set_and_drop() {
        let result = run(
            "set .full_name = .name\ndrop .name\nset .processed = true",
            &simple_map(),
        );
        assert_eq!(
            result.get_path(".full_name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(result.get_path(".name"), None);
        assert_eq!(result.get_path(".processed"), Some(&Value::Bool(true)));
    }

    #[test]
    fn complex_pipeline() {
        let result = run(
            "\
rename .name -> .full_name
set .age_str = .age
cast .age_str as string
default .country = \"US\"
drop .active
select .full_name, .age, .age_str, .country",
            &simple_map(),
        );
        assert_eq!(
            result.get_path(".full_name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(result.get_path(".age"), Some(&Value::Int(30)));
        assert_eq!(
            result.get_path(".age_str"),
            Some(&Value::String("30".into()))
        );
        assert_eq!(
            result.get_path(".country"),
            Some(&Value::String("US".into()))
        );
        assert_eq!(result.get_path(".active"), None);
    }

    // -----------------------------------------------------------------------
    // Expression evaluation edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn division_by_zero_error() {
        let err = run_err("set .x = .age / 0", &simple_map());
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(message.contains("division by zero"), "msg: {message}");
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    #[test]
    fn modulo_by_zero_error() {
        let err = run_err("set .x = .age % 0", &simple_map());
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(message.contains("modulo by zero"), "msg: {message}");
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    #[test]
    fn mixed_int_float_arithmetic() {
        let result = run("set .x = .age + .score", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Float(125.5)));
    }

    #[test]
    fn comparison_equality() {
        let result = run("set .same = .age == 30", &simple_map());
        assert_eq!(result.get_path(".same"), Some(&Value::Bool(true)));
    }

    #[test]
    fn comparison_not_equal() {
        let result = run("set .diff = .age != 30", &simple_map());
        assert_eq!(result.get_path(".diff"), Some(&Value::Bool(false)));
    }

    #[test]
    fn logical_and() {
        let result = run("set .x = .active and .age > 20", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(true)));
    }

    #[test]
    fn logical_or() {
        let result = run("set .x = .age > 50 or .active", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(true)));
    }

    // -----------------------------------------------------------------------
    // Truthiness
    // -----------------------------------------------------------------------

    #[test]
    fn truthy_string() {
        let result = run("set .x = not \"\"", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(true)));
    }

    #[test]
    fn truthy_nonempty_string() {
        let result = run("set .x = not \"hello\"", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(false)));
    }

    #[test]
    fn truthy_null() {
        let result = run("set .x = not null", &simple_map());
        assert_eq!(result.get_path(".x"), Some(&Value::Bool(true)));
    }

    // -----------------------------------------------------------------------
    // Empty / null input
    // -----------------------------------------------------------------------

    #[test]
    fn eval_on_null() {
        let result = run("set .x = 42", &Value::Null);
        assert_eq!(result.get_path(".x"), Some(&Value::Int(42)));
    }

    #[test]
    fn eval_on_empty_map() {
        let result = run("set .x = 42", &Value::Map(IndexMap::new()));
        assert_eq!(result.get_path(".x"), Some(&Value::Int(42)));
    }

    // -----------------------------------------------------------------------
    // Array index paths
    // -----------------------------------------------------------------------

    #[test]
    fn set_array_index() {
        let val = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let mut m = IndexMap::new();
        m.insert("items".into(), val);
        let input = Value::Map(m);
        let result = run("set .items.[1] = 99", &input);
        assert_eq!(result.get_path(".items.[1]"), Some(&Value::Int(99)));
    }

    #[test]
    fn select_nested_with_path() {
        let result = run("select .address.city", &nested_map());
        assert_eq!(
            result.get_path(".city"),
            Some(&Value::String("Springfield".into()))
        );
    }
}
