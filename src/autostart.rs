//! Starting at login. The login entry launches the app with `--background`, so it comes up in the
//! tray without opening a browser window.
//!
//! Each system's entry is one that goes away, or goes quiet, with the app, so deleting the app
//! never leaves login looking for it:
//!
//! - macOS 13+: a launch agent inside the app bundle, registered with `SMAppService`. System
//!   Settings shows it under the app's name, and deleting the app removes it.
//! - Windows: the `Run` registry key. The installer removes it on uninstall (`--uninstall`).
//! - Linux: an XDG autostart entry whose `TryExec` names the binary; desktops skip it once the
//!   binary is gone.
//!
//! Older macOS, and a bare binary outside an app bundle, get a plain launch agent in
//! `~/Library/LaunchAgents` instead.

use std::path::PathBuf;

pub const BACKGROUND_FLAG: &str = "--background";
/// Run by the Windows installer as it removes the app. See [`forget`].
pub const UNINSTALL_FLAG: &str = "--uninstall";

// The plain entry for this system; on macOS, what is used when the bundled agent is not.
#[cfg(not(target_os = "linux"))]
use launcher as system;
#[cfg(target_os = "linux")]
use xdg as system;

pub fn is_enabled() -> bool {
    #[cfg(target_os = "macos")]
    if bundled::is_enabled() {
        return true;
    }
    system::is_enabled()
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if let Some(done) = bundled::set_enabled(enabled) {
        // Registered or unregistered in the bundle: a launch agent left by an older version or a
        // failed registration would otherwise start a second copy at login, or keep starting one.
        let _ = system::set_enabled(false);
        return done;
    }
    system::set_enabled(enabled)
}

fn first_run_marker() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("iphone-inspector").join("autostart-configured"))
}

/// Turn starting at login on the first time the app runs, and keep the entry pointing at wherever
/// the app lives now if it has been moved since. After the first run the tray toggle is the only
/// thing that changes it, so switching it off stays off.
///
/// Release builds only: a `cargo run` should not leave a debug binary in the login items.
pub fn configure_on_launch() {
    if cfg!(debug_assertions) {
        return;
    }
    let Some(marker) = first_run_marker() else { return };
    if !marker.exists() {
        if set_enabled(true).is_ok() {
            let _ = std::fs::create_dir_all(marker.parent().unwrap_or(&marker));
            let _ = std::fs::write(&marker, "");
        }
    } else if is_enabled() {
        let _ = set_enabled(true);
    }
}

/// Remove the login entry and the first-run marker, so login does not go looking for an app that is
/// no longer there, and installing again starts afresh with *Start at Login* on. The history is
/// left alone: it is the user's data.
pub fn forget() {
    let _ = set_enabled(false);
    if let Some(marker) = first_run_marker() {
        let _ = std::fs::remove_file(marker);
    }
}

/// The launch agent shipped in `iPhone Inspector.app/Contents/Library/LaunchAgents`.
#[cfg(target_os = "macos")]
mod bundled {
    use smappservice_rs::{AppService, ServiceStatus, ServiceType};

    /// Must match the file the release workflow puts in the bundle, from `assets/macos`.
    const PLIST: &str = "com.jamesbruddick.iphone-inspector.login.plist";

    /// The agent, when this is macOS 13+ and the app is running from its bundle.
    fn service() -> Option<AppService> {
        // Before macOS 13 there is no such class, and messaging it would abort.
        objc2::runtime::AnyClass::get(c"SMAppService")?;
        // Looked for on disk: the framework reports an agent it has never registered as NotFound
        // too, so its status cannot tell a bare binary from a bundle that has not registered yet.
        let exe = std::env::current_exe().ok()?;
        let contents = exe.parent()?.parent()?;
        contents.join("Library/LaunchAgents").join(PLIST).is_file().then(|| AppService::new(ServiceType::Agent { plist_name: PLIST }))
    }

    /// Only `Enabled` starts at login: `RequiresApproval` is also what switching the app off in
    /// System Settings looks like.
    pub fn is_enabled() -> bool {
        service().is_some_and(|s| s.status() == ServiceStatus::Enabled)
    }

    /// `None` when the bundled agent is not available, or cannot be registered - an ad hoc signed
    /// app can be refused - so the caller falls back to a plain launch agent.
    pub fn set_enabled(enabled: bool) -> Option<Result<(), String>> {
        let service = service()?;
        if !enabled {
            // Unregistering what is not registered fails, and is what was wanted anyway.
            let _ = service.unregister();
            return Some(Ok(()));
        }
        // Registering again is an error, so leave an agent that is already set up alone.
        if !matches!(service.status(), ServiceStatus::Enabled | ServiceStatus::RequiresApproval) {
            service.register().ok()?;
        }
        if service.status() == ServiceStatus::RequiresApproval {
            // Switched off in System Settings, or awaiting a first approval: take them there.
            AppService::open_system_settings_login_items();
        }
        Some(Ok(()))
    }
}

/// A plain login entry through the `auto-launch` crate: the `Run` key on Windows, and on macOS a
/// launch agent in `~/Library/LaunchAgents` that points at wherever the app is.
#[cfg(not(target_os = "linux"))]
mod launcher {
    use auto_launch::{AutoLaunch, AutoLaunchBuilder, MacOSLaunchMode, WindowsEnableMode};

    fn launcher() -> Option<AutoLaunch> {
        let exe = std::env::current_exe().ok()?;
        AutoLaunchBuilder::new()
            .set_app_name("iPhone Inspector")
            .set_app_path(exe.to_str()?)
            .set_args(&[super::BACKGROUND_FLAG])
            .set_macos_launch_mode(MacOSLaunchMode::LaunchAgent)
            .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
            .build()
            .ok()
    }

    pub fn is_enabled() -> bool {
        launcher().and_then(|l| l.is_enabled().ok()).unwrap_or(false)
    }

    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        let launcher = launcher().ok_or("Could not work out where this app is installed.")?;
        if enabled { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())
    }
}

/// `~/.config/autostart/iPhone Inspector.desktop` - the name auto-launch used, so an entry from an
/// older version is the one replaced.
#[cfg(target_os = "linux")]
mod xdg {
    use std::path::{Path, PathBuf};

    fn entry() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("autostart").join("iPhone Inspector.desktop"))
    }

    pub fn is_enabled() -> bool {
        entry().is_some_and(|e| e.exists())
    }

    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        let entry = entry().ok_or("Could not find the autostart folder.")?;
        if !enabled {
            return match std::fs::remove_file(&entry) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            };
        }
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(entry.parent().unwrap_or(&entry)).map_err(|e| e.to_string())?;
        std::fs::write(&entry, desktop_entry(&exe)).map_err(|e| e.to_string())
    }

    fn desktop_entry(exe: &Path) -> String {
        let exe = exe.to_string_lossy();
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=iPhone Inspector\n\
             Comment=Shows a notification when an iPhone is plugged in\n\
             Exec={} {}\n\
             TryExec={}\n\
             Icon=iphone-inspector\n\
             Terminal=false\n\
             StartupNotify=false\n\
             X-GNOME-Autostart-enabled=true\n",
            quote(&exe),
            super::BACKGROUND_FLAG,
            escape(&exe),
        )
    }

    /// A desktop-entry string value: backslashes and line breaks escaped.
    fn escape(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\n', "\\n").replace('\t', "\\t").replace('\r', "\\r")
    }

    /// An `Exec` argument: double-quoted, with the characters the spec reserves inside quotes
    /// backslash-escaped, then escaped once more as a string value.
    fn quote(s: &str) -> String {
        let mut quoted = String::from('"');
        for c in s.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        escape(&quoted)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_entry_survives_awkward_paths_and_goes_quiet_without_the_binary() {
            let entry = desktop_entry(Path::new("/home/a b/$HOME/iphone-inspector"));
            assert!(entry.contains("Exec=\"/home/a b/\\\\$HOME/iphone-inspector\" --background\n"));
            assert!(entry.contains("TryExec=/home/a b/$HOME/iphone-inspector\n"));
        }
    }
}
