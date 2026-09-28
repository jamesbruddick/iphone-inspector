//! Small helpers for reading loosely-typed plist values the way the phone actually sends them.

use chrono::{DateTime, SecondsFormat, Utc};
use plist::{Dictionary, Value};
use serde_json::{Map, Value as Json};

/// Any number the phone sent, integer or real.
pub fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Integer(i) => i.as_signed().map(|n| n as f64).or_else(|| i.as_unsigned().map(|n| n as f64)),
        Value::Real(r) if r.is_finite() => Some(*r),
        _ => None,
    }
}

/// A non-blank string, trimmed.
pub fn text(v: Option<&Value>) -> Option<String> {
    v?.as_string().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

pub fn boolean(v: Option<&Value>) -> Option<bool> {
    v?.as_boolean()
}

/// Depth-first search for the first of `keys` anywhere in a nested plist.
pub fn find<'a>(dict: Option<&'a Dictionary>, keys: &[&str]) -> Option<&'a Value> {
    fn in_value<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a Value> {
        match v {
            Value::Dictionary(d) => in_dict(d, keys),
            Value::Array(items) => items.iter().find_map(|item| in_value(item, keys)),
            _ => None,
        }
    }
    fn in_dict<'a>(d: &'a Dictionary, keys: &[&str]) -> Option<&'a Value> {
        keys.iter().find_map(|k| d.get(k)).or_else(|| d.values().find_map(|v| in_value(v, keys)))
    }
    in_dict(dict?, keys)
}

/// A plist value as a table cell: what a person would expect to read for it, or None when it is
/// not something worth a row (a nested dictionary, a blob).
pub fn display(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Boolean(b) => Some(yes_no(*b)),
        Value::Integer(i) => Some(i.to_string()),
        Value::Real(r) => Some(fmt_num(*r)),
        Value::Date(d) => Some(iso(DateTime::<Utc>::from(std::time::SystemTime::from(*d)))),
        _ => None,
    }
}

pub fn yes_no(b: bool) -> String {
    if b { "Yes" } else { "No" }.into()
}

/// A number as JavaScript would print it: no trailing ".0" on whole values.
pub fn fmt_num(n: f64) -> String {
    format!("{n}")
}

/// Fixed decimals, like `Number.prototype.toFixed`.
pub fn fixed(n: f64, places: usize) -> String {
    format!("{n:.places$}")
}

pub fn gb(bytes: Option<f64>) -> Option<String> {
    let n = bytes?;
    Some(format!("{} GB", fixed(n / 1e9, if n >= 1e11 { 0 } else { 1 })))
}

pub fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Plists carry blobs and dates; make them printable and JSON-safe for the raw dump.
pub fn to_json(v: &Value) -> Json {
    match v {
        Value::Dictionary(d) => dict_to_json(d),
        Value::Array(items) => Json::Array(items.iter().map(to_json).collect()),
        Value::Boolean(b) => Json::Bool(*b),
        Value::Integer(i) => i.as_signed().map(Json::from).or_else(|| i.as_unsigned().map(Json::from)).unwrap_or(Json::Null),
        Value::Real(r) => serde_json::Number::from_f64(*r).map_or(Json::Null, Json::Number),
        Value::String(s) => Json::String(s.clone()),
        Value::Date(d) => Json::String(iso(DateTime::<Utc>::from(std::time::SystemTime::from(*d)))),
        Value::Data(bytes) if bytes.len() > 64 => Json::String(format!("<{} bytes>", bytes.len())),
        Value::Data(bytes) => Json::String(bytes.iter().map(|b| format!("{b:02x}")).collect()),
        Value::Uid(u) => Json::from(u.get()),
        _ => Json::Null,
    }
}

pub fn dict_to_json(d: &Dictionary) -> Json {
    Json::Object(d.iter().map(|(k, v)| (k.clone(), to_json(v))).collect::<Map<_, _>>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_like_javascript() {
        assert_eq!(fmt_num(2.0), "2");
        assert_eq!(fmt_num(0.5), "0.5");
        // Rounding a small negative gives -0.0, which must not print as "-0".
        assert_eq!(fmt_num((-0.4f64).round() + 0.0), "0");
        assert_eq!(fixed(87.456, 1), "87.5");
        assert_eq!(gb(Some(113_400_000_000.0)).as_deref(), Some("113 GB"));
        assert_eq!(gb(Some(5_430_000_000.0)).as_deref(), Some("5.4 GB"));
    }

    #[test]
    fn deep_find() {
        let mut inner = Dictionary::new();
        inner.insert("CycleCount".into(), Value::Integer(412.into()));
        let mut outer = Dictionary::new();
        outer.insert("IORegistry".into(), Value::Array(vec![Value::Dictionary(inner)]));
        assert_eq!(num(find(Some(&outer), &["CycleCount"])), Some(412.0));
        assert_eq!(find(Some(&outer), &["Missing"]), None);
    }
}
