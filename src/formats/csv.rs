use crate::error;
use crate::value::Value;
use indexmap::IndexMap;
use std::io::Read;

/// CSV reader/writer configuration.
#[derive(Debug, Clone)]
pub struct CsvConfig {
    /// Whether the first row is a header row.
    pub has_headers: bool,
    /// Field delimiter byte.
    pub delimiter: u8,
    /// Whether to allow rows with differing column counts.
    pub flexible: bool,
}

impl Default for CsvConfig {
    fn default() -> Self {
        Self {
            has_headers: true,
            delimiter: b',',
            flexible: false,
        }
    }
}

/// Parse CSV data from a string into a Universal Value.
///
/// Returns an Array of Maps, where each map represents a row with header keys.
pub fn from_str(input: &str) -> error::Result<Value> {
    from_str_with_config(input, &CsvConfig::default())
}

/// Parse CSV data from a string with custom configuration.
pub fn from_str_with_config(input: &str, config: &CsvConfig) -> error::Result<Value> {
    from_reader_with_config(input.as_bytes(), config)
}

/// Read CSV from a reader.
pub fn from_reader<R: Read>(reader: R) -> error::Result<Value> {
    from_reader_with_config(reader, &CsvConfig::default())
}

/// Read CSV from a reader with custom configuration.
pub fn from_reader_with_config<R: Read>(reader: R, config: &CsvConfig) -> error::Result<Value> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(config.has_headers)
        .delimiter(config.delimiter)
        .flexible(config.flexible)
        .from_reader(reader);

    if config.has_headers {
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
    } else {
        // No headers: return array of arrays
        let mut rows = Vec::new();
        for result in rdr.records() {
            let record = result?;
            let row: Vec<Value> = record.iter().map(parse_csv_field).collect();
            rows.push(Value::Array(row));
        }
        Ok(Value::Array(rows))
    }
}

/// Serialize a Universal Value to a CSV string.
///
/// Expects the value to be an Array of Maps. All maps should have the same
/// keys (the union of keys from the first row is used as the header).
pub fn to_string(value: &Value) -> error::Result<String> {
    to_string_with_config(value, &CsvConfig::default())
}

/// Parse CSV with explicit user-provided headers.
///
/// Ignores the first row of the input (if `config.has_headers` is true, it
/// is treated as a header but replaced). If `config.has_headers` is false,
/// all rows are treated as data rows.
pub fn from_str_with_explicit_headers(
    input: &str,
    config: &CsvConfig,
    header_str: &str,
) -> error::Result<Value> {
    let headers: Vec<String> = header_str
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Force no-headers mode so the CSV reader doesn't consume the first line
    let mut no_header_config = config.clone();
    no_header_config.has_headers = false;

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(no_header_config.delimiter)
        .flexible(no_header_config.flexible)
        .from_reader(input.as_bytes());

    let mut rows = Vec::new();
    let mut first = true;
    for result in rdr.records() {
        let record = result?;
        // If the original config had headers, skip the first row (it's the original header)
        if first && config.has_headers {
            first = false;
            continue;
        }
        first = false;
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

/// Serialize a Universal Value to a CSV string with custom configuration.
pub fn to_string_with_config(value: &Value, config: &CsvConfig) -> error::Result<String> {
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

    let mut wtr = csv::WriterBuilder::new()
        .delimiter(config.delimiter)
        .from_writer(Vec::new());

    // Check if we have an array of maps or array of arrays
    match &rows[0] {
        Value::Map(_) => {
            // Collect all headers from all rows (union) preserving order from first row
            let mut header_set = IndexMap::new();
            for row in rows {
                if let Value::Map(map) = row {
                    for key in map.keys() {
                        header_set.entry(key.clone()).or_insert(());
                    }
                }
            }
            let headers: Vec<String> = header_set.keys().cloned().collect();

            if config.has_headers {
                wtr.write_record(&headers)
                    .map_err(|e| error::MorphError::format(e.to_string()))?;
            }

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
        }
        Value::Array(_) => {
            // Array of arrays: write directly without headers
            for row in rows {
                match row {
                    Value::Array(arr) => {
                        let fields: Vec<String> = arr.iter().map(csv_field_to_string).collect();
                        wtr.write_record(&fields)
                            .map_err(|e| error::MorphError::format(e.to_string()))?;
                    }
                    _ => {
                        return Err(error::MorphError::format(
                            "CSV rows must be consistent (all maps or all arrays)",
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(error::MorphError::format(
                "CSV rows must be objects (maps) or arrays",
            ));
        }
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| error::MorphError::format(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| error::MorphError::format(e.to_string()))
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
        Value::Float(f) => {
            let s = f.to_string();
            // Ensure float representation always contains a decimal point
            // so that CSV type inference re-parses it as Float, not Int.
            if s.contains('.')
                || s.contains('e')
                || s.contains('E')
                || s.contains("inf")
                || s.contains("NaN")
            {
                s
            } else {
                format!("{s}.0")
            }
        }
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

    // -----------------------------------------------------------------------
    // Basic CSV: header + rows → array of maps
    // -----------------------------------------------------------------------

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
    fn parse_single_row() {
        let input = "x,y\n1,2\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get_path(".x"), Some(&Value::Int(1)));
        assert_eq!(arr[0].get_path(".y"), Some(&Value::Int(2)));
    }

    #[test]
    fn parse_many_columns() {
        let input = "a,b,c,d,e\n1,2,3,4,5\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr[0].get_path(".a"), Some(&Value::Int(1)));
        assert_eq!(arr[0].get_path(".e"), Some(&Value::Int(5)));
    }

    // -----------------------------------------------------------------------
    // No header mode: array of arrays
    // -----------------------------------------------------------------------

    #[test]
    fn no_header_mode() {
        let input = "Alice,30\nBob,25\n";
        let config = CsvConfig {
            has_headers: false,
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
        match &arr[0] {
            Value::Array(row) => {
                assert_eq!(row[0], Value::String("Alice".into()));
                assert_eq!(row[1], Value::Int(30));
            }
            other => panic!("expected inner array, got: {other:?}"),
        }
    }

    #[test]
    fn no_header_single_column() {
        let input = "hello\nworld\n";
        let config = CsvConfig {
            has_headers: false,
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
        match &arr[0] {
            Value::Array(row) => assert_eq!(row[0], Value::String("hello".into())),
            other => panic!("expected inner array, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Custom delimiter: TSV (tab), pipe-separated
    // -----------------------------------------------------------------------

    #[test]
    fn tab_separated() {
        let input = "name\tage\nAlice\t30\nBob\t25\n";
        let config = CsvConfig {
            delimiter: b'\t',
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
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
    }

    #[test]
    fn pipe_separated() {
        let input = "name|age\nAlice|30\n";
        let config = CsvConfig {
            delimiter: b'|',
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(
            arr[0].get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(arr[0].get_path(".age"), Some(&Value::Int(30)));
    }

    #[test]
    fn semicolon_separated() {
        let input = "a;b\n1;2\n";
        let config = CsvConfig {
            delimiter: b';',
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr[0].get_path(".a"), Some(&Value::Int(1)));
        assert_eq!(arr[0].get_path(".b"), Some(&Value::Int(2)));
    }

    #[test]
    fn tsv_roundtrip() {
        let input = "name\tage\nAlice\t30\nBob\t25\n";
        let config = CsvConfig {
            delimiter: b'\t',
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let output = to_string_with_config(&val, &config).unwrap();
        let val2 = from_str_with_config(&output, &config).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Quoted fields: fields with commas, quotes, newlines inside quotes
    // -----------------------------------------------------------------------

    #[test]
    fn quoted_field_with_comma() {
        let input = "name,address\nAlice,\"123 Main St, Apt 4\"\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(
            arr[0].get_path(".address"),
            Some(&Value::String("123 Main St, Apt 4".into()))
        );
    }

    #[test]
    fn quoted_field_with_quotes() {
        let input = "name,quote\nAlice,\"She said \"\"hello\"\"\"\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(
            arr[0].get_path(".quote"),
            Some(&Value::String("She said \"hello\"".into()))
        );
    }

    #[test]
    fn quoted_field_with_newline() {
        let input = "name,bio\nAlice,\"line one\nline two\"\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(
            arr[0].get_path(".bio"),
            Some(&Value::String("line one\nline two".into()))
        );
    }

    #[test]
    fn quoted_field_roundtrip() {
        // Value with commas should be properly quoted in output and roundtrip
        let mut map = IndexMap::new();
        map.insert("name".into(), Value::String("Alice".into()));
        map.insert("address".into(), Value::String("123 Main, Apt 4".into()));
        let val = Value::Array(vec![Value::Map(map)]);
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Type inference: "42" → Int, "3.14" → Float, "true" → Bool
    // -----------------------------------------------------------------------

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
    fn type_detection_negative_numbers() {
        let input = "a,b\n-7,-1.5\n";
        let val = from_str(input).unwrap();
        let row = match &val {
            Value::Array(a) => &a[0],
            _ => panic!("expected array"),
        };
        assert_eq!(row.get_path(".a"), Some(&Value::Int(-7)));
        assert_eq!(row.get_path(".b"), Some(&Value::Float(-1.5)));
    }

    #[test]
    fn type_detection_false() {
        let input = "val\nfalse\n";
        let val = from_str(input).unwrap();
        let row = match &val {
            Value::Array(a) => &a[0],
            _ => panic!("expected array"),
        };
        assert_eq!(row.get_path(".val"), Some(&Value::Bool(false)));
    }

    // -----------------------------------------------------------------------
    // Empty fields: empty string representation
    // -----------------------------------------------------------------------

    #[test]
    fn empty_fields() {
        let input = "a,b,c\n1,,3\n";
        let config = CsvConfig {
            flexible: true,
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let row = match &val {
            Value::Array(a) => &a[0],
            _ => panic!("expected array"),
        };
        assert_eq!(row.get_path(".a"), Some(&Value::Int(1)));
        assert_eq!(row.get_path(".b"), Some(&Value::String("".into())));
        assert_eq!(row.get_path(".c"), Some(&Value::Int(3)));
    }

    #[test]
    fn all_empty_fields() {
        let input = "a,b\n,\n";
        let val = from_str(input).unwrap();
        let row = match &val {
            Value::Array(a) => &a[0],
            _ => panic!("expected array"),
        };
        assert_eq!(row.get_path(".a"), Some(&Value::String("".into())));
        assert_eq!(row.get_path(".b"), Some(&Value::String("".into())));
    }

    // -----------------------------------------------------------------------
    // Round-trip CSV→JSON→CSV: headers and data preserved
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_csv() {
        let input = "name,age\nAlice,30\nBob,25\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_csv_to_json_to_csv() {
        let csv_input = "name,age,active\nAlice,30,true\nBob,25,false\n";
        let val = from_str(csv_input).unwrap();

        // CSV → JSON
        let json_str = crate::formats::json::to_string(&val).unwrap();
        let val_from_json = crate::formats::json::from_str(&json_str).unwrap();

        // JSON → CSV
        let csv_output = to_string(&val_from_json).unwrap();
        let val_roundtrip = from_str(&csv_output).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    #[test]
    fn roundtrip_json_to_csv_to_json() {
        let json_input = r#"[{"name":"Alice","score":95},{"name":"Bob","score":87}]"#;
        let val = crate::formats::json::from_str(json_input).unwrap();

        let csv_str = to_string(&val).unwrap();
        let val_from_csv = from_str(&csv_str).unwrap();

        let json_str = crate::formats::json::to_string(&val_from_csv).unwrap();
        let val_roundtrip = crate::formats::json::from_str(&json_str).unwrap();

        assert_eq!(val, val_roundtrip);
    }

    // -----------------------------------------------------------------------
    // Large CSV: many rows parse correctly
    // -----------------------------------------------------------------------

    #[test]
    fn large_csv() {
        let mut csv = String::from("id,value\n");
        for i in 0..1000 {
            csv.push_str(&format!("{i},val_{i}\n"));
        }
        let val = from_str(&csv).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 1000);
        assert_eq!(arr[0].get_path(".id"), Some(&Value::Int(0)));
        assert_eq!(
            arr[0].get_path(".value"),
            Some(&Value::String("val_0".into()))
        );
        assert_eq!(arr[999].get_path(".id"), Some(&Value::Int(999)));
    }

    // -----------------------------------------------------------------------
    // Unicode in CSV: non-ASCII headers and values
    // -----------------------------------------------------------------------

    #[test]
    fn unicode_headers_and_values() {
        let input = "名前,年齢\nアリス,30\nボブ,25\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
        assert_eq!(
            arr[0].get_path(".名前"),
            Some(&Value::String("アリス".into()))
        );
        assert_eq!(arr[0].get_path(".年齢"), Some(&Value::Int(30)));
    }

    #[test]
    fn unicode_roundtrip() {
        let input = "emoji,text\n🦀,héllo\n🎉,wörld\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Ragged rows: rows with different column counts → error
    // -----------------------------------------------------------------------

    #[test]
    fn ragged_rows_error() {
        let input = "a,b\n1,2,3\n";
        let result = from_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn ragged_rows_flexible() {
        let input = "a,b\n1,2,3\n4\n";
        let config = CsvConfig {
            flexible: true,
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 2);
        // First row has extra column mapped to column_2
        assert_eq!(arr[0].get_path(".a"), Some(&Value::Int(1)));
        assert_eq!(arr[0].get_path(".b"), Some(&Value::Int(2)));
        assert_eq!(arr[0].get_path(".column_2"), Some(&Value::Int(3)));
        // Second row has only one column
        assert_eq!(arr[1].get_path(".a"), Some(&Value::Int(4)));
    }

    // -----------------------------------------------------------------------
    // Write from JSON: array of objects → CSV with consistent headers
    // -----------------------------------------------------------------------

    #[test]
    fn write_from_json_objects() {
        let json_input = r#"[{"name":"Alice","age":30},{"name":"Bob","age":25}]"#;
        let val = crate::formats::json::from_str(json_input).unwrap();
        let csv_output = to_string(&val).unwrap();

        assert!(csv_output.contains("name"));
        assert!(csv_output.contains("age"));
        assert!(csv_output.contains("Alice"));
        assert!(csv_output.contains("30"));
        assert!(csv_output.contains("Bob"));
    }

    #[test]
    fn write_union_headers() {
        // When rows have different keys, all keys should appear as headers
        let mut map1 = IndexMap::new();
        map1.insert("a".into(), Value::Int(1));
        map1.insert("b".into(), Value::Int(2));
        let mut map2 = IndexMap::new();
        map2.insert("b".into(), Value::Int(3));
        map2.insert("c".into(), Value::Int(4));
        let val = Value::Array(vec![Value::Map(map1), Value::Map(map2)]);
        let output = to_string(&val).unwrap();

        // Headers should include a, b, c
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].contains("a"));
        assert!(lines[0].contains("b"));
        assert!(lines[0].contains("c"));
    }

    // -----------------------------------------------------------------------
    // Error cases
    // -----------------------------------------------------------------------

    #[test]
    fn csv_non_array_error() {
        let val = Value::Map(IndexMap::new());
        let result = to_string(&val);
        assert!(result.is_err());
    }

    #[test]
    fn csv_scalar_error() {
        let val = Value::Int(42);
        let result = to_string(&val);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn csv_empty_input() {
        let input = "name,age\n";
        let val = from_str(input).unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    #[test]
    fn csv_empty_string() {
        let input = "";
        let val = from_str(input).unwrap();
        assert_eq!(val, Value::Array(vec![]));
    }

    #[test]
    fn csv_empty_output() {
        let val = Value::Array(vec![]);
        let output = to_string(&val).unwrap();
        assert!(output.is_empty());
    }

    // -----------------------------------------------------------------------
    // from_reader
    // -----------------------------------------------------------------------

    #[test]
    fn from_reader_works() {
        let data = "name,age\nAlice,30\n";
        let val = from_reader(data.as_bytes()).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr.len(), 1);
        assert_eq!(
            arr[0].get_path(".name"),
            Some(&Value::String("Alice".into()))
        );
    }

    #[test]
    fn from_reader_with_tsv_config() {
        let data = "name\tage\nBob\t25\n";
        let config = CsvConfig {
            delimiter: b'\t',
            ..Default::default()
        };
        let val = from_reader_with_config(data.as_bytes(), &config).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(arr[0].get_path(".name"), Some(&Value::String("Bob".into())));
        assert_eq!(arr[0].get_path(".age"), Some(&Value::Int(25)));
    }

    // -----------------------------------------------------------------------
    // Null serialization
    // -----------------------------------------------------------------------

    #[test]
    fn null_serializes_as_empty() {
        let mut map = IndexMap::new();
        map.insert("a".into(), Value::Int(1));
        map.insert("b".into(), Value::Null);
        map.insert("c".into(), Value::Int(3));
        let val = Value::Array(vec![Value::Map(map)]);
        let output = to_string(&val).unwrap();
        // Null should become empty field
        assert!(output.contains("1,,3") || output.contains("1,\"\",3"));
    }

    // -----------------------------------------------------------------------
    // Bool serialization roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn bool_roundtrip() {
        let input = "val\ntrue\nfalse\n";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Header order preserved
    // -----------------------------------------------------------------------

    #[test]
    fn header_order_preserved() {
        let input = "z,a,m\n1,2,3\n";
        let val = from_str(input).unwrap();
        let row = match &val {
            Value::Array(a) => &a[0],
            _ => panic!("expected array"),
        };
        let keys: Vec<&String> = match row {
            Value::Map(m) => m.keys().collect(),
            _ => panic!("expected map"),
        };
        assert_eq!(keys, vec!["z", "a", "m"]);
    }

    // -----------------------------------------------------------------------
    // Whitespace handling
    // -----------------------------------------------------------------------

    #[test]
    fn whitespace_in_fields() {
        let input = "name,value\n\" Alice \",\" hello \"\n";
        let val = from_str(input).unwrap();
        let arr = match &val {
            Value::Array(a) => a,
            _ => panic!("expected array"),
        };
        assert_eq!(
            arr[0].get_path(".name"),
            Some(&Value::String(" Alice ".into()))
        );
        assert_eq!(
            arr[0].get_path(".value"),
            Some(&Value::String(" hello ".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Array of arrays output
    // -----------------------------------------------------------------------

    #[test]
    fn write_array_of_arrays() {
        let val = Value::Array(vec![
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
            Value::Array(vec![Value::Int(4), Value::Int(5), Value::Int(6)]),
        ]);
        let output = to_string(&val).unwrap();
        assert!(output.contains("1,2,3"));
        assert!(output.contains("4,5,6"));
    }

    #[test]
    fn no_header_roundtrip() {
        let config = CsvConfig {
            has_headers: false,
            ..Default::default()
        };
        let val = Value::Array(vec![
            Value::Array(vec![Value::String("Alice".into()), Value::Int(30)]),
            Value::Array(vec![Value::String("Bob".into()), Value::Int(25)]),
        ]);
        let output = to_string_with_config(&val, &config).unwrap();
        let val2 = from_str_with_config(&output, &config).unwrap();
        assert_eq!(val, val2);
    }
}
