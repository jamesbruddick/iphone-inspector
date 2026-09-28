//! What the rest of the app asks of a phone, answered by the built-in client in `idevice`.

use std::fmt;
use std::future::Future;
use std::time::Duration;

use plist::Dictionary;

use crate::idevice::lockdown::Lockdown;
use crate::idevice::services::{self, Diagnostics};
use crate::idevice::{self, pairing, usbmux};

#[derive(Debug, Clone)]
pub struct ToolError {
    pub message: String,
    /// The system's USB service is not installed, which is a setup problem rather than a fault.
    pub missing_tool: bool,
}

impl ToolError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), missing_tool: false }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<idevice::Error> for ToolError {
    fn from(e: idevice::Error) -> Self {
        match e {
            idevice::Error::NoUsbService(_) => Self { message: MISSING_USB_SERVICE.into(), missing_tool: true },
            e => Self::new(e.to_string()),
        }
    }
}

#[cfg(target_os = "macos")]
const MISSING_USB_SERVICE: &str = "macOS's iPhone USB service (usbmuxd) is not answering. Restart the Mac and try again.";
#[cfg(windows)]
const MISSING_USB_SERVICE: &str =
    "Windows needs Apple's USB driver to talk to an iPhone. Install the Apple Devices app (or iTunes), then restart iPhone Inspector.";
#[cfg(all(unix, not(target_os = "macos")))]
const MISSING_USB_SERVICE: &str = "Linux needs usbmuxd to talk to an iPhone. Install it, then plug the iPhone in again.";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// How long a disk-usage query is given.
///
/// iOS computes these figures on demand and the first request after a phone is plugged in is slow -
/// measured at 23 seconds on an iPhone 14, against 0.03s for every other domain.
const DISK_TIMEOUT: Duration = Duration::from_secs(45);

async fn within<T>(limit: Duration, what: &str, work: impl Future<Output = idevice::Result<T>>) -> Result<T, ToolError> {
    match tokio::time::timeout(limit, work).await {
        Ok(result) => Ok(result?),
        Err(_) => Err(ToolError::new(format!("The iPhone did not answer {what} within {} seconds.", limit.as_secs()))),
    }
}

/// Linux starts usbmuxd only while an iPhone is plugged in, so its socket being absent usually
/// just means there is nothing connected. It is missing only when usbmuxd is not installed.
#[cfg(all(unix, not(target_os = "macos")))]
fn usbmuxd_installed() -> bool {
    ["/usr/sbin/usbmuxd", "/usr/bin/usbmuxd", "/usr/local/sbin/usbmuxd", "/sbin/usbmuxd"].iter().any(|p| std::path::Path::new(p).exists())
}

pub async fn list_devices() -> Result<Vec<String>, ToolError> {
    match within(DEFAULT_TIMEOUT, "the device list", usbmux::devices()).await {
        Ok(devices) => Ok(devices.into_iter().map(|d| d.udid).collect()),
        #[cfg(all(unix, not(target_os = "macos")))]
        Err(e) if e.missing_tool && usbmuxd_installed() => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// What the device list shows for a paired phone.
pub struct Identity {
    pub name: String,
    pub product_type: String,
    pub ios: Option<String>,
    pub serial: Option<String>,
}

/// Fails when the phone has not trusted this computer, which is how the monitor tells.
pub async fn identify(udid: &str) -> Result<Identity, ToolError> {
    within(Duration::from_secs(8), "its name", async {
        let mut lockdown = Lockdown::session(udid).await?;
        let name = lockdown.get_string("DeviceName").await?;
        let product_type = lockdown.get_string("ProductType").await?;
        let ios = lockdown.get_string("ProductVersion").await?;
        let serial = lockdown.get_string("SerialNumber").await?;
        match (name, product_type) {
            (Some(name), Some(product_type)) => Ok(Identity { name, product_type, ios, serial }),
            _ => Err(idevice::Error::NotPaired),
        }
    })
    .await
}

pub async fn pair(udid: &str) -> Result<String, ToolError> {
    within(pairing::TRUST_WAIT + DEFAULT_TIMEOUT, "the pairing request", pairing::pair(udid)).await?;
    Ok("This computer is now trusted by the iPhone.".into())
}

/// Everything the phone would tell us, as the raw plists it came in.
#[derive(Debug, Default)]
pub struct RawSources {
    pub lockdown: Dictionary,
    pub disk: Option<Dictionary>,
    pub disk_factory: Option<Dictionary>,
    pub battery: Option<Dictionary>,
    pub fmip: Option<Dictionary>,
    pub international: Option<Dictionary>,
    pub backup: Option<Dictionary>,
    pub chaperone: Option<Dictionary>,
    pub wireless: Option<Dictionary>,
    pub ioreg: Option<Dictionary>,
    pub gas_gauge: Option<Dictionary>,
    pub wifi_diag: Option<Dictionary>,
    pub gestalt: Option<Dictionary>,
    pub app_count: Option<usize>,
}

/// A lockdown session for the optional reads. A read that fails in transit, or times out, leaves
/// the connection in an unknown state, so it is dropped and the next read opens a new one.
struct Optional<'a> {
    udid: &'a str,
    session: Option<Lockdown>,
}

impl Optional<'_> {
    async fn session(&mut self) -> Option<&mut Lockdown> {
        if self.session.is_none() {
            self.session = within(DEFAULT_TIMEOUT, "a new session", Lockdown::session(self.udid)).await.ok();
        }
        self.session.as_mut()
    }

    async fn domain(&mut self, domain: &str, limit: Duration) -> Option<Dictionary> {
        let lockdown = self.session().await?;
        match tokio::time::timeout(limit, lockdown.get_domain(Some(domain))).await {
            Ok(Ok(found)) => found,
            _ => {
                self.session = None;
                None
            }
        }
    }

    async fn diagnostics(&mut self, src: &mut RawSources) {
        let Some(lockdown) = self.session().await else { return };
        let mut diag = match tokio::time::timeout(DEFAULT_TIMEOUT, Diagnostics::start(lockdown)).await {
            Ok(Ok(diag)) => diag,
            // The phone declined the service; the session is still good for the rest.
            Ok(Err(idevice::Error::Device(_))) => return,
            // Cut off mid-request: the session is not safe to reuse.
            _ => {
                self.session = None;
                return;
            }
        };
        // Each answer is independent; a transport failure ends the lot, since the service
        // connection is no use after one.
        let step = |r: Result<idevice::Result<Option<Dictionary>>, _>| match r {
            Ok(Ok(found)) => Ok(found),
            _ => Err(()),
        };
        let t = DEFAULT_TIMEOUT;
        let result: Result<(), ()> = async {
            src.ioreg = match step(tokio::time::timeout(t, diag.ioregistry_entry("AppleSmartBattery")).await)? {
                Some(found) => Some(found),
                None => step(tokio::time::timeout(t, diag.ioregistry_entry("AppleARMPMUCharger")).await)?,
            };
            src.gas_gauge = step(tokio::time::timeout(t, diag.request("GasGauge")).await)?;
            src.wifi_diag = step(tokio::time::timeout(t, diag.request("WiFi")).await)?;
            // Older iOS versions report the housing color here; newer ones usually refuse.
            let colors = ["DeviceEnclosureRGBColor", "DeviceCoverGlassColor", "DeviceHousingColor"];
            src.gestalt = step(tokio::time::timeout(t, diag.mobile_gestalt(&colors)).await)?;
            Ok(())
        }
        .await;
        if result.is_ok() {
            let _ = tokio::time::timeout(Duration::from_secs(2), diag.goodbye()).await;
        }
    }

    /// Installed app count. Optional extra.
    async fn app_count(&mut self) -> Option<usize> {
        let lockdown = self.session().await?;
        match tokio::time::timeout(Duration::from_secs(30), services::user_app_count(lockdown)).await {
            Ok(Ok(count)) => Some(count),
            _ => None,
        }
    }
}

/// Reads everything the phone will tell us.
///
/// Everything goes over one lockdown session, one request after another: some iOS versions reject
/// several simultaneous sessions and start failing at random. It is all cheap - well under a
/// second - which is the point: `with_disk` holds back the queries that are not, so opening a phone
/// does not wait on them.
pub async fn collect(udid: &str, with_disk: bool) -> Result<RawSources, ToolError> {
    let (session, lockdown) = within(DEFAULT_TIMEOUT, "the read", async {
        let mut session = Lockdown::session(udid).await?;
        let all = session.get_domain(None).await?.ok_or_else(|| idevice::Error::Protocol("The iPhone returned no values.".into()))?;
        Ok((session, all))
    })
    .await?;

    let mut src = RawSources { lockdown, ..Default::default() };
    let mut opt = Optional { udid, session: Some(session) };
    src.battery = opt.domain("com.apple.mobile.battery", DEFAULT_TIMEOUT).await;
    src.fmip = opt.domain("com.apple.fmip", DEFAULT_TIMEOUT).await;
    src.international = opt.domain("com.apple.international", DEFAULT_TIMEOUT).await;
    src.backup = opt.domain("com.apple.mobile.backup", DEFAULT_TIMEOUT).await;
    src.chaperone = opt.domain("com.apple.mobile.chaperone", DEFAULT_TIMEOUT).await;
    src.wireless = opt.domain("com.apple.mobile.wireless_lockdown", DEFAULT_TIMEOUT).await;
    opt.diagnostics(&mut src).await;

    if with_disk {
        src.disk = opt.domain("com.apple.disk_usage", DISK_TIMEOUT).await;
        // The factory domain answers with the same keys and is slower still - 57 seconds on the same
        // phone - so it is asked only when the live domain came back without the total capacity.
        if src.disk.as_ref().and_then(|d| d.get("TotalDiskCapacity")).is_none() {
            src.disk_factory = opt.domain("com.apple.disk_usage.factory", DISK_TIMEOUT).await;
        }
        src.app_count = opt.app_count().await;
    }

    Ok(src)
}
