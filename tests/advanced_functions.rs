//! Integration tests for issue #24: collection functions, string interpolation, if()

use indexmap::IndexMap;
use morph::mapping::{eval, parser};
use morph::value::Value;

fn run(mapping: &str, input: &Value) -> Value {
    let program = parser::parse_str(mapping).unwrap();
    eval::eval(&program, input).unwrap()
}

fn make_map(pairs: &[(&str, Value)]) -> Value {
    let mut m = IndexMap::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    Value::Map(m)
}

// ---------------------------------------------------------------------------
// keys()
// ---------------------------------------------------------------------------

#[test]
fn keys_of_map() {
    let input = make_map(&[(
        "data",
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("age", Value::Int(30)),
        ]),
    )]);
    let result = run("set .k = keys(.data)", &input);
    match result.get_path(".k") {
        Some(Value::Array(arr)) => {
            assert_eq!(arr.len(), 2);
            assert!(arr.contains(&Value::String("name".into())));
            assert!(arr.contains(&Value::String("age".into())));
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// values()
// ---------------------------------------------------------------------------

#[test]
fn values_of_map() {
    let input = make_map(&[(
        "data",
        make_map(&[("x", Value::Int(1)), ("y", Value::Int(2))]),
    )]);
    let result = run("set .v = values(.data)", &input);
    match result.get_path(".v") {
        Some(Value::Array(arr)) => {
            assert!(arr.contains(&Value::Int(1)));
            assert!(arr.contains(&Value::Int(2)));
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// unique()
// ---------------------------------------------------------------------------

#[test]
fn unique_array() {
    let input = make_map(&[(
        "items",
        Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(1),
            Value::Int(3),
            Value::Int(2),
        ]),
    )]);
    let result = run("set .items = unique(.items)", &input);
    assert_eq!(
        result.get_path(".items"),
        Some(&Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ]))
    );
}

// ---------------------------------------------------------------------------
// first(), last()
// ---------------------------------------------------------------------------

#[test]
fn first_and_last() {
    let input = make_map(&[(
        "items",
        Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)]),
    )]);
    let r1 = run("set .f = first(.items)", &input);
    assert_eq!(r1.get_path(".f"), Some(&Value::Int(10)));

    let r2 = run("set .l = last(.items)", &input);
    assert_eq!(r2.get_path(".l"), Some(&Value::Int(30)));
}

#[test]
fn first_and_last_empty() {
    let input = make_map(&[("items", Value::Array(vec![]))]);
    let r1 = run("set .f = first(.items)", &input);
    assert_eq!(r1.get_path(".f"), Some(&Value::Null));
    let r2 = run("set .l = last(.items)", &input);
    assert_eq!(r2.get_path(".l"), Some(&Value::Null));
}

// ---------------------------------------------------------------------------
// sum()
// ---------------------------------------------------------------------------

#[test]
fn sum_integers() {
    let input = make_map(&[(
        "nums",
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
    )]);
    let result = run("set .total = sum(.nums)", &input);
    assert_eq!(result.get_path(".total"), Some(&Value::Int(6)));
}

#[test]
fn sum_floats() {
    let input = make_map(&[(
        "nums",
        Value::Array(vec![Value::Float(1.5), Value::Float(2.5)]),
    )]);
    let result = run("set .total = sum(.nums)", &input);
    assert_eq!(result.get_path(".total"), Some(&Value::Float(4.0)));
}

// ---------------------------------------------------------------------------
// group_by()
// ---------------------------------------------------------------------------

#[test]
fn group_by_field() {
    let input = make_map(&[(
        "items",
        Value::Array(vec![
            make_map(&[
                ("type", Value::String("fruit".into())),
                ("name", Value::String("apple".into())),
            ]),
            make_map(&[
                ("type", Value::String("veg".into())),
                ("name", Value::String("carrot".into())),
            ]),
            make_map(&[
                ("type", Value::String("fruit".into())),
                ("name", Value::String("banana".into())),
            ]),
        ]),
    )]);

    let result = run(r#"set .grouped = group_by(.items, "type")"#, &input);
    match result.get_path(".grouped") {
        Some(Value::Map(m)) => {
            assert!(m.contains_key("fruit"));
            assert!(m.contains_key("veg"));
            match m.get("fruit") {
                Some(Value::Array(arr)) => assert_eq!(arr.len(), 2),
                other => panic!("expected array for fruit group, got: {other:?}"),
            }
            match m.get("veg") {
                Some(Value::Array(arr)) => assert_eq!(arr.len(), 1),
                other => panic!("expected array for veg group, got: {other:?}"),
            }
        }
        other => panic!("expected map, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// if(condition, then, else)
// ---------------------------------------------------------------------------

#[test]
fn if_true_branch() {
    let input = make_map(&[("age", Value::Int(25))]);
    let result = run(r#"set .label = if(.age > 18, "adult", "minor")"#, &input);
    assert_eq!(
        result.get_path(".label"),
        Some(&Value::String("adult".into()))
    );
}

#[test]
fn if_false_branch() {
    let input = make_map(&[("age", Value::Int(15))]);
    let result = run(r#"set .label = if(.age > 18, "adult", "minor")"#, &input);
    assert_eq!(
        result.get_path(".label"),
        Some(&Value::String("minor".into()))
    );
}

// ---------------------------------------------------------------------------
// is_array()
// ---------------------------------------------------------------------------

#[test]
fn is_array_true() {
    let input = make_map(&[("items", Value::Array(vec![Value::Int(1)]))]);
    let result = run("set .check = is_array(.items)", &input);
    assert_eq!(result.get_path(".check"), Some(&Value::Bool(true)));
}

#[test]
fn is_array_false() {
    let input = make_map(&[("name", Value::String("Alice".into()))]);
    let result = run("set .check = is_array(.name)", &input);
    assert_eq!(result.get_path(".check"), Some(&Value::Bool(false)));
}

// ---------------------------------------------------------------------------
// String interpolation: "Hello, {.name}!"
// ---------------------------------------------------------------------------

#[test]
fn string_interpolation_basic() {
    let input = make_map(&[("name", Value::String("Alice".into()))]);
    let result = run(r#"set .greeting = "Hello, {.name}!""#, &input);
    assert_eq!(
        result.get_path(".greeting"),
        Some(&Value::String("Hello, Alice!".into()))
    );
}

#[test]
fn string_interpolation_multiple() {
    let input = make_map(&[
        ("first", Value::String("Jane".into())),
        ("last", Value::String("Doe".into())),
    ]);
    let result = run(r#"set .full = "{.first} {.last}""#, &input);
    assert_eq!(
        result.get_path(".full"),
        Some(&Value::String("Jane Doe".into()))
    );
}

#[test]
fn string_interpolation_with_expression() {
    let input = make_map(&[("price", Value::Int(10)), ("qty", Value::Int(3))]);
    let result = run(r#"set .desc = "Total: {.price * .qty}""#, &input);
    assert_eq!(
        result.get_path(".desc"),
        Some(&Value::String("Total: 30".into()))
    );
}

#[test]
fn string_no_interpolation_plain() {
    let input = make_map(&[]);
    let result = run(r#"set .msg = "no interpolation here""#, &input);
    assert_eq!(
        result.get_path(".msg"),
        Some(&Value::String("no interpolation here".into()))
    );
}

#[test]
fn string_interpolation_with_function() {
    let input = make_map(&[("name", Value::String("ALICE".into()))]);
    let result = run(r#"set .greeting = "Hi, {lower(.name)}!""#, &input);
    assert_eq!(
        result.get_path(".greeting"),
        Some(&Value::String("Hi, alice!".into()))
    );
}
