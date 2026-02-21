use indexmap::IndexMap;
use std::fmt;

/// Universal Value type — the internal representation all formats normalize to.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(IndexMap<String, Value>),
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(n) => {
                // Ensure we always print a decimal point so it looks like a float.
                if n.fract() == 0.0 && n.is_finite() {
                    write!(f, "{n:.1}")
                } else {
                    write!(f, "{n}")
                }
            }
            Value::String(s) => write!(f, "\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Value::Bytes(b) => {
                write!(f, "b\"")?;
                for byte in b {
                    write!(f, "\\x{byte:02x}")?;
                }
                write!(f, "\"")
            }
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Map(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(
                        f,
                        "\"{}\": {}",
                        k.replace('\\', "\\\\").replace('"', "\\\""),
                        v
                    )?;
                }
                write!(f, "}}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// From impls
// ---------------------------------------------------------------------------

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_owned())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl From<IndexMap<String, Value>> for Value {
    fn from(v: IndexMap<String, Value>) -> Self {
        Value::Map(v)
    }
}

// ---------------------------------------------------------------------------
// Path helpers (internal)
// ---------------------------------------------------------------------------

/// A single segment of a path — either a map key or an array index.
#[derive(Debug, PartialEq)]
enum Segment {
    Key(String),
    Index(usize),
}

/// Parse a dot-notation path like `.user.name` or `.items[0].field` into
/// a list of [`Segment`]s.
fn parse_path(path: &str) -> Result<Vec<Segment>, String> {
    let path = path.strip_prefix('.').unwrap_or(path);
    if path.is_empty() {
        return Ok(vec![]);
    }

    let mut segments: Vec<Segment> = Vec::new();
    let mut chars = path.chars().peekable();
    let mut buf = String::new();

    while let Some(&c) = chars.peek() {
        match c {
            '.' => {
                if !buf.is_empty() {
                    segments.push(Segment::Key(std::mem::take(&mut buf)));
                }
                chars.next();
            }
            '[' => {
                if !buf.is_empty() {
                    segments.push(Segment::Key(std::mem::take(&mut buf)));
                }
                chars.next(); // consume '['
                let mut idx_buf = String::new();
                loop {
                    match chars.next() {
                        Some(']') => break,
                        Some(d) if d.is_ascii_digit() => idx_buf.push(d),
                        Some(other) => {
                            return Err(format!("unexpected char '{other}' inside index brackets"))
                        }
                        None => return Err("unclosed '['".to_owned()),
                    }
                }
                let idx: usize = idx_buf.parse().map_err(|e| format!("invalid index: {e}"))?;
                segments.push(Segment::Index(idx));
            }
            _ => {
                buf.push(c);
                chars.next();
            }
        }
    }
    if !buf.is_empty() {
        segments.push(Segment::Key(buf));
    }

    Ok(segments)
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

impl Value {
    /// Deep-merge `other` into `self`.
    ///
    /// When both `self` and `other` are `Map`, keys from `other` are merged
    /// into `self` recursively. For all other variant combinations `other`
    /// simply overwrites `self`.
    pub fn merge(&mut self, other: Value) {
        match (self, other) {
            (Value::Map(ref mut lhs), Value::Map(rhs)) => {
                for (k, v) in rhs {
                    match lhs.get_mut(&k) {
                        Some(existing @ Value::Map(_)) if matches!(v, Value::Map(_)) => {
                            existing.merge(v);
                        }
                        _ => {
                            lhs.insert(k, v);
                        }
                    }
                }
            }
            (this, other) => {
                *this = other;
            }
        }
    }

    /// Access a nested value via dot-notation path.
    ///
    /// Examples: `.user.name`, `.items[0]`, `.items[0].field`
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let segments = parse_path(path).ok()?;
        let mut current = self;
        for seg in &segments {
            match (seg, current) {
                (Segment::Key(k), Value::Map(map)) => {
                    current = map.get(k.as_str())?;
                }
                (Segment::Index(i), Value::Array(arr)) => {
                    current = arr.get(*i)?;
                }
                _ => return None,
            }
        }
        Some(current)
    }

    /// Set a value at a dot-notation path, creating intermediate `Map`s as
    /// needed.
    ///
    /// Returns an error when trying to traverse through a non-Map, non-Array
    /// value or when an array index is out of bounds.
    pub fn set_path(&mut self, path: &str, value: Value) -> Result<(), String> {
        let segments = parse_path(path)?;
        if segments.is_empty() {
            *self = value;
            return Ok(());
        }

        let mut current = self;
        for (i, seg) in segments.iter().enumerate() {
            let is_last = i + 1 == segments.len();
            match seg {
                Segment::Key(k) => {
                    // Ensure current is a Map (create one if Null).
                    if matches!(current, Value::Null) {
                        *current = Value::Map(IndexMap::new());
                    }
                    match current {
                        Value::Map(map) => {
                            if is_last {
                                map.insert(k.clone(), value);
                                return Ok(());
                            }
                            // Create intermediate map if key absent.
                            if !map.contains_key(k.as_str()) {
                                map.insert(k.clone(), Value::Null);
                            }
                            current = map.get_mut(k.as_str()).unwrap();
                        }
                        other => {
                            return Err(format!("expected Map at segment '{k}', found {other:?}"));
                        }
                    }
                }
                Segment::Index(idx) => match current {
                    Value::Array(arr) => {
                        let len = arr.len();
                        let arr_val = arr
                            .get_mut(*idx)
                            .ok_or_else(|| format!("index {idx} out of bounds (len {len})"))?;
                        if is_last {
                            *arr_val = value;
                            return Ok(());
                        }
                        current = arr_val;
                    }
                    other => {
                        return Err(format!("expected Array at index [{idx}], found {other:?}"));
                    }
                },
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    // -- Construction -------------------------------------------------------

    #[test]
    fn construct_null() {
        let v = Value::Null;
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn construct_bool() {
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Bool(false), Value::Bool(false));
    }

    #[test]
    fn construct_int() {
        assert_eq!(Value::Int(42), Value::Int(42));
    }

    #[test]
    fn construct_float() {
        assert_eq!(Value::Float(2.72), Value::Float(2.72));
    }

    #[test]
    fn construct_string() {
        assert_eq!(Value::String("hello".into()), Value::String("hello".into()));
    }

    #[test]
    fn construct_bytes() {
        assert_eq!(
            Value::Bytes(vec![0xde, 0xad]),
            Value::Bytes(vec![0xde, 0xad])
        );
    }

    #[test]
    fn construct_array() {
        let arr = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        assert!(matches!(arr, Value::Array(_)));
    }

    #[test]
    fn construct_map() {
        let mut m = IndexMap::new();
        m.insert("key".to_string(), Value::Int(1));
        let map = Value::Map(m);
        assert!(matches!(map, Value::Map(_)));
    }

    // -- Equality -----------------------------------------------------------

    #[test]
    fn equality_int() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
    }

    #[test]
    fn equality_int_ne_float() {
        // Different variants, even if numeric value is the same.
        assert_ne!(Value::Int(42), Value::Float(42.0));
    }

    // -- Clone / PartialEq --------------------------------------------------

    #[test]
    fn clone_and_eq() {
        let original = Value::Array(vec![Value::Int(1), Value::String("two".into())]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // -- Display ------------------------------------------------------------

    #[test]
    fn display_null() {
        assert_eq!(Value::Null.to_string(), "null");
    }

    #[test]
    fn display_bool() {
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Bool(false).to_string(), "false");
    }

    #[test]
    fn display_int() {
        assert_eq!(Value::Int(42).to_string(), "42");
    }

    #[test]
    fn display_float() {
        assert_eq!(Value::Float(2.72).to_string(), "2.72");
        assert_eq!(Value::Float(1.0).to_string(), "1.0");
    }

    #[test]
    fn display_string() {
        assert_eq!(Value::String("hello".into()).to_string(), "\"hello\"");
    }

    #[test]
    fn display_string_with_escapes() {
        assert_eq!(
            Value::String("say \"hi\"".into()).to_string(),
            r#""say \"hi\"""#
        );
    }

    #[test]
    fn display_bytes() {
        assert_eq!(Value::Bytes(vec![0xca, 0xfe]).to_string(), r#"b"\xca\xfe""#);
    }

    #[test]
    fn display_array() {
        let v = Value::Array(vec![Value::Int(1), Value::Bool(true)]);
        assert_eq!(v.to_string(), "[1, true]");
    }

    #[test]
    fn display_map() {
        let mut m = IndexMap::new();
        m.insert("a".to_string(), Value::Int(1));
        let v = Value::Map(m);
        assert_eq!(v.to_string(), r#"{"a": 1}"#);
    }

    #[test]
    fn display_empty_array() {
        assert_eq!(Value::Array(vec![]).to_string(), "[]");
    }

    #[test]
    fn display_empty_map() {
        assert_eq!(Value::Map(IndexMap::new()).to_string(), "{}");
    }

    // -- From impls ---------------------------------------------------------

    #[test]
    fn from_i64() {
        let v: Value = 42i64.into();
        assert_eq!(v, Value::Int(42));
    }

    #[test]
    fn from_f64() {
        let v: Value = 2.72f64.into();
        assert_eq!(v, Value::Float(2.72));
    }

    #[test]
    fn from_bool() {
        let v: Value = true.into();
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn from_str_ref() {
        let v: Value = "hello".into();
        assert_eq!(v, Value::String("hello".into()));
    }

    #[test]
    fn from_string() {
        let v: Value = String::from("world").into();
        assert_eq!(v, Value::String("world".into()));
    }

    #[test]
    fn from_vec_value() {
        let v: Value = vec![Value::Int(1)].into();
        assert_eq!(v, Value::Array(vec![Value::Int(1)]));
    }

    #[test]
    fn from_indexmap() {
        let mut m = IndexMap::new();
        m.insert("k".to_string(), Value::Null);
        let v: Value = m.clone().into();
        assert_eq!(v, Value::Map(m));
    }

    // -- Nested structures --------------------------------------------------

    #[test]
    fn nested_map_array_map() {
        let mut inner_map = IndexMap::new();
        inner_map.insert("id".to_string(), Value::Int(1));

        let arr = Value::Array(vec![Value::Map(inner_map.clone())]);

        let mut outer = IndexMap::new();
        outer.insert("items".to_string(), arr);

        let v = Value::Map(outer);
        assert_eq!(v.to_string(), r#"{"items": [{"id": 1}]}"#);
    }

    // -- Merge --------------------------------------------------------------

    #[test]
    fn merge_non_maps_overwrites() {
        let mut a = Value::Int(1);
        a.merge(Value::Int(2));
        assert_eq!(a, Value::Int(2));
    }

    #[test]
    fn merge_replaces_non_map_with_map() {
        let mut a = Value::Int(1);
        let mut m = IndexMap::new();
        m.insert("x".to_string(), Value::Int(10));
        a.merge(Value::Map(m.clone()));
        assert_eq!(a, Value::Map(m));
    }

    #[test]
    fn merge_shallow_maps() {
        let mut m1 = IndexMap::new();
        m1.insert("a".to_string(), Value::Int(1));
        m1.insert("b".to_string(), Value::Int(2));

        let mut m2 = IndexMap::new();
        m2.insert("b".to_string(), Value::Int(20));
        m2.insert("c".to_string(), Value::Int(3));

        let mut v1 = Value::Map(m1);
        v1.merge(Value::Map(m2));

        let result = match &v1 {
            Value::Map(m) => m,
            _ => panic!("expected Map"),
        };
        assert_eq!(result.get("a"), Some(&Value::Int(1)));
        assert_eq!(result.get("b"), Some(&Value::Int(20)));
        assert_eq!(result.get("c"), Some(&Value::Int(3)));
    }

    #[test]
    fn merge_deep_nested_maps() {
        // {nested: {a: 1, b: 2}}
        let mut inner1 = IndexMap::new();
        inner1.insert("a".to_string(), Value::Int(1));
        inner1.insert("b".to_string(), Value::Int(2));
        let mut m1 = IndexMap::new();
        m1.insert("nested".to_string(), Value::Map(inner1));

        // {nested: {b: 20, c: 3}}
        let mut inner2 = IndexMap::new();
        inner2.insert("b".to_string(), Value::Int(20));
        inner2.insert("c".to_string(), Value::Int(3));
        let mut m2 = IndexMap::new();
        m2.insert("nested".to_string(), Value::Map(inner2));

        let mut v1 = Value::Map(m1);
        v1.merge(Value::Map(m2));

        // nested.a = 1, nested.b = 20, nested.c = 3
        let nested = match &v1 {
            Value::Map(m) => match m.get("nested") {
                Some(Value::Map(inner)) => inner,
                _ => panic!("expected nested Map"),
            },
            _ => panic!("expected Map"),
        };
        assert_eq!(nested.get("a"), Some(&Value::Int(1)));
        assert_eq!(nested.get("b"), Some(&Value::Int(20)));
        assert_eq!(nested.get("c"), Some(&Value::Int(3)));
    }

    // -- get_path -----------------------------------------------------------

    #[test]
    fn get_path_simple() {
        let mut m = IndexMap::new();
        m.insert("name".to_string(), Value::String("Alice".into()));
        let v = Value::Map(m);
        assert_eq!(v.get_path(".name"), Some(&Value::String("Alice".into())));
    }

    #[test]
    fn get_path_nested() {
        let mut inner = IndexMap::new();
        inner.insert("c".to_string(), Value::Int(99));
        let mut mid = IndexMap::new();
        mid.insert("b".to_string(), Value::Map(inner));
        let mut outer = IndexMap::new();
        outer.insert("a".to_string(), Value::Map(mid));
        let v = Value::Map(outer);

        assert_eq!(v.get_path(".a.b.c"), Some(&Value::Int(99)));
    }

    #[test]
    fn get_path_array_index() {
        let arr = Value::Array(vec![Value::Int(10), Value::Int(20)]);
        let mut m = IndexMap::new();
        m.insert("arr".to_string(), arr);
        let v = Value::Map(m);

        assert_eq!(v.get_path(".arr[0]"), Some(&Value::Int(10)));
        assert_eq!(v.get_path(".arr[1]"), Some(&Value::Int(20)));
        assert_eq!(v.get_path(".arr[2]"), None);
    }

    #[test]
    fn get_path_array_then_field() {
        let mut item = IndexMap::new();
        item.insert("field".to_string(), Value::String("val".into()));
        let arr = Value::Array(vec![Value::Map(item)]);
        let mut m = IndexMap::new();
        m.insert("arr".to_string(), arr);
        let v = Value::Map(m);

        assert_eq!(
            v.get_path(".arr[0].field"),
            Some(&Value::String("val".into()))
        );
    }

    #[test]
    fn get_path_missing_key() {
        let v = Value::Map(IndexMap::new());
        assert_eq!(v.get_path(".missing"), None);
    }

    #[test]
    fn get_path_on_non_map() {
        let v = Value::Int(42);
        assert_eq!(v.get_path(".foo"), None);
    }

    #[test]
    fn get_path_empty() {
        let v = Value::Int(42);
        assert_eq!(v.get_path(""), Some(&v));
    }

    // -- set_path -----------------------------------------------------------

    #[test]
    fn set_path_simple() {
        let mut v = Value::Map(IndexMap::new());
        v.set_path(".name", Value::String("Bob".into())).unwrap();
        assert_eq!(v.get_path(".name"), Some(&Value::String("Bob".into())));
    }

    #[test]
    fn set_path_creates_intermediate_maps() {
        let mut v = Value::Null;
        v.set_path(".a.b.c", Value::Int(42)).unwrap();
        assert_eq!(v.get_path(".a.b.c"), Some(&Value::Int(42)));
    }

    #[test]
    fn set_path_overwrites_existing() {
        let mut m = IndexMap::new();
        m.insert("x".to_string(), Value::Int(1));
        let mut v = Value::Map(m);
        v.set_path(".x", Value::Int(2)).unwrap();
        assert_eq!(v.get_path(".x"), Some(&Value::Int(2)));
    }

    #[test]
    fn set_path_array_index() {
        let arr = Value::Array(vec![Value::Int(0), Value::Int(0)]);
        let mut m = IndexMap::new();
        m.insert("arr".to_string(), arr);
        let mut v = Value::Map(m);
        v.set_path(".arr[1]", Value::Int(99)).unwrap();
        assert_eq!(v.get_path(".arr[1]"), Some(&Value::Int(99)));
    }

    #[test]
    fn set_path_array_out_of_bounds() {
        let arr = Value::Array(vec![Value::Int(0)]);
        let mut m = IndexMap::new();
        m.insert("arr".to_string(), arr);
        let mut v = Value::Map(m);
        assert!(v.set_path(".arr[5]", Value::Int(1)).is_err());
    }

    #[test]
    fn set_path_nested_in_array() {
        let mut item = IndexMap::new();
        item.insert("x".to_string(), Value::Int(0));
        let arr = Value::Array(vec![Value::Map(item)]);
        let mut m = IndexMap::new();
        m.insert("arr".to_string(), arr);
        let mut v = Value::Map(m);
        v.set_path(".arr[0].x", Value::Int(7)).unwrap();
        assert_eq!(v.get_path(".arr[0].x"), Some(&Value::Int(7)));
    }

    #[test]
    fn set_path_error_on_traverse_non_map() {
        let mut m = IndexMap::new();
        m.insert("x".to_string(), Value::Int(5));
        let mut v = Value::Map(m);
        assert!(v.set_path(".x.y", Value::Int(1)).is_err());
    }

    #[test]
    fn set_path_empty_replaces_root() {
        let mut v = Value::Int(1);
        v.set_path("", Value::Int(2)).unwrap();
        assert_eq!(v, Value::Int(2));
    }

    // -- IndexMap preserves insertion order ---------------------------------

    #[test]
    fn indexmap_preserves_order() {
        let mut m = IndexMap::new();
        m.insert("z".to_string(), Value::Int(1));
        m.insert("a".to_string(), Value::Int(2));
        m.insert("m".to_string(), Value::Int(3));
        let v = Value::Map(m);
        assert_eq!(v.to_string(), r#"{"z": 1, "a": 2, "m": 3}"#);
    }
}
