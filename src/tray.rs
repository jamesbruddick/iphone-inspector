//! The tray icon and its menu, and the notification when an iPhone is plugged in.
//!
//! Everything here runs on the main thread, where the platform tray APIs require it. The rest of
//! the app talks to it only through `UserEvent`s sent to the event loop.

use std::collections::HashSet;

use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::monitor::{Device, Snapshot};
use crate::{autostart, icon};

pub enum UserEvent {
    Devices(Snapshot),
    Menu(MenuEvent),
}

/// What to do in response to a menu click.
pub enum Action {
    OpenPage(Option<String>),
    Quit,
    None,
}

/// Menus read `&` as a keyboard-shortcut marker; a phone called "Tom & Ana" needs it doubled.
fn menu_text(s: &str) -> String {
    s.replace('&', "&&")
}

fn describe(device: &Device) -> String {
    if !device.paired {
        return "iPhone — Unlock It and Tap Trust".into();
    }
    format!("{} · {}", device.model.as_deref().unwrap_or("iPhone"), device.name)
}

fn count(n: usize) -> String {
    match n {
        0 => "No iPhone Connected".into(),
        1 => "1 iPhone Connected".into(),
        n => format!("{n} iPhones Connected"),
    }
}

pub struct Tray {
    icon: TrayIcon,
    menu: Menu,
    open: MenuItem,
    status: MenuItem,
    /// Menu items for the connected phones, in menu order, with the phone each one opens.
    devices: Vec<(MenuItem, String)>,
    login: CheckMenuItem,
    quit: MenuItem,
    /// Phones that have already had their plug-in notification, so each gets exactly one.
    announced: HashSet<String>,
}

/// Position of the first device item: after "Open", the separator, and the status line.
const DEVICES_AT: usize = 3;

impl Tray {
    pub fn new() -> Result<Self, String> {
        let menu = Menu::new();
        let open = MenuItem::new("Open iPhone Inspector", true, None);
        let status = MenuItem::new(count(0), false, None);
        let login = CheckMenuItem::new("Start at Login", true, autostart::is_enabled(), None);
        let quit = MenuItem::new("Quit iPhone Inspector", true, None);
        let items: [&dyn IsMenuItem; 7] =
            [&open, &PredefinedMenuItem::separator(), &status, &PredefinedMenuItem::separator(), &login, &PredefinedMenuItem::separator(), &quit];
        menu.append_items(&items).map_err(|e| e.to_string())?;

        let icon = TrayIconBuilder::new()
            .with_icon(icon::tray_icon())
            .with_icon_as_template(true)
            .with_tooltip("iPhone Inspector")
            .with_menu(Box::new(menu.clone()))
            // Either click shows the menu, as Linux trays do anyway; the page is its first item.
            .with_menu_on_left_click(true)
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self { icon, menu, open, status, devices: Vec::new(), login, quit, announced: HashSet::new() })
    }

    pub fn handle_menu(&mut self, event: &MenuEvent) -> Action {
        let id = event.id();
        if id == self.open.id() {
            Action::OpenPage(None)
        } else if id == self.quit.id() {
            Action::Quit
        } else if id == self.login.id() {
            let wanted = self.login.is_checked();
            if let Err(e) = autostart::set_enabled(wanted) {
                eprintln!("Could not change start at login: {e}");
            }
            // Show what the system actually did, not what was asked for.
            self.login.set_checked(autostart::is_enabled());
            Action::None
        } else if let Some((_, udid)) = self.devices.iter().find(|(item, _)| item.id() == id) {
            Action::OpenPage(Some(udid.clone()))
        } else {
            Action::None
        }
    }

    pub fn update(&mut self, snapshot: &Snapshot) {
        for (item, _) in self.devices.drain(..) {
            let _ = self.menu.remove(&item);
        }
        let status = match snapshot.tool_error {
            Some(_) if snapshot.tool_missing => "Set Up USB to Read iPhones".to_string(),
            Some(_) => "Can't Read USB Devices Right Now".to_string(),
            None => count(snapshot.devices.len()),
        };
        self.status.set_text(&status);
        for (i, device) in snapshot.devices.iter().enumerate() {
            let item = MenuItem::new(menu_text(&describe(device)), true, None);
            let _ = self.menu.insert(&item, DEVICES_AT + i);
            self.devices.push((item, device.udid.clone()));
        }
        let _ = self.icon.set_tooltip(Some(format!("iPhone Inspector: {status}")));

        // Notify once per plug-in. A phone that was unplugged is forgotten, so plugging it back in
        // announces it again.
        let present: HashSet<String> = snapshot.devices.iter().map(|d| d.udid.clone()).collect();
        self.announced.retain(|u| present.contains(u));
        for device in &snapshot.devices {
            if self.announced.insert(device.udid.clone()) {
                notify(device);
            }
        }
    }
}

/// Name the app macOS notifications are sent as. Must run before the first notification.
///
/// Left to itself, the notification library looks up an app literally called "use_default" to
/// find a sender, and macOS answers that with a "Where is use_default?" dialog. Inside an
/// `iPhone Inspector.app` bundle the notifications come from the app itself; run as a bare binary,
/// they are sent as Finder, which is on every Mac.
#[cfg(target_os = "macos")]
pub fn set_notification_sender() {
    let id = bundle_identifier().unwrap_or_else(|| "com.apple.Finder".into());
    let _ = notify_rust::set_application(&id);
}

#[cfg(not(target_os = "macos"))]
pub fn set_notification_sender() {}

/// The bundle identifier when running as `Something.app/Contents/MacOS/<binary>`.
#[cfg(target_os = "macos")]
fn bundle_identifier() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let contents = exe.parent()?.parent()?;
    if contents.file_name()? != "Contents" {
        return None;
    }
    let info = plist::Value::from_file(contents.join("Info.plist")).ok()?;
    info.as_dictionary()?.get("CFBundleIdentifier")?.as_string().map(String::from)
}

fn notify(device: &Device) {
    let (summary, body) = if device.paired {
        let body = match &device.ios {
            Some(ios) => format!("{} · iOS {ios}", describe(device)),
            None => describe(device),
        };
        (format!("{} Connected", device.model.as_deref().unwrap_or("iPhone")), body)
    } else {
        ("iPhone Connected".to_string(), "Unlock it and tap Trust to see its stats.".to_string())
    };
    // Showing a notification can block while the system delivers it; keep that off the UI thread.
    std::thread::spawn(move || {
        if let Err(e) = notify_rust::Notification::new().appname("iPhone Inspector").summary(&summary).body(&body).show() {
            eprintln!("Could not show a notification: {e}");
        }
    });
}
