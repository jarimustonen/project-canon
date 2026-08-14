//! A minimal, escape-correct JSON writer.
//!
//! The workspace is deliberately dependency-free (core keeps serde a "deferred seam"), and the
//! `doctor` report is a small, fixed shape we fully control — so a ~one-screen [`Json`] value
//! type with a correct string escaper is preferred over pulling in serde/serde_json. If the
//! `--json` surface grows across verbs, revisit and adopt serde.

use std::fmt::{self, Write as _};

/// A JSON value. Object key order is preserved (insertion order) so the emitted payload is
/// stable and diff-friendly.
#[derive(Debug, Clone)]
pub enum Json {
    Null,
    Bool(bool),
    /// An integer. `doctor` never emits fractional numbers, so a single `i64` covers every case.
    Int(i64),
    Str(String),
    Array(Vec<Json>),
    /// Insertion-ordered object.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// A string value from anything `Into<String>`.
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// `Some(v)` → the value; `None` → `null`.
    pub fn opt_str(s: Option<impl Into<String>>) -> Json {
        match s {
            Some(v) => Json::Str(v.into()),
            None => Json::Null,
        }
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                // `write!` to a String is infallible.
                let _ = write!(out, "{n}");
            }
            Json::Str(s) => write_escaped(s, out),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(fields) => {
                out.push('{');
                for (i, (key, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_escaped(key, out);
                    out.push(':');
                    val.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Compact (no-whitespace) JSON. `Json::to_string()` (from `Display`) is the canonical serializer.
impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(&mut out);
        f.write_str(&out)
    }
}

/// Write a JSON string literal (surrounding quotes + RFC 8259 escaping) into `out`.
fn write_escaped(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            // Other C0 control characters must be `\u00XX`-escaped.
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_serialize() {
        assert_eq!(Json::Null.to_string(), "null");
        assert_eq!(Json::Bool(true).to_string(), "true");
        assert_eq!(Json::Bool(false).to_string(), "false");
        assert_eq!(Json::Int(-7).to_string(), "-7");
        assert_eq!(Json::str("hi").to_string(), "\"hi\"");
    }

    #[test]
    fn strings_are_escaped_correctly() {
        assert_eq!(
            Json::str("a\"b\\c\nd\te").to_string(),
            "\"a\\\"b\\\\c\\nd\\te\""
        );
        // A C0 control char is \u-escaped.
        assert_eq!(Json::str("\u{01}").to_string(), "\"\\u0001\"");
        // A path with a quote (arbitrary target path) round-trips through escaping.
        assert_eq!(Json::str("/tmp/we\"ird").to_string(), "\"/tmp/we\\\"ird\"");
        // Non-ASCII passes through as UTF-8 (valid JSON).
        assert_eq!(Json::str("🚀").to_string(), "\"🚀\"");
    }

    #[test]
    fn arrays_and_objects_preserve_order() {
        let v = Json::Array(vec![Json::Int(1), Json::str("x"), Json::Bool(false)]);
        assert_eq!(v.to_string(), "[1,\"x\",false]");

        let o = Json::Object(vec![
            ("b".to_string(), Json::Int(2)),
            ("a".to_string(), Json::Int(1)),
        ]);
        // Insertion order, not sorted.
        assert_eq!(o.to_string(), "{\"b\":2,\"a\":1}");
    }

    #[test]
    fn opt_str_maps_none_to_null() {
        assert_eq!(Json::opt_str(None::<String>).to_string(), "null");
        assert_eq!(Json::opt_str(Some("v")).to_string(), "\"v\"");
    }
}
