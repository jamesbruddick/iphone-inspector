//! Turns the raw plist dumps into what the UI shows: a summary of the values that matter most, and
//! the full read-out arranged into sections. Ported from `server/read.ts` and `server/details.ts`.

use chrono::Utc;
use plist::Dictionary;
use serde::Serialize;
use serde_json::{Map, Value as Json};

use crate::finishes;
use crate::locale::{language_name, language_short};
use crate::lookups::{age_words, age_years, marketing_capacity, model, month_year, part_prefix, region_name, serial_date};
use crate::tools::RawSources;
use crate::value::{boolean, dict_to_json, display, find, fixed, fmt_num, gb, iso, num, text};

#[derive(Serialize)]
pub struct DetailRow {
    pub label: &'static str,
    pub value: String,
}

#[derive(Serialize)]
pub struct DetailSection {
    pub id: &'static str,
    pub name: &'static str,
    pub rows: Vec<DetailRow>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub advanced: bool,
}

/// Mirrors `DeviceReading` in `web/src/lib/types.ts`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceReading {
    pub udid: String,
    pub read_at: String,
    pub device_name: Option<String>,
    pub model: String,
    pub product_type: Option<String>,
    pub chip: Option<&'static str>,
    pub display: Option<&'static str>,
    pub released: Option<i32>,
    pub ios_version: Option<String>,
    pub build_version: Option<String>,
    pub storage: Option<String>,
    pub storage_free: Option<String>,
    /// Raw byte counts behind `storage` and `storage_free`, for the usage bar.
    pub storage_bytes: Option<f64>,
    pub free_bytes: Option<f64>,
    pub color: Option<String>,
    pub color_hex: Option<String>,
    pub imei: Option<String>,
    pub imei2: Option<String>,
    pub serial: Option<String>,
    pub battery_health: Option<f64>,
    pub cycle_count: Option<f64>,
    /// Current charge, in percent.
    pub battery_level: Option<f64>,
    pub charging: Option<bool>,
    /// mAh when new, and the most it holds now.
    pub design_capacity: Option<f64>,
    pub full_charge_capacity: Option<f64>,
    pub carrier: Option<String>,
    /// Apple's part number with its region suffix, e.g. "MJW44LL/A".
    pub part_number: Option<String>,
    pub passcode_set: Option<bool>,
    /// SIM state and SIM tray state in words, e.g. "Ready" / "Not Inserted", "Absent".
    pub sim_status: Option<String>,
    pub sim_tray: Option<String>,
    /// The phone's language, short: "English (US)".
    pub language: Option<String>,
    /// e.g. "America/Chicago".
    pub time_zone: Option<String>,
    pub find_my: Option<bool>,
    pub supervised: Option<bool>,
    pub activation: Option<String>,
    pub region: Option<&'static str>,
    pub unit_type: Option<&'static str>,
    pub unit_short: Option<&'static str>,
    pub unit_level: Option<&'static str>,
    pub manufactured: Option<String>,
    pub age: Option<String>,
    pub unavailable: Vec<&'static str>,
    pub sections: Vec<DetailSection>,
    pub raw: Json,
}

/// The live disk-usage domain wins wherever it and the factory one both answer.
fn merged_disk(src: &RawSources) -> Dictionary {
    let mut disk = src.disk_factory.clone().unwrap_or_default();
    for (k, v) in src.disk.iter().flatten() {
        disk.insert(k.clone(), v.clone());
    }
    disk
}

struct Battery {
    design: Option<f64>,
    full_charge: Option<f64>,
    cycles: Option<f64>,
    /// Full-charge capacity over design capacity - what Settings calls Maximum Capacity.
    health: Option<f64>,
}

/// Some iOS versions refuse the ioreg entry but answer the gas gauge, and the other way round.
fn battery(src: &RawSources) -> Battery {
    let io = src.ioreg.as_ref();
    let gg = src.gas_gauge.as_ref();
    let design = num(find(io, &["DesignCapacity"]).or_else(|| find(gg, &["DesignCapacity"])));
    let full_charge = num(find(io, &["AppleRawMaxCapacity", "NominalChargeCapacity"]).or_else(|| find(gg, &["FullChargeCapacity"])));
    let cycles = num(find(io, &["CycleCount"]).or_else(|| find(gg, &["CycleCount"])));
    let health = match (design, full_charge) {
        (Some(d), Some(f)) if d > 0.0 => Some((f / d * 100.0).min(100.0)),
        _ => None,
    };
    Battery { design, full_charge, cycles, health }
}

pub fn build_reading(udid: &str, src: &RawSources) -> DeviceReading {
    let l = &src.lockdown;
    let product_type = text(l.get("ProductType"));
    let m = model(product_type.as_deref());
    let mut unavailable = Vec::new();

    let disk = merged_disk(src);
    let storage = marketing_capacity(num(disk.get("TotalDiskCapacity")));
    if storage.is_none() {
        unavailable.push("Storage Capacity");
    }
    let storage_bytes = num(disk.get("TotalDiskCapacity"));
    let free_bytes = num(disk.get("AmountDataAvailable")).or_else(|| num(disk.get("TotalDataAvailable")));
    let storage_free = gb(free_bytes);

    let bat = battery(src);
    let battery_health = bat.health.map(|h| (h * 10.0).round() / 10.0);
    if battery_health.is_none() {
        unavailable.push("Battery Health");
    }
    if bat.cycles.is_none() {
        unavailable.push("Charge Cycles");
    }

    let find_my = boolean(src.fmip.as_ref().and_then(|d| d.get("IsAssociated")));
    if find_my.is_none() {
        unavailable.push("Find My");
    }
    let supervised = boolean(src.chaperone.as_ref().and_then(|d| d.get("DeviceIsChaperoned")));
    if supervised.is_none() {
        unavailable.push("Device Management");
    }

    let serial = text(l.get("SerialNumber"));
    let made = serial_date(serial.as_deref(), m.year);
    let part = part_prefix(text(l.get("ModelNumber")).as_deref());

    // Older iOS reports the housing color, as a name, a packed number or an RGB dictionary; newer
    // builds usually refuse. Whatever comes back is matched to the nearest finish for this model.
    let gestalt = src.gestalt.as_ref();
    let finish = finishes::detect(
        &m.name,
        &[
            find(gestalt, &["DeviceEnclosureRGBColor"]),
            find(gestalt, &["DeviceHousingColor"]),
            l.get("DeviceEnclosureColor"),
            find(gestalt, &["DeviceCoverGlassColor"]),
            l.get("DeviceColor"),
        ],
    );
    if finish.is_none() {
        unavailable.push("Color");
    }

    DeviceReading {
        udid: udid.to_string(),
        read_at: iso(Utc::now()),
        device_name: text(l.get("DeviceName")),
        model: m.name.clone(),
        product_type,
        chip: m.chip,
        display: m.screen,
        released: m.year,
        ios_version: text(l.get("ProductVersion")),
        build_version: text(l.get("BuildVersion")),
        storage,
        storage_free,
        storage_bytes,
        free_bytes,
        color: finish.as_ref().map(|f| f.name.clone()),
        color_hex: finish.map(|f| f.hex),
        imei: text(l.get("InternationalMobileEquipmentIdentity")),
        imei2: text(l.get("InternationalMobileEquipmentIdentity2")),
        serial,
        battery_health,
        cycle_count: bat.cycles,
        battery_level: num(src.battery.as_ref().and_then(|b| b.get("BatteryCurrentCapacity"))),
        charging: boolean(src.battery.as_ref().and_then(|b| b.get("BatteryIsCharging"))),
        design_capacity: bat.design,
        full_charge_capacity: bat.full_charge,
        carrier: carriers(l).filter(|c| !c.is_empty()),
        part_number: text(l.get("ModelNumber")).map(|n| format!("{n}{}", text(l.get("RegionInfo")).unwrap_or_default())),
        passcode_set: boolean(l.get("PasswordProtected")),
        sim_status: text(l.get("SIMStatus")).map(|s| words(&s, "kCTSIMSupportSIMStatus")),
        sim_tray: text(l.get("SIMTrayStatus")).map(|s| words(&s, "kCTSIMSupportSIMTray")),
        language: language_short(src.international.as_ref().and_then(|d| d.get("Language")).and_then(|v| v.as_string())),
        time_zone: text(l.get("TimeZone")).map(|t| t.replace('_', " ")),
        find_my,
        supervised,
        activation: text(l.get("ActivationState")),
        region: region_name(text(l.get("RegionInfo")).as_deref()),
        unit_type: part.map(|p| p.label),
        unit_short: part.map(|p| p.short),
        unit_level: part.map(|p| p.level),
        manufactured: made.map(month_year),
        age: age_words(age_years(made, m.year)),
        unavailable,
        sections: build_sections(src),
        raw: raw(src),
    }
}

/// "kCTSIMSupportSIMStatusReady" style constants, with the prefix dropped and the words split.
fn words(s: &str, prefix: &str) -> String {
    let s = s.replacen(prefix, "", 1);
    let mut out = String::with_capacity(s.len() + 4);
    let mut prev_lower = false;
    for c in s.chars() {
        if prev_lower && c.is_ascii_uppercase() {
            out.push(' ');
        }
        prev_lower = c.is_ascii_lowercase();
        out.push(c);
    }
    out
}

/// The carrier bundles on the SIMs, e.g. "ATT NR US".
fn carriers(l: &Dictionary) -> Option<String> {
    l.get("CarrierBundleInfoArray").and_then(|v| v.as_array()).map(|list| {
        list.iter()
            .filter_map(|c| c.as_dictionary()?.get("CFBundleIdentifier")?.as_string())
            .map(|id| id.replacen("com.apple.", "", 1).replace('_', " "))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    })
}

/// Rows with nothing in them are dropped, so a section never shows a blank line.
fn rows(pairs: Vec<(&'static str, Option<String>)>) -> Vec<DetailRow> {
    pairs.into_iter().filter_map(|(label, value)| value.filter(|v| !v.is_empty()).map(|value| DetailRow { label, value })).collect()
}

fn build_sections(src: &RawSources) -> Vec<DetailSection> {
    let l = &src.lockdown;
    let get = |key: &str| l.get(key);
    let io = src.ioreg.as_ref();
    let gg = src.gas_gauge.as_ref();
    let m = model(text(get("ProductType")).as_deref());

    let disk = merged_disk(src);
    let total = num(disk.get("TotalDiskCapacity"));
    let free = num(disk.get("AmountDataAvailable")).or_else(|| num(disk.get("TotalDataAvailable")));
    let used = total.zip(free).map(|(t, f)| t - f);
    let used_text = used.map(|u| match total.filter(|t| *t > 0.0) {
        Some(t) => format!("{} ({}%)", gb(Some(u)).unwrap_or_default(), (u / t * 100.0).round()),
        None => gb(Some(u)).unwrap_or_default(),
    });

    let bat = battery(src);
    let battery_domain = src.battery.as_ref();

    let serial = text(get("SerialNumber"));
    let made = serial_date(serial.as_deref(), m.year);
    let region = text(get("RegionInfo"));
    let model_number = text(get("ModelNumber"));

    let carriers = carriers(l);

    let clock_offset = num(get("TimeIntervalSince1970")).map(|t| {
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        format!("{} s", fmt_num((t - now).round() + 0.0))
    });

    let international = src.international.as_ref();
    let sections = vec![
        DetailSection {
            id: "model",
            name: "Model & Origin",
            advanced: false,
            rows: rows(vec![
                ("Model", Some(m.name.clone())),
                ("Storage Capacity", marketing_capacity(total)),
                ("Chip", m.chip.map(String::from)),
                ("Released", m.year.map(|y| y.to_string())),
                ("Manufactured", Some(made.map_or_else(|| "Not Encoded in This Serial".into(), month_year))),
                ("Unit Type", part_prefix(model_number.as_deref()).map(|p| p.label.to_string())),
                ("Sales Region", region.as_deref().map(|r| region_name(Some(r)).unwrap_or(r).to_string())),
                ("Part Number", model_number.as_ref().map(|n| format!("{n}{}", region.as_deref().unwrap_or("")))),
            ]),
        },
        DetailSection {
            id: "ids",
            name: "Identifiers",
            advanced: false,
            rows: rows(vec![
                ("Serial Number", display(get("SerialNumber"))),
                ("IMEI", display(get("InternationalMobileEquipmentIdentity"))),
                ("IMEI 2", display(get("InternationalMobileEquipmentIdentity2"))),
                ("MEID", display(get("MobileEquipmentIdentifier"))),
                ("Model Identifier", display(get("ProductType"))),
                ("UDID", display(get("UniqueDeviceID"))),
                ("ECID", get("UniqueChipID").and_then(|v| v.as_unsigned_integer()).map(|n| format!("0x{n:X}"))),
                ("Wi-Fi Address", display(get("WiFiAddress"))),
                ("Bluetooth Address", display(get("BluetoothAddress"))),
            ]),
        },
        DetailSection {
            id: "battery",
            name: "Battery",
            advanced: false,
            rows: rows(vec![
                ("Health", bat.health.map(|h| format!("{}%", fixed(h, 1)))),
                ("Maximum Capacity Now", bat.full_charge.map(|c| format!("{} mAh", fmt_num(c)))),
                ("Capacity When New", bat.design.map(|c| format!("{} mAh", fmt_num(c)))),
                ("Charge Cycles", bat.cycles.map(fmt_num)),
                ("Current Charge", display(battery_domain.and_then(|b| b.get("BatteryCurrentCapacity"))).map(|v| format!("{v}%"))),
                ("Charging", display(battery_domain.and_then(|b| b.get("BatteryIsCharging")))),
                ("Temperature", num(find(io, &["Temperature"])).map(|t| format!("{} °C", fixed(t / 100.0, 1)))),
                ("Voltage", num(find(io, &["Voltage"])).map(|v| format!("{} V", fixed(v / 1000.0, 2)))),
                ("Battery Serial", display(find(io, &["Serial", "BatterySerialNumber"]))),
            ]),
        },
        DetailSection {
            id: "storage",
            name: "Storage",
            advanced: false,
            rows: rows(vec![
                ("Capacity", marketing_capacity(total)),
                ("Used", used_text),
                ("Available", gb(free)),
                ("Photos", gb(num(disk.get("PhotoUsage")))),
                ("Apps", gb(num(disk.get("MobileApplicationUsage")))),
                ("Installed Apps", src.app_count.map(|n| n.to_string())),
            ]),
        },
        DetailSection {
            id: "software",
            name: "Software & Settings",
            advanced: false,
            rows: rows(vec![
                ("iOS Version", text(get("ProductVersion")).map(|v| format!("iOS {v}"))),
                ("Build", display(get("BuildVersion"))),
                ("Device Name", display(get("DeviceName"))),
                ("Passcode Set", display(get("PasswordProtected"))),
                ("Language", language_name(international.and_then(|d| d.get("Language")).and_then(|v| v.as_string()))),
                ("Region Format", language_name(international.and_then(|d| d.get("Locale")).and_then(|v| v.as_string()))),
                ("Time Zone", text(get("TimeZone")).map(|t| t.replace('_', " "))),
                ("Encrypted Backups", display(src.backup.as_ref().and_then(|d| d.get("WillEncrypt")))),
            ]),
        },
        DetailSection {
            id: "cellular",
            name: "Cellular & SIM",
            advanced: false,
            rows: rows(vec![
                ("SIM Status", text(get("SIMStatus")).map(|s| words(&s, "kCTSIMSupportSIMStatus"))),
                ("Carrier", carriers),
                ("Phone Number", display(get("PhoneNumber"))),
                ("ICCID", display(get("IntegratedCircuitCardIdentity"))),
                ("IMSI", display(get("InternationalMobileSubscriberIdentity"))),
                ("Modem Firmware", display(get("BasebandVersion"))),
            ]),
        },
        DetailSection {
            id: "technical",
            name: "Technical Details",
            advanced: true,
            rows: rows(vec![
                ("Board", display(get("HardwareModel"))),
                ("Chip ID", num(get("ChipID")).map(|n| format!("0x{:x}", n as i64))),
                ("CPU Architecture", display(get("CPUArchitecture"))),
                ("Production Chip", display(get("ProductionSOC"))),
                ("Bootloader", display(get("FirmwareVersion"))),
                ("Color Code", display(get("DeviceColor"))),
                ("Enclosure Color Code", display(get("DeviceEnclosureColor"))),
                ("Ethernet Address", display(get("EthernetAddress"))),
                ("SIM Tray", text(get("SIMTrayStatus")).map(|s| words(&s, "kCTSIMSupportSIMTray"))),
                ("Modem Status", display(get("BasebandStatus"))),
                ("Modem Bootloader", display(get("BasebandBootloaderVersion"))),
                ("Formatted Capacity", gb(total)),
                ("Data Partition", gb(num(disk.get("TotalDataCapacity")))),
                ("System Partition", gb(num(disk.get("TotalSystemCapacity")))),
                ("Battery Full-Charge Flag", display(find(io, &["FullyCharged"]))),
                ("External Power", display(find(io, &["ExternalConnected"]))),
                ("Battery Manufacturer", display(find(io, &["Manufacturer"]))),
                ("Gas Gauge Status", display(gg.and_then(|d| d.get("Status")))),
                // Newer iOS answers the Wi-Fi diagnostic with a placeholder rather than a status.
                (
                    "Wi-Fi Status",
                    display(find(src.wifi_diag.as_ref(), &["Status"]).or_else(|| find(src.wifi_diag.as_ref(), &["Active"])))
                        .filter(|s| !s.contains("Deprecated")),
                ),
                ("Wi-Fi Sync", display(src.wireless.as_ref().and_then(|d| d.get("EnableWifiConnections")))),
                ("Clock Offset", clock_offset),
                ("Trusted Computer Attached", display(get("TrustedHostAttached"))),
                ("Brick State", display(get("BrickState"))),
            ]),
        },
    ];

    sections.into_iter().filter(|s| !s.rows.is_empty()).collect()
}

/// The untouched dump behind the sections, blobs and dates made JSON-safe.
fn raw(src: &RawSources) -> Json {
    let opt = |d: &Option<Dictionary>| d.as_ref().map_or(Json::Null, dict_to_json);
    let mut out = Map::new();
    out.insert("lockdown".into(), dict_to_json(&src.lockdown));
    out.insert("disk".into(), opt(&src.disk));
    out.insert("diskFactory".into(), opt(&src.disk_factory));
    out.insert("appCount".into(), src.app_count.map_or(Json::Null, Json::from));
    out.insert("battery".into(), opt(&src.battery));
    out.insert("fmip".into(), opt(&src.fmip));
    out.insert("international".into(), opt(&src.international));
    out.insert("backup".into(), opt(&src.backup));
    out.insert("chaperone".into(), opt(&src.chaperone));
    out.insert("wireless".into(), opt(&src.wireless));
    out.insert("ioreg".into(), opt(&src.ioreg));
    out.insert("gasGauge".into(), opt(&src.gas_gauge));
    out.insert("wifiDiag".into(), opt(&src.wifi_diag));
    out.insert("gestalt".into(), opt(&src.gestalt));
    Json::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::yes_no;
    use plist::Value;

    fn lockdown() -> Dictionary {
        let mut l = Dictionary::new();
        for (k, v) in [
            ("ProductType", "iPhone19,2"),
            ("ProductVersion", "26.0"),
            ("DeviceName", "Test iPhone"),
            ("SerialNumber", "RANDOMSERIAL"),
            ("ModelNumber", "MQ123"),
            ("RegionInfo", "LL/A"),
            ("SIMStatus", "kCTSIMSupportSIMStatusReady"),
            ("DeviceColor", "1"),
        ] {
            l.insert(k.into(), Value::String(v.into()));
        }
        l.insert("PasswordProtected".into(), Value::Boolean(true));
        l
    }

    #[test]
    fn reading_from_a_new_phone() {
        let mut ioreg = Dictionary::new();
        ioreg.insert("DesignCapacity".into(), Value::Integer(4000.into()));
        ioreg.insert("AppleRawMaxCapacity".into(), Value::Integer(3801.into()));
        ioreg.insert("CycleCount".into(), Value::Integer(12.into()));
        let src = RawSources { lockdown: lockdown(), ioreg: Some(ioreg), ..Default::default() };

        let r = build_reading("00008150-000A1B2C3D4E5F60", &src);
        assert_eq!(r.model, "iPhone 18 Pro");
        assert_eq!(r.battery_health, Some(95.0));
        assert_eq!(r.cycle_count, Some(12.0));
        assert_eq!(r.region, Some("United States"));
        assert_eq!(r.unit_short, Some("Retail"));
        assert_eq!(r.color, None, "a bare color index must not be matched to a finish");
        assert!(r.unavailable.contains(&"Storage Capacity"));

        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["batteryHealth"], 95.0);
        assert_eq!(json["iosVersion"], "26.0");
        assert_eq!(json["partNumber"], "MQ123LL/A");
        assert_eq!(json["passcodeSet"], true);
        assert_eq!(json["simStatus"], "Ready");
        let cellular = r.sections.iter().find(|s| s.id == "cellular").unwrap();
        assert_eq!(cellular.rows[0].value, "Ready");
        let software = r.sections.iter().find(|s| s.id == "software").unwrap();
        assert!(software.rows.iter().any(|row| row.label == "Passcode Set" && row.value == yes_no(true)));
    }
}
