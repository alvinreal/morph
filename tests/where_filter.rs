//! Integration tests for issue #21: where filtering.

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

fn person(name: &str, age: i64, active: bool) -> Value {
    make_map(&[
        ("name", Value::String(name.into())),
        ("age", Value::Int(age)),
        ("active", Value::Bool(active)),
    ])
}

fn people() -> Value {
    Value::Array(vec![
        person("Alice", 25, true),
        person("Bob", 17, true),
        person("Charlie", 30, false),
        person("Diana", 15, true),
    ])
}

// ---------------------------------------------------------------------------
// where .age > 18
// ---------------------------------------------------------------------------

#[test]
fn where_age_greater_than() {
    let result = run("where .age > 18", &people());
    match &result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("Alice".into()))
            );
            assert_eq!(
                arr[1].get_path(".name"),
                Some(&Value::String("Charlie".into()))
            );
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// where .status == "active"
// ---------------------------------------------------------------------------

#[test]
fn where_string_comparison() {
    let data = Value::Array(vec![
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("status", Value::String("active".into())),
        ]),
        make_map(&[
            ("name", Value::String("Bob".into())),
            ("status", Value::String("inactive".into())),
        ]),
        make_map(&[
            ("name", Value::String("Charlie".into())),
            ("status", Value::String("active".into())),
        ]),
    ]);

    let result = run(r#"where .status == "active""#, &data);
    match &result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("Alice".into()))
            );
            assert_eq!(
                arr[1].get_path(".name"),
                Some(&Value::String("Charlie".into()))
            );
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// where .name != null
// ---------------------------------------------------------------------------

#[test]
fn where_null_check() {
    let data = Value::Array(vec![
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("age", Value::Int(25)),
        ]),
        make_map(&[("name", Value::Null), ("age", Value::Int(30))]),
        make_map(&[("age", Value::Int(20))]),
    ]);

    let result = run("where .name != null", &data);
    match &result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("Alice".into()))
            );
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// where .score >= 50 and .verified == true
// ---------------------------------------------------------------------------

#[test]
fn where_compound_and_condition() {
    let data = Value::Array(vec![
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("score", Value::Int(80)),
            ("verified", Value::Bool(true)),
        ]),
        make_map(&[
            ("name", Value::String("Bob".into())),
            ("score", Value::Int(40)),
            ("verified", Value::Bool(true)),
        ]),
        make_map(&[
            ("name", Value::String("Charlie".into())),
            ("score", Value::Int(90)),
            ("verified", Value::Bool(false)),
        ]),
        make_map(&[
            ("name", Value::String("Diana".into())),
            ("score", Value::Int(60)),
            ("verified", Value::Bool(true)),
        ]),
    ]);

    let result = run("where .score >= 50 and .verified == true", &data);
    match &result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("Alice".into()))
            );
            assert_eq!(
                arr[1].get_path(".name"),
                Some(&Value::String("Diana".into()))
            );
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// where .role == "admin" or .role == "super"
// ---------------------------------------------------------------------------

#[test]
fn where_or_condition() {
    let data = Value::Array(vec![
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("role", Value::String("admin".into())),
        ]),
        make_map(&[
            ("name", Value::String("Bob".into())),
            ("role", Value::String("user".into())),
        ]),
        make_map(&[
            ("name", Value::String("Charlie".into())),
            ("role", Value::String("super".into())),
        ]),
    ]);

    let result = run(r#"where .role == "admin" or .role == "super""#, &data);
    match &result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("Alice".into()))
            );
            assert_eq!(
                arr[1].get_path(".name"),
                Some(&Value::String("Charlie".into()))
            );
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// where not .deleted
// ---------------------------------------------------------------------------

#[test]
fn where_negation() {
    let data = Value::Array(vec![
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("deleted", Value::Bool(false)),
        ]),
        make_map(&[
            ("name", Value::String("Bob".into())),
            ("deleted", Value::Bool(true)),
        ]),
        make_map(&[
            ("name", Value::String("Charlie".into())),
            ("deleted", Value::Bool(false)),
        ]),
    ]);

    let result = run("where not .deleted", &data);
    match &result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("Alice".into()))
            );
            assert_eq!(
                arr[1].get_path(".name"),
                Some(&Value::String("Charlie".into()))
            );
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// where on non-array → applies as boolean gate
// ---------------------------------------------------------------------------

#[test]
fn where_on_non_array_true() {
    let data = make_map(&[
        ("name", Value::String("Alice".into())),
        ("active", Value::Bool(true)),
    ]);

    let result = run("where .active == true", &data);
    assert_eq!(result, data);
}

#[test]
fn where_on_non_array_false() {
    let data = make_map(&[
        ("name", Value::String("Alice".into())),
        ("active", Value::Bool(false)),
    ]);

    let result = run("where .active == true", &data);
    assert_eq!(result, Value::Null);
}

// ---------------------------------------------------------------------------
// Empty result after filter → empty array
// ---------------------------------------------------------------------------

#[test]
fn where_empty_result() {
    let data = Value::Array(vec![person("Alice", 25, true), person("Bob", 17, true)]);

    let result = run("where .age > 100", &data);
    assert_eq!(result, Value::Array(vec![]));
}

// ---------------------------------------------------------------------------
// All pass → unchanged array
// ---------------------------------------------------------------------------

#[test]
fn where_all_pass() {
    let data = Value::Array(vec![person("Alice", 25, true), person("Bob", 30, true)]);

    let result = run("where .age > 10", &data);
    assert_eq!(result, data);
}
