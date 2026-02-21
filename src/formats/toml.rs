use crate::error;
use crate::value::Value;
use indexmap::IndexMap;
use std::io::Read;

/// Parse a TOML string into a Universal Value.
pub fn from_str(input: &str) -> error::Result<Value> {
    let toml_val: toml::Value = toml::from_str(input)?;
    Ok(toml_to_value(toml_val))
}

/// Parse TOML from a reader into a Universal Value.
pub fn from_reader<R: Read>(mut reader: R) -> error::Result<Value> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    from_str(&buf)
}

/// Serialize a Universal Value to a TOML string.
pub fn to_string(value: &Value) -> error::Result<String> {
    let toml_val = value_to_toml(value)
        .ok_or_else(|| error::MorphError::format("TOML requires a top-level table (map)"))?;
    match &toml_val {
        toml::Value::Table(_) => {}
        _ => {
            return Err(error::MorphError::format(
                "TOML requires a top-level table (map)",
            ));
        }
    }
    let s =
        toml::to_string_pretty(&toml_val).map_err(|e| error::MorphError::format(e.to_string()))?;
    Ok(s)
}

fn toml_to_value(t: toml::Value) -> Value {
    match t {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Int(i),
        toml::Value::Float(f) => Value::Float(f),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_to_value).collect()),
        toml::Value::Table(table) => {
            let mut map = IndexMap::new();
            for (k, v) in table {
                map.insert(k, toml_to_value(v));
            }
            Value::Map(map)
        }
    }
}

fn value_to_toml(value: &Value) -> Option<toml::Value> {
    match value {
        Value::Null => Some(toml::Value::String("null".to_string())),
        Value::Bool(b) => Some(toml::Value::Boolean(*b)),
        Value::Int(i) => Some(toml::Value::Integer(*i)),
        Value::Float(f) => Some(toml::Value::Float(*f)),
        Value::String(s) => Some(toml::Value::String(s.clone())),
        Value::Bytes(b) => {
            // Represent as array of ints
            Some(toml::Value::Array(
                b.iter()
                    .map(|byte| toml::Value::Integer(*byte as i64))
                    .collect(),
            ))
        }
        Value::Array(arr) => {
            let toml_arr: Vec<toml::Value> = arr.iter().filter_map(value_to_toml).collect();
            Some(toml::Value::Array(toml_arr))
        }
        Value::Map(map) => {
            let mut table = toml::map::Map::new();
            for (k, v) in map {
                if let Some(tv) = value_to_toml(v) {
                    table.insert(k.clone(), tv);
                }
            }
            Some(toml::Value::Table(table))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Basic key-value pairs
    // -----------------------------------------------------------------------

    #[test]
    fn parse_simple_toml() {
        let input = "name = \"Alice\"\nage = 30\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("Alice".into())));
        assert_eq!(val.get_path(".age"), Some(&Value::Int(30)));
    }

    #[test]
    fn parse_types() {
        let input = "s = \"hello\"\ni = 42\nf = 3.15\nb = true\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".s"), Some(&Value::String("hello".into())));
        assert_eq!(val.get_path(".i"), Some(&Value::Int(42)));
        assert_eq!(val.get_path(".f"), Some(&Value::Float(3.15)));
        assert_eq!(val.get_path(".b"), Some(&Value::Bool(true)));
    }

    #[test]
    fn parse_negative_numbers() {
        let input = "neg_int = -7\nneg_float = -1.5\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".neg_int"), Some(&Value::Int(-7)));
        assert_eq!(val.get_path(".neg_float"), Some(&Value::Float(-1.5)));
    }

    #[test]
    fn roundtrip_basic() {
        let input = "key = \"value\"\nnum = 42\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Tables and nested tables
    // -----------------------------------------------------------------------

    #[test]
    fn parse_nested_toml() {
        let input = "[user]\nname = \"Bob\"\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".user.name"),
            Some(&Value::String("Bob".into()))
        );
    }

    #[test]
    fn deeply_nested_tables() {
        let input = "[a.b.c]\nd = \"deep\"\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".a.b.c.d"),
            Some(&Value::String("deep".into()))
        );
    }

    #[test]
    fn multiple_tables() {
        let input = "\
[database]
server = \"192.168.1.1\"
port = 5432

[owner]
name = \"Tom\"
";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".database.server"),
            Some(&Value::String("192.168.1.1".into()))
        );
        assert_eq!(val.get_path(".database.port"), Some(&Value::Int(5432)));
        assert_eq!(
            val.get_path(".owner.name"),
            Some(&Value::String("Tom".into()))
        );
    }

    #[test]
    fn roundtrip_nested_tables() {
        let input = "\
[server]
host = \"localhost\"
port = 8080

[server.ssl]
enabled = true
";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Arrays of tables
    // -----------------------------------------------------------------------

    #[test]
    fn array_of_tables() {
        let input = "\
[[products]]
name = \"Hammer\"
sku = 738594937

[[products]]
name = \"Nail\"
sku = 284758393
";
        let val = from_str(input).unwrap();
        let products = match val.get_path(".products") {
            Some(Value::Array(a)) => a,
            other => panic!("expected array, got: {other:?}"),
        };
        assert_eq!(products.len(), 2);
        assert_eq!(
            products[0].get_path(".name"),
            Some(&Value::String("Hammer".into()))
        );
        assert_eq!(products[0].get_path(".sku"), Some(&Value::Int(738594937)));
        assert_eq!(
            products[1].get_path(".name"),
            Some(&Value::String("Nail".into()))
        );
    }

    #[test]
    fn roundtrip_array_of_tables() {
        let input = "\
[[users]]
name = \"Alice\"
age = 30

[[users]]
name = \"Bob\"
age = 25
";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Inline tables
    // -----------------------------------------------------------------------

    #[test]
    fn inline_table() {
        let input = "point = { x = 1, y = 2 }\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".point.x"), Some(&Value::Int(1)));
        assert_eq!(val.get_path(".point.y"), Some(&Value::Int(2)));
    }

    #[test]
    fn inline_table_in_array() {
        let input = "points = [{ x = 1, y = 2 }, { x = 3, y = 4 }]\n";
        let val = from_str(input).unwrap();
        let points = match val.get_path(".points") {
            Some(Value::Array(a)) => a,
            other => panic!("expected array, got: {other:?}"),
        };
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].get_path(".x"), Some(&Value::Int(1)));
        assert_eq!(points[1].get_path(".y"), Some(&Value::Int(4)));
    }

    // -----------------------------------------------------------------------
    // TOML datetime → string conversion and back
    // -----------------------------------------------------------------------

    #[test]
    fn datetime_to_string() {
        let input = "dt = 2024-01-15T10:30:00Z\n";
        let val = from_str(input).unwrap();
        match val.get_path(".dt") {
            Some(Value::String(s)) => {
                assert!(s.contains("2024"));
                assert!(s.contains("01"));
                assert!(s.contains("15"));
            }
            other => panic!("expected string datetime, got: {other:?}"),
        }
    }

    #[test]
    fn local_datetime() {
        let input = "dt = 2024-01-15T10:30:00\n";
        let val = from_str(input).unwrap();
        match val.get_path(".dt") {
            Some(Value::String(s)) => {
                assert!(s.contains("2024-01-15"));
                assert!(s.contains("10:30:00"));
            }
            other => panic!("expected string, got: {other:?}"),
        }
    }

    #[test]
    fn local_date() {
        let input = "d = 2024-01-15\n";
        let val = from_str(input).unwrap();
        match val.get_path(".d") {
            Some(Value::String(s)) => {
                assert_eq!(s, "2024-01-15");
            }
            other => panic!("expected string, got: {other:?}"),
        }
    }

    #[test]
    fn local_time() {
        let input = "t = 10:30:00\n";
        let val = from_str(input).unwrap();
        match val.get_path(".t") {
            Some(Value::String(s)) => {
                assert_eq!(s, "10:30:00");
            }
            other => panic!("expected string, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Integer types: decimal, hex, octal, binary
    // -----------------------------------------------------------------------

    #[test]
    fn integer_decimal() {
        let input = "val = 1_000\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::Int(1000)));
    }

    #[test]
    fn integer_hex() {
        let input = "val = 0xDEAD_BEEF\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::Int(0xDEAD_BEEF)));
    }

    #[test]
    fn integer_octal() {
        let input = "val = 0o755\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::Int(0o755)));
    }

    #[test]
    fn integer_binary() {
        let input = "val = 0b1101_0110\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".val"), Some(&Value::Int(0b1101_0110)));
    }

    // -----------------------------------------------------------------------
    // Multiline strings
    // -----------------------------------------------------------------------

    #[test]
    fn multiline_basic_string() {
        let input = "text = \"\"\"\nline one\nline two\nline three\"\"\"\n";
        let val = from_str(input).unwrap();
        let text = match val.get_path(".text") {
            Some(Value::String(s)) => s.clone(),
            other => panic!("expected string, got: {other:?}"),
        };
        assert!(text.contains("line one"));
        assert!(text.contains("line two"));
        assert!(text.contains("line three"));
    }

    #[test]
    fn multiline_literal_string() {
        let input = "text = '''\nline one\nline two\n'''\n";
        let val = from_str(input).unwrap();
        let text = match val.get_path(".text") {
            Some(Value::String(s)) => s.clone(),
            other => panic!("expected string, got: {other:?}"),
        };
        assert!(text.contains("line one"));
        assert!(text.contains("line two"));
    }

    #[test]
    fn multiline_string_roundtrip() {
        // Strings with newlines should roundtrip through TOML
        let mut map = IndexMap::new();
        map.insert("text".into(), Value::String("hello\nworld\nfoo".into()));
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Round-trip YAML→TOML→YAML: data equivalence
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_yaml_to_toml_to_yaml() {
        let yaml_input = "name: Alice\nage: 30\nactive: true\n";
        let val = crate::formats::yaml::from_str(yaml_input).unwrap();

        let toml_str = to_string(&val).unwrap();
        let val_from_toml = from_str(&toml_str).unwrap();

        let yaml_str = crate::formats::yaml::to_string(&val_from_toml).unwrap();
        let val_roundtrip = crate::formats::yaml::from_str(&yaml_str).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    // -----------------------------------------------------------------------
    // Round-trip JSON→TOML→JSON: data equivalence
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_json_to_toml_to_json() {
        let json_input = r#"{"name":"Alice","age":30,"active":true}"#;
        let val = crate::formats::json::from_str(json_input).unwrap();

        let toml_str = to_string(&val).unwrap();
        let val_from_toml = from_str(&toml_str).unwrap();

        let json_str = crate::formats::json::to_string(&val_from_toml).unwrap();
        let val_roundtrip = crate::formats::json::from_str(&json_str).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    #[test]
    fn roundtrip_json_nested_to_toml_to_json() {
        let json_input = r#"{"server":{"host":"localhost","port":8080},"debug":false}"#;
        let val = crate::formats::json::from_str(json_input).unwrap();

        let toml_str = to_string(&val).unwrap();
        let val_from_toml = from_str(&toml_str).unwrap();

        let json_str = crate::formats::json::to_string(&val_from_toml).unwrap();
        let val_roundtrip = crate::formats::json::from_str(&json_str).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    // -----------------------------------------------------------------------
    // TOML constraint: top-level must be a table (error if array)
    // -----------------------------------------------------------------------

    #[test]
    fn toml_requires_top_level_table() {
        let val = Value::Array(vec![Value::Int(1)]);
        let result = to_string(&val);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("table") || msg.contains("map"), "msg: {msg}");
    }

    #[test]
    fn toml_rejects_scalar_at_top_level() {
        let val = Value::Int(42);
        let result = to_string(&val);
        assert!(result.is_err());
    }

    #[test]
    fn toml_rejects_string_at_top_level() {
        let val = Value::String("hello".into());
        let result = to_string(&val);
        assert!(result.is_err());
    }

    #[test]
    fn toml_rejects_bool_at_top_level() {
        let val = Value::Bool(true);
        let result = to_string(&val);
        assert!(result.is_err());
    }

    #[test]
    fn toml_rejects_null_at_top_level() {
        let val = Value::Null;
        let result = to_string(&val);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Invalid TOML: clear error with line number
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_toml_returns_error() {
        let bad = "[broken\nkey = ";
        let err = from_str(bad).unwrap_err();
        match err {
            crate::error::MorphError::Format { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Format error, got: {other:?}"),
        }
    }

    #[test]
    fn invalid_toml_duplicate_key() {
        let bad = "name = \"a\"\nname = \"b\"\n";
        let err = from_str(bad).unwrap_err();
        assert!(matches!(err, crate::error::MorphError::Format { .. }));
    }

    #[test]
    fn invalid_toml_bad_value() {
        let bad = "key = @invalid\n";
        let err = from_str(bad).unwrap_err();
        assert!(matches!(err, crate::error::MorphError::Format { .. }));
    }

    // -----------------------------------------------------------------------
    // from_reader
    // -----------------------------------------------------------------------

    #[test]
    fn from_reader_works() {
        let data = "x = 42\n";
        let val = from_reader(data.as_bytes()).unwrap();
        assert_eq!(val.get_path(".x"), Some(&Value::Int(42)));
    }

    #[test]
    fn from_reader_nested() {
        let data = "[server]\nhost = \"localhost\"\nport = 8080\n";
        let val = from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            val.get_path(".server.host"),
            Some(&Value::String("localhost".into()))
        );
        assert_eq!(val.get_path(".server.port"), Some(&Value::Int(8080)));
    }

    // -----------------------------------------------------------------------
    // Empty and edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn empty_toml() {
        let val = from_str("").unwrap();
        assert_eq!(val, Value::Map(IndexMap::new()));
    }

    #[test]
    fn empty_table() {
        let input = "[empty]\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".empty"), Some(&Value::Map(IndexMap::new())));
    }

    #[test]
    fn empty_array() {
        let input = "arr = []\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".arr"), Some(&Value::Array(vec![])));
    }

    // -----------------------------------------------------------------------
    // Unicode and special strings
    // -----------------------------------------------------------------------

    #[test]
    fn unicode_roundtrip() {
        let input = "emoji = \"🦀\"\naccent = \"héllo\"\n";
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

    // -----------------------------------------------------------------------
    // Null representation
    // -----------------------------------------------------------------------

    #[test]
    fn null_serializes_as_string() {
        let mut map = IndexMap::new();
        map.insert("val".into(), Value::Null);
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        assert!(output.contains("null"));
        let val2 = from_str(&output).unwrap();
        // Null becomes string "null" in TOML since TOML has no null type
        assert_eq!(val2.get_path(".val"), Some(&Value::String("null".into())));
    }

    // -----------------------------------------------------------------------
    // Complex roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_complex() {
        let input = "\
title = \"TOML Example\"
debug = false

[database]
server = \"192.168.1.1\"
ports = [8001, 8001, 8002]
enabled = true

[servers.alpha]
ip = \"10.0.0.1\"
dc = \"eqdc10\"

[servers.beta]
ip = \"10.0.0.2\"
dc = \"eqdc10\"
";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Mixed arrays
    // -----------------------------------------------------------------------

    #[test]
    fn array_of_integers() {
        let input = "arr = [1, 2, 3]\n";
        let val = from_str(input).unwrap();
        let arr = match val.get_path(".arr") {
            Some(Value::Array(a)) => a,
            other => panic!("expected array, got: {other:?}"),
        };
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], Value::Int(1));
        assert_eq!(arr[2], Value::Int(3));
    }

    #[test]
    fn array_of_strings() {
        let input = "colors = [\"red\", \"green\", \"blue\"]\n";
        let val = from_str(input).unwrap();
        let arr = match val.get_path(".colors") {
            Some(Value::Array(a)) => a,
            other => panic!("expected array, got: {other:?}"),
        };
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], Value::String("red".into()));
    }

    #[test]
    fn nested_arrays() {
        let input = "arr = [[1, 2], [3, 4]]\n";
        let val = from_str(input).unwrap();
        let arr = match val.get_path(".arr") {
            Some(Value::Array(a)) => a,
            other => panic!("expected array, got: {other:?}"),
        };
        assert_eq!(arr.len(), 2);
        match &arr[0] {
            Value::Array(inner) => {
                assert_eq!(inner.len(), 2);
                assert_eq!(inner[0], Value::Int(1));
            }
            other => panic!("expected inner array, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Float edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn float_inf_nan() {
        let input = "pos_inf = inf\nneg_inf = -inf\nnot_a_num = nan\n";
        let val = from_str(input).unwrap();
        match val.get_path(".pos_inf") {
            Some(Value::Float(f)) => assert!(f.is_infinite() && *f > 0.0),
            other => panic!("expected +inf, got: {other:?}"),
        }
        match val.get_path(".neg_inf") {
            Some(Value::Float(f)) => assert!(f.is_infinite() && *f < 0.0),
            other => panic!("expected -inf, got: {other:?}"),
        }
        match val.get_path(".not_a_num") {
            Some(Value::Float(f)) => assert!(f.is_nan()),
            other => panic!("expected nan, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Dotted keys
    // -----------------------------------------------------------------------

    #[test]
    fn dotted_keys() {
        let input = "fruit.apple.color = \"red\"\nfruit.apple.taste = \"sweet\"\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".fruit.apple.color"),
            Some(&Value::String("red".into()))
        );
        assert_eq!(
            val.get_path(".fruit.apple.taste"),
            Some(&Value::String("sweet".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Quoted keys
    // -----------------------------------------------------------------------

    #[test]
    fn quoted_keys() {
        let input = "\"key with spaces\" = \"value\"\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".key with spaces"),
            Some(&Value::String("value".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Boolean values
    // -----------------------------------------------------------------------

    #[test]
    fn booleans() {
        let input = "t = true\nf = false\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".t"), Some(&Value::Bool(true)));
        assert_eq!(val.get_path(".f"), Some(&Value::Bool(false)));
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }
}
