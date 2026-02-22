//! Streaming processing for large files.
//!
//! Provides element-by-element processing for JSON arrays, JSONL, and CSV
//! so morph can handle files larger than available RAM.

use crate::cli::{Cli, Format};
use crate::error;
use crate::mapping;
use crate::value::Value;
use std::io::{self, BufRead, BufReader, Read, Write};

/// Formats that support streaming input.
pub fn supports_streaming_input(format: Format) -> bool {
    matches!(format, Format::Jsonl | Format::Csv | Format::Json)
}

/// Formats that support streaming output.
pub fn supports_streaming_output(format: Format) -> bool {
    matches!(format, Format::Jsonl | Format::Csv | Format::Json)
}

/// Check if a conversion pipeline can be streamed.
pub fn can_stream(in_fmt: Format, out_fmt: Format) -> bool {
    supports_streaming_input(in_fmt) && supports_streaming_output(out_fmt)
}

/// A streaming writer that outputs elements one at a time.
pub struct StreamWriter<W: Write> {
    writer: W,
    format: Format,
    csv_config: crate::formats::csv::CsvConfig,
    count: usize,
    csv_headers: Option<Vec<String>>,
}

impl<W: Write> StreamWriter<W> {
    pub fn new(writer: W, format: Format, csv_config: crate::formats::csv::CsvConfig) -> Self {
        Self {
            writer,
            format,
            csv_config,
            count: 0,
            csv_headers: None,
        }
    }

    /// Write the opening delimiter for array-based formats.
    pub fn begin(&mut self) -> error::Result<()> {
        if self.format == Format::Json {
            writeln!(self.writer, "[")?;
        }
        Ok(())
    }

    /// Write a single element.
    pub fn write_element(&mut self, value: &Value) -> error::Result<()> {
        match self.format {
            Format::Jsonl => {
                let json_val = crate::formats::json::value_to_json(value);
                let line = serde_json::to_string(&json_val)
                    .map_err(|e| error::MorphError::format(e.to_string()))?;
                writeln!(self.writer, "{line}")?;
            }
            Format::Json => {
                let json_val = crate::formats::json::value_to_json(value);
                let line = serde_json::to_string(&json_val)
                    .map_err(|e| error::MorphError::format(e.to_string()))?;
                if self.count > 0 {
                    writeln!(self.writer, ",")?;
                }
                write!(self.writer, "  {line}")?;
            }
            Format::Csv => {
                self.write_csv_element(value)?;
            }
            _ => {
                return Err(error::MorphError::format(format!(
                    "streaming output not supported for {}",
                    self.format
                )));
            }
        }
        self.count += 1;
        Ok(())
    }

    fn write_csv_element(&mut self, value: &Value) -> error::Result<()> {
        match value {
            Value::Map(map) => {
                // First element: determine and write headers
                if self.csv_headers.is_none() {
                    let headers: Vec<String> = map.keys().cloned().collect();
                    if self.csv_config.has_headers {
                        let header_line =
                            headers.join(&String::from(self.csv_config.delimiter as char));
                        writeln!(self.writer, "{header_line}")?;
                    }
                    self.csv_headers = Some(headers);
                }
                let headers = self.csv_headers.as_ref().unwrap();
                let fields: Vec<String> = headers
                    .iter()
                    .map(|h| map.get(h).map(csv_field_to_string).unwrap_or_default())
                    .collect();
                let line = csv_escape_record(&fields, self.csv_config.delimiter);
                writeln!(self.writer, "{line}")?;
            }
            Value::Array(arr) => {
                let fields: Vec<String> = arr.iter().map(csv_field_to_string).collect();
                let line = csv_escape_record(&fields, self.csv_config.delimiter);
                writeln!(self.writer, "{line}")?;
            }
            _ => {
                return Err(error::MorphError::format(
                    "CSV streaming requires map or array elements",
                ));
            }
        }
        Ok(())
    }

    /// Write the closing delimiter for array-based formats and flush.
    pub fn end(&mut self) -> error::Result<()> {
        if self.format == Format::Json {
            if self.count > 0 {
                writeln!(self.writer)?;
            }
            writeln!(self.writer, "]")?;
        }
        self.writer.flush()?;
        Ok(())
    }
}

/// Convert a Value to a CSV-safe string representation.
fn csv_field_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Bytes(b) => b.iter().map(|byte| format!("{byte:02x}")).collect(),
        Value::Array(_) | Value::Map(_) => {
            let json = crate::formats::json::value_to_json(value);
            serde_json::to_string(&json).unwrap_or_default()
        }
    }
}

/// Escape and join CSV fields into a record line.
fn csv_escape_record(fields: &[String], delimiter: u8) -> String {
    let delim = delimiter as char;
    fields
        .iter()
        .map(|f| {
            if f.contains(delim) || f.contains('"') || f.contains('\n') || f.contains('\r') {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(&delim.to_string())
}

/// Stream JSONL input: read line by line, apply mapping, write to output.
pub fn stream_jsonl<R: Read, W: Write>(
    reader: R,
    writer: &mut StreamWriter<W>,
    mapping_program: Option<&mapping::ast::Program>,
) -> error::Result<usize> {
    let buf_reader = BufReader::new(reader);
    let mut count = 0;

    for (line_num, line) in buf_reader.lines().enumerate() {
        let line = line?;
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
        let mut value = crate::formats::json::json_to_value(json_val);

        if let Some(program) = mapping_program {
            value = mapping::eval::eval(program, &value)?;
        }

        writer.write_element(&value)?;
        count += 1;
    }

    Ok(count)
}

/// Stream CSV input: read row by row, apply mapping, write to output.
pub fn stream_csv<R: Read, W: Write>(
    reader: R,
    writer: &mut StreamWriter<W>,
    mapping_program: Option<&mapping::ast::Program>,
    csv_config: &crate::formats::csv::CsvConfig,
) -> error::Result<usize> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(csv_config.has_headers)
        .delimiter(csv_config.delimiter)
        .flexible(csv_config.flexible)
        .from_reader(reader);

    let headers: Option<Vec<String>> = if csv_config.has_headers {
        Some(
            rdr.headers()
                .map_err(|e| error::MorphError::format(e.to_string()))?
                .iter()
                .map(|h| h.to_string())
                .collect(),
        )
    } else {
        None
    };

    let mut count = 0;
    for result in rdr.records() {
        let record = result?;
        let value = if let Some(ref headers) = headers {
            let mut map = indexmap::IndexMap::new();
            for (i, field) in record.iter().enumerate() {
                let key = headers
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("column_{i}"));
                map.insert(key, parse_csv_field(field));
            }
            Value::Map(map)
        } else {
            let row: Vec<Value> = record.iter().map(parse_csv_field).collect();
            Value::Array(row)
        };

        let value = if let Some(program) = mapping_program {
            mapping::eval::eval(program, &value)?
        } else {
            value
        };

        writer.write_element(&value)?;
        count += 1;
    }

    Ok(count)
}

/// Try to parse a CSV field into the most specific type.
fn parse_csv_field(field: &str) -> Value {
    if field.is_empty() {
        return Value::String(String::new());
    }
    match field {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        _ => {}
    }
    if let Ok(i) = field.parse::<i64>() {
        return Value::Int(i);
    }
    if let Ok(f) = field.parse::<f64>() {
        return Value::Float(f);
    }
    Value::String(field.to_string())
}

/// Stream JSON array input: read elements one by one.
///
/// Reads the full input, parses as a JSON array, then processes each element
/// individually. This still benefits from streaming *output* — each element is
/// serialized and flushed immediately rather than buffered. For truly enormous
/// inputs that don't fit in RAM, JSONL or CSV streaming should be used instead.
pub fn stream_json_array<R: Read, W: Write>(
    reader: R,
    writer: &mut StreamWriter<W>,
    mapping_program: Option<&mapping::ast::Program>,
) -> error::Result<usize> {
    let mut buf = String::new();
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut buf)?;

    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }

    // Parse as a JSON array
    let json_val: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| error::MorphError::format(e.to_string()))?;

    let arr = match json_val {
        serde_json::Value::Array(a) => a,
        _ => {
            return Err(error::MorphError::format(
                "JSON streaming requires an array at the top level",
            ));
        }
    };

    let mut count = 0;
    for json_elem in arr {
        let mut value = crate::formats::json::json_to_value(json_elem);

        if let Some(program) = mapping_program {
            value = mapping::eval::eval(program, &value)?;
        }

        writer.write_element(&value)?;
        count += 1;
    }

    Ok(count)
}

/// Run the streaming pipeline.
pub fn run_streaming(
    cli: &Cli,
    in_fmt: Format,
    out_fmt: Format,
    mapping_program: Option<&mapping::ast::Program>,
) -> error::Result<()> {
    let input: Box<dyn Read> = match &cli.input {
        Some(path) => {
            let file = std::fs::File::open(path).map_err(|e| {
                error::MorphError::Io(io::Error::new(e.kind(), format!("{}: {e}", path.display())))
            })?;
            Box::new(file)
        }
        None => Box::new(io::stdin()),
    };

    let output: Box<dyn Write> = match &cli.output {
        Some(path) => {
            let file = std::fs::File::create(path).map_err(|e| {
                error::MorphError::Io(io::Error::new(e.kind(), format!("{}: {e}", path.display())))
            })?;
            Box::new(io::BufWriter::new(file))
        }
        None => Box::new(io::BufWriter::new(io::stdout())),
    };

    let csv_config = cli.csv_config();
    let mut writer = StreamWriter::new(output, out_fmt, csv_config.clone());

    writer.begin()?;

    match in_fmt {
        Format::Jsonl => {
            stream_jsonl(input, &mut writer, mapping_program)?;
        }
        Format::Csv => {
            stream_csv(input, &mut writer, mapping_program, &csv_config)?;
        }
        Format::Json => {
            stream_json_array(input, &mut writer, mapping_program)?;
        }
        _ => {
            return Err(error::MorphError::format(format!(
                "streaming input not supported for {}",
                in_fmt
            )));
        }
    }

    writer.end()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    // -----------------------------------------------------------------------
    // JSONL streaming
    // -----------------------------------------------------------------------

    #[test]
    fn stream_jsonl_to_jsonl() {
        let input = b"{\"name\":\"Alice\",\"age\":30}\n{\"name\":\"Bob\",\"age\":25}\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config);
            writer.begin().unwrap();
            stream_jsonl(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("\"Alice\""), "result: {result}");
        assert!(result.contains("\"Bob\""), "result: {result}");
        let lines: Vec<&str> = result.trim().lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines: {result}");
    }

    #[test]
    fn stream_jsonl_to_json() {
        let input = b"{\"a\":1}\n{\"a\":2}\n{\"a\":3}\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Json, csv_config);
            writer.begin().unwrap();
            stream_jsonl(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        // Should be a valid JSON array
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array(), "expected JSON array: {result}");
        assert_eq!(parsed.as_array().unwrap().len(), 3);
    }

    #[test]
    fn stream_jsonl_to_csv() {
        let input = b"{\"name\":\"Alice\",\"age\":30}\n{\"name\":\"Bob\",\"age\":25}\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Csv, csv_config);
            writer.begin().unwrap();
            stream_jsonl(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("name"), "expected header: {result}");
        assert!(result.contains("Alice"), "expected Alice: {result}");
        assert!(result.contains("Bob"), "expected Bob: {result}");
    }

    // -----------------------------------------------------------------------
    // CSV streaming
    // -----------------------------------------------------------------------

    #[test]
    fn stream_csv_to_jsonl() {
        let input = b"name,age\nAlice,30\nBob,25\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config.clone());
            writer.begin().unwrap();
            stream_csv(&input[..], &mut writer, None, &csv_config).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = result.trim().lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines: {result}");
        assert!(result.contains("\"Alice\""), "result: {result}");
    }

    #[test]
    fn stream_csv_to_json() {
        let input = b"name,age\nAlice,30\nBob,25\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Json, csv_config.clone());
            writer.begin().unwrap();
            stream_csv(&input[..], &mut writer, None, &csv_config).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array(), "expected JSON array: {result}");
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn stream_csv_no_header() {
        let input = b"Alice,30\nBob,25\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig {
            has_headers: false,
            ..Default::default()
        };
        {
            let mut writer = StreamWriter::new(
                &mut output,
                Format::Jsonl,
                crate::formats::csv::CsvConfig::default(),
            );
            writer.begin().unwrap();
            stream_csv(&input[..], &mut writer, None, &csv_config).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = result.trim().lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines: {result}");
        // Each line should be a JSON array (no headers)
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert!(first.is_array(), "expected array: {first}");
    }

    // -----------------------------------------------------------------------
    // JSON array streaming
    // -----------------------------------------------------------------------

    #[test]
    fn stream_json_array_to_jsonl() {
        let input = b"[{\"a\":1},{\"a\":2},{\"a\":3}]";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config);
            writer.begin().unwrap();
            stream_json_array(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = result.trim().lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 lines: {result}");
    }

    #[test]
    fn stream_json_array_to_csv() {
        let input = b"[{\"name\":\"Alice\",\"age\":30},{\"name\":\"Bob\",\"age\":25}]";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Csv, csv_config);
            writer.begin().unwrap();
            stream_json_array(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("name"), "expected header: {result}");
        assert!(result.contains("Alice"), "expected Alice: {result}");
    }

    #[test]
    fn stream_json_array_with_nested() {
        let input = b"[{\"a\":{\"b\":1}},{\"a\":{\"b\":2}}]";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config);
            writer.begin().unwrap();
            stream_json_array(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = result.trim().lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines: {result}");
    }

    #[test]
    fn stream_empty_array() {
        let input = b"[]";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config);
            writer.begin().unwrap();
            let count = stream_json_array(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
            assert_eq!(count, 0);
        }
        let result = String::from_utf8(output).unwrap();
        assert_eq!(result.trim(), "");
    }

    #[test]
    fn stream_empty_jsonl() {
        let input = b"\n\n\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config);
            writer.begin().unwrap();
            let count = stream_jsonl(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
            assert_eq!(count, 0);
        }
    }

    // -----------------------------------------------------------------------
    // Streaming with mappings
    // -----------------------------------------------------------------------

    #[test]
    fn stream_jsonl_with_mapping() {
        let input = b"{\"name\":\"Alice\",\"age\":30}\n{\"name\":\"Bob\",\"age\":25}\n";
        let program = crate::mapping::parser::parse_str("select .name").unwrap();

        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config);
            writer.begin().unwrap();
            stream_jsonl(&input[..], &mut writer, Some(&program)).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("\"name\""), "result: {result}");
        assert!(
            !result.contains("\"age\""),
            "age should be dropped: {result}"
        );
    }

    #[test]
    fn stream_csv_with_mapping() {
        let input = b"name,age\nAlice,30\nBob,25\n";
        let program = crate::mapping::parser::parse_str("select .name").unwrap();

        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config.clone());
            writer.begin().unwrap();
            stream_csv(&input[..], &mut writer, Some(&program), &csv_config).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("\"name\""), "result: {result}");
        assert!(
            !result.contains("\"age\""),
            "age should be dropped: {result}"
        );
    }

    // -----------------------------------------------------------------------
    // StreamWriter unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn writer_json_format() {
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Json, csv_config);
            writer.begin().unwrap();
            writer.write_element(&Value::Int(1)).unwrap();
            writer.write_element(&Value::Int(2)).unwrap();
            writer.write_element(&Value::Int(3)).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn writer_csv_map_elements() {
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Csv, csv_config);
            writer.begin().unwrap();
            let mut m1 = IndexMap::new();
            m1.insert("a".to_string(), Value::Int(1));
            m1.insert("b".to_string(), Value::Int(2));
            writer.write_element(&Value::Map(m1)).unwrap();
            let mut m2 = IndexMap::new();
            m2.insert("a".to_string(), Value::Int(3));
            m2.insert("b".to_string(), Value::Int(4));
            writer.write_element(&Value::Map(m2)).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        assert!(result.starts_with("a,b\n"), "expected header: {result}");
        assert!(result.contains("1,2"), "expected row 1: {result}");
        assert!(result.contains("3,4"), "expected row 2: {result}");
    }

    // -----------------------------------------------------------------------
    // Streaming output flushing (flush periodically)
    // -----------------------------------------------------------------------

    #[test]
    fn stream_flushes_output() {
        // This test verifies that the writer flushes at end()
        let input = b"{\"a\":1}\n{\"a\":2}\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig::default();
        {
            let mut writer = StreamWriter::new(&mut output, Format::Jsonl, csv_config);
            writer.begin().unwrap();
            stream_jsonl(&input[..], &mut writer, None).unwrap();
            writer.end().unwrap();
        }
        // Output should be non-empty after flush
        assert!(!output.is_empty(), "output should be flushed");
    }

    // -----------------------------------------------------------------------
    // Tab-delimited CSV streaming
    // -----------------------------------------------------------------------

    #[test]
    fn stream_tsv_to_jsonl() {
        let input = b"name\tage\nAlice\t30\nBob\t25\n";
        let mut output = Vec::new();
        let csv_config = crate::formats::csv::CsvConfig {
            delimiter: b'\t',
            ..Default::default()
        };
        {
            let mut writer = StreamWriter::new(
                &mut output,
                Format::Jsonl,
                crate::formats::csv::CsvConfig::default(),
            );
            writer.begin().unwrap();
            stream_csv(&input[..], &mut writer, None, &csv_config).unwrap();
            writer.end().unwrap();
        }
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("\"Alice\""), "result: {result}");
        assert!(result.contains("\"name\""), "result: {result}");
    }
}
