//! Cross-format round-trip integration test suite.
//!
//! Tests converting between every pair of supported formats (JSON, YAML, TOML,
//! CSV) and verifies data equivalence through the Universal Value type.
//!
//! The conversion path is: format_A string → Value → format_B string → Value,
//! then compare the Values.  Edge cases like format-specific limitations are
//! documented and tested.

use indexmap::IndexMap;
use morph::formats::{csv, json, toml, yaml};
use morph::value::Value;

// ===========================================================================
// Test fixtures
// ===========================================================================

/// Flat key-value pairs (compatible with all formats including TOML).
fn fixture_simple() -> Value {
    let mut m = IndexMap::new();
    m.insert("name".into(), Value::String("Alice".into()));
    m.insert("age".into(), Value::Int(30));
    m.insert("active".into(), Value::Bool(true));
    Value::Map(m)
}

/// 3 levels deep nested structure.
fn fixture_nested() -> Value {
    let mut inner = IndexMap::new();
    inner.insert("street".into(), Value::String("123 Main St".into()));
    inner.insert("city".into(), Value::String("Springfield".into()));

    let mut meta = IndexMap::new();
    meta.insert("verified".into(), Value::Bool(true));
    meta.insert("score".into(), Value::Float(9.5));

    let mut user = IndexMap::new();
    user.insert("name".into(), Value::String("Bob".into()));
    user.insert("address".into(), Value::Map(inner));
    user.insert("meta".into(), Value::Map(meta));

    let mut root = IndexMap::new();
    root.insert("user".into(), Value::Map(user));
    Value::Map(root)
}

/// Array of objects (CSV-compatible: flat rows with uniform keys).
fn fixture_array_of_objects() -> Value {
    let mut row1 = IndexMap::new();
    row1.insert("name".into(), Value::String("Alice".into()));
    row1.insert("age".into(), Value::Int(30));
    row1.insert("score".into(), Value::Float(95.5));

    let mut row2 = IndexMap::new();
    row2.insert("name".into(), Value::String("Bob".into()));
    row2.insert("age".into(), Value::Int(25));
    row2.insert("score".into(), Value::Float(87.0));

    Value::Array(vec![Value::Map(row1), Value::Map(row2)])
}

/// All primitive types: string, int, float, bool (no null for TOML compat).
fn fixture_types() -> Value {
    let mut m = IndexMap::new();
    m.insert("string_val".into(), Value::String("hello world".into()));
    m.insert("int_val".into(), Value::Int(42));
    m.insert("neg_int".into(), Value::Int(-7));
    m.insert("float_val".into(), Value::Float(3.125));
    m.insert("bool_true".into(), Value::Bool(true));
    m.insert("bool_false".into(), Value::Bool(false));
    Value::Map(m)
}

/// All types including null and nested (JSON/YAML only, not TOML-safe).
fn fixture_types_with_null() -> Value {
    let mut m = IndexMap::new();
    m.insert("string_val".into(), Value::String("hello".into()));
    m.insert("int_val".into(), Value::Int(42));
    m.insert("float_val".into(), Value::Float(2.5));
    m.insert("bool_val".into(), Value::Bool(true));
    m.insert("null_val".into(), Value::Null);
    m.insert(
        "array_val".into(),
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
    );
    Value::Map(m)
}

/// Non-ASCII / unicode data.
fn fixture_unicode() -> Value {
    let mut m = IndexMap::new();
    m.insert("emoji".into(), Value::String("🦀🔥✨".into()));
    m.insert("accent".into(), Value::String("café résumé".into()));
    m.insert("cjk".into(), Value::String("你好世界".into()));
    m.insert("arabic".into(), Value::String("مرحبا".into()));
    Value::Map(m)
}

/// Large dataset: array of 500 objects for bulk testing.
/// Scores use `i * 1.1 + 0.1` to ensure all values are non-integer floats,
/// which avoids CSV type inference converting "0" back to Int(0).
fn fixture_large() -> Value {
    let rows: Vec<Value> = (0..500)
        .map(|i| {
            let mut m = IndexMap::new();
            m.insert("id".into(), Value::Int(i));
            m.insert("name".into(), Value::String(format!("user_{i}")));
            m.insert("score".into(), Value::Float(i as f64 + 0.5));
            m.insert("active".into(), Value::Bool(i % 2 == 0));
            Value::Map(m)
        })
        .collect();
    Value::Array(rows)
}

// ===========================================================================
// Helper: Value → serialize to format A → parse back → compare
// ===========================================================================

/// Roundtrip: Value → format string → Value.  Returns the parsed-back Value.
fn roundtrip_through<S, P>(value: &Value, serialize: S, parse: P) -> Value
where
    S: Fn(&Value) -> morph::error::Result<String>,
    P: Fn(&str) -> morph::error::Result<Value>,
{
    let s = serialize(value).expect("serialize failed");
    parse(&s).expect("parse failed")
}

/// Convert Value through two formats: Value → A string → Value → B string → Value.
fn convert_a_to_b<S1, P1, S2, P2>(
    value: &Value,
    ser_a: S1,
    parse_a: P1,
    ser_b: S2,
    parse_b: P2,
) -> Value
where
    S1: Fn(&Value) -> morph::error::Result<String>,
    P1: Fn(&str) -> morph::error::Result<Value>,
    S2: Fn(&Value) -> morph::error::Result<String>,
    P2: Fn(&str) -> morph::error::Result<Value>,
{
    // Roundtrip through format A first to get the canonical Value
    let a_str = ser_a(value).expect("serialize to A failed");
    let val_a = parse_a(&a_str).expect("parse A failed");
    // Now serialize to format B and parse back
    let b_str = ser_b(&val_a).expect("serialize to B failed");
    parse_b(&b_str).expect("parse B failed")
}

// ===========================================================================
// JSON ↔ YAML
// ===========================================================================

#[test]
fn json_to_yaml_to_json_simple() {
    let original = fixture_simple();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        yaml::to_string,
        yaml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

#[test]
fn yaml_to_json_to_yaml_simple() {
    let original = fixture_simple();
    let result = convert_a_to_b(
        &original,
        yaml::to_string,
        yaml::from_str,
        json::to_string,
        json::from_str,
    );
    let back = roundtrip_through(&result, yaml::to_string, yaml::from_str);
    assert_eq!(original, back);
}

#[test]
fn json_to_yaml_to_json_nested() {
    let original = fixture_nested();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        yaml::to_string,
        yaml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

#[test]
fn json_to_yaml_to_json_types_with_null() {
    let original = fixture_types_with_null();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        yaml::to_string,
        yaml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

#[test]
fn json_to_yaml_to_json_unicode() {
    let original = fixture_unicode();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        yaml::to_string,
        yaml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

#[test]
fn json_to_yaml_to_json_large() {
    let original = fixture_large();
    let yaml_str = yaml::to_string(&original).unwrap();
    let yaml_val = yaml::from_str(&yaml_str).unwrap();
    let json_str = json::to_string(&yaml_val).unwrap();
    let final_val = json::from_str(&json_str).unwrap();
    assert_eq!(original, final_val);
}

// ===========================================================================
// JSON ↔ TOML
// ===========================================================================

#[test]
fn json_to_toml_to_json_simple() {
    let original = fixture_simple();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

#[test]
fn toml_to_json_to_toml_simple() {
    let original = fixture_simple();
    let result = convert_a_to_b(
        &original,
        toml::to_string,
        toml::from_str,
        json::to_string,
        json::from_str,
    );
    let back = roundtrip_through(&result, toml::to_string, toml::from_str);
    assert_eq!(original, back);
}

#[test]
fn json_to_toml_to_json_nested() {
    let original = fixture_nested();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

#[test]
fn json_to_toml_to_json_types() {
    let original = fixture_types();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

#[test]
fn json_to_toml_to_json_unicode() {
    let original = fixture_unicode();
    let result = convert_a_to_b(
        &original,
        json::to_string,
        json::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, json::to_string, json::from_str);
    assert_eq!(original, back);
}

// ===========================================================================
// JSON ↔ CSV
// ===========================================================================

#[test]
fn json_to_csv_to_json_array() {
    let original = fixture_array_of_objects();
    let csv_str = csv::to_string(&original).unwrap();
    let csv_val = csv::from_str(&csv_str).unwrap();
    let json_str = json::to_string(&csv_val).unwrap();
    let final_val = json::from_str(&json_str).unwrap();
    assert_eq!(original, final_val);
}

#[test]
fn csv_to_json_to_csv() {
    let csv_input = "name,age,score\nAlice,30,95.5\nBob,25,87\n";
    let csv_val = csv::from_str(csv_input).unwrap();
    let json_str = json::to_string(&csv_val).unwrap();
    let json_val = json::from_str(&json_str).unwrap();
    let csv_output = csv::to_string(&json_val).unwrap();
    let csv_val2 = csv::from_str(&csv_output).unwrap();
    assert_eq!(csv_val, csv_val2);
}

#[test]
fn json_to_csv_to_json_large() {
    let original = fixture_large();
    let csv_str = csv::to_string(&original).unwrap();
    let csv_val = csv::from_str(&csv_str).unwrap();
    let json_str = json::to_string(&csv_val).unwrap();
    let final_val = json::from_str(&json_str).unwrap();
    assert_eq!(original, final_val);
}

// ===========================================================================
// YAML ↔ TOML
// ===========================================================================

#[test]
fn yaml_to_toml_to_yaml_simple() {
    let original = fixture_simple();
    let result = convert_a_to_b(
        &original,
        yaml::to_string,
        yaml::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, yaml::to_string, yaml::from_str);
    assert_eq!(original, back);
}

#[test]
fn toml_to_yaml_to_toml_simple() {
    let original = fixture_simple();
    let result = convert_a_to_b(
        &original,
        toml::to_string,
        toml::from_str,
        yaml::to_string,
        yaml::from_str,
    );
    let back = roundtrip_through(&result, toml::to_string, toml::from_str);
    assert_eq!(original, back);
}

#[test]
fn yaml_to_toml_to_yaml_nested() {
    let original = fixture_nested();
    let result = convert_a_to_b(
        &original,
        yaml::to_string,
        yaml::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, yaml::to_string, yaml::from_str);
    assert_eq!(original, back);
}

#[test]
fn yaml_to_toml_to_yaml_types() {
    let original = fixture_types();
    let result = convert_a_to_b(
        &original,
        yaml::to_string,
        yaml::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, yaml::to_string, yaml::from_str);
    assert_eq!(original, back);
}

#[test]
fn yaml_to_toml_to_yaml_unicode() {
    let original = fixture_unicode();
    let result = convert_a_to_b(
        &original,
        yaml::to_string,
        yaml::from_str,
        toml::to_string,
        toml::from_str,
    );
    let back = roundtrip_through(&result, yaml::to_string, yaml::from_str);
    assert_eq!(original, back);
}

// ===========================================================================
// YAML ↔ CSV
// ===========================================================================

#[test]
fn yaml_to_csv_to_yaml() {
    let original = fixture_array_of_objects();
    let csv_str = csv::to_string(&original).unwrap();
    let csv_val = csv::from_str(&csv_str).unwrap();
    let yaml_str = yaml::to_string(&csv_val).unwrap();
    let final_val = yaml::from_str(&yaml_str).unwrap();
    assert_eq!(original, final_val);
}

#[test]
fn csv_to_yaml_to_csv() {
    let csv_input = "name,age,active\nAlice,30,true\nBob,25,false\n";
    let csv_val = csv::from_str(csv_input).unwrap();
    let yaml_str = yaml::to_string(&csv_val).unwrap();
    let yaml_val = yaml::from_str(&yaml_str).unwrap();
    let csv_output = csv::to_string(&yaml_val).unwrap();
    let csv_val2 = csv::from_str(&csv_output).unwrap();
    assert_eq!(csv_val, csv_val2);
}

// ===========================================================================
// TOML ↔ CSV
// ===========================================================================

#[test]
fn toml_array_to_csv_and_back() {
    // TOML with array of tables → extract records → CSV → back
    let toml_input = "\
[[records]]
name = \"Alice\"
age = 30

[[records]]
name = \"Bob\"
age = 25
";
    let toml_val = toml::from_str(toml_input).unwrap();
    let records = toml_val.get_path(".records").unwrap().clone();
    let csv_str = csv::to_string(&records).unwrap();
    let csv_val = csv::from_str(&csv_str).unwrap();
    assert_eq!(records, csv_val);
}

// ===========================================================================
// Three-format chains (A → B → C → A)
// ===========================================================================

#[test]
fn json_yaml_toml_json() {
    let original = fixture_simple();
    // Value → YAML → Value → TOML → Value → JSON → Value
    let yaml_str = yaml::to_string(&original).unwrap();
    let yaml_val = yaml::from_str(&yaml_str).unwrap();
    let toml_str = toml::to_string(&yaml_val).unwrap();
    let toml_val = toml::from_str(&toml_str).unwrap();
    let json_str = json::to_string(&toml_val).unwrap();
    let final_val = json::from_str(&json_str).unwrap();
    assert_eq!(original, final_val);
}

#[test]
fn yaml_toml_json_yaml() {
    let original = fixture_types();
    let toml_str = toml::to_string(&original).unwrap();
    let toml_val = toml::from_str(&toml_str).unwrap();
    let json_str = json::to_string(&toml_val).unwrap();
    let json_val = json::from_str(&json_str).unwrap();
    let yaml_str = yaml::to_string(&json_val).unwrap();
    let final_val = yaml::from_str(&yaml_str).unwrap();
    assert_eq!(original, final_val);
}

#[test]
fn json_csv_yaml_json() {
    let original = fixture_array_of_objects();
    let csv_str = csv::to_string(&original).unwrap();
    let csv_val = csv::from_str(&csv_str).unwrap();
    let yaml_str = yaml::to_string(&csv_val).unwrap();
    let yaml_val = yaml::from_str(&yaml_str).unwrap();
    let json_str = json::to_string(&yaml_val).unwrap();
    let final_val = json::from_str(&json_str).unwrap();
    assert_eq!(original, final_val);
}

#[test]
fn csv_json_toml_csv() {
    // CSV → JSON Value → wrap in TOML table → extract → CSV
    let csv_input = "name,score\nAlice,100\nBob,95\n";
    let csv_val = csv::from_str(csv_input).unwrap();

    let json_str = json::to_string(&csv_val).unwrap();
    let json_val = json::from_str(&json_str).unwrap();

    // Wrap in a table for TOML (TOML needs top-level map)
    let mut wrapper = IndexMap::new();
    wrapper.insert("records".into(), json_val);
    let toml_val = Value::Map(wrapper);
    let toml_str = toml::to_string(&toml_val).unwrap();
    let toml_back = toml::from_str(&toml_str).unwrap();
    let records = toml_back.get_path(".records").unwrap().clone();

    let csv_str = csv::to_string(&records).unwrap();
    let csv_final = csv::from_str(&csv_str).unwrap();
    assert_eq!(csv_val, csv_final);
}

// ===========================================================================
// Format-specific limitations (documented edge cases)
// ===========================================================================

/// TOML cannot have a top-level array.
#[test]
fn toml_rejects_top_level_array() {
    let val = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let result = toml::to_string(&val);
    assert!(result.is_err(), "TOML should reject top-level arrays");
}

/// TOML cannot represent null — it becomes the string "null".
#[test]
fn toml_null_becomes_string() {
    let mut m = IndexMap::new();
    m.insert("val".into(), Value::Null);
    let val = Value::Map(m);
    let toml_str = toml::to_string(&val).unwrap();
    let toml_val = toml::from_str(&toml_str).unwrap();
    assert_eq!(
        toml_val.get_path(".val"),
        Some(&Value::String("null".into()))
    );
}

/// CSV flattens nested objects (they become JSON strings in cells).
#[test]
fn csv_flattens_nested_to_json_strings() {
    let mut row = IndexMap::new();
    row.insert("name".into(), Value::String("Alice".into()));
    let mut nested = IndexMap::new();
    nested.insert("x".into(), Value::Int(1));
    row.insert("data".into(), Value::Map(nested));
    let val = Value::Array(vec![Value::Map(row)]);

    let csv_str = csv::to_string(&val).unwrap();
    assert!(csv_str.contains("name"));
    assert!(csv_str.contains("data"));
}

/// CSV type inference: numbers and bools get inferred on re-parse.
#[test]
fn csv_type_inference_on_roundtrip() {
    let mut row = IndexMap::new();
    row.insert("int_val".into(), Value::Int(42));
    row.insert("float_val".into(), Value::Float(3.125));
    row.insert("bool_val".into(), Value::Bool(true));
    row.insert("str_val".into(), Value::String("hello".into()));
    let original = Value::Array(vec![Value::Map(row)]);

    let csv_str = csv::to_string(&original).unwrap();
    let csv_val = csv::from_str(&csv_str).unwrap();
    assert_eq!(original, csv_val);
}

/// TOML datetime gets converted to string in Universal Value.
#[test]
fn toml_datetime_to_string() {
    let toml_input = "dt = 2024-06-15T10:30:00Z\n";
    let val = toml::from_str(toml_input).unwrap();
    match val.get_path(".dt") {
        Some(Value::String(s)) => {
            assert!(s.contains("2024"));
            assert!(s.contains("06"));
            assert!(s.contains("15"));
        }
        other => panic!("expected datetime as string, got: {other:?}"),
    }
}

// ===========================================================================
// Unicode cross-format
// ===========================================================================

#[test]
fn unicode_json_yaml_toml_roundtrip() {
    let original = fixture_unicode();

    let yaml_str = yaml::to_string(&original).unwrap();
    let yaml_val = yaml::from_str(&yaml_str).unwrap();
    assert_eq!(original, yaml_val);

    let toml_str = toml::to_string(&yaml_val).unwrap();
    let toml_val = toml::from_str(&toml_str).unwrap();
    assert_eq!(original, toml_val);

    let json_str = json::to_string(&toml_val).unwrap();
    let final_val = json::from_str(&json_str).unwrap();
    assert_eq!(original, final_val);
}

// ===========================================================================
// Large dataset cross-format
// ===========================================================================

#[test]
fn large_json_to_csv_roundtrip() {
    let original = fixture_large();
    let csv_str = csv::to_string(&original).unwrap();
    let csv_val = csv::from_str(&csv_str).unwrap();
    assert_eq!(original, csv_val);
}

#[test]
fn large_json_yaml_roundtrip() {
    let original = fixture_large();
    let yaml_str = yaml::to_string(&original).unwrap();
    let yaml_val = yaml::from_str(&yaml_str).unwrap();
    assert_eq!(original, yaml_val);
}

// ===========================================================================
// CLI integration tests (end-to-end)
// ===========================================================================

#[cfg(test)]
#[allow(deprecated)]
mod cli_integration {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn cli_json_to_yaml() {
        let mut input = NamedTempFile::new().unwrap();
        write!(input, r#"{{"name":"Alice","age":30}}"#).unwrap();

        Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-i",
                input.path().to_str().unwrap(),
                "-f",
                "json",
                "-t",
                "yaml",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("name:"))
            .stdout(predicate::str::contains("Alice"));
    }

    #[test]
    fn cli_yaml_to_json() {
        let mut input = NamedTempFile::new().unwrap();
        write!(input, "name: Bob\nage: 25\n").unwrap();

        Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-i",
                input.path().to_str().unwrap(),
                "-f",
                "yaml",
                "-t",
                "json",
                "--compact",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"name\""))
            .stdout(predicate::str::contains("\"Bob\""));
    }

    #[test]
    fn cli_json_to_toml() {
        let mut input = NamedTempFile::new().unwrap();
        write!(input, r#"{{"server":"localhost","port":8080}}"#).unwrap();

        Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-i",
                input.path().to_str().unwrap(),
                "-f",
                "json",
                "-t",
                "toml",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("server"))
            .stdout(predicate::str::contains("localhost"));
    }

    #[test]
    fn cli_csv_to_json() {
        let mut input = NamedTempFile::new().unwrap();
        write!(input, "name,age\nAlice,30\nBob,25\n").unwrap();

        Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-i",
                input.path().to_str().unwrap(),
                "-f",
                "csv",
                "-t",
                "json",
                "--compact",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("Alice"))
            .stdout(predicate::str::contains("30"));
    }

    #[test]
    fn cli_json_to_csv() {
        let mut input = NamedTempFile::new().unwrap();
        write!(
            input,
            r#"[{{"name":"Alice","age":30}},{{"name":"Bob","age":25}}]"#
        )
        .unwrap();

        Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-i",
                input.path().to_str().unwrap(),
                "-f",
                "json",
                "-t",
                "csv",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("name"))
            .stdout(predicate::str::contains("Alice"));
    }

    #[test]
    fn cli_formats_list() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["--formats"])
            .assert()
            .success()
            .stdout(predicate::str::contains("JSON"))
            .stdout(predicate::str::contains("YAML"))
            .stdout(predicate::str::contains("TOML"))
            .stdout(predicate::str::contains("CSV"));
    }

    #[test]
    fn cli_file_output() {
        let mut input = NamedTempFile::new().unwrap();
        write!(input, r#"{{"key":"value"}}"#).unwrap();

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_str().unwrap().to_string();

        Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-i",
                input.path().to_str().unwrap(),
                "-f",
                "json",
                "-o",
                &output_path,
                "-t",
                "yaml",
            ])
            .assert()
            .success();

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("key"));
        assert!(content.contains("value"));
    }

    #[test]
    fn cli_stdin_to_stdout() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "json", "-t", "yaml"])
            .write_stdin(r#"{"hello":"world"}"#)
            .assert()
            .success()
            .stdout(predicate::str::contains("hello"))
            .stdout(predicate::str::contains("world"));
    }

    #[test]
    fn cli_unknown_format_error() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "protobuf", "-t", "json"])
            .write_stdin("{}")
            .assert()
            .failure();
    }

    #[test]
    fn cli_pretty_flag() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "json", "-t", "json", "--pretty"])
            .write_stdin(r#"{"a":1,"b":2}"#)
            .assert()
            .success()
            .stdout(predicate::str::contains("\n"));
    }

    #[test]
    fn cli_compact_flag() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "json", "-t", "json", "--compact"])
            .write_stdin(r#"{"a":1,"b":2}"#)
            .assert()
            .success()
            .stdout(predicate::str::contains(r#"{"a":1,"b":2}"#));
    }

    // -- Format-specific CLI options -----------------------------------------

    #[test]
    fn cli_csv_delimiter_tab() {
        let tsv = "name\tage\nAlice\t30\nBob\t25\n";
        Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "csv", "-t", "json", "--csv-delimiter", "\\t"])
            .write_stdin(tsv)
            .assert()
            .success()
            .stdout(predicate::str::contains("Alice"))
            .stdout(predicate::str::contains("30"));
    }

    #[test]
    fn cli_csv_no_header() {
        let csv_data = "Alice,30\nBob,25\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "csv", "-t", "json", "--csv-no-header", "--compact"])
            .write_stdin(csv_data)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should return arrays of arrays (no header keys)
        assert!(stdout.contains("["), "expected array of arrays: {stdout}");
        assert!(stdout.contains("\"Alice\""), "expected Alice: {stdout}");
    }

    #[test]
    fn cli_csv_header_override() {
        let csv_data = "old_a,old_b\nAlice,30\nBob,25\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-f",
                "csv",
                "-t",
                "json",
                "--csv-header",
                "name,age",
                "--compact",
            ])
            .write_stdin(csv_data)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should use the overridden headers
        assert!(stdout.contains("\"name\""), "expected 'name' key: {stdout}");
        assert!(stdout.contains("\"age\""), "expected 'age' key: {stdout}");
        assert!(
            !stdout.contains("\"old_a\""),
            "should not contain old header: {stdout}"
        );
    }

    #[test]
    fn cli_xml_root() {
        let json_data = r#"{"name":"Alice"}"#;
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "json", "-t", "xml", "--xml-root", "items"])
            .write_stdin(json_data)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("<items>"), "expected <items>: {stdout}");
        assert!(stdout.contains("</items>"), "expected </items>: {stdout}");
    }

    #[test]
    fn cli_xml_attr_prefix() {
        let xml_data = r#"<root><user id="1">Alice</user></root>"#;
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-f",
                "xml",
                "-t",
                "json",
                "--xml-attr-prefix",
                "_",
                "--compact",
            ])
            .write_stdin(xml_data)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"_id\""), "expected '_id' key: {stdout}");
        assert!(
            !stdout.contains("\"@id\""),
            "should not contain '@id': {stdout}"
        );
    }

    #[test]
    fn cli_yaml_multi() {
        let yaml_data = "---\nname: Alice\n---\nname: Bob\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "yaml", "-t", "json", "--yaml-multi", "--compact"])
            .write_stdin(yaml_data)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        // With --yaml-multi, even a single doc returns an array
        assert!(stdout.contains("["), "expected array: {stdout}");
        assert!(stdout.contains("Alice"), "expected Alice: {stdout}");
        assert!(stdout.contains("Bob"), "expected Bob: {stdout}");
    }

    #[test]
    fn cli_yaml_multi_single_doc() {
        let yaml_data = "name: Alice\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "yaml", "-t", "json", "--yaml-multi", "--compact"])
            .write_stdin(yaml_data)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        // --yaml-multi forces array even with single doc
        assert!(stdout.starts_with("["), "expected array wrapping: {stdout}");
    }

    #[test]
    fn cli_format_options_ignored_when_unused() {
        // XML options should not cause errors when converting JSON → JSON
        Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-f",
                "json",
                "-t",
                "json",
                "--xml-root",
                "items",
                "--csv-delimiter",
                "\\t",
                "--yaml-multi",
            ])
            .write_stdin(r#"{"a":1}"#)
            .assert()
            .success();
    }

    // -- Streaming CLI tests ------------------------------------------------

    #[test]
    fn cli_stream_jsonl_to_json() {
        let input = "{\"name\":\"Alice\",\"age\":30}\n{\"name\":\"Bob\",\"age\":25}\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "jsonl", "-t", "json", "--stream"])
            .write_stdin(input)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(parsed.is_array(), "expected JSON array: {stdout}");
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn cli_stream_csv_to_jsonl() {
        let input = "name,age\nAlice,30\nBob,25\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "csv", "-t", "jsonl", "--stream"])
            .write_stdin(input)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.trim().lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 JSONL lines: {stdout}");
        assert!(stdout.contains("\"Alice\""), "expected Alice: {stdout}");
    }

    #[test]
    fn cli_stream_json_to_jsonl() {
        let input = r#"[{"a":1},{"a":2},{"a":3}]"#;
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "json", "-t", "jsonl", "--stream"])
            .write_stdin(input)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.trim().lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 JSONL lines: {stdout}");
    }

    #[test]
    fn cli_stream_jsonl_to_csv() {
        let input = "{\"name\":\"Alice\",\"age\":30}\n{\"name\":\"Bob\",\"age\":25}\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "jsonl", "-t", "csv", "--stream"])
            .write_stdin(input)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("name"), "expected header: {stdout}");
        assert!(stdout.contains("Alice"), "expected Alice: {stdout}");
        assert!(stdout.contains("Bob"), "expected Bob: {stdout}");
    }

    #[test]
    fn cli_stream_with_mapping() {
        let input = "{\"name\":\"Alice\",\"age\":30}\n{\"name\":\"Bob\",\"age\":25}\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-f",
                "jsonl",
                "-t",
                "jsonl",
                "--stream",
                "-e",
                "select .name",
            ])
            .write_stdin(input)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"name\""), "expected name: {stdout}");
        assert!(
            !stdout.contains("\"age\""),
            "age should be dropped: {stdout}"
        );
    }

    #[test]
    fn cli_stream_csv_with_delimiter() {
        let input = "name\tage\nAlice\t30\nBob\t25\n";
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args([
                "-f",
                "csv",
                "-t",
                "jsonl",
                "--stream",
                "--csv-delimiter",
                "\\t",
            ])
            .write_stdin(input)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"Alice\""), "expected Alice: {stdout}");
        assert!(stdout.contains("\"name\""), "expected name key: {stdout}");
    }

    #[test]
    fn cli_stream_flag_ignored_for_unsupported_formats() {
        // YAML→JSON with --stream should still work (falls back to non-streaming)
        Command::cargo_bin("morph")
            .unwrap()
            .args(["-f", "yaml", "-t", "json", "--stream"])
            .write_stdin("name: Alice\nage: 30\n")
            .assert()
            .success();
    }

    // -- Shell completions and help commands ---------------------------------

    #[test]
    fn cli_completions_bash() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["--completions", "bash"])
            .assert()
            .success()
            .stdout(predicate::str::contains("morph"));
    }

    #[test]
    fn cli_completions_zsh() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["--completions", "zsh"])
            .assert()
            .success()
            .stdout(predicate::str::contains("morph"));
    }

    #[test]
    fn cli_completions_fish() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["--completions", "fish"])
            .assert()
            .success()
            .stdout(predicate::str::contains("morph"));
    }

    #[test]
    fn cli_completions_unknown_shell() {
        Command::cargo_bin("morph")
            .unwrap()
            .args(["--completions", "tcsh"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown shell"));
    }

    #[test]
    fn cli_formats_list_with_capabilities() {
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["--formats"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("JSON"), "expected JSON: {stdout}");
        assert!(stdout.contains("YAML"), "expected YAML: {stdout}");
        assert!(stdout.contains("CSV"), "expected CSV: {stdout}");
        assert!(stdout.contains("XML"), "expected XML: {stdout}");
        assert!(
            stdout.contains("MessagePack"),
            "expected MessagePack: {stdout}"
        );
        assert!(
            stdout.contains("read, write"),
            "expected capabilities: {stdout}"
        );
    }

    #[test]
    fn cli_functions_list() {
        let output = Command::cargo_bin("morph")
            .unwrap()
            .args(["--functions"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("lower(value)"), "expected lower: {stdout}");
        assert!(stdout.contains("upper(value)"), "expected upper: {stdout}");
        assert!(stdout.contains("trim("), "expected trim: {stdout}");
        assert!(stdout.contains("to_int("), "expected to_int: {stdout}");
        assert!(stdout.contains("keys("), "expected keys: {stdout}");
        assert!(stdout.contains("if("), "expected if: {stdout}");
        assert!(
            stdout.contains("String:"),
            "expected String category: {stdout}"
        );
        assert!(
            stdout.contains("Collection:"),
            "expected Collection category: {stdout}"
        );
    }
}
