//! Language and region codes in English, standing in for the `Intl.DisplayNames` the Node server
//! used. Unknown codes come back as they are, which is also what `Intl` does.

const LANGUAGES: &[(&str, &str)] = &[
    ("af", "Afrikaans"),
    ("ar", "Arabic"),
    ("bg", "Bulgarian"),
    ("bn", "Bangla"),
    ("ca", "Catalan"),
    ("cs", "Czech"),
    ("cy", "Welsh"),
    ("da", "Danish"),
    ("de", "German"),
    ("el", "Greek"),
    ("en", "English"),
    ("es", "Spanish"),
    ("et", "Estonian"),
    ("eu", "Basque"),
    ("fa", "Persian"),
    ("fi", "Finnish"),
    ("fil", "Filipino"),
    ("fr", "French"),
    ("ga", "Irish"),
    ("gl", "Galician"),
    ("gu", "Gujarati"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hr", "Croatian"),
    ("hu", "Hungarian"),
    ("hy", "Armenian"),
    ("id", "Indonesian"),
    ("is", "Icelandic"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("ka", "Georgian"),
    ("kk", "Kazakh"),
    ("km", "Khmer"),
    ("kn", "Kannada"),
    ("ko", "Korean"),
    ("lt", "Lithuanian"),
    ("lv", "Latvian"),
    ("mk", "Macedonian"),
    ("ml", "Malayalam"),
    ("mn", "Mongolian"),
    ("mr", "Marathi"),
    ("ms", "Malay"),
    ("my", "Burmese"),
    ("nb", "Norwegian Bokmål"),
    ("ne", "Nepali"),
    ("nl", "Dutch"),
    ("no", "Norwegian"),
    ("or", "Odia"),
    ("pa", "Punjabi"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("si", "Sinhala"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("sq", "Albanian"),
    ("sr", "Serbian"),
    ("sv", "Swedish"),
    ("sw", "Swahili"),
    ("ta", "Tamil"),
    ("te", "Telugu"),
    ("th", "Thai"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("uz", "Uzbek"),
    ("vi", "Vietnamese"),
    ("yue", "Cantonese"),
    ("zh", "Chinese"),
    ("zu", "Zulu"),
];

const REGIONS: &[(&str, &str)] = &[
    ("AE", "United Arab Emirates"),
    ("AR", "Argentina"),
    ("AT", "Austria"),
    ("AU", "Australia"),
    ("BD", "Bangladesh"),
    ("BE", "Belgium"),
    ("BG", "Bulgaria"),
    ("BH", "Bahrain"),
    ("BR", "Brazil"),
    ("CA", "Canada"),
    ("CH", "Switzerland"),
    ("CL", "Chile"),
    ("CN", "China"),
    ("CO", "Colombia"),
    ("CR", "Costa Rica"),
    ("CY", "Cyprus"),
    ("CZ", "Czechia"),
    ("DE", "Germany"),
    ("DK", "Denmark"),
    ("DO", "Dominican Republic"),
    ("DZ", "Algeria"),
    ("EC", "Ecuador"),
    ("EE", "Estonia"),
    ("EG", "Egypt"),
    ("ES", "Spain"),
    ("FI", "Finland"),
    ("FR", "France"),
    ("GB", "United Kingdom"),
    ("GE", "Georgia"),
    ("GH", "Ghana"),
    ("GR", "Greece"),
    ("GT", "Guatemala"),
    ("HK", "Hong Kong SAR China"),
    ("HR", "Croatia"),
    ("HU", "Hungary"),
    ("ID", "Indonesia"),
    ("IE", "Ireland"),
    ("IL", "Israel"),
    ("IN", "India"),
    ("IQ", "Iraq"),
    ("IS", "Iceland"),
    ("IT", "Italy"),
    ("JM", "Jamaica"),
    ("JO", "Jordan"),
    ("JP", "Japan"),
    ("KE", "Kenya"),
    ("KH", "Cambodia"),
    ("KR", "South Korea"),
    ("KW", "Kuwait"),
    ("KZ", "Kazakhstan"),
    ("LB", "Lebanon"),
    ("LK", "Sri Lanka"),
    ("LT", "Lithuania"),
    ("LU", "Luxembourg"),
    ("LV", "Latvia"),
    ("MA", "Morocco"),
    ("MO", "Macao SAR China"),
    ("MT", "Malta"),
    ("MX", "Mexico"),
    ("MY", "Malaysia"),
    ("NG", "Nigeria"),
    ("NL", "Netherlands"),
    ("NO", "Norway"),
    ("NP", "Nepal"),
    ("NZ", "New Zealand"),
    ("OM", "Oman"),
    ("PA", "Panama"),
    ("PE", "Peru"),
    ("PH", "Philippines"),
    ("PK", "Pakistan"),
    ("PL", "Poland"),
    ("PR", "Puerto Rico"),
    ("PT", "Portugal"),
    ("PY", "Paraguay"),
    ("QA", "Qatar"),
    ("RO", "Romania"),
    ("RS", "Serbia"),
    ("RU", "Russia"),
    ("SA", "Saudi Arabia"),
    ("SE", "Sweden"),
    ("SG", "Singapore"),
    ("SI", "Slovenia"),
    ("SK", "Slovakia"),
    ("SV", "El Salvador"),
    ("TH", "Thailand"),
    ("TN", "Tunisia"),
    ("TR", "Türkiye"),
    ("TW", "Taiwan"),
    ("UA", "Ukraine"),
    ("US", "United States"),
    ("UY", "Uruguay"),
    ("UZ", "Uzbekistan"),
    ("VE", "Venezuela"),
    ("VN", "Vietnam"),
    ("ZA", "South Africa"),
];

/// "en_US" -> "English (US)": the language, with the region as its two-letter code. Short enough
/// for a summary card, where "English (United States)" would be cut off on a phone.
pub fn language_short(code: Option<&str>) -> Option<String> {
    let code = code.filter(|c| !c.is_empty())?;
    let tag = code.replace('_', "-");
    let parts: Vec<&str> = tag.split('-').collect();
    let base = language_name(Some(parts[0]))?;
    let region = parts.iter().skip(1).find(|r| r.len() == 2 && r.chars().all(|c| c.is_ascii_uppercase()));
    Some(match region {
        Some(r) => format!("{base} ({r})"),
        None => base,
    })
}

/// "en_GB" -> "English (United Kingdom)", "zh-Hans" -> "Simplified Chinese".
pub fn language_name(code: Option<&str>) -> Option<String> {
    let code = code.filter(|c| !c.is_empty())?;
    let tag = code.replace('_', "-");
    let parts: Vec<&str> = tag.split('-').collect();
    let base = parts[0].to_lowercase();

    let script = parts.iter().skip(1).find(|p| p.len() == 4).copied();
    let name = match (base.as_str(), script) {
        ("zh", Some("Hans")) => "Simplified Chinese".to_string(),
        ("zh", Some("Hant")) => "Traditional Chinese".to_string(),
        _ => match LANGUAGES.iter().find(|l| l.0 == base) {
            Some(l) => l.1.to_string(),
            None => return Some(code.to_string()),
        },
    };

    let region = parts.get(1).filter(|r| r.len() == 2 && r.chars().all(|c| c.is_ascii_uppercase()));
    Some(match region {
        Some(r) => format!("{name} ({})", REGIONS.iter().find(|x| x.0 == *r).map_or(*r, |x| x.1)),
        None => name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(language_name(Some("en_GB")).as_deref(), Some("English (United Kingdom)"));
        assert_eq!(language_name(Some("en")).as_deref(), Some("English"));
        assert_eq!(language_name(Some("zh-Hans")).as_deref(), Some("Simplified Chinese"));
        assert_eq!(language_name(Some("xx_YY")).as_deref(), Some("xx_YY"));
        assert_eq!(language_name(Some("")), None);
        assert_eq!(language_short(Some("en-US")).as_deref(), Some("English (US)"));
        assert_eq!(language_short(Some("fr")).as_deref(), Some("French"));
    }
}
