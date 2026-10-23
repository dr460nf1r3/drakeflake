pub mod symbols;
pub mod window;

#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "declares the Qt functions of macos.mm (see the call sites)"
)]
unsafe extern "C" {
    fn drakeflake_window_alpha();
    fn drakeflake_register_system_theme();
    fn drakeflake_default_text_app(
        path: *mut std::ffi::c_char,
        path_size: usize,
        bundle: *mut std::ffi::c_char,
        bundle_size: usize,
    ) -> bool;
    fn drakeflake_render_symbol(
        symbol: *const std::ffi::c_char,
        points: u32,
        scale: u32,
        dark: bool,
        path: *const std::ffi::c_char,
    ) -> bool;
}

#[cfg(target_os = "macos")]
static RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/macos.rcc"));

pub const CONTROLS_STYLE: &str = "DrakeFlakeMac";

#[cfg(any(test, target_os = "macos"))]
const KIRIGAMI_THEME: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../src/qml/kirigami/Theme.qml"
));

#[cfg(any(test, target_os = "macos"))]
const KIRIGAMI_INLINE_MESSAGE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../src/qml/kirigami/InlineMessage.qml"
));

/// Kirigami reads `Theme.qml` only from the first import path with `org/kde/kirigami`, so our mirror must come first.
#[cfg(target_os = "macos")]
pub fn add_kirigami_theme() {
    let Ok(root) = crate::hub::Cache::root() else {
        log::warn!("no cache folder: the Kirigami theme is not mirrored");
        return;
    };

    let cache = root.join("qml");
    let Some(kirigami) = kirigami_dir(&cache) else {
        log::warn!("Kirigami not found on the QML import paths, its colours stay Breeze's");
        return;
    };

    let mirror = match kirigami_mirror(&kirigami, &cache) {
        Ok(mirror) => mirror,
        Err(err) => {
            log::warn!(
                "cannot add the macOS theme to Kirigami in {}: {err}",
                cache.display()
            );
            return;
        }
    };

    log::debug!("Kirigami from {} with the macOS theme", mirror.display());

    let current = std::env::var_os("QML_IMPORT_PATH").unwrap_or_default();
    let paths = std::iter::once(mirror).chain(std::env::split_paths(&current));
    #[expect(
        unsafe_code,
        reason = "environment set before any other thread starts (set_var is unsafe in Rust 2024)"
    )]
    // SAFETY: called from `main` before any other thread is started.
    unsafe {
        std::env::set_var(
            "QML_IMPORT_PATH",
            std::env::join_paths(paths).unwrap_or_default(),
        );
    }
}

#[cfg(not(target_os = "macos"))]
pub const fn add_kirigami_theme() {}

#[cfg(target_os = "macos")]
fn kirigami_dir(cache: &std::path::Path) -> Option<std::path::PathBuf> {
    crate::environment::qml_import_paths()
        .into_iter()
        .filter(|dir| !dir.starts_with(cache))
        .map(|dir| dir.join("org/kde/kirigami"))
        .find(|dir| dir.is_dir())
}

/// Built aside and renamed, so a concurrent start never sees it half-built.
#[cfg(any(test, target_os = "macos"))]
fn kirigami_mirror(
    kirigami: &std::path::Path,
    cache: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    use std::hash::{DefaultHasher, Hash as _, Hasher as _};

    let mut hasher = DefaultHasher::new();
    kirigami.hash(&mut hasher);
    KIRIGAMI_THEME.hash(&mut hasher);
    KIRIGAMI_INLINE_MESSAGE.hash(&mut hasher);
    let mirror = cache.join(format!("{:016x}", hasher.finish()));
    if mirror.join("org/kde/kirigami/qmldir").is_file() {
        return Ok(mirror);
    }

    std::fs::create_dir_all(cache)?;
    remove_stale_mirrors(cache);
    let building = cache.join(format!(".building-{}", std::process::id()));
    if building.exists() {
        std::fs::remove_dir_all(&building)?;
    }

    // The stale-mirror sweep skips folders being built, so nothing else
    // would remove a failed one.
    if let Err(err) = fill_mirror(kirigami, &building) {
        discard_build(&building);
        return Err(err);
    }

    match std::fs::rename(&building, &mirror) {
        Ok(()) => Ok(mirror),
        Err(_) if mirror.join("org/kde/kirigami/qmldir").is_file() => {
            discard_build(&building);
            Ok(mirror)
        }
        Err(err) => {
            discard_build(&building);
            Err(err)
        }
    }
}

#[cfg(any(test, target_os = "macos"))]
fn discard_build(building: &std::path::Path) {
    if let Err(err) = std::fs::remove_dir_all(building) {
        log::debug!(
            "unfinished Kirigami mirror {} kept: {err}",
            building.display()
        );
    }
}

#[cfg(any(test, target_os = "macos"))]
fn remove_stale_mirrors(cache: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let building = entry.file_name().to_string_lossy().starts_with('.');
        if !building
            && !path.join("org/kde/kirigami/qmldir").is_file()
            && let Err(err) = std::fs::remove_dir_all(&path)
        {
            log::debug!("stale Kirigami mirror {} kept: {err}", path.display());
        }
    }
}

#[cfg(any(test, target_os = "macos"))]
fn fill_mirror(kirigami: &std::path::Path, base: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::symlink;

    let root = base.join("org/kde/kirigami");
    let styles = root.join("styles");
    let ours = styles.join(CONTROLS_STYLE);
    std::fs::create_dir_all(&ours)?;

    for entry in std::fs::read_dir(kirigami)? {
        let entry = entry?;
        if entry.file_name() != "styles" {
            symlink(entry.path(), root.join(entry.file_name()))?;
        }
    }

    if let Ok(entries) = std::fs::read_dir(kirigami.join("styles")) {
        for entry in entries.flatten() {
            symlink(entry.path(), styles.join(entry.file_name()))?;
        }
    }

    std::fs::write(ours.join("Theme.qml"), KIRIGAMI_THEME)?;
    std::fs::write(ours.join("InlineMessage.qml"), KIRIGAMI_INLINE_MESSAGE)
}

#[cfg(target_os = "macos")]
pub fn prepare() {
    if let (Some(home), Ok(root)) = (drakeflake_fs::home_dir(), crate::hub::Cache::root()) {
        drop_stale_qml_cache(
            &root.join("style-resources"),
            &home.join("Library/Caches/DrakeFlake/qmlcache"),
            RESOURCES,
        );
    }

    if !qtbridge::qresource::register_bytes(RESOURCES) {
        log::warn!("macOS controls style not registered, the controls fall back to Fusion");
    }

    #[expect(
        unsafe_code,
        reason = "one Qt call without arguments at startup, before threads exist"
    )]
    // SAFETY: only sets Qt's default surface format, single-threaded at startup.
    unsafe {
        drakeflake_window_alpha();
    }
}

#[cfg(not(target_os = "macos"))]
pub const fn prepare() {}

/// Qt reuses compiled QML of `qrc:` files although a new build changed them.
#[cfg(any(test, target_os = "macos"))]
fn drop_stale_qml_cache(marker: &std::path::Path, qml_cache: &std::path::Path, style: &[u8]) {
    let key = format!("{:016x}", drakeflake_fs::fnv1a(style));

    if std::fs::read_to_string(marker).is_ok_and(|seen| seen == key) {
        return;
    }

    match std::fs::remove_dir_all(qml_cache) {
        Ok(()) => log::debug!("dropped {} after a style change", qml_cache.display()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            log::warn!("cannot drop {}: {err}", qml_cache.display());
            return;
        }
    }

    let written = marker
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(marker, key));

    if let Err(err) = written {
        log::warn!("cannot note the style in {}: {err}", marker.display());
    }
}

/// Must run after the application exists, before the QML is loaded.
#[cfg(target_os = "macos")]
pub fn register_system_theme() {
    #[expect(
        unsafe_code,
        reason = "one Qt call without arguments on the GUI thread"
    )]
    // SAFETY: registers a QML singleton owned by the application, from the GUI thread.
    unsafe {
        drakeflake_register_system_theme();
    }
}

#[cfg(not(target_os = "macos"))]
pub const fn register_system_theme() {}

#[cfg(target_os = "macos")]
pub fn symbol_theme(dark: bool) -> Option<std::path::PathBuf> {
    let root = crate::hub::Cache::root()
        .inspect_err(|err| log::warn!("no SF Symbols theme: {err}"))
        .ok()?;

    let dir = symbols::search_dir(&root, dark);
    let render = |symbol: &str, size: u32, scale: u32, path: &std::path::Path| {
        render_symbol(symbol, size, scale, dark, path)
    };

    match symbols::install(&dir, render) {
        Ok(()) => Some(dir),
        Err(err) => {
            log::warn!("SF Symbols not written to {}: {err}", dir.display());
            None
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub const fn symbol_theme(_dark: bool) -> Option<std::path::PathBuf> {
    None
}

#[cfg(target_os = "macos")]
fn render_symbol(symbol: &str, size: u32, scale: u32, dark: bool, path: &std::path::Path) -> bool {
    let (Ok(symbol), Ok(path)) = (
        std::ffi::CString::new(symbol),
        std::ffi::CString::new(path.as_os_str().as_encoded_bytes()),
    ) else {
        return false;
    };

    #[expect(unsafe_code, reason = "one AppKit drawing into a new bitmap per call")]
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe {
        drakeflake_render_symbol(symbol.as_ptr(), size, scale, dark, path.as_ptr())
    }
}

#[cfg(target_os = "macos")]
pub fn default_text_app() -> Option<(std::path::PathBuf, String)> {
    let mut path = vec![0_u8; 4096];
    let mut bundle = vec![0_u8; 256];

    #[expect(unsafe_code, reason = "one AppKit lookup writing into buffers we own")]
    // SAFETY: both buffers outlive the call and their sizes are passed with
    // them. The function writes NUL-terminated text within those sizes.
    let found = unsafe {
        drakeflake_default_text_app(
            path.as_mut_ptr().cast(),
            path.len(),
            bundle.as_mut_ptr().cast(),
            bundle.len(),
        )
    };

    if !found {
        log::debug!("macOS names no default app for plain text");
        return None;
    }

    let text = |buffer: &[u8]| {
        std::ffi::CStr::from_bytes_until_nul(buffer)
            .ok()
            .map(|text| text.to_string_lossy().into_owned())
    };

    Some((std::path::PathBuf::from(text(&path)?), text(&bundle)?))
}

#[cfg(not(target_os = "macos"))]
pub const fn default_text_app() -> Option<(std::path::PathBuf, String)> {
    None
}

#[cfg(test)]
mod tests;
