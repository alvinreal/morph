use crate::error;
use crate::value::Value;
use indexmap::IndexMap;

/// Parse a YAML string into a Universal Value.
pub fn from_str(input: &str) -> error::Result<Value> {
    let yaml_val: serde_yaml::Value = serde_yaml::from_str(input)?;
    Ok(yaml_to_value(yaml_val))
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
            for (k, v) in map {
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

    #[test]
    fn parse_simple_yaml() {
        let input = "name: Alice\nage: 30\n";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("Alice".into())));
        assert_eq!(val.get_path(".age"), Some(&Value::Int(30)));
    }

    #[test]
    fn parse_nested_yaml() {
        let input = "user:\n  name: Bob\n  scores:\n    - 1\n    - 2\n";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".user.name"),
            Some(&Value::String("Bob".into()))
        );
        assert_eq!(val.get_path(".user.scores[0]"), Some(&Value::Int(1)));
    }

    #[test]
    fn roundtrip_yaml() {
        let input = "key: value\nnum: 42\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
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
    fn empty_mapping() {
        let val = from_str("{}").unwrap();
        assert_eq!(val, Value::Map(IndexMap::new()));
    }
}
