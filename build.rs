//! The UI is compiled into the binary from `web/dist`, so it has to be built first. On Windows the
//! app icon and the file details (what Explorer and Task Manager show) are compiled in too.

fn main() {
    println!("cargo:rerun-if-changed=web/dist");
    if !std::path::Path::new("web/dist/index.html").exists() {
        panic!("\n\n  web/dist is missing. Build the UI first:\n\n    cd web && pnpm install && pnpm build\n\n");
    }

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/windows/app.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/windows/app.ico");
        // Task Manager lists a process by its FileDescription, which would otherwise be the
        // package's whole one-line description.
        res.set("FileDescription", "iPhone Inspector");
        res.set("ProductName", "iPhone Inspector");
        res.set("LegalCopyright", "Copyright (c) 2026 James B Ruddick");
        res.compile().expect("compiling the Windows resources (icon and version details)");
    }
}
