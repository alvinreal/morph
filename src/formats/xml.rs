use crate::error;
use crate::value::Value;
use indexmap::IndexMap;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::io::{Cursor, Read};

/// XML reader/writer configuration.
#[derive(Debug, Clone)]
pub struct XmlConfig {
    /// Prefix for attribute keys (default: `@`).
    pub attr_prefix: String,
    /// Root element name for output (default: `root`).
    pub root_element: String,
}

impl Default for XmlConfig {
    fn default() -> Self {
        Self {
            attr_prefix: "@".to_string(),
            root_element: "root".to_string(),
        }
    }
}

/// Parse an XML string into a Universal Value.
pub fn from_str(input: &str) -> error::Result<Value> {
    from_str_with_config(input, &XmlConfig::default())
}

/// Parse an XML string with custom configuration.
pub fn from_str_with_config(input: &str, config: &XmlConfig) -> error::Result<Value> {
    let mut reader = Reader::from_str(input);
    reader.trim_text(true);

    // We parse the root element; the result is the content of the root.
    let mut result = Value::Null;
    let mut found_root = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if found_root {
                    return Err(error::MorphError::format(
                        "multiple root elements not supported",
                    ));
                }
                found_root = true;
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                result = parse_element(&mut reader, e, config, &tag_name)?;
            }
            Ok(Event::Empty(ref e)) => {
                if found_root {
                    return Err(error::MorphError::format(
                        "multiple root elements not supported",
                    ));
                }
                found_root = true;
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                result = parse_empty_element(e, config, &tag_name)?;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Decl(_)) | Ok(Event::Comment(_)) | Ok(Event::PI(_)) => continue,
            Ok(Event::Text(_)) => {
                // Ignore top-level whitespace text
            }
            Ok(Event::CData(ref e)) => {
                if !found_root {
                    let text = String::from_utf8_lossy(e.as_ref()).to_string();
                    result = Value::String(text);
                    found_root = true;
                }
            }
            Err(e) => {
                return Err(error::MorphError::format(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
    }

    Ok(result)
}

/// Parse XML from a reader.
pub fn from_reader<R: Read>(mut reader: R) -> error::Result<Value> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    from_str(&buf)
}

/// Parse XML from a reader with custom config.
pub fn from_reader_with_config<R: Read>(mut reader: R, config: &XmlConfig) -> error::Result<Value> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    from_str_with_config(&buf, config)
}

/// Parse the content of an element (after the start tag has been read).
/// Returns the Value representing this element's content.
fn parse_element(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart,
    config: &XmlConfig,
    _tag_name: &str,
) -> error::Result<Value> {
    let mut map = IndexMap::new();
    let mut text_parts: Vec<String> = Vec::new();
    let mut has_children = false;

    // Process attributes
    for attr_result in start.attributes() {
        let attr = attr_result
            .map_err(|e| error::MorphError::format(format!("XML attribute error: {e}")))?;
        let key = format!(
            "{}{}",
            config.attr_prefix,
            String::from_utf8_lossy(attr.key.as_ref())
        );
        let value = attr
            .decode_and_unescape_value(reader)
            .map_err(|e| error::MorphError::format(format!("XML attribute decode error: {e}")))?
            .to_string();
        map.insert(key, Value::String(value));
    }

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                has_children = true;
                let child_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let child_value = parse_element(reader, e, config, &child_name)?;
                insert_child(&mut map, child_name, child_value);
            }
            Ok(Event::Empty(ref e)) => {
                has_children = true;
                let child_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let child_value = parse_empty_element(e, config, &child_name)?;
                insert_child(&mut map, child_name, child_value);
            }
            Ok(Event::Text(ref e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| {
                        error::MorphError::format(format!("XML text decode error: {err}"))
                    })?
                    .to_string();
                if !text.is_empty() {
                    text_parts.push(text);
                }
            }
            Ok(Event::CData(ref e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if !text.is_empty() {
                    text_parts.push(text);
                }
            }
            Ok(Event::End(_)) => {
                break;
            }
            Ok(Event::Comment(_))
            | Ok(Event::PI(_))
            | Ok(Event::Decl(_))
            | Ok(Event::DocType(_)) => continue,
            Ok(Event::Eof) => {
                return Err(error::MorphError::format("unexpected end of XML"));
            }
            Err(e) => {
                return Err(error::MorphError::format(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
        }
    }

    let has_attrs = map.keys().any(|k| k.starts_with(&config.attr_prefix));
    let combined_text = text_parts.join("");

    if !has_children && !has_attrs && !combined_text.is_empty() {
        // Element with only text content → return the text as a string value
        return Ok(Value::String(combined_text));
    }

    if !combined_text.is_empty() {
        // Mixed content: has both text and child elements/attributes
        map.insert("#text".to_string(), Value::String(combined_text));
    }

    if map.is_empty() {
        // Empty element with no attributes
        return Ok(Value::Null);
    }

    Ok(Value::Map(map))
}

/// Parse an empty (self-closing) element.
fn parse_empty_element(
    start: &BytesStart,
    config: &XmlConfig,
    _tag_name: &str,
) -> error::Result<Value> {
    let mut map = IndexMap::new();

    // Process attributes
    for attr_result in start.attributes() {
        let attr = attr_result
            .map_err(|e| error::MorphError::format(format!("XML attribute error: {e}")))?;
        let key = format!(
            "{}{}",
            config.attr_prefix,
            String::from_utf8_lossy(attr.key.as_ref())
        );
        // For empty elements we can't use reader to decode, do it manually
        let value = String::from_utf8_lossy(&attr.value).to_string();
        map.insert(key, Value::String(value));
    }

    if map.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::Map(map))
    }
}

/// Insert a child element into the parent map, converting to array for repeated elements.
fn insert_child(map: &mut IndexMap<String, Value>, key: String, value: Value) {
    if let Some(existing) = map.get_mut(&key) {
        // Key already exists: convert to array or append
        match existing {
            Value::Array(arr) => {
                arr.push(value);
            }
            _ => {
                let prev = std::mem::replace(existing, Value::Null);
                *existing = Value::Array(vec![prev, value]);
            }
        }
    } else {
        map.insert(key, value);
    }
}

/// Serialize a Universal Value to an XML string.
pub fn to_string(value: &Value) -> error::Result<String> {
    to_string_with_config(value, &XmlConfig::default())
}

/// Serialize a Universal Value to an XML string with custom configuration.
pub fn to_string_with_config(value: &Value, config: &XmlConfig) -> error::Result<String> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    // Write XML declaration
    writer
        .write_event(Event::Decl(quick_xml::events::BytesDecl::new(
            "1.0",
            Some("UTF-8"),
            None,
        )))
        .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;

    // Write a newline after declaration
    writer.get_mut().get_mut().extend_from_slice(b"\n");

    write_element(&mut writer, &config.root_element, value, config)?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| error::MorphError::format(format!("UTF-8 error: {e}")))
}

/// Write an element and its contents to the XML writer.
fn write_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    value: &Value,
    config: &XmlConfig,
) -> error::Result<()> {
    match value {
        Value::Null => {
            // Empty element: <name/>
            let elem = BytesStart::new(name);
            writer
                .write_event(Event::Empty(elem))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
        }
        Value::Bool(b) => {
            let elem = BytesStart::new(name);
            writer
                .write_event(Event::Start(elem))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(&b.to_string())))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
        }
        Value::Int(i) => {
            let elem = BytesStart::new(name);
            writer
                .write_event(Event::Start(elem))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(&i.to_string())))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
        }
        Value::Float(f) => {
            let elem = BytesStart::new(name);
            writer
                .write_event(Event::Start(elem))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(&f.to_string())))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
        }
        Value::String(s) => {
            let elem = BytesStart::new(name);
            writer
                .write_event(Event::Start(elem))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(s)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
        }
        Value::Bytes(b) => {
            let hex: String = b.iter().map(|byte| format!("{byte:02x}")).collect();
            let elem = BytesStart::new(name);
            writer
                .write_event(Event::Start(elem))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(&hex)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
        }
        Value::Array(arr) => {
            // Array: wrap each element in an <item> tag (or parent name)
            let elem = BytesStart::new(name);
            writer
                .write_event(Event::Start(elem))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            for item in arr {
                write_element(writer, "item", item, config)?;
            }
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
        }
        Value::Map(map) => {
            let mut elem = BytesStart::new(name);

            // Separate attributes from child elements
            let mut children: Vec<(&String, &Value)> = Vec::new();
            let mut text_content: Option<&String> = None;

            for (k, v) in map {
                if let Some(attr_name) = k.strip_prefix(&config.attr_prefix) {
                    if !attr_name.is_empty() {
                        // This is an attribute
                        let attr_val = match v {
                            Value::String(s) => s.clone(),
                            Value::Int(i) => i.to_string(),
                            Value::Float(f) => f.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => format!("{v}"),
                        };
                        elem.push_attribute((attr_name, attr_val.as_str()));
                        continue;
                    }
                }
                if k == "#text" {
                    if let Value::String(s) = v {
                        text_content = Some(s);
                    }
                    continue;
                }
                children.push((k, v));
            }

            if children.is_empty() && text_content.is_none() {
                // Only attributes, no children or text
                writer
                    .write_event(Event::Empty(elem))
                    .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            } else {
                writer
                    .write_event(Event::Start(elem))
                    .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;

                if let Some(text) = text_content {
                    writer
                        .write_event(Event::Text(BytesText::new(text)))
                        .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
                }

                for (k, v) in children {
                    match v {
                        Value::Array(arr) => {
                            // Repeated elements: each array item becomes a separate element with the same tag name
                            for item in arr {
                                write_element(writer, k, item, config)?;
                            }
                        }
                        _ => {
                            write_element(writer, k, v, config)?;
                        }
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new(name)))
                    .map_err(|e| error::MorphError::format(format!("XML write error: {e}")))?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Simple element → Map
    // -----------------------------------------------------------------------

    #[test]
    fn simple_element_to_map() {
        let input = "<root><name>John</name></root>";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("John".into())));
    }

    // -----------------------------------------------------------------------
    // Attributes
    // -----------------------------------------------------------------------

    #[test]
    fn attributes_with_default_prefix() {
        let input = r#"<root><user id="1">Alice</user></root>"#;
        let val = from_str(input).unwrap();
        let user = val.get_path(".user").unwrap();
        assert_eq!(user.get_path(".@id"), Some(&Value::String("1".into())));
        assert_eq!(
            user.get_path(".#text"),
            Some(&Value::String("Alice".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Nested elements
    // -----------------------------------------------------------------------

    #[test]
    fn nested_elements() {
        let input = "<root><user><name>Alice</name><age>30</age></user></root>";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".user.name"),
            Some(&Value::String("Alice".into()))
        );
        assert_eq!(val.get_path(".user.age"), Some(&Value::String("30".into())));
    }

    // -----------------------------------------------------------------------
    // Repeated elements → Array
    // -----------------------------------------------------------------------

    #[test]
    fn repeated_elements_to_array() {
        let input = "<root><item>a</item><item>b</item><item>c</item></root>";
        let val = from_str(input).unwrap();
        let items = val.get_path(".item").unwrap();
        match items {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], Value::String("a".into()));
                assert_eq!(arr[1], Value::String("b".into()));
                assert_eq!(arr[2], Value::String("c".into()));
            }
            _ => panic!("expected array, got: {items:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Mixed content (text + child elements)
    // -----------------------------------------------------------------------

    #[test]
    fn mixed_content() {
        let input = "<root><p>Hello <b>world</b></p></root>";
        let val = from_str(input).unwrap();
        let p = val.get_path(".p").unwrap();
        // With trim_text enabled, trailing space on "Hello " gets trimmed
        assert_eq!(p.get_path(".#text"), Some(&Value::String("Hello".into())));
        assert_eq!(p.get_path(".b"), Some(&Value::String("world".into())));
    }

    // -----------------------------------------------------------------------
    // CDATA sections
    // -----------------------------------------------------------------------

    #[test]
    fn cdata_section() {
        let input = "<root><code><![CDATA[x < y && z > w]]></code></root>";
        let val = from_str(input).unwrap();
        assert_eq!(
            val.get_path(".code"),
            Some(&Value::String("x < y && z > w".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Empty elements
    // -----------------------------------------------------------------------

    #[test]
    fn empty_self_closing_element() {
        let input = "<root><empty/></root>";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".empty"), Some(&Value::Null));
    }

    #[test]
    fn empty_element_with_closing_tag() {
        let input = "<root><empty></empty></root>";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".empty"), Some(&Value::Null));
    }

    // -----------------------------------------------------------------------
    // Namespaces (basic handling — preserved as-is in tag names)
    // -----------------------------------------------------------------------

    #[test]
    fn basic_namespace_handling() {
        let input = r#"<root xmlns:ns="http://example.com"><ns:item>value</ns:item></root>"#;
        let val = from_str(input).unwrap();
        // Namespace prefix is preserved in key name
        assert_eq!(
            val.get_path(".ns:item"),
            Some(&Value::String("value".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Round-trip XML → Value → XML → Value
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_simple() {
        let input = "<root><name>Alice</name><age>30</age></root>";
        let val = from_str(input).unwrap();
        let xml_output = to_string(&val).unwrap();
        let val2 = from_str(&xml_output).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_with_attributes() {
        let input = r#"<root><user id="1"><name>Alice</name></user></root>"#;
        let val = from_str(input).unwrap();
        let xml_output = to_string(&val).unwrap();
        let val2 = from_str(&xml_output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Custom attribute prefix
    // -----------------------------------------------------------------------

    #[test]
    fn custom_attribute_prefix() {
        let input = r#"<root><user id="1">Alice</user></root>"#;
        let config = XmlConfig {
            attr_prefix: "_".to_string(),
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let user = val.get_path(".user").unwrap();
        assert_eq!(user.get_path("._id"), Some(&Value::String("1".into())));
    }

    #[test]
    fn custom_prefix_roundtrip() {
        let input = r#"<root><user id="1"><name>Alice</name></user></root>"#;
        let config = XmlConfig {
            attr_prefix: "_".to_string(),
            ..Default::default()
        };
        let val = from_str_with_config(input, &config).unwrap();
        let xml_output = to_string_with_config(&val, &config).unwrap();
        let val2 = from_str_with_config(&xml_output, &config).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Custom root element name for output
    // -----------------------------------------------------------------------

    #[test]
    fn custom_root_element() {
        let mut map = IndexMap::new();
        map.insert("name".to_string(), Value::String("Alice".into()));
        let val = Value::Map(map);
        let config = XmlConfig {
            root_element: "items".to_string(),
            ..Default::default()
        };
        let output = to_string_with_config(&val, &config).unwrap();
        assert!(output.contains("<items>"));
        assert!(output.contains("</items>"));
    }

    // -----------------------------------------------------------------------
    // Invalid XML → clear error
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_xml_returns_error() {
        let bad = "<root><unclosed>";
        let err = from_str(bad).unwrap_err();
        match err {
            crate::error::MorphError::Format { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Format error, got: {other:?}"),
        }
    }

    #[test]
    fn invalid_xml_mismatched_tags() {
        let bad = "<root><a></b></root>";
        let err = from_str(bad).unwrap_err();
        assert!(matches!(err, crate::error::MorphError::Format { .. }));
    }

    // -----------------------------------------------------------------------
    // XML declaration is handled
    // -----------------------------------------------------------------------

    #[test]
    fn xml_with_declaration() {
        let input = r#"<?xml version="1.0" encoding="UTF-8"?><root><name>Alice</name></root>"#;
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("Alice".into())));
    }

    // -----------------------------------------------------------------------
    // from_reader
    // -----------------------------------------------------------------------

    #[test]
    fn from_reader_works() {
        let data = "<root><x>42</x></root>";
        let val = from_reader(data.as_bytes()).unwrap();
        assert_eq!(val.get_path(".x"), Some(&Value::String("42".into())));
    }

    // -----------------------------------------------------------------------
    // Serialization tests
    // -----------------------------------------------------------------------

    #[test]
    fn serialize_null_as_empty_element() {
        let mut map = IndexMap::new();
        map.insert("empty".to_string(), Value::Null);
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        assert!(output.contains("<empty/>") || output.contains("<empty />"));
    }

    #[test]
    fn serialize_string() {
        let mut map = IndexMap::new();
        map.insert("name".to_string(), Value::String("Alice".into()));
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        assert!(output.contains("<name>Alice</name>"));
    }

    #[test]
    fn serialize_int() {
        let mut map = IndexMap::new();
        map.insert("age".to_string(), Value::Int(30));
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        assert!(output.contains("<age>30</age>"));
    }

    #[test]
    fn serialize_bool() {
        let mut map = IndexMap::new();
        map.insert("active".to_string(), Value::Bool(true));
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        assert!(output.contains("<active>true</active>"));
    }

    #[test]
    fn serialize_map_with_attributes() {
        let mut user = IndexMap::new();
        user.insert("@id".to_string(), Value::String("1".into()));
        user.insert("name".to_string(), Value::String("Alice".into()));
        let mut root = IndexMap::new();
        root.insert("user".to_string(), Value::Map(user));
        let val = Value::Map(root);
        let output = to_string(&val).unwrap();
        assert!(output.contains(r#"id="1""#));
        assert!(output.contains("<name>Alice</name>"));
    }

    #[test]
    fn serialize_repeated_elements() {
        let items = Value::Array(vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ]);
        let mut map = IndexMap::new();
        map.insert("item".to_string(), items);
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        assert!(output.contains("<item>a</item>"));
        assert!(output.contains("<item>b</item>"));
        assert!(output.contains("<item>c</item>"));
    }

    // -----------------------------------------------------------------------
    // XML special characters are escaped
    // -----------------------------------------------------------------------

    #[test]
    fn special_characters_escaped() {
        let mut map = IndexMap::new();
        map.insert("expr".to_string(), Value::String("a < b && c > d".into()));
        let val = Value::Map(map);
        let output = to_string(&val).unwrap();
        // The output should be valid XML
        let val2 = from_str(&output).unwrap();
        assert_eq!(
            val2.get_path(".expr"),
            Some(&Value::String("a < b && c > d".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Unicode
    // -----------------------------------------------------------------------

    #[test]
    fn unicode_content() {
        let input = "<root><emoji>🦀</emoji><accent>héllo</accent></root>";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".emoji"), Some(&Value::String("🦀".into())));
        assert_eq!(
            val.get_path(".accent"),
            Some(&Value::String("héllo".into()))
        );
    }

    #[test]
    fn unicode_roundtrip() {
        let input = "<root><emoji>🦀</emoji><text>héllo wörld</text></root>";
        let val = from_str(input).unwrap();
        let output = to_string(&val).unwrap();
        let val2 = from_str(&output).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Multiple attributes
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_attributes() {
        let input = r#"<root><user id="1" role="admin" active="true">Alice</user></root>"#;
        let val = from_str(input).unwrap();
        let user = val.get_path(".user").unwrap();
        assert_eq!(user.get_path(".@id"), Some(&Value::String("1".into())));
        assert_eq!(
            user.get_path(".@role"),
            Some(&Value::String("admin".into()))
        );
        assert_eq!(
            user.get_path(".@active"),
            Some(&Value::String("true".into()))
        );
        assert_eq!(
            user.get_path(".#text"),
            Some(&Value::String("Alice".into()))
        );
    }

    // -----------------------------------------------------------------------
    // Complex nested structure
    // -----------------------------------------------------------------------

    #[test]
    fn complex_nested_structure() {
        let input = r#"<root>
            <users>
                <user id="1">
                    <name>Alice</name>
                    <scores>
                        <score>100</score>
                        <score>95</score>
                    </scores>
                </user>
                <user id="2">
                    <name>Bob</name>
                    <scores>
                        <score>87</score>
                    </scores>
                </user>
            </users>
        </root>"#;
        let val = from_str(input).unwrap();
        let users = val.get_path(".users.user").unwrap();
        match users {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert_eq!(
                    arr[0].get_path(".name"),
                    Some(&Value::String("Alice".into()))
                );
                assert_eq!(arr[0].get_path(".@id"), Some(&Value::String("1".into())));
            }
            _ => panic!("expected array of users"),
        }
    }

    // -----------------------------------------------------------------------
    // Empty root
    // -----------------------------------------------------------------------

    #[test]
    fn empty_root() {
        let input = "<root/>";
        let val = from_str(input).unwrap();
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn empty_root_with_closing_tag() {
        let input = "<root></root>";
        let val = from_str(input).unwrap();
        assert_eq!(val, Value::Null);
    }

    // -----------------------------------------------------------------------
    // Whitespace handling
    // -----------------------------------------------------------------------

    #[test]
    fn whitespace_between_elements_ignored() {
        let input = "<root>\n  <name>Alice</name>\n  <age>30</age>\n</root>";
        let val = from_str(input).unwrap();
        assert_eq!(val.get_path(".name"), Some(&Value::String("Alice".into())));
        assert_eq!(val.get_path(".age"), Some(&Value::String("30".into())));
    }
}
