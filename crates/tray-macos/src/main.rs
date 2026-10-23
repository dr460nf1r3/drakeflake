#[cfg(target_os = "macos")]
mod icons;
#[cfg(target_os = "macos")]
mod menu_bar;
#[cfg(target_os = "macos")]
mod notify;
#[cfg(target_os = "macos")]
mod watch;

#[cfg(target_os = "macos")]
fn main() {
    use std::io::Write as _;
    use std::sync::mpsc;

    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let nix = drakeflake_tray_core::start("drakeflake-tray");

    // Held (locked) until the process exits: one tray per user session.
    let _instance = drakeflake_tray_core::single_instance_or_exit();

    let Some(main_thread) = MainThreadMarker::new() else {
        // The exit code still tells the failure when stderr is closed.
        writeln!(
            std::io::stderr(),
            "drakeflake-tray-macos: not on the main thread"
        )
        .ok();
        std::process::exit(1);
    };

    let app = NSApplication::sharedApplication(main_thread);

    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    if let Err(error) = std::thread::Builder::new().name("watch".to_owned()).spawn({
        let nix = nix.clone();
        move || drakeflake_tray_core::watch::run(&nix, watch::open)
    }) {
        drakeflake_log::warn!("cannot start the flake watch: {error}");
    }

    let (requests, receiver) = mpsc::channel();
    menu_bar::install(requests);

    let checks = std::thread::Builder::new()
        .name("checks".to_owned())
        .spawn(move || {
            drakeflake_tray_core::run(&nix, &receiver, menu_bar::show, notify::notify);

            // Quit was chosen: `AppKit`'s run loop never returns by itself.
            std::process::exit(0);
        });

    if let Err(error) = checks {
        drakeflake_log::error!("cannot start the update checks: {error}");
        std::process::exit(1);
    }

    app.run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    use std::io::Write as _;

    // The exit code still tells the failure when stderr is closed.
    writeln!(
        std::io::stderr(),
        "drakeflake-tray-macos is the macOS tray: use drakeflake-tray on Linux"
    )
    .ok();
    std::process::exit(1);
}
