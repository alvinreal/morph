//! Golden conformance tests for morph's format conversion and mapping outputs.
//!
//! These tests detect **output drift** — any change in how morph parses,
//! serializes, or maps data will cause a test failure. To update golden files
//! after an intentional change, run:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --test golden_conformance
//! ```

use morph::cli::{parse_input, serialize_output, Format};
use morph::mapping::{eval::eval, lexer::tokenize, parser::parse};
use std::path::Path;

/// Compare actual output against a golden file. If `UPDATE_GOLDEN=1` is set,
/// overwrite the golden file instead of comparing.
fn assert_golden(name: &str, actual: &str) {
    let golden_path = format!("tests/golden/expected/{name}.txt");
    let path = Path::new(&golden_path);

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(path, actual).unwrap_or_else(|e| {
            panic!("Failed to write golden file {golden_path}: {e}");
        });
        eprintln!("Updated golden file: {golden_path}");
        return;
    }

    let expected = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "Golden file {golden_path} not found: {e}\n\
             Run `UPDATE_GOLDEN=1 cargo test --test golden_conformance` to generate it."
        );
    });

    // Normalize line endings so tests pass on Windows (CRLF) and Unix (LF)
    let actual_normalized = actual.replace("\r\n", "\n");
    let expected_normalized = expected.replace("\r\n", "\n");

    if actual_normalized != expected_normalized {
        let actual = &actual_normalized;
        let expected = &expected_normalized;
        // Show a helpful diff
        let actual_lines: Vec<&str> = actual.lines().collect();
        let expected_lines: Vec<&str> = expected.lines().collect();
        let max_lines = actual_lines.len().max(expected_lines.len());

        let mut diff = String::new();
        for i in 0..max_lines {
            let a = actual_lines.get(i).unwrap_or(&"<missing>");
            let e = expected_lines.get(i).unwrap_or(&"<missing>");
            if a != e {
                diff.push_str(&format!("  line {i}: expected: {e:?}\n"));
                diff.push_str(&format!("  line {i}:   actual: {a:?}\n"));
            }
        }

        panic!(
            "Golden file mismatch for {name}!\n\
             Golden file: {golden_path}\n\
             Differences:\n{diff}\n\
             Run `UPDATE_GOLDEN=1 cargo test --test golden_conformance` to update."
        );
    }
}

fn convert(input_path: &str, in_fmt: Format, out_fmt: Format, pretty: bool) -> String {
    let input = std::fs::read_to_string(input_path).unwrap();
    let value = parse_input(&input, in_fmt).unwrap();
    serialize_output(&value, out_fmt, pretty).unwrap()
}

fn convert_with_mapping(
    input_path: &str,
    in_fmt: Format,
    mapping: &str,
    out_fmt: Format,
    pretty: bool,
) -> String {
    let input = std::fs::read_to_string(input_path).unwrap();
    let value = parse_input(&input, in_fmt).unwrap();
    let tokens = tokenize(mapping).unwrap();
    let program = parse(tokens).unwrap();
    let mapped = eval(&program, &value).unwrap();
    serialize_output(&mapped, out_fmt, pretty).unwrap()
}

// ============================================================================
// Format conversion golden tests
// ============================================================================

#[test]
fn golden_json_to_yaml() {
    let actual = convert(
        "tests/golden/inputs/users.json",
        Format::Json,
        Format::Yaml,
        false,
    );
    assert_golden("json_to_yaml", &actual);
}

#[test]
fn golden_json_to_csv() {
    let actual = convert(
        "tests/golden/inputs/users.json",
        Format::Json,
        Format::Csv,
        false,
    );
    assert_golden("json_to_csv", &actual);
}

#[test]
fn golden_json_to_json_pretty() {
    let actual = convert(
        "tests/golden/inputs/users.json",
        Format::Json,
        Format::Json,
        true,
    );
    assert_golden("json_to_json_pretty", &actual);
}

#[test]
fn golden_csv_to_json() {
    let actual = convert(
        "tests/golden/inputs/users.csv",
        Format::Csv,
        Format::Json,
        true,
    );
    assert_golden("csv_to_json", &actual);
}

#[test]
fn golden_csv_to_yaml() {
    let actual = convert(
        "tests/golden/inputs/users.csv",
        Format::Csv,
        Format::Yaml,
        false,
    );
    assert_golden("csv_to_yaml", &actual);
}

#[test]
fn golden_yaml_to_json() {
    let actual = convert(
        "tests/golden/inputs/users.yaml",
        Format::Yaml,
        Format::Json,
        true,
    );
    assert_golden("yaml_to_json", &actual);
}

#[test]
fn golden_yaml_to_csv() {
    let actual = convert(
        "tests/golden/inputs/users.yaml",
        Format::Yaml,
        Format::Csv,
        false,
    );
    assert_golden("yaml_to_csv", &actual);
}

// ============================================================================
// Mapping golden tests
// ============================================================================

#[test]
fn golden_json_rename_mapping() {
    let actual = convert_with_mapping(
        "tests/golden/inputs/users.json",
        Format::Json,
        "rename .name -> .username",
        Format::Json,
        true,
    );
    assert_golden("json_rename_mapping", &actual);
}

#[test]
fn golden_json_select_mapping() {
    let actual = convert_with_mapping(
        "tests/golden/inputs/users.json",
        Format::Json,
        "select .id, .name, .email",
        Format::Json,
        true,
    );
    assert_golden("json_select_mapping", &actual);
}

#[test]
fn golden_json_filter_mapping() {
    let actual = convert_with_mapping(
        "tests/golden/inputs/users.json",
        Format::Json,
        "where .active == true",
        Format::Json,
        true,
    );
    assert_golden("json_filter_mapping", &actual);
}

#[test]
fn golden_json_complex_mapping() {
    let actual = convert_with_mapping(
        "tests/golden/inputs/users.json",
        Format::Json,
        "rename .name -> .username\nset .email_domain = \"example.com\"\ndrop .active",
        Format::Json,
        true,
    );
    assert_golden("json_complex_mapping", &actual);
}

// ============================================================================
// Round-trip consistency tests
// ============================================================================

#[test]
fn golden_roundtrip_json_yaml_json() {
    let input = std::fs::read_to_string("tests/golden/inputs/users.json").unwrap();
    let value = parse_input(&input, Format::Json).unwrap();

    // JSON -> YAML -> JSON
    let yaml = serialize_output(&value, Format::Yaml, false).unwrap();
    let from_yaml = parse_input(&yaml, Format::Yaml).unwrap();
    let back_to_json = serialize_output(&from_yaml, Format::Json, true).unwrap();

    let original_json = serialize_output(&value, Format::Json, true).unwrap();
    assert_eq!(
        original_json, back_to_json,
        "Round-trip JSON -> YAML -> JSON produced different output"
    );
}

#[test]
fn golden_roundtrip_json_csv_json() {
    let input = std::fs::read_to_string("tests/golden/inputs/users.json").unwrap();
    let value = parse_input(&input, Format::Json).unwrap();

    // JSON -> CSV -> JSON
    let csv = serialize_output(&value, Format::Csv, false).unwrap();
    let from_csv = parse_input(&csv, Format::Csv).unwrap();
    let back_to_json = serialize_output(&from_csv, Format::Json, true).unwrap();

    let original_json = serialize_output(&value, Format::Json, true).unwrap();
    assert_eq!(
        original_json, back_to_json,
        "Round-trip JSON -> CSV -> JSON produced different output"
    );
}

#[test]
fn golden_roundtrip_yaml_csv_yaml() {
    let input = std::fs::read_to_string("tests/golden/inputs/users.yaml").unwrap();
    let value = parse_input(&input, Format::Yaml).unwrap();

    // YAML -> CSV -> YAML
    let csv = serialize_output(&value, Format::Csv, false).unwrap();
    let from_csv = parse_input(&csv, Format::Csv).unwrap();
    let back_to_yaml = serialize_output(&from_csv, Format::Yaml, false).unwrap();

    let original_yaml = serialize_output(&value, Format::Yaml, false).unwrap();
    assert_eq!(
        original_yaml, back_to_yaml,
        "Round-trip YAML -> CSV -> YAML produced different output"
    );
}
