use crate::error;
use crate::value::Value;
use indexmap::IndexMap;

/// Parse a JSON string into a Universal Value.
pub fn from_str(input: &str) -> error::Result<Value> {
    let json_val: serde_json::Value = serde_json::from_str(input)?;
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

fn json_to_value(json: serde_json::Value) -> Value {
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

fn value_to_json(value: &Value) -> serde_json::Value {
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

    #[test]
    fn parse_simple_object() {
        let input = r#"{"name": "Alice", "age": 30}"#;
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("Alice".into())));
        assert_eq!(val.get_path(".age"), Some(&Value::Int(30)));
    }

    #[test]
    fn parse_nested_object() {
        let input = r#"{"user": {"name": "Bob", "scores": [1, 2, 3]}}"#;
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".user.name"),
            Some(&Value::String("Bob".into()))
        );
        assert_eq!(val.get_path(".user.scores[0]"), Some(&Value::Int(1)));
    }

    #[test]
    fn parse_all_types() {
        let input = r#"{"s": "hello", "i": 42, "f": 3.15, "b": true, "n": null, "a": [1]}"#;
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".s"), Some(&Value::String("hello".into())));
        assert_eq!(val.get_path(".i"), Some(&Value::Int(42)));
        assert_eq!(val.get_path(".f"), Some(&Value::Float(3.15)));
        assert_eq!(val.get_path(".b"), Some(&Value::Bool(true)));
        assert_eq!(val.get_path(".n"), Some(&Value::Null));
    }

    #[test]
    fn roundtrip_json() {
        let input = r#"{"key":"value","num":42,"arr":[1,2,3]}"#;
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn pretty_output() {
        let input = r#"{"a":1}"#;
        let val = from_str(input).unwrap();
        let pretty = to_string_pretty(&val).unwrap();
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  "));
    }

    #[test]
    fn compact_output() {
        let input = r#"{"a": 1, "b": 2}"#;
        let val = from_str(input).unwrap();
        let compact = to_string(&val).unwrap();
        assert!(!compact.contains('\n'));
    }

    #[test]
    fn parse_error_has_location() {
        let bad = "{ invalid }";
        let err = from_str(bad).unwrap_err();
        match err {
            crate::error::MorphError::Format { line, column, .. } => {
                assert!(line.is_some());
                assert!(column.is_some());
            }
            other => panic!("expected Format error, got: {other:?}"),
        }
    }

    #[test]
    fn empty_object() {
        let val = from_str("{}").unwrap();
        assert_eq!(val, Value::Map(IndexMap::new()));
    }

    #[test]
    fn empty_array() {
        let val = from_str("[]").unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }
}
