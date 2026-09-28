//! iPhone Inspector: a tray app that serves a local web page with every stat a connected iPhone
//! reports over USB.
//!
//! Started normally, it puts an icon in the tray and opens the page. Started at login (with
//! `--background`) it only puts the icon in the tray, and says so with a notification when an
//! iPhone is plugged in.

// A tray app has no console. Debug builds keep one, for the logs.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod autostart;
mod finishes;
mod history;
mod icon;
mod idevice;
mod locale;
mod lookups;
mod monitor;
mod reading;
mod server;
mod tools;
mod tray;
mod value;

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::Duration;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;

use crate::monitor::{Monitor, Snapshot};
use crate::tray::{Action, Tray, UserEvent};

const DEFAULT_PORT: u16 = 3820;

fn port() -> u16 {
    std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(DEFAULT_PORT)
}

fn page_url(port: u16, udid: Option<&str>) -> String {
    match udid {
        Some(udid) => format!("http://127.0.0.1:{port}/#/device/{udid}"),
        None => format!("http://127.0.0.1:{port}/"),
    }
}

fn open_page(port: u16, udid: Option<&str>) {
    if let Err(e) = open::that_detached(page_url(port, udid)) {
        eprintln!("Could not open the browser: {e}");
    }
}

/// Whether the thing already listening on the port is another copy of this app.
fn already_running(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(1)) else { return false };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let request = format!("GET /api/ping HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    let mut response = String::new();
    stream.write_all(request.as_bytes()).is_ok()
        && stream.read_to_string(&mut response).is_ok()
        && response.contains(&format!("\"app\":\"{}\"", server::PING_APP))
}

fn alert(message: &str) {
    eprintln!("{message}");
    let _ = notify_rust::Notification::new().appname("iPhone Inspector").summary("iPhone Inspector").body(message).show();
}

fn main() {
    if std::env::args().any(|a| a == autostart::UNINSTALL_FLAG) {
        autostart::forget();
        return;
    }
    // First, before anything can raise a notification - including the port-in-use alert below.
    tray::set_notification_sender();
    let background = std::env::args().any(|a| a == autostart::BACKGROUND_FLAG);
    let port = port();

    // Bind before anything else, so a second launch finds out at once that it is the second one.
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            if already_running(port) {
                // Launching the app again is how people look for its window: show them the page.
                if !background {
                    open_page(port, None);
                }
                return;
            }
            alert(&format!("Port {port} is already in use by another program. Quit it, or start iPhone Inspector with PORT set to a free port."));
            std::process::exit(1);
        }
        Err(e) => {
            alert(&format!("Could not start the local service on port {port}: {e}"));
            std::process::exit(1);
        }
    };

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("tokio runtime");
    let monitor = Monitor::new();

    runtime.spawn({
        let monitor = monitor.clone();
        async move {
            let listener = listener.set_nonblocking(true).and_then(|()| tokio::net::TcpListener::from_std(listener));
            match listener {
                Ok(listener) => {
                    let state = server::AppState { monitor, history: std::sync::Arc::new(history::History::load()) };
                    if let Err(e) = server::serve(listener, state).await {
                        alert(&format!("The local service stopped: {e}"));
                    }
                }
                Err(e) => alert(&format!("Could not start the local service: {e}")),
            }
        }
    });
    runtime.spawn(monitor.clone().poll_forever());

    let mut builder = EventLoopBuilder::<UserEvent>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        // A menu-bar app: no Dock icon, no app menu.
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        let mut event_loop = builder.build();
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
        run(event_loop, runtime, monitor, port, background);
    }
    #[cfg(not(target_os = "macos"))]
    run(builder.build(), runtime, monitor, port, background);
}

fn run(
    event_loop: tao::event_loop::EventLoop<UserEvent>,
    runtime: tokio::runtime::Runtime,
    monitor: std::sync::Arc<Monitor>,
    port: u16,
    background: bool,
) -> ! {
    // The menu crate calls this from its own thread; it is forwarded to the event loop.
    let proxy = Mutex::new(event_loop.create_proxy());
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.lock().map(|p| p.send_event(UserEvent::Menu(e)));
    }));

    let proxy = event_loop.create_proxy();
    let mut changes = monitor.subscribe();
    // Polling started before this subscription, and a receiver counts the value it subscribed to
    // as seen: without this, a phone plugged in before launch never reaches the tray, because the
    // list does not change again until something is plugged or unplugged.
    changes.mark_changed();
    runtime.spawn(async move {
        while changes.changed().await.is_ok() {
            let snapshot = changes.borrow_and_update().clone();
            if proxy.send_event(UserEvent::Devices(snapshot)).is_err() {
                break;
            }
        }
    });

    autostart::configure_on_launch();
    let mut tray: Option<Tray> = None;
    // The newest list, for a tray created after it arrived.
    let mut latest: Option<Snapshot> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        // Owned by the loop so the service keeps running for as long as the app does.
        let _ = &runtime;

        let action = match event {
            // The tray icon can only be created once the loop is running (macOS requires it).
            Event::NewEvents(StartCause::Init) => {
                match Tray::new() {
                    Ok(mut t) => {
                        if let Some(snapshot) = &latest {
                            t.update(snapshot);
                        }
                        tray = Some(t);
                    }
                    Err(e) => alert(&format!("Could not create the tray icon: {e}")),
                }
                if background { Action::None } else { Action::OpenPage(None) }
            }
            Event::UserEvent(UserEvent::Devices(snapshot)) => {
                if let Some(t) = tray.as_mut() {
                    t.update(&snapshot);
                }
                latest = Some(snapshot);
                Action::None
            }
            Event::UserEvent(UserEvent::Menu(e)) => tray.as_mut().map_or(Action::None, |t| t.handle_menu(&e)),
            _ => Action::None,
        };

        match action {
            Action::OpenPage(udid) => open_page(port, udid.as_deref()),
            Action::Quit => {
                // Dropping the icon removes it; on Windows a leftover one lingers until hovered.
                tray.take();
                *control_flow = ControlFlow::Exit;
            }
            Action::None => {}
        }
    })
}
