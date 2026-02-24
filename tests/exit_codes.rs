#![allow(deprecated)]
//! Integration tests for issue #85: deterministic exit codes per error category.
//!
//! Exit code semantics:
//!   0 — Success
//!   1 — General / unknown
//!   2 — CLI / usage error
//!   3 — I/O error (file not found, permission denied)
//!   4 — Format / parse error (malformed input)
//!   5 — Mapping evaluation error
//!   6 — Value error

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
// Exit code 0 — Success
// ---------------------------------------------------------------------------

#[test]
fn exit_0_on_success() {
    let input = temp_file(r#"{"key": "value"}"#, ".json");
    morph()
        .args(["-i", input.path().to_str().unwrap(), "-t", "yaml"])
        .assert()
        .success()
        .code(0);
}

#[test]
fn exit_0_on_formats_flag() {
    morph().args(["--formats"]).assert().success().code(0);
}

#[test]
fn exit_0_on_functions_flag() {
    morph().args(["--functions"]).assert().success().code(0);
}

#[test]
fn exit_0_on_dry_run_valid_mapping() {
    morph()
        .args([
            "--dry-run",
            "-e",
            "rename .a -> .b",
            "-f",
            "json",
            "-t",
            "json",
        ])
        .assert()
        .success()
        .code(0);
}

// ---------------------------------------------------------------------------
// Exit code 2 — CLI / usage error
// ---------------------------------------------------------------------------

#[test]
fn exit_2_on_unknown_input_format() {
    let input = temp_file(r#"{"key": "value"}"#, ".json");
    morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-f",
            "xyzzy",
            "-t",
            "json",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn exit_2_on_unknown_output_format() {
    let input = temp_file(r#"{"key": "value"}"#, ".json");
    morph()
        .args(["-i", input.path().to_str().unwrap(), "-t", "xyzzy"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn exit_2_on_missing_format_for_stdin() {
    // Reading from stdin without -f should fail with exit code 2
    morph()
        .args(["-t", "json"])
        .write_stdin("{}")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn exit_2_on_cannot_detect_extension() {
    let input = temp_file(r#"{"key": "value"}"#, ".unknown");
    morph()
        .args(["-i", input.path().to_str().unwrap(), "-t", "json"])
        .assert()
        .failure()
        .code(2);
}

// ---------------------------------------------------------------------------
// Exit code 3 — I/O error
// ---------------------------------------------------------------------------

#[test]
fn exit_3_on_file_not_found() {
    morph()
        .args(["-i", "/tmp/nonexistent_morph_test_file.json", "-t", "yaml"])
        .assert()
        .failure()
        .code(3);
}

// ---------------------------------------------------------------------------
// Exit code 4 — Format / parse error
// ---------------------------------------------------------------------------

#[test]
fn exit_4_on_malformed_json() {
    let input = temp_file("{ not valid json !!!", ".json");
    morph()
        .args(["-i", input.path().to_str().unwrap(), "-t", "yaml"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn exit_4_on_malformed_toml() {
    let input = temp_file("[broken\nkey =", ".toml");
    morph()
        .args(["-i", input.path().to_str().unwrap(), "-t", "json"])
        .assert()
        .failure()
        .code(4);
}

#[test]
fn exit_4_on_malformed_yaml() {
    let input = temp_file(":\n  - :\n    - :\n      :", ".yaml");
    morph()
        .args(["-i", input.path().to_str().unwrap(), "-t", "json"])
        .assert()
        .failure()
        .code(4);
}

// ---------------------------------------------------------------------------
// Exit code 5 — Mapping evaluation error
// ---------------------------------------------------------------------------

#[test]
fn exit_5_on_mapping_parse_error() {
    let input = temp_file(r#"{"key": "value"}"#, ".json");
    morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-t",
            "json",
            "-e",
            "invalid!!!syntax@@@",
        ])
        .assert()
        .failure()
        .code(5);
}

#[test]
fn exit_5_on_mapping_eval_error() {
    // `each .nonexistent` should fail during mapping evaluation
    let input = temp_file(r#"{"key": "value"}"#, ".json");
    morph()
        .args([
            "-i",
            input.path().to_str().unwrap(),
            "-t",
            "json",
            "-e",
            "each .nonexistent { drop .key }",
        ])
        .assert()
        .failure()
        .code(5);
}

// ---------------------------------------------------------------------------
// Determinism: same error → same code every time
// ---------------------------------------------------------------------------

#[test]
fn exit_codes_are_deterministic() {
    // Run the same failing command 3 times and verify same exit code
    for _ in 0..3 {
        morph()
            .args(["-i", "/tmp/nonexistent_morph_test_file.json", "-t", "yaml"])
            .assert()
            .failure()
            .code(3);
    }
}
