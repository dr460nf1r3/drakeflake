use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use log::{debug, warn};

/// The store copy on PATH is not setuid and cannot elevate, so the wrapper path is used.
const PKEXEC_WRAPPER: &str = "/run/wrappers/bin/pkexec";
const SUDO_WRAPPER: &str = "/run/wrappers/bin/sudo";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Elevation {
    Pkexec(PathBuf),
    Run0(PathBuf),
    Sudo(PathBuf),
    MacDialog(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prompt {
    DialogOnly,
    TerminalAllowed,
}

impl Elevation {
    #[must_use = "only returns the program path"]
    pub fn program(&self) -> &Path {
        match self {
            Self::Pkexec(path) | Self::Run0(path) | Self::Sudo(path) | Self::MacDialog(path) => {
                path
            }
        }
    }

    #[must_use = "the prefix is only built, nothing runs"]
    pub fn prefix(&self) -> Vec<OsString> {
        let program = self.program().as_os_str().to_owned();

        match self {
            // run0 tints the terminal background red by default.
            Self::Run0(_) => vec![program, OsString::from("--background=")],
            Self::Pkexec(_) | Self::Sudo(_) | Self::MacDialog(_) => vec![program],
        }
    }

    #[must_use = "only returns the name"]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Pkexec(_) => "pkexec",
            Self::Run0(_) => "run0",
            Self::Sudo(_) => "sudo",
            Self::MacDialog(_) => "the macOS administrator dialog",
        }
    }
}

#[must_use = "only searches PATH, nothing is elevated"]
pub fn find_elevation(prompt: Prompt) -> Option<Elevation> {
    let paths = std::env::var_os("PATH").unwrap_or_default();
    let found = choose_elevation(&paths, prompt, mac_dialog(), is_setuid_root);

    match &found {
        Some(elevation) => debug!(
            "elevating with {} ({})",
            elevation.name(),
            elevation.program().display()
        ),
        None => warn!("no way to get root: no setuid pkexec, run0 or (with a terminal) sudo"),
    }

    found
}

fn choose_elevation(
    paths: &std::ffi::OsStr,
    prompt: Prompt,
    mac: Option<PathBuf>,
    is_setuid: impl Fn(&Path) -> bool,
) -> Option<Elevation> {
    let on_path = |name: &'static str| std::env::split_paths(paths).map(move |dir| dir.join(name));
    let setuid = |wrapper: &str, name: &'static str| {
        std::iter::once(PathBuf::from(wrapper))
            .chain(on_path(name))
            .find(|candidate| is_setuid(candidate))
    };

    mac.map(Elevation::MacDialog)
        .or_else(|| setuid(PKEXEC_WRAPPER, "pkexec").map(Elevation::Pkexec))
        .or_else(|| {
            on_path("run0")
                .find(|candidate| is_executable_file(candidate))
                .map(Elevation::Run0)
        })
        .or_else(|| match prompt {
            Prompt::TerminalAllowed => setuid(SUDO_WRAPPER, "sudo").map(Elevation::Sudo),
            Prompt::DialogOnly => None,
        })
}

fn mac_dialog() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") || !Path::new("/usr/bin/osascript").is_file() {
        return None;
    }

    let beside_program = || {
        std::env::current_exe()
            .ok()?
            .parent()
            .map(|dir| dir.join("drakeflake-elevate"))
    };

    std::env::var_os("DRAKEFLAKE_ELEVATE")
        .map(PathBuf::from)
        .or_else(|| beside_program().filter(|path| is_executable_file(path)))
        .or_else(in_source_tree)
        .filter(|path| is_executable_file(path))
}

/// Dev builds have no libexec yet. Not compiled into release builds, where
/// the builder's temporary folder would be a path any local user could recreate.
#[cfg(debug_assertions)]
fn in_source_tree() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/macos/drakeflake-elevate.sh")
        .canonicalize()
        .ok()
}

#[cfg(not(debug_assertions))]
const fn in_source_tree() -> Option<PathBuf> {
    None
}

fn is_setuid_root(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;

    const SETUID: u32 = 0o4000;

    fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && meta.uid() == 0 && meta.mode() & SETUID != 0)
}

fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;

    fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests;
