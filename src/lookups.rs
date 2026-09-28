//! Turns raw device identifiers into readable information.

use chrono::{Datelike, NaiveDate, Utc};

/// ProductType -> (marketing name, release year, chip, screen size).
///
/// Identifiers from Apple's firmware naming, as recorded by AppleDB. Where one model ships as two
/// hardware variants (the iPhone 18 Pro Max has a US and a global board), both are listed.
const MODELS: &[(&str, &str, i32, &str, &str)] = &[
    ("iPhone10,1", "iPhone 8", 2017, "A11 Bionic", "4.7\""),
    ("iPhone10,4", "iPhone 8", 2017, "A11 Bionic", "4.7\""),
    ("iPhone10,2", "iPhone 8 Plus", 2017, "A11 Bionic", "5.5\""),
    ("iPhone10,5", "iPhone 8 Plus", 2017, "A11 Bionic", "5.5\""),
    ("iPhone10,3", "iPhone X", 2017, "A11 Bionic", "5.8\""),
    ("iPhone10,6", "iPhone X", 2017, "A11 Bionic", "5.8\""),
    ("iPhone11,2", "iPhone XS", 2018, "A12 Bionic", "5.8\""),
    ("iPhone11,4", "iPhone XS Max", 2018, "A12 Bionic", "6.5\""),
    ("iPhone11,6", "iPhone XS Max", 2018, "A12 Bionic", "6.5\""),
    ("iPhone11,8", "iPhone XR", 2018, "A12 Bionic", "6.1\""),
    ("iPhone12,1", "iPhone 11", 2019, "A13 Bionic", "6.1\""),
    ("iPhone12,3", "iPhone 11 Pro", 2019, "A13 Bionic", "5.8\""),
    ("iPhone12,5", "iPhone 11 Pro Max", 2019, "A13 Bionic", "6.5\""),
    ("iPhone12,8", "iPhone SE (2nd generation)", 2020, "A13 Bionic", "4.7\""),
    ("iPhone13,1", "iPhone 12 mini", 2020, "A14 Bionic", "5.4\""),
    ("iPhone13,2", "iPhone 12", 2020, "A14 Bionic", "6.1\""),
    ("iPhone13,3", "iPhone 12 Pro", 2020, "A14 Bionic", "6.1\""),
    ("iPhone13,4", "iPhone 12 Pro Max", 2020, "A14 Bionic", "6.7\""),
    ("iPhone14,4", "iPhone 13 mini", 2021, "A15 Bionic", "5.4\""),
    ("iPhone14,5", "iPhone 13", 2021, "A15 Bionic", "6.1\""),
    ("iPhone14,2", "iPhone 13 Pro", 2021, "A15 Bionic", "6.1\""),
    ("iPhone14,3", "iPhone 13 Pro Max", 2021, "A15 Bionic", "6.7\""),
    ("iPhone14,6", "iPhone SE (3rd generation)", 2022, "A15 Bionic", "4.7\""),
    ("iPhone14,7", "iPhone 14", 2022, "A15 Bionic", "6.1\""),
    ("iPhone14,8", "iPhone 14 Plus", 2022, "A15 Bionic", "6.7\""),
    ("iPhone15,2", "iPhone 14 Pro", 2022, "A16 Bionic", "6.1\""),
    ("iPhone15,3", "iPhone 14 Pro Max", 2022, "A16 Bionic", "6.7\""),
    ("iPhone15,4", "iPhone 15", 2023, "A16 Bionic", "6.1\""),
    ("iPhone15,5", "iPhone 15 Plus", 2023, "A16 Bionic", "6.7\""),
    ("iPhone16,1", "iPhone 15 Pro", 2023, "A17 Pro", "6.1\""),
    ("iPhone16,2", "iPhone 15 Pro Max", 2023, "A17 Pro", "6.7\""),
    ("iPhone17,3", "iPhone 16", 2024, "A18", "6.1\""),
    ("iPhone17,4", "iPhone 16 Plus", 2024, "A18", "6.7\""),
    ("iPhone17,1", "iPhone 16 Pro", 2024, "A18 Pro", "6.3\""),
    ("iPhone17,2", "iPhone 16 Pro Max", 2024, "A18 Pro", "6.9\""),
    ("iPhone17,5", "iPhone 16e", 2025, "A18", "6.1\""),
    ("iPhone18,3", "iPhone 17", 2025, "A19", "6.3\""),
    ("iPhone18,4", "iPhone Air", 2025, "A19 Pro", "6.5\""),
    ("iPhone18,1", "iPhone 17 Pro", 2025, "A19 Pro", "6.3\""),
    ("iPhone18,2", "iPhone 17 Pro Max", 2025, "A19 Pro", "6.9\""),
    ("iPhone18,5", "iPhone 17e", 2026, "A19", "6.1\""),
    ("iPhone19,2", "iPhone 18 Pro", 2026, "A20 Pro", "6.3\""),
    ("iPhone19,3", "iPhone 18 Pro Max", 2026, "A20 Pro", "6.9\""),
    ("iPhone19,7", "iPhone 18 Pro Max", 2026, "A20 Pro", "6.9\""),
    ("iPhone19,4", "iPhone Duo", 2026, "A20 Pro", "7.6\" inner, 5.4\" outer"),
];

/// Suffix of RegionInfo, e.g. "LL/A".
const REGIONS: &[(&str, &str)] = &[
    ("LL/A", "United States"),
    ("C/A", "Canada"),
    ("B/A", "United Kingdom / Ireland"),
    ("ZD/A", "Europe (Multiple Countries)"),
    ("FN/A", "France"),
    ("DN/A", "Germany"),
    ("TY/A", "Italy"),
    ("QL/A", "Spain"),
    ("PO/A", "Portugal"),
    ("NF/A", "Belgium / France"),
    ("X/A", "Australia / New Zealand"),
    ("J/A", "Japan"),
    ("KH/A", "South Korea"),
    ("CH/A", "China Mainland"),
    ("ZA/A", "Singapore"),
    ("ZP/A", "Hong Kong / Macau"),
    ("TA/A", "Taiwan"),
    ("HN/A", "India"),
    ("LZ/A", "Latin America"),
    ("AE/A", "UAE / Gulf"),
    ("AB/A", "Middle East"),
    ("RU/A", "Russia"),
    ("TU/A", "Turkey"),
    ("BR/A", "Brazil"),
    ("E/A", "Mexico"),
    ("LA/A", "Latin America"),
    ("VN/A", "Vietnam"),
    ("TH/A", "Thailand"),
    ("MY/A", "Malaysia"),
    ("PP/A", "Philippines"),
    ("ID/A", "Indonesia"),
];

pub struct PartPrefix {
    pub label: &'static str,
    pub short: &'static str,
    pub level: &'static str,
}

/// First character of the model number. `3` is a store demo unit, which has seen far heavier use
/// than a retail phone.
const PART_PREFIX: &[(char, PartPrefix)] = &[
    ('M', PartPrefix { label: "Retail Unit (Sold New)", short: "Retail", level: "ok" }),
    ('F', PartPrefix { label: "Apple Certified Refurbished", short: "Refurbished", level: "warn" }),
    ('N', PartPrefix { label: "Replacement Unit (Apple Repair Swap)", short: "Replacement", level: "warn" }),
    ('P', PartPrefix { label: "Personalized / Engraved Retail Unit", short: "Engraved", level: "ok" }),
    ('3', PartPrefix { label: "Store Demo Unit", short: "Demo Unit", level: "fail" }),
];

pub struct ModelInfo {
    pub name: String,
    pub year: Option<i32>,
    pub chip: Option<&'static str>,
    pub screen: Option<&'static str>,
}

pub fn model(product_type: Option<&str>) -> ModelInfo {
    match product_type.and_then(|p| MODELS.iter().find(|m| m.0 == p)) {
        Some(&(_, name, year, chip, screen)) => ModelInfo { name: name.into(), year: Some(year), chip: Some(chip), screen: Some(screen) },
        None => ModelInfo { name: product_type.filter(|p| !p.is_empty()).unwrap_or("Unknown iPhone").into(), year: None, chip: None, screen: None },
    }
}

pub fn region_name(region: Option<&str>) -> Option<&'static str> {
    let region = region?;
    REGIONS.iter().find(|r| r.0 == region).map(|r| r.1)
}

pub fn part_prefix(model_number: Option<&str>) -> Option<&'static PartPrefix> {
    let first = model_number?.chars().next()?;
    PART_PREFIX.iter().find(|p| p.0 == first).map(|p| &p.1)
}

/// Decode the manufacture date from the older 12-character serial format. Serials issued since
/// roughly 2021 are randomized and carry no date, so this returns None for most modern phones.
pub fn serial_date(serial: Option<&str>, release_year: Option<i32>) -> Option<NaiveDate> {
    const YEAR_CODES: &str = "CDFGHJKLMNPQRSTVWXYZ";
    const WEEK_CODES: &str = "123456789CDFGHJKLMNPQRTVWXY";

    let serial: Vec<char> = serial?.chars().collect();
    if serial.len() != 12 {
        return None;
    }
    let yi = YEAR_CODES.find(serial[3])? as i32;
    let wi = WEEK_CODES.find(serial[4])? as i64;
    let offset = yi / 2;
    let second_half = yi % 2 == 1;
    let mut year = 2010 + offset;
    if release_year.is_some_and(|r| year < r) {
        year = 2020 + offset;
    }
    let week = wi + 1 + if second_half { 26 } else { 0 };
    let date = NaiveDate::from_ymd_opt(year, 1, 1)? + chrono::Duration::days((week - 1) * 7);
    (date <= Utc::now().date_naive()).then_some(date)
}

/// Years since `made`, or since the middle of the release year when the serial carries no date.
pub fn age_years(made: Option<NaiveDate>, release_year: Option<i32>) -> Option<f64> {
    let now = Utc::now();
    match (made, release_year) {
        (Some(d), _) => Some((now.date_naive() - d).num_seconds() as f64 / (365.25 * 86_400.0)),
        (None, Some(y)) => Some(f64::from(now.year() - y) + 0.5),
        _ => None,
    }
}

pub fn age_words(years: Option<f64>) -> Option<String> {
    let whole = years?.floor().max(0.0) as i64;
    Some(match whole {
        0 => "Less Than a Year".into(),
        1 => "About 1 year".into(),
        n => format!("About {n} years"),
    })
}

pub fn month_year(date: NaiveDate) -> String {
    date.format("%B %Y").to_string()
}

/// Round a byte count up to the capacity Apple actually sells.
pub fn marketing_capacity(bytes: Option<f64>) -> Option<String> {
    let gb = bytes? / 1e9;
    let size = [16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0, 2048.0].into_iter().find(|&x| gb <= x * 1.02).unwrap_or(gb.round());
    Some(if size >= 1024.0 { format!("{} TB", size / 1024.0) } else { format!("{size} GB") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_models() {
        assert_eq!(model(Some("iPhone19,2")).name, "iPhone 18 Pro");
        assert_eq!(model(Some("iPhone19,3")).name, "iPhone 18 Pro Max");
        assert_eq!(model(Some("iPhone19,7")).name, "iPhone 18 Pro Max");
        assert_eq!(model(Some("iPhone19,4")).name, "iPhone Duo");
        assert_eq!(model(Some("iPhone18,5")).name, "iPhone 17e");
        assert_eq!(model(Some("iPhone18,1")).name, "iPhone 17 Pro");
        assert_eq!(model(Some("iPhone99,9")).name, "iPhone99,9");
        assert_eq!(model(None).name, "Unknown iPhone");
    }

    #[test]
    fn capacity() {
        assert_eq!(marketing_capacity(Some(127_989_493_760.0)).as_deref(), Some("128 GB"));
        assert_eq!(marketing_capacity(Some(1_000_000_000_000.0)).as_deref(), Some("1 TB"));
        assert_eq!(marketing_capacity(Some(1_999_000_000_000.0)).as_deref(), Some("2 TB"));
        assert_eq!(marketing_capacity(None), None);
    }

    #[test]
    fn old_serials_decode() {
        // Year code 'L' (index 7 -> 2013, second half), week code '5' -> week 5 + 26.
        let date = serial_date(Some("C39L5ABCDEFG"), Some(2013)).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2013, 1, 1).unwrap() + chrono::Duration::days(30 * 7));
        assert_eq!(serial_date(Some("RANDOMIZED1"), None), None);
    }

    #[test]
    fn prefixes() {
        assert_eq!(part_prefix(Some("MQ9X3")).unwrap().short, "Retail");
        assert_eq!(part_prefix(Some("3H123")).unwrap().level, "fail");
        assert!(part_prefix(Some("ZZZ")).is_none());
    }
}
