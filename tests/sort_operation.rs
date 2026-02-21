//! Integration tests for issue #22: sort operation.

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

fn get_names(val: &Value) -> Vec<String> {
    match val {
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v.get_path(".name") {
                Some(Value::String(s)) => s.clone(),
                _ => "?".into(),
            })
            .collect(),
        _ => panic!("expected array"),
    }
}

fn people_data() -> Value {
    Value::Array(vec![
        make_map(&[
            ("name", Value::String("Charlie".into())),
            ("score", Value::Int(70)),
        ]),
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("score", Value::Int(90)),
        ]),
        make_map(&[
            ("name", Value::String("Bob".into())),
            ("score", Value::Int(80)),
        ]),
    ])
}

// ---------------------------------------------------------------------------
// sort .name asc — alphabetical ascending
// ---------------------------------------------------------------------------

#[test]
fn sort_name_asc() {
    let result = run("sort .name asc", &people_data());
    assert_eq!(get_names(&result), vec!["Alice", "Bob", "Charlie"]);
}

// ---------------------------------------------------------------------------
// sort .name desc — descending
// ---------------------------------------------------------------------------

#[test]
fn sort_name_desc() {
    let result = run("sort .name desc", &people_data());
    assert_eq!(get_names(&result), vec!["Charlie", "Bob", "Alice"]);
}

// ---------------------------------------------------------------------------
// sort .score desc, .name asc — multi-key
// ---------------------------------------------------------------------------

#[test]
fn sort_multi_key() {
    let data = Value::Array(vec![
        make_map(&[
            ("name", Value::String("Bob".into())),
            ("score", Value::Int(80)),
        ]),
        make_map(&[
            ("name", Value::String("Alice".into())),
            ("score", Value::Int(90)),
        ]),
        make_map(&[
            ("name", Value::String("Charlie".into())),
            ("score", Value::Int(80)),
        ]),
        make_map(&[
            ("name", Value::String("Diana".into())),
            ("score", Value::Int(90)),
        ]),
    ]);

    let result = run("sort .score desc, .name asc", &data);
    // score desc: 90s first, then 80s. Within same score, name asc.
    assert_eq!(get_names(&result), vec!["Alice", "Diana", "Bob", "Charlie"]);
}

// ---------------------------------------------------------------------------
// Sort integers
// ---------------------------------------------------------------------------

#[test]
fn sort_integers() {
    // Sort objects by an integer field.
    let data = Value::Array(vec![
        make_map(&[("v", Value::Int(30))]),
        make_map(&[("v", Value::Int(10))]),
        make_map(&[("v", Value::Int(20))]),
    ]);
    let result = run("sort .v asc", &data);
    match &result {
        Value::Array(arr) => {
            let vals: Vec<i64> = arr
                .iter()
                .map(|v| match v.get_path(".v") {
                    Some(Value::Int(i)) => *i,
                    _ => panic!("expected int"),
                })
                .collect();
            assert_eq!(vals, vec![10, 20, 30]);
        }
        _ => panic!("expected array"),
    }
}

// ---------------------------------------------------------------------------
// Sort floats
// ---------------------------------------------------------------------------

#[test]
fn sort_floats() {
    let data = Value::Array(vec![
        make_map(&[("v", Value::Float(2.5))]),
        make_map(&[("v", Value::Float(1.1))]),
        make_map(&[("v", Value::Float(3.7))]),
    ]);
    let result = run("sort .v asc", &data);
    match &result {
        Value::Array(arr) => {
            let vals: Vec<f64> = arr
                .iter()
                .map(|v| match v.get_path(".v") {
                    Some(Value::Float(f)) => *f,
                    _ => panic!("expected float"),
                })
                .collect();
            assert_eq!(vals, vec![1.1, 2.5, 3.7]);
        }
        _ => panic!("expected array"),
    }
}

// ---------------------------------------------------------------------------
// Sort strings
// ---------------------------------------------------------------------------

#[test]
fn sort_strings() {
    let data = Value::Array(vec![
        make_map(&[("v", Value::String("banana".into()))]),
        make_map(&[("v", Value::String("apple".into()))]),
        make_map(&[("v", Value::String("cherry".into()))]),
    ]);
    let result = run("sort .v asc", &data);
    match &result {
        Value::Array(arr) => {
            let vals: Vec<&str> = arr
                .iter()
                .map(|v| match v.get_path(".v") {
                    Some(Value::String(s)) => s.as_str(),
                    _ => panic!("expected string"),
                })
                .collect();
            assert_eq!(vals, vec!["apple", "banana", "cherry"]);
        }
        _ => panic!("expected array"),
    }
}

// ---------------------------------------------------------------------------
// Sort with null values (nulls last)
// ---------------------------------------------------------------------------

#[test]
fn sort_nulls_last_asc() {
    let data = Value::Array(vec![
        make_map(&[("name", Value::String("Charlie".into()))]),
        make_map(&[("name", Value::Null)]),
        make_map(&[("name", Value::String("Alice".into()))]),
    ]);
    let result = run("sort .name asc", &data);
    assert_eq!(get_names(&result), vec!["Alice", "Charlie", "?"]);
    // Verify the null is actually last
    match &result {
        Value::Array(arr) => {
            assert_eq!(arr[2].get_path(".name"), Some(&Value::Null));
        }
        _ => panic!("expected array"),
    }
}

#[test]
fn sort_nulls_last_desc() {
    let data = Value::Array(vec![
        make_map(&[("name", Value::Null)]),
        make_map(&[("name", Value::String("Alice".into()))]),
        make_map(&[("name", Value::String("Charlie".into()))]),
    ]);
    let result = run("sort .name desc", &data);
    // desc: Charlie, Alice, then null last
    match &result {
        Value::Array(arr) => {
            assert_eq!(
                arr[0].get_path(".name"),
                Some(&Value::String("Charlie".into()))
            );
            assert_eq!(
                arr[1].get_path(".name"),
                Some(&Value::String("Alice".into()))
            );
            assert_eq!(arr[2].get_path(".name"), Some(&Value::Null));
        }
        _ => panic!("expected array"),
    }
}

// ---------------------------------------------------------------------------
// Sort on non-existent field → stable order (all compare equal)
// ---------------------------------------------------------------------------

#[test]
fn sort_nonexistent_field() {
    let data = people_data();
    let result = run("sort .nonexistent asc", &data);
    // All values are null/missing so order is stable (unchanged)
    assert_eq!(get_names(&result), vec!["Charlie", "Alice", "Bob"]);
}

// ---------------------------------------------------------------------------
// Sort on non-array → no-op
// ---------------------------------------------------------------------------

#[test]
fn sort_non_array_noop() {
    let data = make_map(&[
        ("name", Value::String("Alice".into())),
        ("age", Value::Int(30)),
    ]);
    let result = run("sort .name asc", &data);
    assert_eq!(result, data);
}

// ---------------------------------------------------------------------------
// Default direction is ascending
// ---------------------------------------------------------------------------

#[test]
fn sort_default_direction_asc() {
    let result = run("sort .name", &people_data());
    assert_eq!(get_names(&result), vec!["Alice", "Bob", "Charlie"]);
}
