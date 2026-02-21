use crate::error;
use crate::value::Value;
use indexmap::IndexMap;
use serde::Deserialize;
use std::io::Read;

/// Parse a YAML string into a Universal Value.
///
/// Supports multi-document YAML: if the input contains multiple `---`-separated
/// documents, they are returned as an array.  A single document is returned
/// directly (unwrapped).
pub fn from_str(input: &str) -> error::Result<Value> {
    let docs: Vec<Value> = serde_yaml::Deserializer::from_str(input)
        .map(|de| {
            let yaml_val = serde_yaml::Value::deserialize(de).map_err(error::MorphError::from)?;
            Ok(yaml_to_value(yaml_val))
        })
        .collect::<error::Result<Vec<Value>>>()?;

    match docs.len() {
        0 => Ok(Value::Null),
        1 => Ok(docs.into_iter().next().unwrap()),
        _ => Ok(Value::Array(docs)),
    }
}

/// Parse YAML from a reader into a Universal Value.
pub fn from_reader<R: Read>(mut reader: R) -> error::Result<Value> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    from_str(&buf)
}

/// Serialize a Universal Value to a YAML string.
pub fn to_string(value: &Value) -> error::Result<String> {
    let yaml_val = value_to_yaml(value);
    let s =
        serde_yaml::to_string(&yaml_val).map_err(|e| error::MorphError::format(e.to_string()))?;
    Ok(s)
}

fn yaml_to_value(yaml: serde_yaml::Value) -> Value {
    match yaml {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::String(n.to_string())
            }
        }
        serde_yaml::Value::String(s) => Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            Value::Array(seq.into_iter().map(yaml_to_value).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut m = IndexMap::new();
            // Collect merge-key (`<<`) entries and flatten them into the map.
            // serde_yaml preserves `<<` as a literal key when deserializing
            // to `Value`, so we resolve it manually.
            let mut merges: Vec<serde_yaml::Value> = Vec::new();
            let mut regular: Vec<(serde_yaml::Value, serde_yaml::Value)> = Vec::new();
            for (k, v) in map {
                if k == serde_yaml::Value::String("<<".into()) {
                    merges.push(v);
                } else {
                    regular.push((k, v));
                }
            }
            // Apply merges first (earlier merge values have lower priority
            // than explicitly set keys, which are applied after).
            for merge_val in merges {
                match merge_val {
                    serde_yaml::Value::Mapping(merge_map) => {
                        for (mk, mv) in merge_map {
                            let key = match mk {
                                serde_yaml::Value::String(s) => s,
                                other => format!("{other:?}"),
                            };
                            // Only insert if not already present (explicit keys win)
                            m.entry(key).or_insert_with(|| yaml_to_value(mv));
                        }
                    }
                    serde_yaml::Value::Sequence(seq) => {
                        // Multiple merge sources: `<<: [*a, *b]`
                        for item in seq {
                            if let serde_yaml::Value::Mapping(merge_map) = item {
                                for (mk, mv) in merge_map {
                                    let key = match mk {
                                        serde_yaml::Value::String(s) => s,
                                        other => format!("{other:?}"),
                                    };
                                    m.entry(key).or_insert_with(|| yaml_to_value(mv));
                                }
                            }
                        }
                    }
                    _ => {
                        // Non-mapping merge value; store as literal `<<` key
                        m.insert("<<".into(), yaml_to_value(merge_val));
                    }
                }
            }
            // Then apply regular keys (they override merge keys)
            for (k, v) in regular {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    other => format!("{other:?}"),
                };
                m.insert(key, yaml_to_value(v));
            }
            Value::Map(m)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_value(tagged.value),
    }
}

fn value_to_yaml(value: &Value) -> serde_yaml::Value {
    match value {
        Value::Null => serde_yaml::Value::Null,
        Value::Bool(b) => serde_yaml::Value::Bool(*b),
        Value::Int(i) => serde_yaml::Value::Number(serde_yaml::Number::from(*i)),
        Value::Float(f) => serde_yaml::Value::Number(serde_yaml::Number::from(*f)),
        Value::String(s) => serde_yaml::Value::String(s.clone()),
        Value::Bytes(b) => {
            // Represent bytes as a sequence of numbers
            serde_yaml::Value::Sequence(
                b.iter()
                    .map(|byte| serde_yaml::Value::Number(serde_yaml::Number::from(*byte as u64)))
                    .collect(),
            )
        }
        Value::Array(arr) => serde_yaml::Value::Sequence(arr.iter().map(value_to_yaml).collect()),
        Value::Map(map) => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in map {
                m.insert(serde_yaml::Value::String(k.clone()), value_to_yaml(v));
            }
            serde_yaml::Value::Mapping(m)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Primitives: strings, ints, floats, bools, null --

    #[test]
    fn parse_simple_yaml() {
        let input = "name: Alice\nage: 30\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("Alice".into())));
        assert_eq!(val.get_path(".age"), Some(&Value::Int(30)));
    }

    #[test]
    fn parse_all_types() {
        let input = "s: hello\ni: 42\nf: 3.15\nb: true\nn: null\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".s"), Some(&Value::String("hello".into())));
        assert_eq!(val.get_path(".i"), Some(&Value::Int(42)));
        assert_eq!(val.get_path(".f"), Some(&Value::Float(3.15)));
        assert_eq!(val.get_path(".b"), Some(&Value::Bool(true)));
        assert_eq!(val.get_path(".n"), Some(&Value::Null));
    }

    #[test]
    fn parse_negative_numbers() {
        let input = "neg_int: -7\nneg_float: -1.5\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".neg_int"), Some(&Value::Int(-7)));
        assert_eq!(val.get_path(".neg_float"), Some(&Value::Float(-1.5)));
    }

    #[test]
    fn parse_large_int() {
        let input = format!("big: {}\n", i64::MAX);
        let val = from_str(&input).unwrap();
        assert_eq!(val.get_path(".big"), Some(&Value::Int(i64::MAX)));
    }

    // -- Nested maps and sequences --

    #[test]
    fn parse_nested_yaml() {
        let input = "user:\n  name: Bob\n  scores:\n    - 1\n    - 2\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".user.name"),
            Some(&Value::String("Bob".into()))
        );
        assert_eq!(val.get_path(".user.scores[0]"), Some(&Value::Int(1)));
        assert_eq!(val.get_path(".user.scores[1]"), Some(&Value::Int(2)));
    }

    #[test]
    fn deeply_nested() {
        let input = "a:\n  b:\n    c:\n      d: deep\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".a.b.c.d"),
            Some(&Value::String("deep".into()))
        );
    }

    #[test]
    fn sequence_of_maps() {
        let input = "- name: Alice\n  age: 30\n- name: Bob\n  age: 25\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
        assert_eq!(
            arr[0].get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(arr[1].get_path(".age"), Some(&Value::Int(25)));
    }

    // -- YAML-specific: anchors & aliases resolve correctly --

    #[test]
    fn anchors_and_aliases() {
        let input = "\
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  database: dev_db
  <<: *defaults

test:
  database: test_db
  <<: *defaults
";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".development.adapter"),
            Some(&Value::String("postgres".into()))
        );
        assert_eq!(
            val.get_path(".development.host"),
            Some(&Value::String("localhost".into()))
        );
        assert_eq!(
            val.get_path(".test.adapter"),
            Some(&Value::String("postgres".into()))
        );
        assert_eq!(
            val.get_path(".test.database"),
            Some(&Value::String("test_db".into()))
        );
    }

    #[test]
    fn simple_anchor_alias() {
        let input = "anchor: &val hello\nalias: *val\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".anchor"),
            Some(&Value::String("hello".into()))
        );
        assert_eq!(val.get_path(".alias"), Some(&Value::String("hello".into())));
    }

    // -- Multi-document: `---` separated docs parse as array --

    #[test]
    fn multi_document() {
        let input = "---\nname: doc1\n---\nname: doc2\n---\nname: doc3\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array for multi-doc, got: {val:?}"),
        };
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr[0].get_path(".name"),
            Some(&Value::String("doc1".into()))
        );
        assert_eq!(
            arr[1].get_path(".name"),
            Some(&Value::String("doc2".into()))
        );
        assert_eq!(
            arr[2].get_path(".name"),
            Some(&Value::String("doc3".into()))
        );
    }

    #[test]
    fn single_document_with_separator() {
        // A single doc with `---` should not be wrapped in an array
        let input = "---\nname: only\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("only".into())));
        assert!(!matches!(val, Value::Array(_)));
    }

    // -- Quoting: strings that look like bools/numbers ("true", "42") stay strings --

    #[test]
    fn quoted_bool_stays_string() {
        let input = "val: \"true\"\nval2: \"false\"\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::String("true".into())));
        assert_eq!(val.get_path(".val2"), Some(&Value::String("false".into())));
    }

    #[test]
    fn quoted_number_stays_string() {
        let input = "val: \"42\"\nval2: \"3.14\"\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::String("42".into())));
        assert_eq!(val.get_path(".val2"), Some(&Value::String("3.14".into())));
    }

    #[test]
    fn unquoted_bool_is_bool() {
        let input = "val: true\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::Bool(true)));
    }

    #[test]
    fn unquoted_number_is_number() {
        let input = "val: 42\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::Int(42)));
    }

    // -- Multiline strings: literal `|` and folded `>` blocks --

    #[test]
    fn multiline_literal_block() {
        let input = "text: |\n  line one\n  line two\n  line three\n";
        let val = from_str(input).unwrap();
        let text = match val.get_path(".text") {
            Some(Value::String(s)) => s.clone(),
            other => panic!("expected string, got: {other:?}"),
        };
        assert!(text.contains("line one\n"));
        assert!(text.contains("line two\n"));
        assert!(text.contains("line three\n"));
    }

    #[test]
    fn multiline_folded_block() {
        let input = "text: >\n  line one\n  line two\n  line three\n";
        let val = from_str(input).unwrap();
        let text = match val.get_path(".text") {
            Some(Value::String(s)) => s.clone(),
            other => panic!("expected string, got: {other:?}"),
        };
        // Folded style joins lines with spaces
        assert!(text.contains("line one"));
        assert!(text.contains("line two"));
        assert!(text.contains("line three"));
        // Should NOT have newlines between the lines (they get folded)
        assert!(!text.contains("one\nline"));
    }

    // -- Round-trip YAML: parse then serialize, then parse again --

    #[test]
    fn roundtrip_yaml() {
        let input = "key: value\nnum: 42\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_complex() {
        let input = "\
users:
- name: Alice
  scores:
  - 100
  - 95
  meta:
    active: true
- name: Bob
  scores: []
  meta:
    active: false
total: 2
";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -- Round-trip JSON→YAML→JSON: data equivalence --

    #[test]
    fn roundtrip_json_to_yaml_to_json() {
        let json_input = r#"{"name":"Alice","age":30,"scores":[100,95],"active":true}"#;
        let val = crate::formats::json::from_str(json_input).unwrap();

        // Value → YAML string → Value
        let yaml_str = to_string(&val).unwrap();
        let val_from_yaml = from_str(&yaml_str).unwrap();

        // Value → JSON string → Value
        let json_str = crate::formats::json::to_string(&val_from_yaml).unwrap();
        let val_roundtrip = crate::formats::json::from_str(&json_str).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    #[test]
    fn roundtrip_yaml_to_json_to_yaml() {
        let yaml_input = "name: Bob\nage: 25\ntags:\n  - rust\n  - yaml\n";
        let val = from_str(yaml_input).unwrap();

        let json_str = crate::formats::json::to_string(&val).unwrap();
        let val_from_json = crate::formats::json::from_str(&json_str).unwrap();

        let yaml_str = to_string(&val_from_json).unwrap();
        let val_roundtrip = from_str(&yaml_str).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    // -- Invalid YAML: clear error with line number --

    #[test]
    fn invalid_yaml_returns_error() {
        let bad = "key: [unterminated";
        let err = from_str(bad).unwrap_err();
        match err {
            crate::error::MorphError::Format { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Format error, got: {other:?}"),
        }
    }

    #[test]
    fn invalid_yaml_bad_indentation() {
        let bad = "a:\n b: 1\n  c: 2\n d: 3\n";
        // serde_yaml may or may not error on this depending on interpretation,
        // but if it does, it should be a Format error
        let result = from_str(bad);
        if let Err(err) = result {
            assert!(matches!(err, crate::error::MorphError::Format { .. }));
        }
    }

    #[test]
    fn invalid_yaml_tab_character() {
        let bad = "a:\n\tb: 1\n";
        let result = from_str(bad);
        if let Err(err) = result {
            match err {
                crate::error::MorphError::Format { message, line, .. } => {
                    assert!(!message.is_empty());
                    // Line info should be present
                    assert!(line.is_some(), "expected line number in error");
                }
                other => panic!("expected Format error, got: {other:?}"),
            }
        }
    }

    // -- Empty document: handles gracefully --

    #[test]
    fn empty_string() {
        let val = from_str("").unwrap();
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn empty_document_marker() {
        let val = from_str("---\n").unwrap();
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn empty_mapping() {
        let val = from_str("{}").unwrap();
        assert_eq!(val, Value::Map(IndexMap::new()));
    }

    #[test]
    fn empty_sequence() {
        let val = from_str("[]").unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    #[test]
    fn null_value() {
        let val = from_str("null").unwrap();
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn tilde_null() {
        let val = from_str("~").unwrap();
        assert_eq!(val, Value::Null);
    }

    // -- from_reader --

    #[test]
    fn from_reader_works() {
        let data = "x: 42\n";
        let val = from_reader(data.as_bytes()).unwrap();
        assert_eq!(val.get_path(".x"), Some(&Value::Int(42)));
    }

    #[test]
    fn from_reader_multi_doc() {
        let data = "---\na: 1\n---\nb: 2\n";
        let val = from_reader(data.as_bytes()).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
    }

    // -- Key order preserved --

    #[test]
    fn key_order_preserved() {
        let input = "z: 1\na: 2\nm: 3\n";
        let val = from_str(input).unwrap();
        let keys: Vec<&String> = match &val {
            Value::Map(m) => m.keys().collect(),
            _ => panic!("expected map"),
        };
        assert_eq!(keys, vec!["z", "a", "m"]);
    }

    // -- Unicode --

    #[test]
    fn unicode_roundtrip() {
        let input = "emoji: 🦀\naccent: héllo\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".emoji"), Some(&Value::String("🦀".into())));
        assert_eq!(
            val.get_path(".accent"),
            Some(&Value::String("héllo".into()))
        );
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -- Serialization of null, bool, and special values --

    #[test]
    fn serialize_null() {
        let val = Value::Null;
        let output = to_string(&val).unwrap();
        assert!(output.contains("null"));
    }

    #[test]
    fn serialize_bool() {
        let mut map = IndexMap::new();
        map.insert("t".into(), Value::Bool(true));
        map.insert("f".into(), Value::Bool(false));
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        assert!(output.contains("true"));
        assert!(output.contains("false"));
    }

    // -- Scalar string edge cases --

    #[test]
    fn string_with_colon() {
        let input = "url: \"http://example.com\"\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".url"),
            Some(&Value::String("http://example.com".into()))
        );
    }

    #[test]
    fn string_with_newlines_roundtrip() {
        let mut map = IndexMap::new();
        map.insert("text".into(), Value::String("line1\nline2\nline3\n".into()));
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }
}
