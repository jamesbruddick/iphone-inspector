# iPhone Inspector

A tray app that shows every stat a USB-connected iPhone reports: model, identifiers, battery health
and cycles, storage, software, cellular details, and every raw value behind them. It runs on
Windows, macOS and Linux, and everything stays on your machine.

- **Click the tray icon** (either button) for its menu: open the stats page, see which iPhones are
  connected, switch *Start at Login* on or off, or quit.
- **Launched normally**, it opens the page. **Started at login**, it stays in the tray and shows a
  notification when an iPhone is plugged in.
- Launching it again while it is running just opens the page.

It is one Rust binary: a small web service on `127.0.0.1:3820` with the UI compiled in.

## Download

Get the installer for your system from the latest release on the Releases page:

| System | Download |
|---|---|
| Mac with Apple silicon (M1 and later) | `iphone-inspector-<version>-macos-arm64.dmg` |
| Mac with Intel | `iphone-inspector-<version>-macos-amd64.dmg` |
| Windows 10 and 11 | `iphone-inspector-<version>-windows-amd64.msi` |
| Ubuntu and Debian (Intel/AMD) | `iphone-inspector-<version>-linux-amd64.deb` |
| Ubuntu and Debian (ARM) | `iphone-inspector-<version>-linux-arm64.deb` |

- **macOS:** open the `.dmg` and drag *iPhone Inspector* into Applications. It is not notarized, so
  the first time, open it, then choose **Open Anyway** in System Settings → Privacy & Security.
- **Windows:** run the `.msi`. It installs for your user only (no administrator prompt) into
  `%LocalAppData%\Programs\iPhone Inspector`, adds a Start menu entry, and is removed from
  Settings → Apps, which also takes it out of your login items. If SmartScreen stops it, choose
  **More info → Run anyway**.
- **Ubuntu and Debian:** `sudo apt install ./iphone-inspector-<version>-linux-amd64.deb` installs it with the
  USB service and tray library it needs, and adds it to the app menu. It is built on Ubuntu 24.04,
  so it needs 24.04 or Debian 13 or newer.

Once installed, the downloaded file can be deleted. Each release also has portable versions that
run without installing: `-portable.zip` for Windows and `.tar.gz` for other Linux systems.

## Requirements

Nothing to install on macOS. The app talks to the phone itself (a built-in Rust client for the
protocols libimobiledevice speaks), through the USB service the system already has:

| OS | USB service |
|---|---|
| macOS | Built in |
| Windows | The Apple Devices app (or iTunes), which installs Apple's USB driver |
| Ubuntu / Debian | `sudo apt install usbmuxd` (usually there already on desktop installs) |

Pairing uses that service's pair records, so a phone that already trusts the computer (from Finder,
iTunes or libimobiledevice) is read without pairing again, and a pairing made here is shared with
them. iOS versions that only speak TLS 1.0 (iOS 10 and earlier) are not supported.

## Build

Needs Rust 1.90+ and Node 22.18+ with pnpm.

```sh
cd web && pnpm install && pnpm build && cd ..   # the UI, into web/dist
cargo build --release                           # embeds web/dist
./target/release/iphone-inspector
```

Linux also needs the tray and notification libraries to build:

```sh
sudo apt install libgtk-3-dev libayatana-appindicator3-dev libdbus-1-dev
```

The first time a **release** build runs, it adds itself to your login items with `--background`.
Uncheck *Start at Login* in the tray menu to turn that off; it stays off. Removing the app never
leaves a broken login item behind:

| OS | Login item | When the app is removed |
|---|---|---|
| macOS 13+ | A launch agent inside the app bundle, shown under the app's name in System Settings → General → Login Items | It goes with the app |
| Windows | The `Run` registry key | The installer removes it |
| Linux | `~/.config/autostart/iPhone Inspector.desktop`, with `TryExec` naming the binary | Desktops skip it once the binary is gone |

On macOS 11 and 12, or when run as a bare binary, it uses a launch agent in `~/Library/LaunchAgents`.

| Option | |
|---|---|
| `--background` | Start in the tray without opening the page |
| `--uninstall` | Remove the login item and exit (the Windows installer runs this on uninstall) |
| `PORT=4000` | Serve on another port (default 3820) |

## Working on it

```sh
cargo run                  # the app, serving the last web/dist build
cd web && pnpm dev         # Vite with hot reload, proxying /api to the app on :3820
cargo test                 # model tables, color matching, parsing, formatting, pairing and TLS
cargo test -- --ignored    # also talks to this computer's USB service and lists plugged-in phones
```

```
src/
  main.rs        Startup, single-instance check, the event loop
  tray.rs        Tray icon, menu, plug-in notifications
  icon.rs        The tray icon, drawn in code
  autostart.rs   Start at login
  server.rs      /api/devices, /pair, /read, /api/history, and the embedded UI
  history.rs     Phones read on this computer, saved to history.json
  monitor.rs     The connected-device list shared by the API and the tray
  tools.rs       What the app reads from a phone, with timeouts
  idevice/       The phone client: usbmux, lockdown, TLS, pairing, diagnostics, installed apps
  reading.rs     Raw plists -> the summary and the full read-out
  lookups.rs     Model, region, part-number and serial-date tables
  finishes.rs    Reported color -> nearest Apple finish
  locale.rs      Language and region names
  value.rs       Plist helpers
assets/          App icon and packaging: the .icns, the Windows installer (main.wxs), the Linux
                 menu entry. `python3 assets/generate.py` redraws the icons.
web/             React + Vite UI; `web/src/lib/finishes.json` is shared with finishes.rs
```

### Releasing

Bump `version` in `Cargo.toml` (and `web/package.json` to match) and push to `main`. The Release
workflow notices the new version, builds the installers for every system, signs their build
provenance and publishes them as `v<version>`. Any other push only runs CI.

To try the packaging without publishing, run the Release workflow by hand (Actions → Release → Run
workflow): it builds every download and leaves them on the run's page. Tick **Publish** to release
the current version instead - the way to finish a release whose run failed part way. The version must be plain
`major.minor.patch`, since the Windows installer cannot carry a pre-release suffix.

### Adding a new iPhone

Add its identifier to `MODELS` in `src/lookups.rs` (AppleDB lists them), its colors to
`web/src/lib/finishes.json`, and its camera layout to `LAYOUTS` in `web/src/lib/finishes.ts`. Covered
through the September 2026 lineup: iPhone 17e (`iPhone18,5`), iPhone 18 Pro (`iPhone19,2`), iPhone 18
Pro Max (`iPhone19,3` US, `iPhone19,7` global) and iPhone Duo (`iPhone19,4`).

## What the phone will not say

Newer iOS versions no longer report the housing color, Find My or device management status. When
a phone stays quiet about something, the page says so rather than guessing. For the color, pick it
from the swatches of the finishes Apple sold for that model; the choice is remembered in your
browser for that phone (by serial number) and used for its illustration. Battery health is
calculated from raw capacities and can differ from Settings by a point or two.

## Security

The service listens on `127.0.0.1` only, rejects requests whose `Host` is not local (DNS rebinding),
and rejects cross-site requests by `Origin`, so neither the network nor another web page can read
the phone or start pairing.

## History

Once a phone has been read in full (storage included), its reading is saved so its stats stay
available after it is unplugged. It is listed under **History** in the sidebar; the × beside it
(tap twice) removes it. One entry per phone, replaced each time it is read again.

The history is a JSON file in the app's data folder, never uploaded:

| OS | Location |
|---|---|
| macOS | `~/Library/Application Support/iphone-inspector/history.json` |
| Windows | `%APPDATA%\iphone-inspector\history.json` |
| Linux | `~/.local/share/iphone-inspector/history.json` |

It holds everything the phone reported, including IMEIs, serial, phone number and ICCID, so
treat it like any other file with that information in it.
