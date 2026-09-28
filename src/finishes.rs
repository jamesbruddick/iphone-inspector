//! Matching a color the phone reports to the nearest finish Apple sold for that model.
//!
//! The table is `web/src/lib/finishes.json`, compiled in, so the UI and this service share one copy.

use std::collections::HashMap;
use std::sync::OnceLock;

use plist::Value;

use crate::value::num;

pub struct Finish {
    pub name: String,
    pub hex: String,
}

fn table() -> &'static HashMap<String, Vec<(String, String)>> {
    static TABLE: OnceLock<HashMap<String, Vec<(String, String)>>> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(include_str!("../web/src/lib/finishes.json")).expect("finishes.json is valid"))
}

fn rgb(hex: &str) -> Option<[i64; 3]> {
    let hex = hex.strip_prefix('#')?;
    let channel = |i: usize| i64::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Turn whatever the phone reports into a hex color, or None.
fn parse_reported_color(v: &Value) -> Option<String> {
    match v {
        Value::Integer(_) | Value::Real(_) => {
            let n = num(Some(v))?;
            (n > 255.0).then(|| format!("#{:06x}", (n as i64) & 0xff_ffff))
        }
        Value::String(s) => {
            let t = s.trim();
            let digits = t.strip_prefix('#').unwrap_or(t);
            let valid = digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit());
            // A bare index like "1" is a model-specific code we cannot decode, so it is not a color.
            let clearly_hex = s.starts_with('#') || digits.chars().any(|c| c.is_ascii_alphabetic());
            (valid && clearly_hex).then(|| format!("#{digits}"))
        }
        Value::Dictionary(d) => {
            let pick = |keys: [&str; 3]| keys.iter().find_map(|k| d.get(k));
            let r = num(pick(["red", "Red", "r"]))?;
            let g = num(pick(["green", "Green", "g"]))?;
            let b = num(pick(["blue", "Blue", "b"]))?;
            let scale = if r <= 1.0 && g <= 1.0 && b <= 1.0 { 255.0 } else { 1.0 };
            Some(format!("#{:02x}{:02x}{:02x}", (r * scale).round() as i64, (g * scale).round() as i64, (b * scale).round() as i64))
        }
        _ => None,
    }
}

/// The first candidate that parses as a color, matched to the nearest finish for this model.
pub fn detect(model_name: &str, candidates: &[Option<&Value>]) -> Option<Finish> {
    let finishes = table().get(model_name);
    for candidate in candidates.iter().flatten() {
        let Some(hex) = parse_reported_color(candidate) else { continue };
        let Some(finishes) = finishes.filter(|f| !f.is_empty()) else {
            return Some(Finish { name: "Reported color".into(), hex });
        };
        let [r, g, b] = rgb(&hex)?;
        let nearest = finishes.iter().filter_map(|(name, fhex)| {
            let [r2, g2, b2] = rgb(fhex)?;
            Some(((r - r2).pow(2) + (g - g2).pow(2) + (b - b2).pow(2), name, fhex))
        });
        if let Some((_, name, fhex)) = nearest.min_by_key(|x| x.0) {
            return Some(Finish { name: name.clone(), hex: fhex.clone() });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_new_models() {
        for model in ["iPhone 17e", "iPhone 18 Pro", "iPhone 18 Pro Max", "iPhone Duo"] {
            assert!(table().contains_key(model), "{model} missing from finishes.json");
        }
    }

    #[test]
    fn nearest_finish() {
        let burgundy = Value::String("#501a25".into());
        assert_eq!(detect("iPhone 18 Pro", &[None, Some(&burgundy)]).unwrap().name, "Burgundy");
        // A bare color index is not a color.
        assert!(detect("iPhone 18 Pro", &[Some(&Value::String("1".into()))]).is_none());
        let packed = Value::Integer(0x00e8_f2feu64.into());
        assert_eq!(detect("iPhone 18 Pro", &[Some(&packed)]).unwrap().name, "Glacier");
    }
}
