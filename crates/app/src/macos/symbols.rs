#[cfg(any(test, target_os = "macos"))]
use std::path::{Path, PathBuf};

#[cfg(any(test, target_os = "macos"))]
pub const SYMBOLS: &[(&str, &str)] = &[
    ("arrow-down", "arrow.down"),
    ("checkmark", "checkmark"),
    ("code-context", "chevron.left.forwardslash.chevron.right"),
    ("code-function", "function"),
    ("configure", "gearshape"),
    ("data-error", "exclamationmark.triangle"),
    ("dialog-cancel", "xmark.circle"),
    ("dialog-close", "xmark"),
    ("dialog-messages", "bubble.left.and.bubble.right"),
    ("dialog-ok", "checkmark"),
    ("dialog-ok-apply", "checkmark.circle"),
    ("dialog-password", "key"),
    ("dialog-question", "questionmark.circle"),
    ("dialog-scripts", "scroll"),
    ("document-edit", "pencil"),
    ("document-open", "doc"),
    ("document-open-folder", "folder"),
    ("document-preview", "eye"),
    ("document-preview-archive", "archivebox"),
    ("document-revert", "arrow.uturn.backward"),
    ("document-save", "square.and.arrow.down"),
    ("document-save-as", "square.and.arrow.down.on.square"),
    ("documentation", "book.closed"),
    ("download", "arrow.down.circle"),
    ("edit-clear", "xmark.circle.fill"),
    ("edit-clear-all", "xmark.bin"),
    ("edit-clear-history", "clock.arrow.circlepath"),
    ("edit-clear-locationbar-ltr", "xmark.circle.fill"),
    ("edit-clear-locationbar-rtl", "xmark.circle.fill"),
    ("edit-copy", "doc.on.doc"),
    ("edit-cut", "scissors"),
    ("edit-delete", "trash"),
    ("edit-none", "tray"),
    ("edit-paste", "doc.on.clipboard"),
    ("edit-undo", "arrow.uturn.backward"),
    ("emblem-error", "xmark.octagon"),
    ("emblem-information", "info.circle"),
    ("emblem-success", "checkmark.circle"),
    ("emblem-warning", "exclamationmark.triangle"),
    ("folder-new", "folder.badge.plus"),
    ("folder-open", "folder"),
    ("go-down", "chevron.down"),
    ("go-home", "house"),
    ("go-next", "chevron.forward"),
    ("go-next-symbolic", "chevron.forward"),
    ("go-previous", "chevron.backward"),
    ("go-previous-symbolic", "chevron.backward"),
    ("go-up", "chevron.up"),
    ("handle-sort", "arrow.up.arrow.down"),
    ("help-contents", "book"),
    ("help-hint", "lightbulb"),
    ("internet-services", "globe"),
    ("list-add", "plus"),
    ("list-remove", "minus"),
    ("media-playback-start", "play"),
    ("open-link-symbolic", "arrow.up.forward.square"),
    ("package", "shippingbox"),
    ("package-x-generic", "shippingbox"),
    ("password-show-off", "eye.slash"),
    ("password-show-on", "eye"),
    ("process-stop", "stop.circle"),
    ("run-build", "hammer"),
    ("search", "magnifyingglass"),
    ("settings-configure", "gearshape"),
    ("sidebar-collapse-left", "sidebar.left"),
    ("sidebar-expand-left", "sidebar.left"),
    ("system-reboot", "arrow.clockwise.circle"),
    ("system-run", "play.circle"),
    ("system-switch-user", "person.2"),
    ("tools-wizard", "wand.and.stars"),
    ("update-none", "checkmark.circle"),
    ("user", "person"),
    ("vcs-diff", "plus.forwardslash.minus"),
    ("vcs-normal", "checkmark.seal"),
    ("view-history", "clock.arrow.circlepath"),
    ("view-list-details", "list.bullet"),
    ("view-list-tree", "list.bullet.indent"),
    ("view-refresh", "arrow.clockwise"),
];

pub const THEME_NAME: &str = "drakeflake-sf";

#[cfg(any(test, target_os = "macos"))]
const SIZES: &[u32] = &[16, 22, 32, 48];
#[cfg(any(test, target_os = "macos"))]
const SCALES: &[u32] = &[1, 2];

/// Bumped when the drawing changes, so old caches are not reused.
#[cfg(any(test, target_os = "macos"))]
const RENDER_VERSION: u32 = 1;

/// Keeps `a=b` plus `c=d` apart from `a=bc=d` in the cache key.
#[cfg(any(test, target_os = "macos"))]
const PAIR_SEPARATOR: char = '\n';

#[cfg(any(test, target_os = "macos"))]
pub fn search_dir(cache: &Path, dark: bool) -> PathBuf {
    let mut key = RENDER_VERSION.to_string();

    for (name, symbol) in SYMBOLS {
        key.push(PAIR_SEPARATOR);
        key.push_str(name);
        key.push('=');
        key.push_str(symbol);
    }

    let variant = if dark { "dark" } else { "light" };
    cache.join("icons").join(format!(
        "sf-{variant}-{:016x}",
        drakeflake_fs::fnv1a(key.as_bytes())
    ))
}

#[cfg(any(test, target_os = "macos"))]
pub fn folder(size: u32, scale: u32) -> String {
    if scale == 1 {
        format!("{size}x{size}/actions")
    } else {
        format!("{size}x{size}@{scale}/actions")
    }
}

#[cfg(any(test, target_os = "macos"))]
pub fn index_theme() -> String {
    let folders: Vec<(u32, u32)> = SIZES
        .iter()
        .flat_map(|size| SCALES.iter().map(move |scale| (*size, *scale)))
        .collect();

    let mut text = format!(
        "[Icon Theme]\nName={THEME_NAME}\nComment=SF Symbols\nInherits=breeze\nDirectories={}\n",
        folders
            .iter()
            .map(|(size, scale)| folder(*size, *scale))
            .collect::<Vec<_>>()
            .join(",")
    );

    text.extend(folders.into_iter().map(|(size, scale)| {
        format!(
            "\n[{}]\nSize={size}\nScale={scale}\nContext=Actions\nType=Fixed\n",
            folder(size, scale)
        )
    }));

    text
}

/// `index.theme` is written last so an interrupted run renders again.
#[cfg(any(test, target_os = "macos"))]
pub fn install(dir: &Path, render: impl Fn(&str, u32, u32, &Path) -> bool) -> std::io::Result<()> {
    let theme = dir.join(THEME_NAME);
    let index = theme.join("index.theme");

    if index.is_file() {
        return Ok(());
    }

    let mut failed = Vec::new();

    for size in SIZES {
        for scale in SCALES {
            let folder = theme.join(folder(*size, *scale));
            std::fs::create_dir_all(&folder)?;

            for (name, symbol) in SYMBOLS {
                if !render(symbol, *size, *scale, &folder.join(format!("{name}.png"))) {
                    failed.push(*symbol);
                }
            }
        }
    }

    failed.sort_unstable();
    failed.dedup();

    if !failed.is_empty() {
        log::warn!("SF Symbols missing on this macOS, Breeze stands in: {failed:?}");
    }

    std::fs::write(index, index_theme())
}

#[cfg(test)]
mod tests;
