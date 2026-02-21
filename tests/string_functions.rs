//! Integration tests for issue #18: built-in string functions.
//!
//! These tests exercise the mapping evaluator end-to-end, covering every
//! TDD scenario listed in the issue.

use indexmap::IndexMap;
use morph::mapping::{eval, parser};
use morph::value::Value;

/// Helper: parse a mapping program, apply it to `input`, return the result.
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
// join
// ---------------------------------------------------------------------------

#[test]
fn join_string_args() {
    // join("a", "b", "c") → "abc"
    let input = make_map(&[]);
    let result = run(r#"set .out = join("a", "b", "c")"#, &input);
    assert_eq!(result.get_path(".out"), Some(&Value::String("abc".into())));
}

#[test]
fn join_field_values() {
    // join(.first, " ", .last) with field values
    let input = make_map(&[
        ("first", Value::String("Jane".into())),
        ("last", Value::String("Doe".into())),
    ]);
    let result = run(r#"set .full = join(.first, " ", .last)"#, &input);
    assert_eq!(
        result.get_path(".full"),
        Some(&Value::String("Jane Doe".into()))
    );
}

#[test]
fn join_array_with_separator() {
    // join([...], sep) still works as array join
    let input = make_map(&[(
        "tags",
        Value::Array(vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ]),
    )]);
    let result = run(r#"set .csv = join(.tags, ",")"#, &input);
    assert_eq!(
        result.get_path(".csv"),
        Some(&Value::String("a,b,c".into()))
    );
}

// ---------------------------------------------------------------------------
// split
// ---------------------------------------------------------------------------

#[test]
fn split_basic() {
    // split("a,b,c", ",") → ["a", "b", "c"]
    let input = make_map(&[]);
    let result = run(r#"set .out = split("a,b,c", ",")"#, &input);
    assert_eq!(
        result.get_path(".out"),
        Some(&Value::Array(vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ]))
    );
}

#[test]
fn split_no_match() {
    // split("hello", "x") → ["hello"]
    let input = make_map(&[]);
    let result = run(r#"set .out = split("hello", "x")"#, &input);
    assert_eq!(
        result.get_path(".out"),
        Some(&Value::Array(vec![Value::String("hello".into())]))
    );
}

// ---------------------------------------------------------------------------
// lower
// ---------------------------------------------------------------------------

#[test]
fn lower_basic() {
    // lower("HELLO") → "hello"
    let input = make_map(&[("name", Value::String("HELLO".into()))]);
    let result = run("set .name = lower(.name)", &input);
    assert_eq!(
        result.get_path(".name"),
        Some(&Value::String("hello".into()))
    );
}

#[test]
fn lower_already_lower() {
    // lower("Already lower") → "already lower"
    let input = make_map(&[("s", Value::String("Already lower".into()))]);
    let result = run("set .s = lower(.s)", &input);
    assert_eq!(
        result.get_path(".s"),
        Some(&Value::String("already lower".into()))
    );
}

// ---------------------------------------------------------------------------
// upper
// ---------------------------------------------------------------------------

#[test]
fn upper_basic() {
    // upper("hello") → "HELLO"
    let input = make_map(&[("name", Value::String("hello".into()))]);
    let result = run("set .name = upper(.name)", &input);
    assert_eq!(
        result.get_path(".name"),
        Some(&Value::String("HELLO".into()))
    );
}

// ---------------------------------------------------------------------------
// trim
// ---------------------------------------------------------------------------

#[test]
fn trim_whitespace() {
    // trim("  hello  ") → "hello"
    let input = make_map(&[("s", Value::String("  hello  ".into()))]);
    let result = run("set .s = trim(.s)", &input);
    assert_eq!(result.get_path(".s"), Some(&Value::String("hello".into())));
}

#[test]
fn trim_no_spaces() {
    // trim("no-spaces") → "no-spaces"
    let input = make_map(&[("s", Value::String("no-spaces".into()))]);
    let result = run("set .s = trim(.s)", &input);
    assert_eq!(
        result.get_path(".s"),
        Some(&Value::String("no-spaces".into()))
    );
}

// ---------------------------------------------------------------------------
// replace
// ---------------------------------------------------------------------------

#[test]
fn replace_basic() {
    // replace("hello world", "world", "rust") → "hello rust"
    let input = make_map(&[("s", Value::String("hello world".into()))]);
    let result = run(r#"set .s = replace(.s, "world", "rust")"#, &input);
    assert_eq!(
        result.get_path(".s"),
        Some(&Value::String("hello rust".into()))
    );
}

#[test]
fn replace_all_occurrences() {
    // replace("aaa", "a", "b") → "bbb"
    let input = make_map(&[("s", Value::String("aaa".into()))]);
    let result = run(r#"set .s = replace(.s, "a", "b")"#, &input);
    assert_eq!(result.get_path(".s"), Some(&Value::String("bbb".into())));
}

// ---------------------------------------------------------------------------
// len
// ---------------------------------------------------------------------------

#[test]
fn len_string() {
    // len("hello") → 5
    let input = make_map(&[("s", Value::String("hello".into()))]);
    let result = run("set .out = len(.s)", &input);
    assert_eq!(result.get_path(".out"), Some(&Value::Int(5)));
}

#[test]
fn len_array() {
    // len([1,2,3]) → 3
    let input = make_map(&[(
        "arr",
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
    )]);
    let result = run("set .out = len(.arr)", &input);
    assert_eq!(result.get_path(".out"), Some(&Value::Int(3)));
}

#[test]
fn len_empty_string() {
    // len("") → 0
    let input = make_map(&[("s", Value::String("".into()))]);
    let result = run("set .out = len(.s)", &input);
    assert_eq!(result.get_path(".out"), Some(&Value::Int(0)));
}

// ---------------------------------------------------------------------------
// coalesce
// ---------------------------------------------------------------------------

#[test]
fn coalesce_skips_nulls() {
    // coalesce(null, null, "found") → "found"
    let input = make_map(&[]);
    let result = run(r#"set .out = coalesce(null, null, "found")"#, &input);
    assert_eq!(
        result.get_path(".out"),
        Some(&Value::String("found".into()))
    );
}

#[test]
fn coalesce_first_non_null() {
    // coalesce("first", "second") → "first"
    let input = make_map(&[]);
    let result = run(r#"set .out = coalesce("first", "second")"#, &input);
    assert_eq!(
        result.get_path(".out"),
        Some(&Value::String("first".into()))
    );
}

#[test]
fn coalesce_with_fields() {
    // coalesce(.missing, .name) when .missing is absent
    let input = make_map(&[("name", Value::String("fallback".into()))]);
    let result = run("set .out = coalesce(.missing, .name)", &input);
    assert_eq!(
        result.get_path(".out"),
        Some(&Value::String("fallback".into()))
    );
}

// ---------------------------------------------------------------------------
// Error: wrong arg count
// ---------------------------------------------------------------------------

#[test]
fn error_wrong_arg_count() {
    let input = make_map(&[]);
    let program = parser::parse_str(r#"set .out = lower("a", "b")"#).unwrap();
    let err = eval::eval(&program, &input).unwrap_err();
    match err {
        morph::error::MorphError::Mapping { message, .. } => {
            assert!(
                message.contains("expects"),
                "expected arg-count error, got: {message}"
            );
        }
        other => panic!("expected Mapping error, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Composability: nested function calls through the evaluator
// ---------------------------------------------------------------------------

#[test]
fn nested_lower_trim() {
    // lower(trim("  HELLO  ")) → "hello"
    let input = make_map(&[("s", Value::String("  HELLO  ".into()))]);
    let result = run("set .s = lower(trim(.s))", &input);
    assert_eq!(result.get_path(".s"), Some(&Value::String("hello".into())));
}

#[test]
fn chained_replace_upper() {
    // upper(replace("hello world", "world", "rust")) → "HELLO RUST"
    let input = make_map(&[("s", Value::String("hello world".into()))]);
    let result = run(r#"set .s = upper(replace(.s, "world", "rust"))"#, &input);
    assert_eq!(
        result.get_path(".s"),
        Some(&Value::String("HELLO RUST".into()))
    );
}
