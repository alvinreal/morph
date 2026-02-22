use crate::error;
use crate::value::Value;
use std::io::Read;

/// Parse a JSON Lines string into a Universal Value.
///
/// Each non-empty line is parsed as a separate JSON value.
/// The result is always an array of values (one per line).
/// If the input contains a single non-empty line, the result is still an array
/// with one element (preserving JSONL semantics).
pub fn from_str(input: &str) -> error::Result<Value> {
    let mut values = Vec::new();

    for (line_num, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let json_val: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
            error::MorphError::format_at(
                format!("invalid JSON on line {}: {e}", line_num + 1),
                line_num + 1,
                e.column(),
            )
        })?;
        values.push(crate::formats::json::json_to_value(json_val));
    }

    Ok(Value::Array(values))
}

/// Parse JSON Lines from a reader.
pub fn from_reader<R: Read>(mut reader: R) -> error::Result<Value> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    from_str(&buf)
}

/// Serialize a Universal Value to a JSON Lines string.
///
/// If the value is an array, each element is written as a single JSON line.
/// If the value is not an array, it is written as a single JSON line.
pub fn to_string(value: &Value) -> error::Result<String> {
    match value {
        Value::Array(arr) => {
            let mut lines = Vec::with_capacity(arr.len());
            for item in arr {
                let json_val = crate::formats::json::value_to_json(item);
                let line = serde_json::to_string(&json_val)
                    .map_err(|e| error::MorphError::format(e.to_string()))?;
                lines.push(line);
            }
            Ok(lines.join("\n") + "\n")
        }
        _ => {
            let json_val = crate::formats::json::value_to_json(value);
            let line = serde_json::to_string(&json_val)
                .map_err(|e| error::MorphError::format(e.to_string()))?;
            Ok(line + "\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    // -----------------------------------------------------------------------
    // Read: basic parsing
    // -----------------------------------------------------------------------

    #[test]
    fn read_three_lines() {
        let input = r#"{"name":"Alice","age":30}
{"name":"Bob","age":25}
{"name":"Charlie","age":35}"#;
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr[0].get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(arr[1].get_path(".age"), Some(&Value::Int(25)));
        assert_eq!(
            arr[2].get_path(".name"),
            Some(&Value::String("Charlie".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Write: one JSON per line, no trailing comma
    // -----------------------------------------------------------------------

    #[test]
    fn write_array_of_objects() {
        let mut m1 = IndexMap::new();
        m1.insert("name".to_string(), Value::String("Alice".into()));
        let mut m2 = IndexMap::new();
        m2.insert("name".to_string(), Value::String("Bob".into()));
        let val = Value::Array(vec![Value::Map(m1), Value::Map(m2)]);
        let output = to_string(&val).unwrap();
        let lines: Vec<&str> = output.trim_end().split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], r#"{"name":"Alice"}"#);
        assert_eq!(lines[1], r#"{"name":"Bob"}"#);
        // No trailing comma
        assert!(!output.contains(','));
        // Ends with newline
        assert!(output.ends_with('\n'));
    }

    // -----------------------------------------------------------------------
    // Empty lines skipped
    // -----------------------------------------------------------------------

    #[test]
    fn empty_lines_skipped() {
        let input = r#"{"a":1}

{"b":2}

"#;
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Single object → single-line output
    // -----------------------------------------------------------------------

    #[test]
    fn single_non_array_value() {
        let mut m = IndexMap::new();
        m.insert("x".to_string(), Value::Int(42));
        let val = Value::Map(m);
        let output = to_string(&val).unwrap();
        assert_eq!(output, "{\"x\":42}\n");
    }

    // -----------------------------------------------------------------------
    // Round-trip JSONL → JSON → JSONL
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_jsonl_to_json_to_jsonl() {
        let input = r#"{"name":"Alice","age":30}
{"name":"Bob","age":25}"#;
        let val = from_str(input).unwrap();

        // Value → JSON string → Value
        let json_str = crate::formats::json::to_string(&val).unwrap();
        let val_from_json = crate::formats::json::from_str(&json_str).unwrap();

        // Value → JSONL string → Value
        let jsonl_str = to_string(&val_from_json).unwrap();
        let val_roundtrip = from_str(&jsonl_str).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    // -----------------------------------------------------------------------
    // Invalid line → error with line number
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_line_error_with_line_number() {
        let input = r#"{"valid":true}
{invalid json}
{"also_valid":true}"#;
        let err = from_str(input).unwrap_err();
        match err {
            crate::error::MorphError::Format {
                message,
                line,
                column,
            } => {
                assert!(message.contains("line 2"), "message: {message}");
                assert_eq!(line, Some(2));
                assert!(column.is_some());
            }
            other => panic!("expected Format error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Mixed types per line
    // -----------------------------------------------------------------------

    #[test]
    fn mixed_types_per_line() {
        let input = r#"{"name":"Alice"}
[1,2,3]
42
"hello"
true
null"#;
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 6);
        // Object
        assert_eq!(
            arr[0].get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
        // Array
        assert!(matches!(&arr[1], Value::Array(_)));
        // Number
        assert_eq!(arr[2], Value::Int(42));
        // String
        assert_eq!(arr[3], Value::String("hello".into()));
        // Bool
        assert_eq!(arr[4], Value::Bool(true));
        // Null
        assert_eq!(arr[5], Value::Null);
    }

    // -----------------------------------------------------------------------
    // Write: mixed types
    // -----------------------------------------------------------------------

    #[test]
    fn write_mixed_types() {
        let val = Value::Array(vec![
            Value::Int(42),
            Value::String("hello".into()),
            Value::Bool(true),
            Value::Null,
        ]);
        let output = to_string(&val).unwrap();
        let lines: Vec<&str> = output.trim_end().split('\n').collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "42");
        assert_eq!(lines[1], "\"hello\"");
        assert_eq!(lines[2], "true");
        assert_eq!(lines[3], "null");
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn empty_input() {
        let val = from_str("").unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    #[test]
    fn whitespace_only_input() {
        let val = from_str("   \n  \n   ").unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    // -----------------------------------------------------------------------
    // from_reader
    // -----------------------------------------------------------------------

    #[test]
    fn from_reader_works() {
        let data = "{\"x\":1}\n{\"x\":2}\n";
        let val = from_reader(data.as_bytes()).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Write: empty array
    // -----------------------------------------------------------------------

    #[test]
    fn write_empty_array() {
        let val = Value::Array(vec![]);
        let output = to_string(&val).unwrap();
        // Empty array produces just a newline
        assert_eq!(output, "\n");
    }

    // -----------------------------------------------------------------------
    // Round-trip: write then read
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_write_read() {
        let mut m1 = IndexMap::new();
        m1.insert("id".to_string(), Value::Int(1));
        m1.insert("name".to_string(), Value::String("Alice".into()));
        m1.insert("active".to_string(), Value::Bool(true));
        let mut m2 = IndexMap::new();
        m2.insert("id".to_string(), Value::Int(2));
        m2.insert("name".to_string(), Value::String("Bob".into()));
        m2.insert("active".to_string(), Value::Bool(false));
        let val = Value::Array(vec![Value::Map(m1), Value::Map(m2)]);

        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Unicode
    // -----------------------------------------------------------------------

    #[test]
    fn unicode_roundtrip() {
        let input = "{\"emoji\":\"🦀\"}\n{\"accent\":\"héllo\"}\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Nested objects
    // -----------------------------------------------------------------------

    #[test]
    fn nested_objects() {
        let input = r#"{"user":{"name":"Alice","scores":[100,95]}}"#;
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 1);
        assert_eq!(
            arr[0].get_path(".user.name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(arr[0].get_path(".user.scores[0]"), Some(&Value::Int(100)));
    }

    // -----------------------------------------------------------------------
    // Lines with trailing whitespace
    // -----------------------------------------------------------------------

    #[test]
    fn trailing_whitespace_on_lines() {
        let input = "{\"a\":1}  \n  {\"b\":2}  \n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Single line input → array with one element
    // -----------------------------------------------------------------------

    #[test]
    fn single_line_input() {
        let input = "{\"x\":42}";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get_path(".x"), Some(&Value::Int(42)));
    }
}
