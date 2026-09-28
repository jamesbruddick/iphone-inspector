//! Every iPhone that has been fully read on this computer, so its stats can be looked at again
//! after it is unplugged.
//!
//! Kept as one JSON file in the app's data folder - `~/Library/Application Support/iphone-inspector`
//! on macOS, `%APPDATA%\iphone-inspector` on Windows, `~/.local/share/iphone-inspector` on Linux -
//! rather than in the browser, so it survives clearing site data and is the same in every browser.
//! One entry per phone: reading a phone again replaces its entry.

use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::value::iso;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    udid: String,
    saved_at: String,
    /// The reading exactly as the API returned it.
    reading: Value,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    version: u32,
    devices: Vec<Entry>,
}

/// What the sidebar needs to list a phone, without the whole reading.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    udid: String,
    saved_at: String,
    model: Value,
    device_name: Value,
    serial: Value,
    ios_version: Value,
    color_hex: Value,
}

pub struct History {
    path: Option<PathBuf>,
    entries: Mutex<Vec<Entry>>,
}

impl History {
    /// Load the saved history. A file that cannot be read is set aside rather than overwritten, so
    /// a bad write never costs more than the history it broke.
    pub fn load() -> Self {
        let path = dirs::data_dir().map(|d| d.join("iphone-inspector").join("history.json"));
        let entries = match path.as_ref().map(std::fs::read) {
            Some(Ok(bytes)) => match serde_json::from_slice::<File>(&bytes) {
                Ok(file) => file.devices,
                Err(e) => {
                    eprintln!("History file is unreadable ({e}); keeping it as history.json.bad and starting fresh.");
                    if let Some(p) = &path {
                        let _ = std::fs::rename(p, p.with_extension("json.bad"));
                    }
                    Vec::new()
                }
            },
            _ => Vec::new(),
        };
        Self { path, entries: Mutex::new(entries) }
    }

    /// Newest first.
    pub async fn list(&self) -> Vec<Summary> {
        let entries = self.entries.lock().await;
        // Entries are kept in the order they were saved, so walking them backwards and sorting
        // stably keeps two saves within the same millisecond in the right order.
        let mut list: Vec<Summary> = entries
            .iter()
            .rev()
            .map(|e| {
                let field = |k: &str| e.reading.get(k).cloned().unwrap_or(Value::Null);
                Summary {
                    udid: e.udid.clone(),
                    saved_at: e.saved_at.clone(),
                    model: field("model"),
                    device_name: field("deviceName"),
                    serial: field("serial"),
                    ios_version: field("iosVersion"),
                    color_hex: field("colorHex"),
                }
            })
            .collect();
        list.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
        list
    }

    pub async fn get(&self, udid: &str) -> Option<Value> {
        let entries = self.entries.lock().await;
        let entry = entries.iter().find(|e| e.udid == udid)?;
        let mut reading = entry.reading.clone();
        if let Some(obj) = reading.as_object_mut() {
            obj.insert("savedAt".into(), Value::String(entry.saved_at.clone()));
        }
        Some(reading)
    }

    pub async fn save(&self, udid: &str, reading: Value) {
        let mut entries = self.entries.lock().await;
        entries.retain(|e| e.udid != udid);
        entries.push(Entry { udid: udid.to_string(), saved_at: iso(Utc::now()), reading });
        self.write(&entries).await;
    }

    /// True if there was an entry to remove.
    pub async fn remove(&self, udid: &str) -> bool {
        let mut entries = self.entries.lock().await;
        let before = entries.len();
        entries.retain(|e| e.udid != udid);
        let removed = entries.len() != before;
        if removed {
            self.write(&entries).await;
        }
        removed
    }

    /// Write to a temporary file and rename it into place, so a crash mid-write leaves the old
    /// history intact instead of half a file.
    async fn write(&self, entries: &[Entry]) {
        let Some(path) = &self.path else { return };
        let file = File { version: 1, devices: entries.to_vec() };
        let result = async {
            if let Some(dir) = path.parent() {
                tokio::fs::create_dir_all(dir).await?;
            }
            let tmp = path.with_extension("json.tmp");
            tokio::fs::write(&tmp, serde_json::to_vec_pretty(&file)?).await?;
            // IMEIs, phone numbers and ICCIDs: readable by this user only.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                tokio::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).await?;
            }
            tokio::fs::rename(&tmp, path).await
        }
        .await;
        if let Err(e) = result {
            eprintln!("Could not save history to {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn in_memory() -> History {
        History { path: None, entries: Mutex::new(Vec::new()) }
    }

    #[tokio::test]
    async fn save_replace_and_remove() {
        let history = in_memory();
        history.save("aaaa", json!({ "model": "iPhone 18 Pro", "serial": "S1" })).await;
        history.save("bbbb", json!({ "model": "iPhone 17" })).await;
        history.save("aaaa", json!({ "model": "iPhone 18 Pro", "serial": "S1", "iosVersion": "27.0" })).await;

        let list = history.list().await;
        assert_eq!(list.len(), 2, "reading a phone again replaces its entry");
        assert_eq!(list[0].udid, "aaaa", "newest first");
        assert_eq!(history.get("aaaa").await.unwrap()["iosVersion"], "27.0");
        assert!(history.get("aaaa").await.unwrap()["savedAt"].is_string());

        assert!(history.remove("aaaa").await);
        assert!(!history.remove("aaaa").await);
        assert!(history.get("aaaa").await.is_none());
        assert_eq!(history.list().await.len(), 1);
    }
}
