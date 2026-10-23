//! Runs from `main` before any other thread exists, since `set_var` is only sound then.

use crate::{PROGRAM, stderr_log};

#[expect(
    unsafe_code,
    reason = "declares the Qt icon functions of icon_theme.cpp (see the call sites)"
)]
unsafe extern "C" {
    fn drakeflake_use_breeze_icons();
    fn drakeflake_use_icon_theme(
        search_path: *const std::ffi::c_char,
        name: *const std::ffi::c_char,
    );
}

pub fn qml_import_paths() -> Vec<std::path::PathBuf> {
    [
        "QML_IMPORT_PATH",
        "QML2_IMPORT_PATH",
        "NIXPKGS_QT6_QML_IMPORT_PATH",
    ]
    .iter()
    .filter_map(std::env::var_os)
    .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
    .collect()
}

fn qml_module_available(relative: &str) -> bool {
    module_in(&qml_import_paths(), relative)
}

fn module_in(dirs: &[std::path::PathBuf], relative: &str) -> bool {
    dirs.iter()
        .any(|dir| dir.join(relative).join("qmldir").is_file())
}

pub fn ensure_qt_paths() {
    if qml_module_available("org/kde/kirigami") {
        return;
    }

    let fallback = option_env!("DRAKEFLAKE_QML_FALLBACK_PATH").unwrap_or_default();
    let paths: Vec<_> = std::env::split_paths(fallback)
        .filter(|dir| dir.is_dir())
        .collect();

    if paths.is_empty() {
        return;
    }

    log::debug!("QML import paths from the build: {fallback}");
    set_env(
        "QML_IMPORT_PATH",
        std::env::join_paths(paths).unwrap_or_default(),
    );

    if std::env::var_os("QT_PLUGIN_PATH").is_none() {
        let plugins = option_env!("DRAKEFLAKE_QT_PLUGIN_FALLBACK_PATH").unwrap_or_default();
        let dirs: Vec<_> = std::env::split_paths(plugins)
            .filter(|dir| dir.is_dir())
            .collect();

        if !dirs.is_empty() {
            set_env(
                "QT_PLUGIN_PATH",
                std::env::join_paths(dirs).unwrap_or_default(),
            );
        }
    }
}

/// The QWidget-based `org.kde.desktop` style needs a `QApplication` and misrenders without one.
pub fn choose_controls_style() -> String {
    let current = std::env::var("QT_QUICK_CONTROLS_STYLE").unwrap_or_default();
    let Some(style) = replacement_style(&current, cfg!(target_os = "macos"), || {
        qml_module_available("org/kde/breeze")
    }) else {
        return current;
    };

    log::debug!("controls style {current:?} replaced by {style}");

    set_env("QT_QUICK_CONTROLS_STYLE", style);
    style.to_owned()
}

fn replacement_style(
    current: &str,
    macos: bool,
    breeze_available: impl FnOnce() -> bool,
) -> Option<&'static str> {
    let plain_macos = macos && current == "macOS";
    if !current.is_empty() && current != "org.kde.desktop" && !plain_macos {
        return None;
    }

    Some(if macos {
        crate::macos::CONTROLS_STYLE
    } else if breeze_available() {
        "org.kde.breeze"
    } else {
        "Fusion"
    })
}

/// Only the app's own QML runs, so this opens nothing to outside content.
pub fn allow_qml_file_reads() {
    set_env("QML_XHR_ALLOW_FILE_READ", "1");
}

/// Lets a same-user debugger attach despite Yama `ptrace_scope` 1. Release builds keep the restriction.
#[cfg(target_os = "linux")]
pub fn allow_debugger_attach() {
    if cfg!(debug_assertions)
        && let Err(err) = rustix::process::set_ptracer(rustix::process::PTracer::Any)
    {
        log::debug!("debuggers cannot attach: {err}");
    }
}

#[cfg(not(target_os = "linux"))]
pub const fn allow_debugger_attach() {}

pub fn register_icon_theme() {
    let dir = std::env::var_os("DRAKEFLAKE_ICON_RCC_DIR")
        .or_else(|| option_env!("DRAKEFLAKE_ICON_RCC_FALLBACK_DIR").map(std::ffi::OsString::from));

    let Some(dir) = dir.filter(|dir| !dir.is_empty()) else {
        return;
    };

    let dark = dark_appearance();
    let file = std::path::Path::new(&dir).join(if dark {
        "breeze-dark.rcc"
    } else {
        "breeze.rcc"
    });

    let data = match std::fs::read(&file) {
        Ok(data) => data,
        Err(err) => {
            log::warn!("icons not loaded from {}: {err}", file.display());
            return;
        }
    };

    // Registered bytes stay mapped for the life of the program.
    if qtbridge::qresource::register_bytes(Vec::leak(data)) {
        log::debug!("icon theme from {}", file.display());

        #[expect(
            unsafe_code,
            reason = "one Qt call without arguments, made before the Qt application exists"
        )]
        // SAFETY: the call takes no pointers. Qt has no other user yet, and the
        // threads that exist (stderr capture, graphics probe readers) never
        // call into Qt.
        unsafe {
            drakeflake_use_breeze_icons();
        }

        use_symbol_theme(dark);
    } else {
        log::warn!("{} is not a Qt resource file", file.display());
    }
}

/// SF Symbols on macOS. Only registered after Breeze, which fills the gaps.
fn use_symbol_theme(dark: bool) {
    let Some(dir) = crate::macos::symbol_theme(dark) else {
        return;
    };

    let (Ok(search), Ok(name)) = (
        std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()),
        std::ffi::CString::new(crate::macos::symbols::THEME_NAME),
    ) else {
        return;
    };

    log::debug!("SF Symbols from {}", dir.display());

    #[expect(
        unsafe_code,
        reason = "one Qt call, made before the Qt application exists"
    )]
    // SAFETY: both strings are NUL-terminated and outlive the call. Qt has no
    // other user yet, and the threads that exist never call into Qt.
    unsafe {
        drakeflake_use_icon_theme(search.as_ptr(), name.as_ptr());
    }
}

#[cfg(target_os = "macos")]
fn dark_appearance() -> bool {
    std::process::Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
        .is_ok_and(|out| out.status.success() && out.stdout.starts_with(b"Dark"))
}

#[cfg(not(target_os = "macos"))]
const fn dark_appearance() -> bool {
    false
}

pub fn start_logging(level: &str) {
    drakeflake_log::set_level(level);
    log::debug!(
        "logging to {} at level {}",
        drakeflake_log::log_file(PROGRAM)
            .map_or_else(|_| "nowhere".to_owned(), |path| path.display().to_string()),
        drakeflake_log::level()
    );

    // Qt logs to the journal instead of stderr when there is no terminal.
    set_env("QT_FORCE_STDERR_LOGGING", "1");

    if std::env::var_os("QT_MESSAGE_PATTERN").is_none() {
        set_env("QT_MESSAGE_PATTERN", stderr_log::qt_message_pattern());
    }

    // Qt reads the rules once at startup, so QML debug output follows
    // the level the app started with.
    if log::max_level() >= log::LevelFilter::Debug && std::env::var_os("QT_LOGGING_RULES").is_none()
    {
        set_env("QT_LOGGING_RULES", "qml.debug=true;js.debug=true");
    }
}

fn set_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
    #[expect(
        unsafe_code,
        reason = "set_var is unsafe in Rust 2024 because C code may read the environment concurrently"
    )]
    // SAFETY: every caller runs from `main` before Qt and the stderr capture
    // thread start. Threads that exist earlier (graphics probe readers) only
    // use Rust's std, whose environment access takes the lock `set_var` takes.
    unsafe {
        std::env::set_var(key, value);
    }
}

#[cfg(test)]
mod tests;
