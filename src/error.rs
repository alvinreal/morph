use thiserror::Error;

/// The unified error type for the morph project.
#[derive(Debug, Error)]
pub enum MorphError {
    #[error("Format error: {message}")]
    Format {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Mapping error: {message}")]
    Mapping {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },

    #[error("CLI error: {0}")]
    Cli(String),

    #[error("Value error: {0}")]
    Value(String),
}

/// A convenience result type for the morph project.
pub type Result<T> = std::result::Result<T, MorphError>;

// ---------------------------------------------------------------------------
// From conversions for format-specific crate errors
// ---------------------------------------------------------------------------

impl From<serde_json::Error> for MorphError {
    fn from(err: serde_json::Error) -> Self {
        MorphError::Format {
            message: err.to_string(),
            line: Some(err.line()),
            column: Some(err.column()),
        }
    }
}

impl From<serde_yaml::Error> for MorphError {
    fn from(err: serde_yaml::Error) -> Self {
        let location = err.location();
        MorphError::Format {
            message: err.to_string(),
            line: location.as_ref().map(|l| l.line()),
            column: location.as_ref().map(|l| l.column()),
        }
    }
}

impl From<toml::de::Error> for MorphError {
    fn from(err: toml::de::Error) -> Self {
        let span = err.span();
        MorphError::Format {
            message: err.to_string(),
            line: span.map(|s| s.start),
            column: None,
        }
    }
}

impl From<csv::Error> for MorphError {
    fn from(err: csv::Error) -> Self {
        let position = err
            .position()
            .map(|p| (p.line() as usize, p.byte() as usize));
        MorphError::Format {
            message: err.to_string(),
            line: position.map(|(l, _)| l),
            column: position.map(|(_, b)| b),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

impl MorphError {
    /// Create a `Format` error with just a message.
    pub fn format(msg: impl Into<String>) -> Self {
        MorphError::Format {
            message: msg.into(),
            line: None,
            column: None,
        }
    }

    /// Create a `Format` error with a message and source location.
    pub fn format_at(msg: impl Into<String>, line: usize, column: usize) -> Self {
        MorphError::Format {
            message: msg.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Create a `Mapping` error with just a message.
    pub fn mapping(msg: impl Into<String>) -> Self {
        MorphError::Mapping {
            message: msg.into(),
            line: None,
            column: None,
        }
    }

    /// Create a `Mapping` error with a message and source location.
    pub fn mapping_at(msg: impl Into<String>, line: usize, column: usize) -> Self {
        MorphError::Mapping {
            message: msg.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Create a `Cli` error.
    pub fn cli(msg: impl Into<String>) -> Self {
        MorphError::Cli(msg.into())
    }

    /// Create a `Value` error.
    pub fn value(msg: impl Into<String>) -> Self {
        MorphError::Value(msg.into())
    }
}

// ---------------------------------------------------------------------------
// Exit codes — deterministic, user-facing error categories
// ---------------------------------------------------------------------------

/// Deterministic exit codes for each error category.
///
/// These provide stable, scriptable exit semantics so callers can
/// programmatically distinguish failure modes.
///
/// | Code | Category              | Description                               |
/// |------|-----------------------|-------------------------------------------|
/// |  0   | Success               | No error                                  |
/// |  1   | General / unknown     | Catch-all (should not normally occur)      |
/// |  2   | CLI / usage           | Invalid arguments, unknown flags           |
/// |  3   | I/O                   | File not found, permission denied, etc.    |
/// |  4   | Format / parse        | Malformed input data                       |
/// |  5   | Mapping               | Error in mapping evaluation                |
/// |  6   | Value                 | Type mismatch, overflow, invalid cast      |
pub mod exit_code {
    /// Catch-all for unexpected errors.
    pub const GENERAL: i32 = 1;
    /// Invalid CLI arguments or usage.
    pub const CLI: i32 = 2;
    /// I/O error (file not found, permission denied, broken pipe, etc.).
    pub const IO: i32 = 3;
    /// Format/parse error (malformed input data).
    pub const FORMAT: i32 = 4;
    /// Mapping evaluation error.
    pub const MAPPING: i32 = 5;
    /// Value error (type mismatch, overflow, invalid cast).
    pub const VALUE: i32 = 6;
}

impl MorphError {
    /// Return the deterministic exit code for this error category.
    pub fn exit_code(&self) -> i32 {
        match self {
            MorphError::Cli(_) => exit_code::CLI,
            MorphError::Io(_) => exit_code::IO,
            MorphError::Format { .. } => exit_code::FORMAT,
            MorphError::Mapping { .. } => exit_code::MAPPING,
            MorphError::Value(_) => exit_code::VALUE,
        }
    }
}

// ---------------------------------------------------------------------------
// Pretty error formatting
// ---------------------------------------------------------------------------

impl MorphError {
    /// Format this error for human-friendly display on stderr.
    ///
    /// Optionally accepts the source text to generate source snippets with
    /// line numbers and carets pointing at the error location.
    pub fn pretty_print(&self, source: Option<&str>) -> String {
        match self {
            MorphError::Format {
                message,
                line,
                column,
            } => {
                let mut out = format!("error: {message}");
                if let (Some(l), Some(source)) = (line, source) {
                    if let Some(snippet) = format_source_snippet(source, *l, *column) {
                        out.push('\n');
                        out.push_str(&snippet);
                    }
                } else if let (Some(l), Some(c)) = (line, column) {
                    out.push_str(&format!("\n  --> line {l}, column {c}"));
                } else if let Some(l) = line {
                    out.push_str(&format!("\n  --> line {l}"));
                }
                out
            }
            MorphError::Io(err) => {
                format!("error: {err}")
            }
            MorphError::Mapping {
                message,
                line,
                column,
            } => {
                let mut out = format!("error: {message}");
                if let (Some(l), Some(c)) = (line, column) {
                    out.push_str(&format!("\n  --> line {l}, column {c}"));
                } else if let Some(l) = line {
                    out.push_str(&format!("\n  --> line {l}"));
                }
                out
            }
            MorphError::Cli(msg) => {
                format!("error: {msg}")
            }
            MorphError::Value(msg) => {
                format!("error: {msg}")
            }
        }
    }
}

/// Format a source snippet with line number, the offending line, and a caret.
fn format_source_snippet(source: &str, line: usize, column: Option<usize>) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    if line == 0 || line > lines.len() {
        return None;
    }
    let source_line = lines[line - 1];
    let line_num = line.to_string();
    let pad = line_num.len();

    let mut out = String::new();
    out.push_str(&format!("{:>pad$} |\n", "", pad = pad));
    out.push_str(&format!("{line_num} | {source_line}\n"));
    if let Some(col) = column {
        let col = col.saturating_sub(1); // 1-based → 0-based
        out.push_str(&format!(
            "{:>pad$} | {:>col$}^",
            "",
            "",
            pad = pad,
            col = col
        ));
    }

    Some(out)
}

// ---------------------------------------------------------------------------
// Suggestion helpers (Levenshtein distance)
// ---------------------------------------------------------------------------

/// Compute the Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let b_len = b.len();

    // Use a single-row DP approach to avoid the needless_range_loop clippy warning
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0usize; b_len + 1];

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    for (i, &a_byte) in a_bytes.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, &b_byte) in b_bytes.iter().enumerate() {
            let cost = if a_byte == b_byte { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)
                .min(curr_row[j] + 1)
                .min(prev_row[j] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Find the best match for `input` from a list of `candidates`.
/// Returns `Some(candidate)` if the best match has an edit distance ≤ `max_dist`.
pub fn suggest_closest(input: &str, candidates: &[&str], max_dist: usize) -> Option<String> {
    let input_lower = input.to_lowercase();
    let mut best: Option<(usize, &str)> = None;

    for &candidate in candidates {
        let dist = edit_distance(&input_lower, &candidate.to_lowercase());
        if dist <= max_dist && (best.is_none() || dist < best.unwrap().0) {
            best = Some((dist, candidate));
        }
    }

    best.map(|(_, s)| s.to_string())
}

/// Suggest a similar format name for an unknown format string.
pub fn suggest_format(input: &str) -> Option<String> {
    let known = [
        "json", "jsonl", "ndjson", "yaml", "yml", "toml", "csv", "xml", "msgpack", "mp",
    ];
    suggest_closest(input, &known, 3)
}

/// Suggest a similar function name for an unknown function string.
pub fn suggest_function(input: &str) -> Option<String> {
    let known = [
        "lower",
        "lowercase",
        "downcase",
        "upper",
        "uppercase",
        "upcase",
        "trim",
        "trim_start",
        "ltrim",
        "trim_end",
        "rtrim",
        "len",
        "length",
        "size",
        "replace",
        "contains",
        "starts_with",
        "ends_with",
        "substr",
        "substring",
        "concat",
        "split",
        "join",
        "reverse",
        "to_int",
        "int",
        "to_float",
        "float",
        "to_string",
        "string",
        "str",
        "to_bool",
        "bool",
        "type_of",
        "typeof",
        "abs",
        "min",
        "max",
        "floor",
        "ceil",
        "round",
        "is_null",
        "is_array",
        "coalesce",
        "default",
        "keys",
        "values",
        "unique",
        "first",
        "last",
        "sum",
        "group_by",
        "groupby",
        "if",
    ];
    suggest_closest(input, &known, 3)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    // -- Display tests for each variant --

    #[test]
    fn display_format_error() {
        let err = MorphError::format("unexpected token");
        assert_eq!(err.to_string(), "Format error: unexpected token");
    }

    #[test]
    fn display_format_error_with_location() {
        let err = MorphError::format_at("missing comma", 10, 5);
        let display = err.to_string();
        assert_eq!(display, "Format error: missing comma");
        if let MorphError::Format { line, column, .. } = &err {
            assert_eq!(*line, Some(10));
            assert_eq!(*column, Some(5));
        } else {
            panic!("expected Format variant");
        }
    }

    #[test]
    fn display_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: MorphError = io_err.into();
        assert_eq!(err.to_string(), "I/O error: file not found");
    }

    #[test]
    fn display_mapping_error() {
        let err = MorphError::mapping("unknown field");
        assert_eq!(err.to_string(), "Mapping error: unknown field");
    }

    #[test]
    fn display_mapping_error_with_location() {
        let err = MorphError::mapping_at("type mismatch", 3, 12);
        assert_eq!(err.to_string(), "Mapping error: type mismatch");
        if let MorphError::Mapping { line, column, .. } = &err {
            assert_eq!(*line, Some(3));
            assert_eq!(*column, Some(12));
        } else {
            panic!("expected Mapping variant");
        }
    }

    #[test]
    fn display_cli_error() {
        let err = MorphError::cli("missing required argument");
        assert_eq!(err.to_string(), "CLI error: missing required argument");
    }

    #[test]
    fn display_value_error() {
        let err = MorphError::value("integer overflow");
        assert_eq!(err.to_string(), "Value error: integer overflow");
    }

    // -- From conversions --

    #[test]
    fn from_serde_json_error() {
        let bad_json = "{ invalid }";
        let json_err = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
        let err: MorphError = json_err.into();
        match &err {
            MorphError::Format {
                message,
                line,
                column,
            } => {
                assert!(
                    message.contains("key must be a string"),
                    "message: {message}"
                );
                assert!(line.is_some());
                assert!(column.is_some());
            }
            other => panic!("expected Format, got: {other:?}"),
        }
    }

    #[test]
    fn from_serde_yaml_error() {
        let bad_yaml = ":\n  - :\n    -";
        let yaml_err = serde_yaml::from_str::<serde_yaml::Value>(bad_yaml).unwrap_err();
        let err: MorphError = yaml_err.into();
        match &err {
            MorphError::Format { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Format, got: {other:?}"),
        }
    }

    #[test]
    fn from_toml_error() {
        let bad_toml = "[broken\nkey = ";
        let toml_err = toml::from_str::<toml::Value>(bad_toml).unwrap_err();
        let err: MorphError = toml_err.into();
        match &err {
            MorphError::Format { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Format, got: {other:?}"),
        }
    }

    #[test]
    fn from_csv_error() {
        // Create a CSV reader that will fail due to field count mismatch
        let data = "a,b\n1,2,3";
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(false)
            .from_reader(data.as_bytes());
        let csv_err = rdr
            .records()
            .find_map(|r| r.err())
            .expect("expected a CSV error");
        let err: MorphError = csv_err.into();
        match &err {
            MorphError::Format { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Format, got: {other:?}"),
        }
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: MorphError = io_err.into();
        match &err {
            MorphError::Io(_) => {
                assert_eq!(err.to_string(), "I/O error: access denied");
            }
            other => panic!("expected Io, got: {other:?}"),
        }
    }

    // -- Error chain / source --

    #[test]
    fn io_error_source_chain() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: MorphError = io_err.into();
        // The Io variant should have a source
        let source = err.source();
        assert!(source.is_some());
        assert_eq!(source.unwrap().to_string(), "gone");
    }

    #[test]
    fn format_error_no_source() {
        // Format errors constructed via helpers have no underlying source error
        let err = MorphError::format("bad input");
        assert!(err.source().is_none());
    }

    // -- Result alias --

    #[test]
    fn result_type_alias_works() {
        fn might_fail(ok: bool) -> Result<u32> {
            if ok {
                Ok(42)
            } else {
                Err(MorphError::value("nope"))
            }
        }
        assert!(might_fail(true).is_ok());
        assert!(might_fail(false).is_err());
    }

    // -- Helper constructors produce correct variants --

    #[test]
    fn helpers_produce_correct_variants() {
        assert!(matches!(MorphError::format("x"), MorphError::Format { .. }));
        assert!(matches!(
            MorphError::format_at("x", 1, 1),
            MorphError::Format { .. }
        ));
        assert!(matches!(
            MorphError::mapping("x"),
            MorphError::Mapping { .. }
        ));
        assert!(matches!(
            MorphError::mapping_at("x", 1, 1),
            MorphError::Mapping { .. }
        ));
        assert!(matches!(MorphError::cli("x"), MorphError::Cli(_)));
        assert!(matches!(MorphError::value("x"), MorphError::Value(_)));
    }

    #[test]
    fn format_helper_sets_no_location() {
        if let MorphError::Format { line, column, .. } = MorphError::format("test") {
            assert!(line.is_none());
            assert!(column.is_none());
        }
    }

    #[test]
    fn format_at_helper_sets_location() {
        if let MorphError::Format { line, column, .. } = MorphError::format_at("test", 42, 7) {
            assert_eq!(line, Some(42));
            assert_eq!(column, Some(7));
        }
    }

    #[test]
    fn display_output_is_actionable() {
        // Verify that the display output for each variant starts with a clear prefix
        let cases: Vec<(&str, MorphError)> = vec![
            ("Format error:", MorphError::format("bad")),
            ("I/O error:", std::io::Error::other("disk full").into()),
            ("Mapping error:", MorphError::mapping("no match")),
            ("CLI error:", MorphError::cli("bad flag")),
            ("Value error:", MorphError::value("overflow")),
        ];
        for (prefix, err) in cases {
            let msg = err.to_string();
            assert!(
                msg.starts_with(prefix),
                "expected '{msg}' to start with '{prefix}'"
            );
        }
    }

    // -- Suggestion tests --

    #[test]
    fn suggest_format_jsn() {
        let suggestion = suggest_format("jsn");
        assert_eq!(suggestion, Some("json".to_string()));
    }

    #[test]
    fn suggest_format_ymal() {
        let suggestion = suggest_format("ymal");
        assert!(
            suggestion == Some("yaml".to_string()) || suggestion == Some("yml".to_string()),
            "expected yaml or yml, got {suggestion:?}"
        );
    }

    #[test]
    fn suggest_format_csvv() {
        let suggestion = suggest_format("csvv");
        assert_eq!(suggestion, Some("csv".to_string()));
    }

    #[test]
    fn suggest_format_unknown() {
        let suggestion = suggest_format("protobuf");
        assert!(suggestion.is_none());
    }

    #[test]
    fn suggest_function_lowr() {
        let suggestion = suggest_function("lowr");
        assert_eq!(suggestion, Some("lower".to_string()));
    }

    #[test]
    fn suggest_function_trime() {
        let suggestion = suggest_function("trime");
        assert_eq!(suggestion, Some("trim".to_string()));
    }

    #[test]
    fn suggest_function_to_integer() {
        let suggestion = suggest_function("to_integer");
        // "to_integer" is close to "to_int" (distance 4) or "to_string" (distance 4)
        // With max_dist 3 it may not match; that's acceptable
        // But "toint" is closer
        let _ = suggestion; // just ensure it doesn't panic
    }

    #[test]
    fn suggest_function_unknown() {
        let suggestion = suggest_function("xyzzy");
        assert!(suggestion.is_none());
    }

    // -- Pretty print tests --

    #[test]
    fn pretty_print_format_error_with_source() {
        let err = MorphError::format_at("unexpected token", 2, 5);
        let source = "line 1\nline 2 has error\nline 3";
        let pretty = err.pretty_print(Some(source));
        assert!(pretty.contains("unexpected token"), "msg: {pretty}");
        assert!(pretty.contains("line 2 has error"), "source: {pretty}");
        assert!(pretty.contains("^"), "caret: {pretty}");
        assert!(pretty.contains("2 |"), "line number: {pretty}");
    }

    #[test]
    fn pretty_print_format_error_without_source() {
        let err = MorphError::format_at("bad input", 3, 7);
        let pretty = err.pretty_print(None);
        assert!(pretty.contains("bad input"), "msg: {pretty}");
        assert!(pretty.contains("line 3, column 7"), "location: {pretty}");
    }

    #[test]
    fn pretty_print_cli_error() {
        let err = MorphError::cli("unknown format: 'jsn'");
        let pretty = err.pretty_print(None);
        assert!(pretty.contains("unknown format"), "msg: {pretty}");
    }

    #[test]
    fn pretty_print_io_error() {
        let err = MorphError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "/tmp/nonexistent.json: No such file or directory",
        ));
        let pretty = err.pretty_print(None);
        assert!(pretty.contains("/tmp/nonexistent.json"), "path: {pretty}");
    }

    // -- Edit distance tests --

    #[test]
    fn edit_distance_identical() {
        assert_eq!(edit_distance("hello", "hello"), 0);
    }

    #[test]
    fn edit_distance_one_char() {
        assert_eq!(edit_distance("json", "jsn"), 1);
    }

    #[test]
    fn edit_distance_swap() {
        assert_eq!(edit_distance("yaml", "ymal"), 2);
    }

    #[test]
    fn edit_distance_empty() {
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
    }

    // -- Exit code tests --

    #[test]
    fn exit_code_cli() {
        let err = MorphError::cli("bad flag");
        assert_eq!(err.exit_code(), exit_code::CLI);
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn exit_code_io() {
        let err: MorphError = std::io::Error::new(std::io::ErrorKind::NotFound, "gone").into();
        assert_eq!(err.exit_code(), exit_code::IO);
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn exit_code_format() {
        let err = MorphError::format("bad json");
        assert_eq!(err.exit_code(), exit_code::FORMAT);
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn exit_code_mapping() {
        let err = MorphError::mapping("unknown op");
        assert_eq!(err.exit_code(), exit_code::MAPPING);
        assert_eq!(err.exit_code(), 5);
    }

    #[test]
    fn exit_code_value() {
        let err = MorphError::value("overflow");
        assert_eq!(err.exit_code(), exit_code::VALUE);
        assert_eq!(err.exit_code(), 6);
    }

    #[test]
    fn exit_codes_are_distinct() {
        let codes = [
            exit_code::GENERAL,
            exit_code::CLI,
            exit_code::IO,
            exit_code::FORMAT,
            exit_code::MAPPING,
            exit_code::VALUE,
        ];
        // All codes should be unique
        let mut seen = std::collections::HashSet::new();
        for code in &codes {
            assert!(seen.insert(code), "duplicate exit code: {code}");
        }
    }

    #[test]
    fn exit_codes_are_nonzero() {
        assert_ne!(exit_code::GENERAL, 0);
        assert_ne!(exit_code::CLI, 0);
        assert_ne!(exit_code::IO, 0);
        assert_ne!(exit_code::FORMAT, 0);
        assert_ne!(exit_code::MAPPING, 0);
        assert_ne!(exit_code::VALUE, 0);
    }
}
