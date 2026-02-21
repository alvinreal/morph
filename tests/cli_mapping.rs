#![allow(deprecated)]
//! Integration tests for issue #19: CLI mapping flags (-m, -e, --dry-run).

use assert_cmd::Command;
use std::io::Write;
use tempfile::NamedTempFile;

fn morph() -> Command {
    Command::cargo_bin("morph").unwrap()
}

/// Create a temp file with the given content and extension.
fn temp_file(content: &str, suffix: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// ---------------------------------------------------------------------------
// -m flag (mapping file)
// ---------------------------------------------------------------------------

#[test]
fn mapping_file_applies_transformations() {
    let input = temp_file(r#"{"old_name": "Alice", "age": 30}"#, ".json");
    let mapping = temp_file("rename .old_name -> .name", ".morph");

    let output = morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-t",
            "json",
            "--compact",
            "-m",
            mapping.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("\"name\""),
        "expected 'name' in output: {stdout}"
    );
    assert!(
        !stdout.contains("\"old_name\""),
        "expected no 'old_name' in output: {stdout}"
    );
}

#[test]
fn mapping_file_nonexistent_gives_error() {
    morph()
        .args([
            "-f",
            "json",
            "-t",
            "json",
            "-m",
            "/nonexistent/mapping.morph",
        ])
        .write_stdin(r#"{"a":1}"#)
        .assert()
        .failure()
        .stderr(predicates::str::contains("nonexistent"));
}

#[test]
fn mapping_file_invalid_syntax_gives_parser_error() {
    let mapping = temp_file("invalid!!!syntax here", ".morph");

    morph()
        .args([
            "-f",
            "json",
            "-t",
            "json",
            "-m",
            mapping.path().to_str().unwrap(),
        ])
        .write_stdin(r#"{"a":1}"#)
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// -e flag (inline expressions)
// ---------------------------------------------------------------------------

#[test]
fn single_expr_works() {
    let input = temp_file(r#"{"x": 1, "y": 2}"#, ".json");

    let output = morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-t",
            "json",
            "--compact",
            "-e",
            "drop .y",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"x\""), "stdout: {stdout}");
    assert!(!stdout.contains("\"y\""), "stdout: {stdout}");
}

#[test]
fn multiple_expr_apply_in_order() {
    let input = temp_file(r#"{"a": 1, "b": 2, "c": 3}"#, ".json");

    let output = morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-t",
            "json",
            "--compact",
            "-e",
            "rename .a -> .alpha",
            "-e",
            "drop .c",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"alpha\""), "stdout: {stdout}");
    assert!(!stdout.contains("\"a\""), "stdout: {stdout}");
    assert!(!stdout.contains("\"c\""), "stdout: {stdout}");
    assert!(stdout.contains("\"b\""), "stdout: {stdout}");
}

// ---------------------------------------------------------------------------
// -m + -e combined (file first, then expressions)
// ---------------------------------------------------------------------------

#[test]
fn mapping_and_expr_combined() {
    let input = temp_file(r#"{"old": "val", "extra": "gone", "keep": true}"#, ".json");
    let mapping = temp_file("rename .old -> .new", ".morph");

    let output = morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-t",
            "json",
            "--compact",
            "-m",
            mapping.path().to_str().unwrap(),
            "-e",
            "drop .extra",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    // File mapping applied first: .old → .new
    assert!(stdout.contains("\"new\""), "stdout: {stdout}");
    assert!(!stdout.contains("\"old\""), "stdout: {stdout}");
    // Then inline expression: drop .extra
    assert!(!stdout.contains("\"extra\""), "stdout: {stdout}");
    // Untouched field remains
    assert!(stdout.contains("\"keep\""), "stdout: {stdout}");
}

// ---------------------------------------------------------------------------
// --dry-run
// ---------------------------------------------------------------------------

#[test]
fn dry_run_valid_mapping_reports_valid() {
    morph()
        .args([
            "--dry-run",
            "-e",
            "rename .x -> .y",
            "-f",
            "json",
            "-t",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("mapping valid"));
}

#[test]
fn dry_run_invalid_mapping_reports_error() {
    morph()
        .args([
            "--dry-run",
            "-e",
            "invalid!!!syntax",
            "-f",
            "json",
            "-t",
            "json",
        ])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// No mapping flags → straight passthrough
// ---------------------------------------------------------------------------

#[test]
fn no_mapping_flags_passthrough() {
    let input = temp_file(r#"{"name": "Alice", "age": 30}"#, ".json");

    let output = morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-t",
            "json",
            "--compact",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"name\""), "stdout: {stdout}");
    assert!(stdout.contains("\"Alice\""), "stdout: {stdout}");
    assert!(stdout.contains("\"age\""), "stdout: {stdout}");
}

// ---------------------------------------------------------------------------
// Full integration: end-to-end with file I/O
// ---------------------------------------------------------------------------

#[test]
fn full_integration_json_to_yaml_with_mapping() {
    let input = temp_file(r#"{"x": 42, "y": "hello"}"#, ".json");
    let output_file = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();

    morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-o",
            output_file.path().to_str().unwrap(),
            "-e",
            "rename .x -> .value",
        ])
        .assert()
        .success();

    let result = std::fs::read_to_string(output_file.path()).unwrap();
    assert!(result.contains("value:"), "result: {result}");
    assert!(result.contains("42"), "result: {result}");
    assert!(!result.contains("x:"), "result: {result}");
}
