use crate::error;
use crate::value::Value;
use indexmap::IndexMap;
use std::io::Read;

/// Parse a JSON string into a Universal Value.
pub fn from_str(input: &str) -> error::Result<Value> {
    let json_val: serde_json::Value = serde_json::from_str(input)?;
    Ok(json_to_value(json_val))
}

/// Parse JSON from a reader into a Universal Value.
pub fn from_reader<R: Read>(reader: R) -> error::Result<Value> {
    let json_val: serde_json::Value = serde_json::from_reader(reader)?;
    Ok(json_to_value(json_val))
}

/// Serialize a Universal Value to a pretty-printed JSON string.
pub fn to_string_pretty(value: &Value) -> error::Result<String> {
    let json_val = value_to_json(value);
    let s = serde_json::to_string_pretty(&json_val)
        .map_err(|e| error::MorphError::format(e.to_string()))?;
    Ok(s)
}

/// Serialize a Universal Value to a compact JSON string.
pub fn to_string(value: &Value) -> error::Result<String> {
    let json_val = value_to_json(value);
    let s =
        serde_json::to_string(&json_val).map_err(|e| error::MorphError::format(e.to_string()))?;
    Ok(s)
}

pub(crate) fn json_to_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                // Fallback – shouldn't happen for normal JSON
                Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let mut map = IndexMap::new();
            for (k, v) in obj {
                map.insert(k, json_to_value(v));
            }
            Value::Map(map)
        }
    }
}

pub(crate) fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => {
            // Represent bytes as an array of numbers in JSON
            serde_json::Value::Array(
                b.iter()
                    .map(|byte| serde_json::Value::from(*byte))
                    .collect(),
            )
        }
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
        Value::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Primitives round-trip --

    #[test]
    fn roundtrip_null() {
        let val = from_str("null").unwrap();
        assert_eq!(val, Value::Null);
        let out = to_string(&val).unwrap();
        assert_eq!(out, "null");
    }

    #[test]
    fn roundtrip_bool_true() {
        let val = from_str("true").unwrap();
        assert_eq!(val, Value::Bool(true));
        let out = to_string(&val).unwrap();
        assert_eq!(out, "true");
    }

    #[test]
    fn roundtrip_bool_false() {
        let val = from_str("false").unwrap();
        assert_eq!(val, Value::Bool(false));
        let out = to_string(&val).unwrap();
        assert_eq!(out, "false");
    }

    #[test]
    fn roundtrip_int() {
        let val = from_str("42").unwrap();
        assert_eq!(val, Value::Int(42));
        let out = to_string(&val).unwrap();
        assert_eq!(out, "42");
    }

    #[test]
    fn roundtrip_float() {
        let val = from_str("3.15").unwrap();
        assert_eq!(val, Value::Float(3.15));
        let out = to_string(&val).unwrap();
        let val2 = from_str(&out).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_string() {
        let val = from_str(r#""hello""#).unwrap();
        assert_eq!(val, Value::String("hello".into()));
        let out = to_string(&val).unwrap();
        assert_eq!(out, r#""hello""#);
    }

    // -- Nested objects --

    #[test]
    fn nested_objects_roundtrip() {
        let input = r#"{"a":{"b":{"c":1}}}"#;
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".a.b.c"), Some(&Value::Int(1)));
        let out = to_string(&val).unwrap();
        let val2 = from_str(&out).unwrap();
        assert_eq!(val, val2);
    }

    // -- Arrays --

    #[test]
    fn arrays_roundtrip() {
        let input = r#"[1,"two",null,[3]]"#;
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0], Value::Int(1));
        assert_eq!(arr[1], Value::String("two".into()));
        assert_eq!(arr[2], Value::Null);
        assert!(matches!(&arr[3], Value::Array(_)));

        let out = to_string(&val).unwrap();
        let val2 = from_str(&out).unwrap();
        assert_eq!(val, val2);
    }

    // -- Large integers --

    #[test]
    fn large_integers_i64_max() {
        let input = format!("{}", i64::MAX);
        let val = from_str(&input).unwrap();
        assert_eq!(val, Value::Int(i64::MAX));
        let out = to_string(&val).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn large_integers_i64_min() {
        let input = format!("{}", i64::MIN);
        let val = from_str(&input).unwrap();
        assert_eq!(val, Value::Int(i64::MIN));
        let out = to_string(&val).unwrap();
        assert_eq!(out, input);
    }

    // -- Float precision --

    #[test]
    fn float_precision_preserved() {
        let input = "3.14159265358979";
        let val = from_str(input).unwrap();
        match &val {
            Value::Float(f) => {
                #[allow(clippy::approx_constant)]
                let expected = 3.14159265358979_f64;
                assert!((*f - expected).abs() < f64::EPSILON);
            }
            _ => panic!("expected Float"),
        }
        let out = to_string(&val).unwrap();
        let val2 = from_str(&out).unwrap();
        assert_eq!(val, val2);
    }

    // -- Unicode --

    #[test]
    fn unicode_roundtrip() {
        let input = r#""héllo wörld 🦀""#;
        let val = from_str(input).unwrap();
        assert_eq!(val, Value::String("héllo wörld 🦀".into()));
        let out = to_string(&val).unwrap();
        let val2 = from_str(&out).unwrap();
        assert_eq!(val, val2);
    }

    // -- Empty structures --

    #[test]
    fn empty_object() {
        let val = from_str("{}").unwrap();
        assert_eq!(val, Value::Map(IndexMap::new()));
        let out = to_string(&val).unwrap();
        assert_eq!(out, "{}");
    }

    #[test]
    fn empty_array() {
        let val = from_str("[]").unwrap();
        assert_eq!(val, Value::Array(vec![]));
        let out = to_string(&val).unwrap();
        assert_eq!(out, "[]");
    }

    #[test]
    fn empty_string() {
        let val = from_str(r#""""#).unwrap();
        assert_eq!(val, Value::String(String::new()));
        let out = to_string(&val).unwrap();
        assert_eq!(out, r#""""#);
    }

    // -- Key order preserved --

    #[test]
    fn key_order_preserved() {
        let input = r#"{"z":1,"a":2,"m":3}"#;
        let val = from_str(input).unwrap();
        let keys: Vec<&String> = match &val {
            Value::Map(m) => m.keys().collect(),
            _ => panic!("expected map"),
        };
        assert_eq!(keys, vec!["z", "a", "m"]);

        // Round-trip should preserve order
        let out = to_string(&val).unwrap();
        let val2 = from_str(&out).unwrap();
        let keys2: Vec<&String> = match &val2 {
            Value::Map(m) => m.keys().collect(),
            _ => panic!("expected map"),
        };
        assert_eq!(keys2, vec!["z", "a", "m"]);
    }

    // -- Pretty output --

    #[test]
    fn pretty_output_indented() {
        let input = r#"{"a":1,"b":[2,3]}"#;
        let val = from_str(input).unwrap();
        let pretty = to_string_pretty(&val).unwrap();
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  "));
        // Pretty output should still parse back correctly
        let val2 = from_str(&pretty).unwrap();
        assert_eq!(val, val2);
    }

    // -- Compact output --

    #[test]
    fn compact_output_no_whitespace() {
        let input = r#"{"a": 1, "b": [2, 3]}"#;
        let val = from_str(input).unwrap();
        let compact = to_string(&val).unwrap();
        assert!(!compact.contains('\n'));
        assert!(!compact.contains("  "));
    }

    // -- Invalid JSON --

    #[test]
    fn invalid_json_returns_format_error_with_position() {
        let bad = "{ invalid }";
        let err = from_str(bad).unwrap_err();
        match err {
            crate::error::MorphError::Format {
                message,
                line,
                column,
            } => {
                assert!(!message.is_empty());
                assert!(line.is_some());
                assert!(column.is_some());
            }
            other => panic!("expected Format error, got: {other:?}"),
        }
    }

    #[test]
    fn invalid_json_truncated() {
        let err = from_str(r#"{"key": "#).unwrap_err();
        assert!(matches!(err, crate::error::MorphError::Format { .. }));
    }

    #[test]
    fn invalid_json_trailing_comma() {
        let err = from_str(r#"{"a": 1,}"#).unwrap_err();
        assert!(matches!(err, crate::error::MorphError::Format { .. }));
    }

    // -- Large file --

    #[test]
    fn large_json_array_parses() {
        // Build a JSON array with 10_000 objects
        let mut items: Vec<String> = Vec::with_capacity(10_000);
        for i in 0..10_000 {
            items.push(format!(r#"{{"id":{},"name":"item_{i}"}}"#, i));
        }
        let input = format!("[{}]", items.join(","));
        let val = from_str(&input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 10_000);
        assert_eq!(arr[0].get_path(".id"), Some(&Value::Int(0)));
        assert_eq!(arr[9999].get_path(".id"), Some(&Value::Int(9999)));
    }

    // -- from_reader --

    #[test]
    fn from_reader_works() {
        let data = r#"{"x": 42}"#;
        let val = from_reader(data.as_bytes()).unwrap();
        assert_eq!(val.get_path(".x"), Some(&Value::Int(42)));
    }

    // -- Complex nested round-trip --

    #[test]
    fn complex_roundtrip() {
        let input = r#"{"users":[{"name":"Alice","scores":[100,95],"meta":{"active":true}},{"name":"Bob","scores":[],"meta":{"active":false}}],"total":2}"#;
        let val = from_str(input).unwrap();
        let out = to_string(&val).unwrap();
        let val2 = from_str(&out).unwrap();
        assert_eq!(val, val2);
    }

    // -- Negative numbers --

    #[test]
    fn negative_int() {
        let val = from_str("-42").unwrap();
        assert_eq!(val, Value::Int(-42));
    }

    #[test]
    fn negative_float() {
        let val = from_str("-1.5").unwrap();
        assert_eq!(val, Value::Float(-1.5));
    }
}
