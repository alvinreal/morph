use crate::error;
use crate::value::Value;
use indexmap::IndexMap;
use std::io::Read;

/// Parse CSV data from a string into a Universal Value.
///
/// Returns an Array of Maps, where each map represents a row with header keys.
pub fn from_str(input: &str) -> error::Result<Value> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(input.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| error::MorphError::format(e.to_string()))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let mut map = IndexMap::new();
        for (i, field) in record.iter().enumerate() {
            let key = headers
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("column_{i}"));
            map.insert(key, parse_csv_field(field));
        }
        rows.push(Value::Map(map));
    }

    Ok(Value::Array(rows))
}

/// Serialize a Universal Value to a CSV string.
///
/// Expects the value to be an Array of Maps. All maps should have the same
/// keys (the union of keys from the first row is used as the header).
pub fn to_string(value: &Value) -> error::Result<String> {
    let rows = match value {
        Value::Array(arr) => arr,
        _ => {
            return Err(error::MorphError::format(
                "CSV output requires an array of objects",
            ))
        }
    };

    if rows.is_empty() {
        return Ok(String::new());
    }

    // Collect headers from the first row
    let headers: Vec<String> = match &rows[0] {
        Value::Map(map) => map.keys().cloned().collect(),
        _ => return Err(error::MorphError::format("CSV rows must be objects (maps)")),
    };

    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(&headers)
        .map_err(|e| error::MorphError::format(e.to_string()))?;

    for row in rows {
        match row {
            Value::Map(map) => {
                let fields: Vec<String> = headers
                    .iter()
                    .map(|h| map.get(h).map(csv_field_to_string).unwrap_or_default())
                    .collect();
                wtr.write_record(&fields)
                    .map_err(|e| error::MorphError::format(e.to_string()))?;
            }
            _ => {
                return Err(error::MorphError::format("CSV rows must be objects (maps)"));
            }
        }
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| error::MorphError::format(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| error::MorphError::format(e.to_string()))
}

/// Read CSV from a reader.
pub fn from_reader<R: Read>(reader: R) -> error::Result<Value> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(reader);

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| error::MorphError::format(e.to_string()))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let mut map = IndexMap::new();
        for (i, field) in record.iter().enumerate() {
            let key = headers
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("column_{i}"));
            map.insert(key, parse_csv_field(field));
        }
        rows.push(Value::Map(map));
    }

    Ok(Value::Array(rows))
}

/// Try to parse a CSV field into the most specific type.
fn parse_csv_field(field: &str) -> Value {
    if field.is_empty() {
        return Value::String(String::new());
    }

    // Try bool
    match field {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        _ => {}
    }

    // Try int
    if let Ok(i) = field.parse::<i64>() {
        return Value::Int(i);
    }

    // Try float
    if let Ok(f) = field.parse::<f64>() {
        return Value::Float(f);
    }

    Value::String(field.to_string())
}

fn csv_field_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Bytes(b) => format!("{b:?}"),
        Value::Array(_) | Value::Map(_) => {
            // Flatten complex types to JSON for CSV cells
            serde_json::to_string(&crate::formats::json::to_string(value).unwrap_or_default())
                .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_csv() {
        let input = "name,age\nAlice,30\nBob,25\n";
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
        assert_eq!(arr[0].get_path(".age"), Some(&Value::Int(30)));
        assert_eq!(arr[1].get_path(".name"), Some(&Value::String("Bob".into())));
    }

    #[test]
    fn roundtrip_csv() {
        let input = "name,age\nAlice,30\nBob,25\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn csv_type_detection() {
        let input = "s,i,f,b\nhello,42,3.15,true\n";
        let val = from_str(input).unwrap();
        let row = match &val {
            Value::Array(a) => &a[0],
            _ => panic!("expected array"),
        };
        assert_eq!(row.get_path(".s"), Some(&Value::String("hello".into())));
        assert_eq!(row.get_path(".i"), Some(&Value::Int(42)));
        assert_eq!(row.get_path(".f"), Some(&Value::Float(3.15)));
        assert_eq!(row.get_path(".b"), Some(&Value::Bool(true)));
    }

    #[test]
    fn csv_non_array_error() {
        let val = Value::Map(IndexMap::new());
        let result = to_string(&val);
        assert!(result.is_err());
    }

    #[test]
    fn csv_empty_input() {
        // A CSV with just headers and no data rows
        let input = "name,age\n";
        let val = from_str(input).unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }
}
