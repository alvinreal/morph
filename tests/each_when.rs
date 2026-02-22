//! Integration tests for issue #23: each and when blocks.

use indexmap::IndexMap;
use morph::mapping::{eval, parser};
use morph::value::Value;

fn run(mapping: &str, input: &Value) -> Value {
    let program = parser::parse_str(mapping).unwrap();
    eval::eval(&program, input).unwrap()
}

fn run_err(mapping: &str, input: &Value) -> morph::error::MorphError {
    let program = parser::parse_str(mapping).unwrap();
    eval::eval(&program, input).unwrap_err()
}

fn make_map(pairs: &[(&str, Value)]) -> Value {
    let mut m = IndexMap::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    Value::Map(m)
}

// ---------------------------------------------------------------------------
// each: applies to every element
// ---------------------------------------------------------------------------

#[test]
fn each_rename_in_elements() {
    let input = make_map(&[(
        "items",
        Value::Array(vec![
            make_map(&[("x", Value::Int(1))]),
            make_map(&[("x", Value::Int(2))]),
            make_map(&[("x", Value::Int(3))]),
        ]),
    )]);

    let result = run("each .items { rename .x -> .y }", &input);

    match result.get_path(".items") {
        Some(Value::Array(arr)) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].get_path(".y"), Some(&Value::Int(1)));
            assert_eq!(arr[1].get_path(".y"), Some(&Value::Int(2)));
            assert_eq!(arr[2].get_path(".y"), Some(&Value::Int(3)));
            // Original key gone
            assert_eq!(arr[0].get_path(".x"), None);
        }
        other => panic!("expected array at .items, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// each: computed fields per element
// ---------------------------------------------------------------------------

#[test]
fn each_computed_fields() {
    let input = make_map(&[(
        "items",
        Value::Array(vec![
            make_map(&[("price", Value::Int(10)), ("qty", Value::Int(3))]),
            make_map(&[("price", Value::Int(5)), ("qty", Value::Int(7))]),
        ]),
    )]);

    let result = run("each .items { set .total = .price * .qty }", &input);

    match result.get_path(".items") {
        Some(Value::Array(arr)) => {
            assert_eq!(arr[0].get_path(".total"), Some(&Value::Int(30)));
            assert_eq!(arr[1].get_path(".total"), Some(&Value::Int(35)));
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// each on non-array → error
// ---------------------------------------------------------------------------

#[test]
fn each_on_non_array_error() {
    let input = make_map(&[("name", Value::String("Alice".into()))]);
    let err = run_err("each .name { drop .x }", &input);
    match err {
        morph::error::MorphError::Mapping { message, .. } => {
            assert!(
                message.contains("array"),
                "expected array error, got: {message}"
            );
        }
        other => panic!("expected mapping error, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// each with multiple operations inside block
// ---------------------------------------------------------------------------

#[test]
fn each_multiple_operations() {
    let input = make_map(&[(
        "items",
        Value::Array(vec![make_map(&[
            ("old_name", Value::String("test".into())),
            ("extra", Value::Int(42)),
        ])]),
    )]);

    let result = run(
        "each .items {\n  rename .old_name -> .name\n  drop .extra\n}",
        &input,
    );

    match result.get_path(".items") {
        Some(Value::Array(arr)) => {
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("test".into()))
            );
            assert_eq!(arr[0].get_path(".old_name"), None);
            assert_eq!(arr[0].get_path(".extra"), None);
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Nested each
// ---------------------------------------------------------------------------

#[test]
fn nested_each() {
    let input = make_map(&[(
        "groups",
        Value::Array(vec![make_map(&[(
            "members",
            Value::Array(vec![
                make_map(&[("old", Value::String("a".into()))]),
                make_map(&[("old", Value::String("b".into()))]),
            ]),
        )])]),
    )]);

    let result = run(
        "each .groups { each .members { rename .old -> .new } }",
        &input,
    );

    match result.get_path(".groups") {
        Some(Value::Array(groups)) => match groups[0].get_path(".members") {
            Some(Value::Array(members)) => {
                assert_eq!(
                    members[0].get_path(".new"),
                    Some(&Value::String("a".into()))
                );
                assert_eq!(
                    members[1].get_path(".new"),
                    Some(&Value::String("b".into()))
                );
                assert_eq!(members[0].get_path(".old"), None);
            }
            other => panic!("expected members array, got: {other:?}"),
        },
        other => panic!("expected groups array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// each on empty array → no-op
// ---------------------------------------------------------------------------

#[test]
fn each_empty_array() {
    let input = make_map(&[("items", Value::Array(vec![]))]);
    let result = run("each .items { set .x = 1 }", &input);
    assert_eq!(result.get_path(".items"), Some(&Value::Array(vec![])));
}

// ---------------------------------------------------------------------------
// when: condition true → block applied
// ---------------------------------------------------------------------------

#[test]
fn when_condition_true() {
    let input = make_map(&[
        ("type", Value::String("admin".into())),
        ("name", Value::String("Alice".into())),
    ]);

    let result = run(r#"when .type == "admin" { set .perms = "all" }"#, &input);

    assert_eq!(
        result.get_path(".perms"),
        Some(&Value::String("all".into()))
    );
}

// ---------------------------------------------------------------------------
// when: condition false → block skipped
// ---------------------------------------------------------------------------

#[test]
fn when_condition_false() {
    let input = make_map(&[
        ("type", Value::String("user".into())),
        ("name", Value::String("Bob".into())),
    ]);

    let result = run(r#"when .type == "admin" { set .perms = "all" }"#, &input);

    assert_eq!(result.get_path(".perms"), None);
    // Original data unchanged
    assert_eq!(result.get_path(".name"), Some(&Value::String("Bob".into())));
}

// ---------------------------------------------------------------------------
// Multiple when blocks → all evaluated independently
// ---------------------------------------------------------------------------

#[test]
fn multiple_when_blocks() {
    let input = make_map(&[("age", Value::Int(25)), ("verified", Value::Bool(true))]);

    let result = run(
        "when .age >= 18 { set .adult = true }\nwhen .verified == true { set .trusted = true }",
        &input,
    );

    assert_eq!(result.get_path(".adult"), Some(&Value::Bool(true)));
    assert_eq!(result.get_path(".trusted"), Some(&Value::Bool(true)));
}

// ---------------------------------------------------------------------------
// Nested when inside each
// ---------------------------------------------------------------------------

#[test]
fn when_inside_each() {
    let input = make_map(&[(
        "users",
        Value::Array(vec![
            make_map(&[
                ("name", Value::String("Alice".into())),
                ("role", Value::String("admin".into())),
            ]),
            make_map(&[
                ("name", Value::String("Bob".into())),
                ("role", Value::String("user".into())),
            ]),
        ]),
    )]);

    let result = run(
        r#"each .users { when .role == "admin" { set .elevated = true } }"#,
        &input,
    );

    match result.get_path(".users") {
        Some(Value::Array(users)) => {
            assert_eq!(users[0].get_path(".elevated"), Some(&Value::Bool(true)));
            assert_eq!(users[1].get_path(".elevated"), None);
        }
        other => panic!("expected array, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// when with each: per-element conditional
// ---------------------------------------------------------------------------

#[test]
fn each_with_when_conditional_per_element() {
    let input = make_map(&[(
        "items",
        Value::Array(vec![
            make_map(&[
                ("name", Value::String("cheap".into())),
                ("price", Value::Int(5)),
            ]),
            make_map(&[
                ("name", Value::String("expensive".into())),
                ("price", Value::Int(100)),
            ]),
        ]),
    )]);

    let result = run(
        r#"each .items { when .price >= 50 { set .premium = true } }"#,
        &input,
    );

    match result.get_path(".items") {
        Some(Value::Array(items)) => {
            assert_eq!(items[0].get_path(".premium"), None);
            assert_eq!(items[1].get_path(".premium"), Some(&Value::Bool(true)));
        }
        other => panic!("expected array, got: {other:?}"),
    }
}
