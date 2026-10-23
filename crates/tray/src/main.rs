#[cfg(target_os = "linux")]
mod notify;
#[cfg(target_os = "linux")]
mod tray;
#[cfg(target_os = "linux")]
mod watch;

#[cfg(target_os = "linux")]
fn main() {
    use std::io::Write as _;
    use std::sync::mpsc;

    use drakeflake_tray_core::State;
    use ksni::blocking::TrayMethods as _;

    let nix = drakeflake_tray_core::start("drakeflake-tray");

    // Held (locked) until the process exits: one tray per user session.
    let _instance = drakeflake_tray_core::single_instance_or_exit();

    if let Err(error) = std::thread::Builder::new().name("watch".to_owned()).spawn({
        let nix = nix.clone();
        move || drakeflake_tray_core::watch::run(&nix, watch::open)
    }) {
        drakeflake_log::warn!("cannot start the flake watch: {error}");
    }

    let (requests, receiver) = mpsc::channel();
    let icon = tray::Tray {
        state: State::default(),
        requests,
    };

    let handle = match icon.spawn() {
        Ok(handle) => handle,
        Err(error) => {
            drakeflake_log::error!("no system tray: {error}");

            // The log already has it, so a closed stderr loses nothing.
            writeln!(
                std::io::stderr(),
                "drakeflake-tray: no system tray: {error}"
            )
            .ok();
            std::process::exit(1);
        }
    };

    drakeflake_tray_core::run(
        &nix,
        &receiver,
        |state| {
            let state = state.clone();
            handle.update(move |shown| shown.state = state);
        },
        notify::notify,
    );
    handle.shutdown().wait();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    use std::io::Write as _;

    // The exit code still tells the failure when stderr is closed.
    writeln!(
        std::io::stderr(),
        "drakeflake-tray is the Linux tray: use drakeflake-tray-macos on macOS"
    )
    .ok();
    std::process::exit(1);
}
