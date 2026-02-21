//! Integration tests for issue #20: flatten and nest operations.

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
// flatten
// ---------------------------------------------------------------------------

#[test]
fn flatten_basic() {
    // flatten .address on {address: {street, city}} → {address_street, address_city}
    let input = make_map(&[(
        "address",
        make_map(&[
            ("street", Value::String("123 Main".into())),
            ("city", Value::String("Springfield".into())),
        ]),
    )]);

    let result = run("flatten .address", &input);

    assert_eq!(
        result.get_path(".address_street"),
        Some(&Value::String("123 Main".into()))
    );
    assert_eq!(
        result.get_path(".address_city"),
        Some(&Value::String("Springfield".into()))
    );
    // Original nested field should be gone
    assert_eq!(result.get_path(".address"), None);
}

#[test]
fn flatten_with_custom_prefix() {
    // flatten .address -> prefix "addr" → {addr_street, addr_city}
    let input = make_map(&[(
        "address",
        make_map(&[
            ("street", Value::String("456 Oak".into())),
            ("city", Value::String("Shelbyville".into())),
        ]),
    )]);

    let result = run(r#"flatten .address -> prefix "addr""#, &input);

    assert_eq!(
        result.get_path(".addr_street"),
        Some(&Value::String("456 Oak".into()))
    );
    assert_eq!(
        result.get_path(".addr_city"),
        Some(&Value::String("Shelbyville".into()))
    );
    assert_eq!(result.get_path(".address"), None);
}

#[test]
fn flatten_non_object_is_noop() {
    // flatten on a non-object field → no-op
    let input = make_map(&[("name", Value::String("Alice".into()))]);
    let result = run("flatten .name", &input);
    assert_eq!(result, input);
}

#[test]
fn flatten_missing_path_is_noop() {
    let input = make_map(&[("name", Value::String("Alice".into()))]);
    let result = run("flatten .nonexistent", &input);
    assert_eq!(result, input);
}

#[test]
fn flatten_deeply_nested_one_level() {
    // flatten on deeply nested only flattens one level
    let inner = make_map(&[("zip", Value::String("12345".into()))]);
    let mid = make_map(&[
        ("street", Value::String("789 Elm".into())),
        ("details", inner),
    ]);
    let input = make_map(&[("address", mid)]);

    let result = run("flatten .address", &input);

    assert_eq!(
        result.get_path(".address_street"),
        Some(&Value::String("789 Elm".into()))
    );
    // The nested "details" object should be flattened as-is (not recursively)
    assert!(result.get_path(".address_details").is_some());
    match result.get_path(".address_details") {
        Some(Value::Map(m)) => {
            assert_eq!(m.get("zip"), Some(&Value::String("12345".into())));
        }
        other => panic!("expected map for address_details, got: {other:?}"),
    }
}

#[test]
fn flatten_preserves_other_fields() {
    let input = make_map(&[
        ("name", Value::String("Alice".into())),
        (
            "address",
            make_map(&[("city", Value::String("NYC".into()))]),
        ),
    ]);

    let result = run("flatten .address", &input);

    assert_eq!(
        result.get_path(".name"),
        Some(&Value::String("Alice".into()))
    );
    assert_eq!(
        result.get_path(".address_city"),
        Some(&Value::String("NYC".into()))
    );
}

// ---------------------------------------------------------------------------
// nest
// ---------------------------------------------------------------------------

#[test]
fn nest_basic() {
    // nest .a_x, .a_y -> .a → {a: {x, y}}
    let input = make_map(&[("a_x", Value::Int(1)), ("a_y", Value::Int(2))]);

    let result = run("nest .a_x, .a_y -> .a", &input);

    assert_eq!(result.get_path(".a.x"), Some(&Value::Int(1)));
    assert_eq!(result.get_path(".a.y"), Some(&Value::Int(2)));
    // Original flat fields should be gone
    assert_eq!(result.get_path(".a_x"), None);
    assert_eq!(result.get_path(".a_y"), None);
}

#[test]
fn nest_no_shared_prefix() {
    // nest with fields that don't share the target prefix → uses full field names as keys
    let input = make_map(&[("foo", Value::Int(10)), ("bar", Value::Int(20))]);

    let result = run("nest .foo, .bar -> .group", &input);

    assert_eq!(result.get_path(".group.foo"), Some(&Value::Int(10)));
    assert_eq!(result.get_path(".group.bar"), Some(&Value::Int(20)));
    assert_eq!(result.get_path(".foo"), None);
    assert_eq!(result.get_path(".bar"), None);
}

#[test]
fn nest_preserves_other_fields() {
    let input = make_map(&[
        ("a_x", Value::Int(1)),
        ("a_y", Value::Int(2)),
        ("keep", Value::Bool(true)),
    ]);

    let result = run("nest .a_x, .a_y -> .a", &input);

    assert_eq!(result.get_path(".a.x"), Some(&Value::Int(1)));
    assert_eq!(result.get_path(".a.y"), Some(&Value::Int(2)));
    assert_eq!(result.get_path(".keep"), Some(&Value::Bool(true)));
}

// ---------------------------------------------------------------------------
// Round-trip: flatten then nest
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_flatten_then_nest() {
    // Start with nested, flatten, then nest back
    let original = make_map(&[(
        "address",
        make_map(&[
            ("street", Value::String("123 Main".into())),
            ("city", Value::String("Springfield".into())),
        ]),
    )]);

    let flattened = run("flatten .address", &original);
    assert_eq!(
        flattened.get_path(".address_street"),
        Some(&Value::String("123 Main".into()))
    );
    assert_eq!(
        flattened.get_path(".address_city"),
        Some(&Value::String("Springfield".into()))
    );

    let restored = run(
        "nest .address_street, .address_city -> .address",
        &flattened,
    );
    assert_eq!(
        restored.get_path(".address.street"),
        Some(&Value::String("123 Main".into()))
    );
    assert_eq!(
        restored.get_path(".address.city"),
        Some(&Value::String("Springfield".into()))
    );
}

// ---------------------------------------------------------------------------
// flatten + select combination
// ---------------------------------------------------------------------------

#[test]
fn flatten_then_select() {
    let input = make_map(&[
        ("name", Value::String("Alice".into())),
        (
            "address",
            make_map(&[
                ("street", Value::String("123 Main".into())),
                ("city", Value::String("Springfield".into())),
            ]),
        ),
    ]);

    let result = run("flatten .address\nselect .name, .address_city", &input);

    assert_eq!(
        result.get_path(".name"),
        Some(&Value::String("Alice".into()))
    );
    assert_eq!(
        result.get_path(".address_city"),
        Some(&Value::String("Springfield".into()))
    );
    assert_eq!(result.get_path(".address_street"), None);
}
