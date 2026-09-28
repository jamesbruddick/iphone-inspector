//! The two services the app reads beyond lockdown itself.

use plist::{Dictionary, Value};

use super::lockdown::Lockdown;
use super::{BoxStream, Error, Result, dict, recv_plist, send_plist};

/// diagnostics_relay: ioreg entries, gas gauge, Wi-Fi and MobileGestalt.
pub struct Diagnostics(BoxStream);

impl Diagnostics {
    pub async fn start(lockdown: &mut Lockdown) -> Result<Self> {
        match lockdown.start_service("com.apple.mobile.diagnostics_relay").await {
            Ok(stream) => Ok(Self(stream)),
            // Its name before iOS 5.
            Err(Error::Device(_)) => Ok(Self(lockdown.start_service("com.apple.iosdiagnostics.relay").await?)),
            Err(e) => Err(e),
        }
    }

    /// The reply's `Diagnostics` dictionary - what `idevicediagnostics` prints - or None when the
    /// phone declines. Transport errors are errors: the connection is no use after one.
    async fn ask(&mut self, request: Dictionary) -> Result<Option<Dictionary>> {
        send_plist(&mut self.0, &request).await?;
        let reply = recv_plist(&mut self.0).await?;
        if reply.get("Status").and_then(Value::as_string) != Some("Success") {
            return Ok(None);
        }
        Ok(reply.get("Diagnostics").and_then(Value::as_dictionary).cloned())
    }

    pub async fn ioregistry_entry(&mut self, name: &str) -> Result<Option<Dictionary>> {
        self.ask(dict([("Request", "IORegistry".into()), ("EntryName", name.into()), ("EntryClass", "".into())])).await
    }

    /// `GasGauge`, `WiFi`, `NAND`, `All`.
    pub async fn request(&mut self, kind: &str) -> Result<Option<Dictionary>> {
        self.ask(dict([("Request", kind.into())])).await
    }

    pub async fn mobile_gestalt(&mut self, keys: &[&str]) -> Result<Option<Dictionary>> {
        let keys = keys.iter().map(|k| Value::from(*k)).collect();
        self.ask(dict([("Request", "MobileGestalt".into()), ("MobileGestaltKeys", Value::Array(keys))])).await
    }

    pub async fn goodbye(mut self) {
        let _ = send_plist(&mut self.0, &dict([("Request", "Goodbye".into())])).await;
        let _ = recv_plist(&mut self.0).await;
    }
}

/// How many apps the user installed, from installation_proxy - `ideviceinstaller -l`'s list.
pub async fn user_app_count(lockdown: &mut Lockdown) -> Result<usize> {
    let mut stream = lockdown.start_service("com.apple.mobile.installation_proxy").await?;
    let options = dict([("ApplicationType", "User".into()), ("ReturnAttributes", Value::Array(vec!["CFBundleIdentifier".into()]))]);
    send_plist(&mut stream, &dict([("Command", "Browse".into()), ("ClientOptions", Value::Dictionary(options))])).await?;
    // The list arrives in pages, then a final `Complete`.
    let mut count = 0;
    loop {
        let reply = recv_plist(&mut stream).await?;
        if let Some(code) = reply.get("Error").and_then(Value::as_string) {
            return Err(Error::Device(code.to_string()));
        }
        count += reply.get("CurrentList").and_then(Value::as_array).map_or(0, Vec::len);
        if reply.get("Status").and_then(Value::as_string) == Some("Complete") {
            return Ok(count);
        }
    }
}
