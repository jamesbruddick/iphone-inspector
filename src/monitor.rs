//! Which iPhones are plugged in, shared by the web API and the tray.
//!
//! One list for the whole app: the browser's poll and the background poll both refresh it, and
//! every change is published on a watch channel for the tray menu and the plug-in notification.
//! A phone's name and model are asked for once, when it appears, rather than on every poll - that
//! is a lockdown session per phone saved every few seconds.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::{Mutex, watch};

use crate::lookups::model;
use crate::tools;

const POLL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub udid: String,
    pub name: String,
    pub model: Option<String>,
    pub ios: Option<String>,
    pub serial: Option<String>,
    /// False when the phone has not yet tapped Trust on this computer.
    pub paired: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub devices: Vec<Device>,
    /// Set when the system's iPhone USB service is missing or failing.
    pub tool_error: Option<String>,
    /// The USB service is not installed at all, as opposed to installed and failing.
    #[serde(skip)]
    pub tool_missing: bool,
}

pub fn valid_udid(udid: &str) -> bool {
    (20..=64).contains(&udid.len()) && udid.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub struct Monitor {
    known: Mutex<HashMap<String, Device>>,
    tx: watch::Sender<Snapshot>,
}

impl Monitor {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { known: Mutex::new(HashMap::new()), tx: watch::Sender::new(Snapshot::default()) })
    }

    pub fn subscribe(&self) -> watch::Receiver<Snapshot> {
        self.tx.subscribe()
    }

    /// Look at the USB bus now. Holding the lock across the probe means two refreshes that land
    /// together - the browser's and the background one - never probe the same phone twice.
    pub async fn refresh(&self) -> Snapshot {
        let mut known = self.known.lock().await;
        let snapshot = match tools::list_devices().await {
            Ok(udids) => {
                let udids: Vec<String> = udids.into_iter().filter(|u| valid_udid(u)).collect();
                known.retain(|udid, _| udids.contains(udid));
                for udid in &udids {
                    // A phone that has not trusted this computer is asked again every time, so it
                    // turns into a named, paired device as soon as Trust is tapped.
                    if !known.get(udid).is_some_and(|d| d.paired) {
                        known.insert(udid.clone(), probe(udid).await);
                    }
                }
                Snapshot { devices: udids.iter().filter_map(|u| known.get(u).cloned()).collect(), tool_error: None, tool_missing: false }
            }
            Err(e) => {
                known.clear();
                // A missing toolchain is a setup problem, not a fault: an empty list plus why.
                let tool_missing = e.missing_tool;
                Snapshot { devices: Vec::new(), tool_error: Some(e.message), tool_missing }
            }
        };
        self.tx.send_if_modified(|current| {
            let changed = *current != snapshot;
            if changed {
                *current = snapshot.clone();
            }
            changed
        });
        snapshot
    }

    /// Drop what is known about a phone, so the next refresh asks it again - after pairing, say.
    pub async fn forget(&self, udid: &str) {
        self.known.lock().await.remove(udid);
    }

    /// Keeps the list current with no browser open, which is what the tray and the plug-in
    /// notification run on.
    pub async fn poll_forever(self: Arc<Self>) {
        loop {
            self.refresh().await;
            tokio::time::sleep(POLL).await;
        }
    }
}

/// A phone that has not trusted this computer answers the USB probe but refuses a lockdown session,
/// so the session failing is the signal that it needs pairing - not an error.
async fn probe(udid: &str) -> Device {
    match tools::identify(udid).await {
        Ok(id) => {
            Device { udid: udid.to_string(), name: id.name, model: Some(model(Some(&id.product_type)).name), ios: id.ios, serial: id.serial, paired: true }
        }
        Err(_) => Device { udid: udid.to_string(), name: "iPhone".into(), model: None, ios: None, serial: None, paired: false },
    }
}

#[cfg(test)]
mod tests {
    use super::valid_udid;

    #[test]
    fn udids() {
        assert!(valid_udid("00008150-000A1B2C3D4E5F60"));
        assert!(valid_udid("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"));
        assert!(!valid_udid("../../etc/passwd"));
        assert!(!valid_udid("short"));
    }
}
