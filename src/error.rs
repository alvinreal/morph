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
        // toml::de::Error doesn't expose line/column directly but provides a
        // byte-offset span.  We store the start offset in `line` (since it is
        // the most useful positional information available) and leave `column`
        // as `None` unless we can derive both.  In practice consumers can use
        // the span start to point at the offending position in the source.
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
}
