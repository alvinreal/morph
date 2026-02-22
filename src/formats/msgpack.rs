use crate::error;
use crate::value::Value;
use indexmap::IndexMap;
use std::io::Read;

/// Parse a MessagePack byte slice into a Universal Value.
pub fn from_bytes(input: &[u8]) -> error::Result<Value> {
    let rmp_val: rmpv::Value = rmpv::decode::read_value(&mut &input[..])
        .map_err(|e| error::MorphError::format(format!("MessagePack decode error: {e}")))?;
    Ok(rmpv_to_value(rmp_val))
}

/// Parse MessagePack from a reader.
pub fn from_reader<R: Read>(mut reader: R) -> error::Result<Value> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    from_bytes(&buf)
}

/// Parse MessagePack from a "string" representation.
///
/// For the CLI text pipeline, MessagePack data is encoded as a hex string
/// (each byte → two hex digits). This function accepts either raw binary
/// (for programmatic use) or a hex-encoded string.
pub fn from_str(input: &str) -> error::Result<Value> {
    // Try to decode as hex first (CLI pipeline)
    if let Some(bytes) = hex_decode(input.trim()) {
        return from_bytes(&bytes);
    }
    // Fallback: treat as raw bytes
    from_bytes(input.as_bytes())
}

/// Serialize a Universal Value to MessagePack bytes.
pub fn to_bytes(value: &Value) -> error::Result<Vec<u8>> {
    let rmp_val = value_to_rmpv(value);
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &rmp_val)
        .map_err(|e| error::MorphError::format(format!("MessagePack encode error: {e}")))?;
    Ok(buf)
}

/// Serialize a Universal Value to a hex-encoded string (for the CLI text pipeline).
pub fn to_string(value: &Value) -> error::Result<String> {
    let bytes = to_bytes(value)?;
    Ok(hex_encode(&bytes))
}

// ---------------------------------------------------------------------------
// Hex encoding/decoding helpers
// ---------------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s.push('\n');
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() || !s.len().is_multiple_of(2) {
        return None;
    }
    // Check all chars are hex digits
    if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let high = hex_nibble(chunk[0])?;
        let low = hex_nibble(chunk[1])?;
        bytes.push((high << 4) | low);
    }
    Some(bytes)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// rmpv ↔ Value conversions
// ---------------------------------------------------------------------------

fn rmpv_to_value(v: rmpv::Value) -> Value {
    match v {
        rmpv::Value::Nil => Value::Null,
        rmpv::Value::Boolean(b) => Value::Bool(b),
        rmpv::Value::Integer(i) => {
            if let Some(n) = i.as_i64() {
                Value::Int(n)
            } else if let Some(n) = i.as_u64() {
                // u64 values that don't fit in i64 – store as float to avoid data loss
                Value::Float(n as f64)
            } else {
                Value::Int(0)
            }
        }
        rmpv::Value::F32(f) => Value::Float(f as f64),
        rmpv::Value::F64(f) => Value::Float(f),
        rmpv::Value::String(s) => {
            if s.is_str() {
                Value::String(s.into_str().unwrap().to_string())
            } else {
                // Non-UTF-8 msgpack string → store as Bytes
                Value::Bytes(s.into_bytes())
            }
        }
        rmpv::Value::Binary(b) => Value::Bytes(b),
        rmpv::Value::Array(arr) => Value::Array(arr.into_iter().map(rmpv_to_value).collect()),
        rmpv::Value::Map(entries) => {
            let mut map = IndexMap::new();
            for (k, v) in entries {
                let key = match k {
                    rmpv::Value::String(s) => s.into_str().unwrap_or_default().to_string(),
                    other => format!("{other}"),
                };
                map.insert(key, rmpv_to_value(v));
            }
            Value::Map(map)
        }
        rmpv::Value::Ext(type_id, data) => {
            // Store extension types as a map with metadata
            let mut map = IndexMap::new();
            map.insert("_ext_type".to_string(), Value::Int(type_id as i64));
            map.insert("_ext_data".to_string(), Value::Bytes(data));
            Value::Map(map)
        }
    }
}

fn value_to_rmpv(value: &Value) -> rmpv::Value {
    match value {
        Value::Null => rmpv::Value::Nil,
        Value::Bool(b) => rmpv::Value::Boolean(*b),
        Value::Int(i) => rmpv::Value::Integer((*i).into()),
        Value::Float(f) => rmpv::Value::F64(*f),
        Value::String(s) => rmpv::Value::String(s.clone().into()),
        Value::Bytes(b) => rmpv::Value::Binary(b.clone()),
        Value::Array(arr) => rmpv::Value::Array(arr.iter().map(value_to_rmpv).collect()),
        Value::Map(map) => {
            let entries: Vec<(rmpv::Value, rmpv::Value)> = map
                .iter()
                .map(|(k, v)| (rmpv::Value::String(k.clone().into()), value_to_rmpv(v)))
                .collect();
            rmpv::Value::Map(entries)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Primitive round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_null() {
        let val = Value::Null;
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_bool_true() {
        let val = Value::Bool(true);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_bool_false() {
        let val = Value::Bool(false);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_positive_int() {
        let val = Value::Int(42);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_negative_int() {
        let val = Value::Int(-100);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_zero() {
        let val = Value::Int(0);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_i64_max() {
        let val = Value::Int(i64::MAX);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_i64_min() {
        let val = Value::Int(i64::MIN);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_float() {
        let val = Value::Float(3.125);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_float_negative() {
        let val = Value::Float(-2.5);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_string() {
        let val = Value::String("hello world".into());
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_empty_string() {
        let val = Value::String(String::new());
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_unicode_string() {
        let val = Value::String("héllo wörld 🦀".into());
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Bytes (binary) round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_bytes() {
        let val = Value::Bytes(vec![0x00, 0x01, 0xFF, 0xFE, 0x42]);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_empty_bytes() {
        let val = Value::Bytes(vec![]);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Nested structures
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_nested_map() {
        let mut inner = IndexMap::new();
        inner.insert("x".to_string(), Value::Int(1));
        inner.insert("y".to_string(), Value::Int(2));
        let mut outer = IndexMap::new();
        outer.insert("point".to_string(), Value::Map(inner));
        outer.insert("label".to_string(), Value::String("origin".into()));
        let val = Value::Map(outer);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_array_of_maps() {
        let mut m1 = IndexMap::new();
        m1.insert("name".to_string(), Value::String("Alice".into()));
        m1.insert("age".to_string(), Value::Int(30));
        let mut m2 = IndexMap::new();
        m2.insert("name".to_string(), Value::String("Bob".into()));
        m2.insert("age".to_string(), Value::Int(25));
        let val = Value::Array(vec![Value::Map(m1), Value::Map(m2)]);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_deeply_nested() {
        let mut level3 = IndexMap::new();
        level3.insert("deep".to_string(), Value::Bool(true));
        let mut level2 = IndexMap::new();
        level2.insert("level3".to_string(), Value::Map(level3));
        let mut level1 = IndexMap::new();
        level1.insert("level2".to_string(), Value::Map(level2));
        let val = Value::Map(level1);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_mixed_array() {
        let val = Value::Array(vec![
            Value::Null,
            Value::Bool(true),
            Value::Int(42),
            Value::Float(3.125),
            Value::String("hello".into()),
            Value::Bytes(vec![0xFF]),
            Value::Array(vec![Value::Int(1), Value::Int(2)]),
        ]);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Empty containers
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_empty_array() {
        let val = Value::Array(vec![]);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn roundtrip_empty_map() {
        let val = Value::Map(IndexMap::new());
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // MessagePack → JSON → MessagePack round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_msgpack_json_msgpack() {
        let mut map = IndexMap::new();
        map.insert("name".to_string(), Value::String("Alice".into()));
        map.insert("age".to_string(), Value::Int(30));
        map.insert("active".to_string(), Value::Bool(true));
        map.insert(
            "scores".to_string(),
            Value::Array(vec![Value::Int(100), Value::Int(95), Value::Int(87)]),
        );
        let val = Value::Map(map);

        // msgpack → bytes → value
        let msgpack_bytes = to_bytes(&val).unwrap();
        let from_msgpack = from_bytes(&msgpack_bytes).unwrap();

        // value → JSON string → value
        let json_str = crate::formats::json::to_string(&from_msgpack).unwrap();
        let from_json = crate::formats::json::from_str(&json_str).unwrap();

        // value → msgpack → value
        let msgpack_bytes2 = to_bytes(&from_json).unwrap();
        let final_val = from_bytes(&msgpack_bytes2).unwrap();

        assert_eq!(val, final_val);
    }

    // -----------------------------------------------------------------------
    // Invalid data → clear error
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_msgpack_returns_error() {
        // A fixmap claiming 15 entries but only providing garbage
        let bad = &[0x8f, 0xff, 0xff, 0xff];
        let err = from_bytes(bad).unwrap_err();
        match err {
            crate::error::MorphError::Format { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Format error, got: {other:?}"),
        }
    }

    #[test]
    fn truncated_msgpack_returns_error() {
        // A map header claiming 5 entries but no data follows
        let bad = &[0x85];
        let err = from_bytes(bad).unwrap_err();
        assert!(matches!(err, crate::error::MorphError::Format { .. }));
    }

    #[test]
    fn empty_input_returns_error() {
        let err = from_bytes(&[]).unwrap_err();
        assert!(matches!(err, crate::error::MorphError::Format { .. }));
    }

    // -----------------------------------------------------------------------
    // from_reader
    // -----------------------------------------------------------------------

    #[test]
    fn from_reader_works() {
        let val = Value::Int(42);
        let bytes = to_bytes(&val).unwrap();
        let val2 = from_reader(bytes.as_slice()).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Hex-based to_string / from_str round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn to_string_from_str_roundtrip() {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::String("value".into()));
        let val = Value::Map(map);

        let s = to_string(&val).unwrap();
        let val2 = from_str(&s).unwrap();
        assert_eq!(val, val2);
    }

    #[test]
    fn to_string_from_str_roundtrip_complex() {
        let val = Value::Array(vec![
            Value::Int(42),
            Value::String("hello 🦀".into()),
            Value::Bool(true),
            Value::Null,
        ]);
        let s = to_string(&val).unwrap();
        let val2 = from_str(&s).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Complex structure with all types
    // -----------------------------------------------------------------------

    #[test]
    fn complex_all_types() {
        let mut map = IndexMap::new();
        map.insert("null_val".to_string(), Value::Null);
        map.insert("bool_val".to_string(), Value::Bool(false));
        map.insert("int_val".to_string(), Value::Int(-999));
        map.insert("float_val".to_string(), Value::Float(2.5));
        map.insert("string_val".to_string(), Value::String("test 🎉".into()));
        map.insert(
            "bytes_val".to_string(),
            Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        );
        map.insert(
            "array_val".to_string(),
            Value::Array(vec![Value::Int(1), Value::String("two".into())]),
        );
        let mut nested = IndexMap::new();
        nested.insert("inner".to_string(), Value::Bool(true));
        map.insert("map_val".to_string(), Value::Map(nested));
        let val = Value::Map(map);

        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        assert_eq!(val, val2);
    }

    // -----------------------------------------------------------------------
    // Key order preserved
    // -----------------------------------------------------------------------

    #[test]
    fn key_order_preserved() {
        let mut map = IndexMap::new();
        map.insert("z".to_string(), Value::Int(3));
        map.insert("a".to_string(), Value::Int(1));
        map.insert("m".to_string(), Value::Int(2));
        let val = Value::Map(map);

        let bytes = to_bytes(&val).unwrap();
        let val2 = from_bytes(&bytes).unwrap();
        let keys: Vec<&String> = match &val2 {
            Value::Map(m) => m.keys().collect(),
            _ => panic!("expected map"),
        };
        assert_eq!(keys, vec!["z", "a", "m"]);
    }

    // -----------------------------------------------------------------------
    // Hex helpers
    // -----------------------------------------------------------------------

    #[test]
    fn hex_encode_decode_roundtrip() {
        let data = vec![0x00, 0x42, 0xFF, 0xAB, 0xCD];
        let encoded = hex_encode(&data);
        let decoded = hex_decode(encoded.trim()).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn hex_decode_invalid_returns_none() {
        assert!(hex_decode("xyz").is_none());
        assert!(hex_decode("0").is_none()); // odd length
    }
}
