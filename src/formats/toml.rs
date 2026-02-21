use crate::error;
use crate::value::Value;
use indexmap::IndexMap;

/// Parse a TOML string into a Universal Value.
pub fn from_str(input: &str) -> error::Result<Value> {
    let toml_val: toml::Value = toml::from_str(input)?;
    Ok(toml_to_value(toml_val))
}

/// Serialize a Universal Value to a TOML string.
pub fn to_string(value: &Value) -> error::Result<String> {
    let toml_val = value_to_toml(value)
        .ok_or_else(|| error::MorphError::format("TOML requires a top-level table (map)"))?;
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

    #[test]
    fn parse_simple_toml() {
        let input = "name = \"Alice\"\nage = 30\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("Alice".into())));
        assert_eq!(val.get_path(".age"), Some(&Value::Int(30)));
    }

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
    fn roundtrip_toml() {
        let input = "key = \"value\"\nnum = 42\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn toml_requires_top_level_table() {
        // TOML must have a top-level table; an array at root should error on serialization
        let val = Value::Array(vec![Value::Int(1)]);
        // value_to_toml produces Some(Array(...)) but toml::to_string_pretty
        // will reject it because TOML top-level must be a table.
        let result = to_string(&val);
        assert!(result.is_err());
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
}
