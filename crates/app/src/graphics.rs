//! Qt aborts (qFatal) when the GPU backend fails, so the renderer is probed in a child process first.

use std::io::Write as _;
use std::time::Duration;

use drakeflake_exec::Context;
use qtbridge::QApp;

pub const PROBE_ARG: &str = "--drakeflake-probe-graphics";

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

const PROBE_QML: &[u8] = br#"import QtQuick
Window {
    width: 1
    height: 1
    visible: true
    color: "transparent"
    opacity: 0
    flags: Qt.FramelessWindowHint | Qt.WindowDoesNotAcceptFocus | Qt.Tool
    onFrameSwapped: Qt.quit()
    Timer { interval: 4000; running: true; onTriggered: Qt.exit(3) }
}
"#;

pub fn run_probe() -> i32 {
    let mut app = QApp::new();
    app.load_qml(PROBE_QML);
    app.run()
}

pub fn choose_renderer(exec: &Context) -> &'static str {
    if std::env::var_os("QT_QUICK_BACKEND").is_some()
        || std::env::var_os("QSG_RHI_BACKEND").is_some()
    {
        return "set by the environment";
    }

    let forced = std::env::var("DRAKEFLAKE_SOFTWARE_RENDERING").is_ok_and(|value| value == "1");

    if !forced && probe(exec) {
        return "hardware";
    }

    if !forced {
        writeln!(
            std::io::stderr(),
            "drakeflake: hardware-accelerated rendering is unavailable; using software rendering"
        )
        .ok();
    }

    #[expect(
        unsafe_code,
        reason = "set_var is unsafe in Rust 2024 because C code may read the environment concurrently"
    )]
    // SAFETY: called from `main` before Qt and the stderr capture thread
    // start. The probe's pipe reader threads may still exist, but they only use
    // Rust's std, whose environment access takes the lock `set_var` takes.
    unsafe {
        std::env::set_var("QT_QUICK_BACKEND", "software");
    }

    "software"
}

fn probe(exec: &Context) -> bool {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            log::warn!("cannot find the program for the graphics probe: {error}");
            return false;
        }
    };

    match exec.command(&exe, [PROBE_ARG], PROBE_TIMEOUT) {
        Ok(captured) => {
            log::debug!(
                "graphics probe success: {}, stderr: {}",
                captured.success,
                captured.stderr.trim()
            );
            captured.success
        }
        Err(error) => {
            log::warn!("graphics probe failed to run: {error}");
            false
        }
    }
}
